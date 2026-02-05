//! Error types for coordination primitives.

use std::fmt;

use aspen_core::KeyValueStoreError;
use snafu::Snafu;

/// Errors from coordination primitives.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum CoordinationError {
    /// Lock is held by another client.
    #[snafu(display("lock held by '{holder}' until {deadline_ms}ms"))]
    LockHeld {
        /// Current lock holder.
        holder: String,
        /// When the lock expires (Unix ms).
        deadline_ms: u64,
    },

    /// Lock was lost (another client acquired it).
    #[snafu(display("lock lost: held by '{current_holder}', not '{expected_holder}'"))]
    LockLost {
        /// Who we expected to hold the lock.
        expected_holder: String,
        /// Who currently holds the lock.
        current_holder: String,
    },

    /// Operation timed out.
    #[snafu(display("operation timed out: {operation}"))]
    Timeout {
        /// Description of the operation.
        operation: String,
    },

    /// Maximum retries exceeded.
    #[snafu(display("max retries exceeded for {operation}: {attempts} attempts"))]
    MaxRetriesExceeded {
        /// Description of the operation.
        operation: String,
        /// Number of attempts made.
        attempts: u32,
    },

    /// Sequence numbers exhausted (u64 overflow).
    #[snafu(display("sequence exhausted for key '{key}'"))]
    SequenceExhausted {
        /// The sequence key.
        key: String,
    },

    /// Data in storage is corrupted or unparseable.
    #[snafu(display("corrupted data in key '{key}': {reason}"))]
    CorruptedData {
        /// The key with corrupted data.
        key: String,
        /// Description of what went wrong.
        reason: String,
    },

    /// CAS operation failed, retry may succeed.
    #[snafu(display("CAS conflict, retry needed"))]
    CasConflict,

    /// Underlying storage error.
    #[snafu(display("storage error: {source}"))]
    Storage {
        /// The underlying error.
        source: KeyValueStoreError,
    },

    /// JSON serialization/deserialization error.
    #[snafu(display("serialization error: {source}"))]
    Serialization {
        /// The underlying error.
        source: serde_json::Error,
    },

    /// Too many readers on RWLock.
    #[snafu(display("too many readers on rwlock '{name}': {count} (max: {max})"))]
    TooManyReaders {
        /// RWLock name.
        name: String,
        /// Current reader count.
        count: u32,
        /// Maximum allowed readers.
        max: u32,
    },

    /// Too many pending writers on RWLock.
    #[snafu(display("too many pending writers on rwlock '{name}': {count} (max: {max})"))]
    TooManyPendingWriters {
        /// RWLock name.
        name: String,
        /// Current pending writer count.
        count: u32,
        /// Maximum allowed pending writers.
        max: u32,
    },

    /// Too many holders on semaphore.
    #[snafu(display("too many holders on semaphore '{name}': {count} (max: {max})"))]
    TooManySemaphoreHolders {
        /// Semaphore name.
        name: String,
        /// Current holder count.
        count: u32,
        /// Maximum allowed holders.
        max: u32,
    },
}

impl From<KeyValueStoreError> for CoordinationError {
    fn from(source: KeyValueStoreError) -> Self {
        CoordinationError::Storage { source }
    }
}

impl From<serde_json::Error> for CoordinationError {
    fn from(source: serde_json::Error) -> Self {
        CoordinationError::Serialization { source }
    }
}

/// Error when a fencing token is rejected.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum FenceError {
    /// The presented token is older than the current token.
    #[snafu(display("stale fencing token: presented {presented}, current {current}"))]
    StaleToken {
        /// The token that was presented.
        presented: u64,
        /// The current valid token.
        current: u64,
    },

    /// The token is from the future (should not happen).
    #[snafu(display("invalid fencing token: presented {presented} > current {current}"))]
    FutureToken {
        /// The token that was presented.
        presented: u64,
        /// The current valid token.
        current: u64,
    },
}

/// Error when rate limited or unable to check rate limit.
///
/// Distinguishes between actual rate limiting (tokens exhausted) and
/// storage failures (unable to determine rate limit state).
#[derive(Debug, Clone)]
pub enum RateLimitError {
    /// Request was rate limited due to insufficient tokens.
    TokensExhausted {
        /// Tokens requested.
        requested: u64,
        /// Tokens available.
        available: u64,
        /// Estimated wait time in milliseconds until enough tokens available.
        retry_after_ms: u64,
    },
    /// Storage unavailable, rate limit state cannot be determined.
    ///
    /// This may occur when the cluster is not initialized, during a
    /// network partition, or when storage is otherwise unreachable.
    StorageUnavailable {
        /// Human-readable description of the failure.
        reason: String,
    },
}

impl RateLimitError {
    /// Returns the retry_after_ms if this is a TokensExhausted error, None otherwise.
    pub fn retry_after_ms(&self) -> Option<u64> {
        match self {
            RateLimitError::TokensExhausted { retry_after_ms, .. } => Some(*retry_after_ms),
            RateLimitError::StorageUnavailable { .. } => None,
        }
    }
}

impl fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RateLimitError::TokensExhausted {
                requested,
                available,
                retry_after_ms,
            } => write!(
                f,
                "rate limited: requested {} tokens, {} available, retry after {}ms",
                requested, available, retry_after_ms
            ),
            RateLimitError::StorageUnavailable { reason } => {
                write!(f, "rate limiter storage unavailable: {}", reason)
            }
        }
    }
}

impl std::error::Error for RateLimitError {}
