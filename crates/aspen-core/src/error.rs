//! Error types for Aspen API operations.
//!
//! Provides explicit error types with actionable context following Tiger Style.

use thiserror::Error;

/// Errors that can occur during cluster control plane operations.
///
/// These errors indicate failures in cluster management operations like
/// initialization, membership changes, and state queries.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ControlPlaneError {
    /// The request contained invalid parameters or configuration.
    #[error("invalid request: {reason}")]
    InvalidRequest {
        /// Human-readable description of what was invalid in the request.
        reason: String,
    },

    /// The cluster has not been initialized yet.
    #[error("cluster not initialized")]
    NotInitialized,

    /// The operation failed due to a Raft or internal error.
    #[error("operation failed: {reason}")]
    Failed {
        /// Human-readable description of the failure.
        reason: String,
    },

    /// The operation is not supported by this backend implementation.
    #[error("operation not supported by {backend} backend: {operation}")]
    Unsupported {
        /// Name of the backend implementation (e.g., "in-memory", "sqlite").
        backend: String,
        /// Name of the unsupported operation.
        operation: String,
    },

    /// The operation timed out.
    #[error("operation timed out after {duration_ms}ms")]
    Timeout {
        /// Duration in milliseconds before timeout.
        duration_ms: u64,
    },
}

/// Errors that can occur during key-value operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KeyValueStoreError {
    /// The requested key was not found in the store.
    #[error("key '{key}' not found")]
    NotFound {
        /// The key that was not found.
        key: String,
    },

    /// The operation failed due to an internal error.
    #[error("operation failed: {reason}")]
    Failed {
        /// Human-readable description of the failure.
        reason: String,
    },

    /// The operation was rejected because this node is not the leader.
    #[error("not leader; current leader: {leader:?}; {reason}")]
    NotLeader {
        /// The current leader node ID, if known.
        leader: Option<u64>,
        /// Additional context about the rejection.
        reason: String,
    },

    /// The key exceeds the maximum allowed size.
    #[error("key size {size} exceeds maximum of {max} bytes")]
    KeyTooLarge {
        /// Actual size of the key in bytes.
        size: usize,
        /// Maximum allowed size in bytes.
        max: u32,
    },

    /// The value exceeds the maximum allowed size.
    #[error("value size {size} exceeds maximum of {max} bytes")]
    ValueTooLarge {
        /// Actual size of the value in bytes.
        size: usize,
        /// Maximum allowed size in bytes.
        max: u32,
    },

    /// The batch operation exceeds the maximum allowed number of keys.
    #[error("batch size {size} exceeds maximum of {max} keys")]
    BatchTooLarge {
        /// Actual number of keys in the batch.
        size: usize,
        /// Maximum allowed batch size.
        max: u32,
    },

    /// The operation timed out.
    #[error("operation timed out after {duration_ms}ms")]
    Timeout {
        /// Duration in milliseconds before timeout.
        duration_ms: u64,
    },

    /// Compare-and-swap operation failed because the current value didn't match expected.
    #[error("compare-and-swap failed for key '{key}': expected {expected:?}, found {actual:?}")]
    CompareAndSwapFailed {
        /// The key being compared.
        key: String,
        /// The expected value.
        expected: Option<String>,
        /// The actual current value.
        actual: Option<String>,
    },

    /// Key cannot be empty.
    #[error("key cannot be empty")]
    EmptyKey,

    /// The key has moved to a different shard.
    #[error("key '{key}' moved to shard {new_shard_id} (topology version {topology_version})")]
    ShardMoved {
        /// The key that was requested.
        key: String,
        /// The shard that now owns this key.
        new_shard_id: u32,
        /// Current topology version (client should update cache).
        topology_version: u64,
    },

    /// The shard is not in a state that allows this operation.
    #[error("shard {shard_id} is {state}, operation not allowed")]
    ShardNotReady {
        /// The shard that rejected the operation.
        shard_id: u32,
        /// Current state of the shard.
        state: String,
    },

    /// Topology version mismatch.
    #[error("topology version mismatch: expected {expected}, got {actual}")]
    TopologyVersionMismatch {
        /// Expected topology version.
        expected: u64,
        /// Actual current topology version.
        actual: u64,
    },
}
