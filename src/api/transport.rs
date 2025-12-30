//! Network transport abstraction layer.
//!
//! This module provides trait-based abstractions over the P2P transport layer,
//! allowing different network implementations while maintaining a consistent interface.
//! Located in `api` module to avoid circular dependencies between `raft` and `cluster`.
//!
//! # Design Philosophy
//!
//! The `NetworkTransport` trait abstracts the core transport operations while
//! acknowledging that some types (like cryptographic keys and addresses) are
//! inherently tied to the underlying implementation. Rather than creating
//! leaky abstractions, we:
//!
//! 1. Use associated types for implementation-specific types
//! 2. Provide common operations through trait methods
//! 3. Allow access to the underlying implementation for advanced use cases
//!
//! # Current Implementation
//!
//! The primary implementation is `IrohEndpointManager` which provides:
//! - QUIC-based P2P connections via Iroh
//! - NAT traversal and relay support
//! - ALPN-based protocol routing
//! - Gossip-based peer discovery
//!
//! # Future Extensibility
//!
//! This trait enables future support for alternative transports (e.g., libp2p)
//! without requiring changes to code that depends on the trait interface.

use std::fmt::Debug;
use std::fmt::Display;
use std::sync::Arc;

use async_trait::async_trait;

use super::NodeId;

/// A network transport for P2P communication.
///
/// This trait abstracts the core operations needed for distributed cluster
/// communication, including identity, connection management, and peer discovery.
///
/// # Associated Types
///
/// - `Endpoint`: The underlying transport endpoint (e.g., `iroh::Endpoint`)
/// - `Address`: Network address type for peers (e.g., `iroh::EndpointAddr`)
/// - `SecretKey`: Cryptographic key for signing (e.g., `iroh::SecretKey`)
/// - `Gossip`: Gossip protocol service (e.g., `iroh_gossip::net::Gossip`)
///
/// # Example
///
/// ```ignore
/// use aspen::api::transport::NetworkTransport;
///
/// async fn connect_to_peer<T: NetworkTransport>(
///     transport: &T,
///     peer_addr: &T::Address,
/// ) -> Result<(), anyhow::Error> {
///     // Use the transport's endpoint to connect
///     let endpoint = transport.endpoint();
///     // ... establish connection
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait NetworkTransport: Send + Sync + Debug {
    /// The underlying transport endpoint type.
    ///
    /// This provides raw access to the P2P endpoint for establishing connections.
    type Endpoint: Send + Sync + Clone;

    /// Network address type for identifying and connecting to peers.
    type Address: Send + Sync + Clone + Debug;

    /// Cryptographic secret key for signing operations.
    type SecretKey: Send + Sync + Clone;

    /// Gossip protocol service for peer discovery broadcasts.
    type Gossip: Send + Sync;

    // =========================================================================
    // Identity
    // =========================================================================

    /// Get the node's network address.
    ///
    /// This address can be shared with other nodes to enable them to connect.
    fn node_addr(&self) -> &Self::Address;

    /// Get the node's public key identifier as a string.
    ///
    /// This is a human-readable identifier derived from the cryptographic key.
    fn node_id_string(&self) -> String;

    /// Get the cryptographic secret key for signing operations.
    ///
    /// Used for signing gossip messages, peer announcements, and other
    /// cryptographic operations.
    fn secret_key(&self) -> &Self::SecretKey;

    // =========================================================================
    // Connection
    // =========================================================================

    /// Get a reference to the underlying transport endpoint.
    ///
    /// This provides access to the raw P2P endpoint for establishing connections,
    /// creating streams, and other transport-level operations.
    fn endpoint(&self) -> &Self::Endpoint;

    // =========================================================================
    // Discovery
    // =========================================================================

    /// Get the gossip service if available.
    ///
    /// Returns `None` if gossip-based peer discovery is disabled.
    /// When available, this can be used to subscribe to topics and broadcast
    /// peer announcements.
    fn gossip(&self) -> Option<&Arc<Self::Gossip>>;

    /// Check if gossip-based peer discovery is enabled.
    fn gossip_enabled(&self) -> bool {
        self.gossip().is_some()
    }

    // =========================================================================
    // Lifecycle
    // =========================================================================

    /// Gracefully shut down the transport.
    ///
    /// This should:
    /// 1. Stop accepting new connections
    /// 2. Close existing connections gracefully
    /// 3. Release any held resources
    async fn shutdown(&self) -> anyhow::Result<()>;
}

/// Extension trait for Iroh-specific transport operations.
///
/// This trait provides access to Iroh-specific functionality that cannot be
/// abstracted without losing important capabilities. Code that needs these
/// features can require this trait in addition to `NetworkTransport`.
///
/// # When to Use
///
/// Use this trait when you need:
/// - Access to the Iroh Router for protocol handler registration
/// - Iroh-specific configuration or features
/// - Direct access to iroh-gossip types
///
/// For code that only needs basic transport operations, prefer using
/// `NetworkTransport` alone to maintain abstraction.
pub trait IrohTransportExt: NetworkTransport {
    /// Get the Iroh protocol router if initialized.
    ///
    /// Returns `None` if `spawn_router` hasn't been called yet.
    fn router(&self) -> Option<&iroh::protocol::Router>;

    /// Check if the protocol router has been initialized.
    fn router_initialized(&self) -> bool {
        self.router().is_some()
    }
}

// ============================================================================
// Peer Discovery Abstraction
// ============================================================================

/// A discovered peer announcement.
///
/// This represents information about a peer that was discovered through
/// the peer discovery mechanism. It contains enough information to
/// establish a connection to the peer.
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
///
/// This handle allows shutting down the discovery service gracefully.
/// When dropped, the discovery service will be aborted.
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
///
/// This is called whenever a new peer is discovered through the discovery
/// mechanism. The implementation can choose how to handle the peer
/// (e.g., add to network factory, update routing table, etc.).
pub type PeerDiscoveredCallback<A> =
    Box<dyn Fn(DiscoveredPeer<A>) -> futures::future::BoxFuture<'static, ()> + Send + Sync>;

/// Trait for peer discovery mechanisms.
///
/// This trait abstracts different peer discovery strategies, allowing
/// the same interface to be used for gossip-based discovery, mDNS,
/// DNS discovery, DHT, or manual configuration.
///
/// # Design Notes
///
/// The trait intentionally separates discovery (transport layer) from
/// cluster membership (application layer). Discovered peers are NOT
/// automatically added to cluster membership - that requires explicit
/// `add_learner()` calls. This separation:
///
/// 1. Prevents Sybil attacks (random nodes can't join cluster)
/// 2. Allows bounded cluster growth
/// 3. Keeps transport concerns separate from consensus
///
/// # Example
///
/// ```ignore
/// use aspen::api::transport::{PeerDiscovery, DiscoveredPeer};
///
/// async fn start_discovery<D: PeerDiscovery>(
///     discovery: &D,
///     on_peer: impl Fn(DiscoveredPeer<D::Address>) + Send + Sync + 'static,
/// ) -> Result<DiscoveryHandle, anyhow::Error> {
///     discovery.start(Box::new(move |peer| {
///         on_peer(peer);
///         Box::pin(async {})
///     })).await
/// }
/// ```
#[async_trait]
pub trait PeerDiscovery: Send + Sync {
    /// The network address type for discovered peers.
    type Address: Send + Sync + Clone + Debug;

    /// The topic/channel identifier type for discovery.
    type TopicId: Send + Sync + Clone + Display;

    /// Get the discovery topic/channel identifier.
    ///
    /// This identifies which discovery channel this service is operating on.
    /// For gossip-based discovery, this is the gossip topic ID.
    fn topic_id(&self) -> &Self::TopicId;

    /// Start the peer discovery service.
    ///
    /// This spawns background tasks that:
    /// 1. Periodically announce this node's presence
    /// 2. Listen for announcements from other nodes
    /// 3. Call the callback for each discovered peer
    ///
    /// Returns a handle that can be used to shut down the service.
    async fn start(
        &self,
        on_peer_discovered: Option<PeerDiscoveredCallback<Self::Address>>,
    ) -> anyhow::Result<DiscoveryHandle>;

    /// Announce this node's presence immediately.
    ///
    /// This is useful for forcing an announcement after a significant
    /// event (e.g., address change, rejoin after network partition).
    async fn announce(&self) -> anyhow::Result<()>;

    /// Check if the discovery service is currently running.
    fn is_running(&self) -> bool;
}
