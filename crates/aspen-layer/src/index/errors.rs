//! Error types for index operations.

use snafu::Snafu;

use super::MAX_INDEXES;

/// Errors that can occur during index operations.
#[derive(Debug, Snafu)]
pub enum IndexError {
    /// Index not found in registry.
    #[snafu(display("index not found: {name}"))]
    NotFound {
        /// Name of the missing index.
        name: String,
    },

    /// Too many indexes registered.
    #[snafu(display("too many indexes: max is {}", MAX_INDEXES))]
    TooManyIndexes,

    /// Failed to extract index key from entry.
    #[snafu(display("failed to extract index key for index {name}: {reason}"))]
    ExtractionFailed {
        /// Name of the index.
        name: String,
        /// Reason for the failure.
        reason: String,
    },

    /// Failed to unpack index key.
    #[snafu(display("failed to unpack index key: {reason}"))]
    UnpackFailed {
        /// Reason for the failure.
        reason: String,
    },
}

/// Result type for index operations.
pub type IndexResult<T> = Result<T, IndexError>;
