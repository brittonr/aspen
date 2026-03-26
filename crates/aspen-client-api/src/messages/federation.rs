//! Federation operation types.
//!
//! Request/response types for cross-cluster discovery and synchronization operations.

use serde::Deserialize;
use serde::Serialize;

/// Federation domain request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationRequest {
    /// Get federation status.
    GetFederationStatus,
    /// List discovered clusters.
    ListDiscoveredClusters,
    /// Get details about a discovered cluster.
    GetDiscoveredCluster { cluster_key: String },
    /// Trust a cluster.
    TrustCluster { cluster_key: String },
    /// Untrust a cluster.
    UntrustCluster { cluster_key: String },
    /// Federate a repository.
    FederateRepository { repo_id: String, mode: String },
    /// List federated repositories.
    ListFederatedRepositories,
    /// Fetch a federated repository from a remote cluster.
    ForgeFetchFederated {
        federated_id: String,
        remote_cluster: String,
    },

    // ========================================================================
    // Federation auth primitives
    // ========================================================================
    /// Issue a federation capability token to a remote cluster.
    FederationGrant {
        /// Remote cluster's public key (audience).
        audience: String,
        /// Capabilities to grant (JSON-encoded list).
        capabilities: String,
        /// Token lifetime in seconds.
        lifetime_secs: u64,
        /// Whether the token allows further delegation.
        allow_delegate: bool,
    },
    /// Revoke a federation token by its BLAKE3 hash.
    FederationRevoke {
        /// Hex-encoded BLAKE3 hash of the token to revoke.
        token_hash: String,
    },
    /// List active federation tokens issued by this cluster.
    FederationListTokens,
    /// Publish a KV prefix for federation.
    FederationPublish {
        /// KV prefix to publish.
        prefix: String,
        /// Access policy: "public" or "token_required".
        access_policy: String,
    },
    /// Subscribe to a remote cluster's KV prefix.
    FederationSubscribe {
        /// Source cluster's public key.
        source: String,
        /// KV prefix to subscribe to.
        prefix: String,
        /// Sync mode: "periodic:<secs>" or "on_gossip".
        sync_mode: String,
    },
    /// List active federation subscriptions.
    FederationListSubscriptions,
    /// Unsubscribe from a remote cluster's KV prefix.
    FederationUnsubscribe {
        /// Source cluster's public key.
        source: String,
        /// KV prefix to unsubscribe from.
        prefix: String,
    },

    /// Perform a one-shot federation sync pull from a remote cluster.
    ///
    /// Connects to the remote peer via iroh QUIC, performs a federation
    /// handshake, and queries the peer's resource state.
    FederationSyncPeer {
        /// Remote peer's iroh node ID (base32-encoded PublicKey).
        peer_node_id: String,
        /// Optional direct socket address hint (e.g., "192.168.1.1:54866").
        peer_addr: Option<String>,
    },
}

#[cfg(feature = "auth")]
impl FederationRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            // Read-only / no auth required
            Self::GetFederationStatus
            | Self::ListDiscoveredClusters
            | Self::GetDiscoveredCluster { .. }
            | Self::ListFederatedRepositories => None,
            // Write operations
            Self::TrustCluster { .. }
            | Self::UntrustCluster { .. }
            | Self::FederateRepository { .. }
            | Self::ForgeFetchFederated { .. }
            | Self::FederationGrant { .. }
            | Self::FederationRevoke { .. }
            | Self::FederationPublish { .. }
            | Self::FederationSubscribe { .. }
            | Self::FederationUnsubscribe { .. } => Some(Operation::Write {
                key: "_sys:fed:".to_string(),
                value: vec![],
            }),
            Self::FederationListTokens | Self::FederationListSubscriptions => None,
            Self::FederationSyncPeer { .. } => Some(Operation::Write {
                key: "_sys:fed:sync".to_string(),
                value: vec![],
            }),
        }
    }
}

/// Federation status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationStatusResponse {
    /// Whether federation is enabled.
    #[serde(rename = "enabled")]
    pub is_enabled: bool,
    /// Cluster name.
    pub cluster_name: String,
    /// Cluster public key (base32).
    pub cluster_key: String,
    /// Whether DHT discovery is enabled.
    pub dht_enabled: bool,
    /// Whether gossip is enabled.
    pub gossip_enabled: bool,
    /// Number of discovered clusters.
    pub discovered_clusters: u32,
    /// Number of federated repositories.
    pub federated_repos: u32,
    /// Error message if status retrieval failed.
    pub error: Option<String>,
}

/// Discovered cluster info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredClusterInfo {
    /// Cluster public key.
    pub cluster_key: String,
    /// Cluster name.
    pub name: String,
    /// Number of nodes.
    pub node_count: u32,
    /// Capabilities.
    pub capabilities: Vec<String>,
    /// When discovered.
    pub discovered_at: String,
}

/// List of discovered clusters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredClustersResponse {
    /// List of discovered clusters.
    pub clusters: Vec<DiscoveredClusterInfo>,
    /// Total count.
    pub count: u32,
    /// Error message if retrieval failed.
    pub error: Option<String>,
}

/// Single discovered cluster details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredClusterResponse {
    /// Whether the cluster was found.
    pub was_found: bool,
    /// Cluster public key.
    pub cluster_key: Option<String>,
    /// Cluster name.
    pub name: Option<String>,
    /// Number of nodes.
    pub node_count: Option<u32>,
    /// Capabilities.
    pub capabilities: Option<Vec<String>>,
    /// Relay URLs.
    pub relay_urls: Option<Vec<String>>,
    /// When discovered.
    pub discovered_at: Option<String>,
}

/// Trust cluster result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustClusterResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Untrust cluster result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UntrustClusterResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Federate repository result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederateRepositoryResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Federated ID (if successful).
    pub fed_id: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Federated repository info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedRepoInfo {
    /// Repository ID.
    pub repo_id: String,
    /// Federation mode.
    pub mode: String,
    /// Federated ID.
    pub fed_id: String,
}

/// List of federated repositories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedRepositoriesResponse {
    /// List of federated repositories.
    pub repositories: Vec<FederatedRepoInfo>,
    /// Total count.
    pub count: u32,
    /// Error message if retrieval failed.
    pub error: Option<String>,
}

/// Federation grant result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationGrantResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Base64-encoded token (if successful).
    pub token_b64: Option<String>,
    /// Token hash (hex).
    pub token_hash: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Federation revoke result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationRevokeResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Federation token info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationTokenInfo {
    /// Token hash (hex).
    pub token_hash: String,
    /// Audience public key.
    pub audience: String,
    /// Capabilities (JSON).
    pub capabilities: String,
    /// Expiry timestamp (Unix seconds).
    pub expires_at: u64,
    /// Delegation depth.
    pub delegation_depth: u8,
}

/// Federation list tokens response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationListTokensResponse {
    /// Active tokens.
    pub tokens: Vec<FederationTokenInfo>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Federation publish result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPublishResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// KV key where publication is stored.
    pub key: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Federation subscription info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationSubscriptionInfo {
    /// Source cluster key.
    pub source: String,
    /// Prefix being subscribed to.
    pub prefix: String,
    /// Sync mode description.
    pub sync_mode: String,
    /// Status: active, needs_refresh, paused.
    pub status: String,
    /// Last sync HLC timestamp.
    pub last_sync_hlc: u64,
}

/// Federation list subscriptions response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationListSubscriptionsResponse {
    /// Active subscriptions.
    pub subscriptions: Vec<FederationSubscriptionInfo>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Federation subscribe result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationSubscribeResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// KV key where subscription is stored.
    pub key: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Federation unsubscribe result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationUnsubscribeResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Remote resource info returned from a federation sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPeerResourceInfo {
    /// Resource type (e.g., "forge:repo").
    pub resource_type: String,
    /// Number of ref heads.
    pub ref_count: u32,
    /// Ref head names.
    pub ref_names: Vec<String>,
    /// Ref heads: (ref_name, hex_hash) pairs.
    #[serde(default)]
    pub ref_heads: Vec<(String, String)>,
    /// Federated ID string for this resource.
    #[serde(default)]
    pub fed_id: Option<String>,
}

/// Federation sync peer result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationSyncPeerResponse {
    /// Whether the sync succeeded.
    pub is_success: bool,
    /// Remote cluster name.
    pub remote_cluster_name: Option<String>,
    /// Remote cluster public key (base32).
    pub remote_cluster_key: Option<String>,
    /// Whether the remote cluster trusts us.
    pub trusted: Option<bool>,
    /// Resources discovered on the remote cluster.
    pub resources: Vec<SyncPeerResourceInfo>,
    /// Error message if sync failed.
    pub error: Option<String>,
}

/// Forge fetch federated result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeFetchFederatedResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Remote cluster name.
    pub remote_cluster: Option<String>,
    /// Number of objects fetched.
    pub fetched: u32,
    /// Number of objects already present locally.
    pub already_present: u32,
    /// Errors encountered during fetch.
    pub errors: Vec<String>,
    /// Error message if operation failed.
    pub error: Option<String>,
}
