//! Batch operations client for atomic multi-key reads and writes.

use std::sync::Arc;

use anyhow::Result;
use anyhow::bail;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;

use super::CoordinationRpc;

/// Client for batch read/write operations.
///
/// Provides atomic multi-key operations for efficient bulk access.
pub struct BatchClient<C: CoordinationRpc> {
    client: Arc<C>,
}

impl<C: CoordinationRpc> BatchClient<C> {
    /// Create a new batch client.
    pub fn new(client: Arc<C>) -> Self {
        Self { client }
    }

    /// Read multiple keys atomically.
    ///
    /// Returns values for all keys in the same order as requested.
    /// Non-existent keys return None in their position.
    pub async fn read(&self, keys: Vec<String>) -> Result<Vec<Option<Vec<u8>>>> {
        use aspen_client_api::BatchReadResultResponse;

        let response = self.client.send_coordination_request(ClientRpcRequest::BatchRead { keys }).await?;

        match response {
            ClientRpcResponse::BatchReadResult(BatchReadResultResponse { success, values, error }) => {
                if success {
                    Ok(values.unwrap_or_default())
                } else {
                    bail!("batch read failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for BatchRead"),
        }
    }

    /// Write multiple key-value operations atomically.
    ///
    /// All operations in the batch are applied atomically - either all succeed
    /// or none are applied.
    pub async fn write(&self, operations: Vec<BatchWriteOp>) -> Result<u32> {
        use aspen_client_api::BatchWriteOperation;
        use aspen_client_api::BatchWriteResultResponse;

        let ops: Vec<BatchWriteOperation> = operations
            .into_iter()
            .map(|op| match op {
                BatchWriteOp::Set { key, value } => BatchWriteOperation::Set { key, value },
                BatchWriteOp::Delete { key } => BatchWriteOperation::Delete { key },
            })
            .collect();

        let response = self.client.send_coordination_request(ClientRpcRequest::BatchWrite { operations: ops }).await?;

        match response {
            ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
                success,
                operations_applied,
                error,
            }) => {
                if success {
                    Ok(operations_applied.unwrap_or(0))
                } else {
                    bail!("batch write failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for BatchWrite"),
        }
    }

    /// Write with conditions (etcd-style transaction).
    ///
    /// Checks all conditions first. If all pass, applies all operations atomically.
    /// If any condition fails, returns which condition failed without modifying data.
    pub async fn conditional_write(
        &self,
        conditions: Vec<BatchConditionOp>,
        operations: Vec<BatchWriteOp>,
    ) -> Result<ConditionalBatchResult> {
        use aspen_client_api::BatchCondition;
        use aspen_client_api::BatchWriteOperation;
        use aspen_client_api::ConditionalBatchWriteResultResponse;

        let conds: Vec<BatchCondition> = conditions
            .into_iter()
            .map(|c| match c {
                BatchConditionOp::ValueEquals { key, expected } => BatchCondition::ValueEquals { key, expected },
                BatchConditionOp::KeyExists { key } => BatchCondition::KeyExists { key },
                BatchConditionOp::KeyNotExists { key } => BatchCondition::KeyNotExists { key },
            })
            .collect();

        let ops: Vec<BatchWriteOperation> = operations
            .into_iter()
            .map(|op| match op {
                BatchWriteOp::Set { key, value } => BatchWriteOperation::Set { key, value },
                BatchWriteOp::Delete { key } => BatchWriteOperation::Delete { key },
            })
            .collect();

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::ConditionalBatchWrite {
                conditions: conds,
                operations: ops,
            })
            .await?;

        match response {
            ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
                success: _,
                conditions_met,
                operations_applied,
                failed_condition_index,
                failed_condition_reason,
                error,
            }) => {
                if let Some(err) = error {
                    bail!("conditional batch failed: {}", err);
                }

                if conditions_met {
                    Ok(ConditionalBatchResult::Applied {
                        operations_count: operations_applied.unwrap_or(0),
                    })
                } else {
                    Ok(ConditionalBatchResult::ConditionFailed {
                        index: failed_condition_index.unwrap_or(0),
                        reason: failed_condition_reason,
                    })
                }
            }
            _ => bail!("unexpected response type for ConditionalBatchWrite"),
        }
    }
}

/// A batch write operation.
#[derive(Debug, Clone)]
pub enum BatchWriteOp {
    /// Set a key to a value.
    Set {
        /// Key to set.
        key: String,
        /// Value to set.
        value: Vec<u8>,
    },
    /// Delete a key.
    Delete {
        /// Key to delete.
        key: String,
    },
}

impl BatchWriteOp {
    /// Create a Set operation.
    pub fn set(key: impl Into<String>, value: impl Into<Vec<u8>>) -> Self {
        BatchWriteOp::Set {
            key: key.into(),
            value: value.into(),
        }
    }

    /// Create a Delete operation.
    pub fn delete(key: impl Into<String>) -> Self {
        BatchWriteOp::Delete { key: key.into() }
    }
}

/// A condition for conditional batch writes.
#[derive(Debug, Clone)]
pub enum BatchConditionOp {
    /// Key must have this exact value.
    ValueEquals {
        /// Key to check.
        key: String,
        /// Expected value.
        expected: Vec<u8>,
    },
    /// Key must exist.
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

impl BatchConditionOp {
    /// Create a ValueEquals condition.
    pub fn value_equals(key: impl Into<String>, expected: impl Into<Vec<u8>>) -> Self {
        BatchConditionOp::ValueEquals {
            key: key.into(),
            expected: expected.into(),
        }
    }

    /// Create a KeyExists condition.
    pub fn key_exists(key: impl Into<String>) -> Self {
        BatchConditionOp::KeyExists { key: key.into() }
    }

    /// Create a KeyNotExists condition.
    pub fn key_not_exists(key: impl Into<String>) -> Self {
        BatchConditionOp::KeyNotExists { key: key.into() }
    }
}

/// Result of a conditional batch operation.
#[derive(Debug, Clone)]
pub enum ConditionalBatchResult {
    /// All conditions passed, operations were applied.
    Applied {
        /// Number of operations that were applied.
        operations_count: u32,
    },
    /// A condition failed, no operations were applied.
    ConditionFailed {
        /// Index of the first failed condition.
        index: u32,
        /// Optional reason for failure.
        reason: Option<String>,
    },
}

impl ConditionalBatchResult {
    /// Check if the batch was applied.
    pub fn is_applied(&self) -> bool {
        matches!(self, ConditionalBatchResult::Applied { .. })
    }

    /// Check if a condition failed.
    pub fn is_condition_failed(&self) -> bool {
        matches!(self, ConditionalBatchResult::ConditionFailed { .. })
    }
}
