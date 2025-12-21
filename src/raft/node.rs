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
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use openraft::{Raft, RaftMetrics, ReadPolicy};
use tokio::sync::Semaphore;
use tracing::{error, info, instrument, warn};

use crate::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, ClusterNode, ClusterState,
    ControlPlaneError, DEFAULT_SCAN_LIMIT, DeleteRequest, DeleteResult, InitRequest, KeyValueStore,
    KeyValueStoreError, MAX_SCAN_RESULTS, ReadRequest, ReadResult, ScanEntry, ScanRequest,
    ScanResult, SqlConsistency, SqlQueryError, SqlQueryExecutor, SqlQueryRequest, SqlQueryResult,
    WriteRequest, WriteResult, sql_validation::validate_sql_query, validate_sql_request,
    validate_write_command,
};
use crate::raft::StateMachineVariant;
use crate::raft::types::{AppTypeConfig, NodeId, RaftMemberInfo};

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
}

impl RaftNode {
    /// Create a new Raft node.
    pub fn new(
        node_id: NodeId,
        raft: Arc<Raft<AppTypeConfig>>,
        state_machine: StateMachineVariant,
    ) -> Self {
        Self {
            raft,
            node_id,
            state_machine,
            initialized: AtomicBool::new(false),
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_OPS)),
        }
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
        if metrics
            .membership_config
            .membership()
            .nodes()
            .next()
            .is_some()
        {
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

        let voter_ids: std::collections::HashSet<NodeId> =
            membership.membership().voter_ids().collect();

        for (node_id, member_info) in membership.membership().nodes() {
            let cluster_node = ClusterNode {
                id: (*node_id).into(),
                addr: member_info.iroh_addr.id.to_string(),
                raft_addr: None,
                iroh_addr: Some(member_info.iroh_addr.clone()),
            };

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
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| ControlPlaneError::Failed {
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
            let iroh_addr = cluster_node.iroh_addr.as_ref().ok_or_else(|| {
                ControlPlaneError::InvalidRequest {
                    reason: format!("iroh_addr must be set for node {}", cluster_node.id),
                }
            })?;
            nodes.insert(
                cluster_node.id.into(),
                RaftMemberInfo::new(iroh_addr.clone()),
            );
        }

        info!("calling raft.initialize() with {} nodes", nodes.len());
        self.raft.initialize(nodes).await.map_err(|err| {
            error!("raft.initialize() failed: {:?}", err);
            ControlPlaneError::Failed {
                reason: err.to_string(),
            }
        })?;
        info!("raft.initialize() completed successfully");

        self.initialized.store(true, Ordering::Release);
        info!("initialized flag set to true");

        Ok(self.build_cluster_state())
    }

    #[instrument(skip(self))]
    async fn add_learner(
        &self,
        request: AddLearnerRequest,
    ) -> Result<ClusterState, ControlPlaneError> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| ControlPlaneError::Failed {
                reason: "semaphore closed".into(),
            })?;

        self.ensure_initialized()?;

        let learner = request.learner;
        let iroh_addr =
            learner
                .iroh_addr
                .as_ref()
                .ok_or_else(|| ControlPlaneError::InvalidRequest {
                    reason: format!("iroh_addr must be set for node {}", learner.id),
                })?;

        let node = RaftMemberInfo::new(iroh_addr.clone());

        info!(
            learner_id = learner.id,
            endpoint_id = %iroh_addr.id,
            "adding learner with Iroh address"
        );

        self.raft
            .add_learner(learner.id.into(), node, true)
            .await
            .map_err(|err| ControlPlaneError::Failed {
                reason: err.to_string(),
            })?;

        Ok(self.build_cluster_state())
    }

    #[instrument(skip(self))]
    async fn change_membership(
        &self,
        request: ChangeMembershipRequest,
    ) -> Result<ClusterState, ControlPlaneError> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| ControlPlaneError::Failed {
                reason: "semaphore closed".into(),
            })?;

        self.ensure_initialized()?;

        if request.members.is_empty() {
            return Err(ControlPlaneError::InvalidRequest {
                reason: "members must include at least one voter".into(),
            });
        }

        let members: std::collections::BTreeSet<NodeId> =
            request.members.iter().map(|&id| id.into()).collect();

        self.raft
            .change_membership(members, false)
            .await
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
    async fn get_metrics(&self) -> Result<RaftMetrics<AppTypeConfig>, ControlPlaneError> {
        self.ensure_initialized()?;
        Ok(self.raft.metrics().borrow().clone())
    }

    #[instrument(skip(self))]
    async fn trigger_snapshot(
        &self,
    ) -> Result<Option<openraft::LogId<AppTypeConfig>>, ControlPlaneError> {
        self.ensure_initialized()?;

        // Trigger a snapshot (returns () on success)
        self.raft
            .trigger()
            .snapshot()
            .await
            .map_err(|err| ControlPlaneError::Failed {
                reason: err.to_string(),
            })?;

        // Get the current snapshot from metrics
        let metrics = self.raft.metrics().borrow().clone();
        Ok(metrics.snapshot)
    }
}

#[async_trait]
impl KeyValueStore for RaftNode {
    #[instrument(skip(self))]
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| KeyValueStoreError::Failed {
                reason: "semaphore closed".into(),
            })?;

        self.ensure_initialized_kv()?;

        validate_write_command(&request.command)?;

        // Convert WriteRequest to AppRequest
        use crate::raft::types::AppRequest;
        let app_request = match &request.command {
            crate::api::WriteCommand::Set { key, value } => AppRequest::Set {
                key: key.clone(),
                value: value.clone(),
            },
            crate::api::WriteCommand::SetMulti { pairs } => AppRequest::SetMulti {
                pairs: pairs.clone(),
            },
            crate::api::WriteCommand::Delete { key } => AppRequest::Delete { key: key.clone() },
            crate::api::WriteCommand::DeleteMulti { keys } => {
                AppRequest::DeleteMulti { keys: keys.clone() }
            }
            crate::api::WriteCommand::CompareAndSwap {
                key,
                expected,
                new_value,
            } => AppRequest::CompareAndSwap {
                key: key.clone(),
                expected: expected.clone(),
                new_value: new_value.clone(),
            },
            crate::api::WriteCommand::CompareAndDelete { key, expected } => {
                AppRequest::CompareAndDelete {
                    key: key.clone(),
                    expected: expected.clone(),
                }
            }
        };

        // Apply write through Raft consensus
        let result = self.raft.client_write(app_request).await;

        match result {
            Ok(resp) => {
                // Check if this was a CAS operation that failed its condition
                if let Some(false) = resp.data.cas_succeeded {
                    // CAS condition didn't match - extract key and expected from original command
                    let (key, expected) = match &request.command {
                        crate::api::WriteCommand::CompareAndSwap { key, expected, .. } => {
                            (key.clone(), expected.clone())
                        }
                        crate::api::WriteCommand::CompareAndDelete { key, expected } => {
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
                // Write was successful, return the original command
                Ok(WriteResult {
                    command: request.command,
                })
            }
            Err(err) => {
                warn!(error = %err, "write operation failed");
                Err(KeyValueStoreError::Failed {
                    reason: err.to_string(),
                })
            }
        }
    }

    #[instrument(skip(self))]
    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| KeyValueStoreError::Failed {
                reason: "semaphore closed".into(),
            })?;

        self.ensure_initialized_kv()?;

        // Enforce linearizable reads via ReadIndex protocol.
        //
        // ReadIndex works by:
        // 1. Leader records its current commit index
        // 2. Leader confirms it's still leader via heartbeat quorum
        // 3. await_ready() waits for our state machine to catch up to that commit index
        //
        // This guarantees linearizability because any read after await_ready()
        // sees all writes committed before get_read_linearizer() was called.
        //
        // Note: We refresh leader_hint after errors since leadership may have changed.
        let linearizer = self
            .raft
            .get_read_linearizer(ReadPolicy::ReadIndex)
            .await
            .map_err(|err| {
                // Refresh leader hint on error - leadership may have changed
                let leader_hint = self.raft.metrics().borrow().current_leader.map(|id| id.0);
                KeyValueStoreError::NotLeader {
                    leader: leader_hint,
                    reason: err.to_string(),
                }
            })?;

        linearizer.await_ready(&self.raft).await.map_err(|err| {
            // Refresh leader hint on error - leadership may have changed during await
            let leader_hint = self.raft.metrics().borrow().current_leader.map(|id| id.0);
            KeyValueStoreError::NotLeader {
                leader: leader_hint,
                reason: err.to_string(),
            }
        })?;

        // Read directly from state machine (linearizability guaranteed by ReadIndex above)
        match &self.state_machine {
            StateMachineVariant::InMemory(sm) => match sm.get(&request.key).await {
                Some(value) => Ok(ReadResult {
                    key: request.key,
                    value,
                }),
                None => Err(KeyValueStoreError::NotFound { key: request.key }),
            },
            StateMachineVariant::Sqlite(sm) => match sm.get(&request.key).await {
                Ok(Some(value)) => Ok(ReadResult {
                    key: request.key,
                    value,
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
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| KeyValueStoreError::Failed {
                reason: "semaphore closed".into(),
            })?;

        self.ensure_initialized_kv()?;

        // Apply delete through Raft consensus
        use crate::raft::types::AppRequest;
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
            Err(err) => {
                warn!(error = %err, "delete operation failed");
                Err(KeyValueStoreError::Failed {
                    reason: err.to_string(),
                })
            }
        }
    }

    #[instrument(skip(self))]
    async fn scan(&self, _request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| KeyValueStoreError::Failed {
                reason: "semaphore closed".into(),
            })?;

        self.ensure_initialized_kv()?;

        // Use ReadIndex for linearizable scan (see read() for protocol details)
        let linearizer = self
            .raft
            .get_read_linearizer(ReadPolicy::ReadIndex)
            .await
            .map_err(|err| {
                let leader_hint = self.raft.metrics().borrow().current_leader.map(|id| id.0);
                KeyValueStoreError::NotLeader {
                    leader: leader_hint,
                    reason: err.to_string(),
                }
            })?;

        linearizer.await_ready(&self.raft).await.map_err(|err| {
            let leader_hint = self.raft.metrics().borrow().current_leader.map(|id| id.0);
            KeyValueStoreError::NotLeader {
                leader: leader_hint,
                reason: err.to_string(),
            }
        })?;

        // Scan directly from state machine (linearizability guaranteed by ReadIndex above)
        // Apply default limit if not specified
        let limit = _request
            .limit
            .unwrap_or(DEFAULT_SCAN_LIMIT)
            .min(MAX_SCAN_RESULTS) as usize;

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
                let entries: Vec<ScanEntry> = filtered
                    .into_iter()
                    .take(limit)
                    .map(|(key, value)| ScanEntry { key, value })
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
            StateMachineVariant::Sqlite(sm) => {
                // SQLite scan with pagination
                let start_key = _request.continuation_token.as_deref();
                let all_pairs = sm.scan(&_request.prefix, start_key, Some(limit + 1)).await;

                match all_pairs {
                    Ok(pairs) => {
                        let is_truncated = pairs.len() > limit;
                        let entries: Vec<ScanEntry> = pairs
                            .into_iter()
                            .take(limit)
                            .map(|(key, value)| ScanEntry { key, value })
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
                    Err(err) => Err(KeyValueStoreError::Failed {
                        reason: err.to_string(),
                    }),
                }
            }
        }
    }
}

#[async_trait]
impl SqlQueryExecutor for RaftNode {
    #[instrument(skip(self))]
    async fn execute_sql(&self, request: SqlQueryRequest) -> Result<SqlQueryResult, SqlQueryError> {
        let _permit =
            self.semaphore
                .acquire()
                .await
                .map_err(|_| SqlQueryError::ExecutionFailed {
                    reason: "semaphore closed".into(),
                })?;

        // Validate request bounds (Tiger Style)
        validate_sql_request(&request)?;

        // Validate query is read-only
        validate_sql_query(&request.query)?;

        // For linearizable consistency, use ReadIndex protocol
        if request.consistency == SqlConsistency::Linearizable {
            let linearizer = self
                .raft
                .get_read_linearizer(ReadPolicy::ReadIndex)
                .await
                .map_err(|_err| {
                    let leader_hint = self.raft.metrics().borrow().current_leader.map(|id| id.0);
                    SqlQueryError::NotLeader {
                        leader: leader_hint,
                    }
                })?;

            linearizer.await_ready(&self.raft).await.map_err(|_err| {
                let leader_hint = self.raft.metrics().borrow().current_leader.map(|id| id.0);
                SqlQueryError::NotLeader {
                    leader: leader_hint,
                }
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
            StateMachineVariant::Sqlite(sm) => {
                // Execute SQL on SQLite state machine
                sm.execute_sql(
                    &request.query,
                    &request.params,
                    request.limit,
                    request.timeout_ms,
                )
            }
        }
    }
}

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
    pub state: openraft::ServerState,
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

        let state = borrowed.state;
        let is_shutdown = matches!(state, openraft::ServerState::Shutdown);
        let leader = borrowed.current_leader.map(|id| id.0);
        let has_membership = borrowed
            .membership_config
            .membership()
            .voter_ids()
            .next()
            .is_some();

        let consecutive_failures = self
            .consecutive_failures
            .load(std::sync::atomic::Ordering::Relaxed);

        HealthStatus {
            healthy: !is_shutdown && (has_membership || state == openraft::ServerState::Learner),
            state,
            leader,
            consecutive_failures,
            is_shutdown,
            has_membership,
        }
    }

    /// Reset the failure counter (call after recovery).
    pub fn reset_failures(&self) {
        self.consecutive_failures
            .store(0, std::sync::atomic::Ordering::Relaxed);
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
                let failures = self
                    .consecutive_failures
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    + 1;

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
                let prev_failures = self
                    .consecutive_failures
                    .swap(0, std::sync::atomic::Ordering::Relaxed);
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
