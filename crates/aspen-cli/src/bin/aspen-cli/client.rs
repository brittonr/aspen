//! Aspen client for CLI operations.
//!
//! Handles Iroh P2P connections and RPC communication with Aspen nodes.
//! Adapted from the TUI client implementation.

use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_auth::CapabilityToken;
use aspen_client_api::MAX_CLIENT_MESSAGE_SIZE;
use aspen_client_rpc::CLIENT_ALPN;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_cluster::ticket::AspenClusterTicket;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::EndpointId;
use iroh::endpoint::VarInt;
use tokio::time::timeout;
use tracing::debug;
use tracing::warn;

/// Maximum number of retry attempts for RPC calls per peer.
const MAX_RETRIES_PER_PEER: u32 = 2;

/// Delay between retry attempts.
const RETRY_DELAY: Duration = Duration::from_millis(500);

/// Aspen client for sending RPC requests to cluster nodes.
pub struct AspenClient {
    endpoint: Endpoint,
    ticket: AspenClusterTicket,
    rpc_timeout: Duration,
    token: Option<CapabilityToken>,
}

impl AspenClient {
    /// Connect to an Aspen cluster using a ticket.
    ///
    /// # Arguments
    ///
    /// * `ticket_str` - Base32-encoded cluster ticket (aspen...)
    /// * `rpc_timeout` - RPC timeout duration
    /// * `token` - Optional capability token for authentication
    pub async fn connect(ticket_str: &str, rpc_timeout: Duration, token: Option<CapabilityToken>) -> Result<Self> {
        // Parse the ticket
        let ticket = AspenClusterTicket::deserialize(ticket_str).context("failed to parse cluster ticket")?;

        if ticket.bootstrap.is_empty() {
            anyhow::bail!("ticket contains no bootstrap peers");
        }

        // Create Iroh endpoint
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .alpns(vec![CLIENT_ALPN.to_vec()])
            .bind()
            .await
            .context("failed to create Iroh endpoint")?;

        debug!(
            endpoint_id = %endpoint.id(),
            bootstrap_peers = ticket.bootstrap.len(),
            "CLI client connected"
        );

        Ok(Self {
            endpoint,
            ticket,
            rpc_timeout,
            token,
        })
    }

    /// Send an RPC request and return the response.
    ///
    /// Includes automatic retry logic with peer rotation for transient failures.
    /// When a peer returns NOT_INITIALIZED or SERVICE_UNAVAILABLE errors,
    /// the client automatically tries other bootstrap peers in the ticket.
    ///
    /// # Tiger Style
    ///
    /// - Bounded retries per peer (MAX_RETRIES_PER_PEER)
    /// - Bounded total retries (all peers * retries per peer)
    /// - Fail-fast on permanent errors (auth failures, invalid requests)
    /// - Transparent failover improves CLI reliability in multi-node clusters
    pub async fn send(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        let peers: Vec<EndpointId> = self.ticket.bootstrap.iter().copied().collect();
        let mut last_error = None;

        // Try each bootstrap peer
        for (peer_idx, peer_id) in peers.iter().enumerate() {
            // Retry loop for this specific peer
            for attempt in 0..MAX_RETRIES_PER_PEER {
                if attempt > 0 || peer_idx > 0 {
                    debug!(peer_idx, attempt, peer_id = %peer_id, "retrying RPC request");
                    tokio::time::sleep(RETRY_DELAY).await;
                }

                match self.send_to_peer(*peer_id, request.clone()).await {
                    Ok(response) => {
                        // Check if server returned a retryable error
                        if let ClientRpcResponse::Error(ref e) = response {
                            if Self::is_retryable_error(&e.code) {
                                debug!(
                                    peer_id = %peer_id,
                                    error_code = %e.code,
                                    "received retryable error, trying next peer"
                                );
                                last_error = Some(anyhow::anyhow!("{}: {}", e.code, e.message));
                                // Don't retry this peer, move to next one immediately
                                break;
                            }
                        }
                        return Ok(response);
                    }
                    Err(e) => {
                        warn!(peer_idx, attempt, peer_id = %peer_id, error = %e, "RPC request failed");
                        last_error = Some(e);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!(
                "RPC failed after trying {} peer(s) with {} retries each",
                peers.len(),
                MAX_RETRIES_PER_PEER
            )
        }))
    }

    /// Check if an error code indicates we should try another peer.
    ///
    /// Some errors are specific to a node's state and may succeed on another node:
    /// - NOT_INITIALIZED: Node hasn't joined the cluster yet
    /// - SERVICE_UNAVAILABLE: Rate limiter or other service unavailable
    /// - NOT_LEADER: Node isn't the Raft leader (though operations may still work)
    fn is_retryable_error(code: &str) -> bool {
        matches!(code, "NOT_INITIALIZED" | "SERVICE_UNAVAILABLE")
    }

    /// Send an RPC request to a specific peer.
    async fn send_to_peer(&self, peer_id: EndpointId, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        // Build endpoint address
        let target_addr = EndpointAddr::new(peer_id);

        // Connect to the peer
        let connection = timeout(self.rpc_timeout, async {
            self.endpoint.connect(target_addr, CLIENT_ALPN).await.context("failed to connect to peer")
        })
        .await
        .context("connection timeout")??;

        // Open bidirectional stream
        let (mut send, mut recv) = connection.open_bi().await.context("failed to open stream")?;

        // Serialize request (legacy format - no AuthenticatedRequest wrapper)
        // The server supports both formats and job operations work with legacy format
        // TODO: Convert to AuthenticatedRequest once all types are unified
        let request_bytes = postcard::to_stdvec(&request).context("failed to serialize request")?;

        send.write_all(&request_bytes).await.context("failed to send request")?;

        send.finish().context("failed to finish send stream")?;

        // Read response with timeout
        let response_bytes = timeout(self.rpc_timeout, async {
            recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE).await.context("failed to read response")
        })
        .await
        .context("response timeout")??;

        // Deserialize response
        let response: ClientRpcResponse =
            postcard::from_bytes(&response_bytes).context("failed to deserialize response")?;

        // Close connection gracefully
        connection.close(VarInt::from_u32(0), b"done");

        Ok(response)
    }

    /// Get the cluster ID from the ticket.
    #[allow(dead_code)]
    pub fn cluster_id(&self) -> &str {
        &self.ticket.cluster_id
    }

    /// Shutdown the client and close the endpoint.
    #[allow(dead_code)]
    pub async fn shutdown(self) {
        self.endpoint.close().await;
    }

    /// Send an RPC request to a specific endpoint address.
    ///
    /// Used for cross-node verification where we need to query
    /// multiple nodes directly.
    pub async fn send_to(&self, addr: &EndpointAddr, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        // Connect to the specific peer
        let connection = timeout(self.rpc_timeout, async {
            self.endpoint.connect(addr.clone(), CLIENT_ALPN).await.context("failed to connect to peer")
        })
        .await
        .context("connection timeout")??;

        // Open bidirectional stream
        let (mut send, mut recv) = connection.open_bi().await.context("failed to open stream")?;

        // Serialize request (legacy format - no AuthenticatedRequest wrapper)
        // The server supports both formats and job operations work with legacy format
        // TODO: Convert to AuthenticatedRequest once all types are unified
        let request_bytes = postcard::to_stdvec(&request).context("failed to serialize request")?;

        send.write_all(&request_bytes).await.context("failed to send request")?;

        send.finish().context("failed to finish send stream")?;

        // Read response with timeout
        let response_bytes = timeout(self.rpc_timeout, async {
            recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE).await.context("failed to read response")
        })
        .await
        .context("response timeout")??;

        // Deserialize response
        let response: ClientRpcResponse =
            postcard::from_bytes(&response_bytes).context("failed to deserialize response")?;

        // Close connection gracefully
        connection.close(VarInt::from_u32(0), b"done");

        Ok(response)
    }
}
