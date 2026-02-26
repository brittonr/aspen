//! Batch operation types.
//!
//! Request/response types for atomic multi-key batch read and write operations.

use serde::Deserialize;
use serde::Serialize;

/// Batch domain request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchRequest {
    /// Read multiple keys atomically.
    BatchRead { keys: Vec<String> },
    /// Write multiple operations atomically.
    BatchWrite { operations: Vec<BatchWriteOperation> },
    /// Conditional batch write (etcd-style transaction).
    ConditionalBatchWrite {
        conditions: Vec<BatchCondition>,
        operations: Vec<BatchWriteOperation>,
    },
}

#[cfg(feature = "auth")]
impl BatchRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            Self::BatchRead { keys } => keys.first().map(|key| Operation::Read { key: key.clone() }),
            Self::BatchWrite { operations } | Self::ConditionalBatchWrite { operations, .. } => {
                operations.first().map(|op| match op {
                    BatchWriteOperation::Set { key, value } => Operation::Write {
                        key: key.clone(),
                        value: value.clone(),
                    },
                    BatchWriteOperation::Delete { key } => Operation::Write {
                        key: key.clone(),
                        value: vec![],
                    },
                })
            }
        }
    }
}

/// A single operation within a batch write.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchWriteOperation {
    /// Set a key to a value.
    Set { key: String, value: Vec<u8> },
    /// Delete a key.
    Delete { key: String },
}

/// A condition for conditional batch writes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchCondition {
    /// Key must have exactly this value.
    ValueEquals { key: String, expected: Vec<u8> },
    /// Key must exist (any value).
    KeyExists { key: String },
    /// Key must not exist.
    KeyNotExists { key: String },
}

/// Batch read result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchReadResultResponse {
    /// Whether the batch read succeeded.
    pub is_success: bool,
    /// Values for each key in request order.
    pub values: Option<Vec<Option<Vec<u8>>>>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Batch write result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchWriteResultResponse {
    /// Whether the batch write succeeded.
    pub is_success: bool,
    /// Number of operations applied (all or none).
    pub operations_applied: Option<u32>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Conditional batch write result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalBatchWriteResultResponse {
    /// Whether the batch executed (all conditions passed).
    pub is_success: bool,
    /// Whether all conditions were satisfied.
    pub conditions_met: bool,
    /// Number of operations applied (0 if conditions failed).
    pub operations_applied: Option<u32>,
    /// Index of first failed condition (if any).
    pub failed_condition_index: Option<u32>,
    /// Details about why condition failed.
    pub failed_condition_reason: Option<String>,
    /// Error message if operation failed due to error (not condition).
    pub error: Option<String>,
}

/// Queue enqueue item (re-exported from aspen-coordination-protocol).
pub use aspen_coordination_protocol::QueueEnqueueItem;
