//! Tiger Style resource bounds for the commit DAG.

/// KV prefix for serialized commit entries.
pub const COMMIT_KV_PREFIX: &str = "_sys:commit:";

/// KV prefix for branch head pointers (value = commit ID hex).
pub const COMMIT_TIP_PREFIX: &str = "_sys:commit-tip:";

/// KV prefix for federation provenance records.
pub const COMMIT_ORIGIN_PREFIX: &str = "_sys:commit-origin:";

/// Maximum number of keys in a commit's mutation snapshot.
/// Matches `MAX_BRANCH_DIRTY_KEYS` in `aspen-kv-branch`.
pub const MAX_COMMIT_SNAPSHOT_KEYS: u32 = 10_000;

/// Maximum commits per branch before oldest become GC-eligible regardless of TTL.
pub const MAX_COMMITS_PER_BRANCH: u32 = 10_000;

/// Time-to-live for commit entries (7 days in seconds).
pub const COMMIT_GC_TTL_SECONDS: u64 = 604_800;

/// Maximum commits to delete per GC run.
pub const COMMIT_GC_BATCH_SIZE: u32 = 1_000;

/// Interval between GC runs (1 hour in seconds).
pub const COMMIT_GC_INTERVAL_SECS: u64 = 3_600;

/// Maximum buffered entries waiting for commit metadata during federation import.
pub const MAX_COMMIT_IMPORT_BUFFER: u32 = 1_000;

/// Timeout for buffered entries waiting for commit metadata (30 seconds).
pub const COMMIT_IMPORT_BUFFER_TIMEOUT_SECS: u64 = 30;

/// Maximum commits accepted per peer per minute during federation import.
pub const MAX_COMMITS_PER_PEER_PER_MINUTE: u32 = 100;

/// Timeout for commit chain verification during federation import (5 seconds).
pub const COMMIT_VERIFY_TIMEOUT_MS: u64 = 5_000;
