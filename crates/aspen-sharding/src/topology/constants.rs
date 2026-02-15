//! Constants for shard topology thresholds and timing.

/// Default size threshold for triggering automatic splits (64 MB).
pub const DEFAULT_SPLIT_SIZE_BYTES: u64 = 64 * 1024 * 1024;

/// Default QPS threshold for triggering automatic splits.
pub const DEFAULT_SPLIT_QPS: u32 = 2500;

/// Default size threshold for allowing merges (16 MB).
/// Two shards can merge if both are below this threshold.
pub const DEFAULT_MERGE_SIZE_BYTES: u64 = 16 * 1024 * 1024;

/// Maximum combined size after merge (48 MB).
/// Leaves headroom before next split trigger.
pub const DEFAULT_MERGE_MAX_COMBINED_BYTES: u64 = 48 * 1024 * 1024;

/// Grace period before cleaning up tombstoned shards (in seconds).
pub const SHARD_TOMBSTONE_GRACE_PERIOD_SECS: u64 = 300;
