//! Federation response types.
//!
//! Response types for cross-cluster discovery and synchronization operations.

use serde::{Deserialize, Serialize};

/// Federation status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationStatusResponse {
    /// Whether federation is enabled.
    pub enabled: bool,
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
    pub found: bool,
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
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Untrust cluster result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UntrustClusterResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Federate repository result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederateRepositoryResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
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

/// Forge fetch federated result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeFetchFederatedResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Remote cluster name.
    pub remote_cluster: Option<String>,
    /// Number of objects fetched.
    pub fetched: usize,
    /// Number of objects already present locally.
    pub already_present: usize,
    /// Errors encountered during fetch.
    pub errors: Vec<String>,
    /// Error message if operation failed.
    pub error: Option<String>,
}
