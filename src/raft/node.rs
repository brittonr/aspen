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

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use openraft::{Raft, RaftMetrics, ReadPolicy};
use tokio::sync::{RwLock, Semaphore};
use tracing::{info, instrument, warn};

use crate::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, ClusterNode, ClusterState,
    ControlPlaneError, DeleteRequest, DeleteResult, InitRequest, KeyValueStore, KeyValueStoreError,
    ReadRequest, ReadResult, ScanRequest, ScanResult, WriteRequest, WriteResult,
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

    /// Whether the cluster has been initialized.
    initialized: RwLock<bool>,

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
            initialized: RwLock::new(false),
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
    pub async fn is_initialized(&self) -> bool {
        *self.initialized.read().await
    }

    /// Ensure the cluster is initialized.
    async fn ensure_initialized(&self) -> Result<(), ControlPlaneError> {
        if !*self.initialized.read().await {
            return Err(ControlPlaneError::NotInitialized);
        }
        Ok(())
    }

    /// Ensure the cluster is initialized for KV operations.
    async fn ensure_initialized_kv(&self) -> Result<(), KeyValueStoreError> {
        if !*self.initialized.read().await {
            return Err(KeyValueStoreError::Failed {
                reason: "cluster not initialized".into(),
            });
        }
        Ok(())
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

        self.raft
            .initialize(nodes)
            .await
            .map_err(|err| ControlPlaneError::Failed {
                reason: err.to_string(),
            })?;

        *self.initialized.write().await = true;

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

        self.ensure_initialized().await?;

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

        self.ensure_initialized().await?;

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
        self.ensure_initialized().await?;
        Ok(self.build_cluster_state())
    }

    #[instrument(skip(self))]
    async fn get_leader(&self) -> Result<Option<u64>, ControlPlaneError> {
        self.ensure_initialized().await?;
        let metrics = self.raft.metrics().borrow().clone();
        Ok(metrics.current_leader.map(|id| id.0))
    }

    #[instrument(skip(self))]
    async fn get_metrics(&self) -> Result<RaftMetrics<AppTypeConfig>, ControlPlaneError> {
        self.ensure_initialized().await?;
        Ok(self.raft.metrics().borrow().clone())
    }

    #[instrument(skip(self))]
    async fn trigger_snapshot(
        &self,
    ) -> Result<Option<openraft::LogId<AppTypeConfig>>, ControlPlaneError> {
        self.ensure_initialized().await?;

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

        self.ensure_initialized_kv().await?;

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
        };

        // Apply write through Raft consensus
        let result = self.raft.client_write(app_request).await;

        match result {
            Ok(_resp) => {
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

        self.ensure_initialized_kv().await?;

        // Use ReadIndex for linearizable reads
        let linearizer = self
            .raft
            .get_read_linearizer(ReadPolicy::ReadIndex)
            .await
            .map_err(|err| KeyValueStoreError::Failed {
                reason: err.to_string(),
            })?;

        linearizer
            .await_ready(&self.raft)
            .await
            .map_err(|err| KeyValueStoreError::Failed {
                reason: err.to_string(),
            })?;

        // Read directly from state machine
        match &self.state_machine {
            StateMachineVariant::InMemory(_sm) => {
                // TODO: InMemory state machine needs public read method
                Err(KeyValueStoreError::Failed {
                    reason: "InMemory state machine read not implemented".into(),
                })
            }
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

        self.ensure_initialized_kv().await?;

        // Apply delete through Raft consensus
        use crate::raft::types::AppRequest;
        let app_request = AppRequest::Delete {
            key: request.key.clone(),
        };

        let result = self.raft.client_write(app_request).await;

        match result {
            Ok(_resp) => {
                // Assume delete was successful (we'd need to check the response properly)
                Ok(DeleteResult {
                    key: request.key,
                    deleted: true,
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

        self.ensure_initialized_kv().await?;

        // Use ReadIndex for linearizable scan
        let linearizer = self
            .raft
            .get_read_linearizer(ReadPolicy::ReadIndex)
            .await
            .map_err(|err| KeyValueStoreError::Failed {
                reason: err.to_string(),
            })?;

        linearizer
            .await_ready(&self.raft)
            .await
            .map_err(|err| KeyValueStoreError::Failed {
                reason: err.to_string(),
            })?;

        // Scan directly from state machine
        match &self.state_machine {
            StateMachineVariant::InMemory(_sm) => {
                // TODO: InMemory state machine needs public scan method
                Err(KeyValueStoreError::Failed {
                    reason: "InMemory state machine scan not implemented".into(),
                })
            }
            StateMachineVariant::Sqlite(_sm) => {
                // TODO: SqliteStateMachine needs public scan method
                // For now, we'll just return an empty result
                Ok(ScanResult {
                    entries: vec![],
                    count: 0,
                    is_truncated: false,
                    continuation_token: None,
                })
            }
        }
    }
}

/// Health monitor for RaftNode.
///
/// Provides periodic health checks without actor overhead.
pub struct RaftNodeHealth {
    node: Arc<RaftNode>,
}

impl RaftNodeHealth {
    /// Create a new health monitor.
    pub fn new(node: Arc<RaftNode>) -> Self {
        Self { node }
    }

    /// Check if the node is healthy.
    pub async fn is_healthy(&self) -> bool {
        // Simple health check: can we get metrics and is there a state?
        let metrics = self.node.raft.metrics();
        let borrowed = metrics.borrow();
        // Check if the node is in any valid state (not just created)
        !matches!(&borrowed.state, openraft::ServerState::Shutdown)
    }

    /// Run periodic health monitoring.
    pub async fn monitor(&self, interval_secs: u64) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));

        loop {
            interval.tick().await;

            if !self.is_healthy().await {
                warn!(node_id = %self.node.node_id, "node health check failed");
            }
        }
    }
}
