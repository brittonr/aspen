//! Cluster operation response types.
//!
//! Response types for cluster management operations including health checks,
//! Raft metrics, node info, and cluster state.

use serde::{Deserialize, Serialize};

/// Health status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall status: "healthy", "degraded", or "unhealthy".
    pub status: String,
    /// Node identifier.
    pub node_id: u64,
    /// Raft node ID (may differ from node_id).
    pub raft_node_id: Option<u64>,
    /// Uptime in seconds.
    pub uptime_seconds: u64,
    /// Whether the node is initialized and ready to process non-bootstrap operations.
    /// A node becomes initialized when it receives Raft membership through replication.
    #[serde(default)]
    pub is_initialized: bool,
    /// Number of nodes in the current membership configuration.
    /// None if not yet initialized.
    #[serde(default)]
    pub membership_node_count: Option<u32>,
}

/// Raft metrics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftMetricsResponse {
    /// Node identifier.
    pub node_id: u64,
    /// Current Raft state (Leader, Follower, Candidate).
    pub state: String,
    /// Current leader node ID, if known.
    pub current_leader: Option<u64>,
    /// Current Raft term.
    pub current_term: u64,
    /// Last log index.
    pub last_log_index: Option<u64>,
    /// Last applied log index.
    pub last_applied_index: Option<u64>,
    /// Snapshot log index.
    pub snapshot_index: Option<u64>,
    /// Replication state for each node (only populated when this node is leader).
    ///
    /// Maps node_id -> matched_log_index. The matched index indicates how far
    /// each follower has replicated. A `None` value means the node's progress
    /// is unknown (e.g., newly added learner).
    pub replication: Option<Vec<ReplicationProgress>>,
}

/// Replication progress for a single node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationProgress {
    /// Node identifier.
    pub node_id: u64,
    /// The highest log index known to be replicated on this node.
    /// `None` means replication progress is unknown.
    pub matched_index: Option<u64>,
}

/// Node information response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfoResponse {
    /// Node identifier.
    pub node_id: u64,
    /// Iroh endpoint address (serialized).
    pub endpoint_addr: String,
}

/// Cluster ticket response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterTicketResponse {
    /// Serialized cluster ticket.
    pub ticket: String,
    /// Gossip topic ID (debug format).
    pub topic_id: String,
    /// Cluster identifier (from cookie).
    pub cluster_id: String,
    /// This node's endpoint ID.
    pub endpoint_id: String,
    /// Number of bootstrap peers in ticket (for combined tickets).
    pub bootstrap_peers: Option<usize>,
}

/// Init cluster result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitResultResponse {
    /// Whether initialization succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Read key result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResultResponse {
    /// The value if found.
    pub value: Option<Vec<u8>>,
    /// Whether the key was found.
    pub found: bool,
    /// Optional error message when read fails (e.g., not leader).
    pub error: Option<String>,
}

/// Write key result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResultResponse {
    /// Whether write succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Compare-and-swap result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareAndSwapResultResponse {
    /// Whether the CAS operation succeeded.
    ///
    /// True if the condition matched and the value was updated/deleted.
    /// False if the condition did not match.
    pub success: bool,
    /// The actual value of the key when CAS failed.
    ///
    /// This allows clients to retry with the correct expected value.
    /// None means the key did not exist.
    pub actual_value: Option<Vec<u8>>,
    /// Error message if operation failed due to internal error (not CAS condition).
    pub error: Option<String>,
}

/// Snapshot trigger result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotResultResponse {
    /// Whether snapshot was triggered.
    pub success: bool,
    /// Snapshot log index if successful.
    pub snapshot_index: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Add learner result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddLearnerResultResponse {
    /// Whether adding learner succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Change membership result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeMembershipResultResponse {
    /// Whether membership change succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Error response for failed requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code.
    pub code: String,
    /// Error message.
    pub message: String,
}

/// Cluster state response containing all known nodes.
///
/// Tiger Style: Bounded to MAX_CLUSTER_NODES (16) to prevent unbounded growth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStateResponse {
    /// All known nodes in the cluster.
    pub nodes: Vec<NodeDescriptor>,
    /// Current leader node ID, if known.
    pub leader_id: Option<u64>,
    /// This node's ID.
    pub this_node_id: u64,
}

/// Descriptor for a node in the cluster.
///
/// Contains all information needed to connect to and identify a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDescriptor {
    /// Node identifier.
    pub node_id: u64,
    /// Iroh endpoint address (serialized).
    pub endpoint_addr: String,
    /// Whether this node is a voter in Raft consensus.
    pub is_voter: bool,
    /// Whether this node is a learner (non-voting replica).
    pub is_learner: bool,
    /// Whether this node is the current leader.
    pub is_leader: bool,
}

/// Prometheus metrics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    /// Prometheus text format metrics.
    pub prometheus_text: String,
}

/// Promote learner result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromoteLearnerResultResponse {
    /// Whether promotion succeeded.
    pub success: bool,
    /// ID of promoted learner.
    pub learner_id: u64,
    /// Voters before the change.
    pub previous_voters: Vec<u64>,
    /// Voters after the change.
    pub new_voters: Vec<u64>,
    /// Status message.
    pub message: String,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Checkpoint WAL result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointWalResultResponse {
    /// Whether checkpoint succeeded.
    pub success: bool,
    /// Number of pages checkpointed.
    pub pages_checkpointed: Option<u32>,
    /// WAL file size before checkpoint (bytes).
    pub wal_size_before_bytes: Option<u64>,
    /// WAL file size after checkpoint (bytes).
    pub wal_size_after_bytes: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Add peer result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPeerResultResponse {
    /// Whether add peer succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Client ticket response for overlay subscription.
///
/// Used by clients to connect to a cluster as part of a priority-based
/// overlay system (similar to Nix binary cache substituters).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientTicketResponse {
    /// Serialized AspenClientTicket.
    pub ticket: String,
    /// Cluster identifier.
    pub cluster_id: String,
    /// Access level: "read" or "write".
    pub access: String,
    /// Priority level (0 = highest).
    pub priority: u32,
    /// This node's endpoint ID.
    pub endpoint_id: String,
    /// Error message if generation failed.
    pub error: Option<String>,
}

/// Shard topology result for GetTopology RPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Current topology version.
    pub version: u64,
    /// Whether the topology was updated (false if client version matches).
    pub updated: bool,
    /// Serialized ShardTopology (JSON) if updated is true.
    pub topology_data: Option<String>,
    /// Number of shards in the topology.
    pub shard_count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}
