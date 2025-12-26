//! Aspen client for CLI operations.
//!
//! Handles Iroh P2P connections and RPC communication with Aspen nodes.
//! Adapted from the TUI client implementation.

use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen::auth::CapabilityToken;
use aspen::client_rpc::AuthenticatedRequest;
use aspen::client_rpc::ClientRpcRequest;
use aspen::client_rpc::ClientRpcResponse;
use aspen::client_rpc::MAX_CLIENT_MESSAGE_SIZE;
use aspen::cluster::ticket::AspenClusterTicket;
use aspen::protocol_handlers::CLIENT_ALPN;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::endpoint::VarInt;
use tokio::time::timeout;
use tracing::debug;
use tracing::warn;

/// Maximum number of retry attempts for RPC calls.
const MAX_RETRIES: u32 = 3;

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
    /// Includes automatic retry logic for transient failures.
    pub async fn send(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                debug!(attempt, "retrying RPC request");
                tokio::time::sleep(RETRY_DELAY).await;
            }

            match self.send_once(request.clone()).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    warn!(attempt, error = %e, "RPC request failed");
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("RPC failed after {} retries", MAX_RETRIES)))
    }

    /// Send a single RPC request without retry.
    async fn send_once(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        // Get a bootstrap peer to connect to
        let peer_id =
            *self.ticket.bootstrap.iter().next().ok_or_else(|| anyhow::anyhow!("no bootstrap peers in ticket"))?;

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

        // Wrap request with authentication if token is present
        let authenticated_request = if let Some(ref token) = self.token {
            AuthenticatedRequest::new(request, token.clone())
        } else {
            AuthenticatedRequest::unauthenticated(request)
        };

        // Serialize and send request
        let request_bytes = postcard::to_stdvec(&authenticated_request).context("failed to serialize request")?;

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
}
