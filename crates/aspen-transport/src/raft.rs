//! Raft protocol handler (unauthenticated).
//!
//! This module provides `RaftProtocolHandler`, which handles Raft RPC
//! connections over Iroh with the `raft-rpc` ALPN.
//!
//! **Note:** This handler is for legacy/testing use. For production
//! deployments, use `AuthenticatedRaftProtocolHandler` instead.

use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

use iroh::endpoint::Connection;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use openraft::Raft;
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use aspen_core::raft::constants::MAX_CONCURRENT_CONNECTIONS;
use aspen_core::raft::constants::MAX_RPC_MESSAGE_SIZE;
use aspen_core::raft::constants::MAX_STREAMS_PER_CONNECTION;
use aspen_core::raft::rpc::RaftRpcProtocol;
use aspen_core::raft::rpc::RaftRpcResponse;
use aspen_raft_types::AppTypeConfig;

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
                return Err(AcceptError::from_err(std::io::Error::other("connection limit reached")));
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
async fn handle_raft_connection(connection: Connection, raft_core: Raft<AppTypeConfig>) -> anyhow::Result<()> {
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
    let buffer = recv.read_to_end(MAX_RPC_MESSAGE_SIZE as usize).await.context("failed to read RPC message")?;

    // Deserialize the RPC request
    let request: RaftRpcProtocol = postcard::from_bytes(&buffer).context("failed to deserialize RPC request")?;

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
    let response_bytes = postcard::to_stdvec(&response).context("failed to serialize RPC response")?;

    debug!(response_size = response_bytes.len(), "sending Raft RPC response");

    send.write_all(&response_bytes).await.context("failed to write RPC response")?;
    send.finish().context("failed to finish send stream")?;

    debug!("Raft RPC response sent successfully");

    Ok(())
}
