//! Common client trait for both HTTP and Iroh connections.

use async_trait::async_trait;
use color_eyre::Result;
use color_eyre::eyre::eyre;

use crate::client::{ClusterMetrics, NodeInfo, NodeStatus};

/// Trait for Aspen cluster clients.
///
/// Implemented by both HTTP and Iroh clients for unified access.
#[async_trait]
pub trait ClusterClient: Send + Sync {
    /// Get node information.
    async fn get_node_info(&self) -> Result<NodeInfo>;

    /// Get cluster metrics.
    async fn get_metrics(&self) -> Result<ClusterMetrics>;

    /// Check if the node is healthy.
    async fn is_healthy(&self) -> Result<bool>;

    /// Initialize the cluster.
    async fn init_cluster(&self) -> Result<()>;

    /// Add a learner node.
    async fn add_learner(&self, node_id: u64, addr: String) -> Result<()>;

    /// Change cluster membership.
    async fn change_membership(&self, members: Vec<u64>) -> Result<()>;

    /// Write a key-value pair.
    async fn write(&self, key: String, value: Vec<u8>) -> Result<()>;

    /// Read a value by key.
    async fn read(&self, key: String) -> Result<Option<Vec<u8>>>;

    /// Trigger a snapshot.
    async fn trigger_snapshot(&self) -> Result<()>;
}

/// Wrapper enum for different client types.
pub enum ClientImpl {
    Http(crate::client::AspenClient),
    Iroh(crate::iroh_client::IrohClient),
}

/// Helper to convert anyhow::Error to color_eyre::Report.
fn anyhow_to_eyre(err: anyhow::Error) -> color_eyre::Report {
    eyre!("{:#}", err)
}

#[async_trait]
impl ClusterClient for ClientImpl {
    async fn get_node_info(&self) -> Result<NodeInfo> {
        match self {
            Self::Http(client) => client.get_node_info().await,
            Self::Iroh(client) => {
                let info = client.get_node_info().await.map_err(anyhow_to_eyre)?;
                let health = client.get_health().await.map_err(anyhow_to_eyre)?;
                let metrics = client.get_raft_metrics().await.ok();

                let is_leader = metrics
                    .as_ref()
                    .and_then(|m| m.current_leader)
                    .map(|l| l == info.node_id)
                    .unwrap_or(false);

                Ok(NodeInfo {
                    node_id: info.node_id,
                    status: if health.status == "healthy" {
                        NodeStatus::Healthy
                    } else if health.status == "degraded" {
                        NodeStatus::Degraded
                    } else {
                        NodeStatus::Unhealthy
                    },
                    is_leader,
                    last_applied_index: metrics.as_ref().and_then(|m| m.last_applied_index),
                    current_term: metrics.as_ref().map(|m| m.current_term),
                    uptime_secs: Some(health.uptime_seconds),
                    http_addr: format!("iroh://{}", info.endpoint_addr),
                })
            }
        }
    }

    async fn get_metrics(&self) -> Result<ClusterMetrics> {
        match self {
            Self::Http(client) => client.get_metrics().await,
            Self::Iroh(client) => {
                let metrics = client.get_raft_metrics().await.map_err(anyhow_to_eyre)?;
                Ok(ClusterMetrics {
                    leader: metrics.current_leader,
                    term: metrics.current_term,
                    node_count: 1, // Would need to query cluster for full count
                    last_log_index: metrics.last_log_index,
                    last_applied_index: metrics.last_applied_index,
                })
            }
        }
    }

    async fn is_healthy(&self) -> Result<bool> {
        match self {
            Self::Http(client) => client.is_healthy().await,
            Self::Iroh(client) => {
                let health = client.get_health().await.map_err(anyhow_to_eyre)?;
                Ok(health.status == "healthy")
            }
        }
    }

    async fn init_cluster(&self) -> Result<()> {
        match self {
            Self::Http(client) => {
                client.init_cluster().await.map_err(|e| eyre!("{}", e))?;
                Ok(())
            }
            Self::Iroh(client) => {
                let result = client.init_cluster().await.map_err(anyhow_to_eyre)?;
                if !result.success {
                    if let Some(error) = result.error {
                        return Err(eyre!("Failed to initialize cluster: {}", error));
                    } else {
                        return Err(eyre!("Failed to initialize cluster"));
                    }
                }
                Ok(())
            }
        }
    }

    async fn add_learner(&self, node_id: u64, addr: String) -> Result<()> {
        match self {
            Self::Http(client) => {
                client
                    .add_learner(node_id, &addr)
                    .await
                    .map_err(|e| eyre!("{}", e))?;
                Ok(())
            }
            Self::Iroh(client) => {
                let result = client
                    .add_learner(node_id, addr)
                    .await
                    .map_err(anyhow_to_eyre)?;
                if !result.success {
                    if let Some(error) = result.error {
                        return Err(eyre!("Failed to add learner: {}", error));
                    } else {
                        return Err(eyre!("Failed to add learner"));
                    }
                }
                Ok(())
            }
        }
    }

    async fn change_membership(&self, members: Vec<u64>) -> Result<()> {
        match self {
            Self::Http(client) => {
                client
                    .change_membership(members)
                    .await
                    .map_err(|e| eyre!("{}", e))?;
                Ok(())
            }
            Self::Iroh(client) => {
                let result = client
                    .change_membership(members)
                    .await
                    .map_err(anyhow_to_eyre)?;
                if !result.success {
                    if let Some(error) = result.error {
                        return Err(eyre!("Failed to change membership: {}", error));
                    } else {
                        return Err(eyre!("Failed to change membership"));
                    }
                }
                Ok(())
            }
        }
    }

    async fn write(&self, key: String, value: Vec<u8>) -> Result<()> {
        match self {
            Self::Http(client) => client.write(key, value).await,
            Self::Iroh(client) => {
                let result = client.write_key(key, value).await.map_err(anyhow_to_eyre)?;
                if !result.success {
                    if let Some(error) = result.error {
                        return Err(eyre!("Failed to write key: {}", error));
                    } else {
                        return Err(eyre!("Failed to write key"));
                    }
                }
                Ok(())
            }
        }
    }

    async fn read(&self, key: String) -> Result<Option<Vec<u8>>> {
        match self {
            Self::Http(client) => client.read(key).await,
            Self::Iroh(client) => {
                let result = client.read_key(key).await.map_err(anyhow_to_eyre)?;
                Ok(result.value)
            }
        }
    }

    async fn trigger_snapshot(&self) -> Result<()> {
        match self {
            Self::Http(client) => {
                client
                    .trigger_snapshot()
                    .await
                    .map_err(|e| eyre!("{}", e))?;
                Ok(())
            }
            Self::Iroh(client) => {
                let result = client.trigger_snapshot().await.map_err(anyhow_to_eyre)?;
                if !result.success {
                    if let Some(error) = result.error {
                        return Err(eyre!("Failed to trigger snapshot: {}", error));
                    } else {
                        return Err(eyre!("Failed to trigger snapshot"));
                    }
                }
                Ok(())
            }
        }
    }
}
