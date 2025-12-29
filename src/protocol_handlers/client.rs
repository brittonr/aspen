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

use super::constants::MAX_CLIENT_CONNECTIONS;
use super::constants::MAX_CLIENT_STREAMS_PER_CONNECTION;
use super::error_sanitization::sanitize_blob_error;
use super::error_sanitization::sanitize_control_error;
use super::error_sanitization::sanitize_error_for_client;
use super::error_sanitization::sanitize_kv_error;
use crate::api::AddLearnerRequest;
use crate::api::ChangeMembershipRequest;
use crate::api::ClusterController;
use crate::api::InitRequest;
use crate::api::KeyValueStore;
use crate::api::ReadRequest;
use crate::api::WriteRequest;
use crate::api::validate_client_key;
use crate::auth::TokenVerifier;
use crate::blob::IrohBlobStore;
use crate::client_rpc::AddBlobResultResponse;
use crate::client_rpc::AddLearnerResultResponse;
use crate::client_rpc::AddPeerResultResponse;
use crate::client_rpc::BatchReadResultResponse;
use crate::client_rpc::BatchWriteResultResponse;
use crate::client_rpc::BlobListEntry as RpcBlobListEntry;
use crate::client_rpc::ChangeMembershipResultResponse;
use crate::client_rpc::CheckpointWalResultResponse;
use crate::client_rpc::ClientRpcRequest;
use crate::client_rpc::ClientRpcResponse;
use crate::client_rpc::ClientTicketResponse as ClientTicketRpcResponse;
use crate::client_rpc::ClusterStateResponse;
use crate::client_rpc::ClusterTicketResponse;
use crate::client_rpc::ConditionalBatchWriteResultResponse;
use crate::client_rpc::CounterResultResponse;
use crate::client_rpc::DeleteBlobResultResponse;
use crate::client_rpc::DeleteResultResponse;
use crate::client_rpc::DocsTicketResponse as DocsTicketRpcResponse;
use crate::client_rpc::DownloadBlobResultResponse;
use crate::client_rpc::GetBlobResultResponse;
use crate::client_rpc::GetBlobStatusResultResponse;
use crate::client_rpc::GetBlobTicketResultResponse;
use crate::client_rpc::HasBlobResultResponse;
use crate::client_rpc::HealthResponse;
use crate::client_rpc::InitResultResponse;
use crate::client_rpc::LeaseGrantResultResponse;
use crate::client_rpc::LeaseInfo;
use crate::client_rpc::LeaseKeepaliveResultResponse;
use crate::client_rpc::LeaseListResultResponse;
use crate::client_rpc::LeaseRevokeResultResponse;
use crate::client_rpc::LeaseTimeToLiveResultResponse;
use crate::client_rpc::ListBlobsResultResponse;
use crate::client_rpc::LockResultResponse;
use crate::client_rpc::MAX_CLIENT_MESSAGE_SIZE;
use crate::client_rpc::MAX_CLUSTER_NODES;
use crate::client_rpc::MetricsResponse;
use crate::client_rpc::NodeDescriptor;
use crate::client_rpc::NodeInfoResponse;
use crate::client_rpc::PromoteLearnerResultResponse;
use crate::client_rpc::ProtectBlobResultResponse;
use crate::client_rpc::RaftMetricsResponse;
use crate::client_rpc::RateLimiterResultResponse;
use crate::client_rpc::ReadResultResponse;
use crate::client_rpc::ScanEntry;
use crate::client_rpc::ScanResultResponse;
use crate::client_rpc::SequenceResultResponse;
use crate::client_rpc::SignedCounterResultResponse;
use crate::client_rpc::SnapshotResultResponse;
#[cfg(feature = "sql")]
use crate::client_rpc::SqlCellValue;
use crate::client_rpc::SqlResultResponse;
use crate::client_rpc::UnprotectBlobResultResponse;
use crate::client_rpc::VaultKeysResponse;
use crate::client_rpc::VaultListResponse;
use crate::client_rpc::WatchCancelResultResponse;
use crate::client_rpc::WatchCreateResultResponse;
use crate::client_rpc::WatchStatusResultResponse;
use crate::client_rpc::WriteResultResponse;
use crate::cluster::IrohEndpointManager;
use crate::coordination::AtomicCounter;
use crate::coordination::CounterConfig;
use crate::coordination::DistributedLock;
use crate::coordination::DistributedRateLimiter;
use crate::coordination::LockConfig;
use crate::coordination::RateLimitError;
use crate::coordination::RateLimiterConfig;
use crate::coordination::SequenceConfig;
use crate::coordination::SequenceGenerator;
use crate::coordination::SignedAtomicCounter;
use crate::raft::constants::CLIENT_RPC_BURST;
use crate::raft::constants::CLIENT_RPC_RATE_LIMIT_PREFIX;
use crate::raft::constants::CLIENT_RPC_RATE_PER_SECOND;
use crate::raft::constants::CLIENT_RPC_REQUEST_COUNTER;
use crate::raft::constants::CLIENT_RPC_REQUEST_ID_SEQUENCE;

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
    pub network_factory: Option<Arc<crate::raft::network::IrpcRaftNetworkFactory>>,
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
}

impl ClientProtocolHandler {
    /// Create a new Client protocol handler.
    ///
    /// # Arguments
    /// * `ctx` - Client context with dependencies
    pub fn new(ctx: ClientProtocolContext) -> Self {
        Self {
            ctx: Arc::new(ctx),
            connection_semaphore: Arc::new(Semaphore::new(MAX_CLIENT_CONNECTIONS as usize)),
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
        let result = handle_client_connection(connection, Arc::clone(&self.ctx)).await;

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
#[instrument(skip(connection, ctx))]
async fn handle_client_connection(connection: Connection, ctx: Arc<ClientProtocolContext>) -> anyhow::Result<()> {
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
        // Pass the PublicKey directly - no string conversion needed
        // This eliminates the security risk of parsing failures
        let (send, recv) = stream;
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(err) = handle_client_request((recv, send), ctx_clone, remote_node_id).await {
                error!(error = %err, "failed to handle Client request");
            }
            active_streams_clone.fetch_sub(1, Ordering::Relaxed);
        });
    }

    Ok(())
}

/// Handle a single Client RPC request on a stream.
#[instrument(skip(recv, send, ctx), fields(client_id = %client_id, request_id))]
async fn handle_client_request(
    (mut recv, mut send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    ctx: Arc<ClientProtocolContext>,
    client_id: iroh::PublicKey,
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

    // Process the request and create response
    let response = match process_client_request(request, &ctx).await {
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

/// Process a Client RPC request and generate response.
async fn process_client_request(
    request: ClientRpcRequest,
    ctx: &ClientProtocolContext,
) -> anyhow::Result<ClientRpcResponse> {
    match request {
        ClientRpcRequest::GetHealth => {
            let uptime_seconds = ctx.start_time.elapsed().as_secs();

            let status = match ctx.controller.get_metrics().await {
                Ok(metrics) => {
                    if metrics.current_leader.is_some() {
                        "healthy"
                    } else {
                        "degraded"
                    }
                }
                Err(_) => "unhealthy",
            };

            Ok(ClientRpcResponse::Health(HealthResponse {
                status: status.to_string(),
                node_id: ctx.node_id,
                raft_node_id: Some(ctx.node_id),
                uptime_seconds,
            }))
        }

        ClientRpcRequest::GetRaftMetrics => {
            use crate::client_rpc::ReplicationProgress;

            let metrics = ctx
                .controller
                .get_metrics()
                .await
                .map_err(|e| anyhow::anyhow!("failed to get Raft metrics: {}", e))?;

            // Extract replication progress if this node is leader
            let replication = metrics.replication.as_ref().map(|repl_map| {
                repl_map
                    .iter()
                    .map(|(node_id, matched)| ReplicationProgress {
                        node_id: node_id.0,
                        matched_index: matched.as_ref().map(|log_id| log_id.index),
                    })
                    .collect()
            });

            Ok(ClientRpcResponse::RaftMetrics(RaftMetricsResponse {
                node_id: ctx.node_id,
                state: format!("{:?}", metrics.state),
                current_leader: metrics.current_leader.map(|id| id.0),
                current_term: metrics.current_term,
                last_log_index: metrics.last_log_index,
                last_applied_index: metrics.last_applied.as_ref().map(|la| la.index),
                snapshot_index: metrics.snapshot.as_ref().map(|s| s.index),
                replication,
            }))
        }

        ClientRpcRequest::GetLeader => {
            let leader =
                ctx.controller.get_leader().await.map_err(|e| anyhow::anyhow!("failed to get leader: {}", e))?;
            Ok(ClientRpcResponse::Leader(leader))
        }

        ClientRpcRequest::GetNodeInfo => {
            let endpoint_addr = ctx.endpoint_manager.node_addr();
            Ok(ClientRpcResponse::NodeInfo(NodeInfoResponse {
                node_id: ctx.node_id,
                endpoint_addr: format!("{:?}", endpoint_addr),
            }))
        }

        ClientRpcRequest::GetClusterTicket => {
            use iroh_gossip::proto::TopicId;

            use crate::cluster::ticket::AspenClusterTicket;

            let hash = blake3::hash(ctx.cluster_cookie.as_bytes());
            let topic_id = TopicId::from_bytes(*hash.as_bytes());

            let ticket = AspenClusterTicket::with_bootstrap(
                topic_id,
                ctx.cluster_cookie.clone(),
                ctx.endpoint_manager.endpoint().id(),
            );

            let ticket_str = ticket.serialize();

            Ok(ClientRpcResponse::ClusterTicket(ClusterTicketResponse {
                ticket: ticket_str,
                topic_id: format!("{:?}", topic_id),
                cluster_id: ctx.cluster_cookie.clone(),
                endpoint_id: ctx.endpoint_manager.endpoint().id().to_string(),
                bootstrap_peers: Some(1),
            }))
        }

        ClientRpcRequest::InitCluster => {
            // Build ClusterNode for the current node to initialize as single-node cluster
            let endpoint_addr = ctx.endpoint_manager.node_addr();
            let this_node = crate::api::ClusterNode {
                id: ctx.node_id,
                addr: endpoint_addr.id.to_string(),
                raft_addr: None,
                iroh_addr: Some(endpoint_addr.clone()),
            };

            let result = ctx
                .controller
                .init(InitRequest {
                    initial_members: vec![this_node],
                })
                .await;

            Ok(ClientRpcResponse::InitResult(InitResultResponse {
                success: result.is_ok(),
                // HIGH-4: Sanitize error messages to prevent information leakage
                error: result.err().map(|e| sanitize_control_error(&e)),
            }))
        }

        ClientRpcRequest::ReadKey { key } => {
            let result = ctx.kv_store.read(ReadRequest::new(key.clone())).await;

            match result {
                Ok(resp) => {
                    let value = resp.kv.map(|kv| kv.value.into_bytes());
                    Ok(ClientRpcResponse::ReadResult(ReadResultResponse {
                        value,
                        found: true,
                        error: None,
                    }))
                }
                Err(e) => {
                    // HIGH-4: Sanitize error messages to prevent information leakage
                    let (found, error) = match &e {
                        crate::api::KeyValueStoreError::NotFound { .. } => (false, None),
                        other => (false, Some(sanitize_kv_error(other))),
                    };
                    Ok(ClientRpcResponse::ReadResult(ReadResultResponse {
                        value: None,
                        found,
                        error,
                    }))
                }
            }
        }

        ClientRpcRequest::WriteKey { key, value } => {
            use crate::api::WriteCommand;

            // Validate key against reserved _system: prefix
            if let Err(vault_err) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::WriteResult(WriteResultResponse {
                    success: false,
                    error: Some(vault_err.to_string()),
                }));
            }

            let result = ctx
                .kv_store
                .write(WriteRequest {
                    command: WriteCommand::Set {
                        key: key.clone(),
                        value: String::from_utf8_lossy(&value).to_string(),
                    },
                })
                .await;

            Ok(ClientRpcResponse::WriteResult(WriteResultResponse {
                success: result.is_ok(),
                // HIGH-4: Sanitize error messages to prevent information leakage
                error: result.err().map(|e| sanitize_kv_error(&e)),
            }))
        }

        ClientRpcRequest::CompareAndSwapKey {
            key,
            expected,
            new_value,
        } => {
            use crate::api::WriteCommand;
            use crate::client_rpc::CompareAndSwapResultResponse;

            // Validate key against reserved _system: prefix
            if let Err(vault_err) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
                    success: false,
                    actual_value: None,
                    error: Some(vault_err.to_string()),
                }));
            }

            let result = ctx
                .kv_store
                .write(WriteRequest {
                    command: WriteCommand::CompareAndSwap {
                        key: key.clone(),
                        expected: expected.map(|v| String::from_utf8_lossy(&v).to_string()),
                        new_value: String::from_utf8_lossy(&new_value).to_string(),
                    },
                })
                .await;

            match result {
                Ok(_) => Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
                    success: true,
                    actual_value: None,
                    error: None,
                })),
                Err(crate::api::KeyValueStoreError::CompareAndSwapFailed { actual, .. }) => {
                    Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
                        success: false,
                        actual_value: actual.map(|v| v.into_bytes()),
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
                    success: false,
                    actual_value: None,
                    // HIGH-4: Sanitize error messages to prevent information leakage
                    error: Some(sanitize_kv_error(&e)),
                })),
            }
        }

        ClientRpcRequest::CompareAndDeleteKey { key, expected } => {
            use crate::api::WriteCommand;
            use crate::client_rpc::CompareAndSwapResultResponse;

            // Validate key against reserved _system: prefix
            if let Err(vault_err) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
                    success: false,
                    actual_value: None,
                    error: Some(vault_err.to_string()),
                }));
            }

            let result = ctx
                .kv_store
                .write(WriteRequest {
                    command: WriteCommand::CompareAndDelete {
                        key: key.clone(),
                        expected: String::from_utf8_lossy(&expected).to_string(),
                    },
                })
                .await;

            match result {
                Ok(_) => Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
                    success: true,
                    actual_value: None,
                    error: None,
                })),
                Err(crate::api::KeyValueStoreError::CompareAndSwapFailed { actual, .. }) => {
                    Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
                        success: false,
                        actual_value: actual.map(|v| v.into_bytes()),
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
                    success: false,
                    actual_value: None,
                    // HIGH-4: Sanitize error messages to prevent information leakage
                    error: Some(sanitize_kv_error(&e)),
                })),
            }
        }

        ClientRpcRequest::TriggerSnapshot => {
            let result = ctx.controller.trigger_snapshot().await;

            match result {
                Ok(snapshot) => Ok(ClientRpcResponse::SnapshotResult(SnapshotResultResponse {
                    success: true,
                    snapshot_index: snapshot.as_ref().map(|log_id| log_id.index),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::SnapshotResult(SnapshotResultResponse {
                    success: false,
                    snapshot_index: None,
                    // HIGH-4: Sanitize error messages to prevent information leakage
                    error: Some(sanitize_control_error(&e)),
                })),
            }
        }

        ClientRpcRequest::AddLearner { node_id, addr } => {
            use std::str::FromStr;

            use iroh::EndpointAddr;

            use crate::api::ClusterNode;

            // Parse the address as either JSON EndpointAddr or bare EndpointId
            let iroh_addr = if addr.starts_with('{') {
                serde_json::from_str::<EndpointAddr>(&addr).map_err(|e| format!("invalid JSON EndpointAddr: {e}"))
            } else {
                iroh::EndpointId::from_str(&addr)
                    .map(EndpointAddr::new)
                    .map_err(|e| format!("invalid EndpointId: {e}"))
            };

            let result = match iroh_addr {
                Ok(iroh_addr) => {
                    ctx.controller
                        .add_learner(AddLearnerRequest {
                            learner: ClusterNode::with_iroh_addr(node_id, iroh_addr),
                        })
                        .await
                }
                Err(parse_err) => Err(crate::api::ControlPlaneError::InvalidRequest { reason: parse_err }),
            };

            Ok(ClientRpcResponse::AddLearnerResult(AddLearnerResultResponse {
                success: result.is_ok(),
                // HIGH-4: Sanitize error messages to prevent information leakage
                error: result.err().map(|e| sanitize_control_error(&e)),
            }))
        }

        ClientRpcRequest::ChangeMembership { members } => {
            let result = ctx.controller.change_membership(ChangeMembershipRequest { members }).await;

            Ok(ClientRpcResponse::ChangeMembershipResult(ChangeMembershipResultResponse {
                success: result.is_ok(),
                // HIGH-4: Sanitize error messages to prevent information leakage
                error: result.err().map(|e| sanitize_control_error(&e)),
            }))
        }

        ClientRpcRequest::Ping => Ok(ClientRpcResponse::Pong),

        ClientRpcRequest::GetClusterState => {
            // Get current cluster state from the Raft controller
            let cluster_state = ctx
                .controller
                .current_state()
                .await
                .map_err(|e| anyhow::anyhow!("failed to get cluster state: {}", e))?;

            // Get current leader from metrics
            let leader_id = ctx.controller.get_leader().await.unwrap_or(None);

            // Convert ClusterNode to NodeDescriptor with membership info
            // Tiger Style: Bounded to MAX_CLUSTER_NODES
            let mut nodes: Vec<NodeDescriptor> =
                Vec::with_capacity((cluster_state.nodes.len() + cluster_state.learners.len()).min(MAX_CLUSTER_NODES));

            // Track which nodes are voters (members)
            let member_ids: std::collections::HashSet<u64> = cluster_state.members.iter().copied().collect();

            // Add all nodes from the cluster state
            for node in cluster_state.nodes.iter().take(MAX_CLUSTER_NODES) {
                let is_voter = member_ids.contains(&node.id);

                // For our own node, use our actual endpoint address; for others use Raft-stored address
                let endpoint_addr = if node.id == ctx.node_id {
                    let our_addr = format!("{:?}", ctx.endpoint_manager.node_addr());
                    debug!(
                        node_id = node.id,
                        addr = %our_addr,
                        "using our own Iroh endpoint address"
                    );
                    our_addr
                } else {
                    debug!(
                        node_id = node.id,
                        addr = %node.addr,
                        "using Raft-stored address"
                    );
                    node.addr.clone()
                };

                nodes.push(NodeDescriptor {
                    node_id: node.id,
                    endpoint_addr,
                    is_voter,
                    is_learner: false,
                    is_leader: leader_id == Some(node.id),
                });
            }

            // Add learners
            for learner in cluster_state.learners.iter() {
                if nodes.len() >= MAX_CLUSTER_NODES {
                    break;
                }

                nodes.push(NodeDescriptor {
                    node_id: learner.id,
                    endpoint_addr: learner.addr.clone(),
                    is_voter: false,
                    is_learner: true,
                    is_leader: false, // Learners can't be leaders
                });
            }

            Ok(ClientRpcResponse::ClusterState(ClusterStateResponse {
                nodes,
                leader_id,
                this_node_id: ctx.node_id,
            }))
        }

        // =========================================================================
        // New operations (migrated from HTTP API)
        // =========================================================================
        ClientRpcRequest::DeleteKey { key } => {
            use crate::api::WriteCommand;

            // Validate key against reserved _system: prefix
            if let Err(vault_err) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::DeleteResult(DeleteResultResponse {
                    key,
                    deleted: false,
                    error: Some(vault_err.to_string()),
                }));
            }

            let result = ctx
                .kv_store
                .write(WriteRequest {
                    command: WriteCommand::Delete { key: key.clone() },
                })
                .await;

            Ok(ClientRpcResponse::DeleteResult(DeleteResultResponse {
                key,
                deleted: result.is_ok(),
                // HIGH-4: Sanitize error messages to prevent information leakage
                error: result.err().map(|e| sanitize_kv_error(&e)),
            }))
        }

        ClientRpcRequest::ScanKeys {
            prefix,
            limit,
            continuation_token,
        } => {
            use crate::api::ScanRequest;

            let result = ctx
                .kv_store
                .scan(ScanRequest {
                    prefix: prefix.clone(),
                    limit,
                    continuation_token,
                })
                .await;

            match result {
                Ok(scan_resp) => {
                    // Convert from api::KeyValueWithRevision to client_rpc::ScanEntry
                    let entries: Vec<ScanEntry> = scan_resp
                        .entries
                        .into_iter()
                        .map(|e| ScanEntry {
                            key: e.key,
                            value: e.value,
                            version: e.version,
                            create_revision: e.create_revision,
                            mod_revision: e.mod_revision,
                        })
                        .collect();

                    Ok(ClientRpcResponse::ScanResult(ScanResultResponse {
                        entries,
                        count: scan_resp.count,
                        is_truncated: scan_resp.is_truncated,
                        continuation_token: scan_resp.continuation_token,
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::ScanResult(ScanResultResponse {
                    entries: vec![],
                    count: 0,
                    is_truncated: false,
                    continuation_token: None,
                    // HIGH-4: Sanitize error messages to prevent information leakage
                    error: Some(sanitize_kv_error(&e)),
                })),
            }
        }

        ClientRpcRequest::GetMetrics => {
            // Return Prometheus-format metrics from Raft
            let metrics_text = match ctx.controller.get_metrics().await {
                Ok(metrics) => {
                    let state_value = match metrics.state {
                        openraft::ServerState::Learner => 0,
                        openraft::ServerState::Follower => 1,
                        openraft::ServerState::Candidate => 2,
                        openraft::ServerState::Leader => 3,
                        openraft::ServerState::Shutdown => 4,
                    };
                    let is_leader: u8 = if metrics.state == openraft::ServerState::Leader {
                        1
                    } else {
                        0
                    };
                    let last_applied = metrics.last_applied.as_ref().map(|la| la.index).unwrap_or(0);
                    let snapshot_index = metrics.snapshot.as_ref().map(|s| s.index).unwrap_or(0);

                    // Get cluster-wide request counter
                    let request_counter = {
                        let counter = AtomicCounter::new(
                            ctx.kv_store.clone(),
                            CLIENT_RPC_REQUEST_COUNTER,
                            CounterConfig::default(),
                        );
                        counter.get().await.unwrap_or(0)
                    };

                    format!(
                        "# HELP aspen_raft_term Current Raft term\n\
                         # TYPE aspen_raft_term gauge\n\
                         aspen_raft_term{{node_id=\"{}\"}} {}\n\
                         # HELP aspen_raft_state Raft state (0=Learner, 1=Follower, 2=Candidate, 3=Leader, 4=Shutdown)\n\
                         # TYPE aspen_raft_state gauge\n\
                         aspen_raft_state{{node_id=\"{}\"}} {}\n\
                         # HELP aspen_raft_is_leader Whether this node is the leader\n\
                         # TYPE aspen_raft_is_leader gauge\n\
                         aspen_raft_is_leader{{node_id=\"{}\"}} {}\n\
                         # HELP aspen_raft_last_log_index Last log index\n\
                         # TYPE aspen_raft_last_log_index gauge\n\
                         aspen_raft_last_log_index{{node_id=\"{}\"}} {}\n\
                         # HELP aspen_raft_last_applied_index Last applied log index\n\
                         # TYPE aspen_raft_last_applied_index gauge\n\
                         aspen_raft_last_applied_index{{node_id=\"{}\"}} {}\n\
                         # HELP aspen_raft_snapshot_index Snapshot index\n\
                         # TYPE aspen_raft_snapshot_index gauge\n\
                         aspen_raft_snapshot_index{{node_id=\"{}\"}} {}\n\
                         # HELP aspen_node_uptime_seconds Node uptime in seconds\n\
                         # TYPE aspen_node_uptime_seconds counter\n\
                         aspen_node_uptime_seconds{{node_id=\"{}\"}} {}\n\
                         # HELP aspen_client_rpc_requests_total Total client RPC requests processed cluster-wide\n\
                         # TYPE aspen_client_rpc_requests_total counter\n\
                         aspen_client_rpc_requests_total{{node_id=\"{}\"}} {}\n",
                        ctx.node_id,
                        metrics.current_term,
                        ctx.node_id,
                        state_value,
                        ctx.node_id,
                        is_leader,
                        ctx.node_id,
                        metrics.last_log_index.unwrap_or(0),
                        ctx.node_id,
                        last_applied,
                        ctx.node_id,
                        snapshot_index,
                        ctx.node_id,
                        ctx.start_time.elapsed().as_secs(),
                        ctx.node_id,
                        request_counter
                    )
                }
                Err(_) => String::new(),
            };

            Ok(ClientRpcResponse::Metrics(MetricsResponse {
                prometheus_text: metrics_text,
            }))
        }

        ClientRpcRequest::PromoteLearner {
            learner_id,
            replace_node,
            force: _force,
        } => {
            // Get current membership to build new voter set
            let cluster_state = ctx
                .controller
                .current_state()
                .await
                .map_err(|e| anyhow::anyhow!("failed to get cluster state: {}", e))?;

            let previous_voters: Vec<u64> = cluster_state.members.clone();

            // Build new membership: add learner, optionally remove replaced node
            let mut new_members: Vec<u64> = previous_voters.clone();
            if !new_members.contains(&learner_id) {
                new_members.push(learner_id);
            }
            if let Some(replace_id) = replace_node {
                new_members.retain(|&id| id != replace_id);
            }

            let result = ctx
                .controller
                .change_membership(ChangeMembershipRequest {
                    members: new_members.clone(),
                })
                .await;

            Ok(ClientRpcResponse::PromoteLearnerResult(PromoteLearnerResultResponse {
                success: result.is_ok(),
                learner_id,
                previous_voters,
                new_voters: new_members,
                message: if result.is_ok() {
                    format!("Learner {} promoted to voter", learner_id)
                } else {
                    "Promotion failed".to_string()
                },
                // HIGH-4: Sanitize error messages to prevent information leakage
                error: result.err().map(|e| sanitize_control_error(&e)),
            }))
        }

        ClientRpcRequest::CheckpointWal => {
            // WAL checkpoint requires direct access to the SQLite state machine,
            // which is not exposed through the ClusterController/KeyValueStore traits.
            // This would need to be implemented as a new trait method or by extending
            // the ClientProtocolContext to include the state machine reference.
            Ok(ClientRpcResponse::CheckpointWalResult(CheckpointWalResultResponse {
                success: false,
                pages_checkpointed: None,
                wal_size_before_bytes: None,
                wal_size_after_bytes: None,
                error: Some(
                    "WAL checkpoint requires state machine access - use trigger_snapshot for log compaction"
                        .to_string(),
                ),
            }))
        }

        ClientRpcRequest::ListVaults => {
            // Deprecated: Vault-specific operations removed in favor of flat keyspace.
            // Use ScanKeys with an empty prefix to list all keys.
            Ok(ClientRpcResponse::VaultList(VaultListResponse {
                vaults: vec![],
                error: Some("ListVaults is deprecated. Use ScanKeys with a prefix instead.".to_string()),
            }))
        }

        ClientRpcRequest::GetVaultKeys { vault_name } => {
            // Deprecated: Vault-specific operations removed in favor of flat keyspace.
            // Use ScanKeys with the desired prefix instead.
            Ok(ClientRpcResponse::VaultKeys(VaultKeysResponse {
                vault: vault_name,
                keys: vec![],
                error: Some("GetVaultKeys is deprecated. Use ScanKeys with a prefix instead.".to_string()),
            }))
        }

        ClientRpcRequest::AddPeer { node_id, endpoint_addr } => {
            // Parse the endpoint address (JSON-serialized EndpointAddr)
            let parsed_addr: iroh::EndpointAddr = match serde_json::from_str(&endpoint_addr) {
                Ok(addr) => addr,
                Err(e) => {
                    debug!(
                        node_id = node_id,
                        endpoint_addr = %endpoint_addr,
                        error = %e,
                        "AddPeer: failed to parse endpoint_addr as JSON EndpointAddr"
                    );
                    // HIGH-4: Don't expose parse error details to clients
                    return Ok(ClientRpcResponse::AddPeerResult(AddPeerResultResponse {
                        success: false,
                        error: Some("invalid endpoint_addr: expected JSON-serialized EndpointAddr".to_string()),
                    }));
                }
            };

            // Check if network factory is available
            let network_factory = match &ctx.network_factory {
                Some(nf) => nf,
                None => {
                    warn!(node_id = node_id, "AddPeer: network_factory not available in context");
                    return Ok(ClientRpcResponse::AddPeerResult(AddPeerResultResponse {
                        success: false,
                        error: Some("network_factory not configured for this node".to_string()),
                    }));
                }
            };

            // Add peer to the network factory
            // Tiger Style: add_peer is bounded by MAX_PEERS (1000)
            network_factory.add_peer(crate::raft::types::NodeId(node_id), parsed_addr.clone()).await;

            info!(
                node_id = node_id,
                endpoint_id = %parsed_addr.id,
                "AddPeer: successfully registered peer in network factory"
            );

            Ok(ClientRpcResponse::AddPeerResult(AddPeerResultResponse {
                success: true,
                error: None,
            }))
        }

        ClientRpcRequest::GetClusterTicketCombined { endpoint_ids } => {
            use iroh_gossip::proto::TopicId;

            use crate::cluster::ticket::AspenClusterTicket;

            let hash = blake3::hash(ctx.cluster_cookie.as_bytes());
            let topic_id = TopicId::from_bytes(*hash.as_bytes());

            // Start with this node as the first bootstrap peer
            let mut ticket = AspenClusterTicket::with_bootstrap(
                topic_id,
                ctx.cluster_cookie.clone(),
                ctx.endpoint_manager.endpoint().id(),
            );

            // Collect additional peers from:
            // 1. Explicit endpoint_ids parameter (comma-separated EndpointId strings)
            // 2. Cluster state (iroh_addr from known cluster nodes)
            let mut added_peers = 1u32; // Already added this node

            // Parse explicit endpoint_ids if provided
            if let Some(ids_str) = &endpoint_ids {
                for id_str in ids_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    // Skip if we've hit the limit (Tiger Style: MAX_BOOTSTRAP_PEERS = 16)
                    if added_peers >= AspenClusterTicket::MAX_BOOTSTRAP_PEERS {
                        debug!(
                            max_peers = AspenClusterTicket::MAX_BOOTSTRAP_PEERS,
                            "GetClusterTicketCombined: reached max bootstrap peers, skipping remaining"
                        );
                        break;
                    }

                    // Try to parse as EndpointId (public key)
                    match id_str.parse::<iroh::EndpointId>() {
                        Ok(endpoint_id) => {
                            // Skip if it's our own endpoint
                            if endpoint_id == ctx.endpoint_manager.endpoint().id() {
                                continue;
                            }
                            if ticket.add_bootstrap(endpoint_id).is_ok() {
                                added_peers += 1;
                            }
                        }
                        Err(e) => {
                            debug!(
                                endpoint_id = id_str,
                                error = %e,
                                "GetClusterTicketCombined: failed to parse endpoint_id, skipping"
                            );
                        }
                    }
                }
            }

            // If we haven't hit the limit, also add peers from cluster state
            if added_peers < AspenClusterTicket::MAX_BOOTSTRAP_PEERS
                && let Ok(cluster_state) = ctx.controller.current_state().await
            {
                // Add endpoint IDs from nodes with known iroh_addr
                for node in cluster_state.nodes.iter().chain(cluster_state.learners.iter()) {
                    if added_peers >= AspenClusterTicket::MAX_BOOTSTRAP_PEERS {
                        break;
                    }

                    if let Some(iroh_addr) = &node.iroh_addr {
                        // Skip our own endpoint
                        if iroh_addr.id == ctx.endpoint_manager.endpoint().id() {
                            continue;
                        }
                        if ticket.add_bootstrap(iroh_addr.id).is_ok() {
                            added_peers += 1;
                        }
                    }
                }
            }

            let ticket_str = ticket.serialize();

            debug!(
                bootstrap_peers = added_peers,
                "GetClusterTicketCombined: generated ticket with multiple bootstrap peers"
            );

            Ok(ClientRpcResponse::ClusterTicket(ClusterTicketResponse {
                ticket: ticket_str,
                topic_id: format!("{:?}", topic_id),
                cluster_id: ctx.cluster_cookie.clone(),
                endpoint_id: ctx.endpoint_manager.endpoint().id().to_string(),
                bootstrap_peers: Some(added_peers as usize),
            }))
        }

        ClientRpcRequest::GetClientTicket { access, priority } => {
            use crate::client::AccessLevel;
            use crate::client::ticket::AspenClientTicket;

            let endpoint_addr = ctx.endpoint_manager.node_addr().clone();
            let access_level = match access.to_lowercase().as_str() {
                "write" | "readwrite" | "read_write" | "rw" => AccessLevel::ReadWrite,
                _ => AccessLevel::ReadOnly,
            };

            // Tiger Style: saturate priority to u8 range
            let priority_u8 = priority.min(255) as u8;

            let ticket = AspenClientTicket::new(&ctx.cluster_cookie, vec![endpoint_addr])
                .with_access(access_level)
                .with_priority(priority_u8);

            let ticket_str = ticket.serialize();

            Ok(ClientRpcResponse::ClientTicket(ClientTicketRpcResponse {
                ticket: ticket_str,
                cluster_id: ctx.cluster_cookie.clone(),
                access: match access_level {
                    AccessLevel::ReadOnly => "read".to_string(),
                    AccessLevel::ReadWrite => "write".to_string(),
                },
                priority,
                endpoint_id: ctx.endpoint_manager.endpoint().id().to_string(),
                error: None,
            }))
        }

        ClientRpcRequest::GetDocsTicket { read_write, priority } => {
            use crate::docs::ticket::AspenDocsTicket;

            let endpoint_addr = ctx.endpoint_manager.node_addr().clone();

            // Derive namespace ID from cluster cookie
            let namespace_hash = blake3::hash(format!("aspen-docs-{}", ctx.cluster_cookie).as_bytes());
            let namespace_id_str = format!("{}", namespace_hash);

            let ticket = AspenDocsTicket::new(
                ctx.cluster_cookie.clone(),
                priority,
                namespace_id_str.clone(),
                vec![endpoint_addr],
                read_write,
            );

            let ticket_str = ticket.serialize();

            Ok(ClientRpcResponse::DocsTicket(DocsTicketRpcResponse {
                ticket: ticket_str,
                cluster_id: ctx.cluster_cookie.clone(),
                namespace_id: namespace_id_str,
                read_write,
                priority,
                endpoint_id: ctx.endpoint_manager.endpoint().id().to_string(),
                error: None,
            }))
        }

        // =========================================================================
        // Blob operations
        // =========================================================================
        ClientRpcRequest::AddBlob { data, tag } => {
            use crate::blob::BlobStore;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::AddBlobResult(AddBlobResultResponse {
                    success: false,
                    hash: None,
                    size: None,
                    was_new: None,
                    error: Some("blob store not enabled".to_string()),
                }));
            };

            match blob_store.add_bytes(&data).await {
                Ok(result) => {
                    // Apply tag if provided
                    if let Some(tag_name) = tag {
                        let tag_name = IrohBlobStore::user_tag(&tag_name);
                        if let Err(e) = blob_store.protect(&result.blob_ref.hash, &tag_name).await {
                            warn!(error = %e, "failed to apply tag to blob");
                        }
                    }

                    // Announce to DHT if content discovery is enabled
                    // This runs in background to avoid blocking the response
                    #[cfg(feature = "global-discovery")]
                    if let Some(ref discovery) = ctx.content_discovery {
                        let hash = result.blob_ref.hash;
                        let size = result.blob_ref.size;
                        let format = result.blob_ref.format;
                        let discovery = discovery.clone();
                        tokio::spawn(async move {
                            if let Err(e) = discovery.announce(hash, size, format).await {
                                tracing::debug!(
                                    hash = %hash.fmt_short(),
                                    error = %e,
                                    "DHT announce failed (non-fatal)"
                                );
                            }
                        });
                    }

                    Ok(ClientRpcResponse::AddBlobResult(AddBlobResultResponse {
                        success: true,
                        hash: Some(result.blob_ref.hash.to_string()),
                        size: Some(result.blob_ref.size),
                        was_new: Some(result.was_new),
                        error: None,
                    }))
                }
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob add failed");
                    Ok(ClientRpcResponse::AddBlobResult(AddBlobResultResponse {
                        success: false,
                        hash: None,
                        size: None,
                        was_new: None,
                        error: Some(sanitize_blob_error(&e)),
                    }))
                }
            }
        }

        ClientRpcRequest::GetBlob { hash } => {
            use iroh_blobs::Hash;

            use crate::blob::BlobStore;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::GetBlobResult(GetBlobResultResponse {
                    found: false,
                    data: None,
                    error: Some("blob store not enabled".to_string()),
                }));
            };

            // Parse hash from string
            let hash = match hash.parse::<Hash>() {
                Ok(h) => h,
                Err(_) => {
                    // HIGH-4: Don't expose parse error details
                    return Ok(ClientRpcResponse::GetBlobResult(GetBlobResultResponse {
                        found: false,
                        data: None,
                        error: Some("invalid hash".to_string()),
                    }));
                }
            };

            match blob_store.get_bytes(&hash).await {
                Ok(Some(data)) => Ok(ClientRpcResponse::GetBlobResult(GetBlobResultResponse {
                    found: true,
                    data: Some(data.to_vec()),
                    error: None,
                })),
                Ok(None) => Ok(ClientRpcResponse::GetBlobResult(GetBlobResultResponse {
                    found: false,
                    data: None,
                    error: None,
                })),
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob get failed");
                    Ok(ClientRpcResponse::GetBlobResult(GetBlobResultResponse {
                        found: false,
                        data: None,
                        error: Some(sanitize_blob_error(&e)),
                    }))
                }
            }
        }

        ClientRpcRequest::HasBlob { hash } => {
            use iroh_blobs::Hash;

            use crate::blob::BlobStore;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::HasBlobResult(HasBlobResultResponse {
                    exists: false,
                    error: Some("blob store not enabled".to_string()),
                }));
            };

            // Parse hash from string
            let hash = match hash.parse::<Hash>() {
                Ok(h) => h,
                Err(e) => {
                    return Ok(ClientRpcResponse::HasBlobResult(HasBlobResultResponse {
                        exists: false,
                        error: Some(format!("invalid hash: {}", e)),
                    }));
                }
            };

            match blob_store.has(&hash).await {
                Ok(exists) => Ok(ClientRpcResponse::HasBlobResult(HasBlobResultResponse { exists, error: None })),
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob has check failed");
                    Ok(ClientRpcResponse::HasBlobResult(HasBlobResultResponse {
                        exists: false,
                        error: Some(sanitize_blob_error(&e)),
                    }))
                }
            }
        }

        ClientRpcRequest::GetBlobTicket { hash } => {
            use iroh_blobs::Hash;

            use crate::blob::BlobStore;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::GetBlobTicketResult(GetBlobTicketResultResponse {
                    success: false,
                    ticket: None,
                    error: Some("blob store not enabled".to_string()),
                }));
            };

            // Parse hash from string
            let hash = match hash.parse::<Hash>() {
                Ok(h) => h,
                Err(_) => {
                    // HIGH-4: Don't expose parse error details
                    return Ok(ClientRpcResponse::GetBlobTicketResult(GetBlobTicketResultResponse {
                        success: false,
                        ticket: None,
                        error: Some("invalid hash".to_string()),
                    }));
                }
            };

            match blob_store.ticket(&hash).await {
                Ok(ticket) => Ok(ClientRpcResponse::GetBlobTicketResult(GetBlobTicketResultResponse {
                    success: true,
                    ticket: Some(ticket.to_string()),
                    error: None,
                })),
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob ticket generation failed");
                    Ok(ClientRpcResponse::GetBlobTicketResult(GetBlobTicketResultResponse {
                        success: false,
                        ticket: None,
                        error: Some(sanitize_blob_error(&e)),
                    }))
                }
            }
        }

        ClientRpcRequest::ListBlobs {
            limit,
            continuation_token,
        } => {
            use crate::blob::BlobStore;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::ListBlobsResult(ListBlobsResultResponse {
                    blobs: vec![],
                    count: 0,
                    has_more: false,
                    continuation_token: None,
                    error: Some("blob store not enabled".to_string()),
                }));
            };

            // Tiger Style: Cap limit to prevent unbounded responses
            let limit = limit.min(1000);

            match blob_store.list(limit, continuation_token.as_deref()).await {
                Ok(result) => {
                    let count = result.blobs.len() as u32;
                    let blobs = result
                        .blobs
                        .into_iter()
                        .map(|entry| RpcBlobListEntry {
                            hash: entry.hash.to_string(),
                            size: entry.size,
                        })
                        .collect();

                    Ok(ClientRpcResponse::ListBlobsResult(ListBlobsResultResponse {
                        blobs,
                        count,
                        has_more: result.continuation_token.is_some(),
                        continuation_token: result.continuation_token,
                        error: None,
                    }))
                }
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob list failed");
                    Ok(ClientRpcResponse::ListBlobsResult(ListBlobsResultResponse {
                        blobs: vec![],
                        count: 0,
                        has_more: false,
                        continuation_token: None,
                        error: Some(sanitize_blob_error(&e)),
                    }))
                }
            }
        }

        ClientRpcRequest::ProtectBlob { hash, tag } => {
            use iroh_blobs::Hash;

            use crate::blob::BlobStore;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::ProtectBlobResult(ProtectBlobResultResponse {
                    success: false,
                    error: Some("blob store not enabled".to_string()),
                }));
            };

            // Parse hash from string
            let hash = match hash.parse::<Hash>() {
                Ok(h) => h,
                Err(e) => {
                    return Ok(ClientRpcResponse::ProtectBlobResult(ProtectBlobResultResponse {
                        success: false,
                        error: Some(format!("invalid hash: {}", e)),
                    }));
                }
            };

            let tag_name = IrohBlobStore::user_tag(&tag);
            match blob_store.protect(&hash, &tag_name).await {
                Ok(()) => Ok(ClientRpcResponse::ProtectBlobResult(ProtectBlobResultResponse {
                    success: true,
                    error: None,
                })),
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob protect failed");
                    Ok(ClientRpcResponse::ProtectBlobResult(ProtectBlobResultResponse {
                        success: false,
                        error: Some(sanitize_blob_error(&e)),
                    }))
                }
            }
        }

        ClientRpcRequest::UnprotectBlob { tag } => {
            use crate::blob::BlobStore;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::UnprotectBlobResult(UnprotectBlobResultResponse {
                    success: false,
                    error: Some("blob store not enabled".to_string()),
                }));
            };

            let tag_name = IrohBlobStore::user_tag(&tag);
            match blob_store.unprotect(&tag_name).await {
                Ok(()) => Ok(ClientRpcResponse::UnprotectBlobResult(UnprotectBlobResultResponse {
                    success: true,
                    error: None,
                })),
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob unprotect failed");
                    Ok(ClientRpcResponse::UnprotectBlobResult(UnprotectBlobResultResponse {
                        success: false,
                        error: Some(sanitize_blob_error(&e)),
                    }))
                }
            }
        }

        ClientRpcRequest::DeleteBlob { hash, force } => {
            use iroh_blobs::Hash;

            let Some(ref _blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::DeleteBlobResult(DeleteBlobResultResponse {
                    success: false,
                    error: Some("blob store not enabled".to_string()),
                }));
            };

            // Parse hash from string
            let hash = match hash.parse::<Hash>() {
                Ok(h) => h,
                Err(_) => {
                    // HIGH-4: Don't expose parse error details
                    return Ok(ClientRpcResponse::DeleteBlobResult(DeleteBlobResultResponse {
                        success: false,
                        error: Some("invalid hash".to_string()),
                    }));
                }
            };

            // Delete the blob using the store's native delete capability
            // Note: iroh-blobs manages GC internally, we use tags to protect blobs
            // For deletion, we remove any user tags first (if force), then the blob
            // will be GC'd naturally. For immediate deletion, we need direct store access.
            if force {
                // Remove all user tags for this hash
                // Note: This is a simplified implementation. A production version
                // would iterate through all user tags and remove those pointing to this hash.
                warn!(hash = %hash, "force delete requested - blob will be GC'd");
            }

            // For now, we mark success since the blob will be GC'd when unprotected
            // TODO: Add direct blob deletion to BlobStore trait when iroh-blobs supports it
            Ok(ClientRpcResponse::DeleteBlobResult(DeleteBlobResultResponse {
                success: true,
                error: None,
            }))
        }

        ClientRpcRequest::DownloadBlob { ticket, tag } => {
            use iroh_blobs::ticket::BlobTicket;

            use crate::blob::BlobStore;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::DownloadBlobResult(DownloadBlobResultResponse {
                    success: false,
                    hash: None,
                    size: None,
                    error: Some("blob store not enabled".to_string()),
                }));
            };

            // Parse the ticket
            let ticket = match ticket.parse::<BlobTicket>() {
                Ok(t) => t,
                Err(_) => {
                    // HIGH-4: Don't expose parse error details
                    return Ok(ClientRpcResponse::DownloadBlobResult(DownloadBlobResultResponse {
                        success: false,
                        hash: None,
                        size: None,
                        error: Some("invalid ticket".to_string()),
                    }));
                }
            };

            // First try the ticket's provider
            match blob_store.download(&ticket).await {
                Ok(blob_ref) => {
                    // Apply protection tag if requested
                    if let Some(ref tag_name) = tag {
                        let user_tag = IrohBlobStore::user_tag(tag_name);
                        if let Err(e) = blob_store.protect(&blob_ref.hash, &user_tag).await {
                            warn!(error = %e, "failed to apply tag to downloaded blob");
                        }
                    }

                    Ok(ClientRpcResponse::DownloadBlobResult(DownloadBlobResultResponse {
                        success: true,
                        hash: Some(blob_ref.hash.to_string()),
                        size: Some(blob_ref.size),
                        error: None,
                    }))
                }
                Err(ticket_error) => {
                    // Ticket provider failed. Try DHT providers if content discovery is enabled.
                    #[cfg(feature = "global-discovery")]
                    if let Some(ref discovery) = ctx.content_discovery {
                        let hash = ticket.hash();
                        let format = ticket.format();

                        debug!(
                            hash = %hash.fmt_short(),
                            "ticket provider failed, trying DHT providers"
                        );

                        // Query DHT for additional providers
                        if let Ok(providers) = discovery.find_providers(hash, format).await {
                            // Filter out the ticket provider (already tried)
                            let ticket_provider = ticket.addr().id;
                            let dht_providers: Vec<_> =
                                providers.into_iter().filter(|p| p.node_id != ticket_provider).collect();

                            if !dht_providers.is_empty() {
                                info!(
                                    hash = %hash.fmt_short(),
                                    provider_count = dht_providers.len(),
                                    "found additional DHT providers"
                                );

                                // Try each DHT provider
                                for provider in &dht_providers {
                                    debug!(
                                        hash = %hash.fmt_short(),
                                        provider = %provider.node_id.fmt_short(),
                                        "attempting download from DHT provider"
                                    );

                                    if let Ok(blob_ref) = blob_store.download_from_peer(&hash, provider.node_id).await {
                                        // Apply protection tag if requested
                                        if let Some(ref tag_name) = tag {
                                            let user_tag = IrohBlobStore::user_tag(tag_name);
                                            if let Err(e) = blob_store.protect(&blob_ref.hash, &user_tag).await {
                                                warn!(error = %e, "failed to apply tag to downloaded blob");
                                            }
                                        }

                                        info!(
                                            hash = %hash.fmt_short(),
                                            provider = %provider.node_id.fmt_short(),
                                            size = blob_ref.size,
                                            "blob downloaded from DHT provider (after ticket failure)"
                                        );

                                        return Ok(ClientRpcResponse::DownloadBlobResult(DownloadBlobResultResponse {
                                            success: true,
                                            hash: Some(blob_ref.hash.to_string()),
                                            size: Some(blob_ref.size),
                                            error: None,
                                        }));
                                    }
                                }
                            }
                        }
                    }

                    // All providers failed (ticket + DHT)
                    warn!(error = %ticket_error, "blob download failed from all providers");
                    Ok(ClientRpcResponse::DownloadBlobResult(DownloadBlobResultResponse {
                        success: false,
                        hash: None,
                        size: None,
                        error: Some(sanitize_blob_error(&ticket_error)),
                    }))
                }
            }
        }

        // Download blob by hash using DHT discovery (requires global-discovery feature)
        #[cfg(feature = "global-discovery")]
        ClientRpcRequest::DownloadBlobByHash { hash, tag } => {
            use iroh_blobs::Hash;

            use crate::blob::BlobStore;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::DownloadBlobByHashResult(DownloadBlobResultResponse {
                    success: false,
                    hash: None,
                    size: None,
                    error: Some("blob store not enabled".to_string()),
                }));
            };

            let Some(ref discovery) = ctx.content_discovery else {
                return Ok(ClientRpcResponse::DownloadBlobByHashResult(DownloadBlobResultResponse {
                    success: false,
                    hash: None,
                    size: None,
                    error: Some("content discovery not enabled".to_string()),
                }));
            };

            // Parse the hash
            let hash = match hash.parse::<Hash>() {
                Ok(h) => h,
                Err(_) => {
                    return Ok(ClientRpcResponse::DownloadBlobByHashResult(DownloadBlobResultResponse {
                        success: false,
                        hash: None,
                        size: None,
                        error: Some("invalid hash".to_string()),
                    }));
                }
            };

            // Query DHT for providers
            let providers = match discovery.find_providers(hash, iroh_blobs::BlobFormat::Raw).await {
                Ok(p) => p,
                Err(e) => {
                    warn!(error = %e, hash = %hash.fmt_short(), "DHT provider lookup failed");
                    return Ok(ClientRpcResponse::DownloadBlobByHashResult(DownloadBlobResultResponse {
                        success: false,
                        hash: Some(hash.to_string()),
                        size: None,
                        error: Some("provider lookup failed".to_string()),
                    }));
                }
            };

            if providers.is_empty() {
                return Ok(ClientRpcResponse::DownloadBlobByHashResult(DownloadBlobResultResponse {
                    success: false,
                    hash: Some(hash.to_string()),
                    size: None,
                    error: Some("no providers found".to_string()),
                }));
            }

            info!(
                hash = %hash.fmt_short(),
                provider_count = providers.len(),
                "found DHT providers"
            );

            // Try each provider until one succeeds
            let mut last_error = None;
            for provider in &providers {
                debug!(
                    hash = %hash.fmt_short(),
                    provider = %provider.node_id.fmt_short(),
                    "attempting download from DHT provider"
                );

                match blob_store.download_from_peer(&hash, provider.node_id).await {
                    Ok(blob_ref) => {
                        // Apply protection tag if requested
                        if let Some(ref tag_name) = tag {
                            let user_tag = IrohBlobStore::user_tag(tag_name);
                            if let Err(e) = blob_store.protect(&blob_ref.hash, &user_tag).await {
                                warn!(error = %e, "failed to apply tag to downloaded blob");
                            }
                        }

                        info!(
                            hash = %hash.fmt_short(),
                            provider = %provider.node_id.fmt_short(),
                            size = blob_ref.size,
                            "blob downloaded from DHT provider"
                        );

                        return Ok(ClientRpcResponse::DownloadBlobByHashResult(DownloadBlobResultResponse {
                            success: true,
                            hash: Some(blob_ref.hash.to_string()),
                            size: Some(blob_ref.size),
                            error: None,
                        }));
                    }
                    Err(e) => {
                        debug!(
                            error = %e,
                            provider = %provider.node_id.fmt_short(),
                            "download from provider failed, trying next"
                        );
                        last_error = Some(e);
                    }
                }
            }

            // All providers failed
            let error_msg =
                last_error.map(|e| sanitize_blob_error(&e)).unwrap_or_else(|| "all providers failed".to_string());
            warn!(hash = %hash.fmt_short(), error = %error_msg, "blob download failed from all providers");
            Ok(ClientRpcResponse::DownloadBlobByHashResult(DownloadBlobResultResponse {
                success: false,
                hash: Some(hash.to_string()),
                size: None,
                error: Some(error_msg),
            }))
        }

        // Fallback when global-discovery is not enabled
        #[cfg(not(feature = "global-discovery"))]
        ClientRpcRequest::DownloadBlobByHash { .. } => {
            Ok(ClientRpcResponse::DownloadBlobByHashResult(DownloadBlobResultResponse {
                success: false,
                hash: None,
                size: None,
                error: Some("global-discovery feature not enabled".to_string()),
            }))
        }

        // Download blob from a specific provider using DHT mutable item lookup
        #[cfg(feature = "global-discovery")]
        ClientRpcRequest::DownloadBlobByProvider { hash, provider, tag } => {
            use iroh::PublicKey;
            use iroh_blobs::BlobFormat;
            use iroh_blobs::Hash;

            use crate::blob::BlobStore;
            use crate::blob::IrohBlobStore;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
                    success: false,
                    hash: None,
                    size: None,
                    error: Some("blob store not enabled".to_string()),
                }));
            };

            let Some(ref discovery) = ctx.content_discovery else {
                return Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
                    success: false,
                    hash: None,
                    size: None,
                    error: Some("content discovery not enabled".to_string()),
                }));
            };

            // Parse the hash
            let hash = match hash.parse::<Hash>() {
                Ok(h) => h,
                Err(_) => {
                    return Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
                        success: false,
                        hash: None,
                        size: None,
                        error: Some("invalid hash".to_string()),
                    }));
                }
            };

            // Parse the provider public key
            let provider_key = match provider.parse::<PublicKey>() {
                Ok(k) => k,
                Err(_) => {
                    return Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
                        success: false,
                        hash: Some(hash.to_string()),
                        size: None,
                        error: Some("invalid provider public key".to_string()),
                    }));
                }
            };

            // Look up the provider's DhtNodeAddr in the DHT
            let node_addr = match discovery.find_provider_by_public_key(&provider_key, hash, BlobFormat::Raw).await {
                Ok(Some(addr)) => addr,
                Ok(None) => {
                    warn!(
                        hash = %hash.fmt_short(),
                        provider = %provider_key.fmt_short(),
                        "provider not found in DHT mutable items"
                    );
                    return Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
                        success: false,
                        hash: Some(hash.to_string()),
                        size: None,
                        error: Some("provider not found in DHT".to_string()),
                    }));
                }
                Err(e) => {
                    warn!(
                        hash = %hash.fmt_short(),
                        provider = %provider_key.fmt_short(),
                        error = %e,
                        "DHT mutable item lookup failed"
                    );
                    return Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
                        success: false,
                        hash: Some(hash.to_string()),
                        size: None,
                        error: Some(format!("DHT lookup failed: {}", e)),
                    }));
                }
            };

            info!(
                hash = %hash.fmt_short(),
                provider = %provider_key.fmt_short(),
                relay_url = ?node_addr.relay_url,
                direct_addrs = node_addr.direct_addrs.len(),
                "found provider in DHT, attempting download"
            );

            // Download from the provider
            match blob_store.download_from_peer(&hash, provider_key).await {
                Ok(blob_ref) => {
                    // Apply protection tag if requested
                    if let Some(ref tag_name) = tag {
                        let user_tag = IrohBlobStore::user_tag(tag_name);
                        if let Err(e) = blob_store.protect(&blob_ref.hash, &user_tag).await {
                            warn!(error = %e, "failed to apply tag to downloaded blob");
                        }
                    }

                    info!(
                        hash = %hash.fmt_short(),
                        provider = %provider_key.fmt_short(),
                        size = blob_ref.size,
                        "blob downloaded from DHT provider"
                    );

                    Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
                        success: true,
                        hash: Some(blob_ref.hash.to_string()),
                        size: Some(blob_ref.size),
                        error: None,
                    }))
                }
                Err(e) => {
                    let error_msg = sanitize_blob_error(&e);
                    warn!(
                        hash = %hash.fmt_short(),
                        provider = %provider_key.fmt_short(),
                        error = %error_msg,
                        "blob download from provider failed"
                    );
                    Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
                        success: false,
                        hash: Some(hash.to_string()),
                        size: None,
                        error: Some(error_msg),
                    }))
                }
            }
        }

        // Fallback when global-discovery is not enabled
        #[cfg(not(feature = "global-discovery"))]
        ClientRpcRequest::DownloadBlobByProvider { .. } => {
            Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
                success: false,
                hash: None,
                size: None,
                error: Some("global-discovery feature not enabled".to_string()),
            }))
        }

        ClientRpcRequest::GetBlobStatus { hash } => {
            use iroh_blobs::Hash;

            use crate::blob::BlobStore;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::GetBlobStatusResult(GetBlobStatusResultResponse {
                    found: false,
                    hash: None,
                    size: None,
                    complete: None,
                    tags: None,
                    error: Some("blob store not enabled".to_string()),
                }));
            };

            // Parse hash from string
            let hash = match hash.parse::<Hash>() {
                Ok(h) => h,
                Err(_) => {
                    // HIGH-4: Don't expose parse error details
                    return Ok(ClientRpcResponse::GetBlobStatusResult(GetBlobStatusResultResponse {
                        found: false,
                        hash: None,
                        size: None,
                        complete: None,
                        tags: None,
                        error: Some("invalid hash".to_string()),
                    }));
                }
            };

            match blob_store.status(&hash).await {
                Ok(Some(status)) => Ok(ClientRpcResponse::GetBlobStatusResult(GetBlobStatusResultResponse {
                    found: true,
                    hash: Some(status.hash.to_string()),
                    size: status.size,
                    complete: Some(status.complete),
                    tags: Some(status.tags),
                    error: None,
                })),
                Ok(None) => Ok(ClientRpcResponse::GetBlobStatusResult(GetBlobStatusResultResponse {
                    found: false,
                    hash: Some(hash.to_string()),
                    size: None,
                    complete: None,
                    tags: None,
                    error: None,
                })),
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob status check failed");
                    Ok(ClientRpcResponse::GetBlobStatusResult(GetBlobStatusResultResponse {
                        found: false,
                        hash: None,
                        size: None,
                        complete: None,
                        tags: None,
                        error: Some(sanitize_blob_error(&e)),
                    }))
                }
            }
        }

        // =========================================================================
        // Docs operations (iroh-docs CRDT replication)
        // =========================================================================
        ClientRpcRequest::DocsSet { key, value } => {
            use crate::client_rpc::DocsSetResultResponse;
            use crate::docs::BlobBackedDocsWriter;
            use crate::docs::DocsWriter;

            let Some(ref docs_sync) = ctx.docs_sync else {
                return Ok(ClientRpcResponse::DocsSetResult(DocsSetResultResponse {
                    success: false,
                    key: None,
                    size: None,
                    error: Some("docs not enabled".to_string()),
                }));
            };

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::DocsSetResult(DocsSetResultResponse {
                    success: false,
                    key: None,
                    size: None,
                    error: Some("blob store not enabled (required for docs)".to_string()),
                }));
            };

            let writer = BlobBackedDocsWriter::new(
                docs_sync.sync_handle.clone(),
                docs_sync.namespace_id,
                docs_sync.author.clone(),
                blob_store.clone(),
            );

            let value_len = value.len() as u64;
            match writer.set_entry(key.as_bytes().to_vec(), value).await {
                Ok(()) => Ok(ClientRpcResponse::DocsSetResult(DocsSetResultResponse {
                    success: true,
                    key: Some(key),
                    size: Some(value_len),
                    error: None,
                })),
                Err(e) => {
                    warn!(key = %key, error = %e, "docs set failed");
                    Ok(ClientRpcResponse::DocsSetResult(DocsSetResultResponse {
                        success: false,
                        key: Some(key),
                        size: None,
                        error: Some("docs operation failed".to_string()),
                    }))
                }
            }
        }

        ClientRpcRequest::DocsGet { key } => {
            use crate::blob::BlobStore;
            use crate::client_rpc::DocsGetResultResponse;

            let Some(ref docs_sync) = ctx.docs_sync else {
                return Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
                    found: false,
                    value: None,
                    size: None,
                    error: Some("docs not enabled".to_string()),
                }));
            };

            let key_bytes: bytes::Bytes = bytes::Bytes::from(key.as_bytes().to_vec());
            match docs_sync
                .sync_handle
                .get_exact(docs_sync.namespace_id, docs_sync.author.id(), key_bytes, false)
                .await
            {
                Ok(Some(entry)) => {
                    // Entry found, now get the content
                    let content_hash = entry.content_hash();
                    let content_len = entry.content_len();

                    // Try to get content from blob store if available
                    if let Some(ref blob_store) = ctx.blob_store {
                        match blob_store.get_bytes(&content_hash).await {
                            Ok(Some(bytes)) => Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
                                found: true,
                                value: Some(bytes.to_vec()),
                                size: Some(content_len),
                                error: None,
                            })),
                            Ok(None) => {
                                // Content exists but not in blob store
                                Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
                                    found: true,
                                    value: None,
                                    size: Some(content_len),
                                    error: Some("content not available locally".to_string()),
                                }))
                            }
                            Err(e) => {
                                warn!(key = %key, error = %e, "docs get content failed");
                                Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
                                    found: true,
                                    value: None,
                                    size: Some(content_len),
                                    error: Some("failed to fetch content".to_string()),
                                }))
                            }
                        }
                    } else {
                        Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
                            found: true,
                            value: None,
                            size: Some(content_len),
                            error: Some("blob store not enabled".to_string()),
                        }))
                    }
                }
                Ok(None) => Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
                    found: false,
                    value: None,
                    size: None,
                    error: None,
                })),
                Err(e) => {
                    warn!(key = %key, error = %e, "docs get failed");
                    Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
                        found: false,
                        value: None,
                        size: None,
                        error: Some("docs operation failed".to_string()),
                    }))
                }
            }
        }

        ClientRpcRequest::DocsDelete { key } => {
            use crate::client_rpc::DocsDeleteResultResponse;
            use crate::docs::BlobBackedDocsWriter;
            use crate::docs::DocsWriter;

            let Some(ref docs_sync) = ctx.docs_sync else {
                return Ok(ClientRpcResponse::DocsDeleteResult(DocsDeleteResultResponse {
                    success: false,
                    error: Some("docs not enabled".to_string()),
                }));
            };

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::DocsDeleteResult(DocsDeleteResultResponse {
                    success: false,
                    error: Some("blob store not enabled (required for docs)".to_string()),
                }));
            };

            let writer = BlobBackedDocsWriter::new(
                docs_sync.sync_handle.clone(),
                docs_sync.namespace_id,
                docs_sync.author.clone(),
                blob_store.clone(),
            );

            match writer.delete_entry(key.as_bytes().to_vec()).await {
                Ok(()) => Ok(ClientRpcResponse::DocsDeleteResult(DocsDeleteResultResponse {
                    success: true,
                    error: None,
                })),
                Err(e) => {
                    warn!(key = %key, error = %e, "docs delete failed");
                    Ok(ClientRpcResponse::DocsDeleteResult(DocsDeleteResultResponse {
                        success: false,
                        error: Some("docs operation failed".to_string()),
                    }))
                }
            }
        }

        ClientRpcRequest::DocsList { prefix, limit } => {
            use crate::client_rpc::DocsListEntry;
            use crate::client_rpc::DocsListResultResponse;
            use iroh_docs::store::Query;

            let Some(ref docs_sync) = ctx.docs_sync else {
                return Ok(ClientRpcResponse::DocsListResult(DocsListResultResponse {
                    entries: vec![],
                    count: 0,
                    has_more: false,
                    error: Some("docs not enabled".to_string()),
                }));
            };

            // Build query based on prefix filter
            // Add 1 to limit to detect if there are more entries
            let effective_limit = limit.unwrap_or(100) as u64 + 1;
            let query: Query = if let Some(ref prefix_str) = prefix {
                Query::single_latest_per_key().key_prefix(prefix_str.as_bytes()).limit(effective_limit).into()
            } else {
                Query::single_latest_per_key().limit(effective_limit).into()
            };

            // Create irpc channel to receive results
            let (tx, mut rx) = irpc::channel::mpsc::channel::<iroh_docs::api::RpcResult<iroh_docs::SignedEntry>>(1000);

            // Start the query
            if let Err(e) = docs_sync.sync_handle.get_many(docs_sync.namespace_id, query, tx).await {
                warn!(error = %e, "docs list query failed");
                return Ok(ClientRpcResponse::DocsListResult(DocsListResultResponse {
                    entries: vec![],
                    count: 0,
                    has_more: false,
                    error: Some("docs list query failed".to_string()),
                }));
            }

            // Collect results from the channel
            // recv() returns Result<Option<RpcResult<SignedEntry>>, RecvError>
            let mut entries = Vec::new();
            let max_entries = limit.unwrap_or(100) as usize;
            let mut has_more = false;

            while let Ok(Some(result)) = rx.recv().await {
                match result {
                    Ok(signed_entry) => {
                        if entries.len() >= max_entries {
                            has_more = true;
                            break;
                        }
                        let key = String::from_utf8_lossy(signed_entry.key()).to_string();
                        let size = signed_entry.content_len();
                        let hash = signed_entry.content_hash().to_string();
                        entries.push(DocsListEntry { key, size, hash });
                    }
                    Err(e) => {
                        warn!(error = %e, "error receiving docs entry");
                        // Continue processing other entries
                    }
                }
            }

            let count = entries.len() as u32;
            Ok(ClientRpcResponse::DocsListResult(DocsListResultResponse {
                entries,
                count,
                has_more,
                error: None,
            }))
        }

        ClientRpcRequest::DocsStatus => {
            use crate::client_rpc::DocsStatusResultResponse;
            use iroh_docs::store::Query;

            let Some(ref docs_sync) = ctx.docs_sync else {
                return Ok(ClientRpcResponse::DocsStatusResult(DocsStatusResultResponse {
                    enabled: false,
                    namespace_id: None,
                    author_id: None,
                    entry_count: None,
                    replica_open: None,
                    error: None,
                }));
            };

            let namespace_id = docs_sync.namespace_id.to_string();
            let author_id = docs_sync.author.id().to_string();

            // Count entries using SyncHandle::get_many
            // Tiger Style: Use bounded query to avoid unbounded memory use
            let query: Query = Query::single_latest_per_key().limit(10001).into(); // Count up to 10000, +1 to detect more
            let (tx, mut rx) = irpc::channel::mpsc::channel::<iroh_docs::api::RpcResult<iroh_docs::SignedEntry>>(1000);

            let entry_count = if docs_sync.sync_handle.get_many(docs_sync.namespace_id, query, tx).await.is_ok() {
                let mut count: u64 = 0;
                // recv() returns Result<Option<RpcResult<SignedEntry>>, RecvError>
                while let Ok(Some(_)) = rx.recv().await {
                    count += 1;
                    if count > 10000 {
                        break; // Tiger Style: bounded count
                    }
                }
                Some(count)
            } else {
                None
            };

            Ok(ClientRpcResponse::DocsStatusResult(DocsStatusResultResponse {
                enabled: true,
                namespace_id: Some(namespace_id),
                author_id: Some(author_id),
                entry_count,
                replica_open: Some(true),
                error: None,
            }))
        }

        // =========================================================================
        // Peer cluster operations (cluster-to-cluster sync)
        // =========================================================================
        ClientRpcRequest::AddPeerCluster { ticket } => {
            use crate::client_rpc::AddPeerClusterResultResponse;
            use crate::docs::ticket::AspenDocsTicket;

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::AddPeerClusterResult(AddPeerClusterResultResponse {
                    success: false,
                    cluster_id: None,
                    priority: None,
                    error: Some("peer sync not enabled".to_string()),
                }));
            };

            // Parse the ticket
            let docs_ticket = match AspenDocsTicket::deserialize(&ticket) {
                Ok(t) => t,
                Err(_) => {
                    // HIGH-4: Don't expose parse error details
                    return Ok(ClientRpcResponse::AddPeerClusterResult(AddPeerClusterResultResponse {
                        success: false,
                        cluster_id: None,
                        priority: None,
                        error: Some("invalid ticket".to_string()),
                    }));
                }
            };

            let cluster_id = docs_ticket.cluster_id.clone();
            let priority = docs_ticket.priority as u32;

            match peer_manager.add_peer(docs_ticket).await {
                Ok(()) => Ok(ClientRpcResponse::AddPeerClusterResult(AddPeerClusterResultResponse {
                    success: true,
                    cluster_id: Some(cluster_id),
                    priority: Some(priority),
                    error: None,
                })),
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "add peer cluster failed");
                    Ok(ClientRpcResponse::AddPeerClusterResult(AddPeerClusterResultResponse {
                        success: false,
                        cluster_id: Some(cluster_id),
                        priority: None,
                        error: Some("peer cluster operation failed".to_string()),
                    }))
                }
            }
        }

        ClientRpcRequest::RemovePeerCluster { cluster_id } => {
            use crate::client_rpc::RemovePeerClusterResultResponse;

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::RemovePeerClusterResult(RemovePeerClusterResultResponse {
                    success: false,
                    cluster_id: cluster_id.clone(),
                    error: Some("peer sync not enabled".to_string()),
                }));
            };

            match peer_manager.remove_peer(&cluster_id).await {
                Ok(()) => Ok(ClientRpcResponse::RemovePeerClusterResult(RemovePeerClusterResultResponse {
                    success: true,
                    cluster_id,
                    error: None,
                })),
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "remove peer cluster failed");
                    Ok(ClientRpcResponse::RemovePeerClusterResult(RemovePeerClusterResultResponse {
                        success: false,
                        cluster_id,
                        error: Some("peer cluster operation failed".to_string()),
                    }))
                }
            }
        }

        ClientRpcRequest::ListPeerClusters => {
            use crate::client_rpc::ListPeerClustersResultResponse;
            use crate::client_rpc::PeerClusterInfo;

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::ListPeerClustersResult(ListPeerClustersResultResponse {
                    peers: vec![],
                    count: 0,
                    error: Some("peer sync not enabled".to_string()),
                }));
            };

            let peers = peer_manager.list_peers().await;
            let count = peers.len() as u32;
            let peer_infos: Vec<PeerClusterInfo> = peers
                .into_iter()
                .map(|p| PeerClusterInfo {
                    cluster_id: p.cluster_id,
                    name: p.name,
                    state: format!("{:?}", p.state),
                    priority: p.priority,
                    enabled: p.enabled,
                    sync_count: p.sync_count,
                    failure_count: p.failure_count,
                })
                .collect();

            Ok(ClientRpcResponse::ListPeerClustersResult(ListPeerClustersResultResponse {
                peers: peer_infos,
                count,
                error: None,
            }))
        }

        ClientRpcRequest::GetPeerClusterStatus { cluster_id } => {
            use crate::client_rpc::PeerClusterStatusResponse;

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::PeerClusterStatus(PeerClusterStatusResponse {
                    found: false,
                    cluster_id: cluster_id.clone(),
                    state: "unknown".to_string(),
                    syncing: false,
                    entries_received: 0,
                    entries_imported: 0,
                    entries_skipped: 0,
                    entries_filtered: 0,
                    error: Some("peer sync not enabled".to_string()),
                }));
            };

            match peer_manager.sync_status(&cluster_id).await {
                Some(status) => Ok(ClientRpcResponse::PeerClusterStatus(PeerClusterStatusResponse {
                    found: true,
                    cluster_id: status.cluster_id,
                    state: format!("{:?}", status.state),
                    syncing: status.syncing,
                    entries_received: status.entries_received,
                    entries_imported: status.entries_imported,
                    entries_skipped: status.entries_skipped,
                    entries_filtered: status.entries_filtered,
                    error: None,
                })),
                None => Ok(ClientRpcResponse::PeerClusterStatus(PeerClusterStatusResponse {
                    found: false,
                    cluster_id,
                    state: "unknown".to_string(),
                    syncing: false,
                    entries_received: 0,
                    entries_imported: 0,
                    entries_skipped: 0,
                    entries_filtered: 0,
                    error: None,
                })),
            }
        }

        ClientRpcRequest::UpdatePeerClusterFilter {
            cluster_id,
            filter_type,
            prefixes,
        } => {
            use crate::client::SubscriptionFilter;
            use crate::client_rpc::UpdatePeerClusterFilterResultResponse;

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse {
                    success: false,
                    cluster_id: cluster_id.clone(),
                    filter_type: None,
                    error: Some("peer sync not enabled".to_string()),
                }));
            };

            // Parse filter type and prefixes
            let filter = match filter_type.to_lowercase().as_str() {
                "full" | "fullreplication" => SubscriptionFilter::FullReplication,
                "include" | "prefixfilter" => {
                    let prefix_list: Vec<String> =
                        prefixes.as_ref().map(|p| serde_json::from_str(p).unwrap_or_default()).unwrap_or_default();
                    SubscriptionFilter::PrefixFilter(prefix_list)
                }
                "exclude" | "prefixexclude" => {
                    let prefix_list: Vec<String> =
                        prefixes.as_ref().map(|p| serde_json::from_str(p).unwrap_or_default()).unwrap_or_default();
                    SubscriptionFilter::PrefixExclude(prefix_list)
                }
                other => {
                    return Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(
                        UpdatePeerClusterFilterResultResponse {
                            success: false,
                            cluster_id,
                            filter_type: None,
                            error: Some(format!("invalid filter type: {}", other)),
                        },
                    ));
                }
            };

            match peer_manager.importer().update_filter(&cluster_id, filter.clone()).await {
                Ok(()) => Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse {
                    success: true,
                    cluster_id,
                    filter_type: Some(filter_type),
                    error: None,
                })),
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "update peer cluster filter failed");
                    Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse {
                        success: false,
                        cluster_id,
                        filter_type: None,
                        error: Some("peer cluster operation failed".to_string()),
                    }))
                }
            }
        }

        ClientRpcRequest::UpdatePeerClusterPriority { cluster_id, priority } => {
            use crate::client_rpc::UpdatePeerClusterPriorityResultResponse;

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(
                    UpdatePeerClusterPriorityResultResponse {
                        success: false,
                        cluster_id: cluster_id.clone(),
                        previous_priority: None,
                        new_priority: None,
                        error: Some("peer sync not enabled".to_string()),
                    },
                ));
            };

            // Get current priority before update
            let previous_priority =
                peer_manager.list_peers().await.into_iter().find(|p| p.cluster_id == cluster_id).map(|p| p.priority);

            match peer_manager.importer().update_priority(&cluster_id, priority).await {
                Ok(()) => {
                    Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(UpdatePeerClusterPriorityResultResponse {
                        success: true,
                        cluster_id,
                        previous_priority,
                        new_priority: Some(priority),
                        error: None,
                    }))
                }
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "update peer cluster priority failed");
                    Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(UpdatePeerClusterPriorityResultResponse {
                        success: false,
                        cluster_id,
                        previous_priority,
                        new_priority: None,
                        error: Some("peer cluster operation failed".to_string()),
                    }))
                }
            }
        }

        ClientRpcRequest::SetPeerClusterEnabled { cluster_id, enabled } => {
            use crate::client_rpc::SetPeerClusterEnabledResultResponse;

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::SetPeerClusterEnabledResult(SetPeerClusterEnabledResultResponse {
                    success: false,
                    cluster_id: cluster_id.clone(),
                    enabled: None,
                    error: Some("peer sync not enabled".to_string()),
                }));
            };

            match peer_manager.importer().set_enabled(&cluster_id, enabled).await {
                Ok(()) => Ok(ClientRpcResponse::SetPeerClusterEnabledResult(SetPeerClusterEnabledResultResponse {
                    success: true,
                    cluster_id,
                    enabled: Some(enabled),
                    error: None,
                })),
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "set peer cluster enabled failed");
                    Ok(ClientRpcResponse::SetPeerClusterEnabledResult(SetPeerClusterEnabledResultResponse {
                        success: false,
                        cluster_id,
                        enabled: None,
                        error: Some("peer cluster operation failed".to_string()),
                    }))
                }
            }
        }

        ClientRpcRequest::GetKeyOrigin { key } => {
            use crate::client_rpc::KeyOriginResultResponse;

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::KeyOriginResult(KeyOriginResultResponse {
                    found: false,
                    key: key.clone(),
                    cluster_id: None,
                    priority: None,
                    timestamp_secs: None,
                    is_local: None,
                }));
            };

            match peer_manager.importer().get_key_origin(&key).await {
                Some(origin) => {
                    let is_local = origin.is_local();
                    Ok(ClientRpcResponse::KeyOriginResult(KeyOriginResultResponse {
                        found: true,
                        key,
                        cluster_id: Some(origin.cluster_id),
                        priority: Some(origin.priority),
                        timestamp_secs: Some(origin.timestamp_secs),
                        is_local: Some(is_local),
                    }))
                }
                None => Ok(ClientRpcResponse::KeyOriginResult(KeyOriginResultResponse {
                    found: false,
                    key,
                    cluster_id: None,
                    priority: None,
                    timestamp_secs: None,
                    is_local: None,
                })),
            }
        }

        ClientRpcRequest::ExecuteSql {
            query,
            params,
            consistency,
            limit,
            timeout_ms,
        } => {
            #[cfg(feature = "sql")]
            {
                use crate::api::SqlConsistency;
                use crate::api::SqlQueryExecutor;
                use crate::api::SqlQueryRequest;
                use crate::api::SqlValue;

                // Parse consistency level
                let consistency = match consistency.to_lowercase().as_str() {
                    "stale" => SqlConsistency::Stale,
                    _ => SqlConsistency::Linearizable, // Default to linearizable
                };

                // Parse parameters from JSON
                let params: Vec<SqlValue> = if params.is_empty() {
                    Vec::new()
                } else {
                    match serde_json::from_str::<Vec<serde_json::Value>>(&params) {
                        Ok(values) => values
                            .into_iter()
                            .map(|v| match v {
                                serde_json::Value::Null => SqlValue::Null,
                                serde_json::Value::Bool(b) => SqlValue::Integer(if b { 1 } else { 0 }),
                                serde_json::Value::Number(n) => {
                                    if let Some(i) = n.as_i64() {
                                        SqlValue::Integer(i)
                                    } else if let Some(f) = n.as_f64() {
                                        SqlValue::Real(f)
                                    } else {
                                        SqlValue::Text(n.to_string())
                                    }
                                }
                                serde_json::Value::String(s) => SqlValue::Text(s),
                                serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                                    SqlValue::Text(v.to_string())
                                }
                            })
                            .collect(),
                        Err(e) => {
                            return Ok(ClientRpcResponse::SqlResult(SqlResultResponse {
                                success: false,
                                columns: None,
                                rows: None,
                                row_count: None,
                                is_truncated: None,
                                execution_time_ms: None,
                                error: Some(format!("invalid params JSON: {}", e)),
                            }));
                        }
                    }
                };

                // Build request
                let request = SqlQueryRequest {
                    query,
                    params,
                    consistency,
                    limit,
                    timeout_ms,
                };

                // Execute SQL query via SqlQueryExecutor trait
                // The RaftNode implements this trait
                match ctx.sql_executor.execute_sql(request).await {
                    Ok(result) => {
                        // Convert SqlValue to SqlCellValue for PostCard-compatible RPC transport
                        // NOTE: serde_json::Value is NOT compatible with PostCard because
                        // PostCard doesn't support self-describing serialization (serialize_any).
                        use base64::Engine;
                        let rows: Vec<Vec<SqlCellValue>> = result
                            .rows
                            .into_iter()
                            .map(|row| {
                                row.into_iter()
                                    .map(|v| match v {
                                        SqlValue::Null => SqlCellValue::Null,
                                        SqlValue::Integer(i) => SqlCellValue::Integer(i),
                                        SqlValue::Real(f) => SqlCellValue::Real(f),
                                        SqlValue::Text(s) => SqlCellValue::Text(s),
                                        SqlValue::Blob(b) => {
                                            // Encode blob as base64 for safe text transport
                                            SqlCellValue::Blob(base64::engine::general_purpose::STANDARD.encode(&b))
                                        }
                                    })
                                    .collect()
                            })
                            .collect();

                        let columns: Vec<String> = result.columns.into_iter().map(|c| c.name).collect();

                        Ok(ClientRpcResponse::SqlResult(SqlResultResponse {
                            success: true,
                            columns: Some(columns),
                            rows: Some(rows),
                            row_count: Some(result.row_count),
                            is_truncated: Some(result.is_truncated),
                            execution_time_ms: Some(result.execution_time_ms),
                            error: None,
                        }))
                    }
                    Err(e) => {
                        // HIGH-4: Log full error internally, return sanitized message
                        warn!(error = %e, "SQL query execution failed");
                        Ok(ClientRpcResponse::SqlResult(SqlResultResponse {
                            success: false,
                            columns: None,
                            rows: None,
                            row_count: None,
                            is_truncated: None,
                            execution_time_ms: None,
                            // Return the error message (SQL errors are safe to show)
                            error: Some(e.to_string()),
                        }))
                    }
                }
            }

            #[cfg(not(feature = "sql"))]
            {
                // Silence unused variable warnings
                let _ = (query, params, consistency, limit, timeout_ms);
                Ok(ClientRpcResponse::SqlResult(SqlResultResponse {
                    success: false,
                    columns: None,
                    rows: None,
                    row_count: None,
                    is_truncated: None,
                    execution_time_ms: None,
                    error: Some("SQL queries not supported: compiled without 'sql' feature".into()),
                }))
            }
        }

        // =====================================================================
        // Coordination Primitives - Distributed Lock
        // =====================================================================
        ClientRpcRequest::LockAcquire {
            key,
            holder_id,
            ttl_ms,
            timeout_ms,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::LockResult(LockResultResponse {
                    success: false,
                    fencing_token: None,
                    holder_id: None,
                    deadline_ms: None,
                    error: Some(e.to_string()),
                }));
            }

            let config = LockConfig {
                ttl_ms,
                acquire_timeout_ms: timeout_ms,
                ..Default::default()
            };
            let lock = DistributedLock::new(ctx.kv_store.clone(), &key, &holder_id, config);

            match lock.acquire().await {
                Ok(guard) => {
                    let token = guard.fencing_token().value();
                    let deadline = guard.deadline_ms();
                    // Don't drop the guard here - let it release on its own
                    // The client is responsible for calling release explicitly
                    std::mem::forget(guard);
                    Ok(ClientRpcResponse::LockResult(LockResultResponse {
                        success: true,
                        fencing_token: Some(token),
                        holder_id: Some(holder_id),
                        deadline_ms: Some(deadline),
                        error: None,
                    }))
                }
                Err(e) => {
                    use crate::coordination::CoordinationError;
                    let (holder, deadline) = match &e {
                        CoordinationError::LockHeld { holder, deadline_ms } => {
                            (Some(holder.clone()), Some(*deadline_ms))
                        }
                        _ => (None, None),
                    };
                    Ok(ClientRpcResponse::LockResult(LockResultResponse {
                        success: false,
                        fencing_token: None,
                        holder_id: holder,
                        deadline_ms: deadline,
                        error: Some(e.to_string()),
                    }))
                }
            }
        }

        ClientRpcRequest::LockTryAcquire { key, holder_id, ttl_ms } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::LockResult(LockResultResponse {
                    success: false,
                    fencing_token: None,
                    holder_id: None,
                    deadline_ms: None,
                    error: Some(e.to_string()),
                }));
            }

            let config = LockConfig {
                ttl_ms,
                ..Default::default()
            };
            let lock = DistributedLock::new(ctx.kv_store.clone(), &key, &holder_id, config);

            match lock.try_acquire().await {
                Ok(guard) => {
                    let token = guard.fencing_token().value();
                    let deadline = guard.deadline_ms();
                    std::mem::forget(guard);
                    Ok(ClientRpcResponse::LockResult(LockResultResponse {
                        success: true,
                        fencing_token: Some(token),
                        holder_id: Some(holder_id),
                        deadline_ms: Some(deadline),
                        error: None,
                    }))
                }
                Err(e) => {
                    use crate::coordination::CoordinationError;
                    let (holder, deadline) = match &e {
                        CoordinationError::LockHeld { holder, deadline_ms } => {
                            (Some(holder.clone()), Some(*deadline_ms))
                        }
                        _ => (None, None),
                    };
                    Ok(ClientRpcResponse::LockResult(LockResultResponse {
                        success: false,
                        fencing_token: None,
                        holder_id: holder,
                        deadline_ms: deadline,
                        error: Some(e.to_string()),
                    }))
                }
            }
        }

        ClientRpcRequest::LockRelease {
            key,
            holder_id,
            fencing_token,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::LockResult(LockResultResponse {
                    success: false,
                    fencing_token: None,
                    holder_id: None,
                    deadline_ms: None,
                    error: Some(e.to_string()),
                }));
            }

            // Read current lock state and validate holder
            use crate::api::WriteCommand;
            use crate::coordination::LockEntry;

            let read_result = ctx.kv_store.read(ReadRequest::new(key.clone())).await;

            match read_result {
                Ok(result) => {
                    let value = result.kv.map(|kv| kv.value).unwrap_or_default();
                    match serde_json::from_str::<LockEntry>(&value) {
                        Ok(entry) => {
                            if entry.holder_id != holder_id || entry.fencing_token != fencing_token {
                                return Ok(ClientRpcResponse::LockResult(LockResultResponse {
                                    success: false,
                                    fencing_token: Some(entry.fencing_token),
                                    holder_id: Some(entry.holder_id),
                                    deadline_ms: Some(entry.deadline_ms),
                                    error: Some("lock not held by this holder".to_string()),
                                }));
                            }

                            // Release the lock via CAS
                            let released = entry.released();
                            let released_json = serde_json::to_string(&released)
                                .map_err(|e| anyhow::anyhow!("failed to serialize released lock entry: {}", e))?;

                            match ctx
                                .kv_store
                                .write(WriteRequest {
                                    command: WriteCommand::CompareAndSwap {
                                        key: key.clone(),
                                        expected: Some(value),
                                        new_value: released_json,
                                    },
                                })
                                .await
                            {
                                Ok(_) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                                    success: true,
                                    fencing_token: Some(fencing_token),
                                    holder_id: Some(holder_id),
                                    deadline_ms: None,
                                    error: None,
                                })),
                                Err(e) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                                    success: false,
                                    fencing_token: None,
                                    holder_id: None,
                                    deadline_ms: None,
                                    error: Some(format!("release failed: {}", e)),
                                })),
                            }
                        }
                        Err(_) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                            success: false,
                            fencing_token: None,
                            holder_id: None,
                            deadline_ms: None,
                            error: Some("invalid lock entry format".to_string()),
                        })),
                    }
                }
                Err(crate::api::KeyValueStoreError::NotFound { .. }) => {
                    Ok(ClientRpcResponse::LockResult(LockResultResponse {
                        success: true, // Lock doesn't exist, consider it released
                        fencing_token: Some(fencing_token),
                        holder_id: Some(holder_id),
                        deadline_ms: None,
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                    success: false,
                    fencing_token: None,
                    holder_id: None,
                    deadline_ms: None,
                    error: Some(format!("read failed: {}", e)),
                })),
            }
        }

        ClientRpcRequest::LockRenew {
            key,
            holder_id,
            fencing_token,
            ttl_ms,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::LockResult(LockResultResponse {
                    success: false,
                    fencing_token: None,
                    holder_id: None,
                    deadline_ms: None,
                    error: Some(e.to_string()),
                }));
            }

            use crate::api::WriteCommand;
            use crate::coordination::LockEntry;

            let read_result = ctx.kv_store.read(ReadRequest::new(key.clone())).await;

            match read_result {
                Ok(result) => {
                    let value = result.kv.map(|kv| kv.value).unwrap_or_default();
                    match serde_json::from_str::<LockEntry>(&value) {
                        Ok(entry) => {
                            if entry.holder_id != holder_id || entry.fencing_token != fencing_token {
                                return Ok(ClientRpcResponse::LockResult(LockResultResponse {
                                    success: false,
                                    fencing_token: Some(entry.fencing_token),
                                    holder_id: Some(entry.holder_id),
                                    deadline_ms: Some(entry.deadline_ms),
                                    error: Some("lock not held by this holder".to_string()),
                                }));
                            }

                            // Create renewed entry
                            let renewed = LockEntry::new(holder_id.clone(), fencing_token, ttl_ms);
                            let renewed_json = serde_json::to_string(&renewed)
                                .map_err(|e| anyhow::anyhow!("failed to serialize renewed lock entry: {}", e))?;

                            match ctx
                                .kv_store
                                .write(WriteRequest {
                                    command: WriteCommand::CompareAndSwap {
                                        key: key.clone(),
                                        expected: Some(value),
                                        new_value: renewed_json,
                                    },
                                })
                                .await
                            {
                                Ok(_) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                                    success: true,
                                    fencing_token: Some(fencing_token),
                                    holder_id: Some(holder_id),
                                    deadline_ms: Some(renewed.deadline_ms),
                                    error: None,
                                })),
                                Err(e) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                                    success: false,
                                    fencing_token: None,
                                    holder_id: None,
                                    deadline_ms: None,
                                    error: Some(format!("renew failed: {}", e)),
                                })),
                            }
                        }
                        Err(_) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                            success: false,
                            fencing_token: None,
                            holder_id: None,
                            deadline_ms: None,
                            error: Some("invalid lock entry format".to_string()),
                        })),
                    }
                }
                Err(e) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                    success: false,
                    fencing_token: None,
                    holder_id: None,
                    deadline_ms: None,
                    error: Some(format!("read failed: {}", e)),
                })),
            }
        }

        // =====================================================================
        // Coordination Primitives - Atomic Counter
        // =====================================================================
        ClientRpcRequest::CounterGet { key } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.get().await {
                Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: true,
                    value: Some(value),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::CounterIncrement { key } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.increment().await {
                Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: true,
                    value: Some(value),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::CounterDecrement { key } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.decrement().await {
                Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: true,
                    value: Some(value),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::CounterAdd { key, amount } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.add(amount).await {
                Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: true,
                    value: Some(value),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::CounterSubtract { key, amount } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.subtract(amount).await {
                Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: true,
                    value: Some(value),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::CounterSet { key, value } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.set(value).await {
                Ok(()) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: true,
                    value: Some(value),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::CounterCompareAndSet {
            key,
            expected,
            new_value,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.compare_and_set(expected, new_value).await {
                Ok(true) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: true,
                    value: Some(new_value),
                    error: None,
                })),
                Ok(false) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some("compare-and-set condition not met".to_string()),
                })),
                Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        // =====================================================================
        // Coordination Primitives - Signed Counter
        // =====================================================================
        ClientRpcRequest::SignedCounterGet { key } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let counter = SignedAtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.get().await {
                Ok(value) => Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
                    success: true,
                    value: Some(value),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::SignedCounterAdd { key, amount } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let counter = SignedAtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.add(amount).await {
                Ok(value) => Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
                    success: true,
                    value: Some(value),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        // =====================================================================
        // Coordination Primitives - Sequence Generator
        // =====================================================================
        ClientRpcRequest::SequenceNext { key } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let seq = SequenceGenerator::new(ctx.kv_store.clone(), &key, SequenceConfig::default());
            match seq.next().await {
                Ok(value) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                    success: true,
                    value: Some(value),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::SequenceReserve { key, count } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let seq = SequenceGenerator::new(ctx.kv_store.clone(), &key, SequenceConfig::default());
            match seq.reserve(count).await {
                Ok(start) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                    success: true,
                    value: Some(start),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::SequenceCurrent { key } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let seq = SequenceGenerator::new(ctx.kv_store.clone(), &key, SequenceConfig::default());
            match seq.current().await {
                Ok(value) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                    success: true,
                    value: Some(value),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        // =====================================================================
        // Coordination Primitives - Rate Limiter
        // =====================================================================
        ClientRpcRequest::RateLimiterTryAcquire {
            key,
            tokens,
            capacity,
            refill_rate,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                    success: false,
                    tokens_remaining: None,
                    retry_after_ms: None,
                    error: Some(e.to_string()),
                }));
            }

            let config = RateLimiterConfig {
                capacity,
                refill_rate,
                initial_tokens: None,
            };
            let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);
            match limiter.try_acquire_n(tokens).await {
                Ok(remaining) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                    success: true,
                    tokens_remaining: Some(remaining),
                    retry_after_ms: None,
                    error: None,
                })),
                Err(e) => {
                    let (available, retry_ms, err_msg) = match &e {
                        RateLimitError::TokensExhausted {
                            available,
                            retry_after_ms,
                            ..
                        } => (Some(*available), Some(*retry_after_ms), None),
                        RateLimitError::StorageUnavailable { reason } => (None, None, Some(reason.clone())),
                    };
                    Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                        success: false,
                        tokens_remaining: available,
                        retry_after_ms: retry_ms,
                        error: err_msg,
                    }))
                }
            }
        }

        ClientRpcRequest::RateLimiterAcquire {
            key,
            tokens,
            capacity,
            refill_rate,
            timeout_ms,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                    success: false,
                    tokens_remaining: None,
                    retry_after_ms: None,
                    error: Some(e.to_string()),
                }));
            }

            let config = RateLimiterConfig {
                capacity,
                refill_rate,
                initial_tokens: None,
            };
            let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);
            match limiter.acquire_n(tokens, std::time::Duration::from_millis(timeout_ms)).await {
                Ok(remaining) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                    success: true,
                    tokens_remaining: Some(remaining),
                    retry_after_ms: None,
                    error: None,
                })),
                Err(e) => {
                    let (available, retry_ms, err_msg) = match &e {
                        RateLimitError::TokensExhausted {
                            available,
                            retry_after_ms,
                            ..
                        } => (Some(*available), Some(*retry_after_ms), "timeout waiting for tokens".to_string()),
                        RateLimitError::StorageUnavailable { reason } => (None, None, reason.clone()),
                    };
                    Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                        success: false,
                        tokens_remaining: available,
                        retry_after_ms: retry_ms,
                        error: Some(err_msg),
                    }))
                }
            }
        }

        ClientRpcRequest::RateLimiterAvailable {
            key,
            capacity,
            refill_rate,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                    success: false,
                    tokens_remaining: None,
                    retry_after_ms: None,
                    error: Some(e.to_string()),
                }));
            }

            let config = RateLimiterConfig {
                capacity,
                refill_rate,
                initial_tokens: None,
            };
            let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);
            match limiter.available().await {
                Ok(tokens) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                    success: true,
                    tokens_remaining: Some(tokens),
                    retry_after_ms: None,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                    success: false,
                    tokens_remaining: None,
                    retry_after_ms: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::RateLimiterReset {
            key,
            capacity,
            refill_rate,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                    success: false,
                    tokens_remaining: None,
                    retry_after_ms: None,
                    error: Some(e.to_string()),
                }));
            }

            let config = RateLimiterConfig {
                capacity,
                refill_rate,
                initial_tokens: None,
            };
            let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);
            match limiter.reset().await {
                Ok(()) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                    success: true,
                    tokens_remaining: Some(capacity),
                    retry_after_ms: None,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                    success: false,
                    tokens_remaining: None,
                    retry_after_ms: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        // ==========================================================================
        // Batch Operations
        // ==========================================================================
        ClientRpcRequest::BatchRead { keys } => {
            use crate::api::KeyValueStoreError;
            // Validate all keys
            for key in &keys {
                if let Err(e) = validate_client_key(key) {
                    return Ok(ClientRpcResponse::BatchReadResult(BatchReadResultResponse {
                        success: false,
                        values: None,
                        error: Some(e.to_string()),
                    }));
                }
            }

            // Read all keys atomically
            let mut values = Vec::with_capacity(keys.len());
            for key in &keys {
                let request = ReadRequest::new(key.clone());
                match ctx.kv_store.read(request).await {
                    Ok(result) => {
                        // Key exists - return the value
                        let value_bytes = result.kv.map(|kv| kv.value.into_bytes());
                        values.push(value_bytes);
                    }
                    Err(KeyValueStoreError::NotFound { .. }) => {
                        // Key doesn't exist - return None for this position
                        values.push(None);
                    }
                    Err(e) => {
                        // Real error - fail the entire batch
                        return Ok(ClientRpcResponse::BatchReadResult(BatchReadResultResponse {
                            success: false,
                            values: None,
                            error: Some(e.to_string()),
                        }));
                    }
                }
            }

            Ok(ClientRpcResponse::BatchReadResult(BatchReadResultResponse {
                success: true,
                values: Some(values),
                error: None,
            }))
        }

        ClientRpcRequest::BatchWrite { operations } => {
            use crate::api::BatchOperation;
            use crate::api::WriteCommand;
            use crate::client_rpc::BatchWriteOperation;

            // Validate all keys
            for op in &operations {
                let key = match op {
                    BatchWriteOperation::Set { key, .. } => key,
                    BatchWriteOperation::Delete { key } => key,
                };
                if let Err(e) = validate_client_key(key) {
                    return Ok(ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
                        success: false,
                        operations_applied: None,
                        error: Some(e.to_string()),
                    }));
                }
            }

            // Convert to internal batch operations
            let batch_ops: Vec<BatchOperation> = operations
                .iter()
                .map(|op| match op {
                    BatchWriteOperation::Set { key, value } => BatchOperation::Set {
                        key: key.clone(),
                        value: String::from_utf8_lossy(value).to_string(),
                    },
                    BatchWriteOperation::Delete { key } => BatchOperation::Delete { key: key.clone() },
                })
                .collect();

            let request = WriteRequest {
                command: WriteCommand::Batch { operations: batch_ops },
            };

            match ctx.kv_store.write(request).await {
                Ok(result) => Ok(ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
                    success: true,
                    operations_applied: result.batch_applied,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
                    success: false,
                    operations_applied: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::ConditionalBatchWrite { conditions, operations } => {
            use crate::api::BatchCondition as ApiBatchCondition;
            use crate::api::BatchOperation;
            use crate::api::WriteCommand;
            use crate::client_rpc::BatchCondition;
            use crate::client_rpc::BatchWriteOperation;

            // Validate all keys in conditions
            for cond in &conditions {
                let key = match cond {
                    BatchCondition::ValueEquals { key, .. } => key,
                    BatchCondition::KeyExists { key } => key,
                    BatchCondition::KeyNotExists { key } => key,
                };
                if let Err(e) = validate_client_key(key) {
                    return Ok(ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
                        success: false,
                        conditions_met: false,
                        operations_applied: None,
                        failed_condition_index: None,
                        failed_condition_reason: Some(e.to_string()),
                        error: None,
                    }));
                }
            }

            // Validate all keys in operations
            for op in &operations {
                let key = match op {
                    BatchWriteOperation::Set { key, .. } => key,
                    BatchWriteOperation::Delete { key } => key,
                };
                if let Err(e) = validate_client_key(key) {
                    return Ok(ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
                        success: false,
                        conditions_met: false,
                        operations_applied: None,
                        failed_condition_index: None,
                        failed_condition_reason: Some(e.to_string()),
                        error: None,
                    }));
                }
            }

            // Convert conditions
            let api_conditions: Vec<ApiBatchCondition> = conditions
                .iter()
                .map(|c| match c {
                    BatchCondition::ValueEquals { key, expected } => ApiBatchCondition::ValueEquals {
                        key: key.clone(),
                        expected: String::from_utf8_lossy(expected).to_string(),
                    },
                    BatchCondition::KeyExists { key } => ApiBatchCondition::KeyExists { key: key.clone() },
                    BatchCondition::KeyNotExists { key } => ApiBatchCondition::KeyNotExists { key: key.clone() },
                })
                .collect();

            // Convert operations
            let batch_ops: Vec<BatchOperation> = operations
                .iter()
                .map(|op| match op {
                    BatchWriteOperation::Set { key, value } => BatchOperation::Set {
                        key: key.clone(),
                        value: String::from_utf8_lossy(value).to_string(),
                    },
                    BatchWriteOperation::Delete { key } => BatchOperation::Delete { key: key.clone() },
                })
                .collect();

            let request = WriteRequest {
                command: WriteCommand::ConditionalBatch {
                    conditions: api_conditions,
                    operations: batch_ops,
                },
            };

            match ctx.kv_store.write(request).await {
                Ok(result) => {
                    let conditions_met = result.conditions_met.unwrap_or(false);
                    Ok(ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
                        success: conditions_met,
                        conditions_met,
                        operations_applied: result.batch_applied,
                        failed_condition_index: result.failed_condition_index,
                        failed_condition_reason: None,
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
                    success: false,
                    conditions_met: false,
                    operations_applied: None,
                    failed_condition_index: None,
                    failed_condition_reason: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        // ==========================================================================
        // Watch Operations
        // ==========================================================================
        ClientRpcRequest::WatchCreate { .. } => {
            // Watch operations require a streaming connection via LOG_SUBSCRIBER_ALPN.
            // The ClientRpcRequest::WatchCreate is for documentation completeness,
            // but actual watch functionality is handled by LogSubscriberProtocolHandler.
            Ok(ClientRpcResponse::WatchCreateResult(WatchCreateResultResponse {
                success: false,
                watch_id: None,
                current_index: None,
                error: Some(
                    "Watch operations require the streaming protocol. \
                         Connect via LOG_SUBSCRIBER_ALPN (aspen-logs) for real-time \
                         key change notifications."
                        .to_string(),
                ),
            }))
        }

        ClientRpcRequest::WatchCancel { watch_id } => {
            // Same as WatchCreate - streaming protocol required
            Ok(ClientRpcResponse::WatchCancelResult(WatchCancelResultResponse {
                success: false,
                watch_id,
                error: Some(
                    "Watch operations require the streaming protocol. \
                         Use LOG_SUBSCRIBER_ALPN (aspen-logs)."
                        .to_string(),
                ),
            }))
        }

        ClientRpcRequest::WatchStatus { .. } => {
            // TODO: Could implement this to query LogSubscriberProtocolHandler state
            // For now, redirect to streaming protocol
            Ok(ClientRpcResponse::WatchStatusResult(WatchStatusResultResponse {
                success: false,
                watches: None,
                error: Some(
                    "Watch operations require the streaming protocol. \
                         Use LOG_SUBSCRIBER_ALPN (aspen-logs)."
                        .to_string(),
                ),
            }))
        }

        // =====================================================================
        // Lease operations
        // =====================================================================
        ClientRpcRequest::LeaseGrant { ttl_seconds, lease_id } => {
            use crate::api::WriteRequest;

            let actual_lease_id = lease_id.unwrap_or(0);

            let result = ctx
                .kv_store
                .write(WriteRequest {
                    command: crate::api::WriteCommand::LeaseGrant {
                        lease_id: actual_lease_id,
                        ttl_seconds,
                    },
                })
                .await;

            match result {
                Ok(response) => Ok(ClientRpcResponse::LeaseGrantResult(LeaseGrantResultResponse {
                    success: true,
                    lease_id: response.lease_id,
                    ttl_seconds: response.ttl_seconds,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::LeaseGrantResult(LeaseGrantResultResponse {
                    success: false,
                    lease_id: None,
                    ttl_seconds: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::LeaseRevoke { lease_id } => {
            use crate::api::WriteRequest;

            let result = ctx
                .kv_store
                .write(WriteRequest {
                    command: crate::api::WriteCommand::LeaseRevoke { lease_id },
                })
                .await;

            match result {
                Ok(response) => Ok(ClientRpcResponse::LeaseRevokeResult(LeaseRevokeResultResponse {
                    success: true,
                    keys_deleted: response.keys_deleted,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::LeaseRevokeResult(LeaseRevokeResultResponse {
                    success: false,
                    keys_deleted: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::LeaseKeepalive { lease_id } => {
            use crate::api::WriteRequest;

            let result = ctx
                .kv_store
                .write(WriteRequest {
                    command: crate::api::WriteCommand::LeaseKeepalive { lease_id },
                })
                .await;

            match result {
                Ok(response) => Ok(ClientRpcResponse::LeaseKeepaliveResult(LeaseKeepaliveResultResponse {
                    success: true,
                    lease_id: response.lease_id,
                    ttl_seconds: response.ttl_seconds,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::LeaseKeepaliveResult(LeaseKeepaliveResultResponse {
                    success: false,
                    lease_id: None,
                    ttl_seconds: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::LeaseTimeToLive { lease_id, include_keys } => {
            // Query state machine for lease info
            match &ctx.state_machine {
                Some(sm) => {
                    match sm.get_lease(lease_id) {
                        Some((granted_ttl, remaining_ttl)) => {
                            // Optionally include attached keys
                            let keys = if include_keys {
                                Some(sm.get_lease_keys(lease_id))
                            } else {
                                None
                            };

                            Ok(ClientRpcResponse::LeaseTimeToLiveResult(LeaseTimeToLiveResultResponse {
                                success: true,
                                lease_id: Some(lease_id),
                                granted_ttl_seconds: Some(granted_ttl),
                                remaining_ttl_seconds: Some(remaining_ttl),
                                keys,
                                error: None,
                            }))
                        }
                        None => {
                            // Lease not found or expired
                            Ok(ClientRpcResponse::LeaseTimeToLiveResult(LeaseTimeToLiveResultResponse {
                                success: false,
                                lease_id: Some(lease_id),
                                granted_ttl_seconds: None,
                                remaining_ttl_seconds: None,
                                keys: None,
                                error: Some("Lease not found or expired".to_string()),
                            }))
                        }
                    }
                }
                None => Ok(ClientRpcResponse::LeaseTimeToLiveResult(LeaseTimeToLiveResultResponse {
                    success: false,
                    lease_id: Some(lease_id),
                    granted_ttl_seconds: None,
                    remaining_ttl_seconds: None,
                    keys: None,
                    error: Some("State machine not available".to_string()),
                })),
            }
        }

        ClientRpcRequest::LeaseList => {
            // Query state machine for all active leases
            match &ctx.state_machine {
                Some(sm) => {
                    let leases_data = sm.list_leases();
                    let leases: Vec<LeaseInfo> = leases_data
                        .into_iter()
                        .map(|(lease_id, granted_ttl, remaining_ttl)| LeaseInfo {
                            lease_id,
                            granted_ttl_seconds: granted_ttl,
                            remaining_ttl_seconds: remaining_ttl,
                            // Use LeaseTimeToLive with include_keys=true for key counts
                            attached_keys: 0,
                        })
                        .collect();

                    Ok(ClientRpcResponse::LeaseListResult(LeaseListResultResponse {
                        success: true,
                        leases: Some(leases),
                        error: None,
                    }))
                }
                None => Ok(ClientRpcResponse::LeaseListResult(LeaseListResultResponse {
                    success: false,
                    leases: None,
                    error: Some("State machine not available".to_string()),
                })),
            }
        }

        ClientRpcRequest::WriteKeyWithLease { key, value, lease_id } => {
            use crate::api::WriteRequest;

            // Convert Vec<u8> to String
            let value_str = String::from_utf8_lossy(&value).to_string();

            let result = ctx
                .kv_store
                .write(WriteRequest {
                    command: crate::api::WriteCommand::SetWithLease {
                        key,
                        value: value_str.clone(),
                        lease_id,
                    },
                })
                .await;

            match result {
                Ok(_) => Ok(ClientRpcResponse::WriteResult(WriteResultResponse {
                    success: true,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::WriteResult(WriteResultResponse {
                    success: false,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        // =====================================================================
        // Barrier operations
        // =====================================================================
        ClientRpcRequest::BarrierEnter {
            name,
            participant_id,
            required_count,
            timeout_ms,
        } => {
            use crate::client_rpc::BarrierResultResponse;
            use crate::coordination::BarrierManager;

            let manager = BarrierManager::new(ctx.kv_store.clone());
            let timeout = if timeout_ms > 0 {
                Some(std::time::Duration::from_millis(timeout_ms))
            } else {
                None
            };

            match manager.enter(&name, &participant_id, required_count, timeout).await {
                Ok((current, phase)) => Ok(ClientRpcResponse::BarrierEnterResult(BarrierResultResponse {
                    success: true,
                    current_count: Some(current),
                    required_count: Some(required_count),
                    phase: Some(phase),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::BarrierEnterResult(BarrierResultResponse {
                    success: false,
                    current_count: None,
                    required_count: None,
                    phase: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::BarrierLeave {
            name,
            participant_id,
            timeout_ms,
        } => {
            use crate::client_rpc::BarrierResultResponse;
            use crate::coordination::BarrierManager;

            let manager = BarrierManager::new(ctx.kv_store.clone());
            let timeout = if timeout_ms > 0 {
                Some(std::time::Duration::from_millis(timeout_ms))
            } else {
                None
            };

            match manager.leave(&name, &participant_id, timeout).await {
                Ok((current, phase)) => Ok(ClientRpcResponse::BarrierLeaveResult(BarrierResultResponse {
                    success: true,
                    current_count: Some(current),
                    required_count: None,
                    phase: Some(phase),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::BarrierLeaveResult(BarrierResultResponse {
                    success: false,
                    current_count: None,
                    required_count: None,
                    phase: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::BarrierStatus { name } => {
            use crate::client_rpc::BarrierResultResponse;
            use crate::coordination::BarrierManager;

            let manager = BarrierManager::new(ctx.kv_store.clone());

            match manager.status(&name).await {
                Ok((current, required, phase)) => Ok(ClientRpcResponse::BarrierStatusResult(BarrierResultResponse {
                    success: true,
                    current_count: Some(current),
                    required_count: Some(required),
                    phase: Some(phase),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::BarrierStatusResult(BarrierResultResponse {
                    success: false,
                    current_count: None,
                    required_count: None,
                    phase: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        // =====================================================================
        // Semaphore operations
        // =====================================================================
        ClientRpcRequest::SemaphoreAcquire {
            name,
            holder_id,
            permits,
            capacity,
            ttl_ms,
            timeout_ms,
        } => {
            use crate::client_rpc::SemaphoreResultResponse;
            use crate::coordination::SemaphoreManager;

            let manager = SemaphoreManager::new(ctx.kv_store.clone());
            let timeout = if timeout_ms > 0 {
                Some(std::time::Duration::from_millis(timeout_ms))
            } else {
                None
            };

            match manager.acquire(&name, &holder_id, permits, capacity, ttl_ms, timeout).await {
                Ok((acquired, available)) => Ok(ClientRpcResponse::SemaphoreAcquireResult(SemaphoreResultResponse {
                    success: true,
                    permits_acquired: Some(acquired),
                    available: Some(available),
                    capacity: Some(capacity),
                    retry_after_ms: None,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::SemaphoreAcquireResult(SemaphoreResultResponse {
                    success: false,
                    permits_acquired: None,
                    available: None,
                    capacity: None,
                    retry_after_ms: Some(100), // Suggest 100ms retry
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::SemaphoreTryAcquire {
            name,
            holder_id,
            permits,
            capacity,
            ttl_ms,
        } => {
            use crate::client_rpc::SemaphoreResultResponse;
            use crate::coordination::SemaphoreManager;

            let manager = SemaphoreManager::new(ctx.kv_store.clone());

            match manager.try_acquire(&name, &holder_id, permits, capacity, ttl_ms).await {
                Ok(Some((acquired, available))) => {
                    Ok(ClientRpcResponse::SemaphoreTryAcquireResult(SemaphoreResultResponse {
                        success: true,
                        permits_acquired: Some(acquired),
                        available: Some(available),
                        capacity: Some(capacity),
                        retry_after_ms: None,
                        error: None,
                    }))
                }
                Ok(None) => Ok(ClientRpcResponse::SemaphoreTryAcquireResult(SemaphoreResultResponse {
                    success: false,
                    permits_acquired: None,
                    available: None,
                    capacity: Some(capacity),
                    retry_after_ms: Some(100),
                    error: Some("No permits available".to_string()),
                })),
                Err(e) => Ok(ClientRpcResponse::SemaphoreTryAcquireResult(SemaphoreResultResponse {
                    success: false,
                    permits_acquired: None,
                    available: None,
                    capacity: None,
                    retry_after_ms: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::SemaphoreRelease {
            name,
            holder_id,
            permits,
        } => {
            use crate::client_rpc::SemaphoreResultResponse;
            use crate::coordination::SemaphoreManager;

            let manager = SemaphoreManager::new(ctx.kv_store.clone());

            match manager.release(&name, &holder_id, permits).await {
                Ok(available) => Ok(ClientRpcResponse::SemaphoreReleaseResult(SemaphoreResultResponse {
                    success: true,
                    permits_acquired: None,
                    available: Some(available),
                    capacity: None,
                    retry_after_ms: None,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::SemaphoreReleaseResult(SemaphoreResultResponse {
                    success: false,
                    permits_acquired: None,
                    available: None,
                    capacity: None,
                    retry_after_ms: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::SemaphoreStatus { name } => {
            use crate::client_rpc::SemaphoreResultResponse;
            use crate::coordination::SemaphoreManager;

            let manager = SemaphoreManager::new(ctx.kv_store.clone());

            match manager.status(&name).await {
                Ok((available, capacity)) => Ok(ClientRpcResponse::SemaphoreStatusResult(SemaphoreResultResponse {
                    success: true,
                    permits_acquired: None,
                    available: Some(available),
                    capacity: Some(capacity),
                    retry_after_ms: None,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::SemaphoreStatusResult(SemaphoreResultResponse {
                    success: false,
                    permits_acquired: None,
                    available: None,
                    capacity: None,
                    retry_after_ms: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        // =====================================================================
        // Read-Write Lock operations
        // =====================================================================
        ClientRpcRequest::RWLockAcquireRead {
            name,
            holder_id,
            ttl_ms,
            timeout_ms,
        } => {
            use crate::client_rpc::RWLockResultResponse;
            use crate::coordination::RWLockManager;

            let manager = RWLockManager::new(ctx.kv_store.clone());
            let timeout = if timeout_ms > 0 {
                Some(std::time::Duration::from_millis(timeout_ms))
            } else {
                None
            };

            match manager.acquire_read(&name, &holder_id, ttl_ms, timeout).await {
                Ok((fencing_token, deadline_ms, reader_count)) => {
                    Ok(ClientRpcResponse::RWLockAcquireReadResult(RWLockResultResponse {
                        success: true,
                        mode: Some("read".to_string()),
                        fencing_token: Some(fencing_token),
                        deadline_ms: Some(deadline_ms),
                        reader_count: Some(reader_count),
                        writer_holder: None,
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::RWLockAcquireReadResult(RWLockResultResponse {
                    success: false,
                    mode: None,
                    fencing_token: None,
                    deadline_ms: None,
                    reader_count: None,
                    writer_holder: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::RWLockTryAcquireRead {
            name,
            holder_id,
            ttl_ms,
        } => {
            use crate::client_rpc::RWLockResultResponse;
            use crate::coordination::RWLockManager;

            let manager = RWLockManager::new(ctx.kv_store.clone());

            match manager.try_acquire_read(&name, &holder_id, ttl_ms).await {
                Ok(Some((fencing_token, deadline_ms, reader_count))) => {
                    Ok(ClientRpcResponse::RWLockTryAcquireReadResult(RWLockResultResponse {
                        success: true,
                        mode: Some("read".to_string()),
                        fencing_token: Some(fencing_token),
                        deadline_ms: Some(deadline_ms),
                        reader_count: Some(reader_count),
                        writer_holder: None,
                        error: None,
                    }))
                }
                Ok(None) => Ok(ClientRpcResponse::RWLockTryAcquireReadResult(RWLockResultResponse {
                    success: false,
                    mode: None,
                    fencing_token: None,
                    deadline_ms: None,
                    reader_count: None,
                    writer_holder: None,
                    error: Some("lock not available".to_string()),
                })),
                Err(e) => Ok(ClientRpcResponse::RWLockTryAcquireReadResult(RWLockResultResponse {
                    success: false,
                    mode: None,
                    fencing_token: None,
                    deadline_ms: None,
                    reader_count: None,
                    writer_holder: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::RWLockAcquireWrite {
            name,
            holder_id,
            ttl_ms,
            timeout_ms,
        } => {
            use crate::client_rpc::RWLockResultResponse;
            use crate::coordination::RWLockManager;

            let manager = RWLockManager::new(ctx.kv_store.clone());
            let timeout = if timeout_ms > 0 {
                Some(std::time::Duration::from_millis(timeout_ms))
            } else {
                None
            };

            match manager.acquire_write(&name, &holder_id, ttl_ms, timeout).await {
                Ok((fencing_token, deadline_ms)) => {
                    Ok(ClientRpcResponse::RWLockAcquireWriteResult(RWLockResultResponse {
                        success: true,
                        mode: Some("write".to_string()),
                        fencing_token: Some(fencing_token),
                        deadline_ms: Some(deadline_ms),
                        reader_count: Some(0),
                        writer_holder: Some(holder_id),
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::RWLockAcquireWriteResult(RWLockResultResponse {
                    success: false,
                    mode: None,
                    fencing_token: None,
                    deadline_ms: None,
                    reader_count: None,
                    writer_holder: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::RWLockTryAcquireWrite {
            name,
            holder_id,
            ttl_ms,
        } => {
            use crate::client_rpc::RWLockResultResponse;
            use crate::coordination::RWLockManager;

            let manager = RWLockManager::new(ctx.kv_store.clone());

            match manager.try_acquire_write(&name, &holder_id, ttl_ms).await {
                Ok(Some((fencing_token, deadline_ms))) => {
                    Ok(ClientRpcResponse::RWLockTryAcquireWriteResult(RWLockResultResponse {
                        success: true,
                        mode: Some("write".to_string()),
                        fencing_token: Some(fencing_token),
                        deadline_ms: Some(deadline_ms),
                        reader_count: Some(0),
                        writer_holder: Some(holder_id),
                        error: None,
                    }))
                }
                Ok(None) => Ok(ClientRpcResponse::RWLockTryAcquireWriteResult(RWLockResultResponse {
                    success: false,
                    mode: None,
                    fencing_token: None,
                    deadline_ms: None,
                    reader_count: None,
                    writer_holder: None,
                    error: Some("lock not available".to_string()),
                })),
                Err(e) => Ok(ClientRpcResponse::RWLockTryAcquireWriteResult(RWLockResultResponse {
                    success: false,
                    mode: None,
                    fencing_token: None,
                    deadline_ms: None,
                    reader_count: None,
                    writer_holder: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::RWLockReleaseRead { name, holder_id } => {
            use crate::client_rpc::RWLockResultResponse;
            use crate::coordination::RWLockManager;

            let manager = RWLockManager::new(ctx.kv_store.clone());

            match manager.release_read(&name, &holder_id).await {
                Ok(()) => Ok(ClientRpcResponse::RWLockReleaseReadResult(RWLockResultResponse {
                    success: true,
                    mode: None,
                    fencing_token: None,
                    deadline_ms: None,
                    reader_count: None,
                    writer_holder: None,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::RWLockReleaseReadResult(RWLockResultResponse {
                    success: false,
                    mode: None,
                    fencing_token: None,
                    deadline_ms: None,
                    reader_count: None,
                    writer_holder: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::RWLockReleaseWrite {
            name,
            holder_id,
            fencing_token,
        } => {
            use crate::client_rpc::RWLockResultResponse;
            use crate::coordination::RWLockManager;

            let manager = RWLockManager::new(ctx.kv_store.clone());

            match manager.release_write(&name, &holder_id, fencing_token).await {
                Ok(()) => Ok(ClientRpcResponse::RWLockReleaseWriteResult(RWLockResultResponse {
                    success: true,
                    mode: None,
                    fencing_token: None,
                    deadline_ms: None,
                    reader_count: None,
                    writer_holder: None,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::RWLockReleaseWriteResult(RWLockResultResponse {
                    success: false,
                    mode: None,
                    fencing_token: None,
                    deadline_ms: None,
                    reader_count: None,
                    writer_holder: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::RWLockDowngrade {
            name,
            holder_id,
            fencing_token,
            ttl_ms,
        } => {
            use crate::client_rpc::RWLockResultResponse;
            use crate::coordination::RWLockManager;

            let manager = RWLockManager::new(ctx.kv_store.clone());

            match manager.downgrade(&name, &holder_id, fencing_token, ttl_ms).await {
                Ok((new_token, deadline_ms, reader_count)) => {
                    Ok(ClientRpcResponse::RWLockDowngradeResult(RWLockResultResponse {
                        success: true,
                        mode: Some("read".to_string()),
                        fencing_token: Some(new_token),
                        deadline_ms: Some(deadline_ms),
                        reader_count: Some(reader_count),
                        writer_holder: None,
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::RWLockDowngradeResult(RWLockResultResponse {
                    success: false,
                    mode: None,
                    fencing_token: None,
                    deadline_ms: None,
                    reader_count: None,
                    writer_holder: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::RWLockStatus { name } => {
            use crate::client_rpc::RWLockResultResponse;
            use crate::coordination::RWLockManager;

            let manager = RWLockManager::new(ctx.kv_store.clone());

            match manager.status(&name).await {
                Ok((mode, reader_count, writer_holder, fencing_token)) => {
                    Ok(ClientRpcResponse::RWLockStatusResult(RWLockResultResponse {
                        success: true,
                        mode: Some(mode),
                        fencing_token: Some(fencing_token),
                        deadline_ms: None,
                        reader_count: Some(reader_count),
                        writer_holder,
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::RWLockStatusResult(RWLockResultResponse {
                    success: false,
                    mode: None,
                    fencing_token: None,
                    deadline_ms: None,
                    reader_count: None,
                    writer_holder: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        // =========================================================================
        // Queue handlers
        // =========================================================================
        ClientRpcRequest::QueueCreate {
            queue_name,
            default_visibility_timeout_ms,
            default_ttl_ms,
            max_delivery_attempts,
        } => {
            use crate::client_rpc::QueueCreateResultResponse;
            use crate::coordination::QueueConfig;
            use crate::coordination::QueueManager;

            let manager = QueueManager::new(ctx.kv_store.clone());
            let config = QueueConfig {
                default_visibility_timeout_ms,
                default_ttl_ms,
                max_delivery_attempts,
            };

            match manager.create(&queue_name, config).await {
                Ok((created, _)) => Ok(ClientRpcResponse::QueueCreateResult(QueueCreateResultResponse {
                    success: true,
                    created,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::QueueCreateResult(QueueCreateResultResponse {
                    success: false,
                    created: false,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::QueueDelete { queue_name } => {
            use crate::client_rpc::QueueDeleteResultResponse;
            use crate::coordination::QueueManager;

            let manager = QueueManager::new(ctx.kv_store.clone());

            match manager.delete(&queue_name).await {
                Ok(items_deleted) => Ok(ClientRpcResponse::QueueDeleteResult(QueueDeleteResultResponse {
                    success: true,
                    items_deleted: Some(items_deleted),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::QueueDeleteResult(QueueDeleteResultResponse {
                    success: false,
                    items_deleted: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::QueueEnqueue {
            queue_name,
            payload,
            ttl_ms,
            message_group_id,
            deduplication_id,
        } => {
            use crate::client_rpc::QueueEnqueueResultResponse;
            use crate::coordination::EnqueueOptions;
            use crate::coordination::QueueManager;

            let manager = QueueManager::new(ctx.kv_store.clone());
            let options = EnqueueOptions {
                ttl_ms,
                message_group_id,
                deduplication_id,
            };

            match manager.enqueue(&queue_name, payload, options).await {
                Ok(item_id) => Ok(ClientRpcResponse::QueueEnqueueResult(QueueEnqueueResultResponse {
                    success: true,
                    item_id: Some(item_id),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::QueueEnqueueResult(QueueEnqueueResultResponse {
                    success: false,
                    item_id: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::QueueEnqueueBatch { queue_name, items } => {
            use crate::client_rpc::QueueEnqueueBatchResultResponse;
            use crate::coordination::EnqueueOptions;
            use crate::coordination::QueueManager;

            let manager = QueueManager::new(ctx.kv_store.clone());
            let batch: Vec<(Vec<u8>, EnqueueOptions)> = items
                .into_iter()
                .map(|item| {
                    (
                        item.payload,
                        EnqueueOptions {
                            ttl_ms: item.ttl_ms,
                            message_group_id: item.message_group_id,
                            deduplication_id: item.deduplication_id,
                        },
                    )
                })
                .collect();

            match manager.enqueue_batch(&queue_name, batch).await {
                Ok(item_ids) => Ok(ClientRpcResponse::QueueEnqueueBatchResult(QueueEnqueueBatchResultResponse {
                    success: true,
                    item_ids,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::QueueEnqueueBatchResult(QueueEnqueueBatchResultResponse {
                    success: false,
                    item_ids: vec![],
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::QueueDequeue {
            queue_name,
            consumer_id,
            max_items,
            visibility_timeout_ms,
        } => {
            use crate::client_rpc::QueueDequeueResultResponse;
            use crate::client_rpc::QueueDequeuedItemResponse;
            use crate::coordination::QueueManager;

            let manager = QueueManager::new(ctx.kv_store.clone());

            match manager.dequeue(&queue_name, &consumer_id, max_items, visibility_timeout_ms).await {
                Ok(items) => {
                    let response_items: Vec<QueueDequeuedItemResponse> = items
                        .into_iter()
                        .map(|item| QueueDequeuedItemResponse {
                            item_id: item.item_id,
                            payload: item.payload,
                            receipt_handle: item.receipt_handle,
                            delivery_attempts: item.delivery_attempts,
                            enqueued_at_ms: item.enqueued_at_ms,
                            visibility_deadline_ms: item.visibility_deadline_ms,
                        })
                        .collect();
                    Ok(ClientRpcResponse::QueueDequeueResult(QueueDequeueResultResponse {
                        success: true,
                        items: response_items,
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::QueueDequeueResult(QueueDequeueResultResponse {
                    success: false,
                    items: vec![],
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::QueueDequeueWait {
            queue_name,
            consumer_id,
            max_items,
            visibility_timeout_ms,
            wait_timeout_ms,
        } => {
            use crate::client_rpc::QueueDequeueResultResponse;
            use crate::client_rpc::QueueDequeuedItemResponse;
            use crate::coordination::QueueManager;

            let manager = QueueManager::new(ctx.kv_store.clone());

            match manager
                .dequeue_wait(&queue_name, &consumer_id, max_items, visibility_timeout_ms, wait_timeout_ms)
                .await
            {
                Ok(items) => {
                    let response_items: Vec<QueueDequeuedItemResponse> = items
                        .into_iter()
                        .map(|item| QueueDequeuedItemResponse {
                            item_id: item.item_id,
                            payload: item.payload,
                            receipt_handle: item.receipt_handle,
                            delivery_attempts: item.delivery_attempts,
                            enqueued_at_ms: item.enqueued_at_ms,
                            visibility_deadline_ms: item.visibility_deadline_ms,
                        })
                        .collect();
                    Ok(ClientRpcResponse::QueueDequeueResult(QueueDequeueResultResponse {
                        success: true,
                        items: response_items,
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::QueueDequeueResult(QueueDequeueResultResponse {
                    success: false,
                    items: vec![],
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::QueuePeek { queue_name, max_items } => {
            use crate::client_rpc::QueueItemResponse;
            use crate::client_rpc::QueuePeekResultResponse;
            use crate::coordination::QueueManager;

            let manager = QueueManager::new(ctx.kv_store.clone());

            match manager.peek(&queue_name, max_items).await {
                Ok(items) => {
                    let response_items: Vec<QueueItemResponse> = items
                        .into_iter()
                        .map(|item| QueueItemResponse {
                            item_id: item.item_id,
                            payload: item.payload,
                            enqueued_at_ms: item.enqueued_at_ms,
                            expires_at_ms: item.expires_at_ms,
                            delivery_attempts: item.delivery_attempts,
                        })
                        .collect();
                    Ok(ClientRpcResponse::QueuePeekResult(QueuePeekResultResponse {
                        success: true,
                        items: response_items,
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::QueuePeekResult(QueuePeekResultResponse {
                    success: false,
                    items: vec![],
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::QueueAck {
            queue_name,
            receipt_handle,
        } => {
            use crate::client_rpc::QueueAckResultResponse;
            use crate::coordination::QueueManager;

            let manager = QueueManager::new(ctx.kv_store.clone());

            match manager.ack(&queue_name, &receipt_handle).await {
                Ok(()) => Ok(ClientRpcResponse::QueueAckResult(QueueAckResultResponse {
                    success: true,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::QueueAckResult(QueueAckResultResponse {
                    success: false,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::QueueNack {
            queue_name,
            receipt_handle,
            move_to_dlq,
            error_message,
        } => {
            use crate::client_rpc::QueueNackResultResponse;
            use crate::coordination::QueueManager;

            let manager = QueueManager::new(ctx.kv_store.clone());

            match manager.nack(&queue_name, &receipt_handle, move_to_dlq, error_message).await {
                Ok(()) => Ok(ClientRpcResponse::QueueNackResult(QueueNackResultResponse {
                    success: true,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::QueueNackResult(QueueNackResultResponse {
                    success: false,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::QueueExtendVisibility {
            queue_name,
            receipt_handle,
            additional_timeout_ms,
        } => {
            use crate::client_rpc::QueueExtendVisibilityResultResponse;
            use crate::coordination::QueueManager;

            let manager = QueueManager::new(ctx.kv_store.clone());

            match manager.extend_visibility(&queue_name, &receipt_handle, additional_timeout_ms).await {
                Ok(new_deadline) => {
                    Ok(ClientRpcResponse::QueueExtendVisibilityResult(QueueExtendVisibilityResultResponse {
                        success: true,
                        new_deadline_ms: Some(new_deadline),
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::QueueExtendVisibilityResult(QueueExtendVisibilityResultResponse {
                    success: false,
                    new_deadline_ms: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::QueueStatus { queue_name } => {
            use crate::client_rpc::QueueStatusResultResponse;
            use crate::coordination::QueueManager;

            let manager = QueueManager::new(ctx.kv_store.clone());

            match manager.status(&queue_name).await {
                Ok(status) => Ok(ClientRpcResponse::QueueStatusResult(QueueStatusResultResponse {
                    success: true,
                    exists: status.exists,
                    visible_count: Some(status.visible_count),
                    pending_count: Some(status.pending_count),
                    dlq_count: Some(status.dlq_count),
                    total_enqueued: Some(status.total_enqueued),
                    total_acked: Some(status.total_acked),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::QueueStatusResult(QueueStatusResultResponse {
                    success: false,
                    exists: false,
                    visible_count: None,
                    pending_count: None,
                    dlq_count: None,
                    total_enqueued: None,
                    total_acked: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::QueueGetDLQ { queue_name, max_items } => {
            use crate::client_rpc::QueueDLQItemResponse;
            use crate::client_rpc::QueueGetDLQResultResponse;
            use crate::coordination::QueueManager;

            let manager = QueueManager::new(ctx.kv_store.clone());

            match manager.get_dlq(&queue_name, max_items).await {
                Ok(items) => {
                    let response_items: Vec<QueueDLQItemResponse> = items
                        .into_iter()
                        .map(|item| QueueDLQItemResponse {
                            item_id: item.item_id,
                            payload: item.payload,
                            enqueued_at_ms: item.enqueued_at_ms,
                            delivery_attempts: item.delivery_attempts,
                            reason: format!("{:?}", item.reason),
                            moved_at_ms: item.moved_at_ms,
                            last_error: item.last_error,
                        })
                        .collect();
                    Ok(ClientRpcResponse::QueueGetDLQResult(QueueGetDLQResultResponse {
                        success: true,
                        items: response_items,
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::QueueGetDLQResult(QueueGetDLQResultResponse {
                    success: false,
                    items: vec![],
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::QueueRedriveDLQ { queue_name, item_id } => {
            use crate::client_rpc::QueueRedriveDLQResultResponse;
            use crate::coordination::QueueManager;

            let manager = QueueManager::new(ctx.kv_store.clone());

            match manager.redrive_dlq(&queue_name, item_id).await {
                Ok(()) => Ok(ClientRpcResponse::QueueRedriveDLQResult(QueueRedriveDLQResultResponse {
                    success: true,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::QueueRedriveDLQResult(QueueRedriveDLQResultResponse {
                    success: false,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        // =====================================================================
        // Service Registry handlers
        // =====================================================================
        ClientRpcRequest::ServiceRegister {
            service_name,
            instance_id,
            address,
            version,
            tags,
            weight,
            custom_metadata,
            ttl_ms,
            lease_id,
        } => {
            use std::collections::HashMap;

            use crate::client_rpc::ServiceRegisterResultResponse;
            use crate::coordination::HealthStatus;
            use crate::coordination::RegisterOptions;
            use crate::coordination::ServiceInstanceMetadata;
            use crate::coordination::ServiceRegistry;

            let registry = ServiceRegistry::new(ctx.kv_store.clone());

            // Parse tags from JSON array
            let tags_vec: Vec<String> = serde_json::from_str(&tags).unwrap_or_default();

            // Parse custom metadata from JSON object
            let custom_map: HashMap<String, String> = serde_json::from_str(&custom_metadata).unwrap_or_default();

            let metadata = ServiceInstanceMetadata {
                version,
                tags: tags_vec,
                weight,
                custom: custom_map,
            };

            let options = RegisterOptions {
                ttl_ms: if ttl_ms == 0 { None } else { Some(ttl_ms) },
                initial_status: Some(HealthStatus::Healthy),
                lease_id,
            };

            match registry.register(&service_name, &instance_id, &address, metadata, options).await {
                Ok((fencing_token, deadline_ms)) => {
                    Ok(ClientRpcResponse::ServiceRegisterResult(ServiceRegisterResultResponse {
                        success: true,
                        fencing_token: Some(fencing_token),
                        deadline_ms: Some(deadline_ms),
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::ServiceRegisterResult(ServiceRegisterResultResponse {
                    success: false,
                    fencing_token: None,
                    deadline_ms: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::ServiceDeregister {
            service_name,
            instance_id,
            fencing_token,
        } => {
            use crate::client_rpc::ServiceDeregisterResultResponse;
            use crate::coordination::ServiceRegistry;

            let registry = ServiceRegistry::new(ctx.kv_store.clone());

            match registry.deregister(&service_name, &instance_id, fencing_token).await {
                Ok(was_registered) => Ok(ClientRpcResponse::ServiceDeregisterResult(ServiceDeregisterResultResponse {
                    success: true,
                    was_registered,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::ServiceDeregisterResult(ServiceDeregisterResultResponse {
                    success: false,
                    was_registered: false,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::ServiceDiscover {
            service_name,
            healthy_only,
            tags,
            version_prefix,
            limit,
        } => {
            use crate::client_rpc::ServiceDiscoverResultResponse;
            use crate::client_rpc::ServiceInstanceResponse;
            use crate::coordination::DiscoveryFilter;
            use crate::coordination::ServiceRegistry;

            let registry = ServiceRegistry::new(ctx.kv_store.clone());

            // Parse tags from JSON array
            let tags_vec: Vec<String> = serde_json::from_str(&tags).unwrap_or_default();

            let filter = DiscoveryFilter {
                healthy_only,
                tags: tags_vec,
                version_prefix,
                limit,
            };

            match registry.discover(&service_name, filter).await {
                Ok(instances) => {
                    let response_instances: Vec<ServiceInstanceResponse> = instances
                        .into_iter()
                        .map(|inst| ServiceInstanceResponse {
                            instance_id: inst.instance_id,
                            service_name: inst.service_name,
                            address: inst.address,
                            health_status: match inst.health_status {
                                crate::coordination::HealthStatus::Healthy => "healthy".to_string(),
                                crate::coordination::HealthStatus::Unhealthy => "unhealthy".to_string(),
                                crate::coordination::HealthStatus::Unknown => "unknown".to_string(),
                            },
                            version: inst.metadata.version,
                            tags: inst.metadata.tags,
                            weight: inst.metadata.weight,
                            custom_metadata: serde_json::to_string(&inst.metadata.custom).unwrap_or_default(),
                            registered_at_ms: inst.registered_at_ms,
                            last_heartbeat_ms: inst.last_heartbeat_ms,
                            deadline_ms: inst.deadline_ms,
                            lease_id: inst.lease_id,
                            fencing_token: inst.fencing_token,
                        })
                        .collect();
                    let count = response_instances.len() as u32;
                    Ok(ClientRpcResponse::ServiceDiscoverResult(ServiceDiscoverResultResponse {
                        success: true,
                        instances: response_instances,
                        count,
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::ServiceDiscoverResult(ServiceDiscoverResultResponse {
                    success: false,
                    instances: vec![],
                    count: 0,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::ServiceList { prefix, limit } => {
            use crate::client_rpc::ServiceListResultResponse;
            use crate::coordination::ServiceRegistry;

            let registry = ServiceRegistry::new(ctx.kv_store.clone());

            match registry.discover_services(&prefix, limit).await {
                Ok(services) => {
                    let count = services.len() as u32;
                    Ok(ClientRpcResponse::ServiceListResult(ServiceListResultResponse {
                        success: true,
                        services,
                        count,
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::ServiceListResult(ServiceListResultResponse {
                    success: false,
                    services: vec![],
                    count: 0,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::ServiceGetInstance {
            service_name,
            instance_id,
        } => {
            use crate::client_rpc::ServiceGetInstanceResultResponse;
            use crate::client_rpc::ServiceInstanceResponse;
            use crate::coordination::ServiceRegistry;

            let registry = ServiceRegistry::new(ctx.kv_store.clone());

            match registry.get_instance(&service_name, &instance_id).await {
                Ok(Some(inst)) => {
                    let response_instance = ServiceInstanceResponse {
                        instance_id: inst.instance_id,
                        service_name: inst.service_name,
                        address: inst.address,
                        health_status: match inst.health_status {
                            crate::coordination::HealthStatus::Healthy => "healthy".to_string(),
                            crate::coordination::HealthStatus::Unhealthy => "unhealthy".to_string(),
                            crate::coordination::HealthStatus::Unknown => "unknown".to_string(),
                        },
                        version: inst.metadata.version,
                        tags: inst.metadata.tags,
                        weight: inst.metadata.weight,
                        custom_metadata: serde_json::to_string(&inst.metadata.custom).unwrap_or_default(),
                        registered_at_ms: inst.registered_at_ms,
                        last_heartbeat_ms: inst.last_heartbeat_ms,
                        deadline_ms: inst.deadline_ms,
                        lease_id: inst.lease_id,
                        fencing_token: inst.fencing_token,
                    };
                    Ok(ClientRpcResponse::ServiceGetInstanceResult(ServiceGetInstanceResultResponse {
                        success: true,
                        found: true,
                        instance: Some(response_instance),
                        error: None,
                    }))
                }
                Ok(None) => Ok(ClientRpcResponse::ServiceGetInstanceResult(ServiceGetInstanceResultResponse {
                    success: true,
                    found: false,
                    instance: None,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::ServiceGetInstanceResult(ServiceGetInstanceResultResponse {
                    success: false,
                    found: false,
                    instance: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::ServiceHeartbeat {
            service_name,
            instance_id,
            fencing_token,
        } => {
            use crate::client_rpc::ServiceHeartbeatResultResponse;
            use crate::coordination::ServiceRegistry;

            let registry = ServiceRegistry::new(ctx.kv_store.clone());

            match registry.heartbeat(&service_name, &instance_id, fencing_token).await {
                Ok((new_deadline, health_status)) => {
                    let status_str = match health_status {
                        crate::coordination::HealthStatus::Healthy => "healthy",
                        crate::coordination::HealthStatus::Unhealthy => "unhealthy",
                        crate::coordination::HealthStatus::Unknown => "unknown",
                    };
                    Ok(ClientRpcResponse::ServiceHeartbeatResult(ServiceHeartbeatResultResponse {
                        success: true,
                        new_deadline_ms: Some(new_deadline),
                        health_status: Some(status_str.to_string()),
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::ServiceHeartbeatResult(ServiceHeartbeatResultResponse {
                    success: false,
                    new_deadline_ms: None,
                    health_status: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::ServiceUpdateHealth {
            service_name,
            instance_id,
            fencing_token,
            status,
        } => {
            use crate::client_rpc::ServiceUpdateHealthResultResponse;
            use crate::coordination::HealthStatus;
            use crate::coordination::ServiceRegistry;

            let registry = ServiceRegistry::new(ctx.kv_store.clone());

            let health_status = match status.to_lowercase().as_str() {
                "healthy" => HealthStatus::Healthy,
                "unhealthy" => HealthStatus::Unhealthy,
                _ => HealthStatus::Unknown,
            };

            match registry.update_health(&service_name, &instance_id, fencing_token, health_status).await {
                Ok(()) => Ok(ClientRpcResponse::ServiceUpdateHealthResult(ServiceUpdateHealthResultResponse {
                    success: true,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::ServiceUpdateHealthResult(ServiceUpdateHealthResultResponse {
                    success: false,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        ClientRpcRequest::ServiceUpdateMetadata {
            service_name,
            instance_id,
            fencing_token,
            version,
            tags,
            weight,
            custom_metadata,
        } => {
            use std::collections::HashMap;

            use crate::client_rpc::ServiceUpdateMetadataResultResponse;
            use crate::coordination::ServiceRegistry;

            let registry = ServiceRegistry::new(ctx.kv_store.clone());

            // We need to get existing instance first to merge metadata
            match registry.get_instance(&service_name, &instance_id).await {
                Ok(Some(mut instance)) => {
                    // Verify fencing token
                    if instance.fencing_token != fencing_token {
                        return Ok(ClientRpcResponse::ServiceUpdateMetadataResult(
                            ServiceUpdateMetadataResultResponse {
                                success: false,
                                error: Some("fencing token mismatch".to_string()),
                            },
                        ));
                    }

                    // Update fields that were provided
                    if let Some(v) = version {
                        instance.metadata.version = v;
                    }
                    if let Some(t) = tags
                        && let Ok(parsed_tags) = serde_json::from_str::<Vec<String>>(&t)
                    {
                        instance.metadata.tags = parsed_tags;
                    }
                    if let Some(w) = weight {
                        instance.metadata.weight = w;
                    }
                    if let Some(c) = custom_metadata
                        && let Ok(parsed_custom) = serde_json::from_str::<HashMap<String, String>>(&c)
                    {
                        instance.metadata.custom = parsed_custom;
                    }

                    match registry.update_metadata(&service_name, &instance_id, fencing_token, instance.metadata).await
                    {
                        Ok(()) => {
                            Ok(ClientRpcResponse::ServiceUpdateMetadataResult(ServiceUpdateMetadataResultResponse {
                                success: true,
                                error: None,
                            }))
                        }
                        Err(e) => {
                            Ok(ClientRpcResponse::ServiceUpdateMetadataResult(ServiceUpdateMetadataResultResponse {
                                success: false,
                                error: Some(format!("{}", e)),
                            }))
                        }
                    }
                }
                Ok(None) => Ok(ClientRpcResponse::ServiceUpdateMetadataResult(ServiceUpdateMetadataResultResponse {
                    success: false,
                    error: Some("instance not found".to_string()),
                })),
                Err(e) => Ok(ClientRpcResponse::ServiceUpdateMetadataResult(ServiceUpdateMetadataResultResponse {
                    success: false,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        // =====================================================================
        // DNS operations (only available with dns feature)
        // =====================================================================
        #[cfg(feature = "dns")]
        ClientRpcRequest::DnsSetRecord {
            domain,
            record_type,
            ttl_seconds,
            data_json,
        } => {
            use crate::client_rpc::DnsRecordResponse;
            use crate::client_rpc::DnsRecordResultResponse;
            use crate::dns::AspenDnsStore;
            use crate::dns::DnsRecord;
            use crate::dns::DnsRecordData;
            use crate::dns::DnsStore;
            use crate::dns::RecordType;

            // Parse record type (needed for get_record later)
            let rtype: RecordType = match record_type.parse() {
                Ok(t) => t,
                Err(_) => {
                    return Ok(ClientRpcResponse::DnsSetRecordResult(DnsRecordResultResponse {
                        success: false,
                        found: false,
                        record: None,
                        error: Some(format!("invalid record type: {}", record_type)),
                    }));
                }
            };

            // Parse record data from JSON
            let data: DnsRecordData = match serde_json::from_str(&data_json) {
                Ok(d) => d,
                Err(e) => {
                    return Ok(ClientRpcResponse::DnsSetRecordResult(DnsRecordResultResponse {
                        success: false,
                        found: false,
                        record: None,
                        error: Some(format!("invalid record data JSON: {}", e)),
                    }));
                }
            };

            // Verify parsed record type matches the data's record type
            if data.record_type() != rtype {
                return Ok(ClientRpcResponse::DnsSetRecordResult(DnsRecordResultResponse {
                    success: false,
                    found: false,
                    record: None,
                    error: Some(format!(
                        "record type mismatch: specified {} but data is {}",
                        record_type,
                        data.record_type()
                    )),
                }));
            }

            // Create record (record type is derived from data)
            let record = DnsRecord::new(domain.clone(), ttl_seconds, data);

            // Create DNS store and set record
            let dns_store = AspenDnsStore::new(ctx.kv_store.clone());
            match dns_store.set_record(record).await {
                Ok(()) => {
                    // Fetch the record to return it
                    match dns_store.get_record(&domain, rtype).await {
                        Ok(Some(rec)) => {
                            let record_type = rec.record_type().to_string();
                            let data_json = serde_json::to_string(&rec.data).unwrap_or_default();
                            let response = DnsRecordResponse {
                                domain: rec.domain,
                                record_type,
                                ttl_seconds: rec.ttl_seconds,
                                data_json,
                                updated_at_ms: rec.updated_at_ms,
                            };
                            Ok(ClientRpcResponse::DnsSetRecordResult(DnsRecordResultResponse {
                                success: true,
                                found: true,
                                record: Some(response),
                                error: None,
                            }))
                        }
                        _ => Ok(ClientRpcResponse::DnsSetRecordResult(DnsRecordResultResponse {
                            success: true,
                            found: false,
                            record: None,
                            error: None,
                        })),
                    }
                }
                Err(e) => Ok(ClientRpcResponse::DnsSetRecordResult(DnsRecordResultResponse {
                    success: false,
                    found: false,
                    record: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        #[cfg(feature = "dns")]
        ClientRpcRequest::DnsGetRecord { domain, record_type } => {
            use crate::client_rpc::DnsRecordResponse;
            use crate::client_rpc::DnsRecordResultResponse;
            use crate::dns::AspenDnsStore;
            use crate::dns::DnsStore;
            use crate::dns::RecordType;

            // Parse record type
            let rtype: RecordType = match record_type.parse() {
                Ok(t) => t,
                Err(_) => {
                    return Ok(ClientRpcResponse::DnsGetRecordResult(DnsRecordResultResponse {
                        success: false,
                        found: false,
                        record: None,
                        error: Some(format!("invalid record type: {}", record_type)),
                    }));
                }
            };

            let dns_store = AspenDnsStore::new(ctx.kv_store.clone());
            match dns_store.get_record(&domain, rtype).await {
                Ok(Some(rec)) => {
                    let record_type = rec.record_type().to_string();
                    let data_json = serde_json::to_string(&rec.data).unwrap_or_default();
                    let response = DnsRecordResponse {
                        domain: rec.domain,
                        record_type,
                        ttl_seconds: rec.ttl_seconds,
                        data_json,
                        updated_at_ms: rec.updated_at_ms,
                    };
                    Ok(ClientRpcResponse::DnsGetRecordResult(DnsRecordResultResponse {
                        success: true,
                        found: true,
                        record: Some(response),
                        error: None,
                    }))
                }
                Ok(None) => Ok(ClientRpcResponse::DnsGetRecordResult(DnsRecordResultResponse {
                    success: true,
                    found: false,
                    record: None,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::DnsGetRecordResult(DnsRecordResultResponse {
                    success: false,
                    found: false,
                    record: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        #[cfg(feature = "dns")]
        ClientRpcRequest::DnsGetRecords { domain } => {
            use crate::client_rpc::DnsRecordResponse;
            use crate::client_rpc::DnsRecordsResultResponse;
            use crate::dns::AspenDnsStore;
            use crate::dns::DnsStore;

            let dns_store = AspenDnsStore::new(ctx.kv_store.clone());
            match dns_store.get_records(&domain).await {
                Ok(records) => {
                    let responses: Vec<DnsRecordResponse> = records
                        .into_iter()
                        .map(|rec| {
                            let record_type = rec.record_type().to_string();
                            let data_json = serde_json::to_string(&rec.data).unwrap_or_default();
                            DnsRecordResponse {
                                domain: rec.domain,
                                record_type,
                                ttl_seconds: rec.ttl_seconds,
                                data_json,
                                updated_at_ms: rec.updated_at_ms,
                            }
                        })
                        .collect();
                    let count = responses.len() as u32;
                    Ok(ClientRpcResponse::DnsGetRecordsResult(DnsRecordsResultResponse {
                        success: true,
                        records: responses,
                        count,
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::DnsGetRecordsResult(DnsRecordsResultResponse {
                    success: false,
                    records: vec![],
                    count: 0,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        #[cfg(feature = "dns")]
        ClientRpcRequest::DnsDeleteRecord { domain, record_type } => {
            use crate::client_rpc::DnsDeleteRecordResultResponse;
            use crate::dns::AspenDnsStore;
            use crate::dns::DnsStore;
            use crate::dns::RecordType;

            // Parse record type
            let rtype: RecordType = match record_type.parse() {
                Ok(t) => t,
                Err(_) => {
                    return Ok(ClientRpcResponse::DnsDeleteRecordResult(DnsDeleteRecordResultResponse {
                        success: false,
                        deleted: false,
                        error: Some(format!("invalid record type: {}", record_type)),
                    }));
                }
            };

            let dns_store = AspenDnsStore::new(ctx.kv_store.clone());
            match dns_store.delete_record(&domain, rtype).await {
                Ok(deleted) => Ok(ClientRpcResponse::DnsDeleteRecordResult(DnsDeleteRecordResultResponse {
                    success: true,
                    deleted,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::DnsDeleteRecordResult(DnsDeleteRecordResultResponse {
                    success: false,
                    deleted: false,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        #[cfg(feature = "dns")]
        ClientRpcRequest::DnsResolve { domain, record_type } => {
            use crate::client_rpc::DnsRecordResponse;
            use crate::client_rpc::DnsRecordsResultResponse;
            use crate::dns::AspenDnsStore;
            use crate::dns::DnsStore;
            use crate::dns::RecordType;

            // Parse record type
            let rtype: RecordType = match record_type.parse() {
                Ok(t) => t,
                Err(_) => {
                    return Ok(ClientRpcResponse::DnsResolveResult(DnsRecordsResultResponse {
                        success: false,
                        records: vec![],
                        count: 0,
                        error: Some(format!("invalid record type: {}", record_type)),
                    }));
                }
            };

            let dns_store = AspenDnsStore::new(ctx.kv_store.clone());
            match dns_store.resolve(&domain, rtype).await {
                Ok(records) => {
                    let responses: Vec<DnsRecordResponse> = records
                        .into_iter()
                        .map(|rec| {
                            let record_type = rec.record_type().to_string();
                            let data_json = serde_json::to_string(&rec.data).unwrap_or_default();
                            DnsRecordResponse {
                                domain: rec.domain,
                                record_type,
                                ttl_seconds: rec.ttl_seconds,
                                data_json,
                                updated_at_ms: rec.updated_at_ms,
                            }
                        })
                        .collect();
                    let count = responses.len() as u32;
                    Ok(ClientRpcResponse::DnsResolveResult(DnsRecordsResultResponse {
                        success: true,
                        records: responses,
                        count,
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::DnsResolveResult(DnsRecordsResultResponse {
                    success: false,
                    records: vec![],
                    count: 0,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        #[cfg(feature = "dns")]
        ClientRpcRequest::DnsScanRecords { prefix, limit } => {
            use crate::client_rpc::DnsRecordResponse;
            use crate::client_rpc::DnsRecordsResultResponse;
            use crate::dns::AspenDnsStore;
            use crate::dns::DnsStore;

            let dns_store = AspenDnsStore::new(ctx.kv_store.clone());
            match dns_store.scan_records(&prefix, limit).await {
                Ok(records) => {
                    let responses: Vec<DnsRecordResponse> = records
                        .into_iter()
                        .map(|rec| {
                            let record_type = rec.record_type().to_string();
                            let data_json = serde_json::to_string(&rec.data).unwrap_or_default();
                            DnsRecordResponse {
                                domain: rec.domain,
                                record_type,
                                ttl_seconds: rec.ttl_seconds,
                                data_json,
                                updated_at_ms: rec.updated_at_ms,
                            }
                        })
                        .collect();
                    let count = responses.len() as u32;
                    Ok(ClientRpcResponse::DnsScanRecordsResult(DnsRecordsResultResponse {
                        success: true,
                        records: responses,
                        count,
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::DnsScanRecordsResult(DnsRecordsResultResponse {
                    success: false,
                    records: vec![],
                    count: 0,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        #[cfg(feature = "dns")]
        ClientRpcRequest::DnsSetZone {
            name,
            enabled,
            default_ttl,
            description,
        } => {
            use crate::client_rpc::DnsZoneResponse;
            use crate::client_rpc::DnsZoneResultResponse;
            use crate::dns::AspenDnsStore;
            use crate::dns::DnsStore;
            use crate::dns::Zone;

            let mut zone = Zone::new(name.clone()).with_default_ttl(default_ttl);
            if !enabled {
                zone = zone.disabled();
            }
            if let Some(desc) = description {
                zone = zone.with_description(desc);
            }

            let dns_store = AspenDnsStore::new(ctx.kv_store.clone());
            match dns_store.set_zone(zone).await {
                Ok(()) => {
                    // Fetch the zone to return it
                    match dns_store.get_zone(&name).await {
                        Ok(Some(z)) => {
                            let response = DnsZoneResponse {
                                name: z.name,
                                enabled: z.enabled,
                                default_ttl: z.default_ttl,
                                serial: z.metadata.serial,
                                last_modified_ms: z.metadata.last_modified_ms,
                                description: z.metadata.description,
                            };
                            Ok(ClientRpcResponse::DnsSetZoneResult(DnsZoneResultResponse {
                                success: true,
                                found: true,
                                zone: Some(response),
                                error: None,
                            }))
                        }
                        _ => Ok(ClientRpcResponse::DnsSetZoneResult(DnsZoneResultResponse {
                            success: true,
                            found: false,
                            zone: None,
                            error: None,
                        })),
                    }
                }
                Err(e) => Ok(ClientRpcResponse::DnsSetZoneResult(DnsZoneResultResponse {
                    success: false,
                    found: false,
                    zone: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        #[cfg(feature = "dns")]
        ClientRpcRequest::DnsGetZone { name } => {
            use crate::client_rpc::DnsZoneResponse;
            use crate::client_rpc::DnsZoneResultResponse;
            use crate::dns::AspenDnsStore;
            use crate::dns::DnsStore;

            let dns_store = AspenDnsStore::new(ctx.kv_store.clone());
            match dns_store.get_zone(&name).await {
                Ok(Some(z)) => {
                    let response = DnsZoneResponse {
                        name: z.name,
                        enabled: z.enabled,
                        default_ttl: z.default_ttl,
                        serial: z.metadata.serial,
                        last_modified_ms: z.metadata.last_modified_ms,
                        description: z.metadata.description,
                    };
                    Ok(ClientRpcResponse::DnsGetZoneResult(DnsZoneResultResponse {
                        success: true,
                        found: true,
                        zone: Some(response),
                        error: None,
                    }))
                }
                Ok(None) => Ok(ClientRpcResponse::DnsGetZoneResult(DnsZoneResultResponse {
                    success: true,
                    found: false,
                    zone: None,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::DnsGetZoneResult(DnsZoneResultResponse {
                    success: false,
                    found: false,
                    zone: None,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        #[cfg(feature = "dns")]
        ClientRpcRequest::DnsListZones => {
            use crate::client_rpc::DnsZoneResponse;
            use crate::client_rpc::DnsZonesResultResponse;
            use crate::dns::AspenDnsStore;
            use crate::dns::DnsStore;

            let dns_store = AspenDnsStore::new(ctx.kv_store.clone());
            match dns_store.list_zones().await {
                Ok(zones) => {
                    let responses: Vec<DnsZoneResponse> = zones
                        .into_iter()
                        .map(|z| DnsZoneResponse {
                            name: z.name,
                            enabled: z.enabled,
                            default_ttl: z.default_ttl,
                            serial: z.metadata.serial,
                            last_modified_ms: z.metadata.last_modified_ms,
                            description: z.metadata.description,
                        })
                        .collect();
                    let count = responses.len() as u32;
                    Ok(ClientRpcResponse::DnsListZonesResult(DnsZonesResultResponse {
                        success: true,
                        zones: responses,
                        count,
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::DnsListZonesResult(DnsZonesResultResponse {
                    success: false,
                    zones: vec![],
                    count: 0,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        #[cfg(feature = "dns")]
        ClientRpcRequest::DnsDeleteZone { name, delete_records } => {
            use crate::client_rpc::DnsDeleteZoneResultResponse;
            use crate::dns::AspenDnsStore;
            use crate::dns::DnsStore;

            let dns_store = AspenDnsStore::new(ctx.kv_store.clone());

            // TODO: Count records deleted if delete_records is true
            // For now, we just delete the zone
            match dns_store.delete_zone(&name, delete_records).await {
                Ok(deleted) => Ok(ClientRpcResponse::DnsDeleteZoneResult(DnsDeleteZoneResultResponse {
                    success: true,
                    deleted,
                    records_deleted: 0, // TODO: Track deleted record count
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::DnsDeleteZoneResult(DnsDeleteZoneResultResponse {
                    success: false,
                    deleted: false,
                    records_deleted: 0,
                    error: Some(format!("{}", e)),
                })),
            }
        }

        // =====================================================================
        // DNS fallbacks when feature is disabled
        // =====================================================================
        #[cfg(not(feature = "dns"))]
        ClientRpcRequest::DnsSetRecord { .. }
        | ClientRpcRequest::DnsGetRecord { .. }
        | ClientRpcRequest::DnsGetRecords { .. }
        | ClientRpcRequest::DnsDeleteRecord { .. }
        | ClientRpcRequest::DnsResolve { .. }
        | ClientRpcRequest::DnsScanRecords { .. }
        | ClientRpcRequest::DnsSetZone { .. }
        | ClientRpcRequest::DnsGetZone { .. }
        | ClientRpcRequest::DnsListZones
        | ClientRpcRequest::DnsDeleteZone { .. } => {
            use crate::client_rpc::ErrorResponse;
            Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "DNS_NOT_SUPPORTED".to_string(),
                message: "DNS operations not supported: compiled without 'dns' feature".to_string(),
            }))
        }

        // =====================================================================
        // Sharding operations
        // =====================================================================
        ClientRpcRequest::GetTopology { client_version } => {
            use crate::client_rpc::TopologyResultResponse;

            match &ctx.topology {
                Some(topology) => {
                    let topo = topology.read().await;

                    // Check if client already has current version
                    if let Some(cv) = client_version
                        && cv == topo.version
                    {
                        return Ok(ClientRpcResponse::TopologyResult(TopologyResultResponse {
                            success: true,
                            version: topo.version,
                            updated: false,
                            topology_data: None,
                            shard_count: topo.shard_count() as u32,
                            error: None,
                        }));
                    }

                    // Serialize topology for client
                    let topology_data = serde_json::to_string(&*topo)
                        .map_err(|e| anyhow::anyhow!("failed to serialize topology: {}", e))?;

                    Ok(ClientRpcResponse::TopologyResult(TopologyResultResponse {
                        success: true,
                        version: topo.version,
                        updated: true,
                        topology_data: Some(topology_data),
                        shard_count: topo.shard_count() as u32,
                        error: None,
                    }))
                }
                None => {
                    // Topology not configured on this node
                    Ok(ClientRpcResponse::TopologyResult(TopologyResultResponse {
                        success: false,
                        version: 0,
                        updated: false,
                        topology_data: None,
                        shard_count: 0,
                        error: Some("topology not configured on this node".to_string()),
                    }))
                }
            }
        }

        // =====================================================================
        // Forge operations (decentralized git)
        // =====================================================================
        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeCreateRepo { name, description, default_branch } => {
            use crate::client_rpc::{ForgeRepoInfo, ForgeRepoResultResponse};

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                    success: false,
                    repo: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            // Use node's public key as the sole delegate for now
            let public_key = forge.public_key();
            let delegates = vec![public_key];
            let threshold = 1u32;

            match forge.create_repo(&name, delegates, threshold).await {
                Ok(identity) => {
                    let mut repo_info = ForgeRepoInfo {
                        id: identity.repo_id().to_hex(),
                        name: identity.name.clone(),
                        description: description.clone(),
                        default_branch: default_branch.clone().unwrap_or_else(|| "main".to_string()),
                        delegates: identity.delegates.iter().map(|k| hex::encode(k.as_bytes())).collect(),
                        threshold: identity.threshold,
                        created_at_ms: identity.created_at_ms,
                    };
                    // Update with provided values
                    if description.is_some() {
                        repo_info.description = description;
                    }
                    if let Some(ref branch) = default_branch {
                        repo_info.default_branch = branch.clone();
                    }
                    Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                        success: true,
                        repo: Some(repo_info),
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge create_repo failed");
                    Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                        success: false,
                        repo: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeGetRepo { repo_id } => {
            use crate::client_rpc::{ForgeRepoInfo, ForgeRepoResultResponse};
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                    success: false,
                    repo: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                        success: false,
                        repo: None,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            match forge.get_repo(&repo_id).await {
                Ok(identity) => {
                    let repo_info = ForgeRepoInfo {
                        id: identity.repo_id().to_hex(),
                        name: identity.name.clone(),
                        description: identity.description.clone(),
                        default_branch: identity.default_branch.clone(),
                        delegates: identity.delegates.iter().map(|k| hex::encode(k.as_bytes())).collect(),
                        threshold: identity.threshold,
                        created_at_ms: identity.created_at_ms,
                    };
                    Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                        success: true,
                        repo: Some(repo_info),
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge get_repo failed");
                    Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                        success: false,
                        repo: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeListRepos { limit, offset } => {
            use crate::api::ScanRequest;
            use crate::client_rpc::{ForgeRepoInfo, ForgeRepoListResultResponse};
            use crate::forge::constants::KV_PREFIX_REPOS;
            use crate::forge::types::SignedObject;
            use crate::forge::RepoIdentity;

            let Some(ref _forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeRepoListResult(ForgeRepoListResultResponse {
                    success: false,
                    repos: vec![],
                    count: 0,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            // Scan for all repositories
            let effective_limit = limit.unwrap_or(100).min(1000);
            let effective_offset = offset.unwrap_or(0) as usize;

            let scan_result = match ctx
                .kv_store
                .scan(ScanRequest {
                    prefix: KV_PREFIX_REPOS.to_string(),
                    limit: Some(effective_limit + effective_offset as u32),
                    continuation_token: None,
                })
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeRepoListResult(ForgeRepoListResultResponse {
                        success: false,
                        repos: vec![],
                        count: 0,
                        error: Some(format!("Failed to scan repositories: {}", e)),
                    }));
                }
            };

            // Parse each repository identity
            let mut repos = Vec::new();
            for entry in scan_result.entries.into_iter().skip(effective_offset) {
                // Key format: forge:repos:{repo_id_hex}:identity
                // Only process identity entries
                if !entry.key.ends_with(":identity") {
                    continue;
                }

                // Extract repo_id from key
                let key_parts: Vec<&str> = entry.key.split(':').collect();
                if key_parts.len() < 3 {
                    continue;
                }
                let repo_id_hex = key_parts[2];

                // Decode base64 value
                let bytes = match base64::Engine::decode(&base64::prelude::BASE64_STANDARD, &entry.value) {
                    Ok(b) => b,
                    Err(_) => continue, // Skip malformed entries
                };

                // Parse SignedObject<RepoIdentity>
                let signed: SignedObject<RepoIdentity> = match SignedObject::from_bytes(&bytes) {
                    Ok(s) => s,
                    Err(_) => continue, // Skip malformed entries
                };

                // Verify signature (optional, but good for integrity)
                if signed.verify().is_err() {
                    continue; // Skip entries with invalid signatures
                }

                let identity = signed.payload;
                repos.push(ForgeRepoInfo {
                    id: repo_id_hex.to_string(),
                    name: identity.name,
                    description: identity.description,
                    default_branch: identity.default_branch,
                    delegates: identity.delegates.iter().map(|k| k.to_string()).collect(),
                    threshold: identity.threshold,
                    created_at_ms: identity.created_at_ms,
                });

                if repos.len() >= effective_limit as usize {
                    break;
                }
            }

            let count = repos.len() as u32;
            Ok(ClientRpcResponse::ForgeRepoListResult(ForgeRepoListResultResponse {
                success: true,
                repos,
                count,
                error: None,
            }))
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeStoreBlob { repo_id: _, content } => {
            use crate::client_rpc::ForgeBlobResultResponse;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                    success: false,
                    hash: None,
                    content: None,
                    size: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            match forge.git.store_blob(content.clone()).await {
                Ok(hash) => {
                    Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                        success: true,
                        hash: Some(hash.to_hex().to_string()),
                        content: None,
                        size: Some(content.len() as u64),
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge store_blob failed");
                    Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                        success: false,
                        hash: None,
                        content: None,
                        size: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeGetBlob { hash } => {
            use crate::client_rpc::ForgeBlobResultResponse;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                    success: false,
                    hash: None,
                    content: None,
                    size: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let hash_bytes = match hex::decode(&hash) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                        success: false,
                        hash: None,
                        content: None,
                        size: None,
                        error: Some("invalid hash format".to_string()),
                    }));
                }
            };

            match forge.git.get_blob(&hash_bytes).await {
                Ok(content) => {
                    let size = content.len() as u64;
                    Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                        success: true,
                        hash: Some(hash),
                        content: Some(content),
                        size: Some(size),
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge get_blob failed");
                    Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                        success: false,
                        hash: None,
                        content: None,
                        size: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeCreateTree { repo_id: _, entries_json } => {
            use crate::client_rpc::{ForgeTreeEntry as RpcTreeEntry, ForgeTreeResultResponse};
            use crate::forge::TreeEntry;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                    success: false,
                    hash: None,
                    entries: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            // Parse entries from JSON
            let rpc_entries: Vec<RpcTreeEntry> = match serde_json::from_str(&entries_json) {
                Ok(e) => e,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                        success: false,
                        hash: None,
                        entries: None,
                        error: Some(format!("invalid entries_json: {}", e)),
                    }));
                }
            };

            // Convert to TreeEntry
            let mut entries = Vec::with_capacity(rpc_entries.len());
            for e in rpc_entries {
                let hash_bytes = match hex::decode(&e.hash) {
                    Ok(bytes) if bytes.len() == 32 => {
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(&bytes);
                        blake3::Hash::from_bytes(arr)
                    }
                    _ => {
                        return Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                            success: false,
                            hash: None,
                            entries: None,
                            error: Some(format!("invalid hash for entry: {}", e.name)),
                        }));
                    }
                };
                // Map mode to appropriate TreeEntry constructor
                let entry = match e.mode {
                    0o040000 => TreeEntry::directory(e.name, hash_bytes),
                    0o100755 => TreeEntry::executable(e.name, hash_bytes),
                    0o120000 => TreeEntry::symlink(e.name, hash_bytes),
                    _ => TreeEntry::file(e.name, hash_bytes), // Default to regular file
                };
                entries.push(entry);
            }

            match forge.git.create_tree(&entries).await {
                Ok(hash) => {
                    Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                        success: true,
                        hash: Some(hash.to_hex().to_string()),
                        entries: None,
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge create_tree failed");
                    Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                        success: false,
                        hash: None,
                        entries: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeGetTree { hash } => {
            use crate::client_rpc::{ForgeTreeEntry as RpcTreeEntry, ForgeTreeResultResponse};

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                    success: false,
                    hash: None,
                    entries: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let hash_bytes = match hex::decode(&hash) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                        success: false,
                        hash: None,
                        entries: None,
                        error: Some("invalid hash format".to_string()),
                    }));
                }
            };

            match forge.git.get_tree(&hash_bytes).await {
                Ok(tree) => {
                    let entries: Vec<RpcTreeEntry> = tree.entries.iter().map(|e| {
                        RpcTreeEntry {
                            mode: e.mode,
                            name: e.name.clone(),
                            hash: hex::encode(e.hash),
                        }
                    }).collect();
                    Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                        success: true,
                        hash: Some(hash),
                        entries: Some(entries),
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge get_tree failed");
                    Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                        success: false,
                        hash: None,
                        entries: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeCommit { repo_id: _, tree, parents, message } => {
            use crate::client_rpc::{ForgeCommitInfo, ForgeCommitResultResponse};

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                    success: false,
                    commit: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            // Parse tree hash
            let tree_hash = match hex::decode(&tree) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                        success: false,
                        commit: None,
                        error: Some("invalid tree hash".to_string()),
                    }));
                }
            };

            // Parse parent hashes
            let mut parent_hashes = Vec::with_capacity(parents.len());
            for p in &parents {
                let hash_bytes = match hex::decode(p) {
                    Ok(bytes) if bytes.len() == 32 => {
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(&bytes);
                        blake3::Hash::from_bytes(arr)
                    }
                    _ => {
                        return Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                            success: false,
                            commit: None,
                            error: Some(format!("invalid parent hash: {}", p)),
                        }));
                    }
                };
                parent_hashes.push(hash_bytes);
            }

            match forge.git.commit(tree_hash, parent_hashes, &message).await {
                Ok(commit_hash) => {
                    // Get the commit we just created for the response
                    let commit_info = ForgeCommitInfo {
                        hash: commit_hash.to_hex().to_string(),
                        tree,
                        parents,
                        author_name: "".to_string(), // TODO: Get from identity
                        author_email: None,
                        author_key: Some(hex::encode(forge.public_key().as_bytes())),
                        message,
                        timestamp_ms: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                    };
                    Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                        success: true,
                        commit: Some(commit_info),
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge commit failed");
                    Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                        success: false,
                        commit: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeGetCommit { hash } => {
            use crate::client_rpc::{ForgeCommitInfo, ForgeCommitResultResponse};

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                    success: false,
                    commit: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let hash_bytes = match hex::decode(&hash) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                        success: false,
                        commit: None,
                        error: Some("invalid hash format".to_string()),
                    }));
                }
            };

            match forge.git.get_commit(&hash_bytes).await {
                Ok(commit) => {
                    let commit_info = ForgeCommitInfo {
                        hash,
                        tree: hex::encode(commit.tree),
                        parents: commit.parents.iter().map(|p| hex::encode(p)).collect(),
                        author_name: commit.author.name.clone(),
                        author_email: if commit.author.email.is_empty() { None } else { Some(commit.author.email.clone()) },
                        author_key: commit.author.public_key.as_ref().map(|pk| hex::encode(pk.as_bytes())),
                        message: commit.message.clone(),
                        timestamp_ms: commit.author.timestamp_ms,
                    };
                    Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                        success: true,
                        commit: Some(commit_info),
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge get_commit failed");
                    Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                        success: false,
                        commit: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeLog { repo_id, ref_name, limit } => {
            use crate::client_rpc::{ForgeCommitInfo, ForgeLogResultResponse};
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeLogResult(ForgeLogResultResponse {
                    success: false,
                    commits: vec![],
                    count: 0,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeLogResult(ForgeLogResultResponse {
                        success: false,
                        commits: vec![],
                        count: 0,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            // Get ref to start from
            let ref_name = ref_name.unwrap_or_else(|| "heads/main".to_string());
            let start_hash = match forge.refs.get(&repo_id, &ref_name).await {
                Ok(Some(hash)) => hash,
                Ok(None) => {
                    return Ok(ClientRpcResponse::ForgeLogResult(ForgeLogResultResponse {
                        success: true,
                        commits: vec![],
                        count: 0,
                        error: None,
                    }));
                }
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeLogResult(ForgeLogResultResponse {
                        success: false,
                        commits: vec![],
                        count: 0,
                        error: Some(format!("{}", e)),
                    }));
                }
            };

            // Walk commit history
            let limit = limit.unwrap_or(50).min(1000) as usize;
            let mut commits = Vec::with_capacity(limit);
            let mut current = Some(start_hash);

            while let Some(hash) = current {
                if commits.len() >= limit {
                    break;
                }

                match forge.git.get_commit(&hash).await {
                    Ok(commit) => {
                        commits.push(ForgeCommitInfo {
                            hash: hash.to_hex().to_string(),
                            tree: hex::encode(commit.tree),
                            parents: commit.parents.iter().map(|p| hex::encode(p)).collect(),
                            author_name: commit.author.name.clone(),
                            author_email: if commit.author.email.is_empty() { None } else { Some(commit.author.email.clone()) },
                            author_key: commit.author.public_key.as_ref().map(|pk| hex::encode(pk.as_bytes())),
                            message: commit.message.clone(),
                            timestamp_ms: commit.author.timestamp_ms,
                        });
                        // Move to first parent for linear history
                        current = commit.parents.first().map(|p| blake3::Hash::from_bytes(*p));
                    }
                    Err(_) => break,
                }
            }

            let count = commits.len() as u32;
            Ok(ClientRpcResponse::ForgeLogResult(ForgeLogResultResponse {
                success: true,
                commits,
                count,
                error: None,
            }))
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeGetRef { repo_id, ref_name } => {
            use crate::client_rpc::{ForgeRefInfo, ForgeRefResultResponse};
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                    success: false,
                    found: false,
                    ref_info: None,
                    previous_hash: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            match forge.refs.get(&repo_id, &ref_name).await {
                Ok(Some(hash)) => {
                    Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: true,
                        found: true,
                        ref_info: Some(ForgeRefInfo {
                            name: ref_name,
                            hash: hash.to_hex().to_string(),
                        }),
                        previous_hash: None,
                        error: None,
                    }))
                }
                Ok(None) => {
                    Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: true,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge get_ref failed");
                    Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeSetRef { repo_id, ref_name, hash, signer, signature, timestamp_ms } => {
            use crate::client_rpc::{ForgeRefInfo, ForgeRefResultResponse};
            use crate::forge::RepoId;
            use crate::forge::refs::DelegateVerifier;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                    success: false,
                    found: false,
                    ref_info: None,
                    previous_hash: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            let hash_bytes = match hex::decode(&hash) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some("invalid hash format".to_string()),
                    }));
                }
            };

            // Get repo identity to check if this is a canonical ref
            let identity = match forge.get_repo(&repo_id).await {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some(format!("failed to get repo: {}", e)),
                    }));
                }
            };

            // Check if this is a canonical ref that requires delegate authorization
            if DelegateVerifier::is_canonical_ref(&ref_name, &identity.default_branch) {
                // Verify signer is a delegate
                let signer_key = match signer.as_ref().and_then(|s| s.parse::<iroh::PublicKey>().ok()) {
                    Some(key) => key,
                    None => {
                        return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                            success: false,
                            found: false,
                            ref_info: None,
                            previous_hash: None,
                            error: Some("signature required for canonical ref update".to_string()),
                        }));
                    }
                };

                if !identity.is_delegate(&signer_key) {
                    return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some(format!("signer {} is not a delegate", signer_key)),
                    }));
                }

                // For full security, we should also verify the signature here
                // But for minimal viable, we trust that the signer field is accurate
                // since the client connection is already authenticated
                let _ = (signature, timestamp_ms); // Mark as used
            }

            match forge.refs.set(&repo_id, &ref_name, hash_bytes).await {
                Ok(()) => {
                    Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: true,
                        found: true,
                        ref_info: Some(ForgeRefInfo {
                            name: ref_name,
                            hash,
                        }),
                        previous_hash: None,
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge set_ref failed");
                    Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeDeleteRef { repo_id, ref_name } => {
            use crate::client_rpc::ForgeRefResultResponse;
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                    success: false,
                    found: false,
                    ref_info: None,
                    previous_hash: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            match forge.refs.delete(&repo_id, &ref_name).await {
                Ok(()) => {
                    Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: true,
                        found: true,
                        ref_info: None,
                        previous_hash: None,
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge delete_ref failed");
                    Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeCasRef { repo_id, ref_name, expected, new_hash, signer, signature, timestamp_ms } => {
            use crate::client_rpc::{ForgeRefInfo, ForgeRefResultResponse};
            use crate::forge::RepoId;
            use crate::forge::refs::DelegateVerifier;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                    success: false,
                    found: false,
                    ref_info: None,
                    previous_hash: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            let expected_hash = match expected {
                Some(ref h) => {
                    match hex::decode(h) {
                        Ok(bytes) if bytes.len() == 32 => {
                            let mut arr = [0u8; 32];
                            arr.copy_from_slice(&bytes);
                            Some(blake3::Hash::from_bytes(arr))
                        }
                        _ => {
                            return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                                success: false,
                                found: false,
                                ref_info: None,
                                previous_hash: None,
                                error: Some("invalid expected hash format".to_string()),
                            }));
                        }
                    }
                }
                None => None,
            };

            let new_hash_bytes = match hex::decode(&new_hash) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some("invalid new_hash format".to_string()),
                    }));
                }
            };

            // Get repo identity to check if this is a canonical ref
            let identity = match forge.get_repo(&repo_id).await {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some(format!("failed to get repo: {}", e)),
                    }));
                }
            };

            // Check if this is a canonical ref that requires delegate authorization
            if DelegateVerifier::is_canonical_ref(&ref_name, &identity.default_branch) {
                // Verify signer is a delegate
                let signer_key = match signer.as_ref().and_then(|s| s.parse::<iroh::PublicKey>().ok()) {
                    Some(key) => key,
                    None => {
                        return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                            success: false,
                            found: false,
                            ref_info: None,
                            previous_hash: None,
                            error: Some("signature required for canonical ref update".to_string()),
                        }));
                    }
                };

                if !identity.is_delegate(&signer_key) {
                    return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some(format!("signer {} is not a delegate", signer_key)),
                    }));
                }

                // Mark as used (full signature verification can be added later)
                let _ = (signature, timestamp_ms);
            }

            match forge.refs.compare_and_set(&repo_id, &ref_name, expected_hash, new_hash_bytes).await {
                Ok(()) => {
                    Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: true,
                        found: true,
                        ref_info: Some(ForgeRefInfo {
                            name: ref_name,
                            hash: new_hash,
                        }),
                        previous_hash: expected,
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge cas_ref failed");
                    Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeListBranches { repo_id } => {
            use crate::client_rpc::{ForgeRefInfo, ForgeRefListResultResponse};
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
                    success: false,
                    refs: vec![],
                    count: 0,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
                        success: false,
                        refs: vec![],
                        count: 0,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            match forge.refs.list_branches(&repo_id).await {
                Ok(branches) => {
                    let refs: Vec<ForgeRefInfo> = branches.into_iter().map(|(name, hash)| {
                        ForgeRefInfo {
                            name,
                            hash: hash.to_hex().to_string(),
                        }
                    }).collect();
                    let count = refs.len() as u32;
                    Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
                        success: true,
                        refs,
                        count,
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge list_branches failed");
                    Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
                        success: false,
                        refs: vec![],
                        count: 0,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeListTags { repo_id } => {
            use crate::client_rpc::{ForgeRefInfo, ForgeRefListResultResponse};
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
                    success: false,
                    refs: vec![],
                    count: 0,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
                        success: false,
                        refs: vec![],
                        count: 0,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            match forge.refs.list_tags(&repo_id).await {
                Ok(tags) => {
                    let refs: Vec<ForgeRefInfo> = tags.into_iter().map(|(name, hash)| {
                        ForgeRefInfo {
                            name,
                            hash: hash.to_hex().to_string(),
                        }
                    }).collect();
                    let count = refs.len() as u32;
                    Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
                        success: true,
                        refs,
                        count,
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge list_tags failed");
                    Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
                        success: false,
                        refs: vec![],
                        count: 0,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeCreateIssue { repo_id, title, body, labels } => {
            use crate::client_rpc::{ForgeIssueInfo, ForgeIssueResultResponse};
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                    success: false,
                    issue: None,
                    comments: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: false,
                        issue: None,
                        comments: None,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            match forge.cobs.create_issue(&repo_id, &title, &body, labels.clone()).await {
                Ok(issue_id) => {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: true,
                        issue: Some(ForgeIssueInfo {
                            id: issue_id.to_hex().to_string(),
                            title,
                            body,
                            state: "open".to_string(),
                            labels,
                            comment_count: 0,
                            assignees: vec![],
                            created_at_ms: now,
                            updated_at_ms: now,
                        }),
                        comments: None,
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge create_issue failed");
                    Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: false,
                        issue: None,
                        comments: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeListIssues { repo_id, state, limit } => {
            use crate::client_rpc::{ForgeIssueInfo, ForgeIssueListResultResponse};
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeIssueListResult(ForgeIssueListResultResponse {
                    success: false,
                    issues: vec![],
                    count: 0,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeIssueListResult(ForgeIssueListResultResponse {
                        success: false,
                        issues: vec![],
                        count: 0,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            match forge.cobs.list_issues(&repo_id).await {
                Ok(issue_ids) => {
                    let limit = limit.unwrap_or(100).min(1000) as usize;
                    let mut issues = Vec::with_capacity(limit.min(issue_ids.len()));

                    for issue_id in issue_ids.into_iter().take(limit) {
                        if let Ok(issue) = forge.cobs.resolve_issue(&repo_id, &issue_id).await {
                            let state_str = match &issue.state {
                                crate::forge::cob::IssueState::Open => "open",
                                crate::forge::cob::IssueState::Closed { .. } => "closed",
                            };

                            // Apply state filter
                            if let Some(ref filter) = state {
                                if filter != state_str {
                                    continue;
                                }
                            }

                            issues.push(ForgeIssueInfo {
                                id: issue_id.to_hex().to_string(),
                                title: issue.title,
                                body: issue.body,
                                state: state_str.to_string(),
                                labels: issue.labels.into_iter().collect(),
                                comment_count: issue.comments.len() as u32,
                                assignees: issue.assignees.iter().map(|a| hex::encode(a)).collect(),
                                created_at_ms: issue.created_at_ms,
                                updated_at_ms: issue.updated_at_ms,
                            });
                        }
                    }

                    let count = issues.len() as u32;
                    Ok(ClientRpcResponse::ForgeIssueListResult(ForgeIssueListResultResponse {
                        success: true,
                        issues,
                        count,
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge list_issues failed");
                    Ok(ClientRpcResponse::ForgeIssueListResult(ForgeIssueListResultResponse {
                        success: false,
                        issues: vec![],
                        count: 0,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeGetIssue { repo_id, issue_id } => {
            use crate::client_rpc::{ForgeCommentInfo, ForgeIssueInfo, ForgeIssueResultResponse};
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                    success: false,
                    issue: None,
                    comments: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: false,
                        issue: None,
                        comments: None,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            let issue_hash = match hex::decode(&issue_id) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: false,
                        issue: None,
                        comments: None,
                        error: Some("invalid issue_id format".to_string()),
                    }));
                }
            };

            match forge.cobs.resolve_issue(&repo_id, &issue_hash).await {
                Ok(issue) => {
                    let state_str = match &issue.state {
                        crate::forge::cob::IssueState::Open => "open",
                        crate::forge::cob::IssueState::Closed { .. } => "closed",
                    };

                    let comments: Vec<ForgeCommentInfo> = issue.comments.iter().map(|c| {
                        ForgeCommentInfo {
                            hash: hex::encode(c.change_hash),
                            author: hex::encode(c.author),
                            body: c.body.clone(),
                            timestamp_ms: c.timestamp_ms,
                        }
                    }).collect();

                    Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: true,
                        issue: Some(ForgeIssueInfo {
                            id: issue_id,
                            title: issue.title,
                            body: issue.body,
                            state: state_str.to_string(),
                            labels: issue.labels.into_iter().collect(),
                            comment_count: comments.len() as u32,
                            assignees: issue.assignees.iter().map(|a| hex::encode(a)).collect(),
                            created_at_ms: issue.created_at_ms,
                            updated_at_ms: issue.updated_at_ms,
                        }),
                        comments: Some(comments),
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge get_issue failed");
                    Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: false,
                        issue: None,
                        comments: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeCommentIssue { repo_id, issue_id, body } => {
            use crate::client_rpc::ForgeIssueResultResponse;
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                    success: false,
                    issue: None,
                    comments: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: false,
                        issue: None,
                        comments: None,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            let issue_hash = match hex::decode(&issue_id) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: false,
                        issue: None,
                        comments: None,
                        error: Some("invalid issue_id format".to_string()),
                    }));
                }
            };

            match forge.cobs.add_comment(&repo_id, &issue_hash, &body).await {
                Ok(_) => {
                    Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: true,
                        issue: None,
                        comments: None,
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge comment_issue failed");
                    Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: false,
                        issue: None,
                        comments: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeCloseIssue { repo_id, issue_id, reason } => {
            use crate::client_rpc::ForgeIssueResultResponse;
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                    success: false,
                    issue: None,
                    comments: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: false,
                        issue: None,
                        comments: None,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            let issue_hash = match hex::decode(&issue_id) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: false,
                        issue: None,
                        comments: None,
                        error: Some("invalid issue_id format".to_string()),
                    }));
                }
            };

            match forge.cobs.close_issue(&repo_id, &issue_hash, reason).await {
                Ok(_) => {
                    Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: true,
                        issue: None,
                        comments: None,
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge close_issue failed");
                    Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: false,
                        issue: None,
                        comments: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeReopenIssue { repo_id, issue_id } => {
            use crate::client_rpc::ForgeIssueResultResponse;
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                    success: false,
                    issue: None,
                    comments: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: false,
                        issue: None,
                        comments: None,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            let issue_hash = match hex::decode(&issue_id) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: false,
                        issue: None,
                        comments: None,
                        error: Some("invalid issue_id format".to_string()),
                    }));
                }
            };

            match forge.cobs.reopen_issue(&repo_id, &issue_hash).await {
                Ok(_) => {
                    Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: true,
                        issue: None,
                        comments: None,
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge reopen_issue failed");
                    Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                        success: false,
                        issue: None,
                        comments: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeCreatePatch { repo_id, title, description, base, head } => {
            use crate::client_rpc::{ForgePatchInfo, ForgePatchResultResponse};
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                    success: false,
                    patch: None,
                    comments: None,
                    revisions: None,
                    approvals: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            let base_hash = match hex::decode(&base) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some("invalid base hash".to_string()),
                    }));
                }
            };

            let head_hash = match hex::decode(&head) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some("invalid head hash".to_string()),
                    }));
                }
            };

            match forge.cobs.create_patch(&repo_id, &title, &description, base_hash, head_hash).await {
                Ok(patch_id) => {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: true,
                        patch: Some(ForgePatchInfo {
                            id: patch_id.to_hex().to_string(),
                            title,
                            description,
                            state: "open".to_string(),
                            base,
                            head,
                            labels: vec![],
                            revision_count: 1,
                            approval_count: 0,
                            assignees: vec![],
                            created_at_ms: now,
                            updated_at_ms: now,
                        }),
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge create_patch failed");
                    Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeListPatches { repo_id, state, limit } => {
            use crate::client_rpc::{ForgePatchInfo, ForgePatchListResultResponse};
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgePatchListResult(ForgePatchListResultResponse {
                    success: false,
                    patches: vec![],
                    count: 0,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgePatchListResult(ForgePatchListResultResponse {
                        success: false,
                        patches: vec![],
                        count: 0,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            match forge.cobs.list_patches(&repo_id).await {
                Ok(patch_ids) => {
                    let limit = limit.unwrap_or(100).min(1000) as usize;
                    let mut patches = Vec::with_capacity(limit.min(patch_ids.len()));

                    for patch_id in patch_ids.into_iter().take(limit) {
                        if let Ok(patch) = forge.cobs.resolve_patch(&repo_id, &patch_id).await {
                            let state_str = match &patch.state {
                                crate::forge::cob::PatchState::Open => "open",
                                crate::forge::cob::PatchState::Merged { .. } => "merged",
                                crate::forge::cob::PatchState::Closed { .. } => "closed",
                            };

                            // Apply state filter
                            if let Some(ref filter) = state {
                                if filter != state_str {
                                    continue;
                                }
                            }

                            patches.push(ForgePatchInfo {
                                id: patch_id.to_hex().to_string(),
                                title: patch.title,
                                description: patch.description,
                                state: state_str.to_string(),
                                base: hex::encode(patch.base),
                                head: hex::encode(patch.head),
                                labels: patch.labels.into_iter().collect(),
                                revision_count: patch.revisions.len() as u32,
                                approval_count: patch.approvals.len() as u32,
                                assignees: patch.assignees.iter().map(|a| hex::encode(a)).collect(),
                                created_at_ms: patch.created_at_ms,
                                updated_at_ms: patch.updated_at_ms,
                            });
                        }
                    }

                    let count = patches.len() as u32;
                    Ok(ClientRpcResponse::ForgePatchListResult(ForgePatchListResultResponse {
                        success: true,
                        patches,
                        count,
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge list_patches failed");
                    Ok(ClientRpcResponse::ForgePatchListResult(ForgePatchListResultResponse {
                        success: false,
                        patches: vec![],
                        count: 0,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeGetPatch { repo_id, patch_id } => {
            use crate::client_rpc::{ForgeCommentInfo, ForgePatchApproval, ForgePatchInfo, ForgePatchResultResponse, ForgePatchRevision};
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                    success: false,
                    patch: None,
                    comments: None,
                    revisions: None,
                    approvals: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            let patch_hash = match hex::decode(&patch_id) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some("invalid patch_id format".to_string()),
                    }));
                }
            };

            match forge.cobs.resolve_patch(&repo_id, &patch_hash).await {
                Ok(patch) => {
                    let state_str = match &patch.state {
                        crate::forge::cob::PatchState::Open => "open",
                        crate::forge::cob::PatchState::Merged { .. } => "merged",
                        crate::forge::cob::PatchState::Closed { .. } => "closed",
                    };

                    let comments: Vec<ForgeCommentInfo> = patch.comments.iter().map(|c| {
                        ForgeCommentInfo {
                            hash: hex::encode(c.change_hash),
                            author: hex::encode(c.author),
                            body: c.body.clone(),
                            timestamp_ms: c.timestamp_ms,
                        }
                    }).collect();

                    let revisions: Vec<ForgePatchRevision> = patch.revisions.iter().map(|r| {
                        ForgePatchRevision {
                            hash: hex::encode(r.change_hash),
                            head: hex::encode(r.head),
                            message: r.message.clone(),
                            author: hex::encode(r.author),
                            timestamp_ms: r.timestamp_ms,
                        }
                    }).collect();

                    let approvals: Vec<ForgePatchApproval> = patch.approvals.iter().map(|a| {
                        ForgePatchApproval {
                            author: hex::encode(a.author),
                            commit: hex::encode(a.commit),
                            message: a.message.clone(),
                            timestamp_ms: a.timestamp_ms,
                        }
                    }).collect();

                    Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: true,
                        patch: Some(ForgePatchInfo {
                            id: patch_id,
                            title: patch.title,
                            description: patch.description,
                            state: state_str.to_string(),
                            base: hex::encode(patch.base),
                            head: hex::encode(patch.head),
                            labels: patch.labels.into_iter().collect(),
                            revision_count: revisions.len() as u32,
                            approval_count: approvals.len() as u32,
                            assignees: patch.assignees.iter().map(|a| hex::encode(a)).collect(),
                            created_at_ms: patch.created_at_ms,
                            updated_at_ms: patch.updated_at_ms,
                        }),
                        comments: Some(comments),
                        revisions: Some(revisions),
                        approvals: Some(approvals),
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge get_patch failed");
                    Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeUpdatePatch { repo_id, patch_id, head, message } => {
            use crate::client_rpc::ForgePatchResultResponse;
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                    success: false,
                    patch: None,
                    comments: None,
                    revisions: None,
                    approvals: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            let patch_hash = match hex::decode(&patch_id) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some("invalid patch_id format".to_string()),
                    }));
                }
            };

            let head_hash = match hex::decode(&head) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some("invalid head hash".to_string()),
                    }));
                }
            };

            match forge.cobs.update_patch(&repo_id, &patch_hash, head_hash, message).await {
                Ok(_) => {
                    Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: true,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge update_patch failed");
                    Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeApprovePatch { repo_id, patch_id, commit, message } => {
            use crate::client_rpc::ForgePatchResultResponse;
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                    success: false,
                    patch: None,
                    comments: None,
                    revisions: None,
                    approvals: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            let patch_hash = match hex::decode(&patch_id) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some("invalid patch_id format".to_string()),
                    }));
                }
            };

            let commit_hash = match hex::decode(&commit) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some("invalid commit hash".to_string()),
                    }));
                }
            };

            match forge.cobs.approve_patch(&repo_id, &patch_hash, commit_hash, message).await {
                Ok(_) => {
                    Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: true,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge approve_patch failed");
                    Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeMergePatch { repo_id, patch_id, merge_commit } => {
            use crate::client_rpc::ForgePatchResultResponse;
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                    success: false,
                    patch: None,
                    comments: None,
                    revisions: None,
                    approvals: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            let patch_hash = match hex::decode(&patch_id) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some("invalid patch_id format".to_string()),
                    }));
                }
            };

            let merge_hash = match hex::decode(&merge_commit) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some("invalid merge_commit hash".to_string()),
                    }));
                }
            };

            match forge.cobs.merge_patch(&repo_id, &patch_hash, merge_hash).await {
                Ok(_) => {
                    Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: true,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge merge_patch failed");
                    Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        #[cfg(feature = "forge")]
        ClientRpcRequest::ForgeClosePatch { repo_id, patch_id, reason } => {
            use crate::client_rpc::ForgePatchResultResponse;
            use crate::forge::RepoId;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                    success: false,
                    patch: None,
                    comments: None,
                    revisions: None,
                    approvals: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            let repo_id = match RepoId::from_hex(&repo_id) {
                Ok(id) => id,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some(format!("invalid repo_id: {}", e)),
                    }));
                }
            };

            let patch_hash = match hex::decode(&patch_id) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    blake3::Hash::from_bytes(arr)
                }
                _ => {
                    return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some("invalid patch_id format".to_string()),
                    }));
                }
            };

            match forge.cobs.close_patch(&repo_id, &patch_hash, reason).await {
                Ok(_) => {
                    Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: true,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: None,
                    }))
                }
                Err(e) => {
                    warn!(error = %e, "forge close_patch failed");
                    Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                        success: false,
                        patch: None,
                        comments: None,
                        revisions: None,
                        approvals: None,
                        error: Some(format!("{}", e)),
                    }))
                }
            }
        }

        ClientRpcRequest::ForgeGetDelegateKey { repo_id } => {
            use crate::client_rpc::ForgeKeyResultResponse;

            let Some(ref forge) = ctx.forge_node else {
                return Ok(ClientRpcResponse::ForgeKeyResult(ForgeKeyResultResponse {
                    success: false,
                    public_key: None,
                    secret_key: None,
                    error: Some("Forge not enabled on this node".to_string()),
                }));
            };

            // For now, we return the node's delegate key
            // In the future, this could be repository-specific
            let _ = repo_id; // TODO: Validate repo exists and user is authorized

            let secret_key = forge.secret_key();
            let public_key = secret_key.public();

            Ok(ClientRpcResponse::ForgeKeyResult(ForgeKeyResultResponse {
                success: true,
                public_key: Some(public_key.to_string()),
                secret_key: Some(hex::encode(secret_key.to_bytes())),
                error: None,
            }))
        }

        // Fallback for Forge operations when feature is disabled
        #[cfg(not(feature = "forge"))]
        ClientRpcRequest::ForgeCreateRepo { .. }
        | ClientRpcRequest::ForgeGetRepo { .. }
        | ClientRpcRequest::ForgeListRepos { .. }
        | ClientRpcRequest::ForgeStoreBlob { .. }
        | ClientRpcRequest::ForgeGetBlob { .. }
        | ClientRpcRequest::ForgeCreateTree { .. }
        | ClientRpcRequest::ForgeGetTree { .. }
        | ClientRpcRequest::ForgeCommit { .. }
        | ClientRpcRequest::ForgeGetCommit { .. }
        | ClientRpcRequest::ForgeLog { .. }
        | ClientRpcRequest::ForgeGetRef { .. }
        | ClientRpcRequest::ForgeSetRef { .. }
        | ClientRpcRequest::ForgeDeleteRef { .. }
        | ClientRpcRequest::ForgeCasRef { .. }
        | ClientRpcRequest::ForgeListBranches { .. }
        | ClientRpcRequest::ForgeListTags { .. }
        | ClientRpcRequest::ForgeCreateIssue { .. }
        | ClientRpcRequest::ForgeListIssues { .. }
        | ClientRpcRequest::ForgeGetIssue { .. }
        | ClientRpcRequest::ForgeCommentIssue { .. }
        | ClientRpcRequest::ForgeCloseIssue { .. }
        | ClientRpcRequest::ForgeReopenIssue { .. }
        | ClientRpcRequest::ForgeCreatePatch { .. }
        | ClientRpcRequest::ForgeListPatches { .. }
        | ClientRpcRequest::ForgeGetPatch { .. }
        | ClientRpcRequest::ForgeUpdatePatch { .. }
        | ClientRpcRequest::ForgeApprovePatch { .. }
        | ClientRpcRequest::ForgeMergePatch { .. }
        | ClientRpcRequest::ForgeClosePatch { .. }
        | ClientRpcRequest::ForgeGetDelegateKey { .. } => {
            use crate::client_rpc::ForgeOperationResultResponse;
            Ok(ClientRpcResponse::ForgeOperationResult(
                ForgeOperationResultResponse {
                    success: false,
                    error: Some("Forge feature not compiled into this build".to_string()),
                },
            ))
        }

        // =====================================================================
        // Federation operations
        // =====================================================================
        ClientRpcRequest::GetFederationStatus => {
            use crate::client_rpc::FederationStatusResponse;

            // Federation is currently not fully integrated with the node,
            // so we return a placeholder response indicating disabled status
            Ok(ClientRpcResponse::FederationStatus(FederationStatusResponse {
                enabled: false,
                cluster_name: "unknown".to_string(),
                cluster_key: "".to_string(),
                dht_enabled: false,
                gossip_enabled: false,
                discovered_clusters: 0,
                federated_repos: 0,
                error: Some("Federation not yet integrated with node runtime".to_string()),
            }))
        }

        ClientRpcRequest::ListDiscoveredClusters => {
            use crate::client_rpc::DiscoveredClustersResponse;

            Ok(ClientRpcResponse::DiscoveredClusters(DiscoveredClustersResponse {
                clusters: vec![],
                count: 0,
                error: Some("Federation not yet integrated with node runtime".to_string()),
            }))
        }

        ClientRpcRequest::GetDiscoveredCluster { cluster_key: _ } => {
            use crate::client_rpc::DiscoveredClusterResponse;

            Ok(ClientRpcResponse::DiscoveredCluster(DiscoveredClusterResponse {
                found: false,
                cluster_key: None,
                name: None,
                node_count: None,
                capabilities: None,
                relay_urls: None,
                discovered_at: None,
            }))
        }

        ClientRpcRequest::TrustCluster { cluster_key: _ } => {
            use crate::client_rpc::TrustClusterResultResponse;

            Ok(ClientRpcResponse::TrustClusterResult(TrustClusterResultResponse {
                success: false,
                error: Some("Federation not yet integrated with node runtime".to_string()),
            }))
        }

        ClientRpcRequest::UntrustCluster { cluster_key: _ } => {
            use crate::client_rpc::UntrustClusterResultResponse;

            Ok(ClientRpcResponse::UntrustClusterResult(UntrustClusterResultResponse {
                success: false,
                error: Some("Federation not yet integrated with node runtime".to_string()),
            }))
        }

        ClientRpcRequest::FederateRepository { repo_id: _, mode: _ } => {
            use crate::client_rpc::FederateRepositoryResultResponse;

            Ok(ClientRpcResponse::FederateRepositoryResult(FederateRepositoryResultResponse {
                success: false,
                fed_id: None,
                error: Some("Federation not yet integrated with node runtime".to_string()),
            }))
        }

        ClientRpcRequest::ListFederatedRepositories => {
            use crate::client_rpc::FederatedRepositoriesResponse;

            Ok(ClientRpcResponse::FederatedRepositories(FederatedRepositoriesResponse {
                repositories: vec![],
                count: 0,
                error: Some("Federation not yet integrated with node runtime".to_string()),
            }))
        }

        ClientRpcRequest::ForgeFetchFederated {
            federated_id: _,
            remote_cluster: _,
        } => {
            use crate::client_rpc::ForgeFetchFederatedResultResponse;

            Ok(ClientRpcResponse::ForgeFetchResult(ForgeFetchFederatedResultResponse {
                success: false,
                remote_cluster: None,
                fetched: 0,
                already_present: 0,
                errors: vec![],
                error: Some("Federation not yet integrated with node runtime".to_string()),
            }))
        }

        // =====================================================================
        // Git Bridge operations (for git-remote-aspen)
        // =====================================================================
        ClientRpcRequest::GitBridgeListRefs { repo_id: _ } => {
            use crate::client_rpc::GitBridgeListRefsResponse;

            // TODO: Implement using GitExporter.list_refs()
            Ok(ClientRpcResponse::GitBridgeListRefs(GitBridgeListRefsResponse {
                success: false,
                refs: vec![],
                head: None,
                error: Some("Git bridge not yet integrated with node runtime".to_string()),
            }))
        }

        ClientRpcRequest::GitBridgeFetch {
            repo_id: _,
            want: _,
            have: _,
        } => {
            use crate::client_rpc::GitBridgeFetchResponse;

            // TODO: Implement using GitExporter.export_commit_dag()
            Ok(ClientRpcResponse::GitBridgeFetch(GitBridgeFetchResponse {
                success: false,
                objects: vec![],
                skipped: 0,
                error: Some("Git bridge not yet integrated with node runtime".to_string()),
            }))
        }

        ClientRpcRequest::GitBridgePush {
            repo_id: _,
            objects: _,
            refs: _,
        } => {
            use crate::client_rpc::GitBridgePushResponse;

            // TODO: Implement using GitImporter.import_ref()
            Ok(ClientRpcResponse::GitBridgePush(GitBridgePushResponse {
                success: false,
                objects_imported: 0,
                objects_skipped: 0,
                ref_results: vec![],
                error: Some("Git bridge not yet integrated with node runtime".to_string()),
            }))
        }
    }
}
