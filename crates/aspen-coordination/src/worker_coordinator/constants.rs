//! Constants for the distributed worker coordinator.

/// Maximum number of workers in the cluster.
pub(crate) const MAX_WORKERS: usize = 1024;

/// Maximum number of worker groups.
pub(crate) const MAX_GROUPS: usize = 64;

/// Maximum workers per group.
pub(crate) const MAX_WORKERS_PER_GROUP: usize = 32;

/// Maximum jobs to steal in one batch.
pub(crate) const MAX_STEAL_BATCH: usize = 10;

/// Steal hint TTL in milliseconds (30 seconds).
/// Hints expire if not consumed within this window.
pub(crate) const STEAL_HINT_TTL_MS: u64 = 30_000;

/// Maximum steal hints per worker to prevent unbounded accumulation.
pub(crate) const MAX_STEAL_HINTS_PER_WORKER: usize = 10;

/// Maximum hints to cleanup per sweep.
pub(crate) const MAX_HINT_CLEANUP_BATCH: usize = 100;
