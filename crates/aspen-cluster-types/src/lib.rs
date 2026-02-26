//! Cluster management types for Aspen distributed systems.
//!
//! This crate provides types for managing cluster membership and topology,
//! including node identifiers, addresses, states, and error types.
//!
//! ## Key Types
//!
//! - [`NodeId`]: Type-safe node identifier
//! - [`NodeAddress`]: P2P endpoint address wrapper
//! - [`NodeState`]: Current state of a node in the Raft cluster
//! - [`ClusterNode`]: Node participating in the control-plane cluster
//! - [`ClusterState`]: Snapshot of cluster topology
//! - [`ClusterMetrics`]: Raft metrics wrapper
//! - [`ControlPlaneError`]: Errors for cluster control plane operations

use std::collections::BTreeMap;
use std::collections::HashSet;

use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during cluster control plane operations.
///
/// These errors indicate failures in cluster management operations like
/// initialization, membership changes, and state queries.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ControlPlaneError {
    /// The request contained invalid parameters or configuration.
    #[error("invalid request: {reason}")]
    InvalidRequest {
        /// Human-readable description of what was invalid in the request.
        reason: String,
    },

    /// The cluster has not been initialized yet.
    #[error("cluster not initialized")]
    NotInitialized,

    /// The operation failed due to a Raft or internal error.
    #[error("operation failed: {reason}")]
    Failed {
        /// Human-readable description of the failure.
        reason: String,
    },

    /// The operation is not supported by this backend implementation.
    #[error("operation not supported by {backend} backend: {operation}")]
    Unsupported {
        /// Name of the backend implementation (e.g., "in-memory", "sqlite").
        backend: String,
        /// Name of the unsupported operation.
        operation: String,
    },

    /// The operation timed out.
    #[error("operation timed out after {duration_ms}ms")]
    Timeout {
        /// Duration in milliseconds before timeout.
        duration_ms: u64,
    },
}

// ============================================================================
// NodeAddress - P2P endpoint address wrapper
// ============================================================================

/// P2P endpoint address for connecting to a node.
///
/// This type wraps `iroh::EndpointAddr` to decouple the public API from the
/// underlying iroh implementation. It provides the same functionality while
/// allowing internal implementation changes without breaking the public API.
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
    pub fn inner(&self) -> &iroh::EndpointAddr {
        &self.0
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

// ============================================================================
// NodeState - Raft node state wrapper
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

// ============================================================================
// NodeId - Type-safe node identifier
// ============================================================================

/// Type-safe node identifier for Raft cluster nodes.
///
/// This newtype wrapper around `u64` prevents accidental mixing with other
/// numeric types like log indices, term numbers, or port numbers.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Serialize,
    Deserialize
)]
pub struct NodeId(pub u64);

impl NodeId {
    /// Create a new `NodeId` from a raw `u64`.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for NodeId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<NodeId> for u64 {
    fn from(value: NodeId) -> Self {
        value.0
    }
}

impl std::str::FromStr for NodeId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u64>().map(NodeId)
    }
}

// ============================================================================
// ClusterMetrics - Raft metrics wrapper
// ============================================================================

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

// ============================================================================
// SnapshotLogId - Snapshot position wrapper
// ============================================================================

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

// ============================================================================
// Cluster Management Types
// ============================================================================

/// Describes a node participating in the control-plane cluster.
///
/// Contains both the node's identifier and its P2P endpoint address,
/// which is stored in Raft membership state for persistent discovery.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterNode {
    /// Unique identifier for this node within the cluster.
    pub id: u64,
    /// Display address for logging and human-readable output.
    pub addr: String,
    /// Optional legacy Raft address (host:port) for backwards compatibility.
    pub raft_addr: Option<String>,
    /// P2P endpoint address for connecting to this node.
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

    /// Create a ClusterNode with an iroh EndpointAddr.
    pub fn with_iroh_addr(id: u64, iroh_addr: iroh::EndpointAddr) -> Self {
        Self::with_node_addr(id, NodeAddress::new(iroh_addr))
    }

    /// Get the node address as an iroh EndpointAddr, if available.
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InitRequest {
    /// The founding voting members of the cluster.
    pub initial_members: Vec<ClusterNode>,
}

impl InitRequest {
    /// Validate the initialization request.
    ///
    /// Returns an error if:
    /// - `initial_members` is empty
    /// - Any node ID is zero (reserved in Raft)
    /// - Any two nodes have the same ID
    pub fn validate(&self) -> Result<(), ControlPlaneError> {
        if self.initial_members.is_empty() {
            return Err(ControlPlaneError::InvalidRequest {
                reason: "initial_members must not be empty".into(),
            });
        }

        // Check for node ID zero (reserved)
        for node in &self.initial_members {
            if node.id == 0 {
                return Err(ControlPlaneError::InvalidRequest {
                    reason: "node ID 0 is reserved and cannot be used".into(),
                });
            }
        }

        // Check for duplicate node IDs
        let mut seen_ids = HashSet::new();
        for node in &self.initial_members {
            if !seen_ids.insert(node.id) {
                return Err(ControlPlaneError::InvalidRequest {
                    reason: format!("duplicate node ID {} in initial_members", node.id),
                });
            }
        }

        Ok(())
    }
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
    pub members: Vec<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // ControlPlaneError tests
    // ============================================================================

    #[test]
    fn control_plane_error_invalid_request_display() {
        let err = ControlPlaneError::InvalidRequest {
            reason: "missing required field".to_string(),
        };
        assert_eq!(err.to_string(), "invalid request: missing required field");
    }

    #[test]
    fn control_plane_error_not_initialized_display() {
        let err = ControlPlaneError::NotInitialized;
        assert_eq!(err.to_string(), "cluster not initialized");
    }

    #[test]
    fn control_plane_error_failed_display() {
        let err = ControlPlaneError::Failed {
            reason: "network timeout".to_string(),
        };
        assert_eq!(err.to_string(), "operation failed: network timeout");
    }

    #[test]
    fn control_plane_error_unsupported_display() {
        let err = ControlPlaneError::Unsupported {
            backend: "in-memory".to_string(),
            operation: "get_metrics".to_string(),
        };
        assert_eq!(err.to_string(), "operation not supported by in-memory backend: get_metrics");
    }

    #[test]
    fn control_plane_error_timeout_display() {
        let err = ControlPlaneError::Timeout { duration_ms: 5000 };
        assert_eq!(err.to_string(), "operation timed out after 5000ms");
    }

    #[test]
    fn control_plane_error_clone() {
        let err = ControlPlaneError::InvalidRequest {
            reason: "test".to_string(),
        };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn control_plane_error_equality() {
        let err1 = ControlPlaneError::NotInitialized;
        let err2 = ControlPlaneError::NotInitialized;
        let err3 = ControlPlaneError::Timeout { duration_ms: 100 };

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn control_plane_error_debug() {
        let err = ControlPlaneError::Failed {
            reason: "debug test".to_string(),
        };
        let debug = format!("{:?}", err);
        assert!(debug.contains("Failed"));
        assert!(debug.contains("debug test"));
    }

    // ============================================================================
    // NodeId tests
    // ============================================================================

    #[test]
    fn node_id_new() {
        let id = NodeId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn node_id_from_u64() {
        let id: NodeId = 123.into();
        assert_eq!(id.0, 123);
    }

    #[test]
    fn node_id_into_u64() {
        let id = NodeId::new(456);
        let raw: u64 = id.into();
        assert_eq!(raw, 456);
    }

    #[test]
    fn node_id_display() {
        let id = NodeId::new(789);
        assert_eq!(format!("{}", id), "789");
    }

    #[test]
    fn node_id_from_str() {
        let id: NodeId = "999".parse().unwrap();
        assert_eq!(id.0, 999);
    }

    #[test]
    fn node_id_from_str_invalid() {
        let result: Result<NodeId, _> = "not_a_number".parse();
        assert!(result.is_err());
    }

    #[test]
    fn node_id_from_str_negative() {
        let result: Result<NodeId, _> = "-5".parse();
        assert!(result.is_err());
    }

    #[test]
    fn node_id_ordering() {
        let id1 = NodeId::new(1);
        let id2 = NodeId::new(2);
        let id3 = NodeId::new(1);

        assert!(id1 < id2);
        assert!(id2 > id1);
        assert!(id1 <= id3);
        assert!(id1 >= id3);
    }

    #[test]
    fn node_id_equality() {
        let id1 = NodeId::new(42);
        let id2 = NodeId::new(42);
        let id3 = NodeId::new(43);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn node_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(NodeId::new(1));
        set.insert(NodeId::new(2));
        set.insert(NodeId::new(1)); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&NodeId::new(1)));
        assert!(set.contains(&NodeId::new(2)));
    }

    #[test]
    fn node_id_default() {
        let id = NodeId::default();
        assert_eq!(id.0, 0);
    }

    #[test]
    fn node_id_clone() {
        let id = NodeId::new(100);
        let cloned = id.clone();
        assert_eq!(id, cloned);
    }

    #[test]
    fn node_id_copy() {
        let id = NodeId::new(200);
        let copied = id; // Copy, not move
        assert_eq!(id, copied);
    }

    #[test]
    fn node_id_serialization_roundtrip() {
        let id = NodeId::new(12345);
        let json = serde_json::to_string(&id).expect("serialize");
        let deserialized: NodeId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(id, deserialized);
    }

    #[test]
    fn node_id_max_value() {
        let id = NodeId::new(u64::MAX);
        assert_eq!(id.0, u64::MAX);
        assert_eq!(format!("{}", id), format!("{}", u64::MAX));
    }

    // ============================================================================
    // NodeState tests
    // ============================================================================

    #[test]
    fn node_state_is_leader() {
        assert!(NodeState::Leader.is_leader());
        assert!(!NodeState::Follower.is_leader());
        assert!(!NodeState::Candidate.is_leader());
        assert!(!NodeState::Learner.is_leader());
        assert!(!NodeState::Shutdown.is_leader());
    }

    #[test]
    fn node_state_can_serve_reads() {
        assert!(NodeState::Leader.can_serve_reads());
        assert!(NodeState::Follower.can_serve_reads());
        assert!(!NodeState::Candidate.can_serve_reads());
        assert!(!NodeState::Learner.can_serve_reads());
        assert!(!NodeState::Shutdown.can_serve_reads());
    }

    #[test]
    fn node_state_is_healthy() {
        assert!(NodeState::Leader.is_healthy());
        assert!(NodeState::Follower.is_healthy());
        assert!(NodeState::Candidate.is_healthy());
        assert!(NodeState::Learner.is_healthy());
        assert!(!NodeState::Shutdown.is_healthy());
    }

    #[test]
    fn node_state_as_u8() {
        assert_eq!(NodeState::Learner.as_u8(), 0);
        assert_eq!(NodeState::Follower.as_u8(), 1);
        assert_eq!(NodeState::Candidate.as_u8(), 2);
        assert_eq!(NodeState::Leader.as_u8(), 3);
        assert_eq!(NodeState::Shutdown.as_u8(), 4);
    }

    #[test]
    fn node_state_equality() {
        assert_eq!(NodeState::Leader, NodeState::Leader);
        assert_ne!(NodeState::Leader, NodeState::Follower);
    }

    #[test]
    fn node_state_clone() {
        let state = NodeState::Candidate;
        let cloned = state.clone();
        assert_eq!(state, cloned);
    }

    #[test]
    fn node_state_copy() {
        let state = NodeState::Follower;
        let copied = state; // Copy
        assert_eq!(state, copied);
    }

    #[test]
    fn node_state_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(NodeState::Leader);
        set.insert(NodeState::Follower);
        set.insert(NodeState::Leader); // Duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn node_state_debug() {
        let debug = format!("{:?}", NodeState::Candidate);
        assert_eq!(debug, "Candidate");
    }

    // ============================================================================
    // SnapshotLogId tests
    // ============================================================================

    #[test]
    fn snapshot_log_id_creation() {
        let id = SnapshotLogId { term: 5, index: 100 };
        assert_eq!(id.term, 5);
        assert_eq!(id.index, 100);
    }

    #[test]
    fn snapshot_log_id_equality() {
        let id1 = SnapshotLogId { term: 1, index: 10 };
        let id2 = SnapshotLogId { term: 1, index: 10 };
        let id3 = SnapshotLogId { term: 2, index: 10 };
        let id4 = SnapshotLogId { term: 1, index: 20 };

        assert_eq!(id1, id2);
        assert_ne!(id1, id3); // Different term
        assert_ne!(id1, id4); // Different index
    }

    #[test]
    fn snapshot_log_id_clone() {
        let id = SnapshotLogId { term: 3, index: 50 };
        let cloned = id.clone();
        assert_eq!(id, cloned);
    }

    #[test]
    fn snapshot_log_id_copy() {
        let id = SnapshotLogId { term: 4, index: 60 };
        let copied = id; // Copy
        assert_eq!(id, copied);
    }

    #[test]
    fn snapshot_log_id_debug() {
        let id = SnapshotLogId { term: 7, index: 42 };
        let debug = format!("{:?}", id);
        assert!(debug.contains("term: 7"));
        assert!(debug.contains("index: 42"));
    }

    // ============================================================================
    // ClusterMetrics tests
    // ============================================================================

    #[test]
    fn cluster_metrics_creation() {
        let metrics = ClusterMetrics {
            id: 1,
            state: NodeState::Leader,
            current_leader: Some(1),
            current_term: 5,
            last_log_index: Some(100),
            last_applied_index: Some(95),
            snapshot_index: Some(50),
            replication: None,
            voters: vec![1, 2, 3],
            learners: vec![4],
        };

        assert_eq!(metrics.id, 1);
        assert_eq!(metrics.state, NodeState::Leader);
        assert_eq!(metrics.current_leader, Some(1));
        assert_eq!(metrics.current_term, 5);
    }

    #[test]
    fn cluster_metrics_with_replication() {
        let mut replication = BTreeMap::new();
        replication.insert(2, Some(90));
        replication.insert(3, Some(85));

        let metrics = ClusterMetrics {
            id: 1,
            state: NodeState::Leader,
            current_leader: Some(1),
            current_term: 10,
            last_log_index: Some(100),
            last_applied_index: Some(100),
            snapshot_index: None,
            replication: Some(replication),
            voters: vec![1, 2, 3],
            learners: vec![],
        };

        assert!(metrics.replication.is_some());
        let rep = metrics.replication.as_ref().unwrap();
        assert_eq!(rep.get(&2), Some(&Some(90)));
        assert_eq!(rep.get(&3), Some(&Some(85)));
    }

    #[test]
    fn cluster_metrics_follower() {
        let metrics = ClusterMetrics {
            id: 2,
            state: NodeState::Follower,
            current_leader: Some(1),
            current_term: 3,
            last_log_index: Some(50),
            last_applied_index: Some(50),
            snapshot_index: None,
            replication: None, // Followers don't track replication
            voters: vec![1, 2, 3],
            learners: vec![],
        };

        assert!(!metrics.state.is_leader());
        assert!(metrics.state.can_serve_reads());
    }

    #[test]
    fn cluster_metrics_clone() {
        let metrics = ClusterMetrics {
            id: 5,
            state: NodeState::Candidate,
            current_leader: None,
            current_term: 1,
            last_log_index: None,
            last_applied_index: None,
            snapshot_index: None,
            replication: None,
            voters: vec![],
            learners: vec![],
        };

        let cloned = metrics.clone();
        assert_eq!(cloned.id, metrics.id);
        assert_eq!(cloned.state, metrics.state);
    }

    #[test]
    fn cluster_metrics_debug() {
        let metrics = ClusterMetrics {
            id: 1,
            state: NodeState::Leader,
            current_leader: Some(1),
            current_term: 42,
            last_log_index: Some(999),
            last_applied_index: Some(998),
            snapshot_index: Some(500),
            replication: None,
            voters: vec![1, 2, 3],
            learners: vec![4, 5],
        };

        let debug = format!("{:?}", metrics);
        assert!(debug.contains("id: 1"));
        assert!(debug.contains("Leader"));
        assert!(debug.contains("current_term: 42"));
    }

    // ============================================================================
    // NodeAddress tests
    // ============================================================================

    fn create_test_endpoint_addr() -> iroh::EndpointAddr {
        use iroh::EndpointAddr;
        use iroh::SecretKey;

        let mut seed = [0u8; 32];
        seed[0] = 42; // Deterministic seed
        let secret_key = SecretKey::from(seed);
        let endpoint_id = secret_key.public();
        EndpointAddr::new(endpoint_id)
    }

    #[test]
    fn node_address_new() {
        let iroh_addr = create_test_endpoint_addr();
        let node_addr = NodeAddress::new(iroh_addr.clone());

        assert_eq!(node_addr.inner(), &iroh_addr);
    }

    #[test]
    fn node_address_id() {
        let iroh_addr = create_test_endpoint_addr();
        let node_addr = NodeAddress::new(iroh_addr.clone());

        let id_str = node_addr.id();
        // Should match the iroh endpoint's public key as string
        assert_eq!(id_str, iroh_addr.id.to_string());
    }

    #[test]
    fn node_address_inner() {
        let iroh_addr = create_test_endpoint_addr();
        let node_addr = NodeAddress::new(iroh_addr.clone());

        assert_eq!(node_addr.inner(), &iroh_addr);
    }

    #[test]
    fn node_address_from_endpoint_addr() {
        let iroh_addr = create_test_endpoint_addr();
        let node_addr: NodeAddress = iroh_addr.clone().into();

        assert_eq!(node_addr.inner(), &iroh_addr);
    }

    #[test]
    fn node_address_into_endpoint_addr() {
        let iroh_addr = create_test_endpoint_addr();
        let node_addr = NodeAddress::new(iroh_addr.clone());

        let recovered: iroh::EndpointAddr = node_addr.into();
        assert_eq!(recovered, iroh_addr);
    }

    #[test]
    fn node_address_display() {
        let iroh_addr = create_test_endpoint_addr();
        let node_addr = NodeAddress::new(iroh_addr.clone());

        let display = format!("{}", node_addr);
        // Display should show the public key ID
        assert_eq!(display, iroh_addr.id.to_string());
    }

    #[test]
    fn node_address_clone() {
        let iroh_addr = create_test_endpoint_addr();
        let node_addr = NodeAddress::new(iroh_addr);
        let cloned = node_addr.clone();

        assert_eq!(node_addr, cloned);
    }

    #[test]
    fn node_address_equality() {
        let iroh_addr = create_test_endpoint_addr();
        let addr1 = NodeAddress::new(iroh_addr.clone());
        let addr2 = NodeAddress::new(iroh_addr);

        assert_eq!(addr1, addr2);
    }

    #[test]
    fn node_address_debug() {
        let iroh_addr = create_test_endpoint_addr();
        let node_addr = NodeAddress::new(iroh_addr);

        let debug = format!("{:?}", node_addr);
        assert!(debug.contains("NodeAddress"));
    }

    // ============================================================================
    // ClusterNode tests
    // ============================================================================

    #[test]
    fn cluster_node_new_creates_with_legacy_address() {
        let node = ClusterNode::new(1, "127.0.0.1:5000", Some("127.0.0.1:5001".to_string()));
        assert_eq!(node.id, 1);
        assert_eq!(node.addr, "127.0.0.1:5000");
        assert_eq!(node.raft_addr, Some("127.0.0.1:5001".to_string()));
        assert!(node.node_addr.is_none());
    }

    #[test]
    fn cluster_node_new_without_raft_addr() {
        let node = ClusterNode::new(42, "localhost", None);
        assert_eq!(node.id, 42);
        assert_eq!(node.addr, "localhost");
        assert!(node.raft_addr.is_none());
        assert!(node.node_addr.is_none());
    }

    #[test]
    fn cluster_node_iroh_addr_returns_none_when_not_set() {
        let node = ClusterNode::new(1, "test", None);
        assert!(node.iroh_addr().is_none());
    }

    #[test]
    fn cluster_node_equality() {
        let node1 = ClusterNode::new(1, "addr1", None);
        let node2 = ClusterNode::new(1, "addr1", None);
        let node3 = ClusterNode::new(2, "addr1", None);
        let node4 = ClusterNode::new(1, "addr2", None);

        assert_eq!(node1, node2);
        assert_ne!(node1, node3); // Different ID
        assert_ne!(node1, node4); // Different addr
    }

    #[test]
    fn cluster_node_clone() {
        let node = ClusterNode::new(5, "cloned", Some("raft".to_string()));
        let cloned = node.clone();
        assert_eq!(node, cloned);
    }

    #[test]
    fn cluster_node_debug_format() {
        let node = ClusterNode::new(1, "test", None);
        let debug = format!("{:?}", node);
        assert!(debug.contains("id: 1"));
        assert!(debug.contains("addr: \"test\""));
    }

    #[test]
    fn cluster_node_with_node_addr() {
        let iroh_addr = create_test_endpoint_addr();
        let node_addr = NodeAddress::new(iroh_addr.clone());
        let node = ClusterNode::with_node_addr(42, node_addr);

        assert_eq!(node.id, 42);
        assert_eq!(node.addr, iroh_addr.id.to_string());
        assert!(node.raft_addr.is_none());
        assert!(node.node_addr.is_some());
        assert_eq!(node.iroh_addr(), Some(&iroh_addr));
    }

    #[test]
    fn cluster_node_with_iroh_addr() {
        let iroh_addr = create_test_endpoint_addr();
        let node = ClusterNode::with_iroh_addr(123, iroh_addr.clone());

        assert_eq!(node.id, 123);
        assert_eq!(node.addr, iroh_addr.id.to_string());
        assert!(node.raft_addr.is_none());
        assert!(node.node_addr.is_some());
        assert_eq!(node.iroh_addr(), Some(&iroh_addr));
    }

    #[test]
    fn cluster_node_serialization_roundtrip() {
        let node = ClusterNode::new(99, "serialize-me", Some("raft-addr".to_string()));
        let json = serde_json::to_string(&node).expect("serialize");
        let deserialized: ClusterNode = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(node, deserialized);
    }

    // ============================================================================
    // ClusterState tests
    // ============================================================================

    #[test]
    fn cluster_state_default_is_empty() {
        let state = ClusterState::default();
        assert!(state.nodes.is_empty());
        assert!(state.members.is_empty());
        assert!(state.learners.is_empty());
    }

    #[test]
    fn cluster_state_with_nodes() {
        let state = ClusterState {
            nodes: vec![ClusterNode::new(1, "node1", None), ClusterNode::new(2, "node2", None)],
            members: vec![1, 2],
            learners: vec![],
        };
        assert_eq!(state.nodes.len(), 2);
        assert_eq!(state.members, vec![1, 2]);
    }

    #[test]
    fn cluster_state_clone() {
        let state = ClusterState {
            nodes: vec![ClusterNode::new(1, "n1", None)],
            members: vec![1],
            learners: vec![ClusterNode::new(2, "learner", None)],
        };
        let cloned = state.clone();
        assert_eq!(state, cloned);
    }

    #[test]
    fn cluster_state_serialization_roundtrip() {
        let state = ClusterState {
            nodes: vec![ClusterNode::new(1, "node", None)],
            members: vec![1],
            learners: vec![],
        };
        let json = serde_json::to_string(&state).expect("serialize");
        let deserialized: ClusterState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(state, deserialized);
    }

    // ============================================================================
    // InitRequest validation tests
    // ============================================================================

    #[test]
    fn init_request_validate_empty_members_fails() {
        let request = InitRequest {
            initial_members: vec![],
        };
        let result = request.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            ControlPlaneError::InvalidRequest { reason } => {
                assert!(reason.contains("must not be empty"));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn init_request_validate_node_id_zero_fails() {
        let request = InitRequest {
            initial_members: vec![ClusterNode::new(0, "zero-node", None)],
        };
        let result = request.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            ControlPlaneError::InvalidRequest { reason } => {
                assert!(reason.contains("node ID 0 is reserved"));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn init_request_validate_node_id_zero_in_middle_fails() {
        let request = InitRequest {
            initial_members: vec![
                ClusterNode::new(1, "node1", None),
                ClusterNode::new(0, "zero-node", None),
                ClusterNode::new(2, "node2", None),
            ],
        };
        let result = request.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            ControlPlaneError::InvalidRequest { reason } => {
                assert!(reason.contains("node ID 0 is reserved"));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn init_request_validate_duplicate_node_ids_fails() {
        let request = InitRequest {
            initial_members: vec![
                ClusterNode::new(1, "node1", None),
                ClusterNode::new(2, "node2", None),
                ClusterNode::new(1, "node1-dup", None), // Duplicate ID
            ],
        };
        let result = request.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            ControlPlaneError::InvalidRequest { reason } => {
                assert!(reason.contains("duplicate node ID 1"));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn init_request_validate_single_valid_node_succeeds() {
        let request = InitRequest {
            initial_members: vec![ClusterNode::new(1, "single-node", None)],
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn init_request_validate_multiple_valid_nodes_succeeds() {
        let request = InitRequest {
            initial_members: vec![
                ClusterNode::new(1, "node1", None),
                ClusterNode::new(2, "node2", None),
                ClusterNode::new(3, "node3", None),
            ],
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn init_request_validate_large_node_ids_succeeds() {
        let request = InitRequest {
            initial_members: vec![
                ClusterNode::new(u64::MAX, "max-id", None),
                ClusterNode::new(u64::MAX - 1, "almost-max", None),
            ],
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn init_request_serialization_roundtrip() {
        let request = InitRequest {
            initial_members: vec![
                ClusterNode::new(1, "node1", Some("raft1".to_string())),
                ClusterNode::new(2, "node2", None),
            ],
        };
        let json = serde_json::to_string(&request).expect("serialize");
        let deserialized: InitRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(request, deserialized);
    }

    // ============================================================================
    // AddLearnerRequest tests
    // ============================================================================

    #[test]
    fn add_learner_request_creation() {
        let request = AddLearnerRequest {
            learner: ClusterNode::new(5, "learner-node", None),
        };
        assert_eq!(request.learner.id, 5);
        assert_eq!(request.learner.addr, "learner-node");
    }

    #[test]
    fn add_learner_request_serialization_roundtrip() {
        let request = AddLearnerRequest {
            learner: ClusterNode::new(10, "learner", Some("raft".to_string())),
        };
        let json = serde_json::to_string(&request).expect("serialize");
        let deserialized: AddLearnerRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(request, deserialized);
    }

    // ============================================================================
    // ChangeMembershipRequest tests
    // ============================================================================

    #[test]
    fn change_membership_request_creation() {
        let request = ChangeMembershipRequest { members: vec![1, 2, 3] };
        assert_eq!(request.members, vec![1, 2, 3]);
    }

    #[test]
    fn change_membership_request_empty_members() {
        let request = ChangeMembershipRequest { members: vec![] };
        assert!(request.members.is_empty());
    }

    #[test]
    fn change_membership_request_serialization_roundtrip() {
        let request = ChangeMembershipRequest {
            members: vec![1, 2, 3, 4, 5],
        };
        let json = serde_json::to_string(&request).expect("serialize");
        let deserialized: ChangeMembershipRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(request, deserialized);
    }
}
