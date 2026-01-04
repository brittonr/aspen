//! Aspen Transport Layer
//!
//! Network transport layer for Aspen distributed systems providing:
//! - Protocol handlers for Raft consensus
//! - Connection and stream management with resource limits
//! - ALPN-based protocol routing
//! - Low-level networking primitives
//!
//! This crate handles all low-level network protocol routing and connection
//! management for Aspen cluster nodes.

pub mod connection;
pub mod constants;
pub mod log_subscriber;
pub mod raft;
pub mod raft_authenticated;
pub mod raft_sharded;
pub mod rpc;

// Re-export commonly used types
pub use connection::ConnectionManager;
pub use connection::ConnectionPermit;
pub use connection::StreamManager;
pub use connection::StreamPermit;
pub use connection::handle_connection_streams;
pub use constants::*;
// Re-export protocol handlers
pub use log_subscriber::{LOG_SUBSCRIBER_ALPN, LogSubscriberProtocolHandler};
pub use raft::RaftProtocolHandler;
pub use raft_authenticated::AuthenticatedRaftProtocolHandler;
pub use raft_authenticated::TrustedPeersRegistry;
pub use raft_sharded::ShardedRaftProtocolHandler;
