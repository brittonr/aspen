//! Ephemeral pub/sub — fire-and-forget event delivery without Raft consensus.
//!
//! Events are delivered directly to connected subscribers over in-memory channels.
//! No persistence, no replication, no ordering beyond channel FIFO.
//! Use for real-time streaming where sub-millisecond latency matters more than durability.

pub mod broker;
pub mod handler;
pub mod publisher;
pub mod wire;

pub use broker::EphemeralBroker;
pub use handler::EPHEMERAL_ALPN;
pub use handler::EphemeralProtocolHandler;
pub use publisher::EphemeralPublisher;

/// Maximum concurrent subscriptions per broker instance.
pub const MAX_EPHEMERAL_SUBSCRIPTIONS: usize = 1024;

/// Default buffer size for ephemeral subscription channels.
///
/// When a subscriber's buffer is full, new events are dropped (not blocked).
pub const DEFAULT_EPHEMERAL_BUFFER_SIZE: usize = 256;
