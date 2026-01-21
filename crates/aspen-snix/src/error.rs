//! Error types for SNIX storage integration.
//!
//! Uses snafu for structured, context-rich error handling following Tiger Style.

use snafu::Snafu;

/// Result type alias for SNIX storage operations.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors that can occur during SNIX storage operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// Failed to read from blob storage.
    #[snafu(display("failed to read blob {digest}: {source}"))]
    ReadBlob {
        digest: String,
        source: std::io::Error,
    },

    /// Failed to write to blob storage.
    #[snafu(display("failed to write blob: {source}"))]
    WriteBlob { source: std::io::Error },

    /// Blob not found in storage.
    #[snafu(display("blob not found: {digest}"))]
    BlobNotFound { digest: String },

    /// Failed to read directory from KV store.
    #[snafu(display("failed to read directory {digest}: {source}"))]
    ReadDirectory {
        digest: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Failed to write directory to KV store.
    #[snafu(display("failed to write directory {digest}: {source}"))]
    WriteDirectory {
        digest: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Directory not found in storage.
    #[snafu(display("directory not found: {digest}"))]
    DirectoryNotFound { digest: String },

    /// Failed to decode directory from protobuf.
    #[snafu(display("failed to decode directory {digest}: {source}"))]
    DecodeDirectory {
        digest: String,
        source: prost::DecodeError,
    },

    /// Failed to encode directory to protobuf.
    #[snafu(display("failed to encode directory: {source}"))]
    EncodeDirectory { source: prost::EncodeError },

    /// Failed to read path info from KV store.
    #[snafu(display("failed to read path info for {store_path}: {source}"))]
    ReadPathInfo {
        store_path: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Failed to write path info to KV store.
    #[snafu(display("failed to write path info for {store_path}: {source}"))]
    WritePathInfo {
        store_path: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Path info not found in storage.
    #[snafu(display("path info not found: {store_path}"))]
    PathInfoNotFound { store_path: String },

    /// Failed to decode path info from JSON.
    #[snafu(display("failed to decode path info: {source}"))]
    DecodePathInfo { source: serde_json::Error },

    /// Failed to encode path info to JSON.
    #[snafu(display("failed to encode path info: {source}"))]
    EncodePathInfo { source: serde_json::Error },

    /// Invalid store path format.
    #[snafu(display("invalid store path: {path}"))]
    InvalidStorePath { path: String },

    /// Invalid digest format or length.
    #[snafu(display("invalid digest: {message}"))]
    InvalidDigest { message: String },

    /// Resource limit exceeded.
    #[snafu(display("resource limit exceeded: {message}"))]
    ResourceLimit { message: String },

    /// Hash conversion failed.
    #[snafu(display("hash conversion failed: {message}"))]
    HashConversion { message: String },

    /// Temporary file operation failed.
    #[snafu(display("temp file operation failed: {source}"))]
    TempFile { source: std::io::Error },

    /// Directory graph contains a cycle.
    #[snafu(display("directory graph contains cycle at {digest}"))]
    DirectoryCycle { digest: String },

    /// Maximum directory depth exceeded.
    #[snafu(display("maximum directory depth {max_depth} exceeded"))]
    MaxDepthExceeded { max_depth: u32 },
}

impl From<Error> for std::io::Error {
    fn from(err: Error) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, err)
    }
}
