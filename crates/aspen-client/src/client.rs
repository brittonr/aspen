//! Aspen client for RPC operations over Iroh P2P.
//!
//! Handles Iroh P2P connections and RPC communication with Aspen nodes.

use std::collections::BTreeSet;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use base64::prelude::*;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::EndpointId;
use iroh::endpoint::VarInt;
use iroh_gossip::proto::TopicId;
use serde::Deserialize;
use serde::Serialize;
use tokio::time::timeout;
use tracing::debug;
use tracing::warn;

use crate::constants::CLIENT_ALPN;
use crate::constants::MAX_CLIENT_MESSAGE_SIZE;
use crate::constants::MAX_RETRIES;
use crate::constants::RETRY_DELAY_MS;

/// Cluster ticket for gossip-based node discovery.
/// This is a local copy to keep aspen-client independent of aspen-cluster.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct AspenClusterTicket {
    pub topic_id: TopicId,
    pub bootstrap: BTreeSet<EndpointId>,
    pub cluster_id: String,
}

impl AspenClusterTicket {
    /// Create a new ticket with a topic ID and cluster identifier.
    pub fn new(topic_id: TopicId, cluster_id: String) -> Self {
        Self {
            topic_id,
            bootstrap: BTreeSet::new(),
            cluster_id,
        }
    }

    /// Serialize the ticket to a string.
    pub fn serialize(&self) -> String {
        let bytes = postcard::to_stdvec(self).expect("AspenClusterTicket postcard serialization failed");
        format!("aspen:{}", base64::prelude::BASE64_STANDARD.encode(&bytes))
    }

    /// Deserialize a ticket from a string.
    pub fn deserialize(input: &str) -> anyhow::Result<Self> {
        let stripped = input.strip_prefix("aspen:").ok_or_else(|| anyhow::anyhow!("Invalid ticket format"))?;
        let bytes = base64::prelude::BASE64_STANDARD.decode(stripped)?;
        let ticket = postcard::from_bytes(&bytes)?;
        Ok(ticket)
    }
}

/// Opaque authentication token for client requests.
///
/// Clients obtain tokens from the cluster administrator and pass them
/// opaquely to authenticate their requests. The token contents are
/// validated server-side.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken(Vec<u8>);

impl AuthToken {
    /// Create a new auth token from raw bytes.
    ///
    /// Typically obtained by deserializing a base64-encoded token string
    /// provided by the cluster administrator.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Create a token from a base64-encoded string.
    pub fn from_base64(s: &str) -> Result<Self> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD.decode(s).context("invalid base64 token")?;
        Ok(Self(bytes))
    }

    /// Get the raw bytes of the token.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Authenticated request wrapper for Client RPC.
///
/// Wraps a request with an optional authentication token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedRequest {
    /// The actual RPC request.
    pub request: ClientRpcRequest,
    /// Optional authentication token bytes.
    pub token: Option<Vec<u8>>,
}

impl AuthenticatedRequest {
    /// Create an authenticated request with a token.
    pub fn new(request: ClientRpcRequest, token: AuthToken) -> Self {
        Self {
            request,
            token: Some(token.0),
        }
    }

    /// Create an unauthenticated request.
    pub fn unauthenticated(request: ClientRpcRequest) -> Self {
        Self { request, token: None }
    }
}

/// Aspen client for sending RPC requests to cluster nodes.
///
/// Connects to an Aspen cluster using an Iroh P2P endpoint and sends
/// RPC requests using the Client ALPN protocol.
///
/// # Example
///
/// ```rust,ignore
/// use aspen_client::{AspenClient, ClientRpcRequest};
/// use std::time::Duration;
///
/// let client = AspenClient::connect(
///     "aspen...",  // ticket string
///     Duration::from_secs(5),
///     None,  // no auth token
/// ).await?;
///
/// let response = client.send(ClientRpcRequest::Ping).await?;
/// ```
pub struct AspenClient {
    endpoint: Endpoint,
    ticket: AspenClusterTicket,
    rpc_timeout: Duration,
    token: Option<AuthToken>,
}

impl AspenClient {
    /// Connect to an Aspen cluster using a ticket.
    ///
    /// # Arguments
    ///
    /// * `ticket_str` - Base32-encoded cluster ticket (aspen...)
    /// * `rpc_timeout` - RPC timeout duration
    /// * `token` - Optional auth token for authentication
    pub async fn connect(ticket_str: &str, rpc_timeout: Duration, token: Option<AuthToken>) -> Result<Self> {
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
            "Aspen client connected"
        );

        Ok(Self {
            endpoint,
            ticket,
            rpc_timeout,
            token,
        })
    }

    /// Connect to a cluster using a parsed ticket.
    ///
    /// Use this when you already have a parsed `AspenClusterTicket`.
    pub async fn connect_with_ticket(
        ticket: AspenClusterTicket,
        rpc_timeout: Duration,
        token: Option<AuthToken>,
    ) -> Result<Self> {
        if ticket.bootstrap.is_empty() {
            anyhow::bail!("ticket contains no bootstrap peers");
        }

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
            "Aspen client connected"
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
        let retry_delay = Duration::from_millis(RETRY_DELAY_MS);

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                debug!(attempt, "retrying RPC request");
                tokio::time::sleep(retry_delay).await;
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

        self.send_to_addr(&target_addr, request).await
    }

    /// Send an RPC request to a specific endpoint address.
    ///
    /// Used for cross-node verification where we need to query
    /// multiple nodes directly.
    pub async fn send_to(&self, addr: &EndpointAddr, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        self.send_to_addr(addr, request).await
    }

    /// Internal method to send to a specific address.
    async fn send_to_addr(&self, addr: &EndpointAddr, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        // Connect to the peer
        let connection = timeout(self.rpc_timeout, async {
            self.endpoint.connect(addr.clone(), CLIENT_ALPN).await.context("failed to connect to peer")
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
    pub fn cluster_id(&self) -> &str {
        &self.ticket.cluster_id
    }

    /// Get the cluster ticket.
    pub fn ticket(&self) -> &AspenClusterTicket {
        &self.ticket
    }

    /// Get the endpoint ID of this client.
    pub fn endpoint_id(&self) -> iroh::EndpointId {
        self.endpoint.id()
    }

    /// Shutdown the client and close the endpoint.
    pub async fn shutdown(self) {
        self.endpoint.close().await;
    }
}
