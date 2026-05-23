//! Aspen client for RPC operations over Iroh P2P.
//!
//! Handles Iroh P2P connections and RPC communication with Aspen nodes.

use std::collections::BTreeSet;
use std::future::Future;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_client_api::AuthenticatedRequest;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
// Re-export ticket types for convenience
pub use aspen_ticket::AspenClusterTicket;
pub use aspen_ticket::BootstrapPeer;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::RelayMode;
use iroh::TransportAddr;
use iroh::address_lookup::memory::MemoryLookup;
use iroh::endpoint::VarInt;
use serde::Deserialize;
use serde::Serialize;
use tokio::time::Instant;
use tokio::time::timeout;
use tracing::debug;
use tracing::warn;

use crate::constants::CLIENT_ALPN;
use crate::constants::MAX_CLIENT_MESSAGE_SIZE;
use crate::constants::MAX_RETRIES;
use crate::constants::RETRY_DELAY_MS;

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

    /// Convert to a CapabilityToken.
    ///
    /// Deserializes the raw bytes to a CapabilityToken for use in
    /// AuthenticatedRequest. Returns None if deserialization fails.
    pub fn to_capability_token(&self) -> Option<aspen_auth::CapabilityToken> {
        postcard::from_bytes(&self.0).ok()
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
#[derive(Clone)]
pub struct AspenClient {
    endpoint: Endpoint,
    ticket: AspenClusterTicket,
    rpc_timeout: Duration,
    token: Option<AuthToken>,
    prefer_loopback_direct: bool,
}

fn loopback_only_addr(addr: &EndpointAddr) -> Option<EndpointAddr> {
    let ipv4_loopback_addrs = addr
        .addrs
        .iter()
        .filter_map(|transport_addr| match transport_addr {
            TransportAddr::Ip(socket_addr) if socket_addr.ip().is_ipv4() && socket_addr.ip().is_loopback() => {
                Some(TransportAddr::Ip(*socket_addr))
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();

    if !ipv4_loopback_addrs.is_empty() {
        return Some(EndpointAddr {
            id: addr.id,
            addrs: ipv4_loopback_addrs,
        });
    }

    let loopback_addrs = addr
        .addrs
        .iter()
        .filter_map(|transport_addr| match transport_addr {
            TransportAddr::Ip(socket_addr) if socket_addr.ip().is_loopback() => Some(TransportAddr::Ip(*socket_addr)),
            _ => None,
        })
        .collect::<BTreeSet<_>>();

    if loopback_addrs.is_empty() {
        None
    } else {
        Some(EndpointAddr {
            id: addr.id,
            addrs: loopback_addrs,
        })
    }
}

fn remaining_timeout(deadline: Instant, timeout_context: &'static str) -> Result<Duration> {
    match deadline.checked_duration_since(Instant::now()) {
        Some(remaining) if !remaining.is_zero() => Ok(remaining),
        _ => Err(anyhow::anyhow!(timeout_context)),
    }
}

fn missing_direct_route_peer_ids(ticket: &AspenClusterTicket) -> Vec<String> {
    ticket
        .bootstrap
        .iter()
        .filter(|peer| peer.direct_addrs.is_empty())
        .map(|peer| peer.endpoint_id.to_string())
        .collect()
}

fn validate_direct_only_bootstrap_routes(ticket: &AspenClusterTicket) -> Result<()> {
    let missing = missing_direct_route_peer_ids(ticket);
    if missing.is_empty() {
        return Ok(());
    }

    anyhow::bail!(
        "direct-only Iroh client has no direct address for bootstrap peer(s) {}; ticket/bootstrap address was not registered for later RPCs",
        missing.join(",")
    )
}

fn endpoint_addrs_for_direct_lookup(ticket: &AspenClusterTicket) -> Result<Vec<EndpointAddr>> {
    let addrs = ticket.endpoint_addrs().context("cluster ticket contains invalid bootstrap endpoint address")?;
    let scoped = addrs.iter().filter_map(loopback_only_addr).collect::<Vec<_>>();
    if scoped.is_empty() { Ok(addrs) } else { Ok(scoped) }
}

fn memory_lookup_from_ticket(ticket: &AspenClusterTicket) -> Result<MemoryLookup> {
    Ok(MemoryLookup::from_endpoint_info(endpoint_addrs_for_direct_lookup(ticket)?))
}

async fn run_stage_with_deadline<F, T, E>(
    deadline: Instant,
    future: F,
    timeout_context: &'static str,
    error_context: &'static str,
) -> Result<T>
where
    F: Future<Output = std::result::Result<T, E>>,
    E: std::error::Error + Send + Sync + 'static,
{
    timeout(remaining_timeout(deadline, timeout_context)?, future)
        .await
        .context(timeout_context)?
        .map_err(anyhow::Error::new)
        .context(error_context)
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
        Self::connect_with_relay_setting(ticket_str, rpc_timeout, token, false).await
    }

    /// Connect to an Aspen cluster using a ticket with relay services disabled.
    ///
    /// Use this for deterministic same-host/offline orchestration when tickets
    /// carry direct socket addresses and external DNS/relay lookup should not be
    /// part of the success path.
    pub async fn connect_direct(ticket_str: &str, rpc_timeout: Duration, token: Option<AuthToken>) -> Result<Self> {
        Self::connect_with_relay_setting(ticket_str, rpc_timeout, token, true).await
    }

    async fn connect_with_relay_setting(
        ticket_str: &str,
        rpc_timeout: Duration,
        token: Option<AuthToken>,
        relay_disabled: bool,
    ) -> Result<Self> {
        // Parse the ticket
        let ticket = AspenClusterTicket::deserialize(ticket_str).context("failed to parse cluster ticket")?;

        if ticket.bootstrap.is_empty() {
            anyhow::bail!("ticket contains no bootstrap peers");
        }

        let direct_only = relay_disabled || std::env::var("ASPEN_RELAY_DISABLED").unwrap_or_default() == "1";
        if direct_only {
            validate_direct_only_bootstrap_routes(&ticket)?;
        }
        let endpoint = Self::create_client_endpoint(direct_only, Some(&ticket)).await?;

        debug!(
            endpoint_id = %endpoint.id(),
            bootstrap_peers = ticket.bootstrap.len(),
            relay_disabled = direct_only,
            "Aspen client connected"
        );

        Ok(Self {
            endpoint,
            ticket,
            rpc_timeout,
            token,
            prefer_loopback_direct: direct_only,
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

        let direct_only = std::env::var("ASPEN_RELAY_DISABLED").unwrap_or_default() == "1";
        if direct_only {
            validate_direct_only_bootstrap_routes(&ticket)?;
        }
        let endpoint = Self::create_client_endpoint(direct_only, Some(&ticket)).await?;

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
            prefer_loopback_direct: direct_only,
        })
    }

    /// Create an iroh Endpoint for the client.
    ///
    /// Respects `ASPEN_RELAY_DISABLED=1` for local/offline testing.
    async fn create_client_endpoint(relay_disabled: bool, ticket: Option<&AspenClusterTicket>) -> Result<Endpoint> {
        let secret_key = iroh::SecretKey::generate();
        let direct_only = relay_disabled || std::env::var("ASPEN_RELAY_DISABLED").unwrap_or_default() == "1";
        let mut builder = if direct_only {
            Endpoint::builder(iroh::endpoint::presets::Minimal)
        } else {
            Endpoint::builder(iroh::endpoint::presets::N0)
        }
        .secret_key(secret_key)
        .alpns(vec![CLIENT_ALPN.to_vec()]);

        if direct_only {
            builder = builder.relay_mode(RelayMode::Disabled).clear_address_lookup();
            if let Some(ticket) = ticket {
                builder = builder.address_lookup(memory_lookup_from_ticket(ticket)?);
            }
        }

        builder.bind().await.context("failed to create Iroh endpoint")
    }

    /// Create a client with an existing Iroh endpoint.
    ///
    /// Use this when running inside an existing Iroh node (e.g., worker-only mode)
    /// where you already have an endpoint and don't want to create a new one.
    ///
    /// Existing endpoints may be inside a VM or another network namespace. Do not
    /// infer same-host loopback preference from process environment here: a
    /// same-process worker can still need bridge/LAN addresses from the ticket.
    pub fn with_endpoint(
        endpoint: Endpoint,
        ticket: AspenClusterTicket,
        rpc_timeout: Duration,
        token: Option<AuthToken>,
    ) -> Self {
        Self {
            endpoint,
            ticket,
            rpc_timeout,
            token,
            prefer_loopback_direct: false,
        }
    }

    /// Send an RPC request and return the response.
    ///
    /// Includes automatic retry logic for transient failures.
    pub async fn send(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        self.send_with_response_limit(request, MAX_CLIENT_MESSAGE_SIZE).await
    }

    /// Send an RPC request with an explicit response-size bound.
    ///
    /// Keep ordinary control-plane RPCs on `send`; this is for intentionally
    /// bounded bulk responses such as VM workspace source archives.
    pub async fn send_with_response_limit(
        &self,
        request: ClientRpcRequest,
        max_response_size: usize,
    ) -> Result<ClientRpcResponse> {
        let (response, _trailing_bytes) = self.send_with_trailing_response_limit(request, max_response_size).await?;
        Ok(response)
    }

    /// Send an RPC request and preserve any bytes following the first postcard frame.
    ///
    /// The blob service uses this for large `GetBlob` responses: the first frame is
    /// a `GetBlobResult` header and the remaining bytes are the raw streamed blob.
    pub async fn send_with_trailing_response_limit(
        &self,
        request: ClientRpcRequest,
        max_response_size: usize,
    ) -> Result<(ClientRpcResponse, Vec<u8>)> {
        let mut last_error = None;
        let retry_delay = Duration::from_millis(RETRY_DELAY_MS);
        let mut peer_index = 0usize;

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                debug!(attempt, "retrying RPC request");
                tokio::time::sleep(retry_delay).await;
            }

            match self.send_to_peer_with_trailing_response_limit(peer_index, request.clone(), max_response_size).await {
                Ok((response, trailing_bytes)) => {
                    // Check for NOT_LEADER application-level error — rotate to next peer and retry
                    if let ClientRpcResponse::Error(ref e) = response
                        && e.code == "NOT_LEADER"
                    {
                        warn!(attempt, code = %e.code, "server is not leader, rotating to next peer");
                        peer_index = (peer_index + 1) % self.ticket.bootstrap.len().max(1);
                        last_error = Some(anyhow::anyhow!("NOT_LEADER: {}", e.message));
                        continue;
                    }
                    return Ok((response, trailing_bytes));
                }
                Err(e) => {
                    warn!(attempt, error = %e, "RPC request failed");
                    peer_index = (peer_index + 1) % self.ticket.bootstrap.len().max(1);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("RPC failed after {} retries", MAX_RETRIES)))
    }

    /// Send a single RPC request to a specific bootstrap peer.
    async fn send_to_peer(&self, peer_index: usize, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        self.send_to_peer_with_response_limit(peer_index, request, MAX_CLIENT_MESSAGE_SIZE).await
    }

    async fn send_to_peer_with_response_limit(
        &self,
        peer_index: usize,
        request: ClientRpcRequest,
        max_response_size: usize,
    ) -> Result<ClientRpcResponse> {
        let (response, _trailing_bytes) =
            self.send_to_peer_with_trailing_response_limit(peer_index, request, max_response_size).await?;
        Ok(response)
    }

    async fn send_to_peer_with_trailing_response_limit(
        &self,
        peer_index: usize,
        request: ClientRpcRequest,
        max_response_size: usize,
    ) -> Result<(ClientRpcResponse, Vec<u8>)> {
        let peer = self
            .ticket
            .bootstrap
            .get(peer_index)
            .or_else(|| self.ticket.bootstrap.first())
            .ok_or_else(|| anyhow::anyhow!("no bootstrap peers in ticket"))?;
        let target_addr = peer.to_endpoint_addr().context("cluster ticket contains invalid bootstrap endpoint id")?;
        self.send_to_addr_with_trailing_response_limit(&target_addr, request, max_response_size).await
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
        self.send_to_addr_with_response_limit(addr, request, MAX_CLIENT_MESSAGE_SIZE).await
    }

    async fn send_to_addr_with_response_limit(
        &self,
        addr: &EndpointAddr,
        request: ClientRpcRequest,
        max_response_size: usize,
    ) -> Result<ClientRpcResponse> {
        let (response, _trailing_bytes) =
            self.send_to_addr_with_trailing_response_limit(addr, request, max_response_size).await?;
        Ok(response)
    }

    async fn send_to_addr_with_trailing_response_limit(
        &self,
        addr: &EndpointAddr,
        request: ClientRpcRequest,
        max_response_size: usize,
    ) -> Result<(ClientRpcResponse, Vec<u8>)> {
        let mut addr_to_connect = addr.clone();
        if self.prefer_loopback_direct
            && let Some(loopback_addr) = loopback_only_addr(addr)
        {
            addr_to_connect = loopback_addr;
        }

        // Connect to the peer (connect timeout)
        let connection = timeout(self.rpc_timeout, async {
            self.endpoint.connect(addr_to_connect, CLIENT_ALPN).await.context("failed to connect to peer")
        })
        .await
        .context("connection timeout")??;

        // Wrap request with authentication if token is present
        let authenticated_request = match self.token.as_ref().and_then(|t| t.to_capability_token()) {
            Some(cap_token) => AuthenticatedRequest::new(request, cap_token),
            None => AuthenticatedRequest::unauthenticated(request),
        };

        // Serialize request before entering the timed exchange
        let request_bytes = postcard::to_stdvec(&authenticated_request).context("failed to serialize request")?;

        // Bound the full post-connect exchange with one overall budget while
        // preserving stage-specific timeout errors.
        let deadline = Instant::now() + self.rpc_timeout;
        let (mut send, mut recv) =
            run_stage_with_deadline(deadline, connection.open_bi(), "stream open timeout", "failed to open stream")
                .await?;
        run_stage_with_deadline(
            deadline,
            send.write_all(&request_bytes),
            "request write timeout",
            "failed to send request",
        )
        .await?;
        send.finish().context("failed to finish send stream")?;
        let response_bytes = run_stage_with_deadline(
            deadline,
            recv.read_to_end(max_response_size),
            "response timeout",
            "failed to read response",
        )
        .await?;

        // Deserialize response and preserve any raw bytes appended after the postcard frame.
        let (response, trailing_bytes): (ClientRpcResponse, &[u8]) =
            postcard::take_from_bytes(&response_bytes).context("failed to deserialize response")?;

        // Close connection gracefully
        connection.close(VarInt::from_u32(0), b"done");

        Ok((response, trailing_bytes.to_vec()))
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

    /// Get a reference to the underlying iroh endpoint.
    pub fn endpoint(&self) -> &iroh::Endpoint {
        &self.endpoint
    }

    /// Shutdown the client and close the endpoint.
    pub async fn shutdown(self) {
        self.endpoint.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loopback_only_addr_prefers_local_direct_addrs() {
        let endpoint_id = iroh::SecretKey::from([1; 32]).public();
        let addr = EndpointAddr {
            id: endpoint_id,
            addrs: [
                TransportAddr::Ip("192.168.1.252:12345".parse().unwrap()),
                TransportAddr::Ip("127.0.0.1:12345".parse().unwrap()),
                TransportAddr::Ip("[::1]:12345".parse().unwrap()),
            ]
            .into_iter()
            .collect(),
        };

        let loopback = loopback_only_addr(&addr).expect("loopback addrs should be extracted");
        assert_eq!(loopback.id, endpoint_id);
        assert_eq!(loopback.addrs.len(), 1);
        assert!(loopback.addrs.contains(&TransportAddr::Ip("127.0.0.1:12345".parse().unwrap())));
        assert!(!loopback.addrs.contains(&TransportAddr::Ip("[::1]:12345".parse().unwrap())));
    }

    #[test]
    fn test_loopback_only_addr_returns_none_without_loopback() {
        let endpoint_id = iroh::SecretKey::from([2; 32]).public();
        let addr = EndpointAddr {
            id: endpoint_id,
            addrs: [TransportAddr::Ip("192.168.1.252:12345".parse().unwrap())].into_iter().collect(),
        };

        assert!(loopback_only_addr(&addr).is_none());
    }

    #[test]
    fn direct_only_preflight_accepts_ticket_direct_addrs() {
        let topic_id = iroh_gossip::proto::TopicId::from_bytes([42; 32]);
        let endpoint_id = iroh::SecretKey::from([3; 32]).public();
        let addr = EndpointAddr {
            id: endpoint_id,
            addrs: [TransportAddr::Ip("127.0.0.1:12345".parse().unwrap())].into_iter().collect(),
        };
        let ticket = AspenClusterTicket::with_bootstrap_addr(topic_id, "test".to_string(), &addr);

        validate_direct_only_bootstrap_routes(&ticket).expect("direct-only ticket should carry socket addresses");
        let lookup = memory_lookup_from_ticket(&ticket).expect("ticket addrs should seed memory lookup");
        assert!(lookup.get_endpoint_info(endpoint_id).is_some());
    }

    #[test]
    fn direct_lookup_scopes_same_host_ticket_to_ipv4_loopback() {
        let topic_id = iroh_gossip::proto::TopicId::from_bytes([44; 32]);
        let endpoint_id = iroh::SecretKey::from([5; 32]).public();
        let addr = EndpointAddr {
            id: endpoint_id,
            addrs: [
                TransportAddr::Ip("10.200.0.1:39946".parse().unwrap()),
                TransportAddr::Ip("127.0.0.1:39946".parse().unwrap()),
                TransportAddr::Ip("[::1]:52328".parse().unwrap()),
            ]
            .into_iter()
            .collect(),
        };
        let ticket = AspenClusterTicket::with_bootstrap_addr(topic_id, "test".to_string(), &addr);

        let scoped = endpoint_addrs_for_direct_lookup(&ticket).expect("ticket addrs should be valid");
        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].id, endpoint_id);
        assert_eq!(scoped[0].addrs.len(), 1);
        assert!(scoped[0].addrs.contains(&TransportAddr::Ip("127.0.0.1:39946".parse().unwrap())));
    }

    #[test]
    fn direct_lookup_keeps_non_loopback_ticket_for_vm_namespace() {
        let topic_id = iroh_gossip::proto::TopicId::from_bytes([45; 32]);
        let endpoint_id = iroh::SecretKey::from([6; 32]).public();
        let addr = EndpointAddr {
            id: endpoint_id,
            addrs: [TransportAddr::Ip("10.200.0.1:39946".parse().unwrap())].into_iter().collect(),
        };
        let ticket = AspenClusterTicket::with_bootstrap_addr(topic_id, "test".to_string(), &addr);

        let scoped = endpoint_addrs_for_direct_lookup(&ticket).expect("ticket addrs should be valid");
        assert_eq!(scoped.len(), 1);
        assert!(scoped[0].addrs.contains(&TransportAddr::Ip("10.200.0.1:39946".parse().unwrap())));
    }

    #[test]
    fn direct_only_preflight_rejects_endpoint_id_only_ticket() {
        let topic_id = iroh_gossip::proto::TopicId::from_bytes([43; 32]);
        let endpoint_id = iroh::SecretKey::from([4; 32]).public();
        let ticket = AspenClusterTicket::with_bootstrap(topic_id, "test".to_string(), endpoint_id);

        let error =
            validate_direct_only_bootstrap_routes(&ticket).expect_err("direct-only must fail without direct addrs");
        let message = error.to_string();
        assert!(message.contains("direct-only Iroh client has no direct address"));
        assert!(message.contains("ticket/bootstrap address was not registered"));
    }

    #[tokio::test]
    async fn test_deadline_helper_reports_stream_open_timeout() {
        let deadline = Instant::now() + Duration::from_millis(10);
        let result = run_stage_with_deadline(
            deadline,
            std::future::pending::<std::result::Result<(), std::io::Error>>(),
            "stream open timeout",
            "failed to open stream",
        )
        .await;

        let error = result.expect_err("pending future must time out");
        assert!(error.to_string().contains("stream open timeout"));
    }
}
