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

use crate::context::ClientProtocolContext;
use crate::error_sanitization::sanitize_kv_error;
use crate::registry::RequestHandler;
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
    use std::sync::Arc;

    use aspen_client_api::BatchWriteOperation;
    use aspen_core::inmemory::DeterministicClusterController;
    use aspen_core::inmemory::DeterministicKeyValueStore;

    use super::*;
    use crate::context::test_support::TestContextBuilder;
    use crate::test_mocks::MockEndpointProvider;
    #[cfg(feature = "sql")]
    use crate::test_mocks::mock_sql_executor;

    async fn setup_test_context() -> ClientProtocolContext {
        let controller = Arc::new(DeterministicClusterController::new());
        let kv_store = Arc::new(DeterministicKeyValueStore::new());
        let mock_endpoint = Arc::new(MockEndpointProvider::with_seed(12345).await);

        let builder = TestContextBuilder::new()
            .with_node_id(1)
            .with_controller(controller)
            .with_kv_store(kv_store)
            .with_endpoint_manager(mock_endpoint)
            .with_cookie("test_cluster");

        #[cfg(feature = "sql")]
        let builder = builder.with_sql_executor(mock_sql_executor());

        builder.build()
    }

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

    #[tokio::test]
    async fn test_handle_write_then_read() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        // Write a key
        let write_request = ClientRpcRequest::WriteKey {
            key: "test_key".to_string(),
            value: b"test_value".to_vec(),
        };

        let result = handler.handle(write_request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::WriteResult(response) => {
                assert!(response.success);
                assert!(response.error.is_none());
            }
            other => panic!("expected WriteResult, got {:?}", other),
        }

        // Read the key back
        let read_request = ClientRpcRequest::ReadKey {
            key: "test_key".to_string(),
        };

        let result = handler.handle(read_request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::ReadResult(response) => {
                assert!(response.found);
                assert!(response.error.is_none());
                assert_eq!(response.value, Some(b"test_value".to_vec()));
            }
            other => panic!("expected ReadResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_read_nonexistent_key() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        let request = ClientRpcRequest::ReadKey {
            key: "nonexistent".to_string(),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::ReadResult(response) => {
                assert!(!response.found);
                assert!(response.value.is_none());
                // NotFound is not an error, just means key doesn't exist
                assert!(response.error.is_none());
            }
            other => panic!("expected ReadResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_delete_key() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        // First write a key
        let write_request = ClientRpcRequest::WriteKey {
            key: "to_delete".to_string(),
            value: b"value".to_vec(),
        };
        let _ = handler.handle(write_request, &ctx).await;

        // Delete the key
        let delete_request = ClientRpcRequest::DeleteKey {
            key: "to_delete".to_string(),
        };

        let result = handler.handle(delete_request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::DeleteResult(response) => {
                assert!(response.deleted);
                assert_eq!(response.key, "to_delete");
                assert!(response.error.is_none());
            }
            other => panic!("expected DeleteResult, got {:?}", other),
        }

        // Verify the key is gone
        let read_request = ClientRpcRequest::ReadKey {
            key: "to_delete".to_string(),
        };

        let result = handler.handle(read_request, &ctx).await;
        match result.unwrap() {
            ClientRpcResponse::ReadResult(response) => {
                assert!(!response.found);
            }
            other => panic!("expected ReadResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_reserved_prefix_rejected() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        // Try to write to reserved prefix
        let request = ClientRpcRequest::WriteKey {
            key: "_system:internal".to_string(),
            value: b"value".to_vec(),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::WriteResult(response) => {
                assert!(!response.success);
                assert!(response.error.is_some());
            }
            other => panic!("expected WriteResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_scan_keys() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        // Write some keys
        for i in 0..5 {
            let request = ClientRpcRequest::WriteKey {
                key: format!("scan_test_{}", i),
                value: format!("value_{}", i).into_bytes(),
            };
            let _ = handler.handle(request, &ctx).await;
        }

        // Scan with prefix
        let scan_request = ClientRpcRequest::ScanKeys {
            prefix: "scan_test_".to_string(),
            limit: Some(10),
            continuation_token: None,
        };

        let result = handler.handle(scan_request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::ScanResult(response) => {
                assert!(response.error.is_none());
                assert_eq!(response.entries.len(), 5);
            }
            other => panic!("expected ScanResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_batch_write() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        let request = ClientRpcRequest::BatchWrite {
            operations: vec![
                BatchWriteOperation::Set {
                    key: "batch_a".to_string(),
                    value: b"value_a".to_vec(),
                },
                BatchWriteOperation::Set {
                    key: "batch_b".to_string(),
                    value: b"value_b".to_vec(),
                },
            ],
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::BatchWriteResult(response) => {
                assert!(response.success);
                assert!(response.error.is_none());
            }
            other => panic!("expected BatchWriteResult, got {:?}", other),
        }

        // Verify keys were written
        let read_a = handler
            .handle(
                ClientRpcRequest::ReadKey {
                    key: "batch_a".to_string(),
                },
                &ctx,
            )
            .await;

        match read_a.unwrap() {
            ClientRpcResponse::ReadResult(response) => {
                assert!(response.found);
                assert_eq!(response.value, Some(b"value_a".to_vec()));
            }
            other => panic!("expected ReadResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_batch_read() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        // Write some keys
        let _ = handler
            .handle(
                ClientRpcRequest::WriteKey {
                    key: "batch_read_a".to_string(),
                    value: b"a".to_vec(),
                },
                &ctx,
            )
            .await;

        let _ = handler
            .handle(
                ClientRpcRequest::WriteKey {
                    key: "batch_read_b".to_string(),
                    value: b"b".to_vec(),
                },
                &ctx,
            )
            .await;

        // Batch read
        let request = ClientRpcRequest::BatchRead {
            keys: vec![
                "batch_read_a".to_string(),
                "batch_read_nonexistent".to_string(),
                "batch_read_b".to_string(),
            ],
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::BatchReadResult(response) => {
                assert!(response.success);
                let values = response.values.unwrap();
                assert_eq!(values.len(), 3);
                assert_eq!(values[0], Some(b"a".to_vec()));
                assert_eq!(values[1], None); // nonexistent key
                assert_eq!(values[2], Some(b"b".to_vec()));
            }
            other => panic!("expected BatchReadResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_compare_and_swap_success() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        // CAS on nonexistent key (expected=None)
        let request = ClientRpcRequest::CompareAndSwapKey {
            key: "cas_key".to_string(),
            expected: None,
            new_value: b"new_value".to_vec(),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::CompareAndSwapResult(response) => {
                assert!(response.success);
                assert!(response.error.is_none());
            }
            other => panic!("expected CompareAndSwapResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_compare_and_swap_failure() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        // Write initial value
        let _ = handler
            .handle(
                ClientRpcRequest::WriteKey {
                    key: "cas_fail_key".to_string(),
                    value: b"initial".to_vec(),
                },
                &ctx,
            )
            .await;

        // CAS with wrong expected value
        let request = ClientRpcRequest::CompareAndSwapKey {
            key: "cas_fail_key".to_string(),
            expected: Some(b"wrong_expected".to_vec()),
            new_value: b"new_value".to_vec(),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::CompareAndSwapResult(response) => {
                assert!(!response.success);
                // actual_value should contain the real value
                assert_eq!(response.actual_value, Some(b"initial".to_vec()));
            }
            other => panic!("expected CompareAndSwapResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_unhandled_request() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        // This request is not handled by KvHandler
        let request = ClientRpcRequest::Ping;

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not handled"));
    }
}
