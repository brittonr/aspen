//! Protocol handlers for Iroh Router-based ALPN dispatching.
//!
//! This module provides `ProtocolHandler` implementations for different protocols:
//! - `RaftProtocolHandler`: Handles Raft RPC connections (ALPN: `raft-rpc`)
//! - `TuiProtocolHandler`: Handles TUI client connections (ALPN: `aspen-tui`)
//!
//! These handlers are registered with an Iroh Router to properly dispatch
//! incoming connections based on their ALPN, eliminating the race condition
//! that occurred when both servers were accepting from the same endpoint.
//!
//! # Architecture
//!
//! ```text
//! Iroh Endpoint
//!       |
//!       v
//!   Router (ALPN dispatch)
//!       |
//!       +---> raft-rpc ALPN ---> RaftProtocolHandler
//!       |
//!       +---> aspen-tui ALPN --> TuiProtocolHandler
//!       |
//!       +---> gossip ALPN -----> Gossip (via iroh-gossip)
//! ```
//!
//! # Tiger Style
//!
//! - Bounded connection and stream limits per handler
//! - Explicit error handling with AcceptError
//! - Clean shutdown via ProtocolHandler::shutdown()

use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};
use openraft::Raft;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, instrument, warn};

use crate::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, InitRequest, KeyValueStore,
    ReadRequest, WriteRequest,
};
use crate::cluster::IrohEndpointManager;
use crate::raft::constants::{
    MAX_CONCURRENT_CONNECTIONS, MAX_RPC_MESSAGE_SIZE, MAX_STREAMS_PER_CONNECTION,
};
use crate::raft::rpc::{RaftRpcProtocol, RaftRpcResponse};
use crate::raft::types::AppTypeConfig;
use crate::tui_rpc::{
    AddLearnerResultResponse, ChangeMembershipResultResponse, ClusterStateResponse,
    ClusterTicketResponse, HealthResponse, InitResultResponse, MAX_CLUSTER_NODES,
    MAX_TUI_MESSAGE_SIZE, NodeDescriptor, NodeInfoResponse, RaftMetricsResponse,
    ReadResultResponse, SnapshotResultResponse, TuiRpcRequest, TuiRpcResponse, WriteResultResponse,
};

/// ALPN protocol identifier for Raft RPC.
pub const RAFT_ALPN: &[u8] = b"raft-rpc";

/// ALPN protocol identifier for TUI RPC.
pub const TUI_ALPN: &[u8] = b"aspen-tui";

/// Maximum concurrent TUI connections.
///
/// Tiger Style: Lower limit than Raft since TUI connections are less critical.
const MAX_TUI_CONNECTIONS: u32 = 50;

/// Maximum concurrent streams per TUI connection.
const MAX_TUI_STREAMS_PER_CONNECTION: u32 = 10;

/// Protocol handler for Raft RPC over Iroh.
///
/// Implements `ProtocolHandler` to receive connections routed by the Iroh Router
/// based on the `raft-rpc` ALPN. Processes Raft RPC messages (Vote, AppendEntries,
/// InstallSnapshot) and forwards them to the Raft core.
///
/// # Tiger Style
///
/// - Bounded connection count via semaphore
/// - Bounded stream count per connection
/// - Explicit size limits on RPC messages
#[derive(Debug)]
pub struct RaftProtocolHandler {
    raft_core: Raft<AppTypeConfig>,
    connection_semaphore: Arc<Semaphore>,
}

impl RaftProtocolHandler {
    /// Create a new Raft protocol handler.
    ///
    /// # Arguments
    /// * `raft_core` - Raft instance to forward RPCs to
    pub fn new(raft_core: Raft<AppTypeConfig>) -> Self {
        Self {
            raft_core,
            connection_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS as usize)),
        }
    }
}

impl ProtocolHandler for RaftProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote_node_id = connection.remote_id();

        // Try to acquire a connection permit
        let permit = match self.connection_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    "Raft connection limit reached ({}), rejecting connection from {}",
                    MAX_CONCURRENT_CONNECTIONS, remote_node_id
                );
                return Err(AcceptError::from_err(std::io::Error::other(
                    "connection limit reached",
                )));
            }
        };

        debug!(remote_node = %remote_node_id, "accepted Raft RPC connection");

        // Handle the connection with bounded resources
        let result = handle_raft_connection(connection, self.raft_core.clone()).await;

        // Release permit when done
        drop(permit);

        result.map_err(|err| AcceptError::from_err(std::io::Error::other(err.to_string())))
    }

    async fn shutdown(&self) {
        info!("Raft protocol handler shutting down");
        // Close the semaphore to prevent new connections
        self.connection_semaphore.close();
    }
}

/// Handle a single Raft RPC connection.
///
/// Tiger Style: Bounded stream count per connection.
#[instrument(skip(connection, raft_core))]
async fn handle_raft_connection(
    connection: Connection,
    raft_core: Raft<AppTypeConfig>,
) -> anyhow::Result<()> {
    let remote_node_id = connection.remote_id();

    // Tiger Style: Fixed limit on concurrent streams per connection
    let stream_semaphore = Arc::new(Semaphore::new(MAX_STREAMS_PER_CONNECTION as usize));
    let active_streams = Arc::new(AtomicU32::new(0));

    // Accept bidirectional streams from this connection
    loop {
        let stream = match connection.accept_bi().await {
            Ok(stream) => stream,
            Err(err) => {
                // Connection closed or error
                debug!(remote_node = %remote_node_id, error = %err, "Raft connection closed");
                break;
            }
        };

        // Try to acquire a stream permit
        let permit = match stream_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    remote_node = %remote_node_id,
                    max_streams = MAX_STREAMS_PER_CONNECTION,
                    "Raft stream limit reached, dropping stream"
                );
                continue;
            }
        };

        active_streams.fetch_add(1, Ordering::Relaxed);
        let active_streams_clone = active_streams.clone();

        let raft_core_clone = raft_core.clone();
        let (send, recv) = stream;
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(err) = handle_raft_rpc_stream((recv, send), raft_core_clone).await {
                error!(error = %err, "failed to handle Raft RPC stream");
            }
            active_streams_clone.fetch_sub(1, Ordering::Relaxed);
        });
    }

    Ok(())
}

/// Handle a single Raft RPC message on a bidirectional stream.
#[instrument(skip(recv, send, raft_core))]
async fn handle_raft_rpc_stream(
    (mut recv, mut send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    raft_core: Raft<AppTypeConfig>,
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Read the RPC message with size limit
    let buffer = recv
        .read_to_end(MAX_RPC_MESSAGE_SIZE as usize)
        .await
        .context("failed to read RPC message")?;

    // Deserialize the RPC request
    let request: RaftRpcProtocol =
        postcard::from_bytes(&buffer).context("failed to deserialize RPC request")?;

    debug!(request_type = ?request, "received Raft RPC request");

    // Process the RPC and create response
    let response = match request {
        RaftRpcProtocol::Vote(vote_req) => {
            let result = match raft_core.vote(vote_req.request).await {
                Ok(result) => result,
                Err(err) => {
                    error!(error = ?err, "vote RPC failed with fatal error");
                    return Err(anyhow::anyhow!("vote failed: {:?}", err));
                }
            };
            RaftRpcResponse::Vote(result)
        }
        RaftRpcProtocol::AppendEntries(append_req) => {
            let result = match raft_core.append_entries(append_req.request).await {
                Ok(result) => result,
                Err(err) => {
                    error!(error = ?err, "append_entries RPC failed with fatal error");
                    return Err(anyhow::anyhow!("append_entries failed: {:?}", err));
                }
            };
            RaftRpcResponse::AppendEntries(result)
        }
        RaftRpcProtocol::InstallSnapshot(snapshot_req) => {
            let snapshot_cursor = Cursor::new(snapshot_req.snapshot_data);
            let snapshot = openraft::Snapshot {
                meta: snapshot_req.snapshot_meta,
                snapshot: snapshot_cursor,
            };
            let result = raft_core
                .install_full_snapshot(snapshot_req.vote, snapshot)
                .await
                .map_err(openraft::error::RaftError::Fatal);
            RaftRpcResponse::InstallSnapshot(result)
        }
    };

    // Serialize and send response
    let response_bytes =
        postcard::to_stdvec(&response).context("failed to serialize RPC response")?;

    debug!(
        response_size = response_bytes.len(),
        "sending Raft RPC response"
    );

    send.write_all(&response_bytes)
        .await
        .context("failed to write RPC response")?;
    send.finish().context("failed to finish send stream")?;

    debug!("Raft RPC response sent successfully");

    Ok(())
}

/// Context for TUI protocol handler with all dependencies.
#[derive(Clone)]
pub struct TuiProtocolContext {
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

impl std::fmt::Debug for TuiProtocolContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TuiProtocolContext")
            .field("node_id", &self.node_id)
            .field("cluster_cookie", &self.cluster_cookie)
            .finish_non_exhaustive()
    }
}

/// Protocol handler for TUI RPC over Iroh.
///
/// Implements `ProtocolHandler` to receive connections routed by the Iroh Router
/// based on the `aspen-tui` ALPN. Processes TUI requests for monitoring and
/// control operations.
///
/// # Tiger Style
///
/// - Lower connection limit than Raft (TUI is less critical)
/// - Bounded stream count per connection
/// - Explicit size limits on messages
#[derive(Debug)]
pub struct TuiProtocolHandler {
    ctx: Arc<TuiProtocolContext>,
    connection_semaphore: Arc<Semaphore>,
}

impl TuiProtocolHandler {
    /// Create a new TUI protocol handler.
    ///
    /// # Arguments
    /// * `ctx` - TUI context with dependencies
    pub fn new(ctx: TuiProtocolContext) -> Self {
        Self {
            ctx: Arc::new(ctx),
            connection_semaphore: Arc::new(Semaphore::new(MAX_TUI_CONNECTIONS as usize)),
        }
    }
}

impl ProtocolHandler for TuiProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote_node_id = connection.remote_id();

        // Try to acquire a connection permit
        let permit = match self.connection_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    "TUI connection limit reached ({}), rejecting connection from {}",
                    MAX_TUI_CONNECTIONS, remote_node_id
                );
                return Err(AcceptError::from_err(std::io::Error::other(
                    "connection limit reached",
                )));
            }
        };

        debug!(remote_node = %remote_node_id, "accepted TUI connection");

        // Handle the connection with bounded resources
        let result = handle_tui_connection(connection, Arc::clone(&self.ctx)).await;

        // Release permit when done
        drop(permit);

        result.map_err(|err| AcceptError::from_err(std::io::Error::other(err.to_string())))
    }

    async fn shutdown(&self) {
        info!("TUI protocol handler shutting down");
        self.connection_semaphore.close();
    }
}

/// Handle a single TUI connection.
#[instrument(skip(connection, ctx))]
async fn handle_tui_connection(
    connection: Connection,
    ctx: Arc<TuiProtocolContext>,
) -> anyhow::Result<()> {
    let remote_node_id = connection.remote_id();

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
    ctx: Arc<TuiProtocolContext>,
) -> anyhow::Result<()> {
    use anyhow::Context;

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
    ctx: &TuiProtocolContext,
) -> anyhow::Result<TuiRpcResponse> {
    match request {
        TuiRpcRequest::GetHealth => {
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

            Ok(TuiRpcResponse::Health(HealthResponse {
                status: status.to_string(),
                node_id: ctx.node_id,
                raft_node_id: Some(ctx.node_id),
                uptime_seconds,
            }))
        }

        TuiRpcRequest::GetRaftMetrics => {
            let metrics = ctx
                .controller
                .get_metrics()
                .await
                .map_err(|e| anyhow::anyhow!("failed to get Raft metrics: {}", e))?;

            Ok(TuiRpcResponse::RaftMetrics(RaftMetricsResponse {
                node_id: ctx.node_id,
                state: format!("{:?}", metrics.state),
                current_leader: metrics.current_leader.map(|id| id.0),
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
                .map_err(|e| anyhow::anyhow!("failed to get leader: {}", e))?;
            Ok(TuiRpcResponse::Leader(leader))
        }

        TuiRpcRequest::GetNodeInfo => {
            let endpoint_addr = ctx.endpoint_manager.node_addr();
            Ok(TuiRpcResponse::NodeInfo(NodeInfoResponse {
                node_id: ctx.node_id,
                endpoint_addr: format!("{:?}", endpoint_addr),
            }))
        }

        TuiRpcRequest::GetClusterTicket => {
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
                    error: None,
                })),
                Err(e) => {
                    let (found, error) = match e {
                        crate::api::KeyValueStoreError::NotFound { .. } => (false, None),
                        crate::api::KeyValueStoreError::NotLeader { leader, reason } => (
                            false,
                            Some(format!("not leader: {}; leader={:?}", reason, leader)),
                        ),
                        other => (false, Some(other.to_string())),
                    };
                    Ok(TuiRpcResponse::ReadResult(ReadResultResponse {
                        value: None,
                        found,
                        error,
                    }))
                }
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

        TuiRpcRequest::GetClusterState => {
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

            Ok(TuiRpcResponse::ClusterState(ClusterStateResponse {
                nodes,
                leader_id,
                this_node_id: ctx.node_id,
            }))
        }
    }
}
