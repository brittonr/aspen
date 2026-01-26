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
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_auth::CapabilityToken;
use aspen_client_api::MAX_CLIENT_MESSAGE_SIZE;
use aspen_client_rpc::AuthenticatedRequest;
use aspen_client_rpc::CLIENT_ALPN;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_cluster::ticket::parse_ticket_to_addrs;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::EndpointId;
use iroh::endpoint::Connection;
use iroh::endpoint::VarInt;
use tokio::sync::Mutex;
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
        let endpoint = Endpoint::builder()
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
        // Get connection (cached or new)
        let connection = self.get_connection_for_addr(addr).await?;

        // Open bidirectional stream for this request
        let stream_result = connection.open_bi().await;
        let (mut send, mut recv) = match stream_result {
            Ok(streams) => streams,
            Err(e) => {
                // Connection failed, don't cache it
                self.discard_connection(peer_id, connection);
                return Err(anyhow::Error::new(e).context("failed to open stream"));
            }
        };

        // Wrap request with authentication if token is present
        let authenticated_request = match self.token.as_ref() {
            Some(token) => AuthenticatedRequest::new(request, token.clone()),
            None => AuthenticatedRequest::unauthenticated(request),
        };
        let request_bytes = postcard::to_stdvec(&authenticated_request).context("failed to serialize request")?;

        // Send request
        if let Err(e) = send.write_all(&request_bytes).await {
            self.discard_connection(peer_id, connection);
            return Err(anyhow::Error::new(e).context("failed to send request"));
        }

        if let Err(e) = send.finish() {
            self.discard_connection(peer_id, connection);
            return Err(anyhow::Error::new(e).context("failed to finish send stream"));
        }

        // Read response with timeout
        let response_result = timeout(self.rpc_timeout, async {
            recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE).await.context("failed to read response")
        })
        .await;

        match response_result {
            Ok(Ok(response_bytes)) => {
                // Deserialize response
                let response: ClientRpcResponse =
                    postcard::from_bytes(&response_bytes).context("failed to deserialize response")?;

                // Return connection to cache for reuse
                self.return_connection(peer_id, connection).await;

                Ok(response)
            }
            Ok(Err(e)) => {
                self.discard_connection(peer_id, connection);
                Err(e)
            }
            Err(_) => {
                self.discard_connection(peer_id, connection);
                Err(anyhow::anyhow!("response timeout"))
            }
        }
    }

    /// Get the cluster ID from the ticket.
    #[allow(dead_code)]
    pub fn cluster_id(&self) -> &str {
        &self.cluster_id
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
        // Connect to the specific peer
        let connection = timeout(self.rpc_timeout, async {
            self.endpoint.connect(addr.clone(), CLIENT_ALPN).await.context("failed to connect to peer")
        })
        .await
        .context("connection timeout")??;

        // Open bidirectional stream
        let (mut send, mut recv) = connection.open_bi().await.context("failed to open stream")?;

        // Wrap request with authentication if token is present
        let authenticated_request = match self.token.as_ref() {
            Some(token) => AuthenticatedRequest::new(request, token.clone()),
            None => AuthenticatedRequest::unauthenticated(request),
        };
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
}
