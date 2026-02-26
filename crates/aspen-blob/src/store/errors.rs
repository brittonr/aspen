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

    /// Failed to delete a tag from the store.
    #[snafu(display("failed to delete tag '{tag}': {message}"))]
    DeleteTag {
        /// The tag name that could not be deleted.
        tag: String,
        /// The underlying error message.
        message: String,
    },

    /// Failed to list tags from the store.
    #[snafu(display("failed to list tags with prefix '{prefix}': {message}"))]
    ListTags {
        /// The prefix used for listing.
        prefix: String,
        /// The underlying error message.
        message: String,
    },

    /// Failed to set a tag in the store.
    #[snafu(display("failed to set tag '{tag}': {message}"))]
    SetTag {
        /// The tag name that could not be set.
        tag: String,
        /// The underlying error message.
        message: String,
    },

    /// Failed to add bytes to the blob store.
    #[snafu(display("failed to add bytes to blob store: {message}"))]
    AddBytes {
        /// The underlying error message.
        message: String,
    },

    /// Failed to add a file path to the blob store.
    #[snafu(display("failed to add path to blob store: {message}"))]
    AddPath {
        /// The underlying error message.
        message: String,
    },

    /// Failed to read file metadata.
    #[snafu(display("failed to read file metadata for '{}': {source}", path.display()))]
    ReadFileMetadata {
        /// The path that could not be read.
        path: std::path::PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// Failed to check blob existence.
    #[snafu(display("failed to check blob existence for {hash}: {message}"))]
    CheckExistence {
        /// The hash being checked.
        hash: String,
        /// The underlying error message.
        message: String,
    },

    /// Failed to list blobs from the store.
    #[snafu(display("failed to list blobs: {message}"))]
    ListBlobs {
        /// The underlying error message.
        message: String,
    },

    /// Failed to download blob from peer.
    #[snafu(display("failed to download blob {hash}: {message}"))]
    DownloadBlob {
        /// The hash of the blob being downloaded.
        hash: String,
        /// The underlying error message.
        message: String,
    },
}

impl From<anyhow::Error> for BlobStoreError {
    fn from(e: anyhow::Error) -> Self {
        BlobStoreError::Storage { message: e.to_string() }
    }
}
