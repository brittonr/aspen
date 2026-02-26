//! Cluster operations: refresh, init, connect.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use tracing::info;
use tracing::warn;

use super::state::App;
use super::types::ActiveView;
use crate::client_trait::ClientImpl;
use crate::client_trait::ClusterClient;
use crate::iroh_client::MultiNodeClient;
use crate::iroh_client::parse_cluster_ticket;
use crate::types::NodeInfo;
use crate::types::NodeStatus;

impl App {
    /// Refresh cluster state from the node.
    ///
    /// For HTTP clients: queries each known node port individually.
    /// For MultiNode clients: uses GetClusterState to discover all nodes automatically.
    pub(crate) async fn refresh_cluster_state(&mut self) {
        self.refreshing = true;
        let mut new_nodes = BTreeMap::new();

        // First, get info from the connected node
        match self.client.get_node_info().await {
            Ok(info) => {
                new_nodes.insert(info.node_id, info);

                // Try to get metrics - this also triggers discovery for MultiNode client
                if let Ok(metrics) = self.client.get_metrics().await {
                    self.cluster_metrics = Some(metrics.clone());

                    // For MultiNode (Iroh) client, populate all discovered nodes
                    {
                        // Use the trait method to get discovered nodes
                        match self.client.get_discovered_nodes().await {
                            Ok(discovered_nodes) if !discovered_nodes.is_empty() => {
                                info!(
                                    discovered_count = discovered_nodes.len(),
                                    leader = ?metrics.leader,
                                    "discovered cluster peers via GetClusterState"
                                );

                                // Add each discovered node to our nodes map
                                for node_desc in discovered_nodes {
                                    // Create a NodeInfo from the discovered node descriptor
                                    let node_info = NodeInfo {
                                        node_id: node_desc.node_id,
                                        status: NodeStatus::Healthy, // Assume healthy for discovered nodes
                                        is_leader: node_desc.is_leader,
                                        last_applied_index: None, // Will be updated on next metrics call
                                        current_term: Some(metrics.term),
                                        uptime_secs: None,
                                        addr: format!("iroh://{}", node_desc.endpoint_addr),
                                    };
                                    new_nodes.insert(node_desc.node_id, node_info);
                                }
                            }
                            Ok(_) => {
                                // No discovered nodes (single-node client or HTTP)
                                if metrics.node_count > 1 {
                                    info!(
                                        total_nodes = metrics.node_count,
                                        leader = ?metrics.leader,
                                        "cluster has multiple nodes but discovery not available"
                                    );
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, "failed to discover peers");
                            }
                        }
                    }

                    // For Iroh MultiNode client, we already got the cluster state from
                    // get_metrics() The ClusterMetrics now includes the actual
                    // discovered node count
                }
            }
            Err(e) => {
                warn!(error = %e, "failed to get node info");
                self.set_status(&format!("Failed to refresh: {}", e));
            }
        }

        self.nodes = new_nodes;
        self.last_refresh = Some(Instant::now());
        self.refreshing = false;

        // Also refresh vaults if we're on the vaults view
        if self.active_view == ActiveView::Vaults {
            self.refresh_vaults().await;
        }
    }

    /// Initialize the cluster.
    pub(crate) async fn init_cluster(&mut self) {
        match self.client.init_cluster().await {
            Ok(_) => {
                self.set_status("Cluster initialized successfully");
                info!("cluster initialized");
                self.refresh_cluster_state().await;
            }
            Err(e) => {
                self.set_status(&format!("Init failed: {}", e));
                warn!(error = %e, "cluster init failed");
            }
        }
    }

    /// Connect via Iroh ticket.
    pub(crate) async fn connect_iroh_ticket(&mut self, ticket: &str) {
        let ticket = ticket.trim();

        if ticket.is_empty() {
            self.set_status("No ticket provided");
            return;
        }

        // Parse the ticket to get all endpoint addresses
        match parse_cluster_ticket(ticket) {
            Ok(endpoint_addrs) => {
                info!("Ticket contains {} bootstrap peers", endpoint_addrs.len());

                // Create MultiNodeClient with all bootstrap endpoints
                match MultiNodeClient::new(endpoint_addrs).await {
                    Ok(multi_client) => {
                        let new_client: Arc<dyn ClusterClient> = Arc::new(ClientImpl::MultiNode(multi_client));

                        // Test the connection
                        match new_client.get_node_info().await {
                            Ok(node_info) => {
                                // Connection successful, replace the client
                                self.client = new_client;
                                self.set_status(&format!("Connected to cluster via Iroh (node {})", node_info.node_id));
                                // Immediately refresh to populate all discovered nodes
                                self.refresh_cluster_state().await;
                            }
                            Err(e) => {
                                self.set_status(&format!("Failed to connect via ticket: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        self.set_status(&format!("Failed to create Iroh client: {:#}", e));
                    }
                }
            }
            Err(e) => {
                self.set_status(&format!("Invalid ticket format: {:#}", e));
            }
        }
    }
}
