//! Error types for the Nix cache gateway.

use http::StatusCode;
use snafu::Snafu;

/// Result type for Nix cache gateway operations.
pub type Result<T> = std::result::Result<T, NixCacheError>;

/// Errors from Nix cache gateway operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum NixCacheError {
    /// NAR not found in cache.
    #[snafu(display("NAR not found for store hash: {store_hash}"))]
    NotFound {
        /// The store hash that was requested.
        store_hash: String,
    },

    /// Blob content not available locally.
    #[snafu(display("blob {blob_hash} not available locally"))]
    BlobNotAvailable {
        /// The blob hash that is missing.
        blob_hash: String,
    },

    /// Invalid Range header.
    #[snafu(display("invalid range header: {reason}"))]
    InvalidRange {
        /// Why the range is invalid.
        reason: String,
    },

    /// Range not satisfiable (416 response).
    #[snafu(display("range not satisfiable: {requested} exceeds blob size {blob_size}"))]
    RangeNotSatisfiable {
        /// The requested range.
        requested: String,
        /// The actual blob size.
        blob_size: u64,
    },

    /// Cache index lookup failed.
    #[snafu(display("cache index error: {message}"))]
    CacheIndex {
        /// Error message from the cache index.
        message: String,
    },

    /// Blob store read error.
    #[snafu(display("blob store error: {message}"))]
    BlobStore {
        /// Error message from the blob store.
        message: String,
    },

    /// H3 stream error.
    #[snafu(display("h3 stream error: {message}"))]
    StreamError {
        /// Error message from h3.
        message: String,
    },

    /// Client disconnected during transfer.
    #[snafu(display("client disconnected during transfer"))]
    ClientDisconnected,

    /// Streaming timeout.
    #[snafu(display("streaming timeout after {elapsed_ms}ms"))]
    Timeout {
        /// Elapsed time before timeout.
        elapsed_ms: u64,
    },

    /// Invalid store hash format.
    #[snafu(display("invalid store hash format: {hash}"))]
    InvalidStoreHash {
        /// The invalid hash.
        hash: String,
    },

    /// Signing error.
    #[snafu(display("signing error: {message}"))]
    Signing {
        /// Error message.
        message: String,
    },

    /// Configuration error.
    #[snafu(display("configuration error: {message}"))]
    Config {
        /// Error message.
        message: String,
    },

    /// Failed to build HTTP response.
    #[snafu(display("failed to build HTTP response: {message}"))]
    ResponseBuild {
        /// Error message from http::Error.
        message: String,
    },
}

impl NixCacheError {
    /// Map error to HTTP status code.
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::BlobNotAvailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::InvalidRange { .. } => StatusCode::BAD_REQUEST,
            Self::RangeNotSatisfiable { .. } => StatusCode::RANGE_NOT_SATISFIABLE,
            Self::InvalidStoreHash { .. } => StatusCode::BAD_REQUEST,
            Self::CacheIndex { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::BlobStore { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::StreamError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ClientDisconnected => StatusCode::OK, // Partial success
            Self::Timeout { .. } => StatusCode::GATEWAY_TIMEOUT,
            Self::Signing { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Config { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ResponseBuild { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
