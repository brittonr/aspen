//! Network transport abstraction layer.
//!
//! This module provides trait-based abstractions over the P2P transport layer,
//! allowing different network implementations while maintaining a consistent interface.

use std::fmt::Debug;
use std::fmt::Display;
use std::sync::Arc;

use async_trait::async_trait;

use crate::types::NodeId;

/// A network transport for P2P communication.
///
/// This trait abstracts the core operations needed for distributed cluster
/// communication, including identity, connection management, and peer discovery.
#[async_trait]
pub trait NetworkTransport: Send + Sync + Debug {
    /// The underlying transport endpoint type.
    type Endpoint: Send + Sync + Clone;

    /// Network address type for identifying and connecting to peers.
    type Address: Send + Sync + Clone + Debug;

    /// Cryptographic secret key for signing operations.
    type SecretKey: Send + Sync + Clone;

    /// Gossip protocol service for peer discovery broadcasts.
    type Gossip: Send + Sync;

    /// Get the node's network address.
    fn node_addr(&self) -> &Self::Address;

    /// Get the node's public key identifier as a string.
    fn node_id_string(&self) -> String;

    /// Get the cryptographic secret key for signing operations.
    fn secret_key(&self) -> &Self::SecretKey;

    /// Get a reference to the underlying transport endpoint.
    fn endpoint(&self) -> &Self::Endpoint;

    /// Get the gossip service if available.
    fn gossip(&self) -> Option<&Arc<Self::Gossip>>;

    /// Check if gossip-based peer discovery is enabled.
    fn gossip_enabled(&self) -> bool {
        self.gossip().is_some()
    }

    /// Gracefully shut down the transport.
    async fn shutdown(&self) -> anyhow::Result<()>;
}

/// Extension trait for Iroh-specific transport operations.
pub trait IrohTransportExt: NetworkTransport {
    /// Get the Iroh protocol router if initialized.
    fn router(&self) -> Option<&iroh::protocol::Router>;

    /// Check if the protocol router has been initialized.
    fn router_initialized(&self) -> bool {
        self.router().is_some()
    }
}

/// A discovered peer announcement.
#[derive(Debug, Clone)]
pub struct DiscoveredPeer<A> {
    /// The logical node ID of the discovered peer.
    pub node_id: NodeId,
    /// The network address for connecting to the peer.
    pub address: A,
    /// When this announcement was created (microseconds since epoch).
    pub timestamp_micros: u64,
}

/// Handle for controlling a running peer discovery service.
pub struct DiscoveryHandle {
    cancel_token: tokio_util::sync::CancellationToken,
}

impl DiscoveryHandle {
    /// Create a new discovery handle with the given cancellation token.
    pub fn new(cancel_token: tokio_util::sync::CancellationToken) -> Self {
        Self { cancel_token }
    }

    /// Request graceful shutdown of the discovery service.
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// Check if shutdown has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    /// Get a clone of the cancellation token for sharing.
    pub fn cancellation_token(&self) -> tokio_util::sync::CancellationToken {
        self.cancel_token.clone()
    }
}

/// Callback type for handling discovered peers.
pub type PeerDiscoveredCallback<A> = Box<dyn Fn(DiscoveredPeer<A>) -> n0_future::boxed::BoxFuture<()> + Send + Sync>;

/// Information about a stale topology detection.
#[derive(Debug, Clone)]
pub struct StaleTopologyInfo {
    /// Node that announced the newer topology.
    pub announcing_node_id: u64,
    /// Remote topology version (higher than local).
    pub remote_version: u64,
    /// Remote topology hash for consistency checking.
    pub remote_hash: u64,
    /// Raft term when the remote topology was committed.
    pub remote_term: u64,
}

/// Callback type for handling stale topology detection.
///
/// Called when a gossip announcement indicates a topology version higher than local.
/// The callback should trigger a topology sync RPC to update local state.
pub type TopologyStaleCallback = Box<dyn Fn(StaleTopologyInfo) -> n0_future::boxed::BoxFuture<()> + Send + Sync>;

/// Information about a blob announced via gossip.
#[derive(Debug, Clone)]
pub struct BlobAnnouncedInfo {
    /// Node ID of the node that has this blob.
    pub announcing_node_id: u64,
    /// Public key of the node (for downloading).
    pub provider_public_key: iroh::PublicKey,
    /// BLAKE3 hash of the blob (hex-encoded).
    pub blob_hash_hex: String,
    /// Size of the blob in bytes.
    pub blob_size: u64,
    /// Whether this is a raw blob (true) or hash sequence (false).
    pub is_raw_format: bool,
    /// Optional tag for categorization (e.g., "kv-offload", "user-upload").
    pub tag: Option<String>,
}

/// Callback type for handling blob announcements from gossip.
///
/// Called when a peer announces that they have a blob available.
/// The callback can decide whether to download the blob for redundancy.
pub type BlobAnnouncedCallback = Box<dyn Fn(BlobAnnouncedInfo) -> n0_future::boxed::BoxFuture<()> + Send + Sync>;

/// Trait for peer discovery mechanisms.
#[async_trait]
pub trait PeerDiscovery: Send + Sync {
    /// The network address type for discovered peers.
    type Address: Send + Sync + Clone + Debug;

    /// The topic/channel identifier type for discovery.
    type TopicId: Send + Sync + Clone + Display;

    /// Get the discovery topic/channel identifier.
    fn topic_id(&self) -> &Self::TopicId;

    /// Start the peer discovery service.
    async fn start(
        &self,
        on_peer_discovered: Option<PeerDiscoveredCallback<Self::Address>>,
    ) -> anyhow::Result<DiscoveryHandle>;

    /// Announce this node's presence immediately.
    async fn announce(&self) -> anyhow::Result<()>;

    /// Check if the discovery service is currently running.
    fn is_running(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // DiscoveredPeer Tests
    // ========================================================================

    #[test]
    fn discovered_peer_construction() {
        let peer = DiscoveredPeer {
            node_id: NodeId(42),
            address: "192.168.1.1:8080".to_string(),
            timestamp_micros: 1234567890,
        };
        assert_eq!(peer.node_id, NodeId(42));
        assert_eq!(peer.address, "192.168.1.1:8080");
        assert_eq!(peer.timestamp_micros, 1234567890);
    }

    #[test]
    fn discovered_peer_debug() {
        let peer = DiscoveredPeer {
            node_id: NodeId(1),
            address: "addr",
            timestamp_micros: 100,
        };
        let debug = format!("{:?}", peer);
        assert!(debug.contains("DiscoveredPeer"));
        assert!(debug.contains("node_id"));
        assert!(debug.contains("100"));
    }

    #[test]
    fn discovered_peer_clone() {
        let peer = DiscoveredPeer {
            node_id: NodeId(5),
            address: "test-addr".to_string(),
            timestamp_micros: 999,
        };
        let peer2 = peer.clone();
        assert_eq!(peer.node_id, peer2.node_id);
        assert_eq!(peer.address, peer2.address);
        assert_eq!(peer.timestamp_micros, peer2.timestamp_micros);
    }

    #[test]
    fn discovered_peer_generic_address_types() {
        // Test with different address types
        let _peer_string: DiscoveredPeer<String> = DiscoveredPeer {
            node_id: NodeId(1),
            address: "string-addr".to_string(),
            timestamp_micros: 0,
        };

        let _peer_u64: DiscoveredPeer<u64> = DiscoveredPeer {
            node_id: NodeId(1),
            address: 12345,
            timestamp_micros: 0,
        };

        let _peer_tuple: DiscoveredPeer<(String, u16)> = DiscoveredPeer {
            node_id: NodeId(1),
            address: ("localhost".to_string(), 8080),
            timestamp_micros: 0,
        };
    }

    // ========================================================================
    // DiscoveryHandle Tests
    // ========================================================================

    #[test]
    fn discovery_handle_new() {
        let token = tokio_util::sync::CancellationToken::new();
        let handle = DiscoveryHandle::new(token);
        assert!(!handle.is_cancelled());
    }

    #[test]
    fn discovery_handle_cancel() {
        let token = tokio_util::sync::CancellationToken::new();
        let handle = DiscoveryHandle::new(token);

        assert!(!handle.is_cancelled());
        handle.cancel();
        assert!(handle.is_cancelled());
    }

    #[test]
    fn discovery_handle_cancellation_token_shared() {
        let token = tokio_util::sync::CancellationToken::new();
        let handle = DiscoveryHandle::new(token.clone());

        // Get a clone of the token from the handle
        let cloned_token = handle.cancellation_token();

        // Cancel via the handle
        handle.cancel();

        // Both should show cancelled
        assert!(handle.is_cancelled());
        assert!(cloned_token.is_cancelled());
        assert!(token.is_cancelled());
    }

    #[test]
    fn discovery_handle_external_cancel() {
        let token = tokio_util::sync::CancellationToken::new();
        let handle = DiscoveryHandle::new(token.clone());

        // Cancel via the original token
        token.cancel();

        // Handle should reflect the cancellation
        assert!(handle.is_cancelled());
    }

    #[test]
    fn discovery_handle_multiple_cancel_calls_idempotent() {
        let token = tokio_util::sync::CancellationToken::new();
        let handle = DiscoveryHandle::new(token);

        handle.cancel();
        assert!(handle.is_cancelled());

        // Multiple cancel calls should be safe
        handle.cancel();
        handle.cancel();
        assert!(handle.is_cancelled());
    }

    // ========================================================================
    // StaleTopologyInfo Tests
    // ========================================================================

    #[test]
    fn stale_topology_info_construction() {
        let info = StaleTopologyInfo {
            announcing_node_id: 1,
            remote_version: 42,
            remote_hash: 0xDEADBEEF,
            remote_term: 5,
        };
        assert_eq!(info.announcing_node_id, 1);
        assert_eq!(info.remote_version, 42);
        assert_eq!(info.remote_hash, 0xDEADBEEF);
        assert_eq!(info.remote_term, 5);
    }

    #[test]
    fn stale_topology_info_debug() {
        let info = StaleTopologyInfo {
            announcing_node_id: 99,
            remote_version: 10,
            remote_hash: 12345,
            remote_term: 3,
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("StaleTopologyInfo"));
        assert!(debug.contains("99"));
        assert!(debug.contains("10"));
        assert!(debug.contains("12345"));
        assert!(debug.contains("3"));
    }

    #[test]
    fn stale_topology_info_clone() {
        let info = StaleTopologyInfo {
            announcing_node_id: 7,
            remote_version: 100,
            remote_hash: 999,
            remote_term: 2,
        };
        let info2 = info.clone();
        assert_eq!(info.announcing_node_id, info2.announcing_node_id);
        assert_eq!(info.remote_version, info2.remote_version);
        assert_eq!(info.remote_hash, info2.remote_hash);
        assert_eq!(info.remote_term, info2.remote_term);
    }

    // ========================================================================
    // BlobAnnouncedInfo Tests
    // ========================================================================

    /// Helper to generate a valid public key for tests
    fn test_public_key() -> iroh::PublicKey {
        iroh::SecretKey::generate(&mut rand::thread_rng()).public()
    }

    #[test]
    fn blob_announced_info_construction() {
        let public_key = test_public_key();
        let info = BlobAnnouncedInfo {
            announcing_node_id: 1,
            provider_public_key: public_key,
            blob_hash_hex: "abc123".to_string(),
            blob_size: 1024,
            is_raw_format: true,
            tag: Some("test-tag".to_string()),
        };
        assert_eq!(info.announcing_node_id, 1);
        assert_eq!(info.blob_hash_hex, "abc123");
        assert_eq!(info.blob_size, 1024);
        assert!(info.is_raw_format);
        assert_eq!(info.tag, Some("test-tag".to_string()));
    }

    #[test]
    fn blob_announced_info_no_tag() {
        let public_key = test_public_key();
        let info = BlobAnnouncedInfo {
            announcing_node_id: 2,
            provider_public_key: public_key,
            blob_hash_hex: "def456".to_string(),
            blob_size: 2048,
            is_raw_format: false,
            tag: None,
        };
        assert_eq!(info.tag, None);
        assert!(!info.is_raw_format);
    }

    #[test]
    fn blob_announced_info_debug() {
        let public_key = test_public_key();
        let info = BlobAnnouncedInfo {
            announcing_node_id: 5,
            provider_public_key: public_key,
            blob_hash_hex: "hash123".to_string(),
            blob_size: 4096,
            is_raw_format: true,
            tag: Some("debug-tag".to_string()),
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("BlobAnnouncedInfo"));
        assert!(debug.contains("hash123"));
        assert!(debug.contains("4096"));
        assert!(debug.contains("debug-tag"));
    }

    #[test]
    fn blob_announced_info_clone() {
        let public_key = test_public_key();
        let info = BlobAnnouncedInfo {
            announcing_node_id: 10,
            provider_public_key: public_key,
            blob_hash_hex: "original".to_string(),
            blob_size: 8192,
            is_raw_format: false,
            tag: Some("clone-test".to_string()),
        };
        let info2 = info.clone();
        assert_eq!(info.announcing_node_id, info2.announcing_node_id);
        assert_eq!(info.blob_hash_hex, info2.blob_hash_hex);
        assert_eq!(info.blob_size, info2.blob_size);
        assert_eq!(info.is_raw_format, info2.is_raw_format);
        assert_eq!(info.tag, info2.tag);
    }

    #[test]
    fn blob_announced_info_various_sizes() {
        let public_key = test_public_key();

        // Zero size
        let info = BlobAnnouncedInfo {
            announcing_node_id: 1,
            provider_public_key: public_key.clone(),
            blob_hash_hex: "empty".to_string(),
            blob_size: 0,
            is_raw_format: true,
            tag: None,
        };
        assert_eq!(info.blob_size, 0);

        // Large size (1 TB)
        let info = BlobAnnouncedInfo {
            announcing_node_id: 1,
            provider_public_key: public_key,
            blob_hash_hex: "large".to_string(),
            blob_size: 1_000_000_000_000,
            is_raw_format: true,
            tag: None,
        };
        assert_eq!(info.blob_size, 1_000_000_000_000);
    }

    // ========================================================================
    // NodeId Integration Tests
    // ========================================================================

    #[test]
    fn node_id_in_discovered_peer() {
        let peer = DiscoveredPeer {
            node_id: NodeId(u64::MAX),
            address: "max-node".to_string(),
            timestamp_micros: 0,
        };
        assert_eq!(peer.node_id.0, u64::MAX);
    }

    #[test]
    fn node_id_zero() {
        let peer = DiscoveredPeer {
            node_id: NodeId(0),
            address: "zero-node".to_string(),
            timestamp_micros: 0,
        };
        assert_eq!(peer.node_id.0, 0);
    }
}
