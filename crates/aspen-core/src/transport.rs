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
pub type PeerDiscoveredCallback<A> =
    Box<dyn Fn(DiscoveredPeer<A>) -> futures::future::BoxFuture<'static, ()> + Send + Sync>;

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
pub type TopologyStaleCallback =
    Box<dyn Fn(StaleTopologyInfo) -> futures::future::BoxFuture<'static, ()> + Send + Sync>;

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
pub type BlobAnnouncedCallback =
    Box<dyn Fn(BlobAnnouncedInfo) -> futures::future::BoxFuture<'static, ()> + Send + Sync>;

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
