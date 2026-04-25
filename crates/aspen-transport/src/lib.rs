//! Aspen Transport Layer
//!
//! Network transport layer for Aspen distributed systems providing:
//! - Protocol handlers for Raft consensus
//! - Connection and stream management with resource limits
//! - ALPN-based protocol routing
//! - Low-level networking primitives
//!
//! Default features expose the reusable Iroh connection helpers and protocol
//! identifiers. Aspen runtime integrations (OpenRaft handlers, auth, sharding,
//! trust, and log subscriber support) are available behind named features.

pub mod connection;
pub mod constants;
#[cfg(feature = "log-subscriber")]
pub mod log_subscriber;
#[cfg(feature = "raft-handler")]
pub mod raft;
#[cfg(feature = "auth-raft-handler")]
pub mod raft_authenticated;
#[cfg(feature = "sharded-raft-handler")]
pub mod raft_sharded;
#[cfg(feature = "raft-rpc")]
pub mod rpc;
pub mod snapshot_history;
#[cfg(feature = "trust-handler")]
pub mod trust;
pub mod verified;

// Re-export commonly used types.
pub use connection::ConnectionManager;
pub use connection::ConnectionPermit;
pub use connection::StreamManager;
pub use connection::StreamPermit;
pub use connection::handle_connection_streams;
pub use constants::*;
#[cfg(feature = "log-subscriber")]
pub use log_subscriber::LogSubscriberProtocolHandler;
#[cfg(feature = "raft-handler")]
pub use raft::RaftProtocolHandler;
#[cfg(feature = "auth-raft-handler")]
pub use raft_authenticated::AuthenticatedRaftProtocolHandler;
#[cfg(feature = "auth-raft-handler")]
pub use raft_authenticated::TrustedPeersRegistry;
#[cfg(feature = "sharded-raft-handler")]
pub use raft_sharded::ShardedRaftProtocolHandler;
pub use snapshot_history::SnapshotTransferEntry;
pub use snapshot_history::SnapshotTransferHistory;
#[cfg(feature = "trust-handler")]
pub use trust::TrustProtocolHandler;
#[cfg(feature = "trust-handler")]
pub use trust::TrustShareProvider;
