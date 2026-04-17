//! IRPC server for handling incoming Raft RPC requests over Iroh.
//!
//! This module provides the server-side implementation of the Raft RPC protocol.
//! It listens for incoming IRPC messages over Iroh connections and forwards them
//! to the Raft core for processing.
//!
//! Tiger Style:
//! - Explicit error handling for all I/O operations
//! - Bounded buffer sizes for stream reading
//! - Fail fast on deserialization errors
//! - Clean shutdown via cancellation token
//!
//! # Test Coverage
//!
//! RPC message handling is tested in `tests/server_rpc_test.rs`:
//!   - VoteRequest/VoteResponse serialization and mock RPC flow
//!   - AppendEntriesRequest/Response (all variants: Success, Conflict, HigherVote, PartialSuccess)
//!   - SnapshotRequest handling with large payloads (1MB+)
//!   - Timestamp wrapping for clock drift detection
//!   - Mock stream concurrent RPC tests
//!   - Network partition and connection close handling
//!
//! Server lifecycle and resource limits are tested in `tests/raft_server_integration_test.rs`:
//!   - Connection semaphore enforcement (TEST_MAX_CONNECTIONS for fast tests)
//!   - Stream semaphore enforcement (TEST_MAX_STREAMS for fast tests)
//!   - Semaphore cleanup on connection/stream close
//!   - Concurrent connection/stream stress tests
//!   - Connection lifecycle (establish, exchange, close)

use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

use anyhow::Context;
use anyhow::Result;
use aspen_constants::network::MAX_CONCURRENT_CONNECTIONS;
use aspen_constants::network::MAX_RPC_MESSAGE_SIZE;
use aspen_constants::network::MAX_STREAMS_PER_CONNECTION;
use iroh::Endpoint;
use openraft::Raft;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::Instrument;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::clock_drift_detection::current_time_ms;
use crate::rpc::RaftFatalErrorKind;
use crate::rpc::RaftRpcProtocol;
use crate::rpc::RaftRpcResponse;
use crate::rpc::RaftRpcResponseWithTimestamps;
use crate::rpc::TimestampInfo;
use crate::types::AppTypeConfig;

#[inline]
fn max_concurrent_connections_usize() -> usize {
    match usize::try_from(MAX_CONCURRENT_CONNECTIONS) {
        Ok(max_connections) => max_connections,
        Err(_) => usize::MAX,
    }
}

#[inline]
fn max_streams_per_connection_usize() -> usize {
    match usize::try_from(MAX_STREAMS_PER_CONNECTION) {
        Ok(max_streams) => max_streams,
        Err(_) => usize::MAX,
    }
}

#[inline]
fn max_rpc_message_size_usize() -> usize {
    match usize::try_from(MAX_RPC_MESSAGE_SIZE) {
        Ok(max_message_size) => max_message_size,
        Err(_) => usize::MAX,
    }
}

fn snapshot_from_request(snapshot_req: crate::rpc::RaftSnapshotRequest) -> openraft::Snapshot<AppTypeConfig> {
    openraft::Snapshot {
        meta: snapshot_req.snapshot_meta,
        snapshot: Cursor::new(snapshot_req.snapshot_data),
    }
}

struct RpcResponseTimestamps {
    server_recv_ms: u64,
    server_send_ms: u64,
}

fn response_with_timestamps(
    response: RaftRpcResponse,
    timestamps: RpcResponseTimestamps,
) -> RaftRpcResponseWithTimestamps {
    RaftRpcResponseWithTimestamps {
        inner: response,
        timestamps: Some(TimestampInfo {
            server_recv_ms: timestamps.server_recv_ms,
            server_send_ms: timestamps.server_send_ms,
        }),
    }
}

/// IRPC server for handling Raft RPC requests.
///
/// Spawns a task that listens for incoming Iroh connections and processes
/// RPC messages by forwarding them to the Raft core.
pub struct RaftRpcServer {
    join_handle: JoinHandle<()>,
    cancel_token: CancellationToken,
    task_tracker: TaskTracker,
    /// Expungement flag: when true, all incoming Raft RPCs are rejected.
    is_expunged: Arc<AtomicBool>,
}

impl RaftRpcServer {
    /// Spawn the IRPC server task.
    ///
    /// # Arguments
    /// * `endpoint` - Iroh endpoint to accept connections on
    /// * `raft_core` - Raft instance to forward RPCs to
    ///
    /// # Returns
    /// Server handle with graceful shutdown support.
    pub fn spawn(endpoint: Arc<Endpoint>, raft_core: Raft<AppTypeConfig>) -> Self {
        let cancel_token = CancellationToken::new();
        let cancel_clone = cancel_token.clone();
        let task_tracker = TaskTracker::new();
        let task_tracker_clone = task_tracker.clone();
        let is_expunged = Arc::new(AtomicBool::new(false));
        let is_expunged_clone = is_expunged.clone();

        let join_handle = tokio::spawn(async move {
            if let Err(err) = run_server(endpoint, raft_core, cancel_clone, task_tracker_clone, is_expunged_clone).await
            {
                error!(error = %err, "IRPC server task failed");
            }
        });

        Self {
            join_handle,
            cancel_token,
            task_tracker,
            is_expunged,
        }
    }

    /// Get the expungement flag.
    pub fn expunged_flag(&self) -> &Arc<AtomicBool> {
        &self.is_expunged
    }

    /// Shutdown the server gracefully.
    pub async fn shutdown(self) -> Result<()> {
        info!("shutting down IRPC server");
        self.cancel_token.cancel();
        self.task_tracker.close();
        self.task_tracker.wait().await;
        self.join_handle.await.context("IRPC server task panicked")?;
        Ok(())
    }
}

/// Main server loop that accepts incoming connections and processes RPCs.
///
/// Tiger Style: Bounded connection count to prevent DoS attacks.
async fn run_server(
    endpoint: Arc<Endpoint>,
    raft_core: Raft<AppTypeConfig>,
    cancel: CancellationToken,
    task_tracker: TaskTracker,
    is_expunged: Arc<AtomicBool>,
) -> Result<()> {
    debug_assert!(MAX_CONCURRENT_CONNECTIONS > 0, "SERVER: max connections must be positive");

    // Tiger Style: Fixed limit on concurrent connections to prevent resource exhaustion
    let connection_semaphore = Arc::new(Semaphore::new(max_concurrent_connections_usize()));
    debug_assert!(
        connection_semaphore.available_permits() <= max_concurrent_connections_usize(),
        "SERVER: initial permit count must not exceed configured max"
    );

    info!(max_connections = MAX_CONCURRENT_CONNECTIONS, "IRPC server listening for incoming connections");

    for accept_iteration in 0..u32::MAX {
        debug_assert!(accept_iteration < u32::MAX, "SERVER: accept loop must stay bounded");
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("IRPC server received shutdown signal");
                break;
            }
            incoming = endpoint.accept() => {
                let Some(incoming) = incoming else {
                    warn!("endpoint closed, stopping IRPC server");
                    break;
                };

                // Expunged nodes reject all Raft RPCs
                if is_expunged.load(Ordering::Acquire) {
                    warn!("rejecting Raft connection: this node has been expunged");
                    continue;
                }

                info!(
                    remote_addr = ?incoming.remote_addr(),
                    "received incoming connection"
                );

                // Try to acquire a connection permit
                let permit = match connection_semaphore.clone().try_acquire_owned() {
                    Ok(permit) => permit,
                    Err(_) => {
                        warn!("connection limit reached ({}), rejecting connection", MAX_CONCURRENT_CONNECTIONS);
                        // Drop the incoming connection by not processing it
                        continue;
                    }
                };
                debug_assert!(
                    connection_semaphore.available_permits() < max_concurrent_connections_usize(),
                    "SERVER: acquired connection permit should reduce remaining capacity"
                );

                let raft_core_clone = raft_core.clone();
                let stream_tracker = task_tracker.clone();
                task_tracker.spawn(async move {
                    // Permit is held for the duration of the connection
                    let _permit = permit;
                    if let Err(err) = handle_connection(incoming, raft_core_clone, stream_tracker).await {
                        error!(error = %err, "failed to handle incoming connection");
                    }
                });
            }
        }
    }

    Ok(())
}

/// Handle a single incoming Iroh connection.
///
/// Tiger Style: Bounded stream count per connection to prevent resource exhaustion.
async fn handle_connection(
    connecting: iroh::endpoint::Incoming,
    raft_core: Raft<AppTypeConfig>,
    task_tracker: TaskTracker,
) -> Result<()> {
    async move {
        debug_assert!(MAX_STREAMS_PER_CONNECTION > 0, "SERVER: max streams per connection must be positive");
        info!("awaiting incoming connection completion");
        let connection = connecting.await.context("failed to accept connection")?;
        let remote_node_id = connection.remote_id();

        info!(remote_node = %remote_node_id, "accepted connection, waiting for streams");

        // Tiger Style: Fixed limit on concurrent streams per connection
        let stream_semaphore = Arc::new(Semaphore::new(max_streams_per_connection_usize()));
        let active_streams = Arc::new(AtomicU32::new(0));
        debug_assert!(
            stream_semaphore.available_permits() <= max_streams_per_connection_usize(),
            "SERVER: stream permit count must not exceed configured max"
        );

        // Accept bidirectional streams from this connection
        info!(remote_node = %remote_node_id, "starting accept_bi loop");
        for stream_accept_iteration in 0..u32::MAX {
            debug_assert!(stream_accept_iteration < u32::MAX, "SERVER: stream accept loop must stay bounded");
            info!(remote_node = %remote_node_id, "waiting for stream via accept_bi");
            let stream = match connection.accept_bi().await {
                Ok(stream) => {
                    info!(remote_node = %remote_node_id, "accepted bidirectional stream");
                    stream
                }
                Err(err) => {
                    // Connection closed or error
                    info!(remote_node = %remote_node_id, error = %err, "connection closed");
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
                        "stream limit reached, dropping stream"
                    );
                    // Drop the stream by not processing it
                    continue;
                }
            };

            let prior_active_streams = active_streams.fetch_add(1, Ordering::Relaxed);
            debug_assert!(
                prior_active_streams < MAX_STREAMS_PER_CONNECTION,
                "SERVER: active stream count {} must stay below configured max {}",
                prior_active_streams,
                MAX_STREAMS_PER_CONNECTION
            );
            let active_streams_clone = active_streams.clone();

            let raft_core_clone = raft_core.clone();
            let (send, recv) = stream;
            task_tracker.spawn(async move {
                // Permit is held for the duration of the stream
                let _permit = permit;
                if let Err(err) = handle_rpc_stream((recv, send), raft_core_clone).await {
                    error!(error = %err, "failed to handle RPC stream");
                }
                active_streams_clone.fetch_sub(1, Ordering::Relaxed);
            });
        }

        Ok(())
    }
    .instrument(tracing::info_span!("handle_connection"))
    .await
}

/// Handle a single RPC message on a bidirectional stream.
///
/// This function ensures that EVERY RPC receives a response, even when the
/// RaftCore is in a fatal state (panicked, stopped, or storage error).
/// This prevents clients from receiving empty responses and allows proper
/// failure detection.
async fn handle_rpc_stream(
    (mut recv, mut send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    raft_core: Raft<AppTypeConfig>,
) -> Result<()> {
    async move {
        info!("handle_rpc_stream started, reading RPC message");

        // Read and deserialize the RPC request
        let (request, server_recv_ms) = handle_rpc_read_request(&mut recv).await?;

        // Process the RPC and create response
        let response = handle_rpc_process_request(request, &raft_core).await;

        // Send the response with timestamps
        handle_rpc_send_response(&mut send, response, server_recv_ms).await?;

        info!("RPC response sent successfully");
        Ok(())
    }
    .instrument(tracing::info_span!("handle_rpc_stream"))
    .await
}

/// Read and deserialize an RPC request from the stream.
async fn handle_rpc_read_request(recv: &mut iroh::endpoint::RecvStream) -> Result<(RaftRpcProtocol, u64)> {
    let buffer = recv.read_to_end(max_rpc_message_size_usize()).await.context("failed to read RPC message")?;
    let server_recv_ms = current_time_ms();
    debug!(buffer_size = buffer.len(), "read RPC message bytes");

    let request: RaftRpcProtocol = postcard::from_bytes(&buffer).context("failed to deserialize RPC request")?;
    debug!(request_type = ?request, "received and deserialized RPC request");

    Ok((request, server_recv_ms))
}

/// Process an RPC request and return the response.
async fn handle_rpc_process_request(request: RaftRpcProtocol, raft_core: &Raft<AppTypeConfig>) -> RaftRpcResponse {
    match request {
        RaftRpcProtocol::Vote(vote_req) => handle_rpc_vote(vote_req, raft_core).await,
        RaftRpcProtocol::AppendEntries(append_req) => handle_rpc_append_entries(append_req, raft_core).await,
        RaftRpcProtocol::InstallSnapshot(snapshot_req) => handle_rpc_install_snapshot(snapshot_req, raft_core).await,
    }
}

/// Handle a Vote RPC request.
async fn handle_rpc_vote(vote_req: crate::rpc::RaftVoteRequest, raft_core: &Raft<AppTypeConfig>) -> RaftRpcResponse {
    match raft_core.vote(vote_req.request).await {
        Ok(result) => RaftRpcResponse::Vote(result),
        Err(openraft::error::RaftError::Fatal(fatal)) => {
            let error_kind = RaftFatalErrorKind::from_fatal(&fatal);
            error!(
                error_kind = %error_kind,
                fatal_error = ?fatal,
                rpc_type = "vote",
                "RaftCore in fatal state, sending error response to client"
            );
            RaftRpcResponse::FatalError(error_kind)
        }
        Err(openraft::error::RaftError::APIError(api_err)) => {
            error!(api_error = ?api_err, "unexpected API error in vote RPC");
            RaftRpcResponse::FatalError(RaftFatalErrorKind::Panicked)
        }
    }
}

/// Handle an AppendEntries RPC request.
async fn handle_rpc_append_entries(
    append_req: crate::rpc::RaftAppendEntriesRequest,
    raft_core: &Raft<AppTypeConfig>,
) -> RaftRpcResponse {
    match raft_core.append_entries(append_req.request).await {
        Ok(result) => RaftRpcResponse::AppendEntries(result),
        Err(openraft::error::RaftError::Fatal(fatal)) => {
            let error_kind = RaftFatalErrorKind::from_fatal(&fatal);
            error!(
                error_kind = %error_kind,
                fatal_error = ?fatal,
                rpc_type = "append_entries",
                "RaftCore in fatal state, sending error response to client"
            );
            RaftRpcResponse::FatalError(error_kind)
        }
        Err(openraft::error::RaftError::APIError(api_err)) => {
            error!(api_error = ?api_err, "unexpected API error in append_entries RPC");
            RaftRpcResponse::FatalError(RaftFatalErrorKind::Panicked)
        }
    }
}

/// Handle an InstallSnapshot RPC request.
async fn handle_rpc_install_snapshot(
    snapshot_req: crate::rpc::RaftSnapshotRequest,
    raft_core: &Raft<AppTypeConfig>,
) -> RaftRpcResponse {
    let vote = snapshot_req.vote;
    let snapshot = snapshot_from_request(snapshot_req);
    match raft_core.install_full_snapshot(vote, snapshot).await {
        Ok(result) => RaftRpcResponse::InstallSnapshot(Ok(result)),
        Err(fatal) => {
            let error_kind = RaftFatalErrorKind::from_fatal(&fatal);
            error!(
                error_kind = %error_kind,
                fatal_error = ?fatal,
                rpc_type = "install_snapshot",
                "RaftCore in fatal state, sending error response to client"
            );
            RaftRpcResponse::FatalError(error_kind)
        }
    }
}

/// Send an RPC response with timestamps.
async fn handle_rpc_send_response(
    send: &mut iroh::endpoint::SendStream,
    response: RaftRpcResponse,
    server_recv_ms: u64,
) -> Result<()> {
    let server_send_ms = current_time_ms();

    if let RaftRpcResponse::FatalError(kind) = &response {
        warn!(
            error_kind = %kind,
            "sending fatal error response to client - node requires attention"
        );
    }

    let response_with_timestamps = response_with_timestamps(response, RpcResponseTimestamps {
        server_recv_ms,
        server_send_ms,
    });

    let response_bytes = postcard::to_stdvec(&response_with_timestamps).context("failed to serialize RPC response")?;
    debug_assert!(!response_bytes.is_empty(), "SERVER: serialized response must not be empty");
    debug_assert!(
        response_bytes.len() <= max_rpc_message_size_usize(),
        "SERVER: serialized response size {} exceeds configured max {}",
        response_bytes.len(),
        max_rpc_message_size_usize()
    );
    info!(response_size = response_bytes.len(), "sending RPC response");

    send.write_all(&response_bytes).await.context("failed to write RPC response")?;
    send.finish().context("failed to finish send stream")?;

    Ok(())
}
