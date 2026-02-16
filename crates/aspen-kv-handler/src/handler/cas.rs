//! Compare-and-swap handler functions.
//!
//! Handles: CompareAndSwapKey, CompareAndDeleteKey, ConditionalBatchWrite.

use aspen_client_api::BatchCondition;
use aspen_client_api::BatchWriteOperation;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::CompareAndSwapResultResponse;
use aspen_client_api::ConditionalBatchWriteResultResponse;
use aspen_core::BatchCondition as ApiBatchCondition;
use aspen_core::BatchOperation;
use aspen_core::KeyValueStore;
use aspen_core::KeyValueStoreError;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use aspen_core::validate_client_key;
use aspen_rpc_core::ClientProtocolContext;

use crate::error_sanitization::sanitize_kv_error;
use crate::verified::bytes_to_string_lossy;

/// Sub-handler for compare-and-swap operations.
pub(crate) struct CasHandler;

impl CasHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::CompareAndSwapKey { .. }
                | ClientRpcRequest::CompareAndDeleteKey { .. }
                | ClientRpcRequest::ConditionalBatchWrite { .. }
        )
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::CompareAndSwapKey {
                key,
                expected,
                new_value,
            } => handle_compare_and_swap(ctx, key, expected, new_value).await,
            ClientRpcRequest::CompareAndDeleteKey { key, expected } => {
                handle_compare_and_delete(ctx, key, expected).await
            }
            ClientRpcRequest::ConditionalBatchWrite { conditions, operations } => {
                handle_conditional_batch_write(ctx, conditions, operations).await
            }
            _ => Err(anyhow::anyhow!("request not handled by CasHandler")),
        }
    }
}

async fn handle_compare_and_swap(
    ctx: &ClientProtocolContext,
    key: String,
    expected: Option<Vec<u8>>,
    new_value: Vec<u8>,
) -> anyhow::Result<ClientRpcResponse> {
    // Validate key against reserved _system: prefix
    if let Err(vault_err) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
            is_success: false,
            actual_value: None,
            error: Some(vault_err.to_string()),
        }));
    }

    let result = ctx
        .kv_store
        .write(WriteRequest {
            command: WriteCommand::CompareAndSwap {
                key,
                expected: expected.as_ref().map(|v| bytes_to_string_lossy(v)),
                new_value: bytes_to_string_lossy(&new_value),
            },
        })
        .await;

    match result {
        Ok(_) => Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
            is_success: true,
            actual_value: None,
            error: None,
        })),
        Err(KeyValueStoreError::CompareAndSwapFailed { actual, .. }) => {
            Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
                is_success: false,
                actual_value: actual.map(|v| v.into_bytes()),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
            is_success: false,
            actual_value: None,
            // HIGH-4: Sanitize error messages to prevent information leakage
            error: Some(sanitize_kv_error(&e)),
        })),
    }
}

async fn handle_compare_and_delete(
    ctx: &ClientProtocolContext,
    key: String,
    expected: Vec<u8>,
) -> anyhow::Result<ClientRpcResponse> {
    // Validate key against reserved _system: prefix
    if let Err(vault_err) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
            is_success: false,
            actual_value: None,
            error: Some(vault_err.to_string()),
        }));
    }

    let result = ctx
        .kv_store
        .write(WriteRequest {
            command: WriteCommand::CompareAndDelete {
                key,
                expected: bytes_to_string_lossy(&expected),
            },
        })
        .await;

    match result {
        Ok(_) => Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
            is_success: true,
            actual_value: None,
            error: None,
        })),
        Err(KeyValueStoreError::CompareAndSwapFailed { actual, .. }) => {
            Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
                is_success: false,
                actual_value: actual.map(|v| v.into_bytes()),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
            is_success: false,
            actual_value: None,
            // HIGH-4: Sanitize error messages to prevent information leakage
            error: Some(sanitize_kv_error(&e)),
        })),
    }
}

async fn handle_conditional_batch_write(
    ctx: &ClientProtocolContext,
    conditions: Vec<BatchCondition>,
    operations: Vec<BatchWriteOperation>,
) -> anyhow::Result<ClientRpcResponse> {
    // Validate all keys in conditions
    for cond in &conditions {
        let key = match cond {
            BatchCondition::ValueEquals { key, .. } => key,
            BatchCondition::KeyExists { key } => key,
            BatchCondition::KeyNotExists { key } => key,
        };
        if let Err(e) = validate_client_key(key) {
            return Ok(ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
                is_success: false,
                conditions_met: false,
                operations_applied: None,
                failed_condition_index: None,
                failed_condition_reason: Some(e.to_string()),
                error: None,
            }));
        }
    }

    // Validate all keys in operations
    for op in &operations {
        let key = match op {
            BatchWriteOperation::Set { key, .. } => key,
            BatchWriteOperation::Delete { key } => key,
        };
        if let Err(e) = validate_client_key(key) {
            return Ok(ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
                is_success: false,
                conditions_met: false,
                operations_applied: None,
                failed_condition_index: None,
                failed_condition_reason: Some(e.to_string()),
                error: None,
            }));
        }
    }

    // Convert conditions
    let api_conditions: Vec<ApiBatchCondition> = conditions
        .iter()
        .map(|c| match c {
            BatchCondition::ValueEquals { key, expected } => ApiBatchCondition::ValueEquals {
                key: key.clone(),
                expected: bytes_to_string_lossy(expected),
            },
            BatchCondition::KeyExists { key } => ApiBatchCondition::KeyExists { key: key.clone() },
            BatchCondition::KeyNotExists { key } => ApiBatchCondition::KeyNotExists { key: key.clone() },
        })
        .collect();

    // Convert operations
    let batch_ops: Vec<BatchOperation> = operations
        .iter()
        .map(|op| match op {
            BatchWriteOperation::Set { key, value } => BatchOperation::Set {
                key: key.clone(),
                value: bytes_to_string_lossy(value),
            },
            BatchWriteOperation::Delete { key } => BatchOperation::Delete { key: key.clone() },
        })
        .collect();

    let request = WriteRequest {
        command: WriteCommand::ConditionalBatch {
            conditions: api_conditions,
            operations: batch_ops,
        },
    };

    match ctx.kv_store.write(request).await {
        Ok(result) => {
            let conditions_met = result.conditions_met.unwrap_or(false);
            Ok(ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
                is_success: conditions_met,
                conditions_met,
                operations_applied: result.batch_applied,
                failed_condition_index: result.failed_condition_index,
                failed_condition_reason: None,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
            is_success: false,
            conditions_met: false,
            operations_applied: None,
            failed_condition_index: None,
            failed_condition_reason: None,
            error: Some(e.to_string()),
        })),
    }
}
