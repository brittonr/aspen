// Federation and peer cluster response types.

use serde::Deserialize;
use serde::Serialize;

// Peer cluster operation response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPeerClusterResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub cluster_id: Option<String>,
    pub priority: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovePeerClusterResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub cluster_id: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPeerClustersResultResponse {
    pub peers: Vec<PeerClusterInfo>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerClusterInfo {
    pub cluster_id: String,
    pub name: String,
    pub state: String,
    pub priority: u32,
    #[serde(rename = "enabled")]
    pub is_enabled: bool,
    pub sync_count: u64,
    pub failure_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerClusterStatusResponse {
    #[serde(rename = "found")]
    pub was_found: bool,
    pub cluster_id: String,
    pub state: String,
    #[serde(rename = "syncing")]
    pub is_syncing: bool,
    pub entries_received: u64,
    pub entries_imported: u64,
    pub entries_skipped: u64,
    pub entries_filtered: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePeerClusterFilterResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub cluster_id: String,
    pub filter_type: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePeerClusterPriorityResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub cluster_id: String,
    pub previous_priority: Option<u32>,
    pub new_priority: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPeerClusterEnabledResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub cluster_id: String,
    #[serde(rename = "enabled")]
    pub is_enabled: Option<bool>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyOriginResultResponse {
    #[serde(rename = "found")]
    pub was_found: bool,
    pub key: String,
    pub cluster_id: Option<String>,
    pub priority: Option<u32>,
    pub timestamp_secs: Option<u64>,
    pub is_local: Option<bool>,
}

// Federation Response Structs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationStatusResponse {
    #[serde(rename = "enabled")]
    pub is_enabled: bool,
    pub cluster_name: String,
    pub cluster_key: String,
    #[serde(rename = "dht_enabled")]
    pub is_dht_enabled: bool,
    #[serde(rename = "gossip_enabled")]
    pub is_gossip_enabled: bool,
    pub discovered_clusters: u32,
    pub federated_repos: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredClusterInfo {
    pub cluster_key: String,
    pub name: String,
    pub node_count: u32,
    pub capabilities: Vec<String>,
    pub discovered_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredClustersResponse {
    pub clusters: Vec<DiscoveredClusterInfo>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredClusterResponse {
    #[serde(rename = "found")]
    pub was_found: bool,
    pub cluster_key: Option<String>,
    pub name: Option<String>,
    pub node_count: Option<u32>,
    pub capabilities: Option<Vec<String>>,
    pub relay_urls: Option<Vec<String>>,
    pub discovered_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustClusterResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UntrustClusterResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederateRepositoryResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub fed_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedRepoInfo {
    pub repo_id: String,
    pub mode: String,
    pub fed_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedRepositoriesResponse {
    pub repositories: Vec<FederatedRepoInfo>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeFetchFederatedResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub remote_cluster: Option<String>,
    pub fetched: u32,
    pub already_present: u32,
    pub errors: Vec<String>,
    pub error: Option<String>,
}
