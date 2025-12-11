use async_trait::async_trait;
use iroh::EndpointAddr;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod inmemory;
pub use inmemory::{DeterministicClusterController, DeterministicKeyValueStore};

// Re-export OpenRaft types for observability
pub use openraft::ServerState;
pub use openraft::metrics::RaftMetrics;

/// Describes a node participating in the control-plane cluster.
///
/// Contains both the node's identifier and its Iroh P2P endpoint address,
/// which is stored in Raft membership state for persistent discovery.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterNode {
    pub id: u64,
    /// Display address for logging and human-readable output.
    /// When Iroh address is available, this is derived from `iroh_addr.id()`.
    pub addr: String,
    /// Optional legacy Raft address (host:port) for backwards compatibility.
    pub raft_addr: Option<String>,
    /// Iroh P2P endpoint address for connecting to this node.
    /// This is the primary address used for Raft RPC transport.
    pub iroh_addr: Option<EndpointAddr>,
}

impl ClusterNode {
    /// Create a new ClusterNode with a simple string address (legacy).
    pub fn new(id: u64, addr: impl Into<String>, raft_addr: Option<String>) -> Self {
        Self {
            id,
            addr: addr.into(),
            raft_addr,
            iroh_addr: None,
        }
    }

    /// Create a new ClusterNode with an Iroh endpoint address.
    pub fn with_iroh_addr(id: u64, iroh_addr: EndpointAddr) -> Self {
        Self {
            id,
            addr: iroh_addr.id.to_string(),
            raft_addr: None,
            iroh_addr: Some(iroh_addr),
        }
    }
}

/// Reflects the state of the cluster from the perspective of the control plane.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterState {
    pub nodes: Vec<ClusterNode>,
    pub members: Vec<u64>,
    pub learners: Vec<ClusterNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InitRequest {
    pub initial_members: Vec<ClusterNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AddLearnerRequest {
    pub learner: ClusterNode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangeMembershipRequest {
    pub members: Vec<u64>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ControlPlaneError {
    #[error("invalid request: {reason}")]
    InvalidRequest { reason: String },
    #[error("cluster not initialized")]
    NotInitialized,
    #[error("operation failed: {reason}")]
    Failed { reason: String },
    #[error("operation not supported by {backend} backend: {operation}")]
    Unsupported { backend: String, operation: String },
}

#[async_trait]
pub trait ClusterController: Send + Sync {
    async fn init(&self, request: InitRequest) -> Result<ClusterState, ControlPlaneError>;
    async fn add_learner(
        &self,
        request: AddLearnerRequest,
    ) -> Result<ClusterState, ControlPlaneError>;
    async fn change_membership(
        &self,
        request: ChangeMembershipRequest,
    ) -> Result<ClusterState, ControlPlaneError>;
    async fn current_state(&self) -> Result<ClusterState, ControlPlaneError>;

    /// Get the current Raft metrics for observability.
    ///
    /// Returns comprehensive metrics including:
    /// - Node state (Leader/Follower/Candidate/Learner)
    /// - Current leader ID
    /// - Term and vote information
    /// - Log indices (last_log, last_applied, snapshot, purged)
    /// - Replication state (leader only)
    ///
    /// This method provides raw OpenRaft metrics. For a simplified JSON format,
    /// use the HTTP `/raft-metrics` endpoint.
    async fn get_metrics(
        &self,
    ) -> Result<RaftMetrics<crate::raft::types::AppTypeConfig>, ControlPlaneError>;

    /// Trigger a snapshot to be taken immediately.
    ///
    /// Returns the log ID of the created snapshot.
    /// Useful for testing and manual cluster maintenance.
    ///
    /// # Returns
    /// - `Ok(Some(log_id))` if snapshot was created successfully
    /// - `Ok(None)` if no snapshot was needed (no logs to snapshot)
    /// - `Err(_)` if snapshot creation failed
    async fn trigger_snapshot(
        &self,
    ) -> Result<Option<openraft::LogId<crate::raft::types::AppTypeConfig>>, ControlPlaneError>;

    /// Get the current leader ID, if known.
    ///
    /// Returns None if no leader is elected or leadership is unknown.
    /// This is a convenience method that extracts current_leader from metrics.
    async fn get_leader(&self) -> Result<Option<u64>, ControlPlaneError> {
        Ok(self.get_metrics().await?.current_leader)
    }
}

// Blanket implementation for Arc<T> where T: ClusterController
#[async_trait]
impl<T: ClusterController> ClusterController for std::sync::Arc<T> {
    async fn init(&self, request: InitRequest) -> Result<ClusterState, ControlPlaneError> {
        (**self).init(request).await
    }

    async fn add_learner(
        &self,
        request: AddLearnerRequest,
    ) -> Result<ClusterState, ControlPlaneError> {
        (**self).add_learner(request).await
    }

    async fn change_membership(
        &self,
        request: ChangeMembershipRequest,
    ) -> Result<ClusterState, ControlPlaneError> {
        (**self).change_membership(request).await
    }

    async fn current_state(&self) -> Result<ClusterState, ControlPlaneError> {
        (**self).current_state().await
    }

    async fn get_metrics(
        &self,
    ) -> Result<RaftMetrics<crate::raft::types::AppTypeConfig>, ControlPlaneError> {
        (**self).get_metrics().await
    }

    async fn trigger_snapshot(
        &self,
    ) -> Result<Option<openraft::LogId<crate::raft::types::AppTypeConfig>>, ControlPlaneError> {
        (**self).trigger_snapshot().await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WriteCommand {
    Set { key: String, value: String },
    SetMulti { pairs: Vec<(String, String)> },
    Delete { key: String },
    DeleteMulti { keys: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WriteRequest {
    pub command: WriteCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WriteResult {
    pub command: WriteCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadRequest {
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadResult {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KeyValueStoreError {
    #[error("key '{key}' not found")]
    NotFound { key: String },
    #[error("operation failed: {reason}")]
    Failed { reason: String },
    #[error("key size {size} exceeds maximum of {max} bytes")]
    KeyTooLarge { size: usize, max: u32 },
    #[error("value size {size} exceeds maximum of {max} bytes")]
    ValueTooLarge { size: usize, max: u32 },
    #[error("batch size {size} exceeds maximum of {max} keys")]
    BatchTooLarge { size: usize, max: u32 },
    #[error("operation timed out after {duration_ms}ms")]
    Timeout { duration_ms: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeleteRequest {
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeleteResult {
    pub key: String,
    /// True if the key existed and was deleted, false if it didn't exist.
    pub deleted: bool,
}

#[async_trait]
pub trait KeyValueStore: Send + Sync {
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError>;
    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError>;

    /// Delete a key from the store.
    ///
    /// Returns Ok with deleted=true if the key was found and removed,
    /// or Ok with deleted=false if the key was not found (idempotent).
    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError>;
}
