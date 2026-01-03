//! Key-Value request handler.
//!
//! Handles: ReadKey, WriteKey, DeleteKey, ScanKeys, BatchRead, BatchWrite,
//! ConditionalBatchWrite, CompareAndSwapKey, CompareAndDeleteKey.

use crate::context::ClientProtocolContext;
use crate::error_sanitization::sanitize_kv_error;
use crate::registry::RequestHandler;
use aspen_client_rpc::BatchCondition;
use aspen_client_rpc::BatchReadResultResponse;
use aspen_client_rpc::BatchWriteOperation;
use aspen_client_rpc::BatchWriteResultResponse;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_client_rpc::CompareAndSwapResultResponse;
use aspen_client_rpc::ConditionalBatchWriteResultResponse;
use aspen_client_rpc::DeleteResultResponse;
use aspen_client_rpc::ReadResultResponse;
use aspen_client_rpc::ScanEntry;
use aspen_client_rpc::ScanResultResponse;
use aspen_client_rpc::WriteResultResponse;
use aspen_core::BatchCondition as ApiBatchCondition;
use aspen_core::BatchOperation;
use aspen_core::KeyValueStore;
use aspen_core::KeyValueStoreError;
use aspen_core::ReadRequest;
use aspen_core::ScanRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use aspen_core::validate_client_key;

/// Handler for key-value operations.
pub struct KvHandler;

#[async_trait::async_trait]
impl RequestHandler for KvHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::ReadKey { .. }
                | ClientRpcRequest::WriteKey { .. }
                | ClientRpcRequest::DeleteKey { .. }
                | ClientRpcRequest::ScanKeys { .. }
                | ClientRpcRequest::BatchRead { .. }
                | ClientRpcRequest::BatchWrite { .. }
                | ClientRpcRequest::ConditionalBatchWrite { .. }
                | ClientRpcRequest::CompareAndSwapKey { .. }
                | ClientRpcRequest::CompareAndDeleteKey { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ReadKey { key } => handle_read_key(ctx, key).await,
            ClientRpcRequest::WriteKey { key, value } => handle_write_key(ctx, key, value).await,
            ClientRpcRequest::DeleteKey { key } => handle_delete_key(ctx, key).await,
            ClientRpcRequest::CompareAndSwapKey {
                key,
                expected,
                new_value,
            } => handle_compare_and_swap(ctx, key, expected, new_value).await,
            ClientRpcRequest::CompareAndDeleteKey { key, expected } => {
                handle_compare_and_delete(ctx, key, expected).await
            }
            ClientRpcRequest::ScanKeys {
                prefix,
                limit,
                continuation_token,
            } => handle_scan_keys(ctx, prefix, limit, continuation_token).await,
            ClientRpcRequest::BatchRead { keys } => handle_batch_read(ctx, keys).await,
            ClientRpcRequest::BatchWrite { operations } => handle_batch_write(ctx, operations).await,
            ClientRpcRequest::ConditionalBatchWrite { conditions, operations } => {
                handle_conditional_batch_write(ctx, conditions, operations).await
            }
            _ => Err(anyhow::anyhow!("request not handled by KvHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "KvHandler"
    }
}

// =============================================================================
// Individual handler functions (Tiger Style: functions under 70 lines)
// =============================================================================

async fn handle_read_key(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    let result = ctx.kv_store.read(ReadRequest::new(key)).await;

    match result {
        Ok(resp) => {
            let value = resp.kv.map(|kv| kv.value.into_bytes());
            Ok(ClientRpcResponse::ReadResult(ReadResultResponse {
                value,
                found: true,
                error: None,
            }))
        }
        Err(e) => {
            // HIGH-4: Sanitize error messages to prevent information leakage
            let (found, error) = match &e {
                KeyValueStoreError::NotFound { .. } => (false, None),
                other => (false, Some(sanitize_kv_error(other))),
            };
            Ok(ClientRpcResponse::ReadResult(ReadResultResponse {
                value: None,
                found,
                error,
            }))
        }
    }
}

async fn handle_write_key(
    ctx: &ClientProtocolContext,
    key: String,
    value: Vec<u8>,
) -> anyhow::Result<ClientRpcResponse> {
    // Validate key against reserved _system: prefix
    if let Err(vault_err) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::WriteResult(WriteResultResponse {
            success: false,
            error: Some(vault_err.to_string()),
        }));
    }

    let result = ctx
        .kv_store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key,
                value: String::from_utf8_lossy(&value).to_string(),
            },
        })
        .await;

    Ok(ClientRpcResponse::WriteResult(WriteResultResponse {
        success: result.is_ok(),
        // HIGH-4: Sanitize error messages to prevent information leakage
        error: result.err().map(|e| sanitize_kv_error(&e)),
    }))
}

async fn handle_delete_key(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    // Validate key against reserved _system: prefix
    if let Err(vault_err) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::DeleteResult(DeleteResultResponse {
            key,
            deleted: false,
            error: Some(vault_err.to_string()),
        }));
    }

    let result = ctx
        .kv_store
        .write(WriteRequest {
            command: WriteCommand::Delete { key: key.clone() },
        })
        .await;

    Ok(ClientRpcResponse::DeleteResult(DeleteResultResponse {
        key,
        deleted: result.is_ok(),
        // HIGH-4: Sanitize error messages to prevent information leakage
        error: result.err().map(|e| sanitize_kv_error(&e)),
    }))
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
            success: false,
            actual_value: None,
            error: Some(vault_err.to_string()),
        }));
    }

    let result = ctx
        .kv_store
        .write(WriteRequest {
            command: WriteCommand::CompareAndSwap {
                key,
                expected: expected.map(|v| String::from_utf8_lossy(&v).to_string()),
                new_value: String::from_utf8_lossy(&new_value).to_string(),
            },
        })
        .await;

    match result {
        Ok(_) => Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
            success: true,
            actual_value: None,
            error: None,
        })),
        Err(KeyValueStoreError::CompareAndSwapFailed { actual, .. }) => {
            Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
                success: false,
                actual_value: actual.map(|v| v.into_bytes()),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
            success: false,
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
            success: false,
            actual_value: None,
            error: Some(vault_err.to_string()),
        }));
    }

    let result = ctx
        .kv_store
        .write(WriteRequest {
            command: WriteCommand::CompareAndDelete {
                key,
                expected: String::from_utf8_lossy(&expected).to_string(),
            },
        })
        .await;

    match result {
        Ok(_) => Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
            success: true,
            actual_value: None,
            error: None,
        })),
        Err(KeyValueStoreError::CompareAndSwapFailed { actual, .. }) => {
            Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
                success: false,
                actual_value: actual.map(|v| v.into_bytes()),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
            success: false,
            actual_value: None,
            // HIGH-4: Sanitize error messages to prevent information leakage
            error: Some(sanitize_kv_error(&e)),
        })),
    }
}

async fn handle_scan_keys(
    ctx: &ClientProtocolContext,
    prefix: String,
    limit: Option<u32>,
    continuation_token: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let result = ctx
        .kv_store
        .scan(ScanRequest {
            prefix,
            limit,
            continuation_token,
        })
        .await;

    match result {
        Ok(scan_resp) => {
            // Convert from api::KeyValueWithRevision to client_rpc::ScanEntry
            let entries: Vec<ScanEntry> = scan_resp
                .entries
                .into_iter()
                .map(|e| ScanEntry {
                    key: e.key,
                    value: e.value,
                    version: e.version,
                    create_revision: e.create_revision,
                    mod_revision: e.mod_revision,
                })
                .collect();

            Ok(ClientRpcResponse::ScanResult(ScanResultResponse {
                entries,
                count: scan_resp.count,
                is_truncated: scan_resp.is_truncated,
                continuation_token: scan_resp.continuation_token,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ScanResult(ScanResultResponse {
            entries: vec![],
            count: 0,
            is_truncated: false,
            continuation_token: None,
            // HIGH-4: Sanitize error messages to prevent information leakage
            error: Some(sanitize_kv_error(&e)),
        })),
    }
}

async fn handle_batch_read(ctx: &ClientProtocolContext, keys: Vec<String>) -> anyhow::Result<ClientRpcResponse> {
    // Validate all keys
    for key in &keys {
        if let Err(e) = validate_client_key(key) {
            return Ok(ClientRpcResponse::BatchReadResult(BatchReadResultResponse {
                success: false,
                values: None,
                error: Some(e.to_string()),
            }));
        }
    }

    // Read all keys atomically
    let mut values = Vec::with_capacity(keys.len());
    for key in &keys {
        let request = ReadRequest::new(key.clone());
        match ctx.kv_store.read(request).await {
            Ok(result) => {
                // Key exists - return the value
                let value_bytes = result.kv.map(|kv| kv.value.into_bytes());
                values.push(value_bytes);
            }
            Err(KeyValueStoreError::NotFound { .. }) => {
                // Key doesn't exist - return None for this position
                values.push(None);
            }
            Err(e) => {
                // Real error - fail the entire batch
                return Ok(ClientRpcResponse::BatchReadResult(BatchReadResultResponse {
                    success: false,
                    values: None,
                    error: Some(e.to_string()),
                }));
            }
        }
    }

    Ok(ClientRpcResponse::BatchReadResult(BatchReadResultResponse {
        success: true,
        values: Some(values),
        error: None,
    }))
}

async fn handle_batch_write(
    ctx: &ClientProtocolContext,
    operations: Vec<BatchWriteOperation>,
) -> anyhow::Result<ClientRpcResponse> {
    // Validate all keys
    for op in &operations {
        let key = match op {
            BatchWriteOperation::Set { key, .. } => key,
            BatchWriteOperation::Delete { key } => key,
        };
        if let Err(e) = validate_client_key(key) {
            return Ok(ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
                success: false,
                operations_applied: None,
                error: Some(e.to_string()),
            }));
        }
    }

    // Convert to internal batch operations
    let batch_ops: Vec<BatchOperation> = operations
        .iter()
        .map(|op| match op {
            BatchWriteOperation::Set { key, value } => BatchOperation::Set {
                key: key.clone(),
                value: String::from_utf8_lossy(value).to_string(),
            },
            BatchWriteOperation::Delete { key } => BatchOperation::Delete { key: key.clone() },
        })
        .collect();

    let request = WriteRequest {
        command: WriteCommand::Batch { operations: batch_ops },
    };

    match ctx.kv_store.write(request).await {
        Ok(result) => Ok(ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
            success: true,
            operations_applied: result.batch_applied,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
            success: false,
            operations_applied: None,
            error: Some(e.to_string()),
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
                success: false,
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
                success: false,
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
                expected: String::from_utf8_lossy(expected).to_string(),
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
                value: String::from_utf8_lossy(value).to_string(),
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
                success: conditions_met,
                conditions_met,
                operations_applied: result.batch_applied,
                failed_condition_index: result.failed_condition_index,
                failed_condition_reason: None,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
            success: false,
            conditions_met: false,
            operations_applied: None,
            failed_condition_index: None,
            failed_condition_reason: None,
            error: Some(e.to_string()),
        })),
    }
}
