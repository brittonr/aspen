//! Aspen client for CLI operations.
//!
//! Handles Iroh P2P connections and RPC communication with Aspen nodes.
//! Adapted from the TUI client implementation.
//!
//! ## Connection Caching
//!
//! The client caches QUIC connections to avoid the overhead of creating new connections
//! for each RPC call. This is critical for polling scenarios like `ci status --follow`
//! where many requests are sent in quick succession.
//!
//! ## Retry Strategy
//!
//! The client implements intelligent retry with exponential backoff for NOT_INITIALIZED
//! errors. This handles the race condition where nodes have received Raft membership
//! via replication but haven't yet set their initialized flag.
//!
//! - NOT_INITIALIZED: Exponential backoff (100ms -> 200ms -> 400ms -> 800ms -> 1600ms)
//! - SERVICE_UNAVAILABLE: Immediate peer rotation (transient per-node issue)
//! - Network errors: Fixed delay retry, then peer rotation
//!
//! ## Tiger Style
//!
//! - Bounded retries per peer (MAX_RETRIES_PER_PEER)
//! - Bounded backoff (MAX_BACKOFF_DELAY)
//! - Fail-fast on permanent errors

use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_auth::CapabilityToken;
use aspen_client_api::AuthenticatedRequest;
use aspen_client_api::CLIENT_ALPN;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::MAX_CLIENT_MESSAGE_SIZE;
use aspen_cluster::ticket::parse_ticket_to_addrs;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::EndpointId;
use iroh::endpoint::Connection;
use iroh::endpoint::VarInt;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tokio::time::timeout;
use tracing::debug;
use tracing::warn;

/// Maximum number of retry attempts for RPC calls per peer.
const MAX_RETRIES_PER_PEER: u32 = 5;

/// Initial delay for exponential backoff (100ms).
const INITIAL_BACKOFF_DELAY: Duration = Duration::from_millis(100);

/// Maximum delay for exponential backoff (10 seconds).
const MAX_BACKOFF_DELAY: Duration = Duration::from_secs(10);

/// Fixed delay for network errors (not NOT_INITIALIZED).
const NETWORK_RETRY_DELAY: Duration = Duration::from_millis(500);

/// Maximum number of cached connections per peer.
const MAX_CACHED_CONNECTIONS: usize = 8;

/// Maximum response size for CLI blob fetches (256 MiB).
const MAX_GET_BLOB_RESPONSE_SIZE: usize = 256 * 1024 * 1024;

/// Dedicated read budget for large blob fetch responses.
const GET_BLOB_READ_TIMEOUT: Duration = Duration::from_secs(300);

/// Aspen client for sending RPC requests to cluster nodes.
pub struct AspenClient {
    endpoint: Endpoint,
    /// Bootstrap peer addresses with direct addresses (from V2 tickets) or just IDs (from V1).
    bootstrap_addrs: Vec<EndpointAddr>,
    /// Cluster identifier from the ticket.
    cluster_id: String,
    rpc_timeout: Duration,
    token: Option<CapabilityToken>,
    /// Cached connections per peer for connection reuse.
    /// Using Mutex for interior mutability in async context.
    connections: Mutex<HashMap<EndpointId, Vec<Connection>>>,
}

fn remaining_timeout(deadline: Instant, timeout_context: &'static str) -> Result<Duration> {
    match deadline.checked_duration_since(Instant::now()) {
        Some(remaining) if !remaining.is_zero() => Ok(remaining),
        _ => Err(anyhow::anyhow!(timeout_context)),
    }
}

async fn run_timed_stage<F, T, E>(
    stage_timeout: Duration,
    future: F,
    timeout_context: &'static str,
    error_context: &'static str,
) -> Result<T>
where
    F: Future<Output = std::result::Result<T, E>>,
    E: std::error::Error + Send + Sync + 'static,
{
    timeout(stage_timeout, future)
        .await
        .context(timeout_context)?
        .map_err(anyhow::Error::new)
        .context(error_context)
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
    run_timed_stage(remaining_timeout(deadline, timeout_context)?, future, timeout_context, error_context).await
}

impl AspenClient {
    /// Connect to an Aspen cluster using a ticket.
    ///
    /// Supports both V1 tickets (aspen...) and V2 tickets (aspenv2...).
    /// V2 tickets include direct socket addresses for relay-less connectivity.
    ///
    /// # Arguments
    ///
    /// * `ticket_str` - Base32-encoded cluster ticket (aspen... or aspenv2...)
    /// * `rpc_timeout` - RPC timeout duration
    /// * `token` - Optional capability token for authentication
    pub async fn connect(ticket_str: &str, rpc_timeout: Duration, token: Option<CapabilityToken>) -> Result<Self> {
        // Parse the ticket (supports both V1 and V2 formats)
        let (_topic_id, cluster_id, bootstrap_addrs) =
            parse_ticket_to_addrs(ticket_str).context("failed to parse cluster ticket")?;

        if bootstrap_addrs.is_empty() {
            anyhow::bail!("ticket contains no bootstrap peers");
        }

        // Log the addresses we'll try to connect to
        for addr in &bootstrap_addrs {
            let direct_addrs: Vec<_> = addr
                .addrs
                .iter()
                .filter_map(|a| match a {
                    iroh::TransportAddr::Ip(socket_addr) => Some(socket_addr),
                    _ => None,
                })
                .collect();
            debug!(
                endpoint_id = %addr.id,
                direct_addrs = ?direct_addrs,
                "bootstrap peer from ticket"
            );
        }

        // Create Iroh endpoint
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        let endpoint = Endpoint::builder(iroh::endpoint::presets::N0)
            .secret_key(secret_key)
            .alpns(vec![CLIENT_ALPN.to_vec()])
            .bind()
            .await
            .context("failed to create Iroh endpoint")?;

        debug!(
            endpoint_id = %endpoint.id(),
            bootstrap_peers = bootstrap_addrs.len(),
            "CLI client connected"
        );

        Ok(Self {
            endpoint,
            bootstrap_addrs,
            cluster_id,
            rpc_timeout,
            token,
            connections: Mutex::new(HashMap::new()),
        })
    }

    /// Send an RPC request and return the response.
    ///
    /// Includes automatic retry logic with exponential backoff and peer rotation.
    ///
    /// ## NOT_INITIALIZED Handling
    ///
    /// When a peer returns NOT_INITIALIZED, we use exponential backoff before retrying
    /// the same peer. This handles the race condition where nodes have received Raft
    /// membership via replication but haven't yet set their initialized flag.
    ///
    /// ## SERVICE_UNAVAILABLE Handling
    ///
    /// For SERVICE_UNAVAILABLE errors, we immediately rotate to the next peer since
    /// the issue is specific to that node (e.g., rate limiting).
    ///
    /// ## Network Error Handling
    ///
    /// For network-level failures (connection refused, timeout), we retry with a fixed
    /// delay before rotating to the next peer.
    ///
    /// # Tiger Style
    ///
    /// - Bounded retries per peer (MAX_RETRIES_PER_PEER = 5)
    /// - Bounded backoff (MAX_BACKOFF_DELAY = 10s)
    /// - Fail-fast on permanent errors (auth failures, invalid requests)
    /// - Transparent failover improves CLI reliability in multi-node clusters
    pub async fn send(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        let mut last_error = None;

        // Try each bootstrap peer (with full endpoint addresses from V2 tickets)
        for (peer_idx, peer_addr) in self.bootstrap_addrs.iter().enumerate() {
            let peer_id = peer_addr.id;
            // Retry loop for this specific peer with exponential backoff for NOT_INITIALIZED
            let mut backoff = INITIAL_BACKOFF_DELAY;

            for attempt in 0..MAX_RETRIES_PER_PEER {
                if attempt > 0 {
                    debug!(peer_idx, attempt, peer_id = %peer_id, "retrying RPC request");
                }

                match self.send_to_peer_addr(peer_addr, request.clone()).await {
                    Ok(response) => {
                        // Convert CapabilityUnavailable to Error so all handlers
                        // get a consistent error type they already match against.
                        if let ClientRpcResponse::CapabilityUnavailable(ref cap) = response {
                            return Ok(ClientRpcResponse::Error(aspen_client_api::ErrorResponse {
                                code: "CAPABILITY_UNAVAILABLE".to_string(),
                                message: cap.message.clone(),
                            }));
                        }

                        // Check if server returned a retryable error
                        if let ClientRpcResponse::Error(ref e) = response {
                            if e.code == "NOT_INITIALIZED" {
                                // Exponential backoff: cluster may still be initializing
                                debug!(
                                    peer_id = %peer_id,
                                    attempt,
                                    backoff_ms = backoff.as_millis(),
                                    "cluster not initialized, retrying with exponential backoff"
                                );
                                last_error = Some(anyhow::anyhow!("{}: {}", e.code, e.message));
                                tokio::time::sleep(backoff).await;

                                // Double the backoff, capped at MAX_BACKOFF_DELAY
                                backoff = std::cmp::min(backoff * 2, MAX_BACKOFF_DELAY);
                                continue;
                            } else if e.code == "SERVICE_UNAVAILABLE" {
                                // Immediate peer rotation for transient per-node issues
                                debug!(
                                    peer_id = %peer_id,
                                    error_code = %e.code,
                                    "service unavailable, trying next peer"
                                );
                                last_error = Some(anyhow::anyhow!("{}: {}", e.code, e.message));
                                break; // Move to next peer
                            } else if e.code == "NOT_LEADER" {
                                // Immediate peer rotation — this node isn't the Raft
                                // leader, so write operations must go to another peer.
                                debug!(
                                    peer_id = %peer_id,
                                    error_code = %e.code,
                                    message = %e.message,
                                    "not leader, trying next peer"
                                );
                                last_error = Some(anyhow::anyhow!("{}: {}", e.code, e.message));
                                break; // Move to next peer
                            }
                        }
                        return Ok(response);
                    }
                    Err(e) => {
                        warn!(peer_idx, attempt, peer_id = %peer_id, error = %e, "RPC request failed");
                        last_error = Some(e);
                        // Fixed delay for network errors, then move to next peer after retries
                        if attempt < MAX_RETRIES_PER_PEER - 1 {
                            tokio::time::sleep(NETWORK_RETRY_DELAY).await;
                        }
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!(
                "RPC failed after trying {} peer(s) with {} retries each",
                self.bootstrap_addrs.len(),
                MAX_RETRIES_PER_PEER
            )
        }))
    }

    /// Calculate exponential backoff delay for a given attempt.
    ///
    /// Returns: initial * 2^attempt, capped at max_delay.
    #[allow(dead_code)]
    fn exponential_backoff(attempt: u32, initial: Duration, max_delay: Duration) -> Duration {
        let multiplier = 2u64.saturating_pow(attempt);
        let delay = Duration::from_millis(initial.as_millis() as u64 * multiplier);
        std::cmp::min(delay, max_delay)
    }

    /// Get a connection for a peer, reusing cached connection if available.
    ///
    /// Uses the full EndpointAddr including direct addresses for connection.
    /// This is critical for relay-less environments where discovery doesn't work.
    ///
    /// Returns a cached connection if one exists, otherwise creates a new one.
    /// Connection health is not pre-validated - failures are handled when
    /// the connection is used (e.g., open_bi() fails for closed connections).
    async fn get_connection_for_addr(&self, addr: &EndpointAddr) -> Result<Connection> {
        let peer_id = addr.id;

        // First, try to get a cached connection
        {
            let mut conns = self.connections.lock().await;
            if let Some(peer_conns) = conns.get_mut(&peer_id) {
                if let Some(conn) = peer_conns.pop() {
                    debug!(peer_id = %peer_id, "reusing cached connection");
                    return Ok(conn);
                }
            }
        }

        // No cached connection, create a new one using full EndpointAddr
        let direct_addrs: Vec<_> = addr
            .addrs
            .iter()
            .filter_map(|a| match a {
                iroh::TransportAddr::Ip(socket_addr) => Some(socket_addr),
                _ => None,
            })
            .collect();
        debug!(
            peer_id = %peer_id,
            direct_addrs = ?direct_addrs,
            "creating new connection with direct addresses"
        );

        let connection: Connection = timeout(self.rpc_timeout, async {
            self.endpoint.connect(addr.clone(), CLIENT_ALPN).await.context("failed to connect to peer")
        })
        .await
        .context("connection timeout")??;

        Ok(connection)
    }

    /// Return a connection to the cache for reuse.
    ///
    /// Connections are cached without health validation. If a cached connection
    /// has closed, it will fail on next use and be discarded then.
    async fn return_connection(&self, peer_id: EndpointId, connection: Connection) {
        let mut conns = self.connections.lock().await;
        let peer_conns = conns.entry(peer_id).or_default();

        // Enforce maximum cached connections per peer
        if peer_conns.len() < MAX_CACHED_CONNECTIONS {
            debug!(peer_id = %peer_id, cached = peer_conns.len() + 1, "caching connection for reuse");
            peer_conns.push(connection);
        } else {
            // Too many cached connections, close this one
            debug!(peer_id = %peer_id, "cache full, closing connection");
            connection.close(VarInt::from_u32(0), b"done");
        }
    }

    /// Discard a connection (don't return to cache).
    fn discard_connection(&self, peer_id: EndpointId, connection: Connection) {
        debug!(peer_id = %peer_id, "discarding connection due to error");
        connection.close(VarInt::from_u32(1), b"error");
    }

    /// Send an RPC request to a specific peer using its full EndpointAddr.
    async fn send_to_peer_addr(&self, addr: &EndpointAddr, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        let peer_id = addr.id;
        let connection = self.get_connection_for_addr(addr).await?;

        // Wrap request with authentication if token is present.
        let authenticated_request = match self.token.as_ref() {
            Some(token) => AuthenticatedRequest::new(request, token.clone()),
            None => AuthenticatedRequest::unauthenticated(request),
        };
        let request_bytes = postcard::to_stdvec(&authenticated_request).context("failed to serialize request")?;

        let deadline = Instant::now() + self.rpc_timeout;
        let response_bytes = match async {
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
            run_stage_with_deadline(
                deadline,
                recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE),
                "response timeout",
                "failed to read response",
            )
            .await
        }
        .await
        {
            Ok(response_bytes) => response_bytes,
            Err(error) => {
                self.discard_connection(peer_id, connection);
                return Err(error);
            }
        };

        let response: ClientRpcResponse = match postcard::from_bytes(&response_bytes) {
            Ok(response) => response,
            Err(error) => {
                self.discard_connection(peer_id, connection);
                return Err(anyhow::Error::new(error).context("failed to deserialize response"));
            }
        };

        self.return_connection(peer_id, connection).await;
        Ok(response)
    }

    /// Send a GetBlob request with streaming support for large blobs.
    ///
    /// Returns the metadata response. If `output` is provided and the server
    /// streams blob data, writes the raw bytes directly to the file.
    /// Send a GetBlob request with streaming support for large blobs.
    ///
    /// Returns the metadata response. If `output` is provided and the server
    /// streams blob data (blobs > 4MB), writes the raw bytes directly to the file.
    pub async fn send_get_blob(&self, hash: String, output: Option<&std::path::Path>) -> Result<ClientRpcResponse> {
        let peer_addr = self.bootstrap_addrs.first().context("no peers available")?;
        let connection = self.get_connection_for_addr(peer_addr).await?;

        let request = ClientRpcRequest::GetBlob { hash };
        let authenticated_request = match self.token.as_ref() {
            Some(token) => AuthenticatedRequest::new(request, token.clone()),
            None => AuthenticatedRequest::unauthenticated(request),
        };
        let request_bytes = postcard::to_stdvec(&authenticated_request).context("failed to serialize request")?;

        let deadline = Instant::now() + self.rpc_timeout;
        let blob_read_timeout = std::cmp::max(self.rpc_timeout, GET_BLOB_READ_TIMEOUT);
        let all_bytes = match async {
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
            run_timed_stage(
                blob_read_timeout,
                recv.read_to_end(MAX_GET_BLOB_RESPONSE_SIZE),
                "blob response timeout",
                "failed to read response",
            )
            .await
        }
        .await
        {
            Ok(all_bytes) => all_bytes,
            Err(error) => {
                self.discard_connection(peer_addr.id, connection);
                return Err(error);
            }
        };

        let (response, remaining) = match postcard::take_from_bytes::<ClientRpcResponse>(&all_bytes) {
            Ok(parsed) => parsed,
            Err(error) => {
                self.discard_connection(peer_addr.id, connection);
                return Err(anyhow::Error::new(error).context("failed to deserialize response"));
            }
        };

        self.return_connection(peer_addr.id, connection).await;

        // If streaming, write the remaining bytes (raw blob data) to file.
        if let ClientRpcResponse::GetBlobResult(ref result) = response {
            if result.is_streaming {
                if let Some(output_path) = output {
                    std::fs::write(output_path, remaining)
                        .with_context(|| format!("failed to write blob to {}", output_path.display()))?;
                }
            }
        }

        Ok(response)
    }

    /// Get the cluster ID from the ticket.
    ///
    /// Used as the cluster cookie for `WatchSession` authentication.
    pub fn cluster_id(&self) -> &str {
        &self.cluster_id
    }

    /// Get a reference to the iroh endpoint.
    ///
    /// Callers use this to establish `WatchSession` connections for
    /// real-time KV change notifications (e.g., CI log streaming).
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// Get the first bootstrap peer's endpoint address.
    ///
    /// Returns `None` if no bootstrap peers are configured (should never
    /// happen since `connect()` rejects empty peer lists).
    pub fn first_peer_addr(&self) -> Option<&EndpointAddr> {
        self.bootstrap_addrs.first()
    }

    /// Shutdown the client and close all connections and the endpoint.
    #[allow(dead_code)]
    pub async fn shutdown(self) {
        // Close all cached connections
        let conns = self.connections.lock().await;
        for (peer_id, peer_conns) in conns.iter() {
            for conn in peer_conns {
                debug!(peer_id = %peer_id, "closing cached connection on shutdown");
                conn.close(VarInt::from_u32(0), b"shutdown");
            }
        }
        drop(conns);

        self.endpoint.close().await;
    }

    /// Send an RPC request to a specific endpoint address.
    ///
    /// Used for cross-node verification where we need to query
    /// multiple nodes directly. This method does NOT use connection
    /// caching since verification is typically one-shot.
    pub async fn send_to(&self, addr: &EndpointAddr, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        let connection = timeout(self.rpc_timeout, async {
            self.endpoint.connect(addr.clone(), CLIENT_ALPN).await.context("failed to connect to peer")
        })
        .await
        .context("connection timeout")??;

        let authenticated_request = match self.token.as_ref() {
            Some(token) => AuthenticatedRequest::new(request, token.clone()),
            None => AuthenticatedRequest::unauthenticated(request),
        };
        let request_bytes = postcard::to_stdvec(&authenticated_request).context("failed to serialize request")?;

        let deadline = Instant::now() + self.rpc_timeout;
        let response_bytes = match async {
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
            run_stage_with_deadline(
                deadline,
                recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE),
                "response timeout",
                "failed to read response",
            )
            .await
        }
        .await
        {
            Ok(response_bytes) => response_bytes,
            Err(error) => {
                connection.close(VarInt::from_u32(1), b"error");
                return Err(error);
            }
        };

        let response: ClientRpcResponse = match postcard::from_bytes(&response_bytes) {
            Ok(response) => response,
            Err(error) => {
                connection.close(VarInt::from_u32(1), b"error");
                return Err(anyhow::Error::new(error).context("failed to deserialize response"));
            }
        };

        connection.close(VarInt::from_u32(0), b"done");

        if let ClientRpcResponse::CapabilityUnavailable(ref cap) = response {
            return Ok(ClientRpcResponse::Error(aspen_client_api::ErrorResponse {
                code: "CAPABILITY_UNAVAILABLE".to_string(),
                message: cap.message.clone(),
            }));
        }

        Ok(response)
    }
}

/// Convert a CapabilityUnavailable response into a standard Error.
///
/// Extracted as a free function so the logic can be unit-tested without
/// a live network connection.
pub(crate) fn normalize_capability_unavailable(response: ClientRpcResponse) -> ClientRpcResponse {
    if let ClientRpcResponse::CapabilityUnavailable(ref cap) = response {
        ClientRpcResponse::Error(aspen_client_api::ErrorResponse {
            code: "CAPABILITY_UNAVAILABLE".to_string(),
            message: cap.message.clone(),
        })
    } else {
        response
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::net::IpAddr;
    use std::net::Ipv4Addr;
    use std::net::SocketAddr;

    use super::*;

    fn test_endpoint_addr(endpoint: &Endpoint) -> EndpointAddr {
        let mut addr = EndpointAddr::new(endpoint.id());
        for socket_addr in endpoint.bound_sockets() {
            let fixed = if socket_addr.ip().is_unspecified() {
                SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), socket_addr.port())
            } else {
                socket_addr
            };
            addr.addrs.insert(iroh::TransportAddr::Ip(fixed));
        }
        addr
    }

    async fn make_test_endpoint() -> Endpoint {
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        Endpoint::builder(iroh::endpoint::presets::N0)
            .secret_key(secret_key)
            .alpns(vec![CLIENT_ALPN.to_vec()])
            .clear_address_lookup()
            .bind()
            .await
            .expect("bind test endpoint")
    }

    async fn spawn_nonresponsive_server() -> (Endpoint, EndpointAddr, tokio::task::JoinHandle<()>) {
        let endpoint = make_test_endpoint().await;
        let addr = test_endpoint_addr(&endpoint);
        let server_endpoint = endpoint.clone();

        let task = tokio::spawn(async move {
            let Some(incoming) = server_endpoint.accept().await else {
                return;
            };
            let connection = incoming.await.expect("accept connection");
            let (_send, mut recv) = connection.accept_bi().await.expect("accept stream");
            let _ = recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE).await.expect("drain request");
            tokio::time::sleep(Duration::from_millis(250)).await;
        });

        (endpoint, addr, task)
    }

    async fn make_test_client(peer_addr: EndpointAddr, rpc_timeout: Duration) -> AspenClient {
        AspenClient {
            endpoint: make_test_endpoint().await,
            bootstrap_addrs: vec![peer_addr],
            cluster_id: "test-cluster".to_string(),
            rpc_timeout,
            token: None,
            connections: Mutex::new(HashMap::new()),
        }
    }

    /// CapabilityUnavailable must become a standard Error so every command
    /// handler's `ClientRpcResponse::Error(e)` arm catches it without
    /// needing per-handler match arms.  Regression test for the
    /// "failed to deserialize response / unexpected response type" crash.
    #[test]
    fn test_capability_unavailable_converts_to_error() {
        let cap = ClientRpcResponse::CapabilityUnavailable(aspen_client_api::CapabilityUnavailableResponse {
            required_app: "hooks".to_string(),
            message: "the 'hooks' app is not loaded on this cluster".to_string(),
            hints: vec![],
        });

        let result = normalize_capability_unavailable(cap);

        match result {
            ClientRpcResponse::Error(e) => {
                assert_eq!(e.code, "CAPABILITY_UNAVAILABLE");
                assert!(e.message.contains("hooks"));
            }
            other => panic!("expected Error, got: {other:?}"),
        }
    }

    /// A CapabilityUnavailable response must survive a postcard round-trip
    /// so the CLI can decode what the server actually sends.
    #[test]
    fn test_capability_unavailable_postcard_roundtrip() {
        let original = ClientRpcResponse::CapabilityUnavailable(aspen_client_api::CapabilityUnavailableResponse {
            required_app: "forge".to_string(),
            message: "the 'forge' app is not loaded on this cluster".to_string(),
            hints: vec![],
        });

        let bytes = postcard::to_stdvec(&original).expect("serialize");
        let decoded: ClientRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");

        // After normalization, it must become a well-formed Error
        let normalized = normalize_capability_unavailable(decoded);
        match normalized {
            ClientRpcResponse::Error(e) => {
                assert_eq!(e.code, "CAPABILITY_UNAVAILABLE");
                assert!(e.message.contains("forge"));
            }
            other => panic!("expected Error, got: {other:?}"),
        }
    }

    /// Regression: feature-gated response variants (CI, Automerge) caused
    /// "Found a bool that wasn't 0 or 1" postcard deserialization crashes
    /// when the CLI was built without those features. Since all features are
    /// now default-on in aspen-client-api, every variant must roundtrip.
    #[test]
    fn test_all_feature_gated_responses_deserialize_cleanly() {
        use aspen_client_api::*;

        // CI-gated: CacheMigrationStartResult
        let ci_resp =
            ClientRpcResponse::CacheMigrationStartResult(aspen_client_api::ci::CacheMigrationStartResultResponse {
                started: false,
                status: None,
                error: Some("not available".into()),
            });
        let ci_bytes = postcard::to_stdvec(&ci_resp).expect("CI serialize");
        let ci_decoded: ClientRpcResponse = postcard::from_bytes(&ci_bytes).expect("CI deserialize must not crash");
        assert!(matches!(ci_decoded, ClientRpcResponse::CacheMigrationStartResult(_)));

        // Automerge-gated: AutomergeListResult
        let am_resp =
            ClientRpcResponse::AutomergeListResult(aspen_client_api::automerge::AutomergeListResultResponse {
                is_success: true,
                documents: vec![],
                has_more: false,
                continuation_token: None,
                error: None,
            });
        let am_bytes = postcard::to_stdvec(&am_resp).expect("AM serialize");
        let am_decoded: ClientRpcResponse =
            postcard::from_bytes(&am_bytes).expect("Automerge deserialize must not crash");
        assert!(matches!(am_decoded, ClientRpcResponse::AutomergeListResult(_)));

        // Hook response (was crashing before the fix)
        let hook_resp = ClientRpcResponse::HookListResult(aspen_client_api::HookListResultResponse {
            is_enabled: false,
            handlers: vec![],
        });
        let hook_bytes = postcard::to_stdvec(&hook_resp).expect("hook serialize");
        let hook_decoded: ClientRpcResponse =
            postcard::from_bytes(&hook_bytes).expect("Hook deserialize must not crash");
        assert!(matches!(hook_decoded, ClientRpcResponse::HookListResult(_)));

        // Federation response (was crashing before the fix)
        let fed_resp = ClientRpcResponse::FederationStatus(aspen_client_api::FederationStatusResponse {
            is_enabled: false,
            cluster_name: String::new(),
            cluster_key: String::new(),
            dht_enabled: false,
            gossip_enabled: false,
            discovered_clusters: 0,
            federated_repos: 0,
            error: None,
        });
        let fed_bytes = postcard::to_stdvec(&fed_resp).expect("federation serialize");
        let fed_decoded: ClientRpcResponse =
            postcard::from_bytes(&fed_bytes).expect("Federation deserialize must not crash");
        assert!(matches!(fed_decoded, ClientRpcResponse::FederationStatus(_)));
    }

    /// Error responses from the server must always decode correctly,
    /// regardless of which features are enabled. This is critical for
    /// the CLI's NOT_LEADER retry loop.
    #[test]
    fn test_error_response_always_decodable() {
        // Simulate what the server sends for INTERNAL_ERROR
        let err = ClientRpcResponse::Error(aspen_client_api::ErrorResponse {
            code: "INTERNAL_ERROR".to_string(),
            message: "internal error".to_string(),
        });
        let bytes = postcard::to_stdvec(&err).expect("serialize");
        let decoded: ClientRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");
        match decoded {
            ClientRpcResponse::Error(e) => {
                assert_eq!(e.code, "INTERNAL_ERROR");
            }
            other => panic!("expected Error, got: {other:?}"),
        }
    }

    /// Non-CapabilityUnavailable responses must pass through unchanged.
    #[test]
    fn test_normalize_passes_through_normal_responses() {
        let ok = ClientRpcResponse::Pong;
        let result = normalize_capability_unavailable(ok);
        assert!(matches!(result, ClientRpcResponse::Pong));

        let err = ClientRpcResponse::Error(aspen_client_api::ErrorResponse {
            code: "NOT_FOUND".to_string(),
            message: "key not found".to_string(),
        });
        let result = normalize_capability_unavailable(err);
        match result {
            ClientRpcResponse::Error(e) => assert_eq!(e.code, "NOT_FOUND"),
            other => panic!("expected Error, got: {other:?}"),
        }
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

    #[tokio::test]
    async fn test_timed_stage_reports_request_write_timeout() {
        let result = run_timed_stage(
            Duration::from_millis(10),
            std::future::pending::<std::result::Result<(), std::io::Error>>(),
            "request write timeout",
            "failed to send request",
        )
        .await;

        let error = result.expect_err("pending future must time out");
        assert!(error.to_string().contains("request write timeout"));
    }

    #[tokio::test]
    async fn test_send_to_times_out_when_peer_never_replies() {
        let (server_endpoint, server_addr, server_task) = spawn_nonresponsive_server().await;
        let client = make_test_client(server_addr.clone(), Duration::from_millis(50)).await;

        let result = client.send_to(&server_addr, ClientRpcRequest::Ping).await;
        let error = result.expect_err("peer should not reply");
        assert!(error.to_string().contains("response timeout"));

        client.endpoint.close().await;
        server_endpoint.close().await;
        server_task.abort();
    }

    #[tokio::test]
    async fn test_cached_connection_discarded_after_response_timeout() {
        let (server_endpoint, server_addr, server_task) = spawn_nonresponsive_server().await;
        let client = make_test_client(server_addr.clone(), Duration::from_millis(50)).await;

        let result = client.send_to_peer_addr(&server_addr, ClientRpcRequest::Ping).await;
        let error = result.expect_err("peer should not reply");
        assert!(error.to_string().contains("response timeout"));

        let connections = client.connections.lock().await;
        let cached_count = connections.get(&server_addr.id).map(Vec::len).unwrap_or(0);
        assert_eq!(cached_count, 0, "timed-out connection must not be returned to cache");
        drop(connections);

        client.endpoint.close().await;
        server_endpoint.close().await;
        server_task.abort();
    }
}
