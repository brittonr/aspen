//! Constants for the distributed worker coordinator.

/// Maximum number of workers in the cluster.
pub(crate) const MAX_WORKERS: u32 = 1024;

/// Maximum number of worker groups.
pub(crate) const MAX_GROUPS: u32 = 64;

/// Maximum workers per group.
pub(crate) const MAX_WORKERS_PER_GROUP: u32 = 32;

/// Maximum jobs to steal in one batch.
pub(crate) const MAX_STEAL_BATCH: u32 = 10;

/// Steal hint TTL in milliseconds (30 seconds).
/// Hints expire if not consumed within this window.
pub(crate) const STEAL_HINT_TTL_MS: u64 = 30_000;

/// Maximum steal hints per worker to prevent unbounded accumulation.
pub(crate) const MAX_STEAL_HINTS_PER_WORKER: u32 = 10;

/// Maximum hints to cleanup per sweep.
pub(crate) const MAX_HINT_CLEANUP_BATCH: u32 = 100;

// ============================================================================
// Compile-Time Constant Assertions
// ============================================================================

// Worker limits must be positive
const _: () = assert!(MAX_WORKERS > 0);
const _: () = assert!(MAX_GROUPS > 0);
const _: () = assert!(MAX_WORKERS_PER_GROUP > 0);

// Steal limits must be positive
const _: () = assert!(MAX_STEAL_BATCH > 0);
const _: () = assert!(STEAL_HINT_TTL_MS > 0);
const _: () = assert!(MAX_STEAL_HINTS_PER_WORKER > 0);
const _: () = assert!(MAX_HINT_CLEANUP_BATCH > 0);

// Workers per group should not exceed total workers (sanity check)
const _: () = assert!(MAX_WORKERS_PER_GROUP <= MAX_WORKERS);
