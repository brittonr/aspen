//! Protocol handler constants.
//!
//! ALPN identifiers and resource limits for protocol handlers.

/// ALPN protocol identifier for Raft RPC (legacy, no authentication).
pub const RAFT_ALPN: &[u8] = b"raft-rpc";

/// ALPN protocol identifier for authenticated Raft RPC.
///
/// Uses HMAC-SHA256 challenge-response authentication based on the cluster cookie.
/// This is the recommended ALPN for production deployments.
pub const RAFT_AUTH_ALPN: &[u8] = b"raft-auth";

/// Re-export LOG_SUBSCRIBER_ALPN for convenience.
pub use crate::raft::log_subscriber::LOG_SUBSCRIBER_ALPN;

/// ALPN protocol identifier for Client RPC.
pub const CLIENT_ALPN: &[u8] = b"aspen-client";

/// Maximum concurrent Client connections.
///
/// Tiger Style: Lower limit than Raft since client connections are less critical.
pub const MAX_CLIENT_CONNECTIONS: u32 = 50;

/// Maximum concurrent streams per Client connection.
pub const MAX_CLIENT_STREAMS_PER_CONNECTION: u32 = 10;
