//! Error types for branch operations.

use std::fmt;

/// Errors from branch overlay operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchError {
    /// The branch has reached its maximum number of dirty keys.
    DirtyKeyLimitExceeded { limit: u32, current: u32 },
    /// The branch has exceeded its total dirty bytes limit.
    ByteLimitExceeded { limit_bytes: u64, current_bytes: u64 },
    /// Cannot create a nested branch beyond the depth limit.
    DepthLimitExceeded { max_depth: u8 },
    /// The commit batch exceeds MAX_BATCH_SIZE.
    BatchTooLarge { count: u32, max: u32 },
    /// An optimistic transaction was rejected due to a concurrent modification.
    CommitConflict { key: String },
    /// The commit timed out waiting for Raft consensus.
    CommitTimeout { timeout_ms: u64 },
    /// The underlying KV store returned an error during commit.
    StoreError { reason: String },
}

impl fmt::Display for BranchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BranchError::DirtyKeyLimitExceeded { limit, current } => {
                write!(f, "branch dirty key limit exceeded: {current}/{limit}")
            }
            BranchError::ByteLimitExceeded {
                limit_bytes,
                current_bytes,
            } => {
                write!(f, "branch byte limit exceeded: {current_bytes}/{limit_bytes}")
            }
            BranchError::DepthLimitExceeded { max_depth } => {
                write!(f, "branch depth limit exceeded: max {max_depth}")
            }
            BranchError::BatchTooLarge { count, max } => {
                write!(f, "commit batch too large: {count} entries, max {max}")
            }
            BranchError::CommitConflict { key } => {
                write!(f, "commit conflict on key: {key}")
            }
            BranchError::CommitTimeout { timeout_ms } => {
                write!(f, "commit timed out after {timeout_ms}ms")
            }
            BranchError::StoreError { reason } => {
                write!(f, "store error: {reason}")
            }
        }
    }
}

impl std::error::Error for BranchError {}
