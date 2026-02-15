// Health and node info response types.

use serde::Deserialize;
use serde::Serialize;

/// Health status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub node_id: u64,
    pub raft_node_id: Option<u64>,
    pub uptime_seconds: u64,
}

/// Node information response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfoResponse {
    pub node_id: u64,
    pub endpoint_addr: String,
}

/// Prometheus metrics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub prometheus_text: String,
}
