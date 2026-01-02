//! Direct Raft node wrapper without actors.
//!
//! This module provides a simplified Raft node implementation that directly
//! wraps OpenRaft without the overhead of actor message passing. All operations
//! are async methods that directly call into the Raft core.
//!
//! ## Architecture
//!
//! Instead of:
//! ```text
//! Client -> ActorRef -> Message -> RaftActor -> OpenRaft
//! ```
//!
//! We have:
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
//! TODO: Add unit tests for RaftNode trait implementations:
//!       - ClusterController::init() with various initial member configurations
//!       - ClusterController::add_learner() and promotion to voter
//!       - ClusterController::change_membership() transitions
//!       - KeyValueStore::write() with all WriteCommand variants
//!       - KeyValueStore::read() linearizability via ReadPolicy
//!       - KeyValueStore::scan() pagination boundary testing
//!       - Semaphore limiting (MAX_CONCURRENT_OPS) under load
//!       Coverage: 0% line coverage (tested via router tests and direct API tests)

use std::collections::BTreeMap;
use std::sync::Arc;
#[cfg(feature = "sql")]
use std::sync::OnceLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use async_trait::async_trait;
use openraft::Raft;
use openraft::ReadPolicy;
use tokio::sync::Semaphore;
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use aspen_constants::{MEMBERSHIP_OPERATION_TIMEOUT, READ_INDEX_TIMEOUT};
use aspen_core::AddLearnerRequest;
use aspen_core::ChangeMembershipRequest;
use aspen_core::ClusterController;
use aspen_core::ClusterMetrics;
use aspen_core::CoordinationBackend;
use aspen_core::ClusterNode;
use aspen_core::ClusterState;
use aspen_core::ControlPlaneError;
use aspen_core::NodeState;
use aspen_core::DEFAULT_SCAN_LIMIT;
use aspen_core::SnapshotLogId;
use aspen_core::DeleteRequest;
use aspen_core::DeleteResult;
use aspen_core::InitRequest;
use aspen_core::KeyValueStore;
use aspen_core::KeyValueStoreError;
use aspen_core::KeyValueWithRevision;
use aspen_core::MAX_SCAN_RESULTS;
use aspen_core::ReadConsistency;
use aspen_core::ReadRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteOp;
use aspen_core::ReadResult;
use aspen_core::ScanRequest;
use aspen_core::ScanResult;
#[cfg(feature = "sql")]
use aspen_core::SqlConsistency;
#[cfg(feature = "sql")]
use aspen_core::SqlQueryError;
#[cfg(feature = "sql")]
use aspen_core::SqlQueryExecutor;
#[cfg(feature = "sql")]
use aspen_core::SqlQueryRequest;
#[cfg(feature = "sql")]
use aspen_core::SqlQueryResult;
use aspen_core::WriteRequest;
use aspen_core::WriteResult;
#[cfg(feature = "sql")]
use aspen_core::validate_sql_query;
#[cfg(feature = "sql")]
use aspen_core::validate_sql_request;
use aspen_core::validate_write_command;
use crate::StateMachineVariant;
use crate::types::AppTypeConfig;
use crate::types::NodeId;
use crate::types::RaftMemberInfo;
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
    initialized: AtomicBool,

    /// Semaphore to limit concurrent operations.
    semaphore: Arc<Semaphore>,

    /// Cached SQL executor for Redb state machines.
    ///
    /// Lazily initialized on first SQL query to avoid startup overhead.
    /// Caches the DataFusion SessionContext for ~400µs savings per query.
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
            initialized: AtomicBool::new(false),
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
            initialized: AtomicBool::new(false),
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

    /// Ensure the cluster is initialized.
    fn ensure_initialized(&self) -> Result<(), ControlPlaneError> {
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
    fn ensure_initialized_kv(&self) -> Result<(), KeyValueStoreError> {
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
    fn build_cluster_state(&self) -> ClusterState {
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

#[async_trait]
impl ClusterController for RaftNode {
    #[instrument(skip(self))]
    async fn init(&self, request: InitRequest) -> Result<ClusterState, ControlPlaneError> {
        // Acquire permit to limit concurrency
        let _permit = self.semaphore.acquire().await.map_err(|_| ControlPlaneError::Failed {
            reason: "semaphore closed".into(),
        })?;

        if request.initial_members.is_empty() {
            return Err(ControlPlaneError::InvalidRequest {
                reason: "initial_members must not be empty".into(),
            });
        }

        // Build RaftMemberInfo map
        let mut nodes: BTreeMap<NodeId, RaftMemberInfo> = BTreeMap::new();
        for cluster_node in &request.initial_members {
            let iroh_addr = cluster_node.iroh_addr().ok_or_else(|| ControlPlaneError::InvalidRequest {
                reason: format!("node_addr must be set for node {}", cluster_node.id),
            })?;
            nodes.insert(cluster_node.id.into(), RaftMemberInfo::new(iroh_addr.clone()));
        }

        info!("calling raft.initialize() with {} nodes", nodes.len());
        // Tiger Style: Explicit timeout prevents indefinite hang if quorum unavailable
        tokio::time::timeout(MEMBERSHIP_OPERATION_TIMEOUT, self.raft.initialize(nodes))
            .await
            .map_err(|_| ControlPlaneError::Timeout {
                duration_ms: MEMBERSHIP_OPERATION_TIMEOUT.as_millis() as u64,
            })?
            .map_err(|err| ControlPlaneError::Failed {
                reason: err.to_string(),
            })?;
        info!("raft.initialize() completed successfully");

        self.initialized.store(true, Ordering::Release);
        info!("initialized flag set to true");

        Ok(self.build_cluster_state())
    }

    #[instrument(skip(self))]
    async fn add_learner(&self, request: AddLearnerRequest) -> Result<ClusterState, ControlPlaneError> {
        let _permit = self.semaphore.acquire().await.map_err(|_| ControlPlaneError::Failed {
            reason: "semaphore closed".into(),
        })?;

        self.ensure_initialized()?;

        let learner = request.learner;
        let iroh_addr = learner.iroh_addr().ok_or_else(|| ControlPlaneError::InvalidRequest {
            reason: format!("node_addr must be set for node {}", learner.id),
        })?;

        let node = RaftMemberInfo::new(iroh_addr.clone());

        info!(
            learner_id = learner.id,
            endpoint_id = %iroh_addr.id,
            "adding learner with Iroh address"
        );

        // Tiger Style: Explicit timeout prevents indefinite hang if leader unavailable
        tokio::time::timeout(
            MEMBERSHIP_OPERATION_TIMEOUT,
            self.raft.add_learner(learner.id.into(), node, true),
        )
        .await
        .map_err(|_| ControlPlaneError::Timeout {
            duration_ms: MEMBERSHIP_OPERATION_TIMEOUT.as_millis() as u64,
        })?
        .map_err(|err| ControlPlaneError::Failed {
            reason: err.to_string(),
        })?;

        Ok(self.build_cluster_state())
    }

    #[instrument(skip(self))]
    async fn change_membership(&self, request: ChangeMembershipRequest) -> Result<ClusterState, ControlPlaneError> {
        let _permit = self.semaphore.acquire().await.map_err(|_| ControlPlaneError::Failed {
            reason: "semaphore closed".into(),
        })?;

        self.ensure_initialized()?;

        if request.members.is_empty() {
            return Err(ControlPlaneError::InvalidRequest {
                reason: "members must include at least one voter".into(),
            });
        }

        let members: std::collections::BTreeSet<NodeId> = request.members.iter().map(|&id| id.into()).collect();

        // Tiger Style: Explicit timeout prevents indefinite hang if quorum unavailable
        tokio::time::timeout(
            MEMBERSHIP_OPERATION_TIMEOUT,
            self.raft.change_membership(members, false),
        )
        .await
        .map_err(|_| ControlPlaneError::Timeout {
            duration_ms: MEMBERSHIP_OPERATION_TIMEOUT.as_millis() as u64,
        })?
        .map_err(|err| ControlPlaneError::Failed {
            reason: err.to_string(),
        })?;

        Ok(self.build_cluster_state())
    }

    #[instrument(skip(self))]
    async fn current_state(&self) -> Result<ClusterState, ControlPlaneError> {
        self.ensure_initialized()?;
        Ok(self.build_cluster_state())
    }

    #[instrument(skip(self))]
    async fn get_leader(&self) -> Result<Option<u64>, ControlPlaneError> {
        self.ensure_initialized()?;
        let metrics = self.raft.metrics().borrow().clone();
        Ok(metrics.current_leader.map(|id| id.0))
    }

    #[instrument(skip(self))]
    async fn get_metrics(&self) -> Result<ClusterMetrics, ControlPlaneError> {
        self.ensure_initialized()?;
        let metrics = self.raft.metrics().borrow().clone();
        Ok(cluster_metrics_from_openraft(&metrics))
    }

    #[instrument(skip(self))]
    async fn trigger_snapshot(&self) -> Result<Option<SnapshotLogId>, ControlPlaneError> {
        self.ensure_initialized()?;

        // Trigger a snapshot (returns () on success)
        self.raft.trigger().snapshot().await.map_err(|err| ControlPlaneError::Failed {
            reason: err.to_string(),
        })?;

        // Get the current snapshot from metrics and convert to wrapper type
        let metrics = self.raft.metrics().borrow().clone();
        Ok(metrics.snapshot.as_ref().map(snapshot_log_id_from_openraft))
    }

    fn is_initialized(&self) -> bool {
        // Fast path: check atomic flag (Acquire ensures we see prior writes)
        if self.initialized.load(Ordering::Acquire) {
            return true;
        }

        // Slow path: check if membership exists via Raft replication
        // A node may have received membership through replication without explicit init()
        let metrics = self.raft.metrics().borrow().clone();
        metrics.membership_config.membership().nodes().next().is_some()
    }
}

#[async_trait]
impl KeyValueStore for RaftNode {
    #[instrument(skip(self))]
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        let _permit = self.semaphore.acquire().await.map_err(|_| KeyValueStoreError::Failed {
            reason: "semaphore closed".into(),
        })?;

        self.ensure_initialized_kv()?;

        validate_write_command(&request.command)?;

        // Route simple Set/Delete through batcher if enabled
        if let Some(ref batcher) = self.write_batcher {
            match &request.command {
                WriteCommand::Set { .. } | WriteCommand::Delete { .. } => {
                    return batcher.write_shared(request.command).await;
                }
                // Complex operations bypass batcher for correctness
                _ => {}
            }
        }

        // Convert WriteRequest to AppRequest (direct Raft path)
        use aspen_core::BatchCondition;
        use aspen_core::BatchOperation;
        use crate::types::AppRequest;
        let app_request = match &request.command {
            WriteCommand::Set { key, value } => AppRequest::Set {
                key: key.clone(),
                value: value.clone(),
            },
            WriteCommand::SetWithTTL {
                key,
                value,
                ttl_seconds,
            } => {
                // Convert TTL in seconds to absolute expiration timestamp in milliseconds
                let now_ms =
                    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()
                        as u64;
                let expires_at_ms = now_ms + (*ttl_seconds as u64 * 1000);
                AppRequest::SetWithTTL {
                    key: key.clone(),
                    value: value.clone(),
                    expires_at_ms,
                }
            }
            WriteCommand::SetMulti { pairs } => AppRequest::SetMulti { pairs: pairs.clone() },
            WriteCommand::SetMultiWithTTL { pairs, ttl_seconds } => {
                let now_ms =
                    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()
                        as u64;
                let expires_at_ms = now_ms + (*ttl_seconds as u64 * 1000);
                AppRequest::SetMultiWithTTL {
                    pairs: pairs.clone(),
                    expires_at_ms,
                }
            }
            WriteCommand::Delete { key } => AppRequest::Delete { key: key.clone() },
            WriteCommand::DeleteMulti { keys } => AppRequest::DeleteMulti { keys: keys.clone() },
            WriteCommand::CompareAndSwap {
                key,
                expected,
                new_value,
            } => AppRequest::CompareAndSwap {
                key: key.clone(),
                expected: expected.clone(),
                new_value: new_value.clone(),
            },
            WriteCommand::CompareAndDelete { key, expected } => AppRequest::CompareAndDelete {
                key: key.clone(),
                expected: expected.clone(),
            },
            WriteCommand::Batch { operations } => {
                // Convert to compact tuple format: (is_set, key, value)
                let ops: Vec<(bool, String, String)> = operations
                    .iter()
                    .map(|op| match op {
                        BatchOperation::Set { key, value } => (true, key.clone(), value.clone()),
                        BatchOperation::Delete { key } => (false, key.clone(), String::new()),
                    })
                    .collect();
                AppRequest::Batch { operations: ops }
            }
            WriteCommand::ConditionalBatch { conditions, operations } => {
                // Convert conditions to compact tuple format: (type, key, expected)
                // Types: 0=ValueEquals, 1=KeyExists, 2=KeyNotExists
                let conds: Vec<(u8, String, String)> = conditions
                    .iter()
                    .map(|c| match c {
                        BatchCondition::ValueEquals { key, expected } => (0, key.clone(), expected.clone()),
                        BatchCondition::KeyExists { key } => (1, key.clone(), String::new()),
                        BatchCondition::KeyNotExists { key } => (2, key.clone(), String::new()),
                    })
                    .collect();
                // Convert operations to compact tuple format
                let ops: Vec<(bool, String, String)> = operations
                    .iter()
                    .map(|op| match op {
                        BatchOperation::Set { key, value } => (true, key.clone(), value.clone()),
                        BatchOperation::Delete { key } => (false, key.clone(), String::new()),
                    })
                    .collect();
                AppRequest::ConditionalBatch {
                    conditions: conds,
                    operations: ops,
                }
            }
            // Lease operations
            WriteCommand::SetWithLease { key, value, lease_id } => AppRequest::SetWithLease {
                key: key.clone(),
                value: value.clone(),
                lease_id: *lease_id,
            },
            WriteCommand::SetMultiWithLease { pairs, lease_id } => AppRequest::SetMultiWithLease {
                pairs: pairs.clone(),
                lease_id: *lease_id,
            },
            WriteCommand::LeaseGrant { lease_id, ttl_seconds } => AppRequest::LeaseGrant {
                lease_id: *lease_id,
                ttl_seconds: *ttl_seconds,
            },
            WriteCommand::LeaseRevoke { lease_id } => AppRequest::LeaseRevoke { lease_id: *lease_id },
            WriteCommand::LeaseKeepalive { lease_id } => AppRequest::LeaseKeepalive { lease_id: *lease_id },
            WriteCommand::Transaction {
                compare,
                success,
                failure,
            } => {
                use aspen_core::CompareOp;
                use aspen_core::CompareTarget;
                use aspen_core::TxnOp;

                // Convert compare conditions to compact format:
                // target: 0=Value, 1=Version, 2=CreateRevision, 3=ModRevision
                // op: 0=Equal, 1=NotEqual, 2=Greater, 3=Less
                let cmp: Vec<(u8, u8, String, String)> = compare
                    .iter()
                    .map(|c| {
                        let target = match c.target {
                            CompareTarget::Value => 0,
                            CompareTarget::Version => 1,
                            CompareTarget::CreateRevision => 2,
                            CompareTarget::ModRevision => 3,
                        };
                        let op = match c.op {
                            CompareOp::Equal => 0,
                            CompareOp::NotEqual => 1,
                            CompareOp::Greater => 2,
                            CompareOp::Less => 3,
                        };
                        (target, op, c.key.clone(), c.value.clone())
                    })
                    .collect();

                // Convert operations to compact format:
                // op_type: 0=Put, 1=Delete, 2=Get, 3=Range
                let convert_ops = |ops: &[TxnOp]| -> Vec<(u8, String, String)> {
                    ops.iter()
                        .map(|op| match op {
                            TxnOp::Put { key, value } => (0, key.clone(), value.clone()),
                            TxnOp::Delete { key } => (1, key.clone(), String::new()),
                            TxnOp::Get { key } => (2, key.clone(), String::new()),
                            TxnOp::Range { prefix, limit } => (3, prefix.clone(), limit.to_string()),
                        })
                        .collect()
                };

                AppRequest::Transaction {
                    compare: cmp,
                    success: convert_ops(success),
                    failure: convert_ops(failure),
                }
            }
            WriteCommand::OptimisticTransaction { read_set, write_set } => {
                // Convert WriteOp to compact tuple format: (is_set, key, value)
                let write_ops: Vec<(bool, String, String)> = write_set
                    .iter()
                    .map(|op| match op {
                        WriteOp::Set { key, value } => (true, key.clone(), value.clone()),
                        WriteOp::Delete { key } => (false, key.clone(), String::new()),
                    })
                    .collect();
                AppRequest::OptimisticTransaction {
                    read_set: read_set.clone(),
                    write_set: write_ops,
                }
            }
            // Shard topology operations
            WriteCommand::ShardSplit {
                source_shard,
                split_key,
                new_shard_id,
                topology_version,
            } => AppRequest::ShardSplit {
                source_shard: *source_shard,
                split_key: split_key.clone(),
                new_shard_id: *new_shard_id,
                topology_version: *topology_version,
            },
            WriteCommand::ShardMerge {
                source_shard,
                target_shard,
                topology_version,
            } => AppRequest::ShardMerge {
                source_shard: *source_shard,
                target_shard: *target_shard,
                topology_version: *topology_version,
            },
            WriteCommand::TopologyUpdate { topology_data } => AppRequest::TopologyUpdate {
                topology_data: topology_data.clone(),
            },
        };

        // Apply write through Raft consensus
        let result = self.raft.client_write(app_request).await;

        match result {
            Ok(resp) => {
                // Check if this was a CAS operation that failed its condition
                if let Some(false) = resp.data.cas_succeeded {
                    // CAS condition didn't match - extract key and expected from original command
                    let (key, expected) = match &request.command {
                        WriteCommand::CompareAndSwap { key, expected, .. } => {
                            (key.clone(), expected.clone())
                        }
                        WriteCommand::CompareAndDelete { key, expected } => {
                            (key.clone(), Some(expected.clone()))
                        }
                        _ => unreachable!("cas_succeeded only set for CAS operations"),
                    };
                    return Err(KeyValueStoreError::CompareAndSwapFailed {
                        key,
                        expected,
                        actual: resp.data.value,
                    });
                }
                // Build WriteResult with appropriate fields based on operation type
                Ok(WriteResult {
                    command: Some(request.command),
                    batch_applied: resp.data.batch_applied,
                    conditions_met: resp.data.conditions_met,
                    failed_condition_index: resp.data.failed_condition_index,
                    lease_id: resp.data.lease_id,
                    ttl_seconds: resp.data.ttl_seconds,
                    keys_deleted: resp.data.keys_deleted,
                    succeeded: resp.data.succeeded,
                    txn_results: resp.data.txn_results,
                    header_revision: resp.data.header_revision,
                    occ_conflict: resp.data.occ_conflict,
                    conflict_key: resp.data.conflict_key,
                    conflict_expected_version: resp.data.conflict_expected_version,
                    conflict_actual_version: resp.data.conflict_actual_version,
                })
            }
            Err(err) => Err(KeyValueStoreError::Failed {
                reason: err.to_string(),
            }),
        }
    }

    #[instrument(skip(self))]
    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        let _permit = self.semaphore.acquire().await.map_err(|_| KeyValueStoreError::Failed {
            reason: "semaphore closed".into(),
        })?;

        self.ensure_initialized_kv()?;

        // Apply consistency level based on request
        match request.consistency {
            ReadConsistency::Linearizable => {
                // ReadIndex: Strongest consistency via quorum confirmation
                //
                // ReadIndex works by:
                // 1. Leader records its current commit index
                // 2. Leader confirms it's still leader via heartbeat quorum
                // 3. await_ready() waits for our state machine to catch up to that commit index
                //
                // This guarantees linearizability because any read after await_ready()
                // sees all writes committed before get_read_linearizer() was called.
                let linearizer = self.raft.get_read_linearizer(ReadPolicy::ReadIndex).await.map_err(|err| {
                    let leader_hint = self.raft.metrics().borrow().current_leader.map(|id| id.0);
                    KeyValueStoreError::NotLeader {
                        leader: leader_hint,
                        reason: err.to_string(),
                    }
                })?;

                // Tiger Style: Explicit timeout prevents indefinite hang during network partition
                tokio::time::timeout(READ_INDEX_TIMEOUT, linearizer.await_ready(&self.raft))
                    .await
                    .map_err(|_| KeyValueStoreError::Timeout {
                        duration_ms: READ_INDEX_TIMEOUT.as_millis() as u64,
                    })?
                    .map_err(|err| {
                        let leader_hint = self.raft.metrics().borrow().current_leader.map(|id| id.0);
                        KeyValueStoreError::NotLeader {
                            leader: leader_hint,
                            reason: err.to_string(),
                        }
                    })?;
            }
            ReadConsistency::Lease => {
                // LeaseRead: Lower latency via leader lease (no quorum confirmation)
                //
                // Uses the leader's lease to serve reads without contacting followers.
                // Safe as long as clock drift is less than the lease duration.
                // Falls back to ReadIndex if the lease has expired.
                let linearizer = self.raft.get_read_linearizer(ReadPolicy::LeaseRead).await.map_err(|err| {
                    let leader_hint = self.raft.metrics().borrow().current_leader.map(|id| id.0);
                    KeyValueStoreError::NotLeader {
                        leader: leader_hint,
                        reason: err.to_string(),
                    }
                })?;

                // Tiger Style: Explicit timeout prevents indefinite hang during network partition
                tokio::time::timeout(READ_INDEX_TIMEOUT, linearizer.await_ready(&self.raft))
                    .await
                    .map_err(|_| KeyValueStoreError::Timeout {
                        duration_ms: READ_INDEX_TIMEOUT.as_millis() as u64,
                    })?
                    .map_err(|err| {
                        let leader_hint = self.raft.metrics().borrow().current_leader.map(|id| id.0);
                        KeyValueStoreError::NotLeader {
                            leader: leader_hint,
                            reason: err.to_string(),
                        }
                    })?;
            }
            ReadConsistency::Stale => {
                // Stale: Read directly from local state machine without consistency checks
                // WARNING: May return uncommitted or rolled-back data
            }
        }

        // Read directly from state machine (linearizability guaranteed by ReadIndex above)
        match &self.state_machine {
            StateMachineVariant::InMemory(sm) => match sm.get(&request.key).await {
                Some(value) => Ok(ReadResult {
                    kv: Some(KeyValueWithRevision {
                        key: request.key,
                        value,
                        version: 1,         // In-memory doesn't track versions
                        create_revision: 0, // In-memory doesn't track revisions
                        mod_revision: 0,
                    }),
                }),
                None => Err(KeyValueStoreError::NotFound { key: request.key }),
            },
            StateMachineVariant::Redb(sm) => match sm.get(&request.key) {
                Ok(Some(entry)) => Ok(ReadResult {
                    kv: Some(KeyValueWithRevision {
                        key: request.key,
                        value: entry.value,
                        version: entry.version as u64,
                        create_revision: entry.create_revision as u64,
                        mod_revision: entry.mod_revision as u64,
                    }),
                }),
                Ok(None) => Err(KeyValueStoreError::NotFound { key: request.key }),
                Err(err) => Err(KeyValueStoreError::Failed {
                    reason: err.to_string(),
                }),
            },
        }
    }

    #[instrument(skip(self))]
    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
        let _permit = self.semaphore.acquire().await.map_err(|_| KeyValueStoreError::Failed {
            reason: "semaphore closed".into(),
        })?;

        self.ensure_initialized_kv()?;

        // Apply delete through Raft consensus
        use crate::types::AppRequest;
        let app_request = AppRequest::Delete {
            key: request.key.clone(),
        };

        let result = self.raft.client_write(app_request).await;

        match result {
            Ok(resp) => {
                let deleted = resp.data.deleted.unwrap_or(false);
                Ok(DeleteResult {
                    key: request.key,
                    deleted,
                })
            }
            Err(err) => Err(KeyValueStoreError::Failed {
                reason: err.to_string(),
            }),
        }
    }

    #[instrument(skip(self))]
    async fn scan(&self, _request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        let _permit = self.semaphore.acquire().await.map_err(|_| KeyValueStoreError::Failed {
            reason: "semaphore closed".into(),
        })?;

        self.ensure_initialized_kv()?;

        // Use ReadIndex for linearizable scan (see read() for protocol details)
        let linearizer = self.raft.get_read_linearizer(ReadPolicy::ReadIndex).await.map_err(|err| {
            let leader_hint = self.raft.metrics().borrow().current_leader.map(|id| id.0);
            KeyValueStoreError::NotLeader {
                leader: leader_hint,
                reason: err.to_string(),
            }
        })?;

        // Tiger Style: Explicit timeout prevents indefinite hang during network partition
        tokio::time::timeout(READ_INDEX_TIMEOUT, linearizer.await_ready(&self.raft))
            .await
            .map_err(|_| KeyValueStoreError::Timeout {
                duration_ms: READ_INDEX_TIMEOUT.as_millis() as u64,
            })?
            .map_err(|err| {
                let leader_hint = self.raft.metrics().borrow().current_leader.map(|id| id.0);
                KeyValueStoreError::NotLeader {
                    leader: leader_hint,
                    reason: err.to_string(),
                }
            })?;

        // Scan directly from state machine (linearizability guaranteed by ReadIndex above)
        // Apply default limit if not specified
        let limit = _request.limit.unwrap_or(DEFAULT_SCAN_LIMIT).min(MAX_SCAN_RESULTS) as usize;

        match &self.state_machine {
            StateMachineVariant::InMemory(sm) => {
                // Get all KV pairs matching prefix
                let all_pairs = sm.scan_kv_with_prefix_async(&_request.prefix).await;

                // Handle pagination via continuation token
                //
                // Tiger Style: Use >= comparison and skip the exact token key.
                // This handles the edge case where the continuation token key was
                // deleted between paginated calls - we still return all keys after it.
                // Using only > would skip entries if the token key no longer exists.
                let start_key = _request.continuation_token.as_deref();
                let filtered: Vec<_> = all_pairs
                    .into_iter()
                    .filter(|(k, _)| {
                        // Skip keys before or equal to continuation token
                        start_key.is_none_or(|start| k.as_str() > start)
                    })
                    .collect();

                // Take limit+1 to check if there are more results
                let is_truncated = filtered.len() > limit;
                let entries: Vec<KeyValueWithRevision> = filtered
                    .into_iter()
                    .take(limit)
                    .map(|(key, value)| KeyValueWithRevision {
                        key,
                        value,
                        version: 1,         // In-memory doesn't track versions
                        create_revision: 0, // In-memory doesn't track revisions
                        mod_revision: 0,
                    })
                    .collect();

                let continuation_token = if is_truncated {
                    entries.last().map(|e| e.key.clone())
                } else {
                    None
                };

                Ok(ScanResult {
                    count: entries.len() as u32,
                    entries,
                    is_truncated,
                    continuation_token,
                })
            }
            StateMachineVariant::Redb(sm) => {
                // Redb scan with pagination
                let start_key = _request.continuation_token.as_deref();
                match sm.scan(&_request.prefix, start_key, Some(limit + 1)) {
                    Ok(entries_full) => {
                        let is_truncated = entries_full.len() > limit;
                        let entries: Vec<KeyValueWithRevision> = entries_full.into_iter().take(limit).collect();

                        let continuation_token = if is_truncated {
                            entries.last().map(|e| e.key.clone())
                        } else {
                            None
                        };

                        Ok(ScanResult {
                            count: entries.len() as u32,
                            entries,
                            is_truncated,
                            continuation_token,
                        })
                    }
                    Err(err) => Err(KeyValueStoreError::Failed {
                        reason: err.to_string(),
                    }),
                }
            }
        }
    }
}

#[cfg(feature = "sql")]
#[async_trait]
impl SqlQueryExecutor for RaftNode {
    #[instrument(skip(self))]
    async fn execute_sql(&self, request: SqlQueryRequest) -> Result<SqlQueryResult, SqlQueryError> {
        let _permit = self.semaphore.acquire().await.map_err(|_| SqlQueryError::ExecutionFailed {
            reason: "semaphore closed".into(),
        })?;

        // Validate request bounds (Tiger Style)
        validate_sql_request(&request)?;

        // Validate query is read-only
        validate_sql_query(&request.query)?;

        // For linearizable consistency, use ReadIndex protocol
        if request.consistency == SqlConsistency::Linearizable {
            let linearizer = self.raft.get_read_linearizer(ReadPolicy::ReadIndex).await.map_err(|_err| {
                let leader_hint = self.raft.metrics().borrow().current_leader.map(|id| id.0);
                SqlQueryError::NotLeader { leader: leader_hint }
            })?;

            // Tiger Style: Explicit timeout prevents indefinite hang during network partition
            tokio::time::timeout(READ_INDEX_TIMEOUT, linearizer.await_ready(&self.raft))
                .await
                .map_err(|_| SqlQueryError::Timeout {
                    duration_ms: READ_INDEX_TIMEOUT.as_millis() as u64,
                })?
                .map_err(|_err| {
                    let leader_hint = self.raft.metrics().borrow().current_leader.map(|id| id.0);
                    SqlQueryError::NotLeader { leader: leader_hint }
                })?;
        }

        // Execute query on state machine
        match &self.state_machine {
            StateMachineVariant::InMemory(_) => {
                // In-memory backend doesn't support SQL
                Err(SqlQueryError::NotSupported {
                    backend: "in-memory".into(),
                })
            }
            StateMachineVariant::Redb(sm) => {
                // Execute SQL on Redb state machine via DataFusion
                // Use cached executor for ~400µs savings per query
                let executor = self.sql_executor.get_or_init(|| sm.create_sql_executor());
                executor.execute(&request.query, &request.params, request.limit, request.timeout_ms).await
            }
        }
    }
}

#[async_trait]
impl CoordinationBackend for RaftNode {
    async fn now_unix_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    async fn node_id(&self) -> u64 {
        self.node_id.0
    }

    async fn is_leader(&self) -> bool {
        // Check if this node is the current leader
        let metrics = self.raft.metrics().borrow().clone();
        metrics.current_leader == Some(self.node_id)
    }

    fn kv_store(&self) -> Arc<dyn KeyValueStore> {
        // Return self as Arc<dyn KeyValueStore>
        // This requires the RaftNode to be wrapped in Arc externally
        unimplemented!("RaftNode must be wrapped in Arc externally for CoordinationBackend")
    }

    fn cluster_controller(&self) -> Arc<dyn ClusterController> {
        // Return self as Arc<dyn ClusterController>
        // This requires the RaftNode to be wrapped in Arc externally
        unimplemented!("RaftNode must be wrapped in Arc externally for CoordinationBackend")
    }
}

// Note: Arc<RaftNode> impl removed - can't implement foreign trait for Arc<T>
// Users should wrap RaftNode in a newtype if they need Arc<dyn CoordinationBackend>

/// Health monitor for RaftNode.
///
/// Provides periodic health checks without actor overhead.
/// Can be connected to a supervisor for automatic recovery actions.
pub struct RaftNodeHealth {
    node: Arc<RaftNode>,
    /// Consecutive failed health checks
    consecutive_failures: std::sync::atomic::AtomicU32,
    /// Threshold before triggering recovery actions
    failure_threshold: u32,
}

/// Health check result with detailed status.
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Whether the node is considered healthy overall.
    pub healthy: bool,
    /// Current Raft state (Leader, Follower, Candidate, Learner, Shutdown).
    pub state: NodeState,
    /// Current leader ID, if known.
    pub leader: Option<u64>,
    /// Number of consecutive health check failures.
    pub consecutive_failures: u32,
    /// Whether the node is in shutdown state.
    pub is_shutdown: bool,
    /// Whether the node has a committed membership.
    pub has_membership: bool,
}

impl RaftNodeHealth {
    /// Create a new health monitor.
    pub fn new(node: Arc<RaftNode>) -> Self {
        Self {
            node,
            consecutive_failures: std::sync::atomic::AtomicU32::new(0),
            failure_threshold: 3, // 3 consecutive failures triggers alert
        }
    }

    /// Create a health monitor with custom failure threshold.
    pub fn with_threshold(node: Arc<RaftNode>, threshold: u32) -> Self {
        Self {
            node,
            consecutive_failures: std::sync::atomic::AtomicU32::new(0),
            failure_threshold: threshold,
        }
    }

    /// Check if the node is healthy.
    pub async fn is_healthy(&self) -> bool {
        // Simple health check: can we get metrics and is there a state?
        let metrics = self.node.raft.metrics();
        let borrowed = metrics.borrow();
        // Check if the node is in any valid state (not just created)
        !matches!(&borrowed.state, openraft::ServerState::Shutdown)
    }

    /// Get detailed health status.
    pub async fn status(&self) -> HealthStatus {
        let metrics = self.node.raft.metrics();
        let borrowed = metrics.borrow();

        let state: NodeState = node_state_from_openraft(borrowed.state);
        let is_shutdown = !state.is_healthy();
        let leader = borrowed.current_leader.map(|id| id.0);
        let has_membership = borrowed.membership_config.membership().voter_ids().next().is_some();

        let consecutive_failures = self.consecutive_failures.load(std::sync::atomic::Ordering::Relaxed);

        HealthStatus {
            healthy: !is_shutdown && (has_membership || state == NodeState::Learner),
            state,
            leader,
            consecutive_failures,
            is_shutdown,
            has_membership,
        }
    }

    /// Reset the failure counter (call after recovery).
    pub fn reset_failures(&self) {
        self.consecutive_failures.store(0, std::sync::atomic::Ordering::Relaxed);
    }

    /// Run periodic health monitoring with optional supervisor callback.
    ///
    /// When the failure threshold is exceeded, the callback is invoked to
    /// allow the supervisor to take action (e.g., restart services).
    pub async fn monitor_with_callback<F>(&self, interval_secs: u64, mut on_failure: F)
    where
        F: FnMut(HealthStatus) + Send,
    {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));

        loop {
            interval.tick().await;

            let status = self.status().await;

            if !status.healthy {
                let failures = self.consecutive_failures.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;

                warn!(
                    node_id = %self.node.node_id,
                    consecutive_failures = failures,
                    state = ?status.state,
                    "node health check failed"
                );

                if failures >= self.failure_threshold {
                    error!(
                        node_id = %self.node.node_id,
                        failures = failures,
                        threshold = self.failure_threshold,
                        "health failure threshold exceeded, triggering callback"
                    );
                    on_failure(status);
                }
            } else {
                // Reset failure counter on successful check
                let prev_failures = self.consecutive_failures.swap(0, std::sync::atomic::Ordering::Relaxed);
                if prev_failures > 0 {
                    info!(
                        node_id = %self.node.node_id,
                        previous_failures = prev_failures,
                        "node recovered, resetting failure count"
                    );
                }
            }
        }
    }

    /// Run periodic health monitoring (simple version without callback).
    pub async fn monitor(&self, interval_secs: u64) {
        self.monitor_with_callback(interval_secs, |_| {
            // No-op callback - just log the failure
        })
        .await
    }
}

// ============================================================================
// OpenRaft Conversion Functions
// ============================================================================

/// Convert openraft::ServerState to NodeState.
fn node_state_from_openraft(state: openraft::ServerState) -> NodeState {
    match state {
        openraft::ServerState::Learner => NodeState::Learner,
        openraft::ServerState::Follower => NodeState::Follower,
        openraft::ServerState::Candidate => NodeState::Candidate,
        openraft::ServerState::Leader => NodeState::Leader,
        openraft::ServerState::Shutdown => NodeState::Shutdown,
    }
}

/// Create ClusterMetrics from openraft RaftMetrics.
fn cluster_metrics_from_openraft(
    metrics: &openraft::metrics::RaftMetrics<AppTypeConfig>,
) -> ClusterMetrics {
    let membership = metrics.membership_config.membership();
    ClusterMetrics {
        id: metrics.id.0,
        state: node_state_from_openraft(metrics.state),
        current_leader: metrics.current_leader.map(|id| id.0),
        current_term: metrics.current_term,
        last_log_index: metrics.last_log_index,
        last_applied_index: metrics.last_applied.as_ref().map(|la| la.index),
        snapshot_index: metrics.snapshot.as_ref().map(|s| s.index),
        replication: metrics.replication.as_ref().map(|repl_map| {
            repl_map
                .iter()
                .map(|(node_id, matched)| (node_id.0, matched.as_ref().map(|log_id| log_id.index)))
                .collect()
        }),
        voters: membership.voter_ids().map(|id| id.0).collect(),
        learners: membership.learner_ids().map(|id| id.0).collect(),
    }
}

/// Create SnapshotLogId from openraft LogId.
fn snapshot_log_id_from_openraft(
    log_id: &openraft::LogId<AppTypeConfig>,
) -> SnapshotLogId {
    SnapshotLogId {
        term: log_id.leader_id.term,
        index: log_id.index,
    }
}
