//! Constants for shard topology thresholds and timing.

const BYTES_PER_KIBIBYTE: u64 = 1024;
const KIBIBYTES_PER_MEBIBYTE: u64 = 1024;
const DEFAULT_SPLIT_SIZE_MEBIBYTES: u64 = 64;
const DEFAULT_MERGE_SIZE_MEBIBYTES: u64 = 16;
const DEFAULT_MERGE_MAX_COMBINED_MEBIBYTES: u64 = 48;
const SECONDS_PER_MINUTE: u64 = 60;
const SHARD_TOMBSTONE_GRACE_PERIOD_MINUTES: u64 = 5;

/// Default size threshold for triggering automatic splits (64 MB).
pub const DEFAULT_SPLIT_SIZE_BYTES: u64 = DEFAULT_SPLIT_SIZE_MEBIBYTES * KIBIBYTES_PER_MEBIBYTE * BYTES_PER_KIBIBYTE;

/// Default QPS threshold for triggering automatic splits.
pub const DEFAULT_SPLIT_QPS: u32 = 2500;

/// Default size threshold for allowing merges (16 MB).
/// Two shards can merge if both are below this threshold.
pub const DEFAULT_MERGE_SIZE_BYTES: u64 = DEFAULT_MERGE_SIZE_MEBIBYTES * KIBIBYTES_PER_MEBIBYTE * BYTES_PER_KIBIBYTE;

/// Maximum combined size after merge (48 MB).
/// Leaves headroom before next split trigger.
pub const DEFAULT_MERGE_MAX_COMBINED_BYTES: u64 =
    DEFAULT_MERGE_MAX_COMBINED_MEBIBYTES * KIBIBYTES_PER_MEBIBYTE * BYTES_PER_KIBIBYTE;

/// Grace period before cleaning up tombstoned shards (in seconds).
pub const SHARD_TOMBSTONE_GRACE_PERIOD_SECS: u64 = SHARD_TOMBSTONE_GRACE_PERIOD_MINUTES * SECONDS_PER_MINUTE;

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
