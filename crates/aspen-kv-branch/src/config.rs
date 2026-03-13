//! Branch configuration and statistics.

use serde::Deserialize;
use serde::Serialize;

/// Optional overrides for branch resource limits.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BranchConfig {
    /// Override for `MAX_BRANCH_DIRTY_KEYS`. None uses the default.
    pub max_dirty_keys: Option<u32>,
    /// Override for `MAX_BRANCH_TOTAL_BYTES`. None uses the default.
    pub max_total_bytes: Option<u64>,
    /// Override for `BRANCH_COMMIT_TIMEOUT_MS`. None uses the default.
    pub commit_timeout_ms: Option<u64>,
}

/// Current branch statistics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchStats {
    /// Number of dirty (modified or tombstoned) keys.
    pub dirty_count: u32,
    /// Total bytes of dirty write values.
    pub dirty_bytes: u64,
    /// Number of keys in the read set (tracked for conflict detection).
    pub read_set_size: u32,
    /// Nesting depth of this branch.
    pub depth: u8,
}
