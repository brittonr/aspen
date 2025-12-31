//! Network transport abstraction layer.
//!
//! This module re-exports transport traits from `api::transport` for backward compatibility.
//! The traits are defined in `api` to avoid circular dependencies between `raft` and `cluster`.
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

// Re-export all transport types from api (which re-exports from aspen-core).
// This allows existing code using `crate::cluster::transport::*` to continue working.
pub use aspen_core::api::DiscoveredPeer;
pub use aspen_core::api::DiscoveryHandle;
pub use aspen_core::api::IrohTransportExt;
pub use aspen_core::api::NetworkTransport;
pub use aspen_core::api::PeerDiscoveredCallback;
pub use aspen_core::api::PeerDiscovery;
