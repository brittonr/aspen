//! Docs operation types.
//!
//! Request/response types for iroh-docs CRDT replication operations including
//! set, get, delete, list, status, and peer cluster management.

use serde::Deserialize;
use serde::Serialize;

/// Docs domain request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocsRequest {
    /// Set a key-value pair in the docs namespace.
    DocsSet { key: String, value: Vec<u8> },
    /// Get a value from the docs namespace.
    DocsGet { key: String },
    /// Delete a key from the docs namespace.
    DocsDelete { key: String },
    /// List entries in the docs namespace.
    DocsList { prefix: Option<String>, limit: Option<u32> },
    /// Get docs namespace status and sync information.
    DocsStatus,
    /// Get a docs ticket for iroh-docs subscription.
    GetDocsTicket { read_write: bool, priority: u8 },
    /// Add a peer cluster to sync with.
    AddPeerCluster { ticket: String },
    /// Remove a peer cluster subscription.
    RemovePeerCluster { cluster_id: String },
    /// List all peer cluster subscriptions.
    ListPeerClusters,
    /// Get sync status for a specific peer cluster.
    GetPeerClusterStatus { cluster_id: String },
    /// Update the subscription filter for a peer cluster.
    UpdatePeerClusterFilter {
        cluster_id: String,
        filter_type: String,
        prefixes: Option<String>,
    },
    /// Update the priority for a peer cluster.
    UpdatePeerClusterPriority { cluster_id: String, priority: u32 },
    /// Enable or disable a peer cluster subscription.
    SetPeerClusterEnabled {
        cluster_id: String,
        #[serde(rename = "enabled")]
        is_enabled: bool,
    },
    /// Get the origin metadata for a key.
    GetKeyOrigin { key: String },
}

#[cfg(feature = "auth")]
impl DocsRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            Self::DocsSet { key, value } => Some(Operation::Write {
                key: format!("_docs:{key}"),
                value: value.clone(),
            }),
            Self::DocsGet { key } | Self::DocsDelete { key } => Some(Operation::Read {
                key: format!("_docs:{key}"),
            }),
            Self::DocsList { .. } | Self::DocsStatus => Some(Operation::Read {
                key: "_docs:".to_string(),
            }),
            Self::AddPeerCluster { .. }
            | Self::RemovePeerCluster { .. }
            | Self::UpdatePeerClusterFilter { .. }
            | Self::UpdatePeerClusterPriority { .. }
            | Self::SetPeerClusterEnabled { .. } => Some(Operation::ClusterAdmin {
                action: "peer_cluster_operation".to_string(),
            }),
            Self::ListPeerClusters
            | Self::GetPeerClusterStatus { .. }
            | Self::GetKeyOrigin { .. }
            | Self::GetDocsTicket { .. } => None,
        }
    }
}

/// Docs ticket response for iroh-docs subscription.
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
    pub is_success: bool,
    /// The key that was set.
    pub key: Option<String>,
    /// Size of the value in bytes.
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Docs get result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsGetResultResponse {
    /// Whether the key was found.
    pub was_found: bool,
    /// The value if found.
    pub value: Option<Vec<u8>>,
    /// Size of the value in bytes.
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Docs delete result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsDeleteResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Docs list entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsListEntry {
    /// The key.
    pub key: String,
    /// Size of the value in bytes (Tiger Style: units in name).
    #[serde(alias = "size")]
    pub size_bytes: u64,
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
    #[serde(rename = "enabled")]
    pub is_enabled: bool,
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
    pub is_success: bool,
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
    pub is_success: bool,
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
    #[serde(rename = "enabled")]
    pub is_enabled: bool,
    /// Number of completed sync sessions.
    pub sync_count: u64,
    /// Number of connection failures.
    pub failure_count: u64,
}

/// Peer cluster status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerClusterStatusResponse {
    /// Whether the peer was found.
    pub was_found: bool,
    /// Cluster ID of the peer.
    pub cluster_id: String,
    /// Connection state: "disconnected", "connecting", "connected", "failed".
    pub state: String,
    /// Whether sync is currently in progress.
    pub is_syncing: bool,
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
    pub is_success: bool,
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
    pub is_success: bool,
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
    pub is_success: bool,
    /// Cluster ID of the peer.
    pub cluster_id: String,
    /// Whether the peer is now enabled.
    #[serde(rename = "enabled")]
    pub is_enabled: Option<bool>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Key origin lookup result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyOriginResultResponse {
    /// Whether the key has origin metadata.
    pub was_found: bool,
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
