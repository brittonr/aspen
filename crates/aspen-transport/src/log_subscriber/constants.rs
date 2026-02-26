//! Constants for the log subscriber protocol.

use std::time::Duration;

/// ALPN identifier for log subscription protocol.
pub const LOG_SUBSCRIBER_ALPN: &[u8] = b"aspen-logs";

/// Maximum number of concurrent log subscribers per node.
///
/// Tiger Style: Fixed upper bound on subscriber connections.
pub const MAX_LOG_SUBSCRIBERS: usize = 100;

/// Size of the broadcast channel buffer for log entries.
///
/// Subscribers that fall behind by more than this many entries
/// will experience lag (receive lagged error).
pub const LOG_BROADCAST_BUFFER_SIZE: usize = 1000;

/// Maximum size of a single log entry message (10 MB).
///
/// Matches MAX_RPC_MESSAGE_SIZE for consistency.
pub const MAX_LOG_ENTRY_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// Protocol version for log subscription protocol.
pub const LOG_SUBSCRIBE_PROTOCOL_VERSION: u8 = 1;

/// Maximum size for auth messages.
pub const MAX_AUTH_MESSAGE_SIZE: usize = 1024;

/// Timeout for authentication handshake.
pub const AUTH_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Timeout for subscription handshake.
pub const SUBSCRIBE_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Interval for keepalive messages on idle connections.
pub const SUBSCRIBE_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);

/// Maximum number of historical entries to fetch in a single batch.
pub const MAX_HISTORICAL_BATCH_SIZE: usize = 1000;

// ============================================================================
// Compile-Time Constant Assertions
// ============================================================================

// Subscriber limits must be positive
const _: () = assert!(MAX_LOG_SUBSCRIBERS > 0);
const _: () = assert!(LOG_BROADCAST_BUFFER_SIZE > 0);
const _: () = assert!(MAX_LOG_ENTRY_MESSAGE_SIZE > 0);
const _: () = assert!(MAX_AUTH_MESSAGE_SIZE > 0);
const _: () = assert!(MAX_HISTORICAL_BATCH_SIZE > 0);

// Protocol version must be positive
const _: () = assert!(LOG_SUBSCRIBE_PROTOCOL_VERSION > 0);
