//! Constants for gossip peer discovery and rate limiting.

use std::time::Duration;

/// Maximum number of peers to track in rate limiter.
///
/// Uses bounded LRU cache to prevent memory growth.
/// Tiger Style: Explicit bound on memory consumption (~16KB).
pub const GOSSIP_MAX_TRACKED_PEERS: usize = 256;

/// Per-peer gossip message rate limit (messages per minute).
///
/// Prevents individual peers from flooding the gossip network.
/// Tiger Style: Conservative default that allows reasonable announcement frequency.
pub const GOSSIP_PER_PEER_RATE_PER_MINUTE: u32 = 12;

/// Per-peer burst capacity for rate limiting.
///
/// Allows short bursts of messages before rate limiting kicks in.
/// Tiger Style: Small burst to handle normal announcement retries.
pub const GOSSIP_PER_PEER_BURST: u32 = 3;

/// Global gossip message rate limit (messages per minute).
///
/// Prevents the entire cluster from being overwhelmed by gossip traffic.
/// Tiger Style: High limit that should accommodate large clusters.
pub const GOSSIP_GLOBAL_RATE_PER_MINUTE: u32 = 10_000;

/// Global burst capacity for rate limiting.
///
/// Allows brief spikes in cluster-wide gossip activity.
/// Tiger Style: Moderate burst to handle cluster initialization.
pub const GOSSIP_GLOBAL_BURST: u32 = 100;

/// Maximum stream retries for gossip receiver.
///
/// Controls how many times to retry after stream errors.
/// Tiger Style: Limited retries with exponential backoff.
pub const GOSSIP_MAX_STREAM_RETRIES: u32 = 5;

/// Backoff durations for gossip stream retry (in seconds).
///
/// Exponential backoff sequence for stream recovery.
/// Tiger Style: Bounded backoff prevents unbounded delays.
pub const GOSSIP_STREAM_BACKOFF_SECS: [u64; 5] = [1, 2, 4, 8, 16];

/// Minimum announcement interval (seconds).
///
/// Normal announcement frequency during healthy operation.
/// Tiger Style: Conservative interval to reduce network overhead.
pub const GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS: u64 = 10;

/// Maximum announcement interval (seconds).
///
/// Fallback interval during persistent failures.
/// Tiger Style: Bounded maximum delay for discovery recovery.
pub const GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS: u64 = 60;

/// Failure threshold before increasing announcement interval.
///
/// Number of consecutive failures before backing off.
/// Tiger Style: Quick adaptation to network issues.
pub const GOSSIP_ANNOUNCE_FAILURE_THRESHOLD: u32 = 3;

/// Timeout for gossip topic subscription.
///
/// Prevents indefinite blocking during gossip initialization.
/// Tiger Style: Explicit timeout prevents deadlocks.
pub const GOSSIP_SUBSCRIBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Current gossip message protocol version.
///
/// Version history:
/// - v1: Initial version (unsigned)
/// - v2: Added Ed25519 signatures for message authentication
///
/// Note: This is a breaking change - old nodes will not parse new messages.
pub const GOSSIP_MESSAGE_VERSION: u8 = 2;
