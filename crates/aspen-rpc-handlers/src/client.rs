//! Client protocol handler.
//!
//! Handles client RPC connections over Iroh with the `aspen-client` ALPN.

use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::time::Instant;

use iroh::endpoint::Connection;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use aspen_transport::MAX_CLIENT_CONNECTIONS;
use aspen_transport::MAX_CLIENT_STREAMS_PER_CONNECTION;
use crate::error_sanitization::sanitize_error_for_client;
use crate::HandlerRegistry;
use aspen_core::api::ClusterController;
use aspen_core::api::KeyValueStore;
use aspen_auth::TokenVerifier;
use aspen_blob::IrohBlobStore;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_client_rpc::MAX_CLIENT_MESSAGE_SIZE;
use aspen_cluster::IrohEndpointManager;
use aspen_coordination::AtomicCounter;
use aspen_coordination::CounterConfig;
use aspen_coordination::DistributedRateLimiter;
use aspen_coordination::RateLimitError;
use aspen_coordination::RateLimiterConfig;
use aspen_coordination::SequenceConfig;
use aspen_coordination::SequenceGenerator;
use aspen_raft::constants::CLIENT_RPC_BURST;
use aspen_raft::constants::CLIENT_RPC_RATE_LIMIT_PREFIX;
use aspen_raft::constants::CLIENT_RPC_RATE_PER_SECOND;
use aspen_raft::constants::CLIENT_RPC_REQUEST_COUNTER;
use aspen_raft::constants::CLIENT_RPC_REQUEST_ID_SEQUENCE;

// ============================================================================
// Client Protocol Handler
// ============================================================================

/// Context for Client protocol handler with all dependencies.
#[derive(Clone)]
pub struct ClientProtocolContext {
    /// Node identifier.
    pub node_id: u64,
    /// Cluster controller for Raft operations.
    pub controller: Arc<dyn ClusterController>,
    /// Key-value store interface.
    pub kv_store: Arc<dyn KeyValueStore>,
    /// SQL query executor for read-only SQL queries.
    #[cfg(feature = "sql")]
    pub sql_executor: Arc<dyn crate::api::SqlQueryExecutor>,
    /// State machine for direct reads (lease queries, etc.).
    pub state_machine: Option<crate::raft::StateMachineVariant>,
    /// Iroh endpoint manager for peer info.
    pub endpoint_manager: Arc<IrohEndpointManager>,
    /// Blob store for content-addressed storage (optional).
    pub blob_store: Option<Arc<IrohBlobStore>>,
    /// Peer manager for cluster-to-cluster sync (optional).
    pub peer_manager: Option<Arc<crate::docs::PeerManager>>,
    /// Docs sync resources for iroh-docs operations (optional).
    pub docs_sync: Option<Arc<crate::docs::DocsSyncResources>>,
    /// Cluster cookie for ticket generation.
    pub cluster_cookie: String,
    /// Node start time for uptime calculation.
    pub start_time: Instant,
    /// Network factory for dynamic peer addition (optional).
    ///
    /// When present, enables AddPeer RPC to register peers in the network factory.
    pub network_factory: Option<Arc<crate::cluster::IrpcRaftNetworkFactory>>,
    /// Token verifier for capability-based authorization.
    ///
    /// Optional during migration period. When `None`, all requests are allowed.
    /// When `Some`, requests that require auth must provide valid tokens.
    pub token_verifier: Option<Arc<TokenVerifier>>,
    /// Whether to require authentication for all authorized requests.
    ///
    /// When `false` (default), missing tokens are allowed during migration.
    /// When `true`, requests without valid tokens are rejected.
    pub require_auth: bool,
    /// Shard topology for GetTopology RPC (optional).
    ///
    /// When present, enables topology queries for shard-aware clients.
    pub topology: Option<Arc<tokio::sync::RwLock<crate::sharding::topology::ShardTopology>>>,
    /// Content discovery service for DHT announcements and provider lookup (optional).
    ///
    /// When present, enables:
    /// - Automatic DHT announcements when blobs are added
    /// - DHT provider discovery for hash-only downloads
    /// - Provider aggregation combining ticket + DHT providers
    #[cfg(feature = "global-discovery")]
    pub content_discovery: Option<crate::cluster::content_discovery::ContentDiscoveryService>,
    /// Forge node for decentralized Git operations (optional).
    ///
    /// When present, enables Forge RPC operations for:
    /// - Repository management (create, get, list)
    /// - Git object storage (blobs, trees, commits)
    /// - Ref management (branches, tags)
    /// - Collaborative objects (issues, patches)
    #[cfg(feature = "forge")]
    pub forge_node: Option<Arc<crate::forge::ForgeNode<crate::blob::IrohBlobStore, dyn crate::api::KeyValueStore>>>,
    /// Pijul store for patch-based version control (optional).
    ///
    /// When present, enables Pijul RPC operations for:
    /// - Repository management (init, list, info)
    /// - Channel management (list, create, delete, fork)
    /// - Change operations (record, apply, log, checkout)
    #[cfg(feature = "pijul")]
    pub pijul_store: Option<Arc<crate::pijul::PijulStore<crate::blob::IrohBlobStore, dyn crate::api::KeyValueStore>>>,
}

impl std::fmt::Debug for ClientProtocolContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientProtocolContext")
            .field("node_id", &self.node_id)
            .field("cluster_cookie", &self.cluster_cookie)
            .finish_non_exhaustive()
    }
}

/// Protocol handler for Client RPC over Iroh.
///
/// Implements `ProtocolHandler` to receive connections routed by the Iroh Router
/// based on the `aspen-tui` ALPN. Processes Client requests for monitoring and
/// control operations.
///
/// # Tiger Style
///
/// - Lower connection limit than Raft (Client is less critical)
/// - Bounded stream count per connection
/// - Explicit size limits on messages
#[derive(Debug)]
pub struct ClientProtocolHandler {
    ctx: Arc<ClientProtocolContext>,
    connection_semaphore: Arc<Semaphore>,
    registry: HandlerRegistry,
}

impl ClientProtocolHandler {
    /// Create a new Client protocol handler.
    ///
    /// # Arguments
    /// * `ctx` - Client context with dependencies
    pub fn new(ctx: ClientProtocolContext) -> Self {
        let registry = HandlerRegistry::new(&ctx);
        Self {
            ctx: Arc::new(ctx),
            connection_semaphore: Arc::new(Semaphore::new(MAX_CLIENT_CONNECTIONS as usize)),
            registry,
        }
    }
}

impl ProtocolHandler for ClientProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote_node_id = connection.remote_id();

        // Try to acquire a connection permit
        let permit = match self.connection_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    "Client connection limit reached ({}), rejecting connection from {}",
                    MAX_CLIENT_CONNECTIONS, remote_node_id
                );
                return Err(AcceptError::from_err(std::io::Error::other("connection limit reached")));
            }
        };

        debug!(remote_node = %remote_node_id, "accepted Client connection");

        // Handle the connection with bounded resources
        let result = handle_client_connection(connection, Arc::clone(&self.ctx), self.registry.clone()).await;

        // Release permit when done
        drop(permit);

        result.map_err(|err| AcceptError::from_err(std::io::Error::other(err.to_string())))
    }

    async fn shutdown(&self) {
        info!("Client protocol handler shutting down");
        self.connection_semaphore.close();
    }
}

/// Handle a single Client connection.
#[instrument(skip(connection, ctx, registry))]
async fn handle_client_connection(
    connection: Connection,
    ctx: Arc<ClientProtocolContext>,
    registry: HandlerRegistry,
) -> anyhow::Result<()> {
    let remote_node_id = connection.remote_id();

    let stream_semaphore = Arc::new(Semaphore::new(MAX_CLIENT_STREAMS_PER_CONNECTION as usize));
    let active_streams = Arc::new(AtomicU32::new(0));

    // Accept bidirectional streams from this connection
    loop {
        let stream = match connection.accept_bi().await {
            Ok(stream) => stream,
            Err(err) => {
                debug!(remote_node = %remote_node_id, error = %err, "Client connection closed");
                break;
            }
        };

        let permit = match stream_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    remote_node = %remote_node_id,
                    max_streams = MAX_CLIENT_STREAMS_PER_CONNECTION,
                    "Client stream limit reached, dropping stream"
                );
                continue;
            }
        };

        active_streams.fetch_add(1, Ordering::Relaxed);
        let active_streams_clone = active_streams.clone();

        let ctx_clone = Arc::clone(&ctx);
        let registry_clone = registry.clone();
        // Pass the PublicKey directly - no string conversion needed
        // This eliminates the security risk of parsing failures
        let (send, recv) = stream;
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(err) = handle_client_request((recv, send), ctx_clone, remote_node_id, registry_clone).await {
                error!(error = %err, "failed to handle Client request");
            }
            active_streams_clone.fetch_sub(1, Ordering::Relaxed);
        });
    }

    Ok(())
}

/// Handle a single Client RPC request on a stream.
#[instrument(skip(recv, send, ctx, registry), fields(client_id = %client_id, request_id))]
async fn handle_client_request(
    (mut recv, mut send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    ctx: Arc<ClientProtocolContext>,
    client_id: iroh::PublicKey,
    registry: HandlerRegistry,
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Generate unique request ID using distributed sequence
    // Tiger Style: Monotonic IDs enable cluster-wide request tracing
    let request_id = {
        let seq =
            SequenceGenerator::new(ctx.kv_store.clone(), CLIENT_RPC_REQUEST_ID_SEQUENCE, SequenceConfig::default());
        seq.next().await.unwrap_or(0)
    };

    // Record request_id in current span for distributed tracing
    tracing::Span::current().record("request_id", request_id);

    // Read the request with size limit
    let buffer = recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE).await.context("failed to read Client request")?;

    // Try to parse as AuthenticatedRequest first (new format)
    // Fall back to legacy ClientRpcRequest for backwards compatibility
    let (request, token) = match postcard::from_bytes::<crate::client_rpc::AuthenticatedRequest>(&buffer) {
        Ok(auth_req) => (auth_req.request, auth_req.token),
        Err(_) => {
            // Legacy format: parse as plain ClientRpcRequest
            let req: ClientRpcRequest =
                postcard::from_bytes(&buffer).context("failed to deserialize Client request")?;
            (req, None)
        }
    };

    debug!(request_type = ?request, client_id = %client_id, request_id = %request_id, has_token = token.is_some(), "received Client request");

    // Check if cluster is initialized
    let is_initialized = ctx.controller.is_initialized();

    // Bootstrap operations: can run before cluster initialization
    // These are status/bootstrap operations that should work on any node
    let is_bootstrap_operation = matches!(
        &request,
        ClientRpcRequest::Ping
            | ClientRpcRequest::GetHealth
            | ClientRpcRequest::GetNodeInfo
            | ClientRpcRequest::GetClusterTicket
            | ClientRpcRequest::GetRaftMetrics
            | ClientRpcRequest::InitCluster
    );

    // Rate-limit-exempt operations: skip rate limiting to avoid KV store dependency
    // Includes bootstrap operations plus cluster management operations that:
    // - Are administrative (run by operators, not regular client traffic)
    // - Are protected by Raft consensus anyway
    // - Must work during cluster bootstrap before KV store is available
    let is_rate_limit_exempt = is_bootstrap_operation
        || matches!(
            &request,
            ClientRpcRequest::AddLearner { .. }
                | ClientRpcRequest::ChangeMembership { .. }
                | ClientRpcRequest::TriggerSnapshot
                | ClientRpcRequest::GetClusterState
                | ClientRpcRequest::GetLeader
        );

    // For non-bootstrap operations on uninitialized cluster, return clear error
    // Tiger Style: Fail-fast with actionable error message
    if !is_bootstrap_operation && !is_initialized {
        let response = ClientRpcResponse::error("NOT_INITIALIZED", "cluster not initialized - call InitCluster first");
        let response_bytes = postcard::to_stdvec(&response).context("failed to serialize not initialized response")?;
        send.write_all(&response_bytes).await.context("failed to write not initialized response")?;
        send.finish().context("failed to finish send stream")?;
        return Ok(());
    }

    // Rate limit check for non-exempt operations only
    // Tiger Style: Per-client rate limiting prevents DoS from individual clients
    if !is_rate_limit_exempt {
        let rate_limit_key = format!("{}{}", CLIENT_RPC_RATE_LIMIT_PREFIX, client_id);
        let limiter = DistributedRateLimiter::new(
            ctx.kv_store.clone(),
            &rate_limit_key,
            RateLimiterConfig::new(CLIENT_RPC_RATE_PER_SECOND, CLIENT_RPC_BURST),
        );

        match limiter.try_acquire().await {
            Ok(_) => {}
            Err(RateLimitError::TokensExhausted { retry_after_ms, .. }) => {
                warn!(
                    client_id = %client_id,
                    retry_after_ms,
                    "Client rate limited"
                );
                let response = ClientRpcResponse::error(
                    "RATE_LIMITED",
                    format!("Too many requests. Retry after {}ms", retry_after_ms),
                );
                let response_bytes =
                    postcard::to_stdvec(&response).context("failed to serialize rate limit response")?;
                send.write_all(&response_bytes).await.context("failed to write rate limit response")?;
                send.finish().context("failed to finish send stream")?;
                return Ok(());
            }
            Err(RateLimitError::StorageUnavailable { reason }) => {
                // FAIL CLOSED: Reject request when rate limiter storage is unavailable
                // This maintains DoS protection at the cost of availability during partitions
                warn!(
                    client_id = %client_id,
                    reason = %reason,
                    "Rate limiter storage unavailable, rejecting request"
                );
                let response =
                    ClientRpcResponse::error("SERVICE_UNAVAILABLE", "rate limiter unavailable - try again later");
                let response_bytes =
                    postcard::to_stdvec(&response).context("failed to serialize unavailable response")?;
                send.write_all(&response_bytes).await.context("failed to write unavailable response")?;
                send.finish().context("failed to finish send stream")?;
                return Ok(());
            }
        }
    }

    // Authorization check: verify capability token if auth is enabled
    // Tiger Style: Fail-fast on authorization errors before processing request
    if let Some(ref verifier) = ctx.token_verifier {
        // Check if this request requires authorization
        if let Some(operation) = request.to_operation() {
            match &token {
                Some(cap_token) => {
                    // No parsing needed - client_id is already a PublicKey from Iroh connection
                    // This is type-safe and eliminates the security risk of parsing failures
                    let presenter = Some(&client_id);

                    // Verify token and authorize the operation
                    if let Err(auth_err) = verifier.authorize(cap_token, &operation, presenter) {
                        warn!(
                            client_id = %client_id,
                            error = %auth_err,
                            operation = ?operation,
                            "Authorization failed"
                        );
                        let response =
                            ClientRpcResponse::error("UNAUTHORIZED", format!("Authorization failed: {}", auth_err));
                        let response_bytes =
                            postcard::to_stdvec(&response).context("failed to serialize auth error response")?;
                        send.write_all(&response_bytes).await.context("failed to write auth error response")?;
                        send.finish().context("failed to finish send stream")?;
                        return Ok(());
                    }
                    debug!(client_id = %client_id, operation = ?operation, "Authorization succeeded");
                }
                None => {
                    // No token provided - check if auth is required
                    if ctx.require_auth {
                        warn!(
                            client_id = %client_id,
                            operation = ?operation,
                            "Missing authentication token"
                        );
                        let response =
                            ClientRpcResponse::error("UNAUTHORIZED", "Authentication required but no token provided");
                        let response_bytes =
                            postcard::to_stdvec(&response).context("failed to serialize auth error response")?;
                        send.write_all(&response_bytes).await.context("failed to write auth error response")?;
                        send.finish().context("failed to finish send stream")?;
                        return Ok(());
                    }
                    // During migration: warn but allow unauthenticated requests
                    debug!(
                        client_id = %client_id,
                        operation = ?operation,
                        "Unauthenticated request allowed (migration mode)"
                    );
                }
            }
        }
    }

    // Process the request through the handler registry
    let response = match registry.dispatch(request, &ctx).await {
        Ok(resp) => resp,
        Err(err) => {
            // Log full error internally for debugging, but return sanitized message to client
            // HIGH-4: Prevent information leakage through error messages
            warn!(error = %err, "Client request processing failed");
            ClientRpcResponse::error("INTERNAL_ERROR", sanitize_error_for_client(&err))
        }
    };

    // Serialize and send response
    let response_bytes = postcard::to_stdvec(&response).context("failed to serialize Client response")?;
    send.write_all(&response_bytes).await.context("failed to write Client response")?;
    send.finish().context("failed to finish send stream")?;

    // Increment cluster-wide request counter (best-effort, non-blocking)
    // Tiger Style: Fire-and-forget counter increment doesn't block request path
    let kv_store = ctx.kv_store.clone();
    tokio::spawn(async move {
        let counter = AtomicCounter::new(kv_store, CLIENT_RPC_REQUEST_COUNTER, CounterConfig::default());
        if let Err(e) = counter.increment().await {
            debug!(error = %e, "Failed to increment request counter");
        }
    });

    Ok(())
}
