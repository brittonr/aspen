//! Protocol handler constants.
//!
//! ALPN identifiers and resource limits for protocol handlers.

/// ALPN protocol identifier for Raft RPC (legacy, no authentication).
///
/// # Security Warning
///
/// This ALPN provides **no authentication**. Any node that knows the endpoint
/// address can connect and participate in Raft consensus. This allows:
/// - Unauthorized nodes to join the cluster
/// - Potential data exfiltration via Raft log replication
/// - Cluster disruption by malicious nodes
///
/// **Use `RAFT_AUTH_ALPN` for production deployments.**
///
/// This constant is retained for backward compatibility during migration.
/// New deployments should always use authenticated Raft (`raft-auth` ALPN).
#[deprecated(
    since = "0.2.0",
    note = "Use RAFT_AUTH_ALPN for production deployments. RAFT_ALPN provides no authentication."
)]
pub const RAFT_ALPN: &[u8] = b"raft-rpc";

/// ALPN protocol identifier for authenticated Raft RPC.
///
/// Uses HMAC-SHA256 challenge-response authentication based on the cluster cookie.
/// This is the recommended ALPN for production deployments.
pub const RAFT_AUTH_ALPN: &[u8] = b"raft-auth";

/// ALPN protocol identifier for sharded Raft RPC.
///
/// Handles RPC for multiple Raft shards over a single connection. Each message
/// is prefixed with a 4-byte big-endian shard ID that routes to the appropriate
/// Raft core. This enables horizontal scaling to hundreds of shards without
/// requiring per-shard ALPN registration.
pub const RAFT_SHARDED_ALPN: &[u8] = b"raft-shard";

/// ALPN protocol identifier for Client RPC.
/// Re-exported from aspen-client-api as the canonical source.
pub use aspen_client_api::CLIENT_ALPN;

/// Re-export LOG_SUBSCRIBER_ALPN for convenience.
pub use crate::log_subscriber::LOG_SUBSCRIBER_ALPN;

/// ALPN protocol identifier for Nix binary cache HTTP/3 gateway.
///
/// Uses the standard h3-iroh ALPN to enable HTTP/3 semantics over Iroh QUIC.
/// This allows standard HTTP/3 clients to connect while satisfying Aspen's
/// "Iroh-only networking" architecture constraint.
pub const NIX_CACHE_H3_ALPN: &[u8] = b"iroh+h3";

/// Maximum concurrent Client connections.
///
/// Set high enough to handle bursts of short-lived CLI connections.
/// Each CLI invocation creates a new QUIC connection that persists briefly
/// on the server after the client exits.
pub const MAX_CLIENT_CONNECTIONS: u32 = 200;

/// Maximum concurrent streams per Client connection.
pub const MAX_CLIENT_STREAMS_PER_CONNECTION: u32 = 10;

// ============================================================================
// Compile-Time Constant Assertions
// ============================================================================

// Connection limits must be positive
const _: () = assert!(MAX_CLIENT_CONNECTIONS > 0);
const _: () = assert!(MAX_CLIENT_STREAMS_PER_CONNECTION > 0);
