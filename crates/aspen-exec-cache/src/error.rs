//! Error types for the execution cache.

use snafu::Snafu;

/// Result type for execution cache operations.
pub type Result<T> = std::result::Result<T, ExecCacheError>;

/// Execution cache errors.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ExecCacheError {
    /// Cache entry not found for the given key.
    #[snafu(display("Cache miss for key: {key_hex}"))]
    CacheMiss {
        /// Hex-encoded cache key.
        key_hex: String,
    },

    /// Cache entry has expired.
    #[snafu(display("Cache entry expired for key: {key_hex}"))]
    EntryExpired {
        /// Hex-encoded cache key.
        key_hex: String,
    },

    /// Failed to serialize a cache entry.
    #[snafu(display("Failed to serialize cache entry: {source}"))]
    Serialize {
        /// Underlying serialization error.
        source: serde_json::Error,
    },

    /// Failed to deserialize a cache entry.
    #[snafu(display("Failed to deserialize cache entry: {source}"))]
    Deserialize {
        /// Underlying deserialization error.
        source: serde_json::Error,
    },

    /// Output blob exceeds the maximum allowed size.
    #[snafu(display("Output blob too large: {size_bytes} bytes (max: {max_bytes})"))]
    OutputTooLarge {
        /// Actual size in bytes.
        size_bytes: u64,
        /// Maximum allowed size in bytes.
        max_bytes: u64,
    },

    /// Too many output files in a single cache entry.
    #[snafu(display("Too many output files: {count} (max: {max})"))]
    TooManyOutputFiles {
        /// Actual count.
        count: u32,
        /// Maximum allowed.
        max: u32,
    },

    /// Too many input files tracked in a session.
    #[snafu(display("Too many input files: {count} (max: {max})"))]
    TooManyInputFiles {
        /// Actual count.
        count: u32,
        /// Maximum allowed.
        max: u32,
    },

    /// KV store operation failed.
    #[snafu(display("KV store error: {message}"))]
    KvStore {
        /// Error description.
        message: String,
    },

    /// Blob store operation failed.
    #[snafu(display("Blob store error: {message}"))]
    BlobStore {
        /// Error description.
        message: String,
    },

    /// Cache lookup timed out.
    #[snafu(display("Cache lookup timed out after {timeout_ms}ms"))]
    LookupTimeout {
        /// Timeout duration in milliseconds.
        timeout_ms: u64,
    },

    /// Tracking session not found for the given PID.
    #[snafu(display("No tracking session for PID {pid}"))]
    SessionNotFound {
        /// Process ID.
        pid: u32,
    },

    /// Tracking was disabled for a session due to resource limits.
    #[snafu(display("Tracking disabled for PID {pid}: exceeded resource bounds"))]
    TrackingDisabled {
        /// Process ID.
        pid: u32,
    },
}
