//! Direct Raft node wrapper without actors.
//!
//! This module provides a simplified Raft node implementation that directly
//! wraps OpenRaft without the overhead of actor message passing. All operations
//! are async methods that directly call into the Raft core.
//!
//! ## Architecture
//!
//! ```text
//! Client -> RaftNode -> OpenRaft
//! ```
//!
//! ## Tiger Style
//!
//! - Bounded resources: Semaphore limits concurrent operations
//! - Explicit error handling: All errors use snafu
//! - No unbounded growth: Fixed capacity for pending operations
//!
//! ## Test Coverage
//!
//! RaftNode has 33 unit tests covering:
//! - Node creation, accessors, and initialization checks
//! - ClusterController validation (init, add_learner, change_membership)
//! - KeyValueStore validation (write key/value size limits)
//! - ReadConsistency routing (Linearizable, Lease, Stale)
//! - Pre-initialization error handling for all operations

mod arc_wrapper;
mod cluster_controller;
mod conversions;
mod health;
mod kv_store;
#[cfg(feature = "sql")]
mod sql;
#[cfg(all(test, feature = "testing"))]
mod tests;

// Re-exports for backward compatibility
use std::sync::Arc;
#[cfg(feature = "sql")]
use std::sync::OnceLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

pub use arc_wrapper::ArcRaftNode;
use aspen_cluster_types::ClusterNode;
use aspen_cluster_types::ClusterState;
use aspen_cluster_types::ControlPlaneError;
use aspen_kv_types::KeyValueStoreError;
pub use health::HealthStatus;
pub use health::RaftNodeHealth;
use openraft::Raft;
use tokio::sync::Semaphore;

use crate::StateMachineVariant;
use crate::types::AppTypeConfig;
use crate::types::NodeId;
use crate::write_batcher::BatchConfig;
use crate::write_batcher::WriteBatcher;

/// Maximum concurrent operations (prevents resource exhaustion).
const MAX_CONCURRENT_OPS: usize = 1000;

/// Direct Raft node wrapper.
///
/// Provides both ClusterController and KeyValueStore functionality
/// without the overhead of actor message passing.
pub struct RaftNode {
    /// The OpenRaft instance.
    raft: Arc<Raft<AppTypeConfig>>,

    /// Node ID.
    node_id: NodeId,

    /// State machine (for direct KV operations).
    state_machine: StateMachineVariant,

    /// Whether the cluster has been initialized (atomic for race-free updates).
    ///
    /// Tiger Style: Uses atomic boolean to prevent TOCTOU race condition where
    /// multiple concurrent calls could read false, check metrics, then all write true.
    ///
    /// Wrapped in Arc to allow sharing with the membership watcher, which can
    /// proactively set this flag when membership is received via Raft replication.
    initialized: Arc<AtomicBool>,

    /// Semaphore to limit concurrent operations.
    semaphore: Arc<Semaphore>,

    /// Cached SQL executor for Redb state machines.
    ///
    /// Lazily initialized on first SQL query to avoid startup overhead.
    /// Caches the DataFusion SessionContext for ~400us savings per query.
    #[cfg(feature = "sql")]
    sql_executor: OnceLock<aspen_sql::RedbSqlExecutor>,

    /// Optional write batcher for high-throughput workloads.
    ///
    /// When enabled, batches Set and Delete operations into single Raft proposals
    /// to amortize consensus and fsync costs. Provides ~10x throughput improvement
    /// at the cost of ~2ms added latency per write.
    write_batcher: Option<Arc<WriteBatcher>>,
}

impl RaftNode {
    /// Create a new Raft node without write batching.
    pub fn new(node_id: NodeId, raft: Arc<Raft<AppTypeConfig>>, state_machine: StateMachineVariant) -> Self {
        Self {
            raft,
            node_id,
            state_machine,
            initialized: Arc::new(AtomicBool::new(false)),
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_OPS)),
            #[cfg(feature = "sql")]
            sql_executor: OnceLock::new(),
            write_batcher: None,
        }
    }

    /// Create a new Raft node with write batching enabled.
    ///
    /// Write batching groups multiple Set/Delete operations into single Raft
    /// proposals, amortizing consensus and fsync costs for ~10x throughput
    /// improvement at the cost of ~2ms added latency.
    ///
    /// ## Performance
    ///
    /// | Config | Throughput | Added Latency |
    /// |--------|------------|---------------|
    /// | default | ~10x | ~2ms |
    /// | low_latency | ~3x | ~1ms |
    /// | high_throughput | ~30x | ~5ms |
    pub fn with_write_batching(
        node_id: NodeId,
        raft: Arc<Raft<AppTypeConfig>>,
        state_machine: StateMachineVariant,
        batch_config: BatchConfig,
    ) -> Self {
        let write_batcher = WriteBatcher::new_shared(raft.clone(), batch_config);
        Self {
            raft,
            node_id,
            state_machine,
            initialized: Arc::new(AtomicBool::new(false)),
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_OPS)),
            #[cfg(feature = "sql")]
            sql_executor: OnceLock::new(),
            write_batcher: Some(write_batcher),
        }
    }

    /// Check if write batching is enabled.
    pub fn is_batching_enabled(&self) -> bool {
        self.write_batcher.is_some()
    }

    /// Get the underlying Raft instance.
    pub fn raft(&self) -> &Arc<Raft<AppTypeConfig>> {
        &self.raft
    }

    /// Get the node ID.
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Get the state machine.
    pub fn state_machine(&self) -> &StateMachineVariant {
        &self.state_machine
    }

    /// Check if the cluster is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }

    /// Get a shared reference to the initialized flag for use with membership watcher.
    ///
    /// This allows the membership watcher to proactively set the initialized flag
    /// when membership is received via Raft replication, eliminating the race
    /// condition where CLI queries arrive before the first KV operation.
    ///
    /// The returned Arc shares the same AtomicBool as the node, so any updates
    /// made through this Arc are immediately visible to the node.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let node = Arc::new(RaftNode::new(...));
    /// let cancel = spawn_membership_watcher_with_init(
    ///     node.raft().clone(),
    ///     trusted_peers.clone(),
    ///     Some(node.initialized_flag()),
    /// );
    /// ```
    pub fn initialized_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.initialized)
    }

    /// Internal: Get semaphore reference for trait impl modules.
    pub(crate) fn semaphore(&self) -> &Arc<Semaphore> {
        &self.semaphore
    }

    /// Internal: Get initialized flag reference for trait impl modules.
    pub(crate) fn initialized_ref(&self) -> &Arc<AtomicBool> {
        &self.initialized
    }

    /// Internal: Get write batcher reference for KV store.
    pub(crate) fn write_batcher(&self) -> Option<&Arc<WriteBatcher>> {
        self.write_batcher.as_ref()
    }

    #[cfg(feature = "sql")]
    pub(crate) fn sql_executor(&self) -> &OnceLock<aspen_sql::RedbSqlExecutor> {
        &self.sql_executor
    }

    #[cfg(all(test, feature = "testing"))]
    pub(crate) fn set_initialized_for_test(&self, value: bool) {
        self.initialized.store(value, Ordering::Release);
    }

    /// Ensure the cluster is initialized.
    pub(crate) fn ensure_initialized(&self) -> Result<(), ControlPlaneError> {
        if !self.initialized.load(Ordering::Acquire) {
            return Err(ControlPlaneError::NotInitialized);
        }
        Ok(())
    }

    /// Ensure the cluster is initialized for KV operations.
    ///
    /// A node is considered initialized if:
    /// 1. init() was called on this node directly, OR
    /// 2. The node has received membership info through Raft replication
    ///
    /// Tiger Style: Uses atomic compare_exchange to prevent TOCTOU race where
    /// multiple concurrent calls could all read false, check metrics, then
    /// all try to set true. With CAS, only one succeeds in the transition.
    pub(crate) fn ensure_initialized_kv(&self) -> Result<(), KeyValueStoreError> {
        // Fast path: check if already initialized (Acquire ensures we see prior writes)
        if self.initialized.load(Ordering::Acquire) {
            return Ok(());
        }

        // Slow path: check Raft membership and atomically set initialized
        // This handles nodes that join via replication rather than explicit init
        let metrics = self.raft.metrics().borrow().clone();
        if metrics.membership_config.membership().nodes().next().is_some() {
            // Atomically transition from false to true (only one thread wins)
            // We don't care if we lose the race - another thread already set it
            let _ = self.initialized.compare_exchange(
                false,
                true,
                Ordering::Release, // Ensure membership check happens-before this store
                Ordering::Relaxed, // On failure, we don't need to see the current value
            );
            return Ok(());
        }

        Err(KeyValueStoreError::Failed {
            reason: "cluster not initialized".into(),
        })
    }

    /// Build ClusterState from Raft metrics.
    pub(crate) fn build_cluster_state(&self) -> ClusterState {
        let metrics = self.raft.metrics().borrow().clone();
        let membership = &metrics.membership_config;

        let mut nodes = Vec::new();
        let mut learners = Vec::new();
        let mut members = Vec::new();

        let voter_ids: std::collections::HashSet<NodeId> = membership.membership().voter_ids().collect();

        for (node_id, member_info) in membership.membership().nodes() {
            let cluster_node = ClusterNode::with_iroh_addr((*node_id).into(), member_info.iroh_addr.clone());

            if voter_ids.contains(node_id) {
                members.push((*node_id).into());
                nodes.push(cluster_node);
            } else {
                learners.push(cluster_node);
            }
        }

        ClusterState {
            nodes,
            members,
            learners,
        }
    }
}
