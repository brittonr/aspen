//! Tiger Style resource bounds for KV branch overlays.

/// Maximum number of dirty (modified) keys in a single branch.
pub const MAX_BRANCH_DIRTY_KEYS: u32 = 10_000;

/// Maximum total bytes of dirty values in a single branch (64 MB).
pub const MAX_BRANCH_TOTAL_BYTES: u64 = 64_u64.saturating_mul(1024).saturating_mul(1024);

/// Maximum nesting depth for branch-within-branch composition.
pub const MAX_BRANCH_DEPTH: u8 = 8;

/// Timeout in milliseconds for a branch commit's Raft write.
pub const BRANCH_COMMIT_TIMEOUT_MS: u64 = 10_000;
