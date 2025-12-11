//! Common client trait for both HTTP and Iroh connections.

use aspen::tui_rpc::NodeDescriptor;
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

    /// Get all discovered cluster nodes (for multi-node clients).
    /// Returns empty vec for single-node clients.
    async fn get_discovered_nodes(&self) -> Result<Vec<NodeDescriptor>> {
        Ok(vec![])
    }

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

    /// List all vaults (keys with "vault:" prefix).
    ///
    /// Returns a list of vault names and their key counts.
    async fn list_vaults(&self) -> Result<Vec<VaultSummary>> {
        // Default implementation: scan keys with vault prefix
        // This works for HTTP clients; Iroh clients may override
        Ok(vec![])
    }

    /// List all keys in a specific vault.
    async fn list_vault_keys(&self, vault: &str) -> Result<Vec<VaultKeyEntry>> {
        let _ = vault;
        Ok(vec![])
    }
}

/// Summary information about a vault.
#[derive(Debug, Clone)]
pub struct VaultSummary {
    /// Name of the vault.
    pub name: String,
    /// Number of keys in the vault.
    pub key_count: u64,
}

/// A key-value entry within a vault.
#[derive(Debug, Clone)]
pub struct VaultKeyEntry {
    /// Key name (without vault prefix).
    pub key: String,
    /// Value (as string).
    pub value: String,
}

/// Wrapper enum for different client types.
pub enum ClientImpl {
    Http(crate::client::AspenClient),
    Iroh(crate::iroh_client::IrohClient),
    MultiNode(crate::iroh_client::MultiNodeClient),
    Disconnected(DisconnectedClient),
}

/// A client that's not connected to any nodes.
/// Returns errors for all operations until replaced with a real client.
pub struct DisconnectedClient;

#[async_trait]
impl ClusterClient for DisconnectedClient {
    async fn get_node_info(&self) -> Result<NodeInfo> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn get_metrics(&self) -> Result<ClusterMetrics> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn get_discovered_nodes(&self) -> Result<Vec<NodeDescriptor>> {
        Ok(vec![])
    }

    async fn is_healthy(&self) -> Result<bool> {
        Ok(false)
    }

    async fn init_cluster(&self) -> Result<()> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn add_learner(&self, _node_id: u64, _addr: String) -> Result<()> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn change_membership(&self, _members: Vec<u64>) -> Result<()> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn write(&self, _key: String, _value: Vec<u8>) -> Result<()> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn read(&self, _key: String) -> Result<Option<Vec<u8>>> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn trigger_snapshot(&self) -> Result<()> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn list_vaults(&self) -> Result<Vec<VaultSummary>> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn list_vault_keys(&self, _vault: &str) -> Result<Vec<VaultKeyEntry>> {
        Err(eyre!("Not connected to any cluster"))
    }
}

/// Helper to convert anyhow::Error to color_eyre::Report.
fn anyhow_to_eyre(err: anyhow::Error) -> color_eyre::Report {
    eyre!("{:#}", err)
}

#[async_trait]
impl ClusterClient for ClientImpl {
    async fn get_discovered_nodes(&self) -> Result<Vec<NodeDescriptor>> {
        match self {
            Self::Http(_) => Ok(vec![]), // HTTP doesn't support discovery yet
            Self::Iroh(_) => Ok(vec![]), // Single Iroh client doesn't discover
            Self::MultiNode(client) => {
                // MultiNodeClient can discover peers
                client.discover_peers().await.map_err(anyhow_to_eyre)
            }
            Self::Disconnected(client) => client.get_discovered_nodes().await,
        }
    }

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
            Self::MultiNode(client) => {
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
            Self::Disconnected(client) => client.get_node_info().await,
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
            Self::MultiNode(client) => {
                let metrics = client.get_raft_metrics().await.map_err(anyhow_to_eyre)?;
                // Use discover_peers to get actual node count
                let node_count = client
                    .discover_peers()
                    .await
                    .map(|peers| peers.len())
                    .unwrap_or(1);
                Ok(ClusterMetrics {
                    leader: metrics.current_leader,
                    term: metrics.current_term,
                    node_count,
                    last_log_index: metrics.last_log_index,
                    last_applied_index: metrics.last_applied_index,
                })
            }
            Self::Disconnected(client) => client.get_metrics().await,
        }
    }

    async fn is_healthy(&self) -> Result<bool> {
        match self {
            Self::Http(client) => client.is_healthy().await,
            Self::Iroh(client) => {
                let health = client.get_health().await.map_err(anyhow_to_eyre)?;
                Ok(health.status == "healthy")
            }
            Self::MultiNode(client) => {
                let health = client.get_health().await.map_err(anyhow_to_eyre)?;
                Ok(health.status == "healthy")
            }
            Self::Disconnected(client) => client.is_healthy().await,
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
            Self::MultiNode(client) => {
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
            Self::Disconnected(client) => client.init_cluster().await,
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
            Self::MultiNode(client) => {
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
            Self::Disconnected(client) => client.add_learner(node_id, addr).await,
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
            Self::MultiNode(client) => {
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
            Self::Disconnected(client) => client.change_membership(members).await,
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
            Self::MultiNode(client) => {
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
            Self::Disconnected(client) => client.write(key, value).await,
        }
    }

    async fn read(&self, key: String) -> Result<Option<Vec<u8>>> {
        match self {
            Self::Http(client) => client.read(key).await,
            Self::Iroh(client) => {
                let result = client.read_key(key).await.map_err(anyhow_to_eyre)?;
                Ok(result.value)
            }
            Self::MultiNode(client) => {
                let result = client.read_key(key).await.map_err(anyhow_to_eyre)?;
                Ok(result.value)
            }
            Self::Disconnected(client) => client.read(key).await,
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
            Self::MultiNode(client) => {
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
            Self::Disconnected(client) => client.trigger_snapshot().await,
        }
    }

    async fn list_vaults(&self) -> Result<Vec<VaultSummary>> {
        match self {
            Self::Http(client) => client.list_vaults().await,
            Self::Iroh(_) => Ok(vec![]),      // TODO: implement for Iroh
            Self::MultiNode(_) => Ok(vec![]), // TODO: implement for MultiNode
            Self::Disconnected(client) => client.list_vaults().await,
        }
    }

    async fn list_vault_keys(&self, vault: &str) -> Result<Vec<VaultKeyEntry>> {
        match self {
            Self::Http(client) => client.list_vault_keys(vault).await,
            Self::Iroh(_) => Ok(vec![]),      // TODO: implement for Iroh
            Self::MultiNode(_) => Ok(vec![]), // TODO: implement for MultiNode
            Self::Disconnected(client) => client.list_vault_keys(vault).await,
        }
    }
}
