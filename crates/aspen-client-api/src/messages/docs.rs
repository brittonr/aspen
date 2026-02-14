//! Docs operation response types.
//!
//! Response types for iroh-docs CRDT replication operations including
//! set, get, delete, list, and status.

use serde::{Deserialize, Serialize};

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

/// Docs set result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsSetResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// The key that was set.
    pub key: Option<String>,
    /// Size of the value in bytes.
    pub size: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Docs get result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsGetResultResponse {
    /// Whether the key was found.
    pub found: bool,
    /// The value if found.
    pub value: Option<Vec<u8>>,
    /// Size of the value in bytes.
    pub size: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Docs delete result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsDeleteResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Docs list entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsListEntry {
    /// The key.
    pub key: String,
    /// Size of the value in bytes.
    pub size: u64,
    /// Content hash (hex-encoded).
    pub hash: String,
}

/// Docs list result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsListResultResponse {
    /// List of entries.
    pub entries: Vec<DocsListEntry>,
    /// Total count returned.
    pub count: u32,
    /// Whether more entries are available.
    pub has_more: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Docs status result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsStatusResultResponse {
    /// Whether docs is enabled.
    pub enabled: bool,
    /// Namespace ID (hex-encoded).
    pub namespace_id: Option<String>,
    /// Author ID (hex-encoded).
    pub author_id: Option<String>,
    /// Number of entries in the namespace.
    pub entry_count: Option<u64>,
    /// Whether the replica is open.
    pub replica_open: Option<bool>,
    /// Error message if failed.
    pub error: Option<String>,
}

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

/// Key origin lookup result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyOriginResultResponse {
    /// Whether the key has origin metadata.
    pub found: bool,
    /// The key that was looked up.
    pub key: String,
    /// Cluster ID that wrote the key (if found).
    pub cluster_id: Option<String>,
    /// Priority of the origin cluster (if found). Lower = higher priority.
    pub priority: Option<u32>,
    /// Unix timestamp when the key was last updated (if found).
    pub timestamp_secs: Option<u64>,
    /// Whether this is a local cluster origin (priority 0).
    pub is_local: Option<bool>,
}
