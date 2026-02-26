//! Topology error types.

use super::KeyRange;
use crate::router::ShardId;

/// Errors that can occur during topology operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopologyError {
    /// The specified shard was not found.
    ShardNotFound {
        /// The shard ID that was not found.
        shard_id: ShardId,
    },
    /// A shard with this ID already exists.
    ShardAlreadyExists {
        /// The shard ID that already exists.
        shard_id: ShardId,
    },
    /// The shard is not in the expected state for this operation.
    InvalidState {
        /// The shard ID in invalid state.
        shard_id: ShardId,
        /// The expected state name.
        expected: String,
        /// The actual state name.
        actual: String,
    },
    /// The split key is not valid for the shard's range.
    InvalidSplitKey {
        /// The invalid split key.
        key: String,
        /// The shard's key range.
        range: KeyRange,
    },
    /// The ranges are not adjacent (cannot merge).
    RangesNotAdjacent {
        /// The source range.
        source: KeyRange,
        /// The target range.
        target: KeyRange,
    },
    /// Topology version mismatch (stale update).
    VersionMismatch {
        /// The expected topology version.
        expected: u64,
        /// The actual topology version.
        actual: u64,
    },
}

impl std::fmt::Display for TopologyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TopologyError::ShardNotFound { shard_id } => {
                write!(f, "shard {} not found", shard_id)
            }
            TopologyError::ShardAlreadyExists { shard_id } => {
                write!(f, "shard {} already exists", shard_id)
            }
            TopologyError::InvalidState {
                shard_id,
                expected,
                actual,
            } => {
                write!(f, "shard {} in invalid state: expected {}, got {}", shard_id, expected, actual)
            }
            TopologyError::InvalidSplitKey { key, range } => {
                write!(f, "split key '{}' not in range [{}, {})", key, range.start_key, range.end_key)
            }
            TopologyError::RangesNotAdjacent { source, target } => {
                write!(
                    f,
                    "ranges not adjacent: [{}, {}) and [{}, {})",
                    source.start_key, source.end_key, target.start_key, target.end_key
                )
            }
            TopologyError::VersionMismatch { expected, actual } => {
                write!(f, "topology version mismatch: expected {}, got {}", expected, actual)
            }
        }
    }
}

impl std::error::Error for TopologyError {}
