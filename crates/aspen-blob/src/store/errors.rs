//! Blob store error types.

use snafu::Snafu;

/// Errors from blob store operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum BlobStoreError {
    /// Blob not found.
    #[snafu(display("blob not found: {hash}"))]
    NotFound {
        /// The hash of the blob that was not found.
        hash: String,
    },

    /// Blob exceeds maximum size.
    #[snafu(display("blob size {size} exceeds maximum {max}"))]
    TooLarge {
        /// Actual size of the blob in bytes.
        size: u64,
        /// Maximum allowed size in bytes.
        max: u64,
    },

    /// Storage error.
    #[snafu(display("storage error: {message}"))]
    Storage {
        /// Human-readable description of the storage error.
        message: String,
    },

    /// Network error during download.
    #[snafu(display("download error: {message}"))]
    Download {
        /// Human-readable description of the download error.
        message: String,
    },

    /// Invalid blob ticket.
    #[snafu(display("invalid ticket: {message}"))]
    InvalidTicket {
        /// Human-readable description of why the ticket is invalid.
        message: String,
    },
}

impl From<anyhow::Error> for BlobStoreError {
    fn from(e: anyhow::Error) -> Self {
        BlobStoreError::Storage { message: e.to_string() }
    }
}
