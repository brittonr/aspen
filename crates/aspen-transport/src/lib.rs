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

// Re-export commonly used types
pub use connection::{ConnectionManager, ConnectionPermit, StreamManager, StreamPermit, handle_connection_streams};
pub use constants::*;

// Re-export protocol handlers
pub use raft::RaftProtocolHandler;
pub use raft_authenticated::{AuthenticatedRaftProtocolHandler, TrustedPeersRegistry};
pub use raft_sharded::ShardedRaftProtocolHandler;
pub use log_subscriber::{LogSubscriberProtocolHandler, LOG_SUBSCRIBER_ALPN};