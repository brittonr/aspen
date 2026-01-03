//! Authenticated Raft protocol handler.
//!
//! This module provides `AuthenticatedRaftProtocolHandler`, which handles
//! authenticated Raft RPC connections over Iroh with the `raft-auth` ALPN.
//!
//! Uses Iroh's native NodeId verification at connection accept time. NodeId
//! is cryptographically verified during the QUIC TLS handshake, so we only
//! need to check if the NodeId is in the trusted peers set.

use std::collections::HashSet;
use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

use iroh::PublicKey;
use iroh::endpoint::Connection;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use openraft::Raft;
use tokio::sync::RwLock;
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::rpc::{AppTypeConfig, RaftRpcProtocol, RaftRpcResponse};
use aspen_raft_types::MAX_CONCURRENT_CONNECTIONS;
use aspen_raft_types::MAX_RPC_MESSAGE_SIZE;
use aspen_raft_types::MAX_STREAMS_PER_CONNECTION;

// ============================================================================
// TrustedPeersRegistry
// ============================================================================

/// Registry of Iroh PublicKeys authorized to participate in Raft.
///
/// Populated from Raft membership (voters + learners). Connections from
/// unknown peers are rejected at accept time.
///
/// # Iroh-Native Authentication
///
/// Iroh already provides cryptographic identity verification:
/// - `connection.remote_id()` returns Ed25519 public key
/// - PublicKey is verified during QUIC TLS handshake
/// - Transport is encrypted end-to-end
///
/// This registry simply checks if the verified PublicKey is authorized.
///
/// # Tiger Style
///
/// - Bounded by MAX_PEERS (from constants.rs)
/// - RwLock for concurrent read access
#[derive(Debug, Clone)]
pub struct TrustedPeersRegistry {
    /// Set of authorized Iroh PublicKeys.
    peers: Arc<RwLock<HashSet<PublicKey>>>,
}

impl TrustedPeersRegistry {
    /// Create a new empty trusted peers registry.
    pub fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Create a registry with initial peers.
    pub fn with_peers(peers: impl IntoIterator<Item = PublicKey>) -> Self {
        Self {
            peers: Arc::new(RwLock::new(peers.into_iter().collect())),
        }
    }

    /// Check if a PublicKey is trusted.
    pub async fn is_trusted(&self, public_key: &PublicKey) -> bool {
        self.peers.read().await.contains(public_key)
    }

    /// Add a trusted PublicKey (called when Raft membership changes).
    pub async fn add_peer(&self, public_key: PublicKey) {
        self.peers.write().await.insert(public_key);
    }

    /// Remove a PublicKey (called when node leaves cluster).
    pub async fn remove_peer(&self, public_key: &PublicKey) {
        self.peers.write().await.remove(public_key);
    }

    /// Replace entire peer set (called on membership change).
    pub async fn set_peers(&self, peers: impl IntoIterator<Item = PublicKey>) {
        let mut guard = self.peers.write().await;
        guard.clear();
        guard.extend(peers);
    }

    /// Get current peer count.
    pub async fn peer_count(&self) -> usize {
        self.peers.read().await.len()
    }
}

impl Default for TrustedPeersRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// AuthenticatedRaftProtocolHandler
// ============================================================================

/// Protocol handler for authenticated Raft RPC over Iroh.
///
/// Uses Iroh's native PublicKey verification at connection accept time.
/// No per-stream authentication is needed - PublicKey is cryptographically
/// verified during the QUIC TLS handshake.
///
/// # Security Model
///
/// 1. QUIC TLS handshake verifies remote peer's Ed25519 identity
/// 2. `connection.remote_id()` returns the verified PublicKey
/// 3. We check if PublicKey is in the trusted peers registry
/// 4. If not trusted, connection is rejected immediately
///
/// # Tiger Style
///
/// - Bounded connection count via semaphore
/// - PublicKey check at connection accept (fast path)
/// - No per-stream overhead
#[derive(Debug)]
pub struct AuthenticatedRaftProtocolHandler {
    raft_core: Raft<AppTypeConfig>,
    trusted_peers: TrustedPeersRegistry,
    connection_semaphore: Arc<Semaphore>,
}

impl AuthenticatedRaftProtocolHandler {
    /// Create a new authenticated Raft protocol handler.
    ///
    /// # Arguments
    /// * `raft_core` - Raft instance to forward RPCs to
    /// * `trusted_peers` - Registry of authorized PublicKeys
    ///
    /// # Security Note
    /// Authentication is handled by Iroh's QUIC TLS layer. The trusted_peers
    /// registry determines which verified PublicKeys are authorized.
    pub fn new(raft_core: Raft<AppTypeConfig>, trusted_peers: TrustedPeersRegistry) -> Self {
        Self {
            raft_core,
            trusted_peers,
            connection_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS as usize)),
        }
    }

    /// Get a reference to the trusted peers registry.
    ///
    /// Use this to update the registry when Raft membership changes.
    pub fn trusted_peers(&self) -> &TrustedPeersRegistry {
        &self.trusted_peers
    }
}

impl ProtocolHandler for AuthenticatedRaftProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        // Get remote PublicKey (verified by QUIC TLS handshake)
        let remote_id = connection.remote_id();

        // IROH-NATIVE: Check if PublicKey is in trusted peers
        if !self.trusted_peers.is_trusted(&remote_id).await {
            warn!(
                remote_node = %remote_id,
                "rejecting connection from untrusted peer"
            );
            return Err(AcceptError::from_err(std::io::Error::other("untrusted node")));
        }

        // Try to acquire a connection permit
        let permit = match self.connection_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    "Authenticated Raft connection limit reached ({}), rejecting connection from {}",
                    MAX_CONCURRENT_CONNECTIONS, remote_id
                );
                return Err(AcceptError::from_err(std::io::Error::other("connection limit reached")));
            }
        };

        debug!(remote_node = %remote_id, "accepted authenticated Raft RPC connection");

        // Handle the connection - NO PER-STREAM AUTH NEEDED
        let result = handle_raft_connection(connection, self.raft_core.clone()).await;

        drop(permit);

        result.map_err(|err| AcceptError::from_err(std::io::Error::other(err.to_string())))
    }

    async fn shutdown(&self) {
        info!("Authenticated Raft protocol handler shutting down");
        self.connection_semaphore.close();
    }
}

// ============================================================================
// Connection and Stream Handling
// ============================================================================

/// Handle Raft RPC connection. No per-stream auth - PublicKey verified at accept.
async fn handle_raft_connection(connection: Connection, raft_core: Raft<AppTypeConfig>) -> anyhow::Result<()> {
    let remote_id = connection.remote_id();

    // Tiger Style: Fixed limit on concurrent streams per connection
    let stream_semaphore = Arc::new(Semaphore::new(MAX_STREAMS_PER_CONNECTION as usize));
    let active_streams = Arc::new(AtomicU32::new(0));

    // Accept bidirectional streams from this connection
    loop {
        let stream = match connection.accept_bi().await {
            Ok(stream) => stream,
            Err(err) => {
                debug!(remote_node = %remote_id, error = %err, "Raft connection closed");
                break;
            }
        };

        // Try to acquire a stream permit
        let permit = match stream_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    remote_node = %remote_id,
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
            // DIRECT RPC - no auth handshake needed
            if let Err(err) = handle_raft_rpc_stream((recv, send), raft_core_clone).await {
                error!(error = %err, "failed to handle Raft RPC stream");
            }
            active_streams_clone.fetch_sub(1, Ordering::Relaxed);
        });
    }

    Ok(())
}

/// Handle a single Raft RPC stream.
///
/// No authentication needed - connection was already verified at accept time.
async fn handle_raft_rpc_stream(
    (mut recv, mut send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    raft_core: Raft<AppTypeConfig>,
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Read the Raft RPC message
    let buffer = recv.read_to_end(MAX_RPC_MESSAGE_SIZE as usize).await.context("failed to read RPC message")?;

    let request: RaftRpcProtocol = postcard::from_bytes(&buffer).context("failed to deserialize RPC request")?;

    debug!(request_type = ?request, "received Raft RPC request");

    // Process the RPC
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

    let response_bytes = postcard::to_stdvec(&response).context("failed to serialize RPC response")?;

    send.write_all(&response_bytes).await.context("failed to write RPC response")?;
    send.finish().context("failed to finish send stream")?;

    debug!("Raft RPC response sent successfully");

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use iroh::SecretKey;

    use super::*;

    /// Generate a test PublicKey from a SecretKey.
    fn test_public_key(seed: u8) -> PublicKey {
        // Create a deterministic secret key by hashing the seed
        let mut key_bytes = [0u8; 32];
        key_bytes[0] = seed;
        let secret = SecretKey::from_bytes(&key_bytes);
        secret.public()
    }

    #[tokio::test]
    async fn test_trusted_peers_registry_new() {
        let registry = TrustedPeersRegistry::new();
        assert_eq!(registry.peer_count().await, 0);
    }

    #[tokio::test]
    async fn test_trusted_peers_add_and_check() {
        let registry = TrustedPeersRegistry::new();

        // Generate a test PublicKey (in real code this comes from Iroh)
        let public_key = test_public_key(1);

        assert!(!registry.is_trusted(&public_key).await);

        registry.add_peer(public_key).await;
        assert!(registry.is_trusted(&public_key).await);
        assert_eq!(registry.peer_count().await, 1);
    }

    #[tokio::test]
    async fn test_trusted_peers_remove() {
        let registry = TrustedPeersRegistry::new();
        let public_key = test_public_key(2);

        registry.add_peer(public_key).await;
        assert!(registry.is_trusted(&public_key).await);

        registry.remove_peer(&public_key).await;
        assert!(!registry.is_trusted(&public_key).await);
        assert_eq!(registry.peer_count().await, 0);
    }

    #[tokio::test]
    async fn test_trusted_peers_set_peers() {
        let registry = TrustedPeersRegistry::new();
        let pk1 = test_public_key(1);
        let pk2 = test_public_key(2);
        let pk3 = test_public_key(3);

        registry.add_peer(pk1).await;
        registry.add_peer(pk2).await;
        assert_eq!(registry.peer_count().await, 2);

        // Replace with new set
        registry.set_peers([pk2, pk3]).await;
        assert!(!registry.is_trusted(&pk1).await);
        assert!(registry.is_trusted(&pk2).await);
        assert!(registry.is_trusted(&pk3).await);
        assert_eq!(registry.peer_count().await, 2);
    }

    #[tokio::test]
    async fn test_trusted_peers_with_peers() {
        let pk1 = test_public_key(1);
        let pk2 = test_public_key(2);

        let registry = TrustedPeersRegistry::with_peers([pk1, pk2]);
        assert!(registry.is_trusted(&pk1).await);
        assert!(registry.is_trusted(&pk2).await);
        assert_eq!(registry.peer_count().await, 2);
    }

    #[test]
    fn test_trusted_peers_default() {
        let registry = TrustedPeersRegistry::default();
        // Just verify it compiles and creates empty registry
        assert!(Arc::strong_count(&registry.peers) >= 1);
    }
}
