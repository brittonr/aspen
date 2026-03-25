//! Error types for commit DAG operations.

use snafu::Snafu;

/// Errors from commit DAG operations.
#[derive(Debug, Snafu)]
pub enum CommitDagError {
    /// The requested commit does not exist (may have been garbage collected).
    #[snafu(display("commit not found: {commit_id_hex}"))]
    CommitNotFound { commit_id_hex: String },

    /// The commit's stored mutations_hash does not match recomputed hash.
    #[snafu(display("commit corrupted: mutations_hash mismatch for {commit_id_hex}"))]
    CommitCorrupted { commit_id_hex: String },

    /// Failed to serialize or deserialize a commit.
    #[snafu(display("commit serialization error: {reason}"))]
    CommitSerializationError { reason: String },

    /// The underlying KV store returned an error.
    #[snafu(display("commit storage error: {reason}"))]
    CommitStorageError { reason: String },

    /// Garbage collection encountered an error.
    #[snafu(display("gc error: {reason}"))]
    GcError { reason: String },
}
