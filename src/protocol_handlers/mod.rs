//! Protocol handlers for Iroh Router-based ALPN dispatching.
//!
//! This module provides `ProtocolHandler` implementations for different protocols:
//! - `RaftProtocolHandler`: Handles Raft RPC connections (ALPN: `raft-rpc`)
//! - `AuthenticatedRaftProtocolHandler`: Handles authenticated Raft RPC (ALPN: `raft-auth`)
//! - `ShardedRaftProtocolHandler`: Handles sharded Raft RPC (ALPN: `raft-shard`)
//! - `LogSubscriberProtocolHandler`: Handles log subscription connections (ALPN: `aspen-logs`)
//! - `ClientProtocolHandler`: Handles client connections (ALPN: `aspen-client`)
//!
//! These handlers are registered with an Iroh Router to properly dispatch
//! incoming connections based on their ALPN, eliminating the race condition
//! that occurred when both servers were accepting from the same endpoint.
//!
//! # Architecture
//!
//! ```text
//! Iroh Endpoint
//!       |
//!       v
//!   Router (ALPN dispatch)
//!       |
//!       +---> raft-auth ALPN --> AuthenticatedRaftProtocolHandler (recommended)
//!       |
//!       +---> raft-shard ALPN -> ShardedRaftProtocolHandler (horizontal scaling)
//!       |
//!       +---> raft-rpc ALPN ---> RaftProtocolHandler (legacy, no auth)
//!       |
//!       +---> aspen-logs ALPN -> LogSubscriberProtocolHandler (read-only)
//!       |
//!       +---> aspen-client ALPN --> ClientProtocolHandler
//!       |
//!       +---> gossip ALPN -----> Gossip (via iroh-gossip)
//! ```
//!
//! # Module Organization
//!
//! ```text
//! protocol_handlers/
//! ├── mod.rs                 - This file: re-exports and documentation
//! ├── constants.rs           - ALPN identifiers and resource limits
//! ├── error_sanitization.rs  - Security-focused error message sanitization
//! ├── raft.rs                - RaftProtocolHandler (unauthenticated)
//! ├── raft_authenticated.rs  - AuthenticatedRaftProtocolHandler
//! ├── raft_sharded.rs        - ShardedRaftProtocolHandler (multi-shard)
//! ├── log_subscriber.rs      - LogSubscriberProtocolHandler
//! └── client.rs              - ClientProtocolHandler and context
//! ```
//!
//! # Authentication
//!
//! The `AuthenticatedRaftProtocolHandler` uses HMAC-SHA256 challenge-response
//! authentication based on the cluster cookie. This prevents unauthorized nodes
//! from participating in consensus.
//!
//! # Tiger Style
//!
//! - Bounded connection and stream limits per handler
//! - Explicit error handling with AcceptError
//! - Clean shutdown via ProtocolHandler::shutdown()

// Submodules
pub mod client;
pub mod constants;
pub mod error_sanitization;
pub mod log_subscriber;
pub mod raft;
pub mod raft_authenticated;
pub mod raft_sharded;

// Re-export constants
pub use constants::{
    CLIENT_ALPN, LOG_SUBSCRIBER_ALPN, RAFT_ALPN, RAFT_AUTH_ALPN, RAFT_SHARDED_ALPN,
};

// Re-export handlers
pub use client::{ClientProtocolContext, ClientProtocolHandler};
pub use log_subscriber::LogSubscriberProtocolHandler;
pub use raft::RaftProtocolHandler;
pub use raft_authenticated::{
    AuthenticatedRaftProtocolHandler, read_length_prefixed, write_length_prefixed,
};
pub use raft_sharded::ShardedRaftProtocolHandler;

// Re-export error sanitization functions
pub use error_sanitization::{
    sanitize_blob_error, sanitize_control_error, sanitize_error_for_client,
    sanitize_error_string_for_client, sanitize_kv_error,
};
