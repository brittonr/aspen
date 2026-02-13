//! Key-Value request handler.
//!
//! Handles: ReadKey, WriteKey, DeleteKey, ScanKeys, BatchRead, BatchWrite,
//! ConditionalBatchWrite, CompareAndSwapKey, CompareAndDeleteKey.

use aspen_client_api::BatchCondition;
use aspen_client_api::BatchReadResultResponse;
use aspen_client_api::BatchWriteOperation;
use aspen_client_api::BatchWriteResultResponse;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::CompareAndSwapResultResponse;
use aspen_client_api::ConditionalBatchWriteResultResponse;
use aspen_client_api::DeleteResultResponse;
use aspen_client_api::ReadResultResponse;
use aspen_client_api::ScanEntry;
use aspen_client_api::ScanResultResponse;
use aspen_client_api::WriteResultResponse;
use aspen_core::BatchCondition as ApiBatchCondition;
use aspen_core::BatchOperation;
use aspen_core::KeyValueStore;
use aspen_core::KeyValueStoreError;
use aspen_core::ReadRequest;
use aspen_core::ScanRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use aspen_core::validate_client_key;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;

use crate::error_sanitization::sanitize_kv_error;
use crate::verified::bytes_to_string_lossy;

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
                value: bytes_to_string_lossy(&value),
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
                expected: expected.as_ref().map(|v| bytes_to_string_lossy(v)),
                new_value: bytes_to_string_lossy(&new_value),
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
                expected: bytes_to_string_lossy(&expected),
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
                value: bytes_to_string_lossy(value),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_handle_read_key() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ReadKey {
            key: "test".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_write_key() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::WriteKey {
            key: "test".to_string(),
            value: vec![1, 2, 3],
        }));
    }

    #[test]
    fn test_can_handle_delete_key() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::DeleteKey {
            key: "test".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_scan_keys() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ScanKeys {
            prefix: "test".to_string(),
            limit: Some(10),
            continuation_token: None,
        }));
    }

    #[test]
    fn test_can_handle_batch_read() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::BatchRead {
            keys: vec!["a".to_string(), "b".to_string()],
        }));
    }

    #[test]
    fn test_can_handle_batch_write() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::BatchWrite { operations: vec![] }));
    }

    #[test]
    fn test_can_handle_conditional_batch_write() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ConditionalBatchWrite {
            conditions: vec![],
            operations: vec![],
        }));
    }

    #[test]
    fn test_can_handle_compare_and_swap() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::CompareAndSwapKey {
            key: "test".to_string(),
            expected: None,
            new_value: vec![1, 2, 3],
        }));
    }

    #[test]
    fn test_can_handle_compare_and_delete() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::CompareAndDeleteKey {
            key: "test".to_string(),
            expected: vec![1, 2, 3],
        }));
    }

    #[test]
    fn test_rejects_unrelated_requests() {
        let handler = KvHandler;

        // Core requests
        assert!(!handler.can_handle(&ClientRpcRequest::Ping));
        assert!(!handler.can_handle(&ClientRpcRequest::GetHealth));

        // Cluster requests
        assert!(!handler.can_handle(&ClientRpcRequest::InitCluster));
        assert!(!handler.can_handle(&ClientRpcRequest::GetClusterState));

        // Coordination requests
        assert!(!handler.can_handle(&ClientRpcRequest::LockAcquire {
            key: "test".to_string(),
            holder_id: "holder".to_string(),
            ttl_ms: 30000,
            timeout_ms: 0,
        }));
    }

    #[test]
    fn test_handler_name() {
        let handler = KvHandler;
        assert_eq!(handler.name(), "KvHandler");
    }
}
