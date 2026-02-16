// Cluster management response types.

use serde::Deserialize;
use serde::Serialize;

/// Cluster ticket response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterTicketResponse {
    pub ticket: String,
    pub topic_id: String,
    pub cluster_id: String,
    pub endpoint_id: String,
    pub bootstrap_peers: Option<u32>,
}

/// Init cluster result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Add learner result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddLearnerResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Change membership result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeMembershipResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Cluster state response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStateResponse {
    pub nodes: Vec<NodeDescriptor>,
    pub leader_id: Option<u64>,
    pub this_node_id: u64,
}

/// Descriptor for a node in the cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDescriptor {
    pub node_id: u64,
    pub endpoint_addr: String,
    pub is_voter: bool,
    pub is_learner: bool,
    pub is_leader: bool,
}

/// Promote learner result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromoteLearnerResultResponse {
    pub success: bool,
    pub learner_id: u64,
    pub previous_voters: Vec<u64>,
    pub new_voters: Vec<u64>,
    pub message: String,
    pub error: Option<String>,
}

/// Add peer result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPeerResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Client ticket response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientTicketResponse {
    pub ticket: String,
    pub cluster_id: String,
    pub access: String,
    pub priority: u32,
    pub endpoint_id: String,
    pub error: Option<String>,
}

/// Docs ticket response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsTicketResponse {
    pub ticket: String,
    pub cluster_id: String,
    pub namespace_id: String,
    pub read_write: bool,
    pub priority: u8,
    pub endpoint_id: String,
    pub error: Option<String>,
}

/// Sharding topology result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyResultResponse {
    pub success: bool,
    pub version: u64,
    pub updated: bool,
    pub topology_data: Option<String>,
    pub shard_count: u32,
    pub error: Option<String>,
}
