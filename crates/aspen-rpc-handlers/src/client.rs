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
#[cfg(feature = "deploy")]
use aspen_cluster::upgrade::DrainState;
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
const MAX_STREAM_ACCEPTS_PER_CONNECTION: u32 = u32::MAX;

/// ALPN identifier for the client protocol.
/// Re-exported from aspen-transport for consistency.
pub use aspen_transport::constants::CLIENT_ALPN;
use iroh::endpoint::Connection;

fn usize_from_u32(value: u32) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use tokio::sync::Semaphore;
use tracing::Instrument;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::HandlerRegistry;
use crate::NativeHandlerPlan;
use crate::context::ClientProtocolContext;
use crate::error_sanitization::sanitize_error_for_client;

// ============================================================================
// DrainGuard — RAII guard for in-flight operation tracking
// ============================================================================

/// Drop guard that calls `finish_op()` on the shared `DrainState` when dropped.
///
/// Ensures the in-flight counter is decremented even if the request handler
/// panics or returns early with an error. Without this, a leaked counter
/// would prevent drain from ever completing.
#[cfg(feature = "deploy")]
struct DrainGuard {
    drain_state: Arc<DrainState>,
}

#[cfg(feature = "deploy")]
impl Drop for DrainGuard {
    fn drop(&mut self) {
        self.drain_state.finish_op();
    }
}

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
    pub fn new(ctx: ClientProtocolContext, plan: NativeHandlerPlan) -> anyhow::Result<Self> {
        // Log security warnings about auth configuration
        ctx.log_auth_warnings();

        let registry = HandlerRegistry::new(&ctx, &plan)?;
        Ok(Self {
            ctx: Arc::new(ctx),
            connection_semaphore: Arc::new(Semaphore::new(usize_from_u32(MAX_CLIENT_CONNECTIONS))),
            registry,
            request_counter: AtomicU64::new(0),
        })
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
async fn handle_client_connection(
    connection: Connection,
    ctx: Arc<ClientProtocolContext>,
    registry: HandlerRegistry,
    request_counter: &AtomicU64,
) -> anyhow::Result<()> {
    let remote_node_id = connection.remote_id();
    handle_client_connection_inner(connection, ctx, registry, request_counter)
        .instrument(tracing::info_span!("handle_client_connection", remote_node = %remote_node_id))
        .await
}

async fn handle_client_connection_inner(
    connection: Connection,
    ctx: Arc<ClientProtocolContext>,
    registry: HandlerRegistry,
    request_counter: &AtomicU64,
) -> anyhow::Result<()> {
    let remote_node_id = connection.remote_id();

    const { assert!(MAX_CLIENT_STREAMS_PER_CONNECTION > 0) };
    debug_assert!(!remote_node_id.as_bytes().is_empty());
    let stream_semaphore = Arc::new(Semaphore::new(usize_from_u32(MAX_CLIENT_STREAMS_PER_CONNECTION)));
    let active_streams = Arc::new(AtomicU32::new(0));

    // Accept bidirectional streams from this connection
    for _stream_accept_iteration in 0..MAX_STREAM_ACCEPTS_PER_CONNECTION {
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
            | ClientRpcRequest::InitClusterWithTrust { .. }
    )
}

/// Check if a request is exempt from rate limiting.
fn handle_client_request_is_rate_limit_exempt(request: &ClientRpcRequest) -> bool {
    handle_client_request_is_bootstrap(request)
        || matches!(
            request,
            ClientRpcRequest::AddLearner { .. }
                | ClientRpcRequest::ChangeMembership { .. }
                | ClientRpcRequest::ExpungeNode { .. }
                | ClientRpcRequest::TriggerSnapshot
                | ClientRpcRequest::GetClusterState
                | ClientRpcRequest::GetLeader
                | ClientRpcRequest::HookList
                | ClientRpcRequest::HookGetMetrics { .. }
                | ClientRpcRequest::HookTrigger { .. }
                | ClientRpcRequest::GitBridgeListRefs { .. }
                | ClientRpcRequest::GitBridgeFetch { .. }
                | ClientRpcRequest::GitBridgeFetchStart { .. }
                | ClientRpcRequest::GitBridgeFetchChunk { .. }
                | ClientRpcRequest::GitBridgeFetchComplete { .. }
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
    let client_request_bucket_key = format!("{}{}", CLIENT_RPC_RATE_LIMIT_PREFIX, client_id);
    let request_throttle = DistributedRateLimiter::new(
        ctx.kv_store.clone(),
        &client_request_bucket_key,
        RateLimiterConfig::new(CLIENT_RPC_RATE_PER_SECOND, CLIENT_RPC_BURST),
    );
    debug_assert!(!client_request_bucket_key.is_empty());
    const { assert!(CLIENT_RPC_BURST > 0) };

    match request_throttle.try_acquire().await {
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
            // Fail-open: allow the request when rate limiter storage is unavailable.
            // This matches the fail-open behavior for follower nodes (NotLeader errors
            // in the rate limiter). Blocking all requests due to a transient storage
            // hiccup creates a cascading failure — the cure is worse than the disease.
            warn!(client_id = %client_id, reason = %reason, "Rate limiter storage unavailable, allowing request (fail-open)");
            Ok(true)
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
    debug_assert!(!client_id.as_bytes().is_empty());
    debug_assert!(matches!(token, Some(_) | None));
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

fn handle_client_request_parse(
    buffer: &[u8],
) -> anyhow::Result<(ClientRpcRequest, Option<aspen_auth::CapabilityToken>, u8)> {
    use anyhow::Context;

    match postcard::from_bytes::<AuthenticatedRequest>(buffer) {
        Ok(auth_req) => Ok((auth_req.request, auth_req.token, auth_req.proxy_hops)),
        Err(_) => {
            let request = postcard::from_bytes(buffer).context("failed to deserialize Client request")?;
            Ok((request, None, 0))
        }
    }
}

async fn handle_client_request_check_initialized(
    ctx: &ClientProtocolContext,
    send: &mut iroh::endpoint::SendStream,
    is_bootstrap_operation: bool,
) -> anyhow::Result<bool> {
    if is_bootstrap_operation || ctx.controller.is_initialized() {
        return Ok(true);
    }

    handle_client_request_send_error(send, "NOT_INITIALIZED", "cluster not initialized - call InitCluster first")
        .await?;
    Ok(false)
}

#[cfg(feature = "blob")]
async fn handle_client_request_blob(
    send: &mut iroh::endpoint::SendStream,
    ctx: &ClientProtocolContext,
    request: &ClientRpcRequest,
) -> anyhow::Result<bool> {
    use anyhow::Context;

    let ClientRpcRequest::GetBlob { hash } = request else {
        return Ok(false);
    };

    match aspen_blob_handler::BlobHandler::handle_get_blob_streaming(ctx, hash.clone()).await {
        Ok(aspen_blob_handler::GetBlobOutcome::Stream { header, data }) => {
            let header_bytes = postcard::to_stdvec(&header).context("failed to serialize blob stream header")?;
            send.write_all(&header_bytes).await.context("failed to write blob stream header")?;
            send.write_all(&data).await.context("failed to stream blob data")?;
            send.finish().context("failed to finish blob stream")?;
        }
        Ok(aspen_blob_handler::GetBlobOutcome::Inline(resp) | aspen_blob_handler::GetBlobOutcome::Response(resp)) => {
            let response_bytes = postcard::to_stdvec(&resp).context("failed to serialize Client response")?;
            send.write_all(&response_bytes).await.context("failed to write Client response")?;
            send.finish().context("failed to finish send stream")?;
        }
        Err(err) => {
            warn!(error = %err, "blob get failed");
            let response = ClientRpcResponse::error("INTERNAL_ERROR", sanitize_error_for_client(&err));
            let response_bytes = postcard::to_stdvec(&response).context("failed to serialize error response")?;
            send.write_all(&response_bytes).await.context("failed to write error response")?;
            send.finish().context("failed to finish send stream")?;
        }
    }

    Ok(true)
}

#[cfg(not(feature = "blob"))]
async fn handle_client_request_blob(
    _send: &mut iroh::endpoint::SendStream,
    _ctx: &ClientProtocolContext,
    _request: &ClientRpcRequest,
) -> anyhow::Result<bool> {
    Ok(false)
}

async fn handle_client_request_dispatch(
    send: &mut iroh::endpoint::SendStream,
    registry: &HandlerRegistry,
    ctx: &ClientProtocolContext,
    request: ClientRpcRequest,
    proxy_hops: u8,
) -> anyhow::Result<()> {
    use anyhow::Context;

    let response = match registry.dispatch(request, ctx, proxy_hops).await {
        Ok(resp) => resp,
        Err(err) => {
            warn!(error = %err, "Client request processing failed");
            ClientRpcResponse::error("INTERNAL_ERROR", sanitize_error_for_client(&err))
        }
    };

    let response_bytes = postcard::to_stdvec(&response).context("failed to serialize Client response")?;
    send.write_all(&response_bytes).await.context("failed to write Client response")?;
    send.finish().context("failed to finish send stream")?;
    Ok(())
}

/// Handle a single Client RPC request on a stream.
async fn handle_client_request(
    (recv, send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    ctx: Arc<ClientProtocolContext>,
    client_id: iroh::PublicKey,
    registry: HandlerRegistry,
    request_id: u64,
) -> anyhow::Result<()> {
    handle_client_request_inner((recv, send), ctx, client_id, registry, request_id)
        .instrument(tracing::info_span!("handle_client_request", client_id = %client_id, request_id = request_id))
        .await
}

async fn handle_client_request_inner(
    (mut recv, mut send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    ctx: Arc<ClientProtocolContext>,
    client_id: iroh::PublicKey,
    registry: HandlerRegistry,
    request_id: u64,
) -> anyhow::Result<()> {
    use anyhow::Context;

    debug_assert!(!client_id.as_bytes().is_empty());
    debug_assert!(request_id < u64::MAX);
    let buffer = recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE).await.context("failed to read Client request")?;
    let (request, token, proxy_hops) = handle_client_request_parse(&buffer)?;

    debug!(request_type = ?request, client_id = %client_id, request_id = %request_id, has_token = token.is_some(), "received Client request");

    let is_bootstrap_operation = handle_client_request_is_bootstrap(&request);
    if !handle_client_request_check_initialized(&ctx, &mut send, is_bootstrap_operation).await? {
        return Ok(());
    }
    if !handle_client_request_is_rate_limit_exempt(&request)
        && !handle_client_request_check_rate_limit(&ctx, &client_id, &mut send).await?
    {
        return Ok(());
    }
    if !handle_client_request_check_auth(&ctx, &client_id, &request, &token, &mut send).await? {
        return Ok(());
    }

    #[cfg(feature = "deploy")]
    let _drain_guard = if !is_bootstrap_operation {
        if let Some(ref drain_state) = ctx.drain_state {
            if !drain_state.try_start_op() {
                debug!(client_id = %client_id, request_id = %request_id, "rejecting RPC during drain, returning NOT_LEADER");
                handle_client_request_send_error(
                    &mut send,
                    "NOT_LEADER",
                    "node is draining for upgrade, retry on another node",
                )
                .await?;
                return Ok(());
            }
            Some(DrainGuard {
                drain_state: Arc::clone(drain_state),
            })
        } else {
            None
        }
    } else {
        None
    };

    if !handle_client_request_blob(&mut send, &ctx, &request).await? {
        handle_client_request_dispatch(&mut send, &registry, &ctx, request, proxy_hops).await?;
    }

    let kv_store = ctx.kv_store.clone();
    tokio::spawn(async move {
        let counter = AtomicCounter::new(kv_store, CLIENT_RPC_REQUEST_COUNTER, CounterConfig {
            max_retries: 100,
            retry_delay_ms: 1,
        });
        if let Err(e) = counter.increment().await {
            debug!(error = %e, "Failed to increment request counter");
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use aspen_client_api::ClientRpcRequest;

    use super::handle_client_request_is_bootstrap;
    use super::handle_client_request_is_rate_limit_exempt;

    #[test]
    fn test_init_cluster_with_trust_is_bootstrap_operation() {
        let request = ClientRpcRequest::InitClusterWithTrust { threshold: None };
        assert!(handle_client_request_is_bootstrap(&request));
        assert!(handle_client_request_is_rate_limit_exempt(&request));
    }

    #[test]
    fn test_change_membership_remains_rate_limit_exempt() {
        let request = ClientRpcRequest::ChangeMembership { members: vec![1, 2, 3] };
        assert!(!handle_client_request_is_bootstrap(&request));
        assert!(handle_client_request_is_rate_limit_exempt(&request));
    }
}
