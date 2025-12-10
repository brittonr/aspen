//! TUI RPC server for handling aspen-tui client requests over Iroh.
//!
//! This module provides the server-side implementation of the TUI RPC protocol.
//! It listens for incoming connections with the `aspen-tui` ALPN and processes
//! TUI monitoring and control requests.
//!
//! # Architecture
//!
//! The TUI RPC server runs alongside the Raft RPC server, using a different ALPN
//! to distinguish TUI client connections from cluster-internal Raft connections.
//!
//! # Tiger Style
//!
//! - Bounded connection and stream limits
//! - Explicit error handling
//! - Fail-fast on invalid requests

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use anyhow::{Context, Result};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};

use crate::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, InitRequest, KeyValueStore,
    ReadRequest, WriteRequest,
};
use crate::cluster::IrohEndpointManager;
use crate::tui_rpc::{
    AddLearnerResultResponse, ChangeMembershipResultResponse, ClusterTicketResponse,
    HealthResponse, InitResultResponse, MAX_TUI_MESSAGE_SIZE, NodeInfoResponse,
    RaftMetricsResponse, ReadResultResponse, SnapshotResultResponse, TuiRpcRequest, TuiRpcResponse,
    WriteResultResponse,
};

/// Maximum concurrent TUI connections.
///
/// Tiger Style: Lower limit than Raft since TUI connections are less critical.
const MAX_TUI_CONNECTIONS: u32 = 50;

/// Maximum concurrent streams per TUI connection.
const MAX_TUI_STREAMS_PER_CONNECTION: u32 = 10;

/// TUI RPC server context with all dependencies.
pub struct TuiRpcServerContext {
    /// Node identifier.
    pub node_id: u64,
    /// Cluster controller for Raft operations.
    pub controller: Arc<dyn ClusterController>,
    /// Key-value store interface.
    pub kv_store: Arc<dyn KeyValueStore>,
    /// Iroh endpoint manager for peer info.
    pub endpoint_manager: Arc<IrohEndpointManager>,
    /// Cluster cookie for ticket generation.
    pub cluster_cookie: String,
    /// Node start time for uptime calculation.
    pub start_time: Instant,
}

/// TUI RPC server for handling client requests.
pub struct TuiRpcServer {
    join_handle: JoinHandle<()>,
    cancel_token: CancellationToken,
}

impl TuiRpcServer {
    /// Spawn the TUI RPC server task.
    ///
    /// # Arguments
    /// * `ctx` - Server context with dependencies
    ///
    /// # Returns
    /// Server handle with graceful shutdown support.
    pub fn spawn(ctx: Arc<TuiRpcServerContext>) -> Self {
        let cancel_token = CancellationToken::new();
        let cancel_clone = cancel_token.clone();

        let join_handle = tokio::spawn(async move {
            if let Err(err) = run_tui_server(ctx, cancel_clone).await {
                error!(error = %err, "TUI RPC server task failed");
            }
        });

        Self {
            join_handle,
            cancel_token,
        }
    }

    /// Shutdown the server gracefully.
    pub async fn shutdown(self) -> Result<()> {
        info!("shutting down TUI RPC server");
        self.cancel_token.cancel();
        self.join_handle
            .await
            .context("TUI RPC server task panicked")?;
        Ok(())
    }
}

/// Main TUI server loop that accepts incoming connections.
async fn run_tui_server(ctx: Arc<TuiRpcServerContext>, cancel: CancellationToken) -> Result<()> {
    let endpoint = ctx.endpoint_manager.endpoint();

    // Tiger Style: Fixed limit on concurrent connections
    let connection_semaphore = Arc::new(Semaphore::new(MAX_TUI_CONNECTIONS as usize));

    info!(
        max_connections = MAX_TUI_CONNECTIONS,
        "TUI RPC server listening for incoming connections"
    );

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("TUI RPC server received shutdown signal");
                break;
            }
            incoming = endpoint.accept() => {
                let Some(incoming) = incoming else {
                    warn!("endpoint closed, stopping TUI RPC server");
                    break;
                };

                // Note: In Iroh, connections are already filtered by ALPN at the endpoint level,
                // but both servers are accepting from the same endpoint

                let permit = match connection_semaphore.clone().try_acquire_owned() {
                    Ok(permit) => permit,
                    Err(_) => {
                        warn!("TUI connection limit reached ({}), rejecting", MAX_TUI_CONNECTIONS);
                        continue;
                    }
                };

                let ctx_clone = Arc::clone(&ctx);
                tokio::spawn(async move {
                    let _permit = permit;
                    if let Err(err) = handle_tui_connection(incoming, ctx_clone).await {
                        error!(error = %err, "failed to handle TUI connection");
                    }
                });
            }
        }
    }

    Ok(())
}

/// Handle a single incoming TUI connection.
#[instrument(skip(connecting, ctx))]
async fn handle_tui_connection(
    connecting: iroh::endpoint::Incoming,
    ctx: Arc<TuiRpcServerContext>,
) -> Result<()> {
    let connection = connecting.await.context("failed to accept connection")?;

    // Check if this is a TUI connection by checking the ALPN
    // Note: In practice, the endpoint will only accept connections with matching ALPNs
    // that were configured in the builder

    let remote_node_id = connection.remote_id();
    debug!(remote_node = %remote_node_id, "accepted TUI connection");

    let stream_semaphore = Arc::new(Semaphore::new(MAX_TUI_STREAMS_PER_CONNECTION as usize));
    let active_streams = Arc::new(AtomicU32::new(0));

    // Accept bidirectional streams from this connection
    loop {
        let stream = match connection.accept_bi().await {
            Ok(stream) => stream,
            Err(err) => {
                debug!(remote_node = %remote_node_id, error = %err, "TUI connection closed");
                break;
            }
        };

        let permit = match stream_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    remote_node = %remote_node_id,
                    max_streams = MAX_TUI_STREAMS_PER_CONNECTION,
                    "TUI stream limit reached, dropping stream"
                );
                continue;
            }
        };

        active_streams.fetch_add(1, Ordering::Relaxed);
        let active_streams_clone = active_streams.clone();

        let ctx_clone = Arc::clone(&ctx);
        let (send, recv) = stream;
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(err) = handle_tui_request((recv, send), ctx_clone).await {
                error!(error = %err, "failed to handle TUI request");
            }
            active_streams_clone.fetch_sub(1, Ordering::Relaxed);
        });
    }

    Ok(())
}

/// Handle a single TUI RPC request on a stream.
#[instrument(skip(recv, send, ctx))]
async fn handle_tui_request(
    (mut recv, mut send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    ctx: Arc<TuiRpcServerContext>,
) -> Result<()> {
    // Read the request with size limit
    let buffer = recv
        .read_to_end(MAX_TUI_MESSAGE_SIZE)
        .await
        .context("failed to read TUI request")?;

    // Deserialize the request
    let request: TuiRpcRequest =
        postcard::from_bytes(&buffer).context("failed to deserialize TUI request")?;

    debug!(request_type = ?request, "received TUI request");

    // Process the request and create response
    let response = match process_tui_request(request, &ctx).await {
        Ok(resp) => resp,
        Err(err) => {
            warn!(error = %err, "TUI request processing failed");
            TuiRpcResponse::error("INTERNAL_ERROR", err.to_string())
        }
    };

    // Serialize and send response
    let response_bytes =
        postcard::to_stdvec(&response).context("failed to serialize TUI response")?;
    send.write_all(&response_bytes)
        .await
        .context("failed to write TUI response")?;
    send.finish().context("failed to finish send stream")?;

    Ok(())
}

/// Process a TUI RPC request and generate response.
async fn process_tui_request(
    request: TuiRpcRequest,
    ctx: &TuiRpcServerContext,
) -> Result<TuiRpcResponse> {
    match request {
        TuiRpcRequest::GetHealth => {
            let uptime_seconds = ctx.start_time.elapsed().as_secs();

            // Get Raft metrics to determine health
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

            Ok(TuiRpcResponse::Health(HealthResponse {
                status: status.to_string(),
                node_id: ctx.node_id,
                raft_node_id: Some(ctx.node_id), // In our case they're the same
                uptime_seconds,
            }))
        }

        TuiRpcRequest::GetRaftMetrics => {
            let metrics = ctx
                .controller
                .get_metrics()
                .await
                .context("failed to get Raft metrics")?;

            Ok(TuiRpcResponse::RaftMetrics(RaftMetricsResponse {
                node_id: ctx.node_id,
                state: format!("{:?}", metrics.state),
                current_leader: metrics.current_leader,
                current_term: metrics.current_term,
                last_log_index: metrics.last_log_index,
                last_applied_index: metrics.last_applied.as_ref().map(|la| la.index),
                snapshot_index: metrics.snapshot.as_ref().map(|s| s.index),
            }))
        }

        TuiRpcRequest::GetLeader => {
            let leader = ctx
                .controller
                .get_leader()
                .await
                .context("failed to get leader")?;
            Ok(TuiRpcResponse::Leader(leader))
        }

        TuiRpcRequest::GetNodeInfo => {
            let endpoint_addr = ctx.endpoint_manager.node_addr();
            Ok(TuiRpcResponse::NodeInfo(NodeInfoResponse {
                node_id: ctx.node_id,
                endpoint_addr: format!("{:?}", endpoint_addr), // Serialize as debug format
            }))
        }

        TuiRpcRequest::GetClusterTicket => {
            use crate::cluster::ticket::AspenClusterTicket;
            use iroh_gossip::proto::TopicId;

            // Derive topic ID from cluster cookie
            let hash = blake3::hash(ctx.cluster_cookie.as_bytes());
            let topic_id = TopicId::from_bytes(*hash.as_bytes());

            // Create ticket with this node as bootstrap peer
            let ticket = AspenClusterTicket::with_bootstrap(
                topic_id,
                ctx.cluster_cookie.clone(),
                ctx.endpoint_manager.endpoint().id(),
            );

            let ticket_str = ticket.serialize();

            Ok(TuiRpcResponse::ClusterTicket(ClusterTicketResponse {
                ticket: ticket_str,
                topic_id: format!("{:?}", topic_id),
                cluster_id: ctx.cluster_cookie.clone(),
                endpoint_id: ctx.endpoint_manager.endpoint().id().to_string(),
            }))
        }

        TuiRpcRequest::InitCluster => {
            let result = ctx
                .controller
                .init(InitRequest {
                    initial_members: vec![],
                })
                .await;

            Ok(TuiRpcResponse::InitResult(InitResultResponse {
                success: result.is_ok(),
                error: result.err().map(|e| e.to_string()),
            }))
        }

        TuiRpcRequest::ReadKey { key } => {
            let result = ctx.kv_store.read(ReadRequest { key: key.clone() }).await;

            match result {
                Ok(resp) => Ok(TuiRpcResponse::ReadResult(ReadResultResponse {
                    value: Some(resp.value.into_bytes()),
                    found: true,
                })),
                Err(_e) => Ok(TuiRpcResponse::ReadResult(ReadResultResponse {
                    value: None,
                    found: false,
                })),
            }
        }

        TuiRpcRequest::WriteKey { key, value } => {
            use crate::api::WriteCommand;

            let result = ctx
                .kv_store
                .write(WriteRequest {
                    command: WriteCommand::Set {
                        key,
                        value: String::from_utf8_lossy(&value).to_string(),
                    },
                })
                .await;

            Ok(TuiRpcResponse::WriteResult(WriteResultResponse {
                success: result.is_ok(),
                error: result.err().map(|e| e.to_string()),
            }))
        }

        TuiRpcRequest::TriggerSnapshot => {
            let result = ctx.controller.trigger_snapshot().await;

            match result {
                Ok(snapshot) => Ok(TuiRpcResponse::SnapshotResult(SnapshotResultResponse {
                    success: true,
                    snapshot_index: snapshot.as_ref().map(|log_id| log_id.index),
                    error: None,
                })),
                Err(e) => Ok(TuiRpcResponse::SnapshotResult(SnapshotResultResponse {
                    success: false,
                    snapshot_index: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        TuiRpcRequest::AddLearner { node_id, addr } => {
            use crate::api::ClusterNode;

            let result = ctx
                .controller
                .add_learner(AddLearnerRequest {
                    learner: ClusterNode {
                        id: node_id,
                        addr: addr.clone(),
                        raft_addr: Some(addr.clone()),
                    },
                })
                .await;

            Ok(TuiRpcResponse::AddLearnerResult(AddLearnerResultResponse {
                success: result.is_ok(),
                error: result.err().map(|e| e.to_string()),
            }))
        }

        TuiRpcRequest::ChangeMembership { members } => {
            let result = ctx
                .controller
                .change_membership(ChangeMembershipRequest { members })
                .await;

            Ok(TuiRpcResponse::ChangeMembershipResult(
                ChangeMembershipResultResponse {
                    success: result.is_ok(),
                    error: result.err().map(|e| e.to_string()),
                },
            ))
        }

        TuiRpcRequest::Ping => Ok(TuiRpcResponse::Pong),
    }
}
