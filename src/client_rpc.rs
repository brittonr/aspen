//! Client RPC protocol for Aspen communication over Iroh.
//!
//! This module defines the RPC protocol used by clients (including aspen-tui) to
//! communicate with aspen-node over Iroh P2P connections. It is separate from the
//! Raft RPC protocol which is used for cluster-internal consensus communication.
//!
//! # Architecture
//!
//! The Client RPC uses a distinct ALPN (`aspen-tui`) to distinguish it from Raft RPC.
//! This allows clients to connect directly to nodes without needing HTTP.
//!
//! # Tiger Style
//!
//! - Explicit request/response pairs
//! - Bounded message sizes
//! - Fail-fast on invalid requests

use serde::{Deserialize, Serialize};

/// ALPN protocol identifier for Client RPC.
///
/// Uses "aspen-tui" for backward compatibility with existing TUI clients.
/// Distinct from `raft-rpc` to allow separate connection handling.
pub const CLIENT_ALPN: &[u8] = b"aspen-tui";

/// Maximum Client RPC message size (1 MB).
///
/// Tiger Style: Bounded to prevent memory exhaustion attacks.
pub const MAX_CLIENT_MESSAGE_SIZE: usize = 1024 * 1024;

/// Client RPC request protocol.
///
/// Defines all operations clients can request from a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientRpcRequest {
    /// Get node health status.
    GetHealth,

    /// Get Raft metrics (leader, term, log indices, etc.).
    GetRaftMetrics,

    /// Get current leader node ID.
    GetLeader,

    /// Get node information including Iroh endpoint address.
    GetNodeInfo,

    /// Get cluster ticket for peer discovery.
    GetClusterTicket,

    /// Initialize the cluster.
    InitCluster,

    /// Read a key from the key-value store.
    ReadKey { key: String },

    /// Write a key-value pair to the store.
    WriteKey { key: String, value: Vec<u8> },

    /// Trigger a snapshot.
    TriggerSnapshot,

    /// Add a learner node to the cluster.
    AddLearner { node_id: u64, addr: String },

    /// Change cluster membership.
    ChangeMembership { members: Vec<u64> },

    /// Ping for connection health check.
    Ping,

    /// Get cluster state with all known nodes.
    ///
    /// Returns information about all nodes in the cluster including
    /// their endpoint addresses, membership status, and role.
    GetClusterState,

    // =========================================================================
    // New operations (migrated from HTTP API)
    // =========================================================================
    /// Delete a key from the key-value store.
    DeleteKey { key: String },

    /// Scan keys with prefix and pagination.
    ScanKeys {
        /// Key prefix to match (empty string matches all).
        prefix: String,
        /// Maximum results (default 1000, max 10000).
        limit: Option<u32>,
        /// Continuation token from previous scan.
        continuation_token: Option<String>,
    },

    /// Get Prometheus-format metrics.
    GetMetrics,

    /// Promote a learner node to voter.
    PromoteLearner {
        /// ID of learner to promote.
        learner_id: u64,
        /// Optional voter to replace.
        replace_node: Option<u64>,
        /// Skip safety checks if true.
        force: bool,
    },

    /// Manually checkpoint SQLite WAL file.
    CheckpointWal,

    /// List all vaults (key namespaces).
    ListVaults,

    /// Get keys in a specific vault.
    GetVaultKeys { vault_name: String },

    /// Add a peer to the network factory.
    AddPeer {
        node_id: u64,
        /// JSON-serialized EndpointAddr.
        endpoint_addr: String,
    },

    /// Get cluster ticket with multiple bootstrap peers.
    GetClusterTicketCombined {
        /// Comma-separated endpoint IDs to include.
        endpoint_ids: Option<String>,
    },
}

/// Client RPC response protocol.
///
/// Response types matching the request variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientRpcResponse {
    /// Health status response.
    Health(HealthResponse),

    /// Raft metrics response.
    RaftMetrics(RaftMetricsResponse),

    /// Current leader response.
    Leader(Option<u64>),

    /// Node info response.
    NodeInfo(NodeInfoResponse),

    /// Cluster ticket response.
    ClusterTicket(ClusterTicketResponse),

    /// Cluster init response.
    InitResult(InitResultResponse),

    /// Read key response.
    ReadResult(ReadResultResponse),

    /// Write key response.
    WriteResult(WriteResultResponse),

    /// Snapshot trigger response.
    SnapshotResult(SnapshotResultResponse),

    /// Add learner response.
    AddLearnerResult(AddLearnerResultResponse),

    /// Change membership response.
    ChangeMembershipResult(ChangeMembershipResultResponse),

    /// Pong response for ping.
    Pong,

    /// Cluster state response.
    ClusterState(ClusterStateResponse),

    /// Error response for any request.
    Error(ErrorResponse),

    // =========================================================================
    // New responses (migrated from HTTP API)
    // =========================================================================
    /// Delete key response.
    DeleteResult(DeleteResultResponse),

    /// Scan keys response.
    ScanResult(ScanResultResponse),

    /// Prometheus metrics response.
    Metrics(MetricsResponse),

    /// Promote learner response.
    PromoteLearnerResult(PromoteLearnerResultResponse),

    /// Checkpoint WAL response.
    CheckpointWalResult(CheckpointWalResultResponse),

    /// List vaults response.
    VaultList(VaultListResponse),

    /// Vault keys response.
    VaultKeys(VaultKeysResponse),

    /// Add peer response.
    AddPeerResult(AddPeerResultResponse),
}

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

/// Maximum number of nodes in cluster state response.
///
/// Tiger Style: Bounded limit to prevent memory exhaustion.
pub const MAX_CLUSTER_NODES: usize = 16;

// =============================================================================
// New response types (migrated from HTTP API)
// =============================================================================

/// Delete key result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResultResponse {
    /// The key that was targeted.
    pub key: String,
    /// True if key existed and was deleted, false if not found.
    pub deleted: bool,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Scan keys result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResultResponse {
    /// Matching key-value pairs.
    pub entries: Vec<ScanEntry>,
    /// Number of entries returned.
    pub count: u32,
    /// True if more results available.
    pub is_truncated: bool,
    /// Token for next page (if truncated).
    pub continuation_token: Option<String>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Single entry from scan operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanEntry {
    /// Key name.
    pub key: String,
    /// Value (as UTF-8 string).
    pub value: String,
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

/// List vaults response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultListResponse {
    /// All vaults.
    pub vaults: Vec<VaultInfo>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Information about a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultInfo {
    /// Vault name.
    pub name: String,
    /// Number of keys in vault.
    pub key_count: u64,
}

/// Vault keys response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultKeysResponse {
    /// Vault name.
    pub vault: String,
    /// Keys in the vault.
    pub keys: Vec<VaultKeyValue>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Key-value pair within a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultKeyValue {
    /// Key name (without vault prefix).
    pub key: String,
    /// Value.
    pub value: String,
}

/// Add peer result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPeerResultResponse {
    /// Whether add peer succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

impl ClientRpcResponse {
    /// Create an error response.
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error(ErrorResponse {
            code: code.into(),
            message: message.into(),
        })
    }
}

// =============================================================================
// Backward compatibility re-exports (deprecated)
// =============================================================================

/// Deprecated: Use `CLIENT_ALPN` instead.
#[deprecated(since = "0.1.0", note = "Use CLIENT_ALPN instead")]
pub const TUI_ALPN: &[u8] = CLIENT_ALPN;

/// Deprecated: Use `MAX_CLIENT_MESSAGE_SIZE` instead.
#[deprecated(since = "0.1.0", note = "Use MAX_CLIENT_MESSAGE_SIZE instead")]
pub const MAX_TUI_MESSAGE_SIZE: usize = MAX_CLIENT_MESSAGE_SIZE;

/// Deprecated: Use `ClientRpcRequest` instead.
#[deprecated(since = "0.1.0", note = "Use ClientRpcRequest instead")]
pub type TuiRpcRequest = ClientRpcRequest;

/// Deprecated: Use `ClientRpcResponse` instead.
#[deprecated(since = "0.1.0", note = "Use ClientRpcResponse instead")]
pub type TuiRpcResponse = ClientRpcResponse;
