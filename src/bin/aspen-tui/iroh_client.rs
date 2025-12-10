//! Iroh client for connecting to Aspen nodes over QUIC.
//!
//! This module provides the client-side implementation for connecting to
//! Aspen nodes using Iroh's peer-to-peer transport with the TUI ALPN.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use aspen::tui_rpc::{
    AddLearnerResultResponse, ChangeMembershipResultResponse, ClusterStateResponse,
    ClusterTicketResponse, HealthResponse, InitResultResponse, MAX_TUI_MESSAGE_SIZE,
    NodeDescriptor, NodeInfoResponse, RaftMetricsResponse, ReadResultResponse,
    SnapshotResultResponse, TuiRpcRequest, TuiRpcResponse, WriteResultResponse,
};
use iroh::{Endpoint, EndpointAddr, SecretKey};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Timeout for individual RPC calls.
const RPC_TIMEOUT: Duration = Duration::from_secs(10);

/// Connection retry delay.
const RETRY_DELAY: Duration = Duration::from_secs(5);

/// Maximum connection retries before giving up.
const MAX_RETRIES: u32 = 5;

/// TUI ALPN for identifying TUI connections.
const TUI_ALPN: &[u8] = b"aspen-tui";

/// Iroh client for TUI connections to Aspen nodes.
pub struct IrohClient {
    /// Iroh endpoint for making connections.
    endpoint: Endpoint,
    /// Target node address.
    target_addr: Arc<RwLock<EndpointAddr>>,
    /// Connection status.
    connected: Arc<RwLock<bool>>,
}

impl IrohClient {
    /// Create a new Iroh client for TUI connections.
    ///
    /// # Arguments
    /// * `target_addr` - The endpoint address of the target node
    ///
    /// # Returns
    /// A new IrohClient instance.
    pub async fn new(target_addr: EndpointAddr) -> Result<Self> {
        // Generate a new secret key for the TUI client
        // Use bytes from thread_rng since the iroh SecretKey expects a CryptoRng
        // which has version compatibility issues with the rand crate
        use rand::RngCore;
        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_bytes);
        let secret_key = SecretKey::from(key_bytes);

        // Build the Iroh endpoint with TUI ALPN
        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .alpns(vec![TUI_ALPN.to_vec()])
            .bind()
            .await
            .context("failed to bind Iroh endpoint")?;

        info!(
            node_id = %endpoint.id(),
            target_node_id = %target_addr.id,
            "TUI client endpoint created"
        );

        Ok(Self {
            endpoint,
            target_addr: Arc::new(RwLock::new(target_addr)),
            connected: Arc::new(RwLock::new(false)),
        })
    }

    /// Update the target node address.
    pub async fn set_target(&self, addr: EndpointAddr) {
        let mut target = self.target_addr.write().await;
        *target = addr;

        // Mark as disconnected so next request will reconnect
        let mut connected = self.connected.write().await;
        *connected = false;
    }

    /// Send an RPC request to the target node.
    async fn send_rpc(&self, request: TuiRpcRequest) -> Result<TuiRpcResponse> {
        let target_addr = self.target_addr.read().await.clone();

        debug!(
            target_node_id = %target_addr.id,
            request_type = ?request,
            "sending TUI RPC request"
        );

        // Connect to the target node
        let connection = self
            .endpoint
            .connect(target_addr, TUI_ALPN)
            .await
            .context("failed to connect to node")?;

        // Open a bidirectional stream for the RPC
        let (mut send, mut recv) = connection
            .open_bi()
            .await
            .context("failed to open stream")?;

        // Serialize and send the request
        let request_bytes = postcard::to_stdvec(&request).context("failed to serialize request")?;

        send.write_all(&request_bytes)
            .await
            .context("failed to send request")?;
        send.finish().context("failed to finish send stream")?;

        // Read the response
        let response_bytes = recv
            .read_to_end(MAX_TUI_MESSAGE_SIZE)
            .await
            .context("failed to read response")?;

        // Deserialize the response
        let response: TuiRpcResponse =
            postcard::from_bytes(&response_bytes).context("failed to deserialize response")?;

        // Check for error response
        if let TuiRpcResponse::Error(err) = &response {
            anyhow::bail!("RPC error {}: {}", err.code, err.message);
        }

        // Mark as connected on successful RPC
        let mut connected = self.connected.write().await;
        *connected = true;

        Ok(response)
    }

    /// Send an RPC request with automatic retry on failure.
    async fn send_rpc_with_retry(&self, request: TuiRpcRequest) -> Result<TuiRpcResponse> {
        let mut retries = 0;

        loop {
            match tokio::time::timeout(RPC_TIMEOUT, self.send_rpc(request.clone())).await {
                Ok(Ok(response)) => return Ok(response),
                Ok(Err(err)) => {
                    warn!(
                        error = %err,
                        retries,
                        max_retries = MAX_RETRIES,
                        "RPC request failed"
                    );

                    if retries >= MAX_RETRIES {
                        return Err(err).context("max retries exceeded");
                    }

                    retries += 1;
                    tokio::time::sleep(RETRY_DELAY).await;
                }
                Err(_) => {
                    warn!(retries, max_retries = MAX_RETRIES, "RPC request timed out");

                    if retries >= MAX_RETRIES {
                        anyhow::bail!("RPC request timed out after {} retries", MAX_RETRIES);
                    }

                    retries += 1;
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }
    }

    /// Check if the client is connected.
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Get health status from the node.
    pub async fn get_health(&self) -> Result<HealthResponse> {
        let response = self.send_rpc_with_retry(TuiRpcRequest::GetHealth).await?;

        match response {
            TuiRpcResponse::Health(health) => Ok(health),
            _ => anyhow::bail!("unexpected response type for GetHealth"),
        }
    }

    /// Get Raft metrics from the node.
    pub async fn get_raft_metrics(&self) -> Result<RaftMetricsResponse> {
        let response = self
            .send_rpc_with_retry(TuiRpcRequest::GetRaftMetrics)
            .await?;

        match response {
            TuiRpcResponse::RaftMetrics(metrics) => Ok(metrics),
            _ => anyhow::bail!("unexpected response type for GetRaftMetrics"),
        }
    }

    /// Get the current leader node ID.
    pub async fn get_leader(&self) -> Result<Option<u64>> {
        let response = self.send_rpc_with_retry(TuiRpcRequest::GetLeader).await?;

        match response {
            TuiRpcResponse::Leader(leader) => Ok(leader),
            _ => anyhow::bail!("unexpected response type for GetLeader"),
        }
    }

    /// Get node information.
    pub async fn get_node_info(&self) -> Result<NodeInfoResponse> {
        let response = self.send_rpc_with_retry(TuiRpcRequest::GetNodeInfo).await?;

        match response {
            TuiRpcResponse::NodeInfo(info) => Ok(info),
            _ => anyhow::bail!("unexpected response type for GetNodeInfo"),
        }
    }

    /// Get cluster ticket for joining.
    pub async fn get_cluster_ticket(&self) -> Result<ClusterTicketResponse> {
        let response = self
            .send_rpc_with_retry(TuiRpcRequest::GetClusterTicket)
            .await?;

        match response {
            TuiRpcResponse::ClusterTicket(ticket) => Ok(ticket),
            _ => anyhow::bail!("unexpected response type for GetClusterTicket"),
        }
    }

    /// Initialize a new cluster.
    pub async fn init_cluster(&self) -> Result<InitResultResponse> {
        let response = self.send_rpc_with_retry(TuiRpcRequest::InitCluster).await?;

        match response {
            TuiRpcResponse::InitResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for InitCluster"),
        }
    }

    /// Read a key from the key-value store.
    pub async fn read_key(&self, key: String) -> Result<ReadResultResponse> {
        let response = self
            .send_rpc_with_retry(TuiRpcRequest::ReadKey { key })
            .await?;

        match response {
            TuiRpcResponse::ReadResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for ReadKey"),
        }
    }

    /// Write a key-value pair to the store.
    pub async fn write_key(&self, key: String, value: Vec<u8>) -> Result<WriteResultResponse> {
        let response = self
            .send_rpc_with_retry(TuiRpcRequest::WriteKey { key, value })
            .await?;

        match response {
            TuiRpcResponse::WriteResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for WriteKey"),
        }
    }

    /// Trigger a Raft snapshot.
    pub async fn trigger_snapshot(&self) -> Result<SnapshotResultResponse> {
        let response = self
            .send_rpc_with_retry(TuiRpcRequest::TriggerSnapshot)
            .await?;

        match response {
            TuiRpcResponse::SnapshotResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for TriggerSnapshot"),
        }
    }

    /// Add a learner node to the cluster.
    pub async fn add_learner(
        &self,
        node_id: u64,
        addr: String,
    ) -> Result<AddLearnerResultResponse> {
        let response = self
            .send_rpc_with_retry(TuiRpcRequest::AddLearner { node_id, addr })
            .await?;

        match response {
            TuiRpcResponse::AddLearnerResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for AddLearner"),
        }
    }

    /// Change cluster membership.
    pub async fn change_membership(
        &self,
        members: Vec<u64>,
    ) -> Result<ChangeMembershipResultResponse> {
        let response = self
            .send_rpc_with_retry(TuiRpcRequest::ChangeMembership { members })
            .await?;

        match response {
            TuiRpcResponse::ChangeMembershipResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for ChangeMembership"),
        }
    }

    /// Send a ping to check connectivity.
    pub async fn ping(&self) -> Result<()> {
        let response = self.send_rpc_with_retry(TuiRpcRequest::Ping).await?;

        match response {
            TuiRpcResponse::Pong => Ok(()),
            _ => anyhow::bail!("unexpected response type for Ping"),
        }
    }

    /// Get cluster state with all known nodes.
    pub async fn get_cluster_state(&self) -> Result<ClusterStateResponse> {
        let response = self
            .send_rpc_with_retry(TuiRpcRequest::GetClusterState)
            .await?;

        match response {
            TuiRpcResponse::ClusterState(state) => Ok(state),
            _ => anyhow::bail!("unexpected response type for GetClusterState"),
        }
    }

    /// Shutdown the client and close all connections.
    pub async fn shutdown(self) -> Result<()> {
        self.endpoint.close().await;
        Ok(())
    }
}

/// Parse an Aspen cluster ticket into a list of EndpointAddrs.
///
/// Tickets have the format: "aspen{base32-encoded-data}"
/// Returns all bootstrap peers in the ticket.
pub fn parse_cluster_ticket(ticket: &str) -> Result<Vec<EndpointAddr>> {
    use aspen::cluster::ticket::AspenClusterTicket;

    let ticket = AspenClusterTicket::deserialize(ticket)?;

    // Extract ALL bootstrap peer endpoint IDs
    let endpoints: Vec<EndpointAddr> = ticket
        .bootstrap
        .iter()
        .map(|peer_id| EndpointAddr::new(*peer_id))
        .collect();

    if endpoints.is_empty() {
        anyhow::bail!("no bootstrap peers in ticket");
    }

    Ok(endpoints)
}

/// Maximum number of nodes to track in multi-node client.
///
/// Tiger Style: Bounded to prevent unbounded resource growth.
const MAX_TRACKED_NODES: usize = 16;

/// Connection state for a single node.
#[derive(Debug)]
pub struct NodeConnection {
    /// Node identifier.
    pub node_id: u64,
    /// Target endpoint address.
    pub endpoint_addr: EndpointAddr,
    /// Whether the node is currently reachable.
    pub is_reachable: bool,
    /// Whether this node is the leader.
    pub is_leader: bool,
    /// Whether this node is a voter.
    pub is_voter: bool,
}

/// Multi-node client for connecting to and managing multiple cluster nodes.
///
/// Tiger Style:
/// - Bounded node count (MAX_TRACKED_NODES)
/// - Shared Iroh endpoint across all connections
/// - Automatic discovery via GetClusterState RPC
pub struct MultiNodeClient {
    /// Shared Iroh endpoint for all connections.
    endpoint: Arc<Endpoint>,
    /// Known nodes and their connection state.
    nodes: RwLock<std::collections::HashMap<u64, NodeConnection>>,
    /// Primary target node for discovery queries.
    primary_target: RwLock<EndpointAddr>,
    /// Current leader node ID, if known.
    leader_id: RwLock<Option<u64>>,
}

impl MultiNodeClient {
    /// Create a new multi-node client.
    ///
    /// # Arguments
    /// * `initial_targets` - Bootstrap endpoint addresses to connect to
    ///
    /// # Returns
    /// A new MultiNodeClient instance.
    pub async fn new(initial_targets: Vec<EndpointAddr>) -> Result<Self> {
        if initial_targets.is_empty() {
            anyhow::bail!("at least one bootstrap target is required");
        }

        // Generate a new secret key for the TUI client
        use rand::RngCore;
        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_bytes);
        let secret_key = SecretKey::from(key_bytes);

        // Build the shared Iroh endpoint with TUI ALPN
        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .alpns(vec![TUI_ALPN.to_vec()])
            .bind()
            .await
            .context("failed to bind Iroh endpoint")?;

        info!(
            node_id = %endpoint.id(),
            bootstrap_count = initial_targets.len(),
            "Multi-node TUI client endpoint created"
        );

        let primary_target = initial_targets[0].clone();

        Ok(Self {
            endpoint: Arc::new(endpoint),
            nodes: RwLock::new(std::collections::HashMap::new()),
            primary_target: RwLock::new(primary_target),
            leader_id: RwLock::new(None),
        })
    }

    /// Send an RPC request to a specific node address.
    async fn send_rpc_to(
        &self,
        target: &EndpointAddr,
        request: TuiRpcRequest,
    ) -> Result<TuiRpcResponse> {
        debug!(
            target_node_id = %target.id,
            request_type = ?request,
            "sending TUI RPC request"
        );

        // Connect to the target node
        let connection = self
            .endpoint
            .connect(target.clone(), TUI_ALPN)
            .await
            .context("failed to connect to node")?;

        // Open a bidirectional stream for the RPC
        let (mut send, mut recv) = connection
            .open_bi()
            .await
            .context("failed to open stream")?;

        // Serialize and send the request
        let request_bytes = postcard::to_stdvec(&request).context("failed to serialize request")?;

        send.write_all(&request_bytes)
            .await
            .context("failed to send request")?;
        send.finish().context("failed to finish send stream")?;

        // Read the response
        let response_bytes = recv
            .read_to_end(MAX_TUI_MESSAGE_SIZE)
            .await
            .context("failed to read response")?;

        // Deserialize the response
        let response: TuiRpcResponse =
            postcard::from_bytes(&response_bytes).context("failed to deserialize response")?;

        // Check for error response
        if let TuiRpcResponse::Error(err) = &response {
            anyhow::bail!("RPC error {}: {}", err.code, err.message);
        }

        Ok(response)
    }

    /// Send an RPC to the primary target with retry.
    async fn send_rpc_primary(&self, request: TuiRpcRequest) -> Result<TuiRpcResponse> {
        let target = self.primary_target.read().await.clone();
        let mut retries = 0;

        loop {
            match tokio::time::timeout(RPC_TIMEOUT, self.send_rpc_to(&target, request.clone()))
                .await
            {
                Ok(Ok(response)) => return Ok(response),
                Ok(Err(err)) => {
                    warn!(
                        error = %err,
                        retries,
                        max_retries = MAX_RETRIES,
                        "RPC request failed"
                    );

                    if retries >= MAX_RETRIES {
                        return Err(err).context("max retries exceeded");
                    }

                    retries += 1;
                    tokio::time::sleep(RETRY_DELAY).await;
                }
                Err(_) => {
                    warn!(retries, max_retries = MAX_RETRIES, "RPC request timed out");

                    if retries >= MAX_RETRIES {
                        anyhow::bail!("RPC request timed out after {} retries", MAX_RETRIES);
                    }

                    retries += 1;
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }
    }

    /// Discover peers by querying the primary node for cluster state.
    ///
    /// Updates internal node tracking with discovered peers.
    pub async fn discover_peers(&self) -> Result<Vec<NodeDescriptor>> {
        let response = self
            .send_rpc_primary(TuiRpcRequest::GetClusterState)
            .await?;

        match response {
            TuiRpcResponse::ClusterState(state) => {
                // Update leader ID
                {
                    let mut leader = self.leader_id.write().await;
                    *leader = state.leader_id;
                }

                // Update node tracking
                let mut nodes = self.nodes.write().await;

                // Tiger Style: Bounded node count
                for descriptor in state.nodes.iter().take(MAX_TRACKED_NODES) {
                    // Parse the endpoint address from the descriptor
                    // The endpoint_addr is stored as a debug string, so we need to handle this
                    // For now, we'll just track what we know
                    nodes.entry(descriptor.node_id).and_modify(|conn| {
                        conn.is_leader = descriptor.is_leader;
                        conn.is_voter = descriptor.is_voter;
                        conn.is_reachable = true;
                    });
                }

                info!(
                    discovered_nodes = state.nodes.len(),
                    leader_id = ?state.leader_id,
                    "discovered cluster peers"
                );

                Ok(state.nodes)
            }
            _ => anyhow::bail!("unexpected response type for GetClusterState"),
        }
    }

    /// Add a new node to track.
    ///
    /// # Arguments
    /// * `node_id` - Node identifier
    /// * `endpoint_addr` - Endpoint address for the node
    pub async fn add_node(&self, node_id: u64, endpoint_addr: EndpointAddr) {
        let mut nodes = self.nodes.write().await;

        // Tiger Style: Bounded node count
        if nodes.len() >= MAX_TRACKED_NODES && !nodes.contains_key(&node_id) {
            warn!(
                node_id,
                max_nodes = MAX_TRACKED_NODES,
                "node limit reached, not adding new node"
            );
            return;
        }

        nodes.insert(
            node_id,
            NodeConnection {
                node_id,
                endpoint_addr,
                is_reachable: false,
                is_leader: false,
                is_voter: false,
            },
        );

        info!(node_id, "added node to tracking");
    }

    /// Remove a node from tracking.
    pub async fn remove_node(&self, node_id: u64) {
        let mut nodes = self.nodes.write().await;
        nodes.remove(&node_id);
        info!(node_id, "removed node from tracking");
    }

    /// Get all tracked nodes.
    pub async fn get_nodes(&self) -> Vec<NodeConnection> {
        let nodes = self.nodes.read().await;
        nodes
            .values()
            .map(|conn| NodeConnection {
                node_id: conn.node_id,
                endpoint_addr: conn.endpoint_addr.clone(),
                is_reachable: conn.is_reachable,
                is_leader: conn.is_leader,
                is_voter: conn.is_voter,
            })
            .collect()
    }

    /// Get the current leader ID.
    pub async fn get_leader_id(&self) -> Option<u64> {
        *self.leader_id.read().await
    }

    /// Set the primary target for discovery queries.
    pub async fn set_primary_target(&self, addr: EndpointAddr) {
        let mut target = self.primary_target.write().await;
        *target = addr;
    }

    /// Get node info from the primary target.
    pub async fn get_node_info(&self) -> Result<NodeInfoResponse> {
        let response = self.send_rpc_primary(TuiRpcRequest::GetNodeInfo).await?;

        match response {
            TuiRpcResponse::NodeInfo(info) => Ok(info),
            _ => anyhow::bail!("unexpected response type for GetNodeInfo"),
        }
    }

    /// Get health from the primary target.
    pub async fn get_health(&self) -> Result<HealthResponse> {
        let response = self.send_rpc_primary(TuiRpcRequest::GetHealth).await?;

        match response {
            TuiRpcResponse::Health(health) => Ok(health),
            _ => anyhow::bail!("unexpected response type for GetHealth"),
        }
    }

    /// Get Raft metrics from the primary target.
    pub async fn get_raft_metrics(&self) -> Result<RaftMetricsResponse> {
        let response = self.send_rpc_primary(TuiRpcRequest::GetRaftMetrics).await?;

        match response {
            TuiRpcResponse::RaftMetrics(metrics) => Ok(metrics),
            _ => anyhow::bail!("unexpected response type for GetRaftMetrics"),
        }
    }

    /// Initialize the cluster via the primary target.
    pub async fn init_cluster(&self) -> Result<InitResultResponse> {
        let response = self.send_rpc_primary(TuiRpcRequest::InitCluster).await?;

        match response {
            TuiRpcResponse::InitResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for InitCluster"),
        }
    }

    /// Read a key from the cluster.
    pub async fn read_key(&self, key: String) -> Result<ReadResultResponse> {
        let response = self
            .send_rpc_primary(TuiRpcRequest::ReadKey { key })
            .await?;

        match response {
            TuiRpcResponse::ReadResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for ReadKey"),
        }
    }

    /// Write a key-value pair to the cluster.
    pub async fn write_key(&self, key: String, value: Vec<u8>) -> Result<WriteResultResponse> {
        let response = self
            .send_rpc_primary(TuiRpcRequest::WriteKey { key, value })
            .await?;

        match response {
            TuiRpcResponse::WriteResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for WriteKey"),
        }
    }

    /// Trigger a Raft snapshot.
    pub async fn trigger_snapshot(&self) -> Result<SnapshotResultResponse> {
        let response = self
            .send_rpc_primary(TuiRpcRequest::TriggerSnapshot)
            .await?;

        match response {
            TuiRpcResponse::SnapshotResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for TriggerSnapshot"),
        }
    }

    /// Add a learner node to the cluster.
    pub async fn add_learner(
        &self,
        node_id: u64,
        addr: String,
    ) -> Result<AddLearnerResultResponse> {
        let response = self
            .send_rpc_primary(TuiRpcRequest::AddLearner { node_id, addr })
            .await?;

        match response {
            TuiRpcResponse::AddLearnerResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for AddLearner"),
        }
    }

    /// Change cluster membership.
    pub async fn change_membership(
        &self,
        members: Vec<u64>,
    ) -> Result<ChangeMembershipResultResponse> {
        let response = self
            .send_rpc_primary(TuiRpcRequest::ChangeMembership { members })
            .await?;

        match response {
            TuiRpcResponse::ChangeMembershipResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for ChangeMembership"),
        }
    }

    /// Shutdown the client.
    pub async fn shutdown(self) -> Result<()> {
        // Arc::try_unwrap will only succeed if we're the only reference holder
        match Arc::try_unwrap(self.endpoint) {
            Ok(endpoint) => {
                endpoint.close().await;
            }
            Err(_) => {
                warn!("endpoint has other references, not closing");
            }
        }
        Ok(())
    }
}
