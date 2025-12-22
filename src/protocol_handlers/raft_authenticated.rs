//! Authenticated Raft protocol handler.
//!
//! This module provides `AuthenticatedRaftProtocolHandler`, which handles
//! authenticated Raft RPC connections over Iroh with the `raft-auth` ALPN.
//!
//! Uses HMAC-SHA256 challenge-response authentication to verify that connecting
//! nodes share the same cluster cookie before processing Raft RPCs.

use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};
use openraft::Raft;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, instrument, warn};

use crate::raft::auth::{
    AUTH_HANDSHAKE_TIMEOUT, AuthContext, AuthResponse, AuthResult, MAX_AUTH_MESSAGE_SIZE,
};
use crate::raft::constants::{
    MAX_CONCURRENT_CONNECTIONS, MAX_RPC_MESSAGE_SIZE, MAX_STREAMS_PER_CONNECTION,
};
use crate::raft::rpc::{RaftRpcProtocol, RaftRpcResponse};
use crate::raft::types::AppTypeConfig;

/// Protocol handler for authenticated Raft RPC over Iroh.
///
/// Uses HMAC-SHA256 challenge-response authentication to verify that connecting
/// nodes share the same cluster cookie before processing Raft RPCs.
///
/// # Tiger Style
///
/// - Bounded connection count via semaphore
/// - Explicit authentication timeout
/// - Fixed nonce and HMAC sizes
#[derive(Debug)]
pub struct AuthenticatedRaftProtocolHandler {
    raft_core: Raft<AppTypeConfig>,
    auth_context: AuthContext,
    connection_semaphore: Arc<Semaphore>,
}

impl AuthenticatedRaftProtocolHandler {
    /// Create a new authenticated Raft protocol handler.
    ///
    /// # Arguments
    /// * `raft_core` - Raft instance to forward RPCs to
    /// * `cluster_cookie` - Shared secret for authentication
    ///
    /// # Security Note
    /// Authentication is handled by HMAC-SHA256 challenge-response using the cluster cookie.
    /// Iroh already provides cryptographic identity verification at the transport layer.
    pub fn new(raft_core: Raft<AppTypeConfig>, cluster_cookie: &str) -> Self {
        Self {
            raft_core,
            auth_context: AuthContext::new(cluster_cookie),
            connection_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS as usize)),
        }
    }
}

impl ProtocolHandler for AuthenticatedRaftProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote_node_id = connection.remote_id();

        // Try to acquire a connection permit
        let permit = match self.connection_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    "Authenticated Raft connection limit reached ({}), rejecting connection from {}",
                    MAX_CONCURRENT_CONNECTIONS, remote_node_id
                );
                return Err(AcceptError::from_err(std::io::Error::other(
                    "connection limit reached",
                )));
            }
        };

        debug!(remote_node = %remote_node_id, "accepted authenticated Raft RPC connection");

        // Handle the connection with authentication
        let result = handle_authenticated_raft_connection(
            connection,
            self.raft_core.clone(),
            self.auth_context.clone(),
        )
        .await;

        drop(permit);

        result.map_err(|err| AcceptError::from_err(std::io::Error::other(err.to_string())))
    }

    async fn shutdown(&self) {
        info!("Authenticated Raft protocol handler shutting down");
        self.connection_semaphore.close();
    }
}

/// Handle an authenticated Raft RPC connection.
///
/// Performs challenge-response authentication before processing RPCs.
#[instrument(skip(connection, raft_core, auth_context))]
async fn handle_authenticated_raft_connection(
    connection: Connection,
    raft_core: Raft<AppTypeConfig>,
    auth_context: AuthContext,
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
                debug!(remote_node = %remote_node_id, error = %err, "Authenticated Raft connection closed");
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
                    "Authenticated Raft stream limit reached, dropping stream"
                );
                continue;
            }
        };

        active_streams.fetch_add(1, Ordering::Relaxed);
        let active_streams_clone = active_streams.clone();

        let raft_core_clone = raft_core.clone();
        let auth_context_clone = auth_context.clone();
        let (send, recv) = stream;
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(err) =
                handle_authenticated_raft_stream((recv, send), raft_core_clone, auth_context_clone)
                    .await
            {
                error!(error = %err, "failed to handle authenticated Raft RPC stream");
            }
            active_streams_clone.fetch_sub(1, Ordering::Relaxed);
        });
    }

    Ok(())
}

/// Handle a single authenticated Raft RPC stream.
///
/// Uses length-prefixed framing for auth messages so they can share a stream with RPC:
/// - Each auth message is prefixed with a 4-byte big-endian length
/// - RPC message uses read_to_end after auth completes (client finishes send)
///
/// Protocol:
/// 1. Send AuthChallenge (length-prefixed)
/// 2. Receive AuthResponse (length-prefixed)
/// 3. Verify HMAC
/// 4. Send AuthResult (length-prefixed)
/// 5. If authenticated, read RPC (read_to_end) and process
#[instrument(skip(recv, send, raft_core, auth_context))]
async fn handle_authenticated_raft_stream(
    (mut recv, mut send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    raft_core: Raft<AppTypeConfig>,
    auth_context: AuthContext,
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Step 1: Generate and send challenge (length-prefixed)
    let challenge = auth_context.generate_challenge();
    let challenge_bytes =
        postcard::to_stdvec(&challenge).context("failed to serialize challenge")?;
    write_length_prefixed(&mut send, &challenge_bytes)
        .await
        .context("failed to send challenge")?;

    // Step 2: Receive response with timeout (length-prefixed)
    let response_result = tokio::time::timeout(AUTH_HANDSHAKE_TIMEOUT, async {
        let buffer = read_length_prefixed(&mut recv, MAX_AUTH_MESSAGE_SIZE)
            .await
            .context("failed to read auth response")?;
        let response: AuthResponse =
            postcard::from_bytes(&buffer).context("failed to deserialize auth response")?;
        Ok::<_, anyhow::Error>(response)
    })
    .await;

    let response = match response_result {
        Ok(Ok(response)) => response,
        Ok(Err(err)) => {
            warn!(error = %err, "authentication failed: bad response");
            let result_bytes = postcard::to_stdvec(&AuthResult::Failed)
                .context("failed to serialize auth result")?;
            // Best-effort send of failure response - log if it fails but don't block error return
            if let Err(write_err) = write_length_prefixed(&mut send, &result_bytes).await {
                debug!(error = %write_err, "failed to send auth failure response to client");
            }
            if let Err(finish_err) = send.finish() {
                debug!(error = %finish_err, "failed to finish stream after auth failure");
            }
            return Err(err);
        }
        Err(_) => {
            warn!("authentication timed out");
            let result_bytes = postcard::to_stdvec(&AuthResult::Failed)
                .context("failed to serialize auth result")?;
            // Best-effort send of failure response - log if it fails but don't block error return
            if let Err(write_err) = write_length_prefixed(&mut send, &result_bytes).await {
                debug!(error = %write_err, "failed to send auth timeout response to client");
            }
            if let Err(finish_err) = send.finish() {
                debug!(error = %finish_err, "failed to finish stream after auth timeout");
            }
            return Err(anyhow::anyhow!("authentication timeout"));
        }
    };

    // Step 3: Verify the response
    let auth_result = auth_context.verify_response(&challenge, &response);

    // Step 4: Send result (length-prefixed)
    let result_bytes =
        postcard::to_stdvec(&auth_result).context("failed to serialize auth result")?;
    write_length_prefixed(&mut send, &result_bytes)
        .await
        .context("failed to send auth result")?;

    if !auth_result.is_ok() {
        warn!(result = ?auth_result, "authentication failed");
        if let Err(finish_err) = send.finish() {
            debug!(error = %finish_err, "failed to finish stream after auth verification failure");
        }
        return Err(anyhow::anyhow!("authentication failed: {:?}", auth_result));
    }

    debug!("authentication successful, processing Raft RPC");

    // Step 5: Read and process the Raft RPC message (read_to_end - client finishes send)
    let buffer = recv
        .read_to_end(MAX_RPC_MESSAGE_SIZE as usize)
        .await
        .context("failed to read RPC message")?;

    let request: RaftRpcProtocol =
        postcard::from_bytes(&buffer).context("failed to deserialize RPC request")?;

    debug!(request_type = ?request, "received authenticated Raft RPC request");

    // Process the RPC (same as unauthenticated handler)
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

    let response_bytes =
        postcard::to_stdvec(&response).context("failed to serialize RPC response")?;

    send.write_all(&response_bytes)
        .await
        .context("failed to write RPC response")?;
    send.finish().context("failed to finish send stream")?;

    debug!("authenticated Raft RPC response sent successfully");

    Ok(())
}

/// Read a length-prefixed message from a stream.
///
/// Format: 4-byte big-endian length + payload
/// Tiger Style: Bounded by max_size to prevent memory exhaustion.
pub async fn read_length_prefixed(
    recv: &mut iroh::endpoint::RecvStream,
    max_size: usize,
) -> anyhow::Result<Vec<u8>> {
    use anyhow::Context;

    // Read 4-byte length prefix
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf)
        .await
        .context("failed to read length prefix")?;
    let len = u32::from_be_bytes(len_buf) as usize;

    // Validate length
    if len > max_size {
        return Err(anyhow::anyhow!("message too large: {} > {}", len, max_size));
    }

    // Read payload
    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf)
        .await
        .context("failed to read message payload")?;

    Ok(buf)
}

/// Write a length-prefixed message to a stream.
///
/// Format: 4-byte big-endian length + payload
pub async fn write_length_prefixed(
    send: &mut iroh::endpoint::SendStream,
    data: &[u8],
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Write 4-byte length prefix
    let len = data.len() as u32;
    send.write_all(&len.to_be_bytes())
        .await
        .context("failed to write length prefix")?;

    // Write payload
    send.write_all(data)
        .await
        .context("failed to write message payload")?;

    Ok(())
}
