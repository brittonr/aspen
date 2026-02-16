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
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use crate::clock_drift_detection::current_time_ms;
use crate::rpc::RaftFatalErrorKind;
use crate::rpc::RaftRpcProtocol;
use crate::rpc::RaftRpcResponse;
use crate::rpc::RaftRpcResponseWithTimestamps;
use crate::rpc::TimestampInfo;
use crate::types::AppTypeConfig;

/// IRPC server for handling Raft RPC requests.
///
/// Spawns a task that listens for incoming Iroh connections and processes
/// RPC messages by forwarding them to the Raft core.
pub struct RaftRpcServer {
    join_handle: JoinHandle<()>,
    cancel_token: CancellationToken,
    task_tracker: TaskTracker,
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

        let join_handle = tokio::spawn(async move {
            if let Err(err) = run_server(endpoint, raft_core, cancel_clone, task_tracker_clone).await {
                error!(error = %err, "IRPC server task failed");
            }
        });

        Self {
            join_handle,
            cancel_token,
            task_tracker,
        }
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
) -> Result<()> {
    // Tiger Style: Fixed limit on concurrent connections to prevent resource exhaustion
    let connection_semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS as usize));

    info!(max_connections = MAX_CONCURRENT_CONNECTIONS, "IRPC server listening for incoming connections");

    loop {
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

                info!(
                    remote_addr = ?incoming.remote_address(),
                    "received incoming connection"
                );

                // We'll check ALPN after accepting the connection
                // Note: In Iroh, connections are already filtered by ALPN at the endpoint level,
                // but we should handle this gracefully if there's a race condition

                // Try to acquire a connection permit
                let permit = match connection_semaphore.clone().try_acquire_owned() {
                    Ok(permit) => permit,
                    Err(_) => {
                        warn!("connection limit reached ({}), rejecting connection", MAX_CONCURRENT_CONNECTIONS);
                        // Drop the incoming connection by not processing it
                        continue;
                    }
                };

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
#[instrument(skip(connecting, raft_core, task_tracker))]
async fn handle_connection(
    connecting: iroh::endpoint::Incoming,
    raft_core: Raft<AppTypeConfig>,
    task_tracker: TaskTracker,
) -> Result<()> {
    info!("awaiting incoming connection completion");
    let connection = connecting.await.context("failed to accept connection")?;
    let remote_node_id = connection.remote_id();

    info!(remote_node = %remote_node_id, "accepted connection, waiting for streams");

    // Tiger Style: Fixed limit on concurrent streams per connection
    let stream_semaphore = Arc::new(Semaphore::new(MAX_STREAMS_PER_CONNECTION as usize));
    let active_streams = Arc::new(AtomicU32::new(0));

    // Accept bidirectional streams from this connection
    info!(remote_node = %remote_node_id, "starting accept_bi loop");
    loop {
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

        active_streams.fetch_add(1, Ordering::Relaxed);
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

/// Handle a single RPC message on a bidirectional stream.
///
/// This function ensures that EVERY RPC receives a response, even when the
/// RaftCore is in a fatal state (panicked, stopped, or storage error).
/// This prevents clients from receiving empty responses and allows proper
/// failure detection.
#[instrument(skip(recv, send, raft_core))]
async fn handle_rpc_stream(
    (mut recv, mut send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    raft_core: Raft<AppTypeConfig>,
) -> Result<()> {
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

/// Read and deserialize an RPC request from the stream.
async fn handle_rpc_read_request(recv: &mut iroh::endpoint::RecvStream) -> Result<(RaftRpcProtocol, u64)> {
    let buffer = recv.read_to_end(MAX_RPC_MESSAGE_SIZE as usize).await.context("failed to read RPC message")?;
    let server_recv_ms = current_time_ms();
    info!(buffer_size = buffer.len(), "read RPC message bytes");

    let request: RaftRpcProtocol = postcard::from_bytes(&buffer).context("failed to deserialize RPC request")?;
    info!(request_type = ?request, "received and deserialized RPC request");

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
    let snapshot_cursor = Cursor::new(snapshot_req.snapshot_data);
    let snapshot = openraft::Snapshot {
        meta: snapshot_req.snapshot_meta,
        snapshot: snapshot_cursor,
    };
    match raft_core.install_full_snapshot(snapshot_req.vote, snapshot).await {
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

    let response_with_timestamps = RaftRpcResponseWithTimestamps {
        inner: response,
        timestamps: Some(TimestampInfo {
            server_recv_ms,
            server_send_ms,
        }),
    };

    let response_bytes = postcard::to_stdvec(&response_with_timestamps).context("failed to serialize RPC response")?;
    info!(response_size = response_bytes.len(), "sending RPC response");

    send.write_all(&response_bytes).await.context("failed to write RPC response")?;
    send.finish().context("failed to finish send stream")?;

    Ok(())
}
