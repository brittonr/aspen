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
//! # Test Coverage
//!
//! TODO: Add unit tests for trait implementations via integration tests:
//!       - ClusterNode construction and serialization
//!       - ClusterState transitions and validation
//!       - WriteCommand validation with edge cases (max key/value sizes)
//!       - ScanRequest pagination boundary testing
//!       Coverage: 0% line coverage (traits tested via RaftNode integration tests)
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
//!     let result = store.read(ReadRequest::new("user:123".into())).await?;
//!     assert_eq!(result.value, "Alice");
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

// ============================================================================
// Wrapper Types for External Dependencies
// ============================================================================

/// P2P endpoint address for connecting to a node.
///
/// This type wraps `iroh::EndpointAddr` to decouple the public API from the
/// underlying iroh implementation. It provides the same functionality while
/// allowing internal implementation changes without breaking the public API.
///
/// # Usage
///
/// ```ignore
/// use aspen::api::NodeAddress;
///
/// // Create from iroh endpoint address
/// let addr: NodeAddress = endpoint.node_addr().into();
///
/// // Convert back to iroh type when needed internally
/// let iroh_addr: iroh::EndpointAddr = addr.into();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeAddress(iroh::EndpointAddr);

impl NodeAddress {
    /// Create a new NodeAddress from an iroh EndpointAddr.
    pub fn new(addr: iroh::EndpointAddr) -> Self {
        Self(addr)
    }

    /// Get the node's public key ID as a string.
    pub fn id(&self) -> String {
        self.0.id.to_string()
    }

    /// Get a reference to the underlying iroh EndpointAddr.
    ///
    /// This is provided for internal use where the iroh type is required.
    pub fn inner(&self) -> &iroh::EndpointAddr {
        &self.0
    }

    /// Consume this wrapper and return the underlying iroh EndpointAddr.
    ///
    /// This is provided for internal use where the iroh type is required.
    pub fn into_inner(self) -> iroh::EndpointAddr {
        self.0
    }
}

impl From<iroh::EndpointAddr> for NodeAddress {
    fn from(addr: iroh::EndpointAddr) -> Self {
        Self(addr)
    }
}

impl From<NodeAddress> for iroh::EndpointAddr {
    fn from(addr: NodeAddress) -> Self {
        addr.0
    }
}

impl std::fmt::Display for NodeAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.id)
    }
}

pub mod inmemory;
pub mod pure;
#[cfg(feature = "sql")]
pub mod sql_validation;
pub mod vault;
pub use inmemory::DeterministicClusterController;
pub use inmemory::DeterministicKeyValueStore;
// Note: RaftMetrics is no longer re-exported - use ClusterMetrics instead
pub use vault::SYSTEM_PREFIX;
pub use vault::VaultError;
pub use vault::is_system_key;
pub use vault::validate_client_key;

#[cfg(feature = "sql")]
use crate::constants::DEFAULT_SQL_RESULT_ROWS;
#[cfg(feature = "sql")]
use crate::constants::DEFAULT_SQL_TIMEOUT_MS;
use crate::constants::MAX_KEY_SIZE;
use crate::constants::MAX_SETMULTI_KEYS;
#[cfg(feature = "sql")]
use crate::constants::MAX_SQL_PARAMS;
#[cfg(feature = "sql")]
use crate::constants::MAX_SQL_QUERY_SIZE;
#[cfg(feature = "sql")]
use crate::constants::MAX_SQL_RESULT_ROWS;
#[cfg(feature = "sql")]
use crate::constants::MAX_SQL_TIMEOUT_MS;
use crate::constants::MAX_VALUE_SIZE;

use std::collections::BTreeMap;

// ============================================================================
// Wrapper Types (hide OpenRaft implementation details)
// ============================================================================

/// The current state of a node in the Raft cluster.
///
/// This is an API-owned enum that abstracts away the underlying openraft
/// implementation details, providing a stable public interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeState {
    /// A learner node that replicates data but does not participate in voting.
    Learner,
    /// A voting follower that replicates the leader's log.
    Follower,
    /// A node attempting to become leader through an election.
    Candidate,
    /// The elected leader that handles all client requests.
    Leader,
    /// The node is shutting down.
    Shutdown,
}

impl NodeState {
    /// Returns true if this node is the leader.
    #[must_use]
    pub fn is_leader(&self) -> bool {
        matches!(self, Self::Leader)
    }

    /// Returns true if this node can accept reads (leader or follower with ReadIndex).
    #[must_use]
    pub fn can_serve_reads(&self) -> bool {
        matches!(self, Self::Leader | Self::Follower)
    }

    /// Returns true if this node is healthy (not shutdown).
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        !matches!(self, Self::Shutdown)
    }

    /// Convert to a numeric value for metrics/serialization.
    #[must_use]
    pub fn as_u8(&self) -> u8 {
        match self {
            Self::Learner => 0,
            Self::Follower => 1,
            Self::Candidate => 2,
            Self::Leader => 3,
            Self::Shutdown => 4,
        }
    }
}

impl From<openraft::ServerState> for NodeState {
    fn from(state: openraft::ServerState) -> Self {
        match state {
            openraft::ServerState::Learner => Self::Learner,
            openraft::ServerState::Follower => Self::Follower,
            openraft::ServerState::Candidate => Self::Candidate,
            openraft::ServerState::Leader => Self::Leader,
            openraft::ServerState::Shutdown => Self::Shutdown,
        }
    }
}

impl From<NodeState> for openraft::ServerState {
    fn from(state: NodeState) -> Self {
        match state {
            NodeState::Learner => Self::Learner,
            NodeState::Follower => Self::Follower,
            NodeState::Candidate => Self::Candidate,
            NodeState::Leader => Self::Leader,
            NodeState::Shutdown => Self::Shutdown,
        }
    }
}

/// Cluster metrics wrapper that hides openraft implementation details.
///
/// This type provides access to commonly-needed Raft metrics without
/// exposing the underlying openraft types in the public API.
#[derive(Debug, Clone)]
pub struct ClusterMetrics {
    /// This node's ID.
    pub id: u64,
    /// Current Raft state (Leader, Follower, Candidate, Learner, Shutdown).
    pub state: NodeState,
    /// Current leader node ID, if known.
    pub current_leader: Option<u64>,
    /// Current Raft term.
    pub current_term: u64,
    /// Last log index in the Raft log.
    pub last_log_index: Option<u64>,
    /// Last applied log index (state machine is caught up to this point).
    pub last_applied_index: Option<u64>,
    /// Snapshot log index (state up to this point is in the snapshot).
    pub snapshot_index: Option<u64>,
    /// Replication progress for each follower (only populated on leader).
    /// Maps node_id -> matched_log_index.
    pub replication: Option<BTreeMap<u64, Option<u64>>>,
    /// Current voting members in the cluster.
    pub voters: Vec<u64>,
    /// Current learner (non-voting) members in the cluster.
    pub learners: Vec<u64>,
}

impl ClusterMetrics {
    /// Create ClusterMetrics from openraft RaftMetrics.
    pub(crate) fn from_openraft(metrics: &openraft::metrics::RaftMetrics<crate::raft::types::AppTypeConfig>) -> Self {
        let membership = metrics.membership_config.membership();
        Self {
            id: metrics.id.0,
            state: metrics.state.into(),
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
}

/// Snapshot log identifier wrapper that hides openraft implementation details.
///
/// Represents the position in the Raft log where a snapshot was taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotLogId {
    /// The term in which this log entry was created.
    pub term: u64,
    /// The index of this log entry.
    pub index: u64,
}

impl SnapshotLogId {
    /// Create SnapshotLogId from openraft LogId.
    pub(crate) fn from_openraft(log_id: &openraft::LogId<crate::raft::types::AppTypeConfig>) -> Self {
        Self {
            term: log_id.leader_id.term,
            index: log_id.index,
        }
    }
}

/// Describes a node participating in the control-plane cluster.
///
/// Contains both the node's identifier and its P2P endpoint address,
/// which is stored in Raft membership state for persistent discovery.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterNode {
    /// Unique identifier for this node within the cluster.
    ///
    /// This is the Raft NodeId used for consensus operations. Must be unique
    /// across all nodes in the cluster and stable across restarts.
    pub id: u64,
    /// Display address for logging and human-readable output.
    /// When node address is available, this is derived from `node_addr.id()`.
    pub addr: String,
    /// Optional legacy Raft address (host:port) for backwards compatibility.
    pub raft_addr: Option<String>,
    /// P2P endpoint address for connecting to this node.
    /// This is the primary address used for Raft RPC transport.
    pub node_addr: Option<NodeAddress>,
}

impl ClusterNode {
    /// Create a new ClusterNode with a simple string address (legacy).
    pub fn new(id: u64, addr: impl Into<String>, raft_addr: Option<String>) -> Self {
        Self {
            id,
            addr: addr.into(),
            raft_addr,
            node_addr: None,
        }
    }

    /// Create a new ClusterNode with a P2P endpoint address.
    pub fn with_node_addr(id: u64, node_addr: NodeAddress) -> Self {
        Self {
            id,
            addr: node_addr.id(),
            raft_addr: None,
            node_addr: Some(node_addr),
        }
    }

    /// Create a new ClusterNode with an iroh EndpointAddr (convenience method).
    ///
    /// This is a convenience method that converts the iroh type to NodeAddress.
    pub fn with_iroh_addr(id: u64, iroh_addr: iroh::EndpointAddr) -> Self {
        Self::with_node_addr(id, NodeAddress::new(iroh_addr))
    }

    /// Get the node address as an iroh EndpointAddr, if available.
    ///
    /// This is provided for internal use where the iroh type is required.
    pub fn iroh_addr(&self) -> Option<&iroh::EndpointAddr> {
        self.node_addr.as_ref().map(|addr| addr.inner())
    }
}

/// Reflects the state of the cluster from the perspective of the control plane.
///
/// Provides a snapshot of cluster topology including all known nodes,
/// current voting members, and learner nodes.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterState {
    /// All nodes known to the cluster (both voters and learners).
    pub nodes: Vec<ClusterNode>,
    /// Node IDs of current voting members participating in Raft consensus.
    pub members: Vec<u64>,
    /// Non-voting learner nodes that replicate data but don't vote.
    pub learners: Vec<ClusterNode>,
}

/// Request to initialize a new Raft cluster.
///
/// Used with [`ClusterController::init()`] to bootstrap a cluster.
/// The initial members become the founding voters of the cluster.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InitRequest {
    /// The founding voting members of the cluster.
    ///
    /// Must contain at least one node. For fault tolerance, use an odd
    /// number of nodes (3 or 5 recommended for production).
    pub initial_members: Vec<ClusterNode>,
}

/// Request to add a non-voting learner to the cluster.
///
/// Used with [`ClusterController::add_learner()`] to add nodes that
/// replicate data without participating in consensus voting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AddLearnerRequest {
    /// The learner node to add to the cluster.
    pub learner: ClusterNode,
}

/// Request to change the voting membership of the cluster.
///
/// Used with [`ClusterController::change_membership()`] to reconfigure
/// which nodes participate in Raft consensus voting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangeMembershipRequest {
    /// The complete set of node IDs that should be voting members.
    ///
    /// This replaces the current voter set entirely. Include all nodes
    /// that should vote, not just new additions.
    pub members: Vec<u64>,
}

/// Errors that can occur during cluster control plane operations.
///
/// These errors indicate failures in cluster management operations like
/// initialization, membership changes, and state queries.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ControlPlaneError {
    /// The request contained invalid parameters or configuration.
    ///
    /// Check the `reason` field for details about what was invalid.
    #[error("invalid request: {reason}")]
    InvalidRequest {
        /// Human-readable description of what was invalid in the request.
        reason: String,
    },

    /// The cluster has not been initialized yet.
    ///
    /// Call [`ClusterController::init()`] to bootstrap the cluster
    /// before attempting other operations.
    #[error("cluster not initialized")]
    NotInitialized,

    /// The operation failed due to a Raft or internal error.
    ///
    /// This may indicate the node is not the leader, quorum is unavailable,
    /// or another internal error occurred.
    #[error("operation failed: {reason}")]
    Failed {
        /// Human-readable description of the failure.
        reason: String,
    },

    /// The operation is not supported by this backend implementation.
    ///
    /// Some backends (like in-memory test implementations) may not support
    /// all operations.
    #[error("operation not supported by {backend} backend: {operation}")]
    Unsupported {
        /// Name of the backend implementation (e.g., "in-memory", "sqlite").
        backend: String,
        /// Name of the unsupported operation.
        operation: String,
    },

    /// The operation timed out.
    ///
    /// Tiger Style: Explicit timeout prevents indefinite hangs during
    /// membership operations when leader is unavailable or quorum lost.
    #[error("operation timed out after {duration_ms}ms")]
    Timeout {
        /// Duration in milliseconds before timeout.
        duration_ms: u64,
    },
}

/// Manages cluster membership and Raft consensus operations.
///
/// This trait provides the control plane interface for initializing clusters,
/// managing node membership, and monitoring cluster health. All operations
/// that modify cluster membership require leader status.
#[async_trait]
pub trait ClusterController: Send + Sync {
    /// Initialize a new Raft cluster with founding members.
    ///
    /// Bootstrap a cluster by establishing the initial voting membership.
    /// This is a one-time operation per cluster - calling init on an already
    /// initialized cluster will fail.
    ///
    /// # Errors
    ///
    /// - `InvalidRequest`: Empty or invalid member list
    /// - `Failed`: Cluster already initialized or Raft error
    async fn init(&self, request: InitRequest) -> Result<ClusterState, ControlPlaneError>;

    /// Add a non-voting learner node to the cluster.
    ///
    /// Learners replicate the Raft log but don't participate in leader elections
    /// or vote counting. This is typically the first step when adding a new node
    /// to an existing cluster, allowing it to catch up before becoming a voter.
    ///
    /// # Errors
    ///
    /// - `NotInitialized`: Cluster has not been initialized
    /// - `InvalidRequest`: Invalid node configuration
    /// - `Failed`: Not leader or Raft operation failed
    async fn add_learner(&self, request: AddLearnerRequest) -> Result<ClusterState, ControlPlaneError>;

    /// Change the set of voting members in the cluster.
    ///
    /// Atomically reconfigure which nodes participate in Raft consensus.
    /// The membership change replaces the entire voter set - include all
    /// nodes that should vote, not just additions.
    ///
    /// # Errors
    ///
    /// - `NotInitialized`: Cluster has not been initialized
    /// - `InvalidRequest`: Invalid member set (empty, non-existent nodes)
    /// - `Failed`: Not leader, quorum unavailable, or Raft error
    async fn change_membership(&self, request: ChangeMembershipRequest) -> Result<ClusterState, ControlPlaneError>;

    /// Get the current cluster topology and membership state.
    ///
    /// Returns information about all known nodes, current voting members,
    /// and learners. This reflects the committed Raft membership state.
    ///
    /// # Errors
    ///
    /// - `NotInitialized`: Cluster has not been initialized
    /// - `Failed`: Unable to read cluster state
    async fn current_state(&self) -> Result<ClusterState, ControlPlaneError>;

    /// Get the current Raft metrics for observability.
    ///
    /// Returns comprehensive metrics including:
    /// - Node state (Leader/Follower/Candidate/Learner)
    /// - Current leader ID
    /// - Current term
    /// - Log indices (last_log, last_applied, snapshot)
    /// - Replication state (leader only)
    ///
    /// Returns a `ClusterMetrics` struct that wraps the essential metrics
    /// without exposing OpenRaft implementation details.
    async fn get_metrics(&self) -> Result<ClusterMetrics, ControlPlaneError>;

    /// Trigger a snapshot to be taken immediately.
    ///
    /// Returns the log ID of the created snapshot.
    /// Useful for testing and manual cluster maintenance.
    ///
    /// # Returns
    /// - `Ok(Some(log_id))` if snapshot was created successfully
    /// - `Ok(None)` if no snapshot was needed (no logs to snapshot)
    /// - `Err(_)` if snapshot creation failed
    async fn trigger_snapshot(&self) -> Result<Option<SnapshotLogId>, ControlPlaneError>;

    /// Get the current leader ID, if known.
    ///
    /// Returns None if no leader is elected or leadership is unknown.
    /// This is a convenience method that extracts current_leader from metrics.
    async fn get_leader(&self) -> Result<Option<u64>, ControlPlaneError> {
        Ok(self.get_metrics().await?.current_leader)
    }

    /// Check if the cluster has been initialized.
    ///
    /// Returns `true` if init() has been called or if this node has
    /// received membership info through Raft replication.
    ///
    /// This is a non-blocking, synchronous check that does not require
    /// consensus. Use this for fast-path decisions about cluster readiness.
    fn is_initialized(&self) -> bool;
}

// Blanket implementation for Arc<T> where T: ClusterController
#[async_trait]
impl<T: ClusterController> ClusterController for std::sync::Arc<T> {
    async fn init(&self, request: InitRequest) -> Result<ClusterState, ControlPlaneError> {
        (**self).init(request).await
    }

    async fn add_learner(&self, request: AddLearnerRequest) -> Result<ClusterState, ControlPlaneError> {
        (**self).add_learner(request).await
    }

    async fn change_membership(&self, request: ChangeMembershipRequest) -> Result<ClusterState, ControlPlaneError> {
        (**self).change_membership(request).await
    }

    async fn current_state(&self) -> Result<ClusterState, ControlPlaneError> {
        (**self).current_state().await
    }

    async fn get_metrics(&self) -> Result<ClusterMetrics, ControlPlaneError> {
        (**self).get_metrics().await
    }

    async fn trigger_snapshot(&self) -> Result<Option<SnapshotLogId>, ControlPlaneError> {
        (**self).trigger_snapshot().await
    }

    fn is_initialized(&self) -> bool {
        (**self).is_initialized()
    }
}

/// Commands for modifying key-value state through Raft consensus.
///
/// All write commands are replicated through Raft and applied atomically.
/// Tiger Style: Fixed limits on batch sizes and key/value lengths to prevent
/// unbounded resource usage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WriteCommand {
    /// Set a single key-value pair.
    Set {
        /// The key to set.
        key: String,
        /// The value to set.
        value: String,
    },
    /// Set a key-value pair with a time-to-live.
    ///
    /// The key will be automatically expired after `ttl_seconds`.
    /// Expired keys are filtered out at read time and cleaned up in background.
    SetWithTTL {
        /// The key to set.
        key: String,
        /// The value to set.
        value: String,
        /// Time-to-live in seconds. After this duration, the key is considered expired.
        ttl_seconds: u32,
    },
    /// Set multiple key-value pairs atomically.
    SetMulti {
        /// The key-value pairs to set.
        pairs: Vec<(String, String)>,
    },
    /// Set multiple keys with TTL.
    ///
    /// All keys in this batch will have the same TTL.
    SetMultiWithTTL {
        /// The key-value pairs to set.
        pairs: Vec<(String, String)>,
        /// Time-to-live in seconds for all keys in this batch.
        ttl_seconds: u32,
    },
    /// Delete a single key.
    Delete {
        /// The key to delete.
        key: String,
    },
    /// Delete multiple keys atomically.
    DeleteMulti {
        /// The keys to delete.
        keys: Vec<String>,
    },
    /// Compare-and-swap: atomically update value if current value matches expected.
    ///
    /// - `expected: None` means the key must NOT exist (create-if-absent)
    /// - `expected: Some(val)` means the key must exist with exactly that value
    ///
    /// If the condition is met, the key is set to `new_value`.
    /// If not, the operation fails with `CompareAndSwapFailed` error containing the actual value.
    CompareAndSwap {
        /// The key to update.
        key: String,
        /// Expected current value (None means key must not exist).
        expected: Option<String>,
        /// New value to set if condition is met.
        new_value: String,
    },
    /// Compare-and-delete: atomically delete key if current value matches expected.
    ///
    /// The key must exist and have exactly the expected value for the delete to succeed.
    /// If not, the operation fails with `CompareAndSwapFailed` error containing the actual value.
    CompareAndDelete {
        /// The key to delete.
        key: String,
        /// Expected current value for the delete to succeed.
        expected: String,
    },
    /// Batch write: atomically apply multiple Set/Delete operations.
    ///
    /// All operations are applied in a single atomic transaction.
    /// Unlike SetMulti/DeleteMulti, this allows mixing Set and Delete in one batch.
    Batch {
        /// Operations to execute atomically (max 100).
        operations: Vec<BatchOperation>,
    },
    /// Conditional batch write: apply operations only if all conditions are met.
    ///
    /// Checks all conditions first (reads are done atomically), then if all pass,
    /// applies all operations in a single atomic transaction.
    /// Similar to etcd's `Txn().If(conditions).Then(ops).Commit()`.
    ConditionalBatch {
        /// Conditions that must all be true (max 100).
        conditions: Vec<BatchCondition>,
        /// Operations to execute if all conditions pass (max 100).
        operations: Vec<BatchOperation>,
    },
    // =========================================================================
    // Lease operations
    // =========================================================================
    /// Set a key attached to a lease. Key is deleted when lease expires or is revoked.
    SetWithLease {
        /// The key to set.
        key: String,
        /// The value to set.
        value: String,
        /// Lease ID to attach this key to.
        lease_id: u64,
    },
    /// Set multiple keys attached to a lease.
    SetMultiWithLease {
        /// The key-value pairs to set.
        pairs: Vec<(String, String)>,
        /// Lease ID to attach these keys to.
        lease_id: u64,
    },
    /// Grant a new lease with specified TTL.
    LeaseGrant {
        /// Client-provided lease ID (0 = auto-generate).
        lease_id: u64,
        /// Time-to-live in seconds.
        ttl_seconds: u32,
    },
    /// Revoke a lease and delete all attached keys.
    LeaseRevoke {
        /// Lease ID to revoke.
        lease_id: u64,
    },
    /// Refresh a lease's TTL.
    LeaseKeepalive {
        /// Lease ID to refresh.
        lease_id: u64,
    },
    // =========================================================================
    // Transaction operations (etcd-style If/Then/Else)
    // =========================================================================
    /// Transaction: atomic If/Then/Else with rich comparisons.
    ///
    /// Evaluates all comparisons in `compare`. If ALL pass, executes `success` operations.
    /// If ANY comparison fails, executes `failure` operations instead.
    ///
    /// Similar to etcd's `Txn().If(...).Then(...).Else(...).Commit()`.
    ///
    /// # Comparison Targets
    ///
    /// - `Value`: Compare the key's current value
    /// - `Version`: Compare the key's version (per-key modification counter)
    /// - `CreateRevision`: Compare when the key was first created (Raft log index)
    /// - `ModRevision`: Compare when the key was last modified (Raft log index)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Atomic check-and-update: only update if version matches
    /// WriteCommand::Transaction {
    ///     compare: vec![TxnCompare {
    ///         key: "config".into(),
    ///         target: CompareTarget::Version,
    ///         op: CompareOp::Equal,
    ///         value: "5".into(),
    ///     }],
    ///     success: vec![TxnOp::Put {
    ///         key: "config".into(),
    ///         value: "new_value".into(),
    ///     }],
    ///     failure: vec![TxnOp::Get { key: "config".into() }],
    /// }
    /// ```
    Transaction {
        /// Comparisons that must ALL be true for success branch.
        /// Max 100 comparisons (Tiger Style).
        compare: Vec<TxnCompare>,
        /// Operations to execute if all comparisons pass.
        /// Max 100 operations (Tiger Style).
        success: Vec<TxnOp>,
        /// Operations to execute if any comparison fails.
        /// Max 100 operations (Tiger Style).
        failure: Vec<TxnOp>,
    },
    // =========================================================================
    // Optimistic Concurrency Control (OCC) Transaction
    // =========================================================================
    /// Optimistic transaction with read set conflict detection.
    ///
    /// Implements Optimistic Concurrency Control (OCC) similar to FoundationDB.
    /// The client captures a "read set" of keys with their versions at read time,
    /// then submits the transaction with a "write set" of operations to apply.
    ///
    /// At commit time, the state machine validates that all keys in the read set
    /// still have the expected versions. If any key has been modified since read,
    /// the transaction is rejected with a `ConflictError`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Client reads key at version 5
    /// let result = store.read("balance").await?;
    /// let balance: i64 = result.value.parse()?;
    /// let version = result.version; // 5
    ///
    /// // Client computes new balance
    /// let new_balance = balance + 100;
    ///
    /// // Client submits optimistic transaction
    /// store.write(WriteCommand::OptimisticTransaction {
    ///     read_set: vec![("balance".into(), version)],
    ///     write_set: vec![WriteOp::Set {
    ///         key: "balance".into(),
    ///         value: new_balance.to_string(),
    ///     }],
    /// }).await?;
    /// ```
    ///
    /// If another client modified "balance" between the read and commit,
    /// this transaction will fail with `ConflictError` and the client can retry.
    OptimisticTransaction {
        /// Keys that were read with their expected versions at read time.
        /// If any key's current version differs from expected, transaction fails.
        /// Max 100 entries (Tiger Style).
        read_set: Vec<(String, i64)>,
        /// Operations to apply if all read set validations pass.
        /// Max 100 operations (Tiger Style).
        write_set: Vec<WriteOp>,
    },
    // =========================================================================
    // Shard topology operations (applied atomically via Raft)
    // =========================================================================
    /// Split a shard into two shards at a given key.
    ///
    /// This command is applied atomically through Raft consensus on shard 0
    /// (the control plane shard). All replicas apply the split at the same
    /// log index, ensuring consistent topology across the cluster.
    ///
    /// After split:
    /// - Source shard owns keys in range `[original_start, split_key)`
    /// - New shard owns keys in range `[split_key, original_end)`
    ///
    /// # TiKV-Style Split Protocol
    ///
    /// 1. Leader detects split trigger (size > 64MB or QPS > 2500)
    /// 2. Leader computes split key (median or load-based)
    /// 3. Leader proposes ShardSplit through Raft
    /// 4. All replicas apply split at same log index
    /// 5. Topology broadcast via gossip
    ShardSplit {
        /// Shard being split.
        source_shard: u32,
        /// Key at which to split (keys >= this go to new shard).
        split_key: String,
        /// ID for the new shard being created.
        new_shard_id: u32,
        /// Expected topology version (prevents stale splits).
        topology_version: u64,
    },
    /// Merge two adjacent shards into one.
    ///
    /// The source shard's keys are moved to the target shard, and the source
    /// shard becomes a tombstone (kept for redirect purposes, then cleaned up).
    ///
    /// # Merge Conditions
    ///
    /// - Both shards must be in Active state
    /// - Shards must have adjacent key ranges
    /// - Combined size should be < 48MB (to leave room before next split)
    ShardMerge {
        /// Shard to be merged (will become tombstone).
        source_shard: u32,
        /// Shard to merge into (will expand to cover source's range).
        target_shard: u32,
        /// Expected topology version (prevents stale merges).
        topology_version: u64,
    },
    /// Update the shard topology (internal command).
    ///
    /// Used for direct topology updates such as:
    /// - Initial topology creation
    /// - Membership changes within shards
    /// - Tombstone cleanup
    ///
    /// This is an internal command, not typically used by clients.
    TopologyUpdate {
        /// Serialized ShardTopology (bincode).
        topology_data: Vec<u8>,
    },
}

/// Operations for optimistic transactions.
///
/// These are the write operations that can be included in an `OptimisticTransaction`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WriteOp {
    /// Set a key to a value.
    Set {
        /// The key to set.
        key: String,
        /// The value to set.
        value: String,
    },
    /// Delete a key.
    Delete {
        /// The key to delete.
        key: String,
    },
}

/// A single operation within a batch write.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchOperation {
    /// Set a key to a value.
    Set {
        /// The key to set.
        key: String,
        /// The value to set.
        value: String,
    },
    /// Delete a key.
    Delete {
        /// The key to delete.
        key: String,
    },
}

/// A condition for conditional batch writes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchCondition {
    /// Key must have exactly this value.
    ValueEquals {
        /// The key to check.
        key: String,
        /// The expected value.
        expected: String,
    },
    /// Key must exist (any value).
    KeyExists {
        /// The key that must exist.
        key: String,
    },
    /// Key must not exist.
    KeyNotExists {
        /// The key that must not exist.
        key: String,
    },
}

// ============================================================================
// Transaction types (etcd-style If/Then/Else)
// ============================================================================

/// Comparison target for transaction conditions.
///
/// Specifies what attribute of a key to compare against.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompareTarget {
    /// Compare the value of the key.
    Value,
    /// Compare the version (per-key modification counter, starts at 1).
    Version,
    /// Compare the create revision (Raft log index when key was created).
    CreateRevision,
    /// Compare the mod revision (Raft log index of last modification).
    ModRevision,
}

/// Comparison operator for transaction conditions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompareOp {
    /// Equal (==)
    Equal,
    /// Not equal (!=)
    NotEqual,
    /// Greater than (>)
    Greater,
    /// Less than (<)
    Less,
}

/// A comparison condition for transactions.
///
/// Used in the `compare` clause of a `Transaction` to check key state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TxnCompare {
    /// Key to check.
    pub key: String,
    /// What attribute to compare (value, version, create_revision, mod_revision).
    pub target: CompareTarget,
    /// Comparison operator.
    pub op: CompareOp,
    /// Value to compare against.
    /// - For Value: the expected string value
    /// - For Version/CreateRevision/ModRevision: the numeric value as string
    pub value: String,
}

/// Operations that can be performed in a transaction branch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TxnOp {
    /// Set a key to a value.
    Put {
        /// The key to set.
        key: String,
        /// The value to set.
        value: String,
    },
    /// Delete a key.
    Delete {
        /// The key to delete.
        key: String,
    },
    /// Read a key (result included in response).
    Get {
        /// The key to read.
        key: String,
    },
    /// Range scan with prefix (bounded by limit).
    Range {
        /// Key prefix to match.
        prefix: String,
        /// Maximum number of results.
        limit: u32,
    },
}

/// Result of a single transaction operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TxnOpResult {
    /// Result of a Put operation.
    Put {
        /// The revision after this put.
        revision: u64,
    },
    /// Result of a Delete operation.
    Delete {
        /// Number of keys deleted (0 or 1).
        deleted: u32,
    },
    /// Result of a Get operation.
    Get {
        /// The key-value with revision info, None if key not found.
        kv: Option<KeyValueWithRevision>,
    },
    /// Result of a Range operation.
    Range {
        /// Matching key-values with revision info.
        kvs: Vec<KeyValueWithRevision>,
        /// True if more results available beyond limit.
        more: bool,
    },
}

/// Key-value pair with revision metadata.
///
/// Returned by read operations and transaction Get/Range results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyValueWithRevision {
    /// The key.
    pub key: String,
    /// The value.
    pub value: String,
    /// Per-key version counter (1, 2, 3...). Reset to 1 on delete+recreate.
    pub version: u64,
    /// Raft log index when key was first created.
    pub create_revision: u64,
    /// Raft log index of last modification.
    pub mod_revision: u64,
}

/// Request to perform a write operation through Raft consensus.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WriteRequest {
    /// The write command to execute.
    pub command: WriteCommand,
}

/// Result of a write operation.
///
/// Contains different fields depending on the type of write command executed.
/// Tiger Style: Optional fields allow the same struct to represent results from
/// different command types without allocating unnecessary data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WriteResult {
    /// The command that was executed (echo for validation).
    pub command: Option<WriteCommand>,
    /// Number of operations applied in a batch (for Batch/ConditionalBatch).
    pub batch_applied: Option<u32>,
    /// Whether conditions were met (for ConditionalBatch).
    pub conditions_met: Option<bool>,
    /// Index of first failed condition (for ConditionalBatch).
    pub failed_condition_index: Option<u32>,
    // Lease operation results
    /// Lease ID for lease operations.
    pub lease_id: Option<u64>,
    /// TTL in seconds for lease operations.
    pub ttl_seconds: Option<u32>,
    /// Number of keys deleted (for LeaseRevoke).
    pub keys_deleted: Option<u32>,
    // Transaction results
    /// For Transaction: whether the success branch was executed (all comparisons passed).
    pub succeeded: Option<bool>,
    /// For Transaction: results of the executed operations (from success or failure branch).
    pub txn_results: Option<Vec<TxnOpResult>>,
    /// For Transaction: the cluster revision after this transaction.
    pub header_revision: Option<u64>,
    // OCC transaction results
    /// For OptimisticTransaction: whether a conflict was detected.
    pub occ_conflict: Option<bool>,
    /// For OptimisticTransaction: the key that had a version mismatch.
    pub conflict_key: Option<String>,
    /// For OptimisticTransaction: the expected version.
    pub conflict_expected_version: Option<i64>,
    /// For OptimisticTransaction: the actual version found.
    pub conflict_actual_version: Option<i64>,
}

/// Consistency level for read operations.
///
/// Controls the trade-off between read latency and consistency guarantees.
/// See OpenRaft's `ReadPolicy` for implementation details.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReadConsistency {
    /// Strong consistency via ReadIndex protocol (default).
    ///
    /// Guarantees linearizable reads by confirming leadership with a quorum
    /// before serving the read. Higher latency (~50-100µs overhead) but
    /// guaranteed to see all committed writes.
    ///
    /// Use when: Correctness is critical (financial transactions, counters).
    #[default]
    Linearizable,

    /// Lease-based reads for lower latency.
    ///
    /// Uses the leader lease to serve reads without quorum confirmation.
    /// Only works when the leader has a valid lease (typically within
    /// the heartbeat interval). Falls back to ReadIndex if lease expired.
    ///
    /// Latency: ~1-5µs (vs ~50-100µs for Linearizable)
    ///
    /// Safety: Requires clock synchronization (drift < lease duration).
    /// Safe for LAN deployments and same-datacenter clusters.
    ///
    /// Use when: Low latency is critical and brief staleness is acceptable.
    Lease,

    /// Local read without any consistency checks.
    ///
    /// Reads directly from the local state machine without confirming
    /// leadership or waiting for log application. May return stale data.
    ///
    /// Latency: ~1µs (just a local lookup)
    ///
    /// WARNING: May read uncommitted or rolled-back data. Only use for
    /// monitoring, debugging, or when eventual consistency is acceptable.
    ///
    /// Use when: Maximum performance needed, staleness is acceptable.
    Stale,
}

/// Request to read a single key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadRequest {
    /// The key to read.
    pub key: String,
    /// Consistency level for the read operation.
    ///
    /// Defaults to `Linearizable` for strong consistency.
    #[serde(default)]
    pub consistency: ReadConsistency,
}

impl ReadRequest {
    /// Create a new read request with default (Linearizable) consistency.
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            consistency: ReadConsistency::default(),
        }
    }

    /// Create a read request with lease-based consistency (lower latency).
    pub fn with_lease(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            consistency: ReadConsistency::Lease,
        }
    }

    /// Create a stale read request (local read, no consistency guarantees).
    pub fn stale(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            consistency: ReadConsistency::Stale,
        }
    }
}

/// Response from a read operation.
///
/// Contains the key-value with full revision metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadResult {
    /// The key-value with revision metadata, None if key not found.
    pub kv: Option<KeyValueWithRevision>,
}

/// Errors that can occur during key-value operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KeyValueStoreError {
    /// The requested key was not found in the store.
    #[error("key '{key}' not found")]
    NotFound {
        /// The key that was not found.
        key: String,
    },
    /// The operation failed due to an internal error.
    #[error("operation failed: {reason}")]
    Failed {
        /// Human-readable description of the failure.
        reason: String,
    },
    /// The operation was rejected because this node is not the leader.
    #[error("not leader; current leader: {leader:?}; {reason}")]
    NotLeader {
        /// The current leader node ID, if known.
        leader: Option<u64>,
        /// Additional context about the rejection.
        reason: String,
    },
    /// The key exceeds the maximum allowed size.
    #[error("key size {size} exceeds maximum of {max} bytes")]
    KeyTooLarge {
        /// Actual size of the key in bytes.
        size: usize,
        /// Maximum allowed size in bytes.
        max: u32,
    },
    /// The value exceeds the maximum allowed size.
    #[error("value size {size} exceeds maximum of {max} bytes")]
    ValueTooLarge {
        /// Actual size of the value in bytes.
        size: usize,
        /// Maximum allowed size in bytes.
        max: u32,
    },
    /// The batch operation exceeds the maximum allowed number of keys.
    #[error("batch size {size} exceeds maximum of {max} keys")]
    BatchTooLarge {
        /// Actual number of keys in the batch.
        size: usize,
        /// Maximum allowed batch size.
        max: u32,
    },
    /// The operation timed out.
    #[error("operation timed out after {duration_ms}ms")]
    Timeout {
        /// Duration in milliseconds before timeout.
        duration_ms: u64,
    },
    /// Compare-and-swap operation failed because the current value didn't match expected.
    ///
    /// The `actual` field contains the current value of the key (None if key doesn't exist),
    /// allowing clients to retry with the correct expected value.
    #[error("compare-and-swap failed for key '{key}': expected {expected:?}, found {actual:?}")]
    CompareAndSwapFailed {
        /// The key being compared.
        key: String,
        /// The expected value.
        expected: Option<String>,
        /// The actual current value.
        actual: Option<String>,
    },
    /// Key cannot be empty.
    ///
    /// Tiger Style: Empty keys are rejected to prevent issues with prefix scans
    /// and to ensure all keys have meaningful identifiers.
    #[error("key cannot be empty")]
    EmptyKey,
    /// The key has moved to a different shard.
    ///
    /// This error is returned when a shard split or merge has occurred and
    /// the requested key is now owned by a different shard. Clients should
    /// update their topology cache and retry the request to the new shard.
    #[error("key '{key}' moved to shard {new_shard_id} (topology version {topology_version})")]
    ShardMoved {
        /// The key that was requested.
        key: String,
        /// The shard that now owns this key.
        new_shard_id: u32,
        /// Current topology version (client should update cache).
        topology_version: u64,
    },
    /// The shard is not in a state that allows this operation.
    ///
    /// For example, a shard that is splitting or merging may reject writes.
    #[error("shard {shard_id} is {state}, operation not allowed")]
    ShardNotReady {
        /// The shard that rejected the operation.
        shard_id: u32,
        /// Current state of the shard.
        state: String,
    },
    /// Topology version mismatch.
    ///
    /// The topology version in the request doesn't match the current version.
    /// This typically indicates a race condition with another topology change.
    #[error("topology version mismatch: expected {expected}, got {actual}")]
    TopologyVersionMismatch {
        /// Expected topology version.
        expected: u64,
        /// Actual current topology version.
        actual: u64,
    },
}

/// Request to delete a key from the distributed store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeleteRequest {
    /// The key to delete.
    pub key: String,
}

/// Result of a delete operation on the distributed store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeleteResult {
    /// The key that was requested for deletion.
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

/// Response from a scan operation.
///
/// Tiger Style: Bounded results with explicit truncation indicator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanResult {
    /// Matching key-value pairs with revision metadata (ordered by key).
    pub entries: Vec<KeyValueWithRevision>,
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

    /// Read a value by key with revision metadata.
    ///
    /// Returns the current committed value for the specified key with full revision info.
    /// If the key doesn't exist, returns `ReadResult { kv: None }`.
    ///
    /// Read consistency depends on the backend implementation:
    /// - Raft backend: Linearizable reads (leader-only or read index)
    /// - In-memory backend: Local reads (for testing)
    ///
    /// # Returns
    ///
    /// `ReadResult` containing `Option<KeyValueWithRevision>`:
    /// - `Some(kv)` if key exists with version, create_revision, mod_revision
    /// - `None` if key does not exist
    ///
    /// # Errors
    ///
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
    /// Returns keys in sorted order (lexicographic) with full revision metadata.
    /// The continuation_token from the response can be used to fetch the next page.
    ///
    /// Each entry includes version, create_revision, and mod_revision.
    ///
    /// Tiger Style: Bounded results prevent unbounded memory usage.
    /// Maximum limit is MAX_SCAN_RESULTS (10,000).
    async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError>;
}

/// Validate a write command against fixed size limits.
///
/// Enforces:
/// - Key is non-empty
/// - Key length <= MAX_KEY_SIZE bytes
/// - Value length <= MAX_VALUE_SIZE bytes
/// - Batch length (SetMulti/DeleteMulti) <= MAX_SETMULTI_KEYS
pub fn validate_write_command(command: &WriteCommand) -> Result<(), KeyValueStoreError> {
    let check_key = |key: &str| {
        if key.is_empty() {
            return Err(KeyValueStoreError::EmptyKey);
        }
        let len = key.len();
        if len > MAX_KEY_SIZE as usize {
            Err(KeyValueStoreError::KeyTooLarge {
                size: len,
                max: MAX_KEY_SIZE,
            })
        } else {
            Ok(())
        }
    };

    let check_value = |value: &str| {
        let len = value.len();
        if len > MAX_VALUE_SIZE as usize {
            Err(KeyValueStoreError::ValueTooLarge {
                size: len,
                max: MAX_VALUE_SIZE,
            })
        } else {
            Ok(())
        }
    };

    match command {
        WriteCommand::Set { key, value } => {
            check_key(key)?;
            check_value(value)?;
        }
        WriteCommand::SetWithTTL { key, value, .. } => {
            check_key(key)?;
            check_value(value)?;
        }
        WriteCommand::SetMulti { pairs } => {
            if pairs.len() > MAX_SETMULTI_KEYS as usize {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: pairs.len(),
                    max: MAX_SETMULTI_KEYS,
                });
            }
            for (key, value) in pairs {
                check_key(key)?;
                check_value(value)?;
            }
        }
        WriteCommand::SetMultiWithTTL { pairs, .. } => {
            if pairs.len() > MAX_SETMULTI_KEYS as usize {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: pairs.len(),
                    max: MAX_SETMULTI_KEYS,
                });
            }
            for (key, value) in pairs {
                check_key(key)?;
                check_value(value)?;
            }
        }
        WriteCommand::Delete { key } => {
            check_key(key)?;
        }
        WriteCommand::DeleteMulti { keys } => {
            if keys.len() > MAX_SETMULTI_KEYS as usize {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: keys.len(),
                    max: MAX_SETMULTI_KEYS,
                });
            }
            for key in keys {
                check_key(key)?;
            }
        }
        WriteCommand::CompareAndSwap {
            key,
            expected,
            new_value,
        } => {
            check_key(key)?;
            if let Some(exp) = expected {
                check_value(exp)?;
            }
            check_value(new_value)?;
        }
        WriteCommand::CompareAndDelete { key, expected } => {
            check_key(key)?;
            check_value(expected)?;
        }
        WriteCommand::Batch { operations } => {
            if operations.len() > MAX_SETMULTI_KEYS as usize {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: operations.len(),
                    max: MAX_SETMULTI_KEYS,
                });
            }
            for op in operations {
                match op {
                    BatchOperation::Set { key, value } => {
                        check_key(key)?;
                        check_value(value)?;
                    }
                    BatchOperation::Delete { key } => {
                        check_key(key)?;
                    }
                }
            }
        }
        WriteCommand::ConditionalBatch { conditions, operations } => {
            // Check total size of conditions + operations
            let total_size = conditions.len() + operations.len();
            if total_size > MAX_SETMULTI_KEYS as usize {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: total_size,
                    max: MAX_SETMULTI_KEYS,
                });
            }
            // Validate conditions
            for cond in conditions {
                match cond {
                    BatchCondition::ValueEquals { key, expected } => {
                        check_key(key)?;
                        check_value(expected)?;
                    }
                    BatchCondition::KeyExists { key } | BatchCondition::KeyNotExists { key } => {
                        check_key(key)?;
                    }
                }
            }
            // Validate operations
            for op in operations {
                match op {
                    BatchOperation::Set { key, value } => {
                        check_key(key)?;
                        check_value(value)?;
                    }
                    BatchOperation::Delete { key } => {
                        check_key(key)?;
                    }
                }
            }
        }
        // Transaction operations
        WriteCommand::Transaction {
            compare,
            success,
            failure,
        } => {
            // Check total size of all components
            let total_size = compare.len() + success.len() + failure.len();
            if total_size > MAX_SETMULTI_KEYS as usize {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: total_size,
                    max: MAX_SETMULTI_KEYS,
                });
            }
            // Validate comparisons
            for cmp in compare {
                check_key(&cmp.key)?;
                check_value(&cmp.value)?;
            }
            // Validate success operations
            for op in success {
                match op {
                    TxnOp::Put { key, value } => {
                        check_key(key)?;
                        check_value(value)?;
                    }
                    TxnOp::Delete { key } | TxnOp::Get { key } => {
                        check_key(key)?;
                    }
                    TxnOp::Range { prefix, limit } => {
                        check_key(prefix)?;
                        if *limit > MAX_SCAN_RESULTS {
                            return Err(KeyValueStoreError::BatchTooLarge {
                                size: *limit as usize,
                                max: MAX_SCAN_RESULTS,
                            });
                        }
                    }
                }
            }
            // Validate failure operations
            for op in failure {
                match op {
                    TxnOp::Put { key, value } => {
                        check_key(key)?;
                        check_value(value)?;
                    }
                    TxnOp::Delete { key } | TxnOp::Get { key } => {
                        check_key(key)?;
                    }
                    TxnOp::Range { prefix, limit } => {
                        check_key(prefix)?;
                        if *limit > MAX_SCAN_RESULTS {
                            return Err(KeyValueStoreError::BatchTooLarge {
                                size: *limit as usize,
                                max: MAX_SCAN_RESULTS,
                            });
                        }
                    }
                }
            }
        }
        // Lease operations
        WriteCommand::SetWithLease { key, value, .. } => {
            check_key(key)?;
            check_value(value)?;
        }
        WriteCommand::SetMultiWithLease { pairs, .. } => {
            if pairs.len() > MAX_SETMULTI_KEYS as usize {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: pairs.len(),
                    max: MAX_SETMULTI_KEYS,
                });
            }
            for (key, value) in pairs {
                check_key(key)?;
                check_value(value)?;
            }
        }
        WriteCommand::LeaseGrant { .. } | WriteCommand::LeaseRevoke { .. } | WriteCommand::LeaseKeepalive { .. } => {
            // No key/value validation needed for lease operations
        }
        WriteCommand::OptimisticTransaction { read_set, write_set } => {
            // Tiger Style: Validate batch sizes
            if read_set.len() > MAX_SETMULTI_KEYS as usize {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: read_set.len(),
                    max: MAX_SETMULTI_KEYS,
                });
            }
            if write_set.len() > MAX_SETMULTI_KEYS as usize {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: write_set.len(),
                    max: MAX_SETMULTI_KEYS,
                });
            }
            // Validate read set keys
            for (key, _) in read_set {
                check_key(key)?;
            }
            // Validate write set keys and values
            for op in write_set {
                match op {
                    WriteOp::Set { key, value } => {
                        check_key(key)?;
                        check_value(value)?;
                    }
                    WriteOp::Delete { key } => {
                        check_key(key)?;
                    }
                }
            }
        }
        // Shard topology operations
        WriteCommand::ShardSplit { split_key, .. } => {
            // Split key must be non-empty and within size limits
            check_key(split_key)?;
        }
        WriteCommand::ShardMerge { .. } => {
            // No key/value validation needed for merge operations
        }
        WriteCommand::TopologyUpdate { topology_data } => {
            // Validate topology data size (max 1MB, reasonable for up to 256 shards)
            if topology_data.len() > MAX_VALUE_SIZE as usize {
                return Err(KeyValueStoreError::ValueTooLarge {
                    size: topology_data.len(),
                    max: MAX_VALUE_SIZE,
                });
            }
        }
    }

    Ok(())
}

/// Implement KeyValueStore for `Arc<T>` where T: KeyValueStore.
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

// ============================================================================
// SQL Query Types and Traits (requires 'sql' feature)
// ============================================================================

/// Consistency level for SQL read queries.
///
/// - `Linearizable`: Query sees all prior writes (uses Raft ReadIndex protocol). This is the safe
///   default but has higher latency due to leader confirmation.
/// - `Stale`: Fast local read without consistency guarantee. May return stale data if the node is
///   behind the leader.
#[cfg(feature = "sql")]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SqlConsistency {
    /// Linearizable consistency via Raft ReadIndex protocol.
    /// Guarantees the read sees all prior committed writes.
    #[default]
    Linearizable,
    /// Stale read from local state machine.
    /// Fast but may return outdated data.
    Stale,
}

/// A typed SQL value preserving SQLite's type system.
///
/// SQLite has a dynamic type system with 5 storage classes.
/// This enum preserves the type information from query results.
#[cfg(feature = "sql")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SqlValue {
    /// SQL NULL value.
    Null,
    /// 64-bit signed integer (SQLite INTEGER).
    Integer(i64),
    /// 64-bit floating point (SQLite REAL).
    Real(f64),
    /// UTF-8 text string (SQLite TEXT).
    Text(String),
    /// Binary data (SQLite BLOB).
    Blob(Vec<u8>),
}

/// Information about a result column.
#[cfg(feature = "sql")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SqlColumnInfo {
    /// Column name (or alias if specified in query).
    pub name: String,
}

/// Request to execute a read-only SQL query.
///
/// Tiger Style: All parameters have fixed bounds to prevent resource exhaustion.
#[cfg(feature = "sql")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SqlQueryRequest {
    /// SQL query string. Must be a SELECT statement (or WITH...SELECT for CTEs).
    /// Maximum size: MAX_SQL_QUERY_SIZE (64 KB).
    pub query: String,
    /// Query parameters for prepared statement binding (?1, ?2, ...).
    /// Maximum count: MAX_SQL_PARAMS (100).
    pub params: Vec<SqlValue>,
    /// Consistency level for the read operation.
    pub consistency: SqlConsistency,
    /// Maximum number of rows to return.
    /// Default: DEFAULT_SQL_RESULT_ROWS (1,000), Max: MAX_SQL_RESULT_ROWS (10,000).
    pub limit: Option<u32>,
    /// Query timeout in milliseconds.
    /// Default: DEFAULT_SQL_TIMEOUT_MS (5,000), Max: MAX_SQL_TIMEOUT_MS (30,000).
    pub timeout_ms: Option<u32>,
}

/// Result of a SQL query execution.
///
/// Tiger Style: Bounded results with explicit truncation indicator.
#[cfg(feature = "sql")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SqlQueryResult {
    /// Column metadata for the result set.
    pub columns: Vec<SqlColumnInfo>,
    /// Result rows, each containing values in column order.
    pub rows: Vec<Vec<SqlValue>>,
    /// Number of rows returned.
    pub row_count: u32,
    /// True if more rows exist but were not returned due to limit.
    pub is_truncated: bool,
    /// Query execution time in milliseconds.
    pub execution_time_ms: u64,
}

/// Errors that can occur during SQL query execution.
#[cfg(feature = "sql")]
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SqlQueryError {
    /// Query is not a valid read-only statement (contains write operations).
    #[error("query not allowed: {reason}")]
    QueryNotAllowed {
        /// Explanation of why the query is not allowed.
        reason: String,
    },
    /// SQL syntax error or invalid query.
    #[error("SQL syntax error: {message}")]
    SyntaxError {
        /// Error message from the SQL parser or validator.
        message: String,
    },
    /// Query execution failed.
    #[error("execution failed: {reason}")]
    ExecutionFailed {
        /// Description of the execution failure.
        reason: String,
    },
    /// Query exceeded timeout.
    #[error("query timed out after {duration_ms}ms")]
    Timeout {
        /// Duration in milliseconds before the query timed out.
        duration_ms: u64,
    },
    /// Not the Raft leader (for linearizable reads).
    #[error("not leader; current leader: {leader:?}")]
    NotLeader {
        /// The current leader node ID, if known.
        leader: Option<u64>,
    },
    /// Query size exceeds limit.
    #[error("query size {size} exceeds maximum of {max} bytes")]
    QueryTooLarge {
        /// Actual size of the query in bytes.
        size: usize,
        /// Maximum allowed query size in bytes.
        max: u32,
    },
    /// Too many parameters.
    #[error("parameter count {count} exceeds maximum of {max}")]
    TooManyParams {
        /// Actual number of parameters provided.
        count: usize,
        /// Maximum allowed parameter count.
        max: u32,
    },
    /// SQL queries not supported by this backend.
    #[error("SQL queries not supported by {backend} backend")]
    NotSupported {
        /// Name of the backend implementation.
        backend: String,
    },
}

/// Validate a SQL query request against Tiger Style bounds.
///
/// Checks:
/// - Query size <= MAX_SQL_QUERY_SIZE
/// - Parameter count <= MAX_SQL_PARAMS
/// - Limit <= MAX_SQL_RESULT_ROWS (if specified)
/// - Timeout <= MAX_SQL_TIMEOUT_MS (if specified)
#[cfg(feature = "sql")]
pub fn validate_sql_request(request: &SqlQueryRequest) -> Result<(), SqlQueryError> {
    // Check query size
    if request.query.len() > MAX_SQL_QUERY_SIZE as usize {
        return Err(SqlQueryError::QueryTooLarge {
            size: request.query.len(),
            max: MAX_SQL_QUERY_SIZE,
        });
    }

    // Check parameter count
    if request.params.len() > MAX_SQL_PARAMS as usize {
        return Err(SqlQueryError::TooManyParams {
            count: request.params.len(),
            max: MAX_SQL_PARAMS,
        });
    }

    // Validate and clamp limit (done in execute, but check upper bound here)
    if let Some(limit) = request.limit
        && limit > MAX_SQL_RESULT_ROWS
    {
        return Err(SqlQueryError::QueryNotAllowed {
            reason: format!("limit {} exceeds maximum of {} rows", limit, MAX_SQL_RESULT_ROWS),
        });
    }

    // Validate timeout
    if let Some(timeout) = request.timeout_ms
        && timeout > MAX_SQL_TIMEOUT_MS
    {
        return Err(SqlQueryError::QueryNotAllowed {
            reason: format!("timeout {}ms exceeds maximum of {}ms", timeout, MAX_SQL_TIMEOUT_MS),
        });
    }

    Ok(())
}

/// Get effective limit, applying defaults and bounds.
#[cfg(feature = "sql")]
pub fn effective_sql_limit(request_limit: Option<u32>) -> u32 {
    request_limit.unwrap_or(DEFAULT_SQL_RESULT_ROWS).min(MAX_SQL_RESULT_ROWS)
}

/// Get effective timeout in milliseconds, applying defaults and bounds.
#[cfg(feature = "sql")]
pub fn effective_sql_timeout_ms(request_timeout: Option<u32>) -> u32 {
    request_timeout.unwrap_or(DEFAULT_SQL_TIMEOUT_MS).min(MAX_SQL_TIMEOUT_MS)
}

/// SQL query executor interface.
///
/// Provides read-only SQL query execution against the state machine.
/// This is a separate trait from `KeyValueStore` because:
/// 1. Not all backends support SQL (in-memory backend uses HashMap)
/// 2. SQL queries have different consistency and timeout semantics
/// 3. Keeps the KeyValueStore interface simple
///
/// Tiger Style: Queries have bounded size limits to prevent resource exhaustion.
#[cfg(feature = "sql")]
#[async_trait]
pub trait SqlQueryExecutor: Send + Sync {
    /// Execute a read-only SQL query against the state machine.
    ///
    /// Only SELECT statements (and WITH...SELECT for CTEs) are allowed.
    /// The query is executed with `PRAGMA query_only = ON` for defense-in-depth.
    ///
    /// # Consistency
    ///
    /// - `Linearizable`: Uses Raft ReadIndex to ensure the read sees all committed writes. Higher
    ///   latency but consistent.
    /// - `Stale`: Executes directly on local state machine. Fast but may return stale data if this
    ///   node is behind the leader.
    ///
    /// # Errors
    ///
    /// - `QueryNotAllowed`: Query contains write operations or forbidden keywords
    /// - `SyntaxError`: Invalid SQL syntax
    /// - `ExecutionFailed`: Query execution error (e.g., no such table)
    /// - `Timeout`: Query exceeded timeout_ms
    /// - `NotLeader`: Linearizable read on non-leader (includes leader hint)
    /// - `QueryTooLarge`: Query exceeds MAX_SQL_QUERY_SIZE
    /// - `TooManyParams`: Parameters exceed MAX_SQL_PARAMS
    /// - `NotSupported`: Backend doesn't support SQL queries
    async fn execute_sql(&self, request: SqlQueryRequest) -> Result<SqlQueryResult, SqlQueryError>;
}

/// Blanket implementation for `Arc<T>` where T: SqlQueryExecutor.
#[cfg(feature = "sql")]
#[async_trait]
impl<T: SqlQueryExecutor + ?Sized> SqlQueryExecutor for std::sync::Arc<T> {
    async fn execute_sql(&self, request: SqlQueryRequest) -> Result<SqlQueryResult, SqlQueryError> {
        (**self).execute_sql(request).await
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn empty_key_rejected() {
        let cmd = WriteCommand::Set {
            key: "".into(),
            value: "v".into(),
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::EmptyKey)));
    }

    #[test]
    fn valid_key_accepted() {
        let cmd = WriteCommand::Set {
            key: "k".into(),
            value: "v".into(),
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn key_too_large_rejected() {
        let big_key = "x".repeat(MAX_KEY_SIZE as usize + 1);
        let cmd = WriteCommand::Set {
            key: big_key,
            value: "v".into(),
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::KeyTooLarge { .. })));
    }

    #[test]
    fn value_too_large_rejected() {
        let big_value = "x".repeat(MAX_VALUE_SIZE as usize + 1);
        let cmd = WriteCommand::Set {
            key: "k".into(),
            value: big_value,
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::ValueTooLarge { .. })));
    }

    #[test]
    fn batch_too_large_rejected() {
        let pairs: Vec<_> = (0..MAX_SETMULTI_KEYS + 1).map(|i| (format!("k{}", i), "v".into())).collect();
        let cmd = WriteCommand::SetMulti { pairs };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::BatchTooLarge { .. })));
    }

    #[test]
    fn transaction_total_size_validated() {
        let compare: Vec<_> = (0..50)
            .map(|i| TxnCompare {
                key: format!("k{}", i),
                target: CompareTarget::Value,
                op: CompareOp::Equal,
                value: "v".into(),
            })
            .collect();
        let success: Vec<_> = (0..51).map(|i| TxnOp::Get { key: format!("k{}", i) }).collect();
        let cmd = WriteCommand::Transaction {
            compare,
            success,
            failure: vec![],
        };
        // Total = 50 + 51 = 101 > MAX_SETMULTI_KEYS (100)
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::BatchTooLarge { .. })));
    }

    #[test]
    fn empty_key_in_setmulti_rejected() {
        let cmd = WriteCommand::SetMulti {
            pairs: vec![("".into(), "v".into())],
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::EmptyKey)));
    }

    #[test]
    fn empty_key_in_transaction_rejected() {
        let cmd = WriteCommand::Transaction {
            compare: vec![TxnCompare {
                key: "".into(),
                target: CompareTarget::Value,
                op: CompareOp::Equal,
                value: "v".into(),
            }],
            success: vec![],
            failure: vec![],
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::EmptyKey)));
    }

    #[test]
    fn empty_key_in_delete_rejected() {
        let cmd = WriteCommand::Delete { key: "".into() };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::EmptyKey)));
    }

    #[test]
    fn empty_key_in_deletemulti_rejected() {
        let cmd = WriteCommand::DeleteMulti { keys: vec!["".into()] };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::EmptyKey)));
    }

    #[test]
    fn empty_key_in_compare_and_swap_rejected() {
        let cmd = WriteCommand::CompareAndSwap {
            key: "".into(),
            expected: None,
            new_value: "v".into(),
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::EmptyKey)));
    }

    #[test]
    fn boundary_key_size_accepted() {
        let key = "x".repeat(MAX_KEY_SIZE as usize);
        let cmd = WriteCommand::Set { key, value: "v".into() };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn boundary_value_size_accepted() {
        let value = "x".repeat(MAX_VALUE_SIZE as usize);
        let cmd = WriteCommand::Set { key: "k".into(), value };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn boundary_batch_size_accepted() {
        let pairs: Vec<_> = (0..MAX_SETMULTI_KEYS).map(|i| (format!("k{}", i), "v".into())).collect();
        let cmd = WriteCommand::SetMulti { pairs };
        assert!(validate_write_command(&cmd).is_ok());
    }
}
