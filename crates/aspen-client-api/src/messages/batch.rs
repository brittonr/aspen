//! Batch operation types.
//!
//! Request/response types for atomic multi-key batch read and write operations.

use alloc::string::String;
#[cfg(feature = "auth")]
use alloc::string::ToString;
use alloc::vec::Vec;

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
fn batch_write_key(operation: &BatchWriteOperation) -> &str {
    match operation {
        BatchWriteOperation::Set { key, .. } | BatchWriteOperation::Delete { key } => key,
    }
}

#[cfg(feature = "auth")]
fn batch_condition_key(condition: &BatchCondition) -> &str {
    match condition {
        BatchCondition::ValueEquals { key, .. }
        | BatchCondition::KeyExists { key }
        | BatchCondition::KeyNotExists { key } => key,
    }
}

#[cfg(feature = "auth")]
fn conditional_batch_write_keys(conditions: &[BatchCondition], operations: &[BatchWriteOperation]) -> Vec<String> {
    conditions
        .iter()
        .map(|condition| batch_condition_key(condition).to_string())
        .chain(operations.iter().map(|operation| batch_write_key(operation).to_string()))
        .collect()
}

#[cfg(feature = "auth")]
impl BatchRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth_core::Operation> {
        use aspen_auth_core::Operation;
        match self {
            Self::BatchRead { keys } => (!keys.is_empty()).then(|| Operation::BatchRead { keys: keys.clone() }),
            Self::BatchWrite { operations } => (!operations.is_empty()).then(|| Operation::BatchWrite {
                keys: operations.iter().map(|operation| batch_write_key(operation).to_string()).collect(),
            }),
            Self::ConditionalBatchWrite { conditions, operations } => {
                let keys = conditional_batch_write_keys(conditions, operations);
                (!keys.is_empty()).then(|| Operation::BatchWrite { keys })
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

#[cfg(all(test, feature = "auth"))]
mod tests {
    use aspen_auth_core::Operation;

    use super::*;

    #[test]
    fn batch_request_to_operation_covers_every_read_key() {
        let request = BatchRequest::BatchRead {
            keys: alloc::vec!["allowed/a".to_string(), "denied/b".to_string()],
        };

        let Some(Operation::BatchRead { keys }) = request.to_operation() else {
            panic!("batch request should produce batch-read authorization");
        };

        assert_eq!(keys, alloc::vec!["allowed/a".to_string(), "denied/b".to_string()]);
    }

    #[test]
    fn batch_request_to_operation_covers_every_write_key() {
        let request = BatchRequest::BatchWrite {
            operations: alloc::vec![
                BatchWriteOperation::Set {
                    key: "allowed/a".to_string(),
                    value: alloc::vec![1],
                },
                BatchWriteOperation::Delete {
                    key: "denied/b".to_string(),
                },
            ],
        };

        let Some(Operation::BatchWrite { keys }) = request.to_operation() else {
            panic!("batch request should produce batch-write authorization");
        };

        assert_eq!(keys, alloc::vec!["allowed/a".to_string(), "denied/b".to_string()]);
    }

    #[test]
    fn conditional_batch_request_to_operation_covers_condition_and_write_keys() {
        let request = BatchRequest::ConditionalBatchWrite {
            conditions: alloc::vec![
                BatchCondition::ValueEquals {
                    key: "condition/value".to_string(),
                    expected: alloc::vec![1],
                },
                BatchCondition::KeyExists {
                    key: "condition/exists".to_string(),
                },
                BatchCondition::KeyNotExists {
                    key: "condition/missing".to_string(),
                },
            ],
            operations: alloc::vec![BatchWriteOperation::Set {
                key: "write/a".to_string(),
                value: alloc::vec![2],
            }],
        };

        let Some(Operation::BatchWrite { keys }) = request.to_operation() else {
            panic!("conditional batch request should authorize all condition and write keys");
        };

        assert_eq!(keys.len(), 4);
        assert_eq!(keys, alloc::vec![
            "condition/value".to_string(),
            "condition/exists".to_string(),
            "condition/missing".to_string(),
            "write/a".to_string(),
        ],);
    }

    #[test]
    fn conditional_batch_request_with_only_conditions_still_requires_authorization() {
        let request = BatchRequest::ConditionalBatchWrite {
            conditions: alloc::vec![BatchCondition::KeyExists {
                key: "condition/exists".to_string(),
            }],
            operations: alloc::vec![],
        };

        let Some(Operation::BatchWrite { keys }) = request.to_operation() else {
            panic!("condition-only batch request still reads condition keys");
        };

        assert_eq!(keys, alloc::vec!["condition/exists".to_string()]);
    }
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
