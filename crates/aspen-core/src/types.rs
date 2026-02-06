//! Core wrapper types for Aspen.
//!
//! This module provides type-safe wrappers around external dependencies,
//! hiding implementation details while providing stable public APIs.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
