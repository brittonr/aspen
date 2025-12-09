//! Iroh client for connecting to Aspen nodes over QUIC.
//!
//! This module provides the client-side implementation for connecting to
//! Aspen nodes using Iroh's peer-to-peer transport with the TUI ALPN.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use aspen::tui_rpc::{
    TuiRpcRequest, TuiRpcResponse, HealthResponse, RaftMetricsResponse,
    NodeInfoResponse, ClusterTicketResponse, InitResultResponse,
    ReadResultResponse, WriteResultResponse, SnapshotResultResponse,
    AddLearnerResultResponse, ChangeMembershipResultResponse,
    MAX_TUI_MESSAGE_SIZE,
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
        let connection = self.endpoint
            .connect(target_addr, TUI_ALPN)
            .await
            .context("failed to connect to node")?;

        // Open a bidirectional stream for the RPC
        let (mut send, mut recv) = connection
            .open_bi()
            .await
            .context("failed to open stream")?;

        // Serialize and send the request
        let request_bytes = postcard::to_stdvec(&request)
            .context("failed to serialize request")?;

        send.write_all(&request_bytes)
            .await
            .context("failed to send request")?;
        send.finish()
            .context("failed to finish send stream")?;

        // Read the response
        let response_bytes = recv
            .read_to_end(MAX_TUI_MESSAGE_SIZE)
            .await
            .context("failed to read response")?;

        // Deserialize the response
        let response: TuiRpcResponse = postcard::from_bytes(&response_bytes)
            .context("failed to deserialize response")?;

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
                    warn!(
                        retries,
                        max_retries = MAX_RETRIES,
                        "RPC request timed out"
                    );

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
        let response = self.send_rpc_with_retry(TuiRpcRequest::GetRaftMetrics).await?;

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
        let response = self.send_rpc_with_retry(TuiRpcRequest::GetClusterTicket).await?;

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
        let response = self.send_rpc_with_retry(TuiRpcRequest::ReadKey { key }).await?;

        match response {
            TuiRpcResponse::ReadResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for ReadKey"),
        }
    }

    /// Write a key-value pair to the store.
    pub async fn write_key(&self, key: String, value: Vec<u8>) -> Result<WriteResultResponse> {
        let response = self.send_rpc_with_retry(TuiRpcRequest::WriteKey { key, value }).await?;

        match response {
            TuiRpcResponse::WriteResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for WriteKey"),
        }
    }

    /// Trigger a Raft snapshot.
    pub async fn trigger_snapshot(&self) -> Result<SnapshotResultResponse> {
        let response = self.send_rpc_with_retry(TuiRpcRequest::TriggerSnapshot).await?;

        match response {
            TuiRpcResponse::SnapshotResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for TriggerSnapshot"),
        }
    }

    /// Add a learner node to the cluster.
    pub async fn add_learner(&self, node_id: u64, addr: String) -> Result<AddLearnerResultResponse> {
        let response = self.send_rpc_with_retry(TuiRpcRequest::AddLearner { node_id, addr }).await?;

        match response {
            TuiRpcResponse::AddLearnerResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for AddLearner"),
        }
    }

    /// Change cluster membership.
    pub async fn change_membership(&self, members: Vec<u64>) -> Result<ChangeMembershipResultResponse> {
        let response = self.send_rpc_with_retry(TuiRpcRequest::ChangeMembership { members }).await?;

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

    /// Shutdown the client and close all connections.
    pub async fn shutdown(self) -> Result<()> {
        self.endpoint.close().await;
        Ok(())
    }
}

/// Parse an Aspen cluster ticket into an EndpointAddr.
///
/// Tickets have the format: "aspen{base32-encoded-data}"
pub fn parse_cluster_ticket(ticket: &str) -> Result<EndpointAddr> {
    use aspen::cluster::ticket::AspenClusterTicket;

    let ticket = AspenClusterTicket::deserialize(ticket)?;

    // Extract the first bootstrap peer endpoint ID
    let bootstrap_peer_id = ticket
        .bootstrap
        .iter()
        .next()
        .context("no bootstrap peers in ticket")?;

    // Create an EndpointAddr with just the node ID (no direct addresses)
    // The Iroh endpoint will use discovery mechanisms to find the node
    let endpoint_addr = EndpointAddr::new(*bootstrap_peer_id);

    Ok(endpoint_addr)
}
