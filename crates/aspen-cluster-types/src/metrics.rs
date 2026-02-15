use std::collections::BTreeMap;

use crate::NodeState;

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
