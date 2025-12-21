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

// Re-export ALPN constant from canonical location
pub use crate::protocol_handlers::CLIENT_ALPN;

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

    /// Get a client ticket for overlay subscription.
    ///
    /// Returns a ticket that clients can use to connect to this cluster
    /// as part of a priority-based overlay (like Nix binary caches).
    GetClientTicket {
        /// Access level: "read" or "write".
        access: String,
        /// Priority level (0 = highest).
        priority: u32,
    },

    /// Get a docs ticket for iroh-docs subscription.
    ///
    /// Returns a ticket for subscribing to the cluster's iroh-docs
    /// namespace for real-time state synchronization.
    GetDocsTicket {
        /// Whether client should have write access to docs.
        read_write: bool,
        /// Priority level for this subscription.
        priority: u8,
    },

    // =========================================================================
    // Blob operations (content-addressed storage)
    // =========================================================================
    /// Add a blob to the store.
    ///
    /// Stores the provided bytes and returns a blob reference with the hash.
    AddBlob {
        /// Blob data to store.
        data: Vec<u8>,
        /// Optional tag to protect the blob from GC.
        tag: Option<String>,
    },

    /// Get a blob by hash.
    ///
    /// Returns the blob data if it exists.
    GetBlob {
        /// BLAKE3 hash of the blob (hex-encoded).
        hash: String,
    },

    /// Check if a blob exists.
    HasBlob {
        /// BLAKE3 hash of the blob (hex-encoded).
        hash: String,
    },

    /// Get a ticket for sharing a blob.
    ///
    /// Returns a BlobTicket that can be used to download the blob from this node.
    GetBlobTicket {
        /// BLAKE3 hash of the blob (hex-encoded).
        hash: String,
    },

    /// List blobs in the store.
    ListBlobs {
        /// Maximum number of blobs to return.
        limit: u32,
        /// Continuation token from previous list call.
        continuation_token: Option<String>,
    },

    /// Protect a blob from garbage collection.
    ProtectBlob {
        /// BLAKE3 hash of the blob (hex-encoded).
        hash: String,
        /// Tag name for the protection.
        tag: String,
    },

    /// Remove protection from a blob.
    UnprotectBlob {
        /// Tag name to remove.
        tag: String,
    },

    // =========================================================================
    // Peer cluster operations (cluster-to-cluster sync)
    // =========================================================================
    /// Add a peer cluster to sync with.
    ///
    /// Subscribes to the peer cluster's iroh-docs namespace for real-time
    /// synchronization with priority-based conflict resolution.
    AddPeerCluster {
        /// Serialized AspenDocsTicket from the peer cluster.
        ticket: String,
    },

    /// Remove a peer cluster subscription.
    RemovePeerCluster {
        /// Cluster ID of the peer to remove.
        cluster_id: String,
    },

    /// List all peer cluster subscriptions.
    ListPeerClusters,

    /// Get sync status for a specific peer cluster.
    GetPeerClusterStatus {
        /// Cluster ID of the peer.
        cluster_id: String,
    },

    /// Update the subscription filter for a peer cluster.
    UpdatePeerClusterFilter {
        /// Cluster ID of the peer.
        cluster_id: String,
        /// Filter type: "full", "include", or "exclude".
        filter_type: String,
        /// Prefixes for include/exclude filters (JSON array).
        prefixes: Option<String>,
    },

    /// Update the priority for a peer cluster.
    UpdatePeerClusterPriority {
        /// Cluster ID of the peer.
        cluster_id: String,
        /// New priority (0 = highest, lower wins conflicts).
        priority: u32,
    },

    /// Enable or disable a peer cluster subscription.
    SetPeerClusterEnabled {
        /// Cluster ID of the peer.
        cluster_id: String,
        /// Whether to enable the subscription.
        enabled: bool,
    },

    // =========================================================================
    // SQL query operations
    // =========================================================================
    /// Execute a read-only SQL query against the state machine.
    ///
    /// Only SELECT statements are allowed. The query is validated before
    /// execution and runs with `PRAGMA query_only = ON` for safety.
    ExecuteSql {
        /// SQL query string (must be SELECT or WITH...SELECT).
        query: String,
        /// Query parameters (JSON-serialized SqlValue array).
        params: String,
        /// Consistency level: "linearizable" (default) or "stale".
        consistency: String,
        /// Maximum rows to return (default 1000, max 10000).
        limit: Option<u32>,
        /// Query timeout in milliseconds (default 5000, max 30000).
        timeout_ms: Option<u32>,
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

    /// Client ticket response for overlay subscription.
    ClientTicket(ClientTicketResponse),

    /// Docs ticket response for iroh-docs subscription.
    DocsTicket(DocsTicketResponse),

    // =========================================================================
    // Blob operation responses
    // =========================================================================
    /// Add blob result.
    AddBlobResult(AddBlobResultResponse),

    /// Get blob result.
    GetBlobResult(GetBlobResultResponse),

    /// Has blob result.
    HasBlobResult(HasBlobResultResponse),

    /// Get blob ticket result.
    GetBlobTicketResult(GetBlobTicketResultResponse),

    /// List blobs result.
    ListBlobsResult(ListBlobsResultResponse),

    /// Protect blob result.
    ProtectBlobResult(ProtectBlobResultResponse),

    /// Unprotect blob result.
    UnprotectBlobResult(UnprotectBlobResultResponse),

    // =========================================================================
    // Peer cluster operation responses
    // =========================================================================
    /// Add peer cluster result.
    AddPeerClusterResult(AddPeerClusterResultResponse),

    /// Remove peer cluster result.
    RemovePeerClusterResult(RemovePeerClusterResultResponse),

    /// List peer clusters result.
    ListPeerClustersResult(ListPeerClustersResultResponse),

    /// Get peer cluster status result.
    PeerClusterStatus(PeerClusterStatusResponse),

    /// Update peer cluster filter result.
    UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse),

    /// Update peer cluster priority result.
    UpdatePeerClusterPriorityResult(UpdatePeerClusterPriorityResultResponse),

    /// Set peer cluster enabled result.
    SetPeerClusterEnabledResult(SetPeerClusterEnabledResultResponse),

    // =========================================================================
    // SQL query response
    // =========================================================================
    /// SQL query result.
    SqlResult(SqlResultResponse),
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

/// Docs ticket response for iroh-docs subscription.
///
/// Used by clients to subscribe to a cluster's iroh-docs namespace
/// for real-time state synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsTicketResponse {
    /// Serialized AspenDocsTicket.
    pub ticket: String,
    /// Cluster identifier.
    pub cluster_id: String,
    /// Namespace ID (derived from cluster cookie).
    pub namespace_id: String,
    /// Whether client has write access.
    pub read_write: bool,
    /// Priority level for this subscription.
    pub priority: u8,
    /// This node's endpoint ID.
    pub endpoint_id: String,
    /// Error message if generation failed.
    pub error: Option<String>,
}

// =============================================================================
// Blob operation response types
// =============================================================================

/// Add blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddBlobResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// BLAKE3 hash of the stored blob (hex-encoded).
    pub hash: Option<String>,
    /// Size of the blob in bytes.
    pub size: Option<u64>,
    /// Whether the blob was new (not already in store).
    pub was_new: Option<bool>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Get blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobResultResponse {
    /// Whether the blob was found.
    pub found: bool,
    /// Blob data if found.
    pub data: Option<Vec<u8>>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Has blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasBlobResultResponse {
    /// Whether the blob exists in the store.
    pub exists: bool,
    /// Error message if check failed.
    pub error: Option<String>,
}

/// Get blob ticket result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobTicketResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Serialized BlobTicket.
    pub ticket: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Blob list entry for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobListEntry {
    /// BLAKE3 hash (hex-encoded).
    pub hash: String,
    /// Size in bytes.
    pub size: u64,
}

/// List blobs result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBlobsResultResponse {
    /// List of blobs.
    pub blobs: Vec<BlobListEntry>,
    /// Total count returned.
    pub count: u32,
    /// Whether more blobs are available.
    pub has_more: bool,
    /// Continuation token for next page.
    pub continuation_token: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Protect blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectBlobResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Unprotect blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnprotectBlobResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

// =============================================================================
// Peer cluster operation response types
// =============================================================================

/// Add peer cluster result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPeerClusterResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Cluster ID of the added peer.
    pub cluster_id: Option<String>,
    /// Priority assigned to this peer.
    pub priority: Option<u32>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Remove peer cluster result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovePeerClusterResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Cluster ID of the removed peer.
    pub cluster_id: String,
    /// Error message if failed.
    pub error: Option<String>,
}

/// List peer clusters result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPeerClustersResultResponse {
    /// List of peer cluster information.
    pub peers: Vec<PeerClusterInfo>,
    /// Total number of peer clusters.
    pub count: u32,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Information about a peer cluster subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerClusterInfo {
    /// Cluster ID of the peer.
    pub cluster_id: String,
    /// Human-readable name of the peer.
    pub name: String,
    /// Connection state: "disconnected", "connecting", "connected", "failed".
    pub state: String,
    /// Priority for conflict resolution (0 = highest).
    pub priority: u32,
    /// Whether sync is enabled.
    pub enabled: bool,
    /// Number of completed sync sessions.
    pub sync_count: u64,
    /// Number of connection failures.
    pub failure_count: u64,
}

/// Peer cluster status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerClusterStatusResponse {
    /// Whether the peer was found.
    pub found: bool,
    /// Cluster ID of the peer.
    pub cluster_id: String,
    /// Connection state: "disconnected", "connecting", "connected", "failed".
    pub state: String,
    /// Whether sync is currently in progress.
    pub syncing: bool,
    /// Entries received in current/last sync.
    pub entries_received: u64,
    /// Entries imported in current/last sync.
    pub entries_imported: u64,
    /// Entries skipped due to priority.
    pub entries_skipped: u64,
    /// Entries skipped due to filter.
    pub entries_filtered: u64,
    /// Error message if lookup failed.
    pub error: Option<String>,
}

/// Update peer cluster filter result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePeerClusterFilterResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Cluster ID of the peer.
    pub cluster_id: String,
    /// New filter type: "full", "include", or "exclude".
    pub filter_type: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Update peer cluster priority result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePeerClusterPriorityResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Cluster ID of the peer.
    pub cluster_id: String,
    /// Previous priority value.
    pub previous_priority: Option<u32>,
    /// New priority value.
    pub new_priority: Option<u32>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Set peer cluster enabled result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPeerClusterEnabledResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Cluster ID of the peer.
    pub cluster_id: String,
    /// Whether the peer is now enabled.
    pub enabled: Option<bool>,
    /// Error message if failed.
    pub error: Option<String>,
}

// =============================================================================
// SQL query response types
// =============================================================================

/// SQL query result response.
///
/// Contains the result of a read-only SQL query execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlResultResponse {
    /// Whether the query succeeded.
    pub success: bool,
    /// Column names (JSON array).
    pub columns: Option<Vec<String>>,
    /// Result rows (JSON array of arrays).
    /// Each inner array contains the values for one row.
    pub rows: Option<Vec<Vec<serde_json::Value>>>,
    /// Number of rows returned.
    pub row_count: Option<u32>,
    /// True if more rows exist but were not returned due to limit.
    pub is_truncated: Option<bool>,
    /// Query execution time in milliseconds.
    pub execution_time_ms: Option<u64>,
    /// Error message if query failed.
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
