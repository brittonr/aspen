//! Sharded Raft protocol handler.
//!
//! This module provides `ShardedRaftProtocolHandler`, which handles Raft RPC
//! for multiple shards over a single ALPN (`raft-shard`). Each message is
//! prefixed with a 4-byte big-endian shard ID that routes to the appropriate
//! Raft core.
//!
//! # Wire Format
//!
//! Request:
//! ```text
//! +----------------+------------------------+
//! | shard_id (4B)  | RaftRpcProtocol (var)  |
//! +----------------+------------------------+
//! ```
//!
//! Response:
//! ```text
//! +----------------+------------------------+
//! | shard_id (4B)  | RaftRpcResponse (var)  |
//! +----------------+------------------------+
//! ```
//!
//! # Tiger Style
//!
//! - Bounded connection count via semaphore
//! - Bounded stream count per connection
//! - Explicit size limits on RPC messages
//! - HashMap access is O(1) for shard lookup

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

use iroh::endpoint::Connection;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use openraft::Raft;
use parking_lot::RwLock;
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use aspen_raft_types::MAX_CONCURRENT_CONNECTIONS;
use aspen_raft_types::MAX_RPC_MESSAGE_SIZE;
use aspen_raft_types::MAX_STREAMS_PER_CONNECTION;
use crate::rpc::{AppTypeConfig, RaftRpcProtocol, RaftRpcResponse, SHARD_PREFIX_SIZE, encode_shard_prefix, try_decode_shard_prefix};
use aspen_sharding::router::ShardId;

/// Protocol handler for sharded Raft RPC over Iroh.
///
/// Handles RPC for multiple Raft shards over a single ALPN connection.
/// Each message is prefixed with a 4-byte shard ID that routes to the
/// appropriate Raft core.
///
/// # Registration
///
/// Shard Raft instances must be registered with `register_shard()` before
/// they can receive RPCs. Unregistered shards will receive error responses.
///
/// # Thread Safety
///
/// The handler is thread-safe. Shard registration uses a RwLock for
/// concurrent read access during RPC handling with exclusive write
/// access for registration/deregistration.
///
/// # Tiger Style
///
/// - Bounded connection count via semaphore
/// - Bounded stream count per connection
/// - O(1) shard lookup via HashMap
#[derive(Debug)]
pub struct ShardedRaftProtocolHandler {
    /// Map of shard ID to Raft core.
    shard_cores: Arc<RwLock<HashMap<ShardId, Raft<AppTypeConfig>>>>,
    /// Semaphore to limit concurrent connections.
    connection_semaphore: Arc<Semaphore>,
}

impl ShardedRaftProtocolHandler {
    /// Create a new sharded Raft protocol handler.
    pub fn new() -> Self {
        Self {
            shard_cores: Arc::new(RwLock::new(HashMap::new())),
            connection_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS as usize)),
        }
    }

    /// Register a Raft core for a shard.
    ///
    /// # Arguments
    /// * `shard_id` - The shard ID to register
    /// * `raft_core` - The Raft instance for this shard
    ///
    /// # Returns
    /// The previous Raft core if one was already registered for this shard.
    pub fn register_shard(&self, shard_id: ShardId, raft_core: Raft<AppTypeConfig>) -> Option<Raft<AppTypeConfig>> {
        let mut cores = self.shard_cores.write();
        let previous = cores.insert(shard_id, raft_core);
        info!(shard_id, replaced = previous.is_some(), "registered shard");
        previous
    }

    /// Deregister a Raft core for a shard.
    ///
    /// # Arguments
    /// * `shard_id` - The shard ID to deregister
    ///
    /// # Returns
    /// The Raft core if one was registered for this shard.
    pub fn deregister_shard(&self, shard_id: ShardId) -> Option<Raft<AppTypeConfig>> {
        let mut cores = self.shard_cores.write();
        let removed = cores.remove(&shard_id);
        info!(shard_id, removed = removed.is_some(), "deregistered shard");
        removed
    }

    /// Check if a shard is registered.
    pub fn has_shard(&self, shard_id: ShardId) -> bool {
        self.shard_cores.read().contains_key(&shard_id)
    }

    /// Get the number of registered shards.
    pub fn shard_count(&self) -> usize {
        self.shard_cores.read().len()
    }

    /// Get a list of registered shard IDs.
    pub fn registered_shards(&self) -> Vec<ShardId> {
        self.shard_cores.read().keys().copied().collect()
    }
}

impl Default for ShardedRaftProtocolHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolHandler for ShardedRaftProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote_node_id = connection.remote_id();

        // Try to acquire a connection permit
        let permit = match self.connection_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    "Sharded Raft connection limit reached ({}), rejecting connection from {}",
                    MAX_CONCURRENT_CONNECTIONS, remote_node_id
                );
                return Err(AcceptError::from_err(std::io::Error::other("connection limit reached")));
            }
        };

        debug!(remote_node = %remote_node_id, "accepted sharded Raft RPC connection");

        // Handle the connection with bounded resources
        let shard_cores = Arc::clone(&self.shard_cores);
        let result = handle_sharded_connection(connection, shard_cores).await;

        // Release permit when done
        drop(permit);

        result.map_err(|err| AcceptError::from_err(std::io::Error::other(err.to_string())))
    }

    async fn shutdown(&self) {
        info!("Sharded Raft protocol handler shutting down");
        // Close the semaphore to prevent new connections
        self.connection_semaphore.close();
    }
}

/// Handle a single sharded Raft RPC connection.
///
/// Tiger Style: Bounded stream count per connection.
#[instrument(skip(connection, shard_cores))]
async fn handle_sharded_connection(
    connection: Connection,
    shard_cores: Arc<RwLock<HashMap<ShardId, Raft<AppTypeConfig>>>>,
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
                debug!(remote_node = %remote_node_id, error = %err, "sharded Raft connection closed");
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
                    "sharded Raft stream limit reached, dropping stream"
                );
                continue;
            }
        };

        active_streams.fetch_add(1, Ordering::Relaxed);
        let active_streams_clone = active_streams.clone();

        let shard_cores_clone = Arc::clone(&shard_cores);
        let (send, recv) = stream;
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(err) = handle_sharded_rpc_stream((recv, send), shard_cores_clone).await {
                error!(error = %err, "failed to handle sharded Raft RPC stream");
            }
            active_streams_clone.fetch_sub(1, Ordering::Relaxed);
        });
    }

    Ok(())
}

/// Handle a single sharded Raft RPC message on a bidirectional stream.
#[instrument(skip(recv, send, shard_cores))]
async fn handle_sharded_rpc_stream(
    (mut recv, mut send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    shard_cores: Arc<RwLock<HashMap<ShardId, Raft<AppTypeConfig>>>>,
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Read the RPC message with size limit
    let buffer = recv.read_to_end(MAX_RPC_MESSAGE_SIZE as usize).await.context("failed to read RPC message")?;

    // Extract shard ID from the prefix
    let shard_id = try_decode_shard_prefix(&buffer).ok_or_else(|| {
        anyhow::anyhow!("message too short: expected at least {} bytes, got {}", SHARD_PREFIX_SIZE, buffer.len())
    })?;

    debug!(shard_id, "routing sharded Raft RPC");

    // Get the Raft core for this shard
    let raft_core = {
        let cores = shard_cores.read();
        cores.get(&shard_id).cloned()
    };

    let raft_core = match raft_core {
        Some(core) => core,
        None => {
            warn!(shard_id, "received RPC for unknown shard");
            // Send error response with shard ID prefix
            let error_response = ShardNotFoundResponse { shard_id };
            let response_bytes = postcard::to_stdvec(&error_response).context("failed to serialize error response")?;
            let mut prefixed_response = Vec::with_capacity(SHARD_PREFIX_SIZE + response_bytes.len());
            prefixed_response.extend_from_slice(&encode_shard_prefix(shard_id));
            prefixed_response.extend_from_slice(&response_bytes);
            send.write_all(&prefixed_response).await.context("failed to write error response")?;
            send.finish().context("failed to finish error stream")?;
            return Ok(());
        }
    };

    // Deserialize the RPC request (skip shard prefix)
    let request: RaftRpcProtocol =
        postcard::from_bytes(&buffer[SHARD_PREFIX_SIZE..]).context("failed to deserialize RPC request")?;

    debug!(shard_id, request_type = ?request, "received sharded Raft RPC request");

    // Process the RPC and create response
    let response = match request {
        RaftRpcProtocol::Vote(vote_req) => {
            let result = match raft_core.vote(vote_req.request).await {
                Ok(result) => result,
                Err(err) => {
                    error!(shard_id, error = ?err, "vote RPC failed with fatal error");
                    return Err(anyhow::anyhow!("vote failed: {:?}", err));
                }
            };
            RaftRpcResponse::Vote(result)
        }
        RaftRpcProtocol::AppendEntries(append_req) => {
            let result = match raft_core.append_entries(append_req.request).await {
                Ok(result) => result,
                Err(err) => {
                    error!(shard_id, error = ?err, "append_entries RPC failed with fatal error");
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

    // Serialize response with shard ID prefix
    let response_bytes = postcard::to_stdvec(&response).context("failed to serialize RPC response")?;

    let mut prefixed_response = Vec::with_capacity(SHARD_PREFIX_SIZE + response_bytes.len());
    prefixed_response.extend_from_slice(&encode_shard_prefix(shard_id));
    prefixed_response.extend_from_slice(&response_bytes);

    debug!(shard_id, response_size = prefixed_response.len(), "sending sharded Raft RPC response");

    send.write_all(&prefixed_response).await.context("failed to write RPC response")?;
    send.finish().context("failed to finish send stream")?;

    debug!(shard_id, "sharded Raft RPC response sent successfully");

    Ok(())
}

/// Error response for unknown shards.
///
/// This is a minimal response sent when a shard ID is not registered.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ShardNotFoundResponse {
    shard_id: ShardId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raft::rpc::decode_shard_prefix;

    #[test]
    fn test_handler_creation() {
        let handler = ShardedRaftProtocolHandler::new();
        assert_eq!(handler.shard_count(), 0);
        assert!(handler.registered_shards().is_empty());
    }

    #[test]
    fn test_handler_default() {
        let handler = ShardedRaftProtocolHandler::default();
        assert_eq!(handler.shard_count(), 0);
    }

    #[test]
    fn test_has_shard_empty() {
        let handler = ShardedRaftProtocolHandler::new();
        assert!(!handler.has_shard(0));
        assert!(!handler.has_shard(1));
        assert!(!handler.has_shard(100));
    }

    #[test]
    fn test_shard_not_found_response_serde() {
        let response = ShardNotFoundResponse { shard_id: 42 };
        let bytes = postcard::to_stdvec(&response).unwrap();
        let decoded: ShardNotFoundResponse = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.shard_id, 42);
    }

    #[test]
    fn test_encode_decode_shard_prefix() {
        // Test roundtrip for various shard IDs
        for shard_id in [0, 1, 42, 255, 256, 1000, u32::MAX] {
            let encoded = encode_shard_prefix(shard_id);
            let decoded = decode_shard_prefix(&encoded);
            assert_eq!(decoded, shard_id);
        }
    }

    #[test]
    fn test_try_decode_too_short() {
        assert!(try_decode_shard_prefix(&[]).is_none());
        assert!(try_decode_shard_prefix(&[0]).is_none());
        assert!(try_decode_shard_prefix(&[0, 0]).is_none());
        assert!(try_decode_shard_prefix(&[0, 0, 0]).is_none());
    }

    #[test]
    fn test_try_decode_exact() {
        let result = try_decode_shard_prefix(&[0, 0, 0, 5]);
        assert_eq!(result, Some(5));
    }

    #[test]
    fn test_try_decode_with_extra_data() {
        let data = [0, 0, 0, 10, 1, 2, 3, 4, 5];
        let result = try_decode_shard_prefix(&data);
        assert_eq!(result, Some(10));
    }
}
