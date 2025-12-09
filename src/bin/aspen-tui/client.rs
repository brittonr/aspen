//! HTTP client for communicating with Aspen nodes.
//!
//! Provides async methods for all Aspen HTTP API endpoints.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Default request timeout.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Node health status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeStatus {
    /// Node is healthy and responsive.
    Healthy,
    /// Node is responding but has warnings.
    Degraded,
    /// Node is unhealthy or unreachable.
    Unhealthy,
    /// Node status is unknown.
    Unknown,
}

/// Information about a single node.
#[derive(Debug, Clone)]
pub struct NodeInfo {
    /// Node identifier.
    pub node_id: u64,
    /// Current health status.
    pub status: NodeStatus,
    /// Whether this node is the Raft leader.
    pub is_leader: bool,
    /// Last applied log index.
    pub last_applied_index: Option<u64>,
    /// Current Raft term.
    pub current_term: Option<u64>,
    /// Uptime in seconds.
    pub uptime_secs: Option<u64>,
    /// HTTP address.
    pub http_addr: String,
}

/// Aggregated cluster metrics.
#[derive(Debug, Clone)]
pub struct ClusterMetrics {
    /// Current leader node ID.
    pub leader: Option<u64>,
    /// Current Raft term.
    pub term: u64,
    /// Total number of nodes.
    pub node_count: usize,
    /// Last log index.
    pub last_log_index: Option<u64>,
    /// Last applied log index.
    pub last_applied_index: Option<u64>,
}

/// Health check response from node.
#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub node_id: u64,
    pub raft_node_id: Option<u64>,
    pub uptime_seconds: u64,
}

/// Raft metrics response from node.
#[derive(Debug, Deserialize)]
pub struct RaftMetricsResponse {
    pub node_id: u64,
    pub state: String,
    pub current_leader: Option<u64>,
    pub current_term: u64,
    pub last_log_index: Option<u64>,
    pub last_applied: Option<LogId>,
    pub snapshot: Option<LogId>,
    /// Replication status for all nodes in the cluster.
    /// Maps node_id to replication status.
    #[serde(default)]
    pub replication: std::collections::HashMap<String, LogId>,
}

/// Log ID from Raft.
#[derive(Debug, Deserialize)]
pub struct LogId {
    pub term: u64,
    pub index: u64,
}

/// Write request payload.
#[derive(Debug, Serialize)]
pub struct WriteRequest {
    pub key: String,
    pub value: Vec<u8>,
}

/// Read request payload.
#[derive(Debug, Serialize)]
pub struct ReadRequest {
    pub key: String,
}

/// Read response payload.
#[derive(Debug, Deserialize)]
pub struct ReadResponse {
    pub value: Option<Vec<u8>>,
}

/// Cluster init response.
#[derive(Debug, Deserialize)]
pub struct InitResponse {
    pub log_id: Option<LogId>,
}

/// HTTP client for a single Aspen node.
pub struct AspenClient {
    client: reqwest::Client,
    base_url: String,
}

impl AspenClient {
    /// Create a new client for the given node URL.
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("failed to create HTTP client");

        // Ensure base URL doesn't have trailing slash
        let base_url = base_url.trim_end_matches('/').to_string();

        Self { client, base_url }
    }

    /// Get the base URL for this client.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get node health.
    pub async fn get_health(&self) -> Result<HealthResponse, reqwest::Error> {
        let url = format!("{}/health", self.base_url);
        self.client.get(&url).send().await?.json().await
    }

    /// Get Raft metrics.
    pub async fn get_raft_metrics(&self) -> Result<RaftMetricsResponse, reqwest::Error> {
        let url = format!("{}/raft-metrics", self.base_url);
        self.client.get(&url).send().await?.json().await
    }

    /// Get current leader.
    pub async fn get_leader(&self) -> Result<Option<u64>, reqwest::Error> {
        #[derive(Deserialize)]
        struct LeaderResponse {
            leader: Option<u64>,
        }

        let url = format!("{}/leader", self.base_url);
        let resp: LeaderResponse = self.client.get(&url).send().await?.json().await?;
        Ok(resp.leader)
    }

    /// Initialize the cluster.
    pub async fn init_cluster(&self) -> Result<InitResponse, reqwest::Error> {
        let url = format!("{}/init", self.base_url);
        self.client
            .post(&url)
            .json(&serde_json::json!({"initial_members": []}))
            .send()
            .await?
            .json()
            .await
    }

    /// Add a learner node.
    pub async fn add_learner(&self, node_id: u64, addr: &str) -> Result<(), reqwest::Error> {
        let url = format!("{}/add-learner", self.base_url);
        let body = serde_json::json!({
            "learner": {
                "id": node_id,
                "addr": addr
            }
        });
        self.client.post(&url).json(&body).send().await?;
        Ok(())
    }

    /// Change cluster membership.
    pub async fn change_membership(&self, members: Vec<u64>) -> Result<(), reqwest::Error> {
        let url = format!("{}/change-membership", self.base_url);
        let body = serde_json::json!({
            "members": members
        });
        self.client.post(&url).json(&body).send().await?;
        Ok(())
    }

    /// Write a key-value pair.
    pub async fn write_key(&self, key: &str, value: Vec<u8>) -> Result<(), reqwest::Error> {
        let url = format!("{}/write", self.base_url);
        let body = WriteRequest {
            key: key.to_string(),
            value,
        };
        self.client.post(&url).json(&body).send().await?;
        Ok(())
    }

    /// Read a key.
    pub async fn read_key(&self, key: &str) -> Result<Option<Vec<u8>>, reqwest::Error> {
        let url = format!("{}/read", self.base_url);
        let body = ReadRequest {
            key: key.to_string(),
        };
        let resp: ReadResponse = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;
        Ok(resp.value)
    }

    /// Trigger a snapshot.
    pub async fn trigger_snapshot(&self) -> Result<(), reqwest::Error> {
        let url = format!("{}/trigger-snapshot", self.base_url);
        self.client.post(&url).send().await?;
        Ok(())
    }

    /// Get node info.
    pub async fn get_node_info(&self) -> color_eyre::Result<NodeInfo> {
        let health = self.get_health().await?;
        let metrics = self.get_raft_metrics().await.ok();

        let is_leader = metrics
            .as_ref()
            .and_then(|m| m.current_leader)
            .map(|l| l == health.node_id)
            .unwrap_or(false);

        Ok(NodeInfo {
            node_id: health.node_id,
            status: if health.status == "healthy" {
                NodeStatus::Healthy
            } else if health.status == "degraded" {
                NodeStatus::Degraded
            } else {
                NodeStatus::Unhealthy
            },
            is_leader,
            last_applied_index: metrics
                .as_ref()
                .and_then(|m| m.last_applied.as_ref().map(|la| la.index)),
            current_term: metrics.as_ref().map(|m| m.current_term),
            uptime_secs: Some(health.uptime_seconds),
            http_addr: self.base_url.clone(),
        })
    }

    /// Get cluster metrics.
    pub async fn get_metrics(&self) -> color_eyre::Result<ClusterMetrics> {
        let metrics = self.get_raft_metrics().await?;

        Ok(ClusterMetrics {
            leader: metrics.current_leader,
            term: metrics.current_term,
            node_count: 1, // Would need to query full cluster
            last_log_index: metrics.last_log_index,
            last_applied_index: metrics.last_applied.as_ref().map(|la| la.index),
        })
    }

    /// Get cluster ticket.
    pub async fn get_cluster_ticket(&self) -> Result<serde_json::Value, reqwest::Error> {
        let url = format!("{}/cluster-ticket", self.base_url);
        self.client.get(&url).send().await?.json().await
    }

    /// Check if the node is healthy.
    pub async fn is_healthy(&self) -> color_eyre::Result<bool> {
        let health = self.get_health().await?;
        Ok(health.status == "healthy")
    }

    /// Write a key-value pair (trait-compatible version).
    pub async fn write(&self, key: String, value: Vec<u8>) -> color_eyre::Result<()> {
        self.write_key(&key, value).await?;
        Ok(())
    }

    /// Read a value by key (trait-compatible version).
    pub async fn read(&self, key: String) -> color_eyre::Result<Option<Vec<u8>>> {
        Ok(self.read_key(&key).await?)
    }
}
