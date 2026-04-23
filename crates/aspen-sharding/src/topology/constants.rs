//! Constants for shard topology thresholds and timing.

/// Default size threshold for triggering automatic splits (64 MiB).
pub const DEFAULT_SPLIT_SIZE_BYTES: u64 = 67_108_864;

/// Default QPS threshold for triggering automatic splits.
pub const DEFAULT_SPLIT_QPS: u32 = 2500;

/// Default size threshold for allowing merges (16 MiB).
/// Two shards can merge if both are below this threshold.
pub const DEFAULT_MERGE_SIZE_BYTES: u64 = 16_777_216;

/// Maximum combined size after merge (48 MiB).
/// Leaves headroom before next split trigger.
pub const DEFAULT_MERGE_MAX_COMBINED_BYTES: u64 = 50_331_648;

/// Grace period before cleaning up tombstoned shards (in seconds).
pub const SHARD_TOMBSTONE_GRACE_PERIOD_SECS: u64 = 300;

// ============================================================================
// Compile-Time Constant Assertions
// ============================================================================

// Split thresholds must be positive
const _: () = assert!(DEFAULT_SPLIT_SIZE_BYTES > 0);
const _: () = assert!(DEFAULT_SPLIT_QPS > 0);

// Merge thresholds must be positive and ordered
const _: () = assert!(DEFAULT_MERGE_SIZE_BYTES > 0);
const _: () = assert!(DEFAULT_MERGE_MAX_COMBINED_BYTES > 0);
const _: () = assert!(DEFAULT_MERGE_SIZE_BYTES < DEFAULT_SPLIT_SIZE_BYTES);
const _: () = assert!(DEFAULT_MERGE_MAX_COMBINED_BYTES < DEFAULT_SPLIT_SIZE_BYTES);

// Tombstone grace period must be positive
const _: () = assert!(SHARD_TOMBSTONE_GRACE_PERIOD_SECS > 0);
