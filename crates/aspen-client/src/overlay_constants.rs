//! Constants for client overlay system.

use std::time::Duration;

/// Maximum number of cluster subscriptions per client.
pub const MAX_SUBSCRIPTIONS: usize = 16;

/// Maximum number of total cached entries across all subscriptions.
pub const MAX_TOTAL_CACHE_ENTRIES: usize = 1_000_000;

/// Default cache entry TTL.
pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(60);

/// Maximum cache entry TTL.
pub const MAX_CACHE_TTL: Duration = Duration::from_secs(3600);

/// Connection timeout for cluster connections.
pub const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

/// Reconnection delay after connection failure.
pub const RECONNECT_DELAY: Duration = Duration::from_secs(5);

/// Maximum reconnection delay (exponential backoff cap).
pub const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(60);

/// Heartbeat interval for checking connection health.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
