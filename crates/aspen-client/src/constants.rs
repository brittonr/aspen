//! Client protocol constants.
//!
//! ALPN identifiers and size limits for client RPC.

/// ALPN protocol identifier for Client RPC.
///
/// Used to identify Aspen client connections over Iroh QUIC.
pub const CLIENT_ALPN: &[u8] = b"aspen-client";

/// Maximum Client RPC message size (1 MB).
///
/// Tiger Style: Bounded to prevent memory exhaustion attacks.
pub const MAX_CLIENT_MESSAGE_SIZE: usize = 1024 * 1024;

/// Maximum number of retry attempts for RPC calls.
pub const MAX_RETRIES: u32 = 3;

/// Delay between retry attempts in milliseconds.
pub const RETRY_DELAY_MS: u64 = 500;

/// Maximum number of bootstrap peers in a ticket.
///
/// Tiger Style: Fixed limit to prevent unbounded ticket size.
pub const MAX_BOOTSTRAP_PEERS: u32 = 16;
