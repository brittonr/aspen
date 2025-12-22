//! Client protocol handler.
//!
//! Handles client RPC connections over Iroh with the `aspen-client` ALPN.

use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};
use tokio::sync::Semaphore;
use tracing::{debug, error, info, instrument, warn};

use crate::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, InitRequest, KeyValueStore,
    ReadRequest, WriteRequest, validate_client_key,
};
use crate::auth::TokenVerifier;
use crate::blob::IrohBlobStore;
use crate::client_rpc::{
    AddBlobResultResponse, AddLearnerResultResponse, AddPeerResultResponse,
    BatchReadResultResponse, BatchWriteResultResponse, BlobListEntry as RpcBlobListEntry,
    ChangeMembershipResultResponse, CheckpointWalResultResponse, ClientRpcRequest,
    ClientRpcResponse, ClientTicketResponse as ClientTicketRpcResponse, ClusterStateResponse,
    ClusterTicketResponse, ConditionalBatchWriteResultResponse, CounterResultResponse,
    DeleteResultResponse, DocsTicketResponse as DocsTicketRpcResponse, GetBlobResultResponse,
    GetBlobTicketResultResponse, HasBlobResultResponse, HealthResponse, InitResultResponse,
    LeaseGrantResultResponse, LeaseInfo, LeaseKeepaliveResultResponse, LeaseListResultResponse,
    LeaseRevokeResultResponse, LeaseTimeToLiveResultResponse, ListBlobsResultResponse,
    LockResultResponse, MAX_CLIENT_MESSAGE_SIZE, MAX_CLUSTER_NODES, MetricsResponse,
    NodeDescriptor, NodeInfoResponse, PromoteLearnerResultResponse, ProtectBlobResultResponse,
    RaftMetricsResponse, RateLimiterResultResponse, ReadResultResponse, ScanEntry,
    ScanResultResponse, SequenceResultResponse, SignedCounterResultResponse,
    SnapshotResultResponse, SqlResultResponse, UnprotectBlobResultResponse, VaultKeysResponse,
    VaultListResponse, WatchCancelResultResponse, WatchCreateResultResponse,
    WatchStatusResultResponse, WriteResultResponse,
};
use crate::cluster::IrohEndpointManager;
use crate::coordination::{
    AtomicCounter, CounterConfig, DistributedLock, DistributedRateLimiter, LockConfig,
    RateLimiterConfig, SequenceConfig, SequenceGenerator, SignedAtomicCounter,
};
use crate::raft::constants::{
    CLIENT_RPC_BURST, CLIENT_RPC_RATE_LIMIT_PREFIX, CLIENT_RPC_RATE_PER_SECOND,
    CLIENT_RPC_REQUEST_COUNTER, CLIENT_RPC_REQUEST_ID_SEQUENCE,
};

use super::constants::{MAX_CLIENT_CONNECTIONS, MAX_CLIENT_STREAMS_PER_CONNECTION};
use super::error_sanitization::{
    sanitize_blob_error, sanitize_control_error, sanitize_error_for_client, sanitize_kv_error,
};

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
    pub sql_executor: Arc<dyn crate::api::SqlQueryExecutor>,
    /// State machine for direct reads (lease queries, etc.).
    pub state_machine: Option<crate::raft::StateMachineVariant>,
    /// Iroh endpoint manager for peer info.
    pub endpoint_manager: Arc<IrohEndpointManager>,
    /// Blob store for content-addressed storage (optional).
    pub blob_store: Option<Arc<IrohBlobStore>>,
    /// Peer manager for cluster-to-cluster sync (optional).
    pub peer_manager: Option<Arc<crate::docs::PeerManager>>,
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
                return Err(AcceptError::from_err(std::io::Error::other(
                    "connection limit reached",
                )));
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
async fn handle_client_connection(
    connection: Connection,
    ctx: Arc<ClientProtocolContext>,
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
        let remote_id = remote_node_id.to_string();
        let (send, recv) = stream;
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(err) = handle_client_request((recv, send), ctx_clone, &remote_id).await {
                error!(error = %err, "failed to handle Client request");
            }
            active_streams_clone.fetch_sub(1, Ordering::Relaxed);
        });
    }

    Ok(())
}

/// Handle a single Client RPC request on a stream.
#[instrument(skip(recv, send, ctx, client_id), fields(request_id))]
async fn handle_client_request(
    (mut recv, mut send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    ctx: Arc<ClientProtocolContext>,
    client_id: &str,
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Generate unique request ID using distributed sequence
    // Tiger Style: Monotonic IDs enable cluster-wide request tracing
    let request_id = {
        let seq = SequenceGenerator::new(
            ctx.kv_store.clone(),
            CLIENT_RPC_REQUEST_ID_SEQUENCE,
            SequenceConfig::default(),
        );
        seq.next().await.unwrap_or(0)
    };

    // Record request_id in current span for distributed tracing
    tracing::Span::current().record("request_id", request_id);

    // Read the request with size limit
    let buffer = recv
        .read_to_end(MAX_CLIENT_MESSAGE_SIZE)
        .await
        .context("failed to read Client request")?;

    // Try to parse as AuthenticatedRequest first (new format)
    // Fall back to legacy ClientRpcRequest for backwards compatibility
    let (request, token) =
        match postcard::from_bytes::<crate::client_rpc::AuthenticatedRequest>(&buffer) {
            Ok(auth_req) => (auth_req.request, auth_req.token),
            Err(_) => {
                // Legacy format: parse as plain ClientRpcRequest
                let req: ClientRpcRequest = postcard::from_bytes(&buffer)
                    .context("failed to deserialize Client request")?;
                (req, None)
            }
        };

    debug!(request_type = ?request, client_id = %client_id, request_id = %request_id, has_token = token.is_some(), "received Client request");

    // Rate limit check using distributed rate limiter
    // Tiger Style: Per-client rate limiting prevents DoS from individual clients
    let rate_limit_key = format!("{}{}", CLIENT_RPC_RATE_LIMIT_PREFIX, client_id);
    let limiter = DistributedRateLimiter::new(
        ctx.kv_store.clone(),
        &rate_limit_key,
        RateLimiterConfig::new(CLIENT_RPC_RATE_PER_SECOND, CLIENT_RPC_BURST),
    );

    if let Err(e) = limiter.try_acquire().await {
        warn!(
            client_id = %client_id,
            retry_after_ms = e.retry_after_ms,
            "Client rate limited"
        );
        let response = ClientRpcResponse::error(
            "RATE_LIMITED",
            format!("Too many requests. Retry after {}ms", e.retry_after_ms),
        );
        let response_bytes =
            postcard::to_stdvec(&response).context("failed to serialize rate limit response")?;
        send.write_all(&response_bytes)
            .await
            .context("failed to write rate limit response")?;
        send.finish().context("failed to finish send stream")?;
        return Ok(());
    }

    // Authorization check: verify capability token if auth is enabled
    // Tiger Style: Fail-fast on authorization errors before processing request
    if let Some(ref verifier) = ctx.token_verifier {
        // Check if this request requires authorization
        if let Some(operation) = request.to_operation() {
            match &token {
                Some(cap_token) => {
                    // Parse client_id as PublicKey for audience verification
                    let presenter = iroh::PublicKey::from_str(client_id).ok();

                    // Verify token and authorize the operation
                    if let Err(auth_err) =
                        verifier.authorize(cap_token, &operation, presenter.as_ref())
                    {
                        warn!(
                            client_id = %client_id,
                            error = %auth_err,
                            operation = ?operation,
                            "Authorization failed"
                        );
                        let response = ClientRpcResponse::error(
                            "UNAUTHORIZED",
                            format!("Authorization failed: {}", auth_err),
                        );
                        let response_bytes = postcard::to_stdvec(&response)
                            .context("failed to serialize auth error response")?;
                        send.write_all(&response_bytes)
                            .await
                            .context("failed to write auth error response")?;
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
                        let response = ClientRpcResponse::error(
                            "UNAUTHORIZED",
                            "Authentication required but no token provided",
                        );
                        let response_bytes = postcard::to_stdvec(&response)
                            .context("failed to serialize auth error response")?;
                        send.write_all(&response_bytes)
                            .await
                            .context("failed to write auth error response")?;
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
    let response_bytes =
        postcard::to_stdvec(&response).context("failed to serialize Client response")?;
    send.write_all(&response_bytes)
        .await
        .context("failed to write Client response")?;
    send.finish().context("failed to finish send stream")?;

    // Increment cluster-wide request counter (best-effort, non-blocking)
    // Tiger Style: Fire-and-forget counter increment doesn't block request path
    let kv_store = ctx.kv_store.clone();
    tokio::spawn(async move {
        let counter = AtomicCounter::new(
            kv_store,
            CLIENT_RPC_REQUEST_COUNTER,
            CounterConfig::default(),
        );
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
            let metrics = ctx
                .controller
                .get_metrics()
                .await
                .map_err(|e| anyhow::anyhow!("failed to get Raft metrics: {}", e))?;

            Ok(ClientRpcResponse::RaftMetrics(RaftMetricsResponse {
                node_id: ctx.node_id,
                state: format!("{:?}", metrics.state),
                current_leader: metrics.current_leader.map(|id| id.0),
                current_term: metrics.current_term,
                last_log_index: metrics.last_log_index,
                last_applied_index: metrics.last_applied.as_ref().map(|la| la.index),
                snapshot_index: metrics.snapshot.as_ref().map(|s| s.index),
            }))
        }

        ClientRpcRequest::GetLeader => {
            let leader = ctx
                .controller
                .get_leader()
                .await
                .map_err(|e| anyhow::anyhow!("failed to get leader: {}", e))?;
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
            use crate::cluster::ticket::AspenClusterTicket;
            use iroh_gossip::proto::TopicId;

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
            let result = ctx
                .controller
                .init(InitRequest {
                    initial_members: vec![],
                })
                .await;

            Ok(ClientRpcResponse::InitResult(InitResultResponse {
                success: result.is_ok(),
                // HIGH-4: Sanitize error messages to prevent information leakage
                error: result.err().map(|e| sanitize_control_error(&e)),
            }))
        }

        ClientRpcRequest::ReadKey { key } => {
            let result = ctx.kv_store.read(ReadRequest { key: key.clone() }).await;

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
                return Ok(ClientRpcResponse::CompareAndSwapResult(
                    CompareAndSwapResultResponse {
                        success: false,
                        actual_value: None,
                        error: Some(vault_err.to_string()),
                    },
                ));
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
                Ok(_) => Ok(ClientRpcResponse::CompareAndSwapResult(
                    CompareAndSwapResultResponse {
                        success: true,
                        actual_value: None,
                        error: None,
                    },
                )),
                Err(crate::api::KeyValueStoreError::CompareAndSwapFailed { actual, .. }) => Ok(
                    ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
                        success: false,
                        actual_value: actual.map(|v| v.into_bytes()),
                        error: None,
                    }),
                ),
                Err(e) => Ok(ClientRpcResponse::CompareAndSwapResult(
                    CompareAndSwapResultResponse {
                        success: false,
                        actual_value: None,
                        // HIGH-4: Sanitize error messages to prevent information leakage
                        error: Some(sanitize_kv_error(&e)),
                    },
                )),
            }
        }

        ClientRpcRequest::CompareAndDeleteKey { key, expected } => {
            use crate::api::WriteCommand;
            use crate::client_rpc::CompareAndSwapResultResponse;

            // Validate key against reserved _system: prefix
            if let Err(vault_err) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CompareAndSwapResult(
                    CompareAndSwapResultResponse {
                        success: false,
                        actual_value: None,
                        error: Some(vault_err.to_string()),
                    },
                ));
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
                Ok(_) => Ok(ClientRpcResponse::CompareAndSwapResult(
                    CompareAndSwapResultResponse {
                        success: true,
                        actual_value: None,
                        error: None,
                    },
                )),
                Err(crate::api::KeyValueStoreError::CompareAndSwapFailed { actual, .. }) => Ok(
                    ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
                        success: false,
                        actual_value: actual.map(|v| v.into_bytes()),
                        error: None,
                    }),
                ),
                Err(e) => Ok(ClientRpcResponse::CompareAndSwapResult(
                    CompareAndSwapResultResponse {
                        success: false,
                        actual_value: None,
                        // HIGH-4: Sanitize error messages to prevent information leakage
                        error: Some(sanitize_kv_error(&e)),
                    },
                )),
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
            use crate::api::ClusterNode;
            use iroh::EndpointAddr;
            use std::str::FromStr;

            // Parse the address as either JSON EndpointAddr or bare EndpointId
            let iroh_addr = if addr.starts_with('{') {
                serde_json::from_str::<EndpointAddr>(&addr)
                    .map_err(|e| format!("invalid JSON EndpointAddr: {e}"))
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
                Err(parse_err) => {
                    Err(crate::api::ControlPlaneError::InvalidRequest { reason: parse_err })
                }
            };

            Ok(ClientRpcResponse::AddLearnerResult(
                AddLearnerResultResponse {
                    success: result.is_ok(),
                    // HIGH-4: Sanitize error messages to prevent information leakage
                    error: result.err().map(|e| sanitize_control_error(&e)),
                },
            ))
        }

        ClientRpcRequest::ChangeMembership { members } => {
            let result = ctx
                .controller
                .change_membership(ChangeMembershipRequest { members })
                .await;

            Ok(ClientRpcResponse::ChangeMembershipResult(
                ChangeMembershipResultResponse {
                    success: result.is_ok(),
                    // HIGH-4: Sanitize error messages to prevent information leakage
                    error: result.err().map(|e| sanitize_control_error(&e)),
                },
            ))
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
            let mut nodes: Vec<NodeDescriptor> = Vec::with_capacity(
                (cluster_state.nodes.len() + cluster_state.learners.len()).min(MAX_CLUSTER_NODES),
            );

            // Track which nodes are voters (members)
            let member_ids: std::collections::HashSet<u64> =
                cluster_state.members.iter().copied().collect();

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
                    let last_applied = metrics
                        .last_applied
                        .as_ref()
                        .map(|la| la.index)
                        .unwrap_or(0);
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

            Ok(ClientRpcResponse::PromoteLearnerResult(
                PromoteLearnerResultResponse {
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
                },
            ))
        }

        ClientRpcRequest::CheckpointWal => {
            // WAL checkpoint requires direct access to the SQLite state machine,
            // which is not exposed through the ClusterController/KeyValueStore traits.
            // This would need to be implemented as a new trait method or by extending
            // the ClientProtocolContext to include the state machine reference.
            Ok(ClientRpcResponse::CheckpointWalResult(
                CheckpointWalResultResponse {
                    success: false,
                    pages_checkpointed: None,
                    wal_size_before_bytes: None,
                    wal_size_after_bytes: None,
                    error: Some("WAL checkpoint requires state machine access - use trigger_snapshot for log compaction".to_string()),
                },
            ))
        }

        ClientRpcRequest::ListVaults => {
            // Deprecated: Vault-specific operations removed in favor of flat keyspace.
            // Use ScanKeys with an empty prefix to list all keys.
            Ok(ClientRpcResponse::VaultList(VaultListResponse {
                vaults: vec![],
                error: Some(
                    "ListVaults is deprecated. Use ScanKeys with a prefix instead.".to_string(),
                ),
            }))
        }

        ClientRpcRequest::GetVaultKeys { vault_name } => {
            // Deprecated: Vault-specific operations removed in favor of flat keyspace.
            // Use ScanKeys with the desired prefix instead.
            Ok(ClientRpcResponse::VaultKeys(VaultKeysResponse {
                vault: vault_name,
                keys: vec![],
                error: Some(
                    "GetVaultKeys is deprecated. Use ScanKeys with a prefix instead.".to_string(),
                ),
            }))
        }

        ClientRpcRequest::AddPeer {
            node_id,
            endpoint_addr,
        } => {
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
                        error: Some(
                            "invalid endpoint_addr: expected JSON-serialized EndpointAddr"
                                .to_string(),
                        ),
                    }));
                }
            };

            // Check if network factory is available
            let network_factory = match &ctx.network_factory {
                Some(nf) => nf,
                None => {
                    warn!(
                        node_id = node_id,
                        "AddPeer: network_factory not available in context"
                    );
                    return Ok(ClientRpcResponse::AddPeerResult(AddPeerResultResponse {
                        success: false,
                        error: Some("network_factory not configured for this node".to_string()),
                    }));
                }
            };

            // Add peer to the network factory
            // Tiger Style: add_peer is bounded by MAX_PEERS (1000)
            network_factory
                .add_peer(crate::raft::types::NodeId(node_id), parsed_addr.clone())
                .await;

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
            use crate::cluster::ticket::AspenClusterTicket;
            use iroh_gossip::proto::TopicId;

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
                for id_str in ids_str
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                {
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
                for node in cluster_state
                    .nodes
                    .iter()
                    .chain(cluster_state.learners.iter())
                {
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

        ClientRpcRequest::GetDocsTicket {
            read_write,
            priority,
        } => {
            use crate::docs::ticket::AspenDocsTicket;

            let endpoint_addr = ctx.endpoint_manager.node_addr().clone();

            // Derive namespace ID from cluster cookie
            let namespace_hash =
                blake3::hash(format!("aspen-docs-{}", ctx.cluster_cookie).as_bytes());
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
            use crate::blob::BlobStore;
            use iroh_blobs::Hash;

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
            use crate::blob::BlobStore;
            use iroh_blobs::Hash;

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
                Ok(exists) => Ok(ClientRpcResponse::HasBlobResult(HasBlobResultResponse {
                    exists,
                    error: None,
                })),
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
            use crate::blob::BlobStore;
            use iroh_blobs::Hash;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::GetBlobTicketResult(
                    GetBlobTicketResultResponse {
                        success: false,
                        ticket: None,
                        error: Some("blob store not enabled".to_string()),
                    },
                ));
            };

            // Parse hash from string
            let hash = match hash.parse::<Hash>() {
                Ok(h) => h,
                Err(_) => {
                    // HIGH-4: Don't expose parse error details
                    return Ok(ClientRpcResponse::GetBlobTicketResult(
                        GetBlobTicketResultResponse {
                            success: false,
                            ticket: None,
                            error: Some("invalid hash".to_string()),
                        },
                    ));
                }
            };

            match blob_store.ticket(&hash).await {
                Ok(ticket) => Ok(ClientRpcResponse::GetBlobTicketResult(
                    GetBlobTicketResultResponse {
                        success: true,
                        ticket: Some(ticket.to_string()),
                        error: None,
                    },
                )),
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob ticket generation failed");
                    Ok(ClientRpcResponse::GetBlobTicketResult(
                        GetBlobTicketResultResponse {
                            success: false,
                            ticket: None,
                            error: Some(sanitize_blob_error(&e)),
                        },
                    ))
                }
            }
        }

        ClientRpcRequest::ListBlobs {
            limit,
            continuation_token,
        } => {
            use crate::blob::BlobStore;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::ListBlobsResult(
                    ListBlobsResultResponse {
                        blobs: vec![],
                        count: 0,
                        has_more: false,
                        continuation_token: None,
                        error: Some("blob store not enabled".to_string()),
                    },
                ));
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

                    Ok(ClientRpcResponse::ListBlobsResult(
                        ListBlobsResultResponse {
                            blobs,
                            count,
                            has_more: result.continuation_token.is_some(),
                            continuation_token: result.continuation_token,
                            error: None,
                        },
                    ))
                }
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob list failed");
                    Ok(ClientRpcResponse::ListBlobsResult(
                        ListBlobsResultResponse {
                            blobs: vec![],
                            count: 0,
                            has_more: false,
                            continuation_token: None,
                            error: Some(sanitize_blob_error(&e)),
                        },
                    ))
                }
            }
        }

        ClientRpcRequest::ProtectBlob { hash, tag } => {
            use crate::blob::BlobStore;
            use iroh_blobs::Hash;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::ProtectBlobResult(
                    ProtectBlobResultResponse {
                        success: false,
                        error: Some("blob store not enabled".to_string()),
                    },
                ));
            };

            // Parse hash from string
            let hash = match hash.parse::<Hash>() {
                Ok(h) => h,
                Err(e) => {
                    return Ok(ClientRpcResponse::ProtectBlobResult(
                        ProtectBlobResultResponse {
                            success: false,
                            error: Some(format!("invalid hash: {}", e)),
                        },
                    ));
                }
            };

            let tag_name = IrohBlobStore::user_tag(&tag);
            match blob_store.protect(&hash, &tag_name).await {
                Ok(()) => Ok(ClientRpcResponse::ProtectBlobResult(
                    ProtectBlobResultResponse {
                        success: true,
                        error: None,
                    },
                )),
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob protect failed");
                    Ok(ClientRpcResponse::ProtectBlobResult(
                        ProtectBlobResultResponse {
                            success: false,
                            error: Some(sanitize_blob_error(&e)),
                        },
                    ))
                }
            }
        }

        ClientRpcRequest::UnprotectBlob { tag } => {
            use crate::blob::BlobStore;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::UnprotectBlobResult(
                    UnprotectBlobResultResponse {
                        success: false,
                        error: Some("blob store not enabled".to_string()),
                    },
                ));
            };

            let tag_name = IrohBlobStore::user_tag(&tag);
            match blob_store.unprotect(&tag_name).await {
                Ok(()) => Ok(ClientRpcResponse::UnprotectBlobResult(
                    UnprotectBlobResultResponse {
                        success: true,
                        error: None,
                    },
                )),
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob unprotect failed");
                    Ok(ClientRpcResponse::UnprotectBlobResult(
                        UnprotectBlobResultResponse {
                            success: false,
                            error: Some(sanitize_blob_error(&e)),
                        },
                    ))
                }
            }
        }

        // =========================================================================
        // Peer cluster operations (cluster-to-cluster sync)
        // =========================================================================
        ClientRpcRequest::AddPeerCluster { ticket } => {
            use crate::client_rpc::AddPeerClusterResultResponse;
            use crate::docs::ticket::AspenDocsTicket;

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::AddPeerClusterResult(
                    AddPeerClusterResultResponse {
                        success: false,
                        cluster_id: None,
                        priority: None,
                        error: Some("peer sync not enabled".to_string()),
                    },
                ));
            };

            // Parse the ticket
            let docs_ticket = match AspenDocsTicket::deserialize(&ticket) {
                Ok(t) => t,
                Err(_) => {
                    // HIGH-4: Don't expose parse error details
                    return Ok(ClientRpcResponse::AddPeerClusterResult(
                        AddPeerClusterResultResponse {
                            success: false,
                            cluster_id: None,
                            priority: None,
                            error: Some("invalid ticket".to_string()),
                        },
                    ));
                }
            };

            let cluster_id = docs_ticket.cluster_id.clone();
            let priority = docs_ticket.priority as u32;

            match peer_manager.add_peer(docs_ticket).await {
                Ok(()) => Ok(ClientRpcResponse::AddPeerClusterResult(
                    AddPeerClusterResultResponse {
                        success: true,
                        cluster_id: Some(cluster_id),
                        priority: Some(priority),
                        error: None,
                    },
                )),
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "add peer cluster failed");
                    Ok(ClientRpcResponse::AddPeerClusterResult(
                        AddPeerClusterResultResponse {
                            success: false,
                            cluster_id: Some(cluster_id),
                            priority: None,
                            error: Some("peer cluster operation failed".to_string()),
                        },
                    ))
                }
            }
        }

        ClientRpcRequest::RemovePeerCluster { cluster_id } => {
            use crate::client_rpc::RemovePeerClusterResultResponse;

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::RemovePeerClusterResult(
                    RemovePeerClusterResultResponse {
                        success: false,
                        cluster_id: cluster_id.clone(),
                        error: Some("peer sync not enabled".to_string()),
                    },
                ));
            };

            match peer_manager.remove_peer(&cluster_id).await {
                Ok(()) => Ok(ClientRpcResponse::RemovePeerClusterResult(
                    RemovePeerClusterResultResponse {
                        success: true,
                        cluster_id,
                        error: None,
                    },
                )),
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "remove peer cluster failed");
                    Ok(ClientRpcResponse::RemovePeerClusterResult(
                        RemovePeerClusterResultResponse {
                            success: false,
                            cluster_id,
                            error: Some("peer cluster operation failed".to_string()),
                        },
                    ))
                }
            }
        }

        ClientRpcRequest::ListPeerClusters => {
            use crate::client_rpc::{ListPeerClustersResultResponse, PeerClusterInfo};

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::ListPeerClustersResult(
                    ListPeerClustersResultResponse {
                        peers: vec![],
                        count: 0,
                        error: Some("peer sync not enabled".to_string()),
                    },
                ));
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

            Ok(ClientRpcResponse::ListPeerClustersResult(
                ListPeerClustersResultResponse {
                    peers: peer_infos,
                    count,
                    error: None,
                },
            ))
        }

        ClientRpcRequest::GetPeerClusterStatus { cluster_id } => {
            use crate::client_rpc::PeerClusterStatusResponse;

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::PeerClusterStatus(
                    PeerClusterStatusResponse {
                        found: false,
                        cluster_id: cluster_id.clone(),
                        state: "unknown".to_string(),
                        syncing: false,
                        entries_received: 0,
                        entries_imported: 0,
                        entries_skipped: 0,
                        entries_filtered: 0,
                        error: Some("peer sync not enabled".to_string()),
                    },
                ));
            };

            match peer_manager.sync_status(&cluster_id).await {
                Some(status) => Ok(ClientRpcResponse::PeerClusterStatus(
                    PeerClusterStatusResponse {
                        found: true,
                        cluster_id: status.cluster_id,
                        state: format!("{:?}", status.state),
                        syncing: status.syncing,
                        entries_received: status.entries_received,
                        entries_imported: status.entries_imported,
                        entries_skipped: status.entries_skipped,
                        entries_filtered: status.entries_filtered,
                        error: None,
                    },
                )),
                None => Ok(ClientRpcResponse::PeerClusterStatus(
                    PeerClusterStatusResponse {
                        found: false,
                        cluster_id,
                        state: "unknown".to_string(),
                        syncing: false,
                        entries_received: 0,
                        entries_imported: 0,
                        entries_skipped: 0,
                        entries_filtered: 0,
                        error: None,
                    },
                )),
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
                return Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(
                    UpdatePeerClusterFilterResultResponse {
                        success: false,
                        cluster_id: cluster_id.clone(),
                        filter_type: None,
                        error: Some("peer sync not enabled".to_string()),
                    },
                ));
            };

            // Parse filter type and prefixes
            let filter = match filter_type.to_lowercase().as_str() {
                "full" | "fullreplication" => SubscriptionFilter::FullReplication,
                "include" | "prefixfilter" => {
                    let prefix_list: Vec<String> = prefixes
                        .as_ref()
                        .map(|p| serde_json::from_str(p).unwrap_or_default())
                        .unwrap_or_default();
                    SubscriptionFilter::PrefixFilter(prefix_list)
                }
                "exclude" | "prefixexclude" => {
                    let prefix_list: Vec<String> = prefixes
                        .as_ref()
                        .map(|p| serde_json::from_str(p).unwrap_or_default())
                        .unwrap_or_default();
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

            match peer_manager
                .importer()
                .update_filter(&cluster_id, filter.clone())
                .await
            {
                Ok(()) => Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(
                    UpdatePeerClusterFilterResultResponse {
                        success: true,
                        cluster_id,
                        filter_type: Some(filter_type),
                        error: None,
                    },
                )),
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "update peer cluster filter failed");
                    Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(
                        UpdatePeerClusterFilterResultResponse {
                            success: false,
                            cluster_id,
                            filter_type: None,
                            error: Some("peer cluster operation failed".to_string()),
                        },
                    ))
                }
            }
        }

        ClientRpcRequest::UpdatePeerClusterPriority {
            cluster_id,
            priority,
        } => {
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
            let previous_priority = peer_manager
                .list_peers()
                .await
                .into_iter()
                .find(|p| p.cluster_id == cluster_id)
                .map(|p| p.priority);

            match peer_manager
                .importer()
                .update_priority(&cluster_id, priority)
                .await
            {
                Ok(()) => Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(
                    UpdatePeerClusterPriorityResultResponse {
                        success: true,
                        cluster_id,
                        previous_priority,
                        new_priority: Some(priority),
                        error: None,
                    },
                )),
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "update peer cluster priority failed");
                    Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(
                        UpdatePeerClusterPriorityResultResponse {
                            success: false,
                            cluster_id,
                            previous_priority,
                            new_priority: None,
                            error: Some("peer cluster operation failed".to_string()),
                        },
                    ))
                }
            }
        }

        ClientRpcRequest::SetPeerClusterEnabled {
            cluster_id,
            enabled,
        } => {
            use crate::client_rpc::SetPeerClusterEnabledResultResponse;

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::SetPeerClusterEnabledResult(
                    SetPeerClusterEnabledResultResponse {
                        success: false,
                        cluster_id: cluster_id.clone(),
                        enabled: None,
                        error: Some("peer sync not enabled".to_string()),
                    },
                ));
            };

            match peer_manager
                .importer()
                .set_enabled(&cluster_id, enabled)
                .await
            {
                Ok(()) => Ok(ClientRpcResponse::SetPeerClusterEnabledResult(
                    SetPeerClusterEnabledResultResponse {
                        success: true,
                        cluster_id,
                        enabled: Some(enabled),
                        error: None,
                    },
                )),
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "set peer cluster enabled failed");
                    Ok(ClientRpcResponse::SetPeerClusterEnabledResult(
                        SetPeerClusterEnabledResultResponse {
                            success: false,
                            cluster_id,
                            enabled: None,
                            error: Some("peer cluster operation failed".to_string()),
                        },
                    ))
                }
            }
        }

        ClientRpcRequest::ExecuteSql {
            query,
            params,
            consistency,
            limit,
            timeout_ms,
        } => {
            use crate::api::{SqlConsistency, SqlQueryExecutor, SqlQueryRequest, SqlValue};

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
                    // Convert SqlValue to serde_json::Value for response
                    let rows: Vec<Vec<serde_json::Value>> = result
                        .rows
                        .into_iter()
                        .map(|row| {
                            row.into_iter()
                                .map(|v| match v {
                                    SqlValue::Null => serde_json::Value::Null,
                                    SqlValue::Integer(i) => serde_json::json!(i),
                                    SqlValue::Real(f) => serde_json::json!(f),
                                    SqlValue::Text(s) => serde_json::Value::String(s),
                                    SqlValue::Blob(b) => {
                                        // Encode blob as base64
                                        use base64::Engine;
                                        serde_json::Value::String(
                                            base64::engine::general_purpose::STANDARD.encode(&b),
                                        )
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
                        CoordinationError::LockHeld {
                            holder,
                            deadline_ms,
                        } => (Some(holder.clone()), Some(*deadline_ms)),
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

        ClientRpcRequest::LockTryAcquire {
            key,
            holder_id,
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
                        CoordinationError::LockHeld {
                            holder,
                            deadline_ms,
                        } => (Some(holder.clone()), Some(*deadline_ms)),
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

            let read_result = ctx.kv_store.read(ReadRequest { key: key.clone() }).await;

            match read_result {
                Ok(result) => {
                    let value = result.kv.map(|kv| kv.value).unwrap_or_default();
                    match serde_json::from_str::<LockEntry>(&value) {
                        Ok(entry) => {
                            if entry.holder_id != holder_id || entry.fencing_token != fencing_token
                            {
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
                            let released_json = serde_json::to_string(&released).unwrap();

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

            let read_result = ctx.kv_store.read(ReadRequest { key: key.clone() }).await;

            match read_result {
                Ok(result) => {
                    let value = result.kv.map(|kv| kv.value).unwrap_or_default();
                    match serde_json::from_str::<LockEntry>(&value) {
                        Ok(entry) => {
                            if entry.holder_id != holder_id || entry.fencing_token != fencing_token
                            {
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
                            let renewed_json = serde_json::to_string(&renewed).unwrap();

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
                return Ok(ClientRpcResponse::SignedCounterResult(
                    SignedCounterResultResponse {
                        success: false,
                        value: None,
                        error: Some(e.to_string()),
                    },
                ));
            }

            let counter =
                SignedAtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.get().await {
                Ok(value) => Ok(ClientRpcResponse::SignedCounterResult(
                    SignedCounterResultResponse {
                        success: true,
                        value: Some(value),
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::SignedCounterResult(
                    SignedCounterResultResponse {
                        success: false,
                        value: None,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }

        ClientRpcRequest::SignedCounterAdd { key, amount } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::SignedCounterResult(
                    SignedCounterResultResponse {
                        success: false,
                        value: None,
                        error: Some(e.to_string()),
                    },
                ));
            }

            let counter =
                SignedAtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.add(amount).await {
                Ok(value) => Ok(ClientRpcResponse::SignedCounterResult(
                    SignedCounterResultResponse {
                        success: true,
                        value: Some(value),
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::SignedCounterResult(
                    SignedCounterResultResponse {
                        success: false,
                        value: None,
                        error: Some(e.to_string()),
                    },
                )),
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
                return Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: false,
                        tokens_remaining: None,
                        retry_after_ms: None,
                        error: Some(e.to_string()),
                    },
                ));
            }

            let config = RateLimiterConfig {
                capacity,
                refill_rate,
                initial_tokens: None,
            };
            let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);
            match limiter.try_acquire_n(tokens).await {
                Ok(remaining) => Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: true,
                        tokens_remaining: Some(remaining),
                        retry_after_ms: None,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: false,
                        tokens_remaining: Some(e.available),
                        retry_after_ms: Some(e.retry_after_ms),
                        error: None,
                    },
                )),
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
                return Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: false,
                        tokens_remaining: None,
                        retry_after_ms: None,
                        error: Some(e.to_string()),
                    },
                ));
            }

            let config = RateLimiterConfig {
                capacity,
                refill_rate,
                initial_tokens: None,
            };
            let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);
            match limiter
                .acquire_n(tokens, std::time::Duration::from_millis(timeout_ms))
                .await
            {
                Ok(remaining) => Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: true,
                        tokens_remaining: Some(remaining),
                        retry_after_ms: None,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: false,
                        tokens_remaining: Some(e.available),
                        retry_after_ms: Some(e.retry_after_ms),
                        error: Some("timeout waiting for tokens".to_string()),
                    },
                )),
            }
        }

        ClientRpcRequest::RateLimiterAvailable {
            key,
            capacity,
            refill_rate,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: false,
                        tokens_remaining: None,
                        retry_after_ms: None,
                        error: Some(e.to_string()),
                    },
                ));
            }

            let config = RateLimiterConfig {
                capacity,
                refill_rate,
                initial_tokens: None,
            };
            let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);
            match limiter.available().await {
                Ok(tokens) => Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: true,
                        tokens_remaining: Some(tokens),
                        retry_after_ms: None,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: false,
                        tokens_remaining: None,
                        retry_after_ms: None,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }

        ClientRpcRequest::RateLimiterReset {
            key,
            capacity,
            refill_rate,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: false,
                        tokens_remaining: None,
                        retry_after_ms: None,
                        error: Some(e.to_string()),
                    },
                ));
            }

            let config = RateLimiterConfig {
                capacity,
                refill_rate,
                initial_tokens: None,
            };
            let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);
            match limiter.reset().await {
                Ok(()) => Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: true,
                        tokens_remaining: Some(capacity),
                        retry_after_ms: None,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: false,
                        tokens_remaining: None,
                        retry_after_ms: None,
                        error: Some(e.to_string()),
                    },
                )),
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
                    return Ok(ClientRpcResponse::BatchReadResult(
                        BatchReadResultResponse {
                            success: false,
                            values: None,
                            error: Some(e.to_string()),
                        },
                    ));
                }
            }

            // Read all keys atomically
            let mut values = Vec::with_capacity(keys.len());
            for key in &keys {
                let request = ReadRequest { key: key.clone() };
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
                        return Ok(ClientRpcResponse::BatchReadResult(
                            BatchReadResultResponse {
                                success: false,
                                values: None,
                                error: Some(e.to_string()),
                            },
                        ));
                    }
                }
            }

            Ok(ClientRpcResponse::BatchReadResult(
                BatchReadResultResponse {
                    success: true,
                    values: Some(values),
                    error: None,
                },
            ))
        }

        ClientRpcRequest::BatchWrite { operations } => {
            use crate::api::{BatchOperation, WriteCommand};
            use crate::client_rpc::BatchWriteOperation;

            // Validate all keys
            for op in &operations {
                let key = match op {
                    BatchWriteOperation::Set { key, .. } => key,
                    BatchWriteOperation::Delete { key } => key,
                };
                if let Err(e) = validate_client_key(key) {
                    return Ok(ClientRpcResponse::BatchWriteResult(
                        BatchWriteResultResponse {
                            success: false,
                            operations_applied: None,
                            error: Some(e.to_string()),
                        },
                    ));
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
                    BatchWriteOperation::Delete { key } => {
                        BatchOperation::Delete { key: key.clone() }
                    }
                })
                .collect();

            let request = WriteRequest {
                command: WriteCommand::Batch {
                    operations: batch_ops,
                },
            };

            match ctx.kv_store.write(request).await {
                Ok(result) => Ok(ClientRpcResponse::BatchWriteResult(
                    BatchWriteResultResponse {
                        success: true,
                        operations_applied: result.batch_applied,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::BatchWriteResult(
                    BatchWriteResultResponse {
                        success: false,
                        operations_applied: None,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }

        ClientRpcRequest::ConditionalBatchWrite {
            conditions,
            operations,
        } => {
            use crate::api::{BatchCondition as ApiBatchCondition, BatchOperation, WriteCommand};
            use crate::client_rpc::{BatchCondition, BatchWriteOperation};

            // Validate all keys in conditions
            for cond in &conditions {
                let key = match cond {
                    BatchCondition::ValueEquals { key, .. } => key,
                    BatchCondition::KeyExists { key } => key,
                    BatchCondition::KeyNotExists { key } => key,
                };
                if let Err(e) = validate_client_key(key) {
                    return Ok(ClientRpcResponse::ConditionalBatchWriteResult(
                        ConditionalBatchWriteResultResponse {
                            success: false,
                            conditions_met: false,
                            operations_applied: None,
                            failed_condition_index: None,
                            failed_condition_reason: Some(e.to_string()),
                            error: None,
                        },
                    ));
                }
            }

            // Validate all keys in operations
            for op in &operations {
                let key = match op {
                    BatchWriteOperation::Set { key, .. } => key,
                    BatchWriteOperation::Delete { key } => key,
                };
                if let Err(e) = validate_client_key(key) {
                    return Ok(ClientRpcResponse::ConditionalBatchWriteResult(
                        ConditionalBatchWriteResultResponse {
                            success: false,
                            conditions_met: false,
                            operations_applied: None,
                            failed_condition_index: None,
                            failed_condition_reason: Some(e.to_string()),
                            error: None,
                        },
                    ));
                }
            }

            // Convert conditions
            let api_conditions: Vec<ApiBatchCondition> = conditions
                .iter()
                .map(|c| match c {
                    BatchCondition::ValueEquals { key, expected } => {
                        ApiBatchCondition::ValueEquals {
                            key: key.clone(),
                            expected: String::from_utf8_lossy(expected).to_string(),
                        }
                    }
                    BatchCondition::KeyExists { key } => {
                        ApiBatchCondition::KeyExists { key: key.clone() }
                    }
                    BatchCondition::KeyNotExists { key } => {
                        ApiBatchCondition::KeyNotExists { key: key.clone() }
                    }
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
                    BatchWriteOperation::Delete { key } => {
                        BatchOperation::Delete { key: key.clone() }
                    }
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
                    Ok(ClientRpcResponse::ConditionalBatchWriteResult(
                        ConditionalBatchWriteResultResponse {
                            success: conditions_met,
                            conditions_met,
                            operations_applied: result.batch_applied,
                            failed_condition_index: result.failed_condition_index,
                            failed_condition_reason: None,
                            error: None,
                        },
                    ))
                }
                Err(e) => Ok(ClientRpcResponse::ConditionalBatchWriteResult(
                    ConditionalBatchWriteResultResponse {
                        success: false,
                        conditions_met: false,
                        operations_applied: None,
                        failed_condition_index: None,
                        failed_condition_reason: None,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }

        // ==========================================================================
        // Watch Operations
        // ==========================================================================
        ClientRpcRequest::WatchCreate { .. } => {
            // Watch operations require a streaming connection via LOG_SUBSCRIBER_ALPN.
            // The ClientRpcRequest::WatchCreate is for documentation completeness,
            // but actual watch functionality is handled by LogSubscriberProtocolHandler.
            Ok(ClientRpcResponse::WatchCreateResult(
                WatchCreateResultResponse {
                    success: false,
                    watch_id: None,
                    current_index: None,
                    error: Some(
                        "Watch operations require the streaming protocol. \
                         Connect via LOG_SUBSCRIBER_ALPN (aspen-logs) for real-time \
                         key change notifications."
                            .to_string(),
                    ),
                },
            ))
        }

        ClientRpcRequest::WatchCancel { watch_id } => {
            // Same as WatchCreate - streaming protocol required
            Ok(ClientRpcResponse::WatchCancelResult(
                WatchCancelResultResponse {
                    success: false,
                    watch_id,
                    error: Some(
                        "Watch operations require the streaming protocol. \
                         Use LOG_SUBSCRIBER_ALPN (aspen-logs)."
                            .to_string(),
                    ),
                },
            ))
        }

        ClientRpcRequest::WatchStatus { .. } => {
            // TODO: Could implement this to query LogSubscriberProtocolHandler state
            // For now, redirect to streaming protocol
            Ok(ClientRpcResponse::WatchStatusResult(
                WatchStatusResultResponse {
                    success: false,
                    watches: None,
                    error: Some(
                        "Watch operations require the streaming protocol. \
                         Use LOG_SUBSCRIBER_ALPN (aspen-logs)."
                            .to_string(),
                    ),
                },
            ))
        }

        // =====================================================================
        // Lease operations
        // =====================================================================
        ClientRpcRequest::LeaseGrant {
            ttl_seconds,
            lease_id,
        } => {
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
                Ok(response) => Ok(ClientRpcResponse::LeaseGrantResult(
                    LeaseGrantResultResponse {
                        success: true,
                        lease_id: response.lease_id,
                        ttl_seconds: response.ttl_seconds,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::LeaseGrantResult(
                    LeaseGrantResultResponse {
                        success: false,
                        lease_id: None,
                        ttl_seconds: None,
                        error: Some(format!("{}", e)),
                    },
                )),
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
                Ok(response) => Ok(ClientRpcResponse::LeaseRevokeResult(
                    LeaseRevokeResultResponse {
                        success: true,
                        keys_deleted: response.keys_deleted,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::LeaseRevokeResult(
                    LeaseRevokeResultResponse {
                        success: false,
                        keys_deleted: None,
                        error: Some(format!("{}", e)),
                    },
                )),
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
                Ok(response) => Ok(ClientRpcResponse::LeaseKeepaliveResult(
                    LeaseKeepaliveResultResponse {
                        success: true,
                        lease_id: response.lease_id,
                        ttl_seconds: response.ttl_seconds,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::LeaseKeepaliveResult(
                    LeaseKeepaliveResultResponse {
                        success: false,
                        lease_id: None,
                        ttl_seconds: None,
                        error: Some(format!("{}", e)),
                    },
                )),
            }
        }

        ClientRpcRequest::LeaseTimeToLive {
            lease_id,
            include_keys,
        } => {
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

                            Ok(ClientRpcResponse::LeaseTimeToLiveResult(
                                LeaseTimeToLiveResultResponse {
                                    success: true,
                                    lease_id: Some(lease_id),
                                    granted_ttl_seconds: Some(granted_ttl),
                                    remaining_ttl_seconds: Some(remaining_ttl),
                                    keys,
                                    error: None,
                                },
                            ))
                        }
                        None => {
                            // Lease not found or expired
                            Ok(ClientRpcResponse::LeaseTimeToLiveResult(
                                LeaseTimeToLiveResultResponse {
                                    success: false,
                                    lease_id: Some(lease_id),
                                    granted_ttl_seconds: None,
                                    remaining_ttl_seconds: None,
                                    keys: None,
                                    error: Some("Lease not found or expired".to_string()),
                                },
                            ))
                        }
                    }
                }
                None => Ok(ClientRpcResponse::LeaseTimeToLiveResult(
                    LeaseTimeToLiveResultResponse {
                        success: false,
                        lease_id: Some(lease_id),
                        granted_ttl_seconds: None,
                        remaining_ttl_seconds: None,
                        keys: None,
                        error: Some("State machine not available".to_string()),
                    },
                )),
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

                    Ok(ClientRpcResponse::LeaseListResult(
                        LeaseListResultResponse {
                            success: true,
                            leases: Some(leases),
                            error: None,
                        },
                    ))
                }
                None => Ok(ClientRpcResponse::LeaseListResult(
                    LeaseListResultResponse {
                        success: false,
                        leases: None,
                        error: Some("State machine not available".to_string()),
                    },
                )),
            }
        }

        ClientRpcRequest::WriteKeyWithLease {
            key,
            value,
            lease_id,
        } => {
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

            match manager
                .enter(&name, &participant_id, required_count, timeout)
                .await
            {
                Ok((current, phase)) => Ok(ClientRpcResponse::BarrierEnterResult(
                    BarrierResultResponse {
                        success: true,
                        current_count: Some(current),
                        required_count: Some(required_count),
                        phase: Some(phase),
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::BarrierEnterResult(
                    BarrierResultResponse {
                        success: false,
                        current_count: None,
                        required_count: None,
                        phase: None,
                        error: Some(format!("{}", e)),
                    },
                )),
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
                Ok((current, phase)) => Ok(ClientRpcResponse::BarrierLeaveResult(
                    BarrierResultResponse {
                        success: true,
                        current_count: Some(current),
                        required_count: None,
                        phase: Some(phase),
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::BarrierLeaveResult(
                    BarrierResultResponse {
                        success: false,
                        current_count: None,
                        required_count: None,
                        phase: None,
                        error: Some(format!("{}", e)),
                    },
                )),
            }
        }

        ClientRpcRequest::BarrierStatus { name } => {
            use crate::client_rpc::BarrierResultResponse;
            use crate::coordination::BarrierManager;

            let manager = BarrierManager::new(ctx.kv_store.clone());

            match manager.status(&name).await {
                Ok((current, required, phase)) => Ok(ClientRpcResponse::BarrierStatusResult(
                    BarrierResultResponse {
                        success: true,
                        current_count: Some(current),
                        required_count: Some(required),
                        phase: Some(phase),
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::BarrierStatusResult(
                    BarrierResultResponse {
                        success: false,
                        current_count: None,
                        required_count: None,
                        phase: None,
                        error: Some(format!("{}", e)),
                    },
                )),
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

            match manager
                .acquire(&name, &holder_id, permits, capacity, ttl_ms, timeout)
                .await
            {
                Ok((acquired, available)) => Ok(ClientRpcResponse::SemaphoreAcquireResult(
                    SemaphoreResultResponse {
                        success: true,
                        permits_acquired: Some(acquired),
                        available: Some(available),
                        capacity: Some(capacity),
                        retry_after_ms: None,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::SemaphoreAcquireResult(
                    SemaphoreResultResponse {
                        success: false,
                        permits_acquired: None,
                        available: None,
                        capacity: None,
                        retry_after_ms: Some(100), // Suggest 100ms retry
                        error: Some(format!("{}", e)),
                    },
                )),
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

            match manager
                .try_acquire(&name, &holder_id, permits, capacity, ttl_ms)
                .await
            {
                Ok(Some((acquired, available))) => Ok(
                    ClientRpcResponse::SemaphoreTryAcquireResult(SemaphoreResultResponse {
                        success: true,
                        permits_acquired: Some(acquired),
                        available: Some(available),
                        capacity: Some(capacity),
                        retry_after_ms: None,
                        error: None,
                    }),
                ),
                Ok(None) => Ok(ClientRpcResponse::SemaphoreTryAcquireResult(
                    SemaphoreResultResponse {
                        success: false,
                        permits_acquired: None,
                        available: None,
                        capacity: Some(capacity),
                        retry_after_ms: Some(100),
                        error: Some("No permits available".to_string()),
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::SemaphoreTryAcquireResult(
                    SemaphoreResultResponse {
                        success: false,
                        permits_acquired: None,
                        available: None,
                        capacity: None,
                        retry_after_ms: None,
                        error: Some(format!("{}", e)),
                    },
                )),
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
                Ok(available) => Ok(ClientRpcResponse::SemaphoreReleaseResult(
                    SemaphoreResultResponse {
                        success: true,
                        permits_acquired: None,
                        available: Some(available),
                        capacity: None,
                        retry_after_ms: None,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::SemaphoreReleaseResult(
                    SemaphoreResultResponse {
                        success: false,
                        permits_acquired: None,
                        available: None,
                        capacity: None,
                        retry_after_ms: None,
                        error: Some(format!("{}", e)),
                    },
                )),
            }
        }

        ClientRpcRequest::SemaphoreStatus { name } => {
            use crate::client_rpc::SemaphoreResultResponse;
            use crate::coordination::SemaphoreManager;

            let manager = SemaphoreManager::new(ctx.kv_store.clone());

            match manager.status(&name).await {
                Ok((available, capacity)) => Ok(ClientRpcResponse::SemaphoreStatusResult(
                    SemaphoreResultResponse {
                        success: true,
                        permits_acquired: None,
                        available: Some(available),
                        capacity: Some(capacity),
                        retry_after_ms: None,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::SemaphoreStatusResult(
                    SemaphoreResultResponse {
                        success: false,
                        permits_acquired: None,
                        available: None,
                        capacity: None,
                        retry_after_ms: None,
                        error: Some(format!("{}", e)),
                    },
                )),
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

            match manager
                .acquire_read(&name, &holder_id, ttl_ms, timeout)
                .await
            {
                Ok((fencing_token, deadline_ms, reader_count)) => Ok(
                    ClientRpcResponse::RWLockAcquireReadResult(RWLockResultResponse {
                        success: true,
                        mode: Some("read".to_string()),
                        fencing_token: Some(fencing_token),
                        deadline_ms: Some(deadline_ms),
                        reader_count: Some(reader_count),
                        writer_holder: None,
                        error: None,
                    }),
                ),
                Err(e) => Ok(ClientRpcResponse::RWLockAcquireReadResult(
                    RWLockResultResponse {
                        success: false,
                        mode: None,
                        fencing_token: None,
                        deadline_ms: None,
                        reader_count: None,
                        writer_holder: None,
                        error: Some(format!("{}", e)),
                    },
                )),
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
                Ok(Some((fencing_token, deadline_ms, reader_count))) => Ok(
                    ClientRpcResponse::RWLockTryAcquireReadResult(RWLockResultResponse {
                        success: true,
                        mode: Some("read".to_string()),
                        fencing_token: Some(fencing_token),
                        deadline_ms: Some(deadline_ms),
                        reader_count: Some(reader_count),
                        writer_holder: None,
                        error: None,
                    }),
                ),
                Ok(None) => Ok(ClientRpcResponse::RWLockTryAcquireReadResult(
                    RWLockResultResponse {
                        success: false,
                        mode: None,
                        fencing_token: None,
                        deadline_ms: None,
                        reader_count: None,
                        writer_holder: None,
                        error: Some("lock not available".to_string()),
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::RWLockTryAcquireReadResult(
                    RWLockResultResponse {
                        success: false,
                        mode: None,
                        fencing_token: None,
                        deadline_ms: None,
                        reader_count: None,
                        writer_holder: None,
                        error: Some(format!("{}", e)),
                    },
                )),
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

            match manager
                .acquire_write(&name, &holder_id, ttl_ms, timeout)
                .await
            {
                Ok((fencing_token, deadline_ms)) => Ok(
                    ClientRpcResponse::RWLockAcquireWriteResult(RWLockResultResponse {
                        success: true,
                        mode: Some("write".to_string()),
                        fencing_token: Some(fencing_token),
                        deadline_ms: Some(deadline_ms),
                        reader_count: Some(0),
                        writer_holder: Some(holder_id),
                        error: None,
                    }),
                ),
                Err(e) => Ok(ClientRpcResponse::RWLockAcquireWriteResult(
                    RWLockResultResponse {
                        success: false,
                        mode: None,
                        fencing_token: None,
                        deadline_ms: None,
                        reader_count: None,
                        writer_holder: None,
                        error: Some(format!("{}", e)),
                    },
                )),
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
                Ok(Some((fencing_token, deadline_ms))) => Ok(
                    ClientRpcResponse::RWLockTryAcquireWriteResult(RWLockResultResponse {
                        success: true,
                        mode: Some("write".to_string()),
                        fencing_token: Some(fencing_token),
                        deadline_ms: Some(deadline_ms),
                        reader_count: Some(0),
                        writer_holder: Some(holder_id),
                        error: None,
                    }),
                ),
                Ok(None) => Ok(ClientRpcResponse::RWLockTryAcquireWriteResult(
                    RWLockResultResponse {
                        success: false,
                        mode: None,
                        fencing_token: None,
                        deadline_ms: None,
                        reader_count: None,
                        writer_holder: None,
                        error: Some("lock not available".to_string()),
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::RWLockTryAcquireWriteResult(
                    RWLockResultResponse {
                        success: false,
                        mode: None,
                        fencing_token: None,
                        deadline_ms: None,
                        reader_count: None,
                        writer_holder: None,
                        error: Some(format!("{}", e)),
                    },
                )),
            }
        }

        ClientRpcRequest::RWLockReleaseRead { name, holder_id } => {
            use crate::client_rpc::RWLockResultResponse;
            use crate::coordination::RWLockManager;

            let manager = RWLockManager::new(ctx.kv_store.clone());

            match manager.release_read(&name, &holder_id).await {
                Ok(()) => Ok(ClientRpcResponse::RWLockReleaseReadResult(
                    RWLockResultResponse {
                        success: true,
                        mode: None,
                        fencing_token: None,
                        deadline_ms: None,
                        reader_count: None,
                        writer_holder: None,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::RWLockReleaseReadResult(
                    RWLockResultResponse {
                        success: false,
                        mode: None,
                        fencing_token: None,
                        deadline_ms: None,
                        reader_count: None,
                        writer_holder: None,
                        error: Some(format!("{}", e)),
                    },
                )),
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

            match manager
                .release_write(&name, &holder_id, fencing_token)
                .await
            {
                Ok(()) => Ok(ClientRpcResponse::RWLockReleaseWriteResult(
                    RWLockResultResponse {
                        success: true,
                        mode: None,
                        fencing_token: None,
                        deadline_ms: None,
                        reader_count: None,
                        writer_holder: None,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::RWLockReleaseWriteResult(
                    RWLockResultResponse {
                        success: false,
                        mode: None,
                        fencing_token: None,
                        deadline_ms: None,
                        reader_count: None,
                        writer_holder: None,
                        error: Some(format!("{}", e)),
                    },
                )),
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

            match manager
                .downgrade(&name, &holder_id, fencing_token, ttl_ms)
                .await
            {
                Ok((new_token, deadline_ms, reader_count)) => Ok(
                    ClientRpcResponse::RWLockDowngradeResult(RWLockResultResponse {
                        success: true,
                        mode: Some("read".to_string()),
                        fencing_token: Some(new_token),
                        deadline_ms: Some(deadline_ms),
                        reader_count: Some(reader_count),
                        writer_holder: None,
                        error: None,
                    }),
                ),
                Err(e) => Ok(ClientRpcResponse::RWLockDowngradeResult(
                    RWLockResultResponse {
                        success: false,
                        mode: None,
                        fencing_token: None,
                        deadline_ms: None,
                        reader_count: None,
                        writer_holder: None,
                        error: Some(format!("{}", e)),
                    },
                )),
            }
        }

        ClientRpcRequest::RWLockStatus { name } => {
            use crate::client_rpc::RWLockResultResponse;
            use crate::coordination::RWLockManager;

            let manager = RWLockManager::new(ctx.kv_store.clone());

            match manager.status(&name).await {
                Ok((mode, reader_count, writer_holder, fencing_token)) => Ok(
                    ClientRpcResponse::RWLockStatusResult(RWLockResultResponse {
                        success: true,
                        mode: Some(mode),
                        fencing_token: Some(fencing_token),
                        deadline_ms: None,
                        reader_count: Some(reader_count),
                        writer_holder,
                        error: None,
                    }),
                ),
                Err(e) => Ok(ClientRpcResponse::RWLockStatusResult(
                    RWLockResultResponse {
                        success: false,
                        mode: None,
                        fencing_token: None,
                        deadline_ms: None,
                        reader_count: None,
                        writer_holder: None,
                        error: Some(format!("{}", e)),
                    },
                )),
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
            use crate::coordination::{QueueConfig, QueueManager};

            let manager = QueueManager::new(ctx.kv_store.clone());
            let config = QueueConfig {
                default_visibility_timeout_ms,
                default_ttl_ms,
                max_delivery_attempts,
            };

            match manager.create(&queue_name, config).await {
                Ok((created, _)) => Ok(ClientRpcResponse::QueueCreateResult(
                    QueueCreateResultResponse {
                        success: true,
                        created,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::QueueCreateResult(
                    QueueCreateResultResponse {
                        success: false,
                        created: false,
                        error: Some(format!("{}", e)),
                    },
                )),
            }
        }

        ClientRpcRequest::QueueDelete { queue_name } => {
            use crate::client_rpc::QueueDeleteResultResponse;
            use crate::coordination::QueueManager;

            let manager = QueueManager::new(ctx.kv_store.clone());

            match manager.delete(&queue_name).await {
                Ok(items_deleted) => Ok(ClientRpcResponse::QueueDeleteResult(
                    QueueDeleteResultResponse {
                        success: true,
                        items_deleted: Some(items_deleted),
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::QueueDeleteResult(
                    QueueDeleteResultResponse {
                        success: false,
                        items_deleted: None,
                        error: Some(format!("{}", e)),
                    },
                )),
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
            use crate::coordination::{EnqueueOptions, QueueManager};

            let manager = QueueManager::new(ctx.kv_store.clone());
            let options = EnqueueOptions {
                ttl_ms,
                message_group_id,
                deduplication_id,
            };

            match manager.enqueue(&queue_name, payload, options).await {
                Ok(item_id) => Ok(ClientRpcResponse::QueueEnqueueResult(
                    QueueEnqueueResultResponse {
                        success: true,
                        item_id: Some(item_id),
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::QueueEnqueueResult(
                    QueueEnqueueResultResponse {
                        success: false,
                        item_id: None,
                        error: Some(format!("{}", e)),
                    },
                )),
            }
        }

        ClientRpcRequest::QueueEnqueueBatch { queue_name, items } => {
            use crate::client_rpc::QueueEnqueueBatchResultResponse;
            use crate::coordination::{EnqueueOptions, QueueManager};

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
                Ok(item_ids) => Ok(ClientRpcResponse::QueueEnqueueBatchResult(
                    QueueEnqueueBatchResultResponse {
                        success: true,
                        item_ids,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::QueueEnqueueBatchResult(
                    QueueEnqueueBatchResultResponse {
                        success: false,
                        item_ids: vec![],
                        error: Some(format!("{}", e)),
                    },
                )),
            }
        }

        ClientRpcRequest::QueueDequeue {
            queue_name,
            consumer_id,
            max_items,
            visibility_timeout_ms,
        } => {
            use crate::client_rpc::{QueueDequeueResultResponse, QueueDequeuedItemResponse};
            use crate::coordination::QueueManager;

            let manager = QueueManager::new(ctx.kv_store.clone());

            match manager
                .dequeue(&queue_name, &consumer_id, max_items, visibility_timeout_ms)
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
                    Ok(ClientRpcResponse::QueueDequeueResult(
                        QueueDequeueResultResponse {
                            success: true,
                            items: response_items,
                            error: None,
                        },
                    ))
                }
                Err(e) => Ok(ClientRpcResponse::QueueDequeueResult(
                    QueueDequeueResultResponse {
                        success: false,
                        items: vec![],
                        error: Some(format!("{}", e)),
                    },
                )),
            }
        }

        ClientRpcRequest::QueueDequeueWait {
            queue_name,
            consumer_id,
            max_items,
            visibility_timeout_ms,
            wait_timeout_ms,
        } => {
            use crate::client_rpc::{QueueDequeueResultResponse, QueueDequeuedItemResponse};
            use crate::coordination::QueueManager;

            let manager = QueueManager::new(ctx.kv_store.clone());

            match manager
                .dequeue_wait(
                    &queue_name,
                    &consumer_id,
                    max_items,
                    visibility_timeout_ms,
                    wait_timeout_ms,
                )
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
                    Ok(ClientRpcResponse::QueueDequeueResult(
                        QueueDequeueResultResponse {
                            success: true,
                            items: response_items,
                            error: None,
                        },
                    ))
                }
                Err(e) => Ok(ClientRpcResponse::QueueDequeueResult(
                    QueueDequeueResultResponse {
                        success: false,
                        items: vec![],
                        error: Some(format!("{}", e)),
                    },
                )),
            }
        }

        ClientRpcRequest::QueuePeek {
            queue_name,
            max_items,
        } => {
            use crate::client_rpc::{QueueItemResponse, QueuePeekResultResponse};
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
                    Ok(ClientRpcResponse::QueuePeekResult(
                        QueuePeekResultResponse {
                            success: true,
                            items: response_items,
                            error: None,
                        },
                    ))
                }
                Err(e) => Ok(ClientRpcResponse::QueuePeekResult(
                    QueuePeekResultResponse {
                        success: false,
                        items: vec![],
                        error: Some(format!("{}", e)),
                    },
                )),
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

            match manager
                .nack(&queue_name, &receipt_handle, move_to_dlq, error_message)
                .await
            {
                Ok(()) => Ok(ClientRpcResponse::QueueNackResult(
                    QueueNackResultResponse {
                        success: true,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::QueueNackResult(
                    QueueNackResultResponse {
                        success: false,
                        error: Some(format!("{}", e)),
                    },
                )),
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

            match manager
                .extend_visibility(&queue_name, &receipt_handle, additional_timeout_ms)
                .await
            {
                Ok(new_deadline) => Ok(ClientRpcResponse::QueueExtendVisibilityResult(
                    QueueExtendVisibilityResultResponse {
                        success: true,
                        new_deadline_ms: Some(new_deadline),
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::QueueExtendVisibilityResult(
                    QueueExtendVisibilityResultResponse {
                        success: false,
                        new_deadline_ms: None,
                        error: Some(format!("{}", e)),
                    },
                )),
            }
        }

        ClientRpcRequest::QueueStatus { queue_name } => {
            use crate::client_rpc::QueueStatusResultResponse;
            use crate::coordination::QueueManager;

            let manager = QueueManager::new(ctx.kv_store.clone());

            match manager.status(&queue_name).await {
                Ok(status) => Ok(ClientRpcResponse::QueueStatusResult(
                    QueueStatusResultResponse {
                        success: true,
                        exists: status.exists,
                        visible_count: Some(status.visible_count),
                        pending_count: Some(status.pending_count),
                        dlq_count: Some(status.dlq_count),
                        total_enqueued: Some(status.total_enqueued),
                        total_acked: Some(status.total_acked),
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::QueueStatusResult(
                    QueueStatusResultResponse {
                        success: false,
                        exists: false,
                        visible_count: None,
                        pending_count: None,
                        dlq_count: None,
                        total_enqueued: None,
                        total_acked: None,
                        error: Some(format!("{}", e)),
                    },
                )),
            }
        }

        ClientRpcRequest::QueueGetDLQ {
            queue_name,
            max_items,
        } => {
            use crate::client_rpc::{QueueDLQItemResponse, QueueGetDLQResultResponse};
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
                    Ok(ClientRpcResponse::QueueGetDLQResult(
                        QueueGetDLQResultResponse {
                            success: true,
                            items: response_items,
                            error: None,
                        },
                    ))
                }
                Err(e) => Ok(ClientRpcResponse::QueueGetDLQResult(
                    QueueGetDLQResultResponse {
                        success: false,
                        items: vec![],
                        error: Some(format!("{}", e)),
                    },
                )),
            }
        }

        ClientRpcRequest::QueueRedriveDLQ {
            queue_name,
            item_id,
        } => {
            use crate::client_rpc::QueueRedriveDLQResultResponse;
            use crate::coordination::QueueManager;

            let manager = QueueManager::new(ctx.kv_store.clone());

            match manager.redrive_dlq(&queue_name, item_id).await {
                Ok(()) => Ok(ClientRpcResponse::QueueRedriveDLQResult(
                    QueueRedriveDLQResultResponse {
                        success: true,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::QueueRedriveDLQResult(
                    QueueRedriveDLQResultResponse {
                        success: false,
                        error: Some(format!("{}", e)),
                    },
                )),
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
            use crate::client_rpc::ServiceRegisterResultResponse;
            use crate::coordination::{
                HealthStatus, RegisterOptions, ServiceInstanceMetadata, ServiceRegistry,
            };
            use std::collections::HashMap;

            let registry = ServiceRegistry::new(ctx.kv_store.clone());

            // Parse tags from JSON array
            let tags_vec: Vec<String> = serde_json::from_str(&tags).unwrap_or_default();

            // Parse custom metadata from JSON object
            let custom_map: HashMap<String, String> =
                serde_json::from_str(&custom_metadata).unwrap_or_default();

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

            match registry
                .register(&service_name, &instance_id, &address, metadata, options)
                .await
            {
                Ok((fencing_token, deadline_ms)) => Ok(ClientRpcResponse::ServiceRegisterResult(
                    ServiceRegisterResultResponse {
                        success: true,
                        fencing_token: Some(fencing_token),
                        deadline_ms: Some(deadline_ms),
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::ServiceRegisterResult(
                    ServiceRegisterResultResponse {
                        success: false,
                        fencing_token: None,
                        deadline_ms: None,
                        error: Some(format!("{}", e)),
                    },
                )),
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

            match registry
                .deregister(&service_name, &instance_id, fencing_token)
                .await
            {
                Ok(was_registered) => Ok(ClientRpcResponse::ServiceDeregisterResult(
                    ServiceDeregisterResultResponse {
                        success: true,
                        was_registered,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::ServiceDeregisterResult(
                    ServiceDeregisterResultResponse {
                        success: false,
                        was_registered: false,
                        error: Some(format!("{}", e)),
                    },
                )),
            }
        }

        ClientRpcRequest::ServiceDiscover {
            service_name,
            healthy_only,
            tags,
            version_prefix,
            limit,
        } => {
            use crate::client_rpc::{ServiceDiscoverResultResponse, ServiceInstanceResponse};
            use crate::coordination::{DiscoveryFilter, ServiceRegistry};

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
                                crate::coordination::HealthStatus::Unhealthy => {
                                    "unhealthy".to_string()
                                }
                                crate::coordination::HealthStatus::Unknown => "unknown".to_string(),
                            },
                            version: inst.metadata.version,
                            tags: inst.metadata.tags,
                            weight: inst.metadata.weight,
                            custom_metadata: serde_json::to_string(&inst.metadata.custom)
                                .unwrap_or_default(),
                            registered_at_ms: inst.registered_at_ms,
                            last_heartbeat_ms: inst.last_heartbeat_ms,
                            deadline_ms: inst.deadline_ms,
                            lease_id: inst.lease_id,
                            fencing_token: inst.fencing_token,
                        })
                        .collect();
                    let count = response_instances.len() as u32;
                    Ok(ClientRpcResponse::ServiceDiscoverResult(
                        ServiceDiscoverResultResponse {
                            success: true,
                            instances: response_instances,
                            count,
                            error: None,
                        },
                    ))
                }
                Err(e) => Ok(ClientRpcResponse::ServiceDiscoverResult(
                    ServiceDiscoverResultResponse {
                        success: false,
                        instances: vec![],
                        count: 0,
                        error: Some(format!("{}", e)),
                    },
                )),
            }
        }

        ClientRpcRequest::ServiceList { prefix, limit } => {
            use crate::client_rpc::ServiceListResultResponse;
            use crate::coordination::ServiceRegistry;

            let registry = ServiceRegistry::new(ctx.kv_store.clone());

            match registry.discover_services(&prefix, limit).await {
                Ok(services) => {
                    let count = services.len() as u32;
                    Ok(ClientRpcResponse::ServiceListResult(
                        ServiceListResultResponse {
                            success: true,
                            services,
                            count,
                            error: None,
                        },
                    ))
                }
                Err(e) => Ok(ClientRpcResponse::ServiceListResult(
                    ServiceListResultResponse {
                        success: false,
                        services: vec![],
                        count: 0,
                        error: Some(format!("{}", e)),
                    },
                )),
            }
        }

        ClientRpcRequest::ServiceGetInstance {
            service_name,
            instance_id,
        } => {
            use crate::client_rpc::{ServiceGetInstanceResultResponse, ServiceInstanceResponse};
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
                        custom_metadata: serde_json::to_string(&inst.metadata.custom)
                            .unwrap_or_default(),
                        registered_at_ms: inst.registered_at_ms,
                        last_heartbeat_ms: inst.last_heartbeat_ms,
                        deadline_ms: inst.deadline_ms,
                        lease_id: inst.lease_id,
                        fencing_token: inst.fencing_token,
                    };
                    Ok(ClientRpcResponse::ServiceGetInstanceResult(
                        ServiceGetInstanceResultResponse {
                            success: true,
                            found: true,
                            instance: Some(response_instance),
                            error: None,
                        },
                    ))
                }
                Ok(None) => Ok(ClientRpcResponse::ServiceGetInstanceResult(
                    ServiceGetInstanceResultResponse {
                        success: true,
                        found: false,
                        instance: None,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::ServiceGetInstanceResult(
                    ServiceGetInstanceResultResponse {
                        success: false,
                        found: false,
                        instance: None,
                        error: Some(format!("{}", e)),
                    },
                )),
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

            match registry
                .heartbeat(&service_name, &instance_id, fencing_token)
                .await
            {
                Ok((new_deadline, health_status)) => {
                    let status_str = match health_status {
                        crate::coordination::HealthStatus::Healthy => "healthy",
                        crate::coordination::HealthStatus::Unhealthy => "unhealthy",
                        crate::coordination::HealthStatus::Unknown => "unknown",
                    };
                    Ok(ClientRpcResponse::ServiceHeartbeatResult(
                        ServiceHeartbeatResultResponse {
                            success: true,
                            new_deadline_ms: Some(new_deadline),
                            health_status: Some(status_str.to_string()),
                            error: None,
                        },
                    ))
                }
                Err(e) => Ok(ClientRpcResponse::ServiceHeartbeatResult(
                    ServiceHeartbeatResultResponse {
                        success: false,
                        new_deadline_ms: None,
                        health_status: None,
                        error: Some(format!("{}", e)),
                    },
                )),
            }
        }

        ClientRpcRequest::ServiceUpdateHealth {
            service_name,
            instance_id,
            fencing_token,
            status,
        } => {
            use crate::client_rpc::ServiceUpdateHealthResultResponse;
            use crate::coordination::{HealthStatus, ServiceRegistry};

            let registry = ServiceRegistry::new(ctx.kv_store.clone());

            let health_status = match status.to_lowercase().as_str() {
                "healthy" => HealthStatus::Healthy,
                "unhealthy" => HealthStatus::Unhealthy,
                _ => HealthStatus::Unknown,
            };

            match registry
                .update_health(&service_name, &instance_id, fencing_token, health_status)
                .await
            {
                Ok(()) => Ok(ClientRpcResponse::ServiceUpdateHealthResult(
                    ServiceUpdateHealthResultResponse {
                        success: true,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::ServiceUpdateHealthResult(
                    ServiceUpdateHealthResultResponse {
                        success: false,
                        error: Some(format!("{}", e)),
                    },
                )),
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
            use crate::client_rpc::ServiceUpdateMetadataResultResponse;
            use crate::coordination::ServiceRegistry;
            use std::collections::HashMap;

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
                        && let Ok(parsed_custom) =
                            serde_json::from_str::<HashMap<String, String>>(&c)
                    {
                        instance.metadata.custom = parsed_custom;
                    }

                    match registry
                        .update_metadata(
                            &service_name,
                            &instance_id,
                            fencing_token,
                            instance.metadata,
                        )
                        .await
                    {
                        Ok(()) => Ok(ClientRpcResponse::ServiceUpdateMetadataResult(
                            ServiceUpdateMetadataResultResponse {
                                success: true,
                                error: None,
                            },
                        )),
                        Err(e) => Ok(ClientRpcResponse::ServiceUpdateMetadataResult(
                            ServiceUpdateMetadataResultResponse {
                                success: false,
                                error: Some(format!("{}", e)),
                            },
                        )),
                    }
                }
                Ok(None) => Ok(ClientRpcResponse::ServiceUpdateMetadataResult(
                    ServiceUpdateMetadataResultResponse {
                        success: false,
                        error: Some("instance not found".to_string()),
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::ServiceUpdateMetadataResult(
                    ServiceUpdateMetadataResultResponse {
                        success: false,
                        error: Some(format!("{}", e)),
                    },
                )),
            }
        }
    }
}
