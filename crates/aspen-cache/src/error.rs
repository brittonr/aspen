//! Error types for the Nix binary cache.

use snafu::Snafu;

/// Result type for cache operations.
pub type Result<T> = std::result::Result<T, CacheError>;

/// Nix binary cache errors.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum CacheError {
    /// Invalid store path format.
    #[snafu(display("Invalid store path: {reason}"))]
    InvalidStorePath {
        /// Reason for invalidity.
        reason: String,
    },

    /// Invalid store hash format.
    #[snafu(display("Invalid store hash '{hash}': {reason}"))]
    InvalidStoreHash {
        /// The invalid hash.
        hash: String,
        /// Reason for invalidity.
        reason: String,
    },

    /// Cache entry not found.
    #[snafu(display("Cache entry not found for hash: {store_hash}"))]
    EntryNotFound {
        /// Store hash that was not found.
        store_hash: String,
    },

    /// Failed to serialize cache entry.
    #[snafu(display("Serialization error: {message}"))]
    Serialization {
        /// Error message.
        message: String,
    },

    /// Failed to deserialize cache entry.
    #[snafu(display("Deserialization error: {message}"))]
    Deserialization {
        /// Error message.
        message: String,
    },

    /// KV store operation failed.
    #[snafu(display("KV store error: {message}"))]
    KvStore {
        /// Error message.
        message: String,
    },

    /// Blob store operation failed.
    #[snafu(display("Blob store error: {message}"))]
    BlobStore {
        /// Error message.
        message: String,
    },

    /// Nix command failed.
    #[snafu(display("Nix command '{command}' failed: {reason}"))]
    NixCommand {
        /// Command that failed.
        command: String,
        /// Failure reason.
        reason: String,
    },

    /// Too many references in cache entry.
    #[snafu(display("Too many references: {count} (max: {max})"))]
    TooManyReferences {
        /// Actual count.
        count: u32,
        /// Maximum allowed.
        max: u32,
    },

    /// Deriver path too long.
    #[snafu(display("Deriver path too long: {length_bytes} bytes (max: {max_bytes})"))]
    DeriverTooLong {
        /// Actual length in bytes.
        length_bytes: u64,
        /// Maximum allowed in bytes.
        max_bytes: u64,
    },
}

impl From<serde_json::Error> for CacheError {
    fn from(err: serde_json::Error) -> Self {
        CacheError::Deserialization {
            message: err.to_string(),
        }
    }
}
