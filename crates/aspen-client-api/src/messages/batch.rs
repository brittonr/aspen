//! Batch operation types.
//!
//! Types for atomic multi-key batch read and write operations.

use serde::{Deserialize, Serialize};

/// A single operation within a batch write.
///
/// Supports Set and Delete operations that can be mixed freely.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchWriteOperation {
    /// Set a key to a value.
    Set {
        /// Key to set.
        key: String,
        /// Value to set (as bytes for RPC transport).
        value: Vec<u8>,
    },
    /// Delete a key.
    Delete {
        /// Key to delete.
        key: String,
    },
}

/// A condition for conditional batch writes.
///
/// All conditions must be satisfied for the batch to execute.
/// Similar to etcd's transaction compare operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchCondition {
    /// Key must have exactly this value.
    ValueEquals {
        /// Key to check.
        key: String,
        /// Expected value (as bytes).
        expected: Vec<u8>,
    },
    /// Key must exist (any value).
    KeyExists {
        /// Key to check.
        key: String,
    },
    /// Key must not exist.
    KeyNotExists {
        /// Key to check.
        key: String,
    },
}

/// Batch read result response.
///
/// Contains values for all requested keys in order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchReadResultResponse {
    /// Whether the batch read succeeded.
    pub success: bool,
    /// Values for each key in request order.
    /// None for keys that don't exist.
    pub values: Option<Vec<Option<Vec<u8>>>>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Batch write result response.
///
/// Reports success/failure for the entire atomic batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchWriteResultResponse {
    /// Whether the batch write succeeded.
    pub success: bool,
    /// Number of operations applied (all or none).
    pub operations_applied: Option<u32>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Conditional batch write result response.
///
/// Reports whether conditions passed and operations were applied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalBatchWriteResultResponse {
    /// Whether the batch executed (all conditions passed).
    pub success: bool,
    /// Whether all conditions were satisfied.
    pub conditions_met: bool,
    /// Number of operations applied (0 if conditions failed).
    pub operations_applied: Option<u32>,
    /// Index of first failed condition (if any).
    pub failed_condition_index: Option<u32>,
    /// Details about why condition failed (e.g., actual value).
    pub failed_condition_reason: Option<String>,
    /// Error message if operation failed due to error (not condition).
    pub error: Option<String>,
}
