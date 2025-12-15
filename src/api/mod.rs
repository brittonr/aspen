//! Core API traits and types for Aspen cluster operations.
//!
//! This module defines the primary interfaces for cluster control and key-value storage,
//! enabling different backend implementations (Raft-based for production, in-memory for testing).
//!
//! # Key Traits
//!
//! - `ClusterController`: Cluster membership management (init, add learner, change membership)
//! - `KeyValueStore`: Distributed key-value operations (read, write, delete, scan)
//!
//! # Tiger Style
//!
//! - Fixed limits on scan results (MAX_SCAN_RESULTS = 10,000)
//! - Explicit error types with actionable context
//! - Size validation on keys and values (prevents memory exhaustion)
//! - Pagination support for bounded memory usage
//!
//! # Example
//!
//! ```ignore
//! use aspen::api::{KeyValueStore, WriteRequest, WriteCommand, ReadRequest};
//!
//! async fn example(store: &impl KeyValueStore) -> Result<(), anyhow::Error> {
//!     // Write a key-value pair
//!     store.write(WriteRequest {
//!         command: WriteCommand::Set {
//!             key: "user:123".into(),
//!             value: "Alice".into(),
//!         },
//!     }).await?;
//!
//!     // Read it back
//!     let result = store.read(ReadRequest { key: "user:123".into() }).await?;
//!     assert_eq!(result.value, "Alice");
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use iroh::EndpointAddr;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod inmemory;
pub mod vault;
pub use inmemory::{DeterministicClusterController, DeterministicKeyValueStore};
pub use vault::{VaultInfo, VaultKeyValue, VaultListRequest, VaultListResponse};

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
        Ok(self.get_metrics().await?.current_leader.map(|id| id.0))
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
    #[error("not leader; current leader: {leader:?}; {reason}")]
    NotLeader { leader: Option<u64>, reason: String },
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

/// Request to scan keys with a given prefix.
///
/// Supports pagination via continuation tokens.
/// Tiger Style: Fixed limits prevent unbounded memory usage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanRequest {
    /// Key prefix to match (empty string matches all keys).
    pub prefix: String,
    /// Maximum number of results to return (default: 1000, max: 10000).
    pub limit: Option<u32>,
    /// Opaque continuation token from previous scan response.
    pub continuation_token: Option<String>,
}

/// A single key-value pair from a scan operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanEntry {
    pub key: String,
    pub value: String,
}

/// Response from a scan operation.
///
/// Tiger Style: Bounded results with explicit truncation indicator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanResult {
    /// Matching key-value pairs (ordered by key).
    pub entries: Vec<ScanEntry>,
    /// Total number of entries returned.
    pub count: u32,
    /// True if more results are available.
    pub is_truncated: bool,
    /// Token for fetching the next page (present if is_truncated is true).
    pub continuation_token: Option<String>,
}

/// Maximum number of keys that can be returned in a single scan.
///
/// Tiger Style: Fixed limit prevents unbounded memory allocation.
pub const MAX_SCAN_RESULTS: u32 = 10_000;

/// Default number of keys returned in a scan if limit is not specified.
pub const DEFAULT_SCAN_LIMIT: u32 = 1_000;

/// Distributed key-value store interface.
///
/// Provides linearizable read/write access to a distributed key-value store
/// backed by Raft consensus. All write operations are replicated to the cluster
/// and return only after the write is committed to a quorum of nodes.
///
/// Tiger Style: Operations have bounded size limits to prevent resource exhaustion.
#[async_trait]
pub trait KeyValueStore: Send + Sync {
    /// Write one or more key-value pairs to the store.
    ///
    /// Supports single key writes and batch operations via `WriteCommand`:
    /// - `Set { key, value }`: Write a single key-value pair
    /// - `SetMulti { pairs }`: Write multiple key-value pairs atomically
    /// - `Delete { key }`: Delete a single key
    /// - `DeleteMulti { keys }`: Delete multiple keys atomically
    ///
    /// # Errors
    ///
    /// - `KeyTooLarge`: Key exceeds MAX_KEY_SIZE bytes
    /// - `ValueTooLarge`: Value exceeds MAX_VALUE_SIZE bytes
    /// - `BatchTooLarge`: SetMulti/DeleteMulti exceeds MAX_BATCH_SIZE keys
    /// - `Timeout`: Operation did not complete within timeout
    /// - `Failed`: Raft replication or other internal error
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError>;

    /// Read a value by key.
    ///
    /// Returns the current committed value for the specified key.
    /// Read consistency depends on the backend implementation:
    /// - Raft backend: Linearizable reads (leader-only or read index)
    /// - In-memory backend: Local reads (for testing)
    ///
    /// # Errors
    ///
    /// - `NotFound`: Key does not exist in the store
    /// - `Timeout`: Read did not complete within timeout
    /// - `Failed`: Internal error
    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError>;

    /// Delete a key from the store.
    ///
    /// Returns Ok with deleted=true if the key was found and removed,
    /// or Ok with deleted=false if the key was not found (idempotent).
    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError>;

    /// Scan keys matching a prefix with pagination support.
    ///
    /// Returns keys in sorted order (lexicographic). The continuation_token
    /// from the response can be used to fetch the next page of results.
    ///
    /// Tiger Style: Bounded results prevent unbounded memory usage.
    /// Maximum limit is MAX_SCAN_RESULTS (10,000).
    async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError>;
}

/// Implement KeyValueStore for Arc<T> where T: KeyValueStore.
///
/// This allows Arc-wrapped KeyValueStore implementations to be used
/// directly where the trait is expected.
#[async_trait]
impl<T: KeyValueStore + ?Sized> KeyValueStore for std::sync::Arc<T> {
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        (**self).write(request).await
    }

    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        (**self).read(request).await
    }

    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
        (**self).delete(request).await
    }

    async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        (**self).scan(request).await
    }
}
