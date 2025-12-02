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

use std::io::Cursor;
use std::sync::Arc;

use anyhow::{Context, Result};
use openraft::Raft;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::cluster::IrohEndpointManager;
use crate::raft::rpc::{RaftRpcProtocol, RaftRpcResponse};
use crate::raft::types::AppTypeConfig;

/// Maximum size for RPC messages (10 MB).
///
/// Tiger Style: Fixed limit to prevent unbounded memory use.
const MAX_RPC_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// IRPC server for handling Raft RPC requests.
///
/// Spawns a task that listens for incoming Iroh connections and processes
/// RPC messages by forwarding them to the Raft core.
pub struct RaftRpcServer {
    join_handle: JoinHandle<()>,
    cancel_token: CancellationToken,
}

impl RaftRpcServer {
    /// Spawn the IRPC server task.
    ///
    /// # Arguments
    /// * `endpoint_manager` - Iroh endpoint to accept connections on
    /// * `raft_core` - Raft instance to forward RPCs to
    ///
    /// # Returns
    /// Server handle with graceful shutdown support.
    pub fn spawn(
        endpoint_manager: Arc<IrohEndpointManager>,
        raft_core: Raft<AppTypeConfig>,
    ) -> Self {
        let cancel_token = CancellationToken::new();
        let cancel_clone = cancel_token.clone();

        let join_handle = tokio::spawn(async move {
            if let Err(err) = run_server(endpoint_manager, raft_core, cancel_clone).await {
                error!(error = %err, "IRPC server task failed");
            }
        });

        Self {
            join_handle,
            cancel_token,
        }
    }

    /// Shutdown the server gracefully.
    pub async fn shutdown(self) -> Result<()> {
        info!("shutting down IRPC server");
        self.cancel_token.cancel();
        self.join_handle
            .await
            .context("IRPC server task panicked")?;
        Ok(())
    }
}

/// Main server loop that accepts incoming connections and processes RPCs.
async fn run_server(
    endpoint_manager: Arc<IrohEndpointManager>,
    raft_core: Raft<AppTypeConfig>,
    cancel: CancellationToken,
) -> Result<()> {
    let endpoint = endpoint_manager.endpoint();

    info!("IRPC server listening for incoming connections");

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

                let raft_core_clone = raft_core.clone();
                tokio::spawn(async move {
                    if let Err(err) = handle_connection(incoming, raft_core_clone).await {
                        error!(error = %err, "failed to handle incoming connection");
                    }
                });
            }
        }
    }

    Ok(())
}

/// Handle a single incoming Iroh connection.
async fn handle_connection(
    connecting: iroh::endpoint::Incoming,
    raft_core: Raft<AppTypeConfig>,
) -> Result<()> {
    let connection = connecting.await.context("failed to accept connection")?;
    let remote_node_id = connection.remote_id();

    debug!(remote_node = %remote_node_id, "accepted connection");

    // Accept bidirectional streams from this connection
    loop {
        let stream = match connection.accept_bi().await {
            Ok(stream) => stream,
            Err(err) => {
                // Connection closed or error
                debug!(remote_node = %remote_node_id, error = %err, "connection closed");
                break;
            }
        };

        let raft_core_clone = raft_core.clone();
        let (send, recv) = stream;
        tokio::spawn(async move {
            if let Err(err) = handle_rpc_stream((recv, send), raft_core_clone).await {
                error!(error = %err, "failed to handle RPC stream");
            }
        });
    }

    Ok(())
}

/// Handle a single RPC message on a bidirectional stream.
async fn handle_rpc_stream(
    (mut recv, mut send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    raft_core: Raft<AppTypeConfig>,
) -> Result<()> {
    // Read the RPC message with size limit
    let buffer = recv
        .read_to_end(MAX_RPC_MESSAGE_SIZE)
        .await
        .context("failed to read RPC message")?;

    // Deserialize the RPC request (protocol enum without channels)
    let request: RaftRpcProtocol = postcard::from_bytes(&buffer)
        .context("failed to deserialize RPC request")?;

    debug!(request_type = ?request, "received RPC request");

    // Process the RPC and create response
    let response = match request {
        RaftRpcProtocol::Vote(vote_req) => {
            // vote() returns Result<T, Infallible>, unwrap to get T
            let result = raft_core.vote(vote_req.request).await.unwrap();
            RaftRpcResponse::Vote(result)
        }
        RaftRpcProtocol::AppendEntries(append_req) => {
            // append_entries() returns Result<T, Infallible>, unwrap to get T
            let result = raft_core.append_entries(append_req.request).await.unwrap();
            RaftRpcResponse::AppendEntries(result)
        }
        RaftRpcProtocol::InstallSnapshot(snapshot_req) => {
            // Convert snapshot bytes back to Cursor for Raft
            let snapshot_cursor = Cursor::new(snapshot_req.snapshot_data);
            let snapshot = openraft::Snapshot {
                meta: snapshot_req.snapshot_meta,
                snapshot: snapshot_cursor,
            };
            let result = raft_core
                .install_full_snapshot(snapshot_req.vote, snapshot)
                .await
                .map_err(|fatal_err| openraft::error::RaftError::Fatal(fatal_err));
            RaftRpcResponse::InstallSnapshot(result)
        }
    };

    // Serialize and send response
    let response_bytes = postcard::to_stdvec(&response)
        .context("failed to serialize RPC response")?;
    send.write_all(&response_bytes)
        .await
        .context("failed to write RPC response")?;
    send.finish().context("failed to finish send stream")?;

    Ok(())
}
