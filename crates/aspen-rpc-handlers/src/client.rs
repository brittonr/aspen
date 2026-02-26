//! Client protocol handler.
//!
//! Handles client RPC connections over Iroh with the `aspen-client` ALPN.

use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use aspen_client_api::AuthenticatedRequest;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::MAX_CLIENT_MESSAGE_SIZE;
use aspen_coordination::AtomicCounter;
use aspen_coordination::CounterConfig;
use aspen_coordination::DistributedRateLimiter;
use aspen_coordination::RateLimitError;
use aspen_coordination::RateLimiterConfig;
use aspen_raft::constants::CLIENT_RPC_BURST;
use aspen_raft::constants::CLIENT_RPC_RATE_LIMIT_PREFIX;
use aspen_raft::constants::CLIENT_RPC_RATE_PER_SECOND;
use aspen_raft::constants::CLIENT_RPC_REQUEST_COUNTER;
use aspen_transport::MAX_CLIENT_CONNECTIONS;
use aspen_transport::MAX_CLIENT_STREAMS_PER_CONNECTION;
// ============================================================================
// Constants
// ============================================================================
/// ALPN identifier for the client protocol.
/// Re-exported from aspen-transport for consistency.
pub use aspen_transport::constants::CLIENT_ALPN;
use iroh::endpoint::Connection;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use crate::HandlerRegistry;
use crate::context::ClientProtocolContext;
use crate::error_sanitization::sanitize_error_for_client;

// ============================================================================
// Client Protocol Handler
// ============================================================================

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
    /// Local request ID counter.
    ///
    /// Uses a simple atomic counter instead of distributed sequence to avoid
    /// Raft consensus in the RPC critical path. Request IDs are node-local
    /// and combined with node_id for cluster-wide uniqueness.
    request_counter: AtomicU64,
}

impl ClientProtocolHandler {
    /// Create a new Client protocol handler.
    ///
    /// # Arguments
    /// * `ctx` - Client context with dependencies
    ///
    /// # Security Notes
    ///
    /// This constructor logs security warnings if authentication is not properly
    /// configured. Check logs for warnings with target `aspen_rpc::security`.
    pub fn new(ctx: ClientProtocolContext) -> Self {
        // Log security warnings about auth configuration
        ctx.log_auth_warnings();

        let registry = HandlerRegistry::new(&ctx);
        Self {
            ctx: Arc::new(ctx),
            connection_semaphore: Arc::new(Semaphore::new(MAX_CLIENT_CONNECTIONS as usize)),
            registry,
            request_counter: AtomicU64::new(0),
        }
    }

    /// Load WASM handler plugins from the cluster KV store.
    ///
    /// Must be called before the handler is consumed by the router.
    /// Failures are logged but do not prevent the node from starting.
    #[cfg(feature = "plugins-rpc")]
    pub async fn load_wasm_plugins(&mut self) {
        match self.registry.load_wasm_plugins(&self.ctx).await {
            Ok(count) if count > 0 => {
                info!(plugin_count = count, "WASM plugin handlers loaded");
            }
            Ok(_) => debug!("no WASM plugins to load"),
            Err(e) => warn!(error = %e, "failed to load WASM plugins, continuing without"),
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
        let result =
            handle_client_connection(connection, Arc::clone(&self.ctx), self.registry.clone(), &self.request_counter)
                .await;

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
#[instrument(skip(connection, ctx, registry, request_counter))]
async fn handle_client_connection(
    connection: Connection,
    ctx: Arc<ClientProtocolContext>,
    registry: HandlerRegistry,
    request_counter: &AtomicU64,
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

        // Generate request ID using local atomic counter (fast path, no Raft dependency)
        // Combined with node_id in tracing for cluster-wide uniqueness
        let request_id = request_counter.fetch_add(1, Ordering::Relaxed);

        // Pass the PublicKey directly - no string conversion needed
        // This eliminates the security risk of parsing failures
        let (send, recv) = stream;
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(err) =
                handle_client_request((recv, send), ctx_clone, remote_node_id, registry_clone, request_id).await
            {
                error!(error = %err, "failed to handle Client request");
            }
            active_streams_clone.fetch_sub(1, Ordering::Relaxed);
        });
    }

    Ok(())
}

/// Check if a request is a bootstrap operation (can run before cluster init).
fn handle_client_request_is_bootstrap(request: &ClientRpcRequest) -> bool {
    matches!(
        request,
        ClientRpcRequest::Ping
            | ClientRpcRequest::GetHealth
            | ClientRpcRequest::GetNodeInfo
            | ClientRpcRequest::GetClusterTicket
            | ClientRpcRequest::GetRaftMetrics
            | ClientRpcRequest::InitCluster
    )
}

/// Check if a request is exempt from rate limiting.
fn handle_client_request_is_rate_limit_exempt(request: &ClientRpcRequest) -> bool {
    handle_client_request_is_bootstrap(request)
        || matches!(
            request,
            ClientRpcRequest::AddLearner { .. }
                | ClientRpcRequest::ChangeMembership { .. }
                | ClientRpcRequest::TriggerSnapshot
                | ClientRpcRequest::GetClusterState
                | ClientRpcRequest::GetLeader
                | ClientRpcRequest::HookList
                | ClientRpcRequest::HookGetMetrics { .. }
                | ClientRpcRequest::HookTrigger { .. }
                | ClientRpcRequest::GitBridgeListRefs { .. }
                | ClientRpcRequest::GitBridgeFetch { .. }
                | ClientRpcRequest::GitBridgePush { .. }
                | ClientRpcRequest::GitBridgeProbeObjects { .. }
                | ClientRpcRequest::CiWatchRepo { .. }
                | ClientRpcRequest::CiUnwatchRepo { .. }
                | ClientRpcRequest::ForgeCreateRepo { .. }
                | ClientRpcRequest::ForgeListRepos { .. }
                | ClientRpcRequest::ForgeGetRepo { .. }
        )
}

/// Send an error response and finish the stream.
async fn handle_client_request_send_error(
    send: &mut iroh::endpoint::SendStream,
    code: &str,
    message: impl Into<String>,
) -> anyhow::Result<()> {
    use anyhow::Context;
    let response = ClientRpcResponse::error(code, message);
    let response_bytes = postcard::to_stdvec(&response).context("failed to serialize error response")?;
    send.write_all(&response_bytes).await.context("failed to write error response")?;
    send.finish().context("failed to finish send stream")?;
    Ok(())
}

/// Check rate limiting for a client request.
async fn handle_client_request_check_rate_limit(
    ctx: &ClientProtocolContext,
    client_id: &iroh::PublicKey,
    send: &mut iroh::endpoint::SendStream,
) -> anyhow::Result<bool> {
    let rate_limit_key = format!("{}{}", CLIENT_RPC_RATE_LIMIT_PREFIX, client_id);
    let limiter = DistributedRateLimiter::new(
        ctx.kv_store.clone(),
        &rate_limit_key,
        RateLimiterConfig::new(CLIENT_RPC_RATE_PER_SECOND, CLIENT_RPC_BURST),
    );

    match limiter.try_acquire().await {
        Ok(_) => Ok(true),
        Err(RateLimitError::TokensExhausted { retry_after_ms, .. }) => {
            warn!(client_id = %client_id, retry_after_ms, "Client rate limited");
            handle_client_request_send_error(
                send,
                "RATE_LIMITED",
                format!("Too many requests. Retry after {}ms", retry_after_ms),
            )
            .await?;
            Ok(false)
        }
        Err(RateLimitError::StorageUnavailable { reason }) => {
            warn!(client_id = %client_id, reason = %reason, "Rate limiter storage unavailable, rejecting request");
            handle_client_request_send_error(send, "SERVICE_UNAVAILABLE", "rate limiter unavailable - try again later")
                .await?;
            Ok(false)
        }
    }
}

/// Check authorization for a client request.
async fn handle_client_request_check_auth(
    ctx: &ClientProtocolContext,
    client_id: &iroh::PublicKey,
    request: &ClientRpcRequest,
    token: &Option<aspen_auth::CapabilityToken>,
    send: &mut iroh::endpoint::SendStream,
) -> anyhow::Result<bool> {
    let verifier = match &ctx.token_verifier {
        Some(v) => v,
        None => return Ok(true),
    };

    let operation = match request.to_operation() {
        Some(op) => op,
        None => return Ok(true),
    };

    match token {
        Some(cap_token) => {
            let presenter = Some(client_id);
            if let Err(auth_err) = verifier.authorize(cap_token, &operation, presenter) {
                warn!(client_id = %client_id, error = %auth_err, operation = ?operation, "Authorization failed");
                handle_client_request_send_error(send, "UNAUTHORIZED", format!("Authorization failed: {}", auth_err))
                    .await?;
                return Ok(false);
            }
            debug!(client_id = %client_id, operation = ?operation, "Authorization succeeded");
        }
        None => {
            if ctx.require_auth {
                warn!(client_id = %client_id, operation = ?operation, "Missing authentication token");
                handle_client_request_send_error(send, "UNAUTHORIZED", "Authentication required but no token provided")
                    .await?;
                return Ok(false);
            }
            debug!(client_id = %client_id, operation = ?operation, "Unauthenticated request allowed (migration mode)");
        }
    }
    Ok(true)
}

/// Handle a single Client RPC request on a stream.
#[instrument(skip(recv, send, ctx, registry), fields(client_id = %client_id, request_id = %request_id))]
async fn handle_client_request(
    (mut recv, mut send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    ctx: Arc<ClientProtocolContext>,
    client_id: iroh::PublicKey,
    registry: HandlerRegistry,
    request_id: u64,
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Read and parse the request
    let buffer = recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE).await.context("failed to read Client request")?;
    let (request, token, proxy_hops): (ClientRpcRequest, Option<aspen_auth::CapabilityToken>, u8) =
        match postcard::from_bytes::<AuthenticatedRequest>(&buffer) {
            Ok(auth_req) => (auth_req.request, auth_req.token, auth_req.proxy_hops),
            Err(_) => {
                let req: ClientRpcRequest =
                    postcard::from_bytes(&buffer).context("failed to deserialize Client request")?;
                (req, None, 0)
            }
        };

    debug!(request_type = ?request, client_id = %client_id, request_id = %request_id, has_token = token.is_some(), "received Client request");

    // Check cluster initialization
    let is_bootstrap_operation = handle_client_request_is_bootstrap(&request);
    if !is_bootstrap_operation && !ctx.controller.is_initialized() {
        handle_client_request_send_error(
            &mut send,
            "NOT_INITIALIZED",
            "cluster not initialized - call InitCluster first",
        )
        .await?;
        return Ok(());
    }

    // Rate limit check for non-exempt operations
    if !handle_client_request_is_rate_limit_exempt(&request)
        && !handle_client_request_check_rate_limit(&ctx, &client_id, &mut send).await?
    {
        return Ok(());
    }

    // Authorization check
    if !handle_client_request_check_auth(&ctx, &client_id, &request, &token, &mut send).await? {
        return Ok(());
    }

    // Process the request through the handler registry
    let response = match registry.dispatch(request, &ctx, proxy_hops).await {
        Ok(resp) => resp,
        Err(err) => {
            warn!(error = %err, "Client request processing failed");
            ClientRpcResponse::error("INTERNAL_ERROR", sanitize_error_for_client(&err))
        }
    };

    // Send response
    let response_bytes = postcard::to_stdvec(&response).context("failed to serialize Client response")?;
    send.write_all(&response_bytes).await.context("failed to write Client response")?;
    send.finish().context("failed to finish send stream")?;

    // Increment cluster-wide request counter (best-effort, non-blocking)
    let kv_store = ctx.kv_store.clone();
    tokio::spawn(async move {
        let counter = AtomicCounter::new(kv_store, CLIENT_RPC_REQUEST_COUNTER, CounterConfig::default());
        if let Err(e) = counter.increment().await {
            debug!(error = %e, "Failed to increment request counter");
        }
    });

    Ok(())
}
