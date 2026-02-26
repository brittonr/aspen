//! Native KV request handler.
//!
//! Handles the foundational key-value store operations: ReadKey, WriteKey,
//! DeleteKey, ScanKeys, BatchRead, BatchWrite, ConditionalBatchWrite,
//! CompareAndSwapKey, CompareAndDeleteKey, WriteKeyWithLease.
//!
//! KV is too foundational to be a WASM-only plugin — plugin install, plugin
//! reload, and many other system operations depend on KV being available
//! before any plugins are loaded.

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
use aspen_core::error::KeyValueStoreError;
use aspen_core::kv::DeleteRequest;
use aspen_core::kv::ReadConsistency;
use aspen_core::kv::ReadRequest;
use aspen_core::kv::ScanRequest;
use aspen_core::kv::WriteCommand;
use aspen_core::kv::WriteRequest;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;

/// Handler for key-value store operations.
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
                | ClientRpcRequest::CompareAndSwapKey { .. }
                | ClientRpcRequest::CompareAndDeleteKey { .. }
                | ClientRpcRequest::BatchRead { .. }
                | ClientRpcRequest::BatchWrite { .. }
                | ClientRpcRequest::ConditionalBatchWrite { .. }
                | ClientRpcRequest::WriteKeyWithLease { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ReadKey { key } => handle_read(ctx, key).await,
            ClientRpcRequest::WriteKey { key, value } => handle_write(ctx, key, value).await,
            ClientRpcRequest::DeleteKey { key } => handle_delete(ctx, key).await,
            ClientRpcRequest::ScanKeys {
                prefix,
                limit,
                continuation_token,
            } => handle_scan(ctx, prefix, limit, continuation_token).await,
            ClientRpcRequest::CompareAndSwapKey {
                key,
                expected,
                new_value,
            } => handle_cas(ctx, key, expected, new_value).await,
            ClientRpcRequest::CompareAndDeleteKey { key, expected } => handle_cad(ctx, key, expected).await,
            ClientRpcRequest::BatchRead { keys } => handle_batch_read(ctx, keys).await,
            ClientRpcRequest::BatchWrite { operations } => handle_batch_write(ctx, operations).await,
            ClientRpcRequest::ConditionalBatchWrite { conditions, operations } => {
                handle_conditional_batch_write(ctx, conditions, operations).await
            }
            ClientRpcRequest::WriteKeyWithLease { key, value, lease_id } => {
                handle_write_with_lease(ctx, key, value, lease_id).await
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

async fn handle_read(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    let req = ReadRequest {
        key,
        consistency: ReadConsistency::Linearizable,
    };
    match ctx.kv_store.read(req).await {
        Ok(result) => {
            let (value, was_found) = match result.kv {
                Some(kv) => (Some(kv.value.into_bytes()), true),
                None => (None, false),
            };
            Ok(ClientRpcResponse::ReadResult(ReadResultResponse {
                value,
                was_found,
                error: None,
            }))
        }
        // NotFound is not a real error — key simply doesn't exist
        Err(KeyValueStoreError::NotFound { .. }) => Ok(ClientRpcResponse::ReadResult(ReadResultResponse {
            value: None,
            was_found: false,
            error: None,
        })),
        Err(ref e) if is_not_leader_error(e) => Ok(ClientRpcResponse::error("NOT_LEADER", sanitize_kv_error(e))),
        Err(e) => Ok(ClientRpcResponse::ReadResult(ReadResultResponse {
            value: None,
            was_found: false,
            error: Some(sanitize_kv_error(&e)),
        })),
    }
}

async fn handle_write(ctx: &ClientProtocolContext, key: String, value: Vec<u8>) -> anyhow::Result<ClientRpcResponse> {
    let value_str = String::from_utf8_lossy(&value).to_string();
    let req = WriteRequest::set(key, value_str);
    match ctx.kv_store.write(req).await {
        Ok(_) => Ok(ClientRpcResponse::WriteResult(WriteResultResponse {
            is_success: true,
            error: None,
        })),
        Err(ref e) if is_not_leader_error(e) => Ok(ClientRpcResponse::error("NOT_LEADER", sanitize_kv_error(e))),
        Err(e) => Ok(ClientRpcResponse::WriteResult(WriteResultResponse {
            is_success: false,
            error: Some(sanitize_kv_error(&e)),
        })),
    }
}

async fn handle_write_with_lease(
    ctx: &ClientProtocolContext,
    key: String,
    value: Vec<u8>,
    lease_id: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let value_str = String::from_utf8_lossy(&value).to_string();
    let req = WriteRequest {
        command: WriteCommand::SetWithLease {
            key,
            value: value_str,
            lease_id,
        },
    };
    match ctx.kv_store.write(req).await {
        Ok(_) => Ok(ClientRpcResponse::WriteResult(WriteResultResponse {
            is_success: true,
            error: None,
        })),
        Err(ref e) if is_not_leader_error(e) => Ok(ClientRpcResponse::error("NOT_LEADER", sanitize_kv_error(e))),
        Err(e) => Ok(ClientRpcResponse::WriteResult(WriteResultResponse {
            is_success: false,
            error: Some(sanitize_kv_error(&e)),
        })),
    }
}

async fn handle_delete(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    let req = DeleteRequest::new(key.clone());
    match ctx.kv_store.delete(req).await {
        Ok(result) => Ok(ClientRpcResponse::DeleteResult(DeleteResultResponse {
            key,
            was_deleted: result.is_deleted,
            error: None,
        })),
        Err(ref e) if is_not_leader_error(e) => Ok(ClientRpcResponse::error("NOT_LEADER", sanitize_kv_error(e))),
        Err(e) => Ok(ClientRpcResponse::DeleteResult(DeleteResultResponse {
            key,
            was_deleted: false,
            error: Some(sanitize_kv_error(&e)),
        })),
    }
}

async fn handle_scan(
    ctx: &ClientProtocolContext,
    prefix: String,
    limit: Option<u32>,
    continuation_token: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let req = ScanRequest {
        prefix,
        limit_results: limit,
        continuation_token,
    };
    match ctx.kv_store.scan(req).await {
        Ok(result) => {
            let entries: Vec<ScanEntry> = result
                .entries
                .into_iter()
                .map(|kv| ScanEntry {
                    key: kv.key,
                    value: kv.value,
                    version: kv.version,
                    create_revision: kv.create_revision,
                    mod_revision: kv.mod_revision,
                })
                .collect();
            let count = entries.len() as u32;
            Ok(ClientRpcResponse::ScanResult(ScanResultResponse {
                entries,
                count,
                is_truncated: result.is_truncated,
                continuation_token: result.continuation_token,
                error: None,
            }))
        }
        Err(ref e) if is_not_leader_error(e) => Ok(ClientRpcResponse::error("NOT_LEADER", sanitize_kv_error(e))),
        Err(e) => Ok(ClientRpcResponse::ScanResult(ScanResultResponse {
            entries: vec![],
            count: 0,
            is_truncated: false,
            continuation_token: None,
            error: Some(sanitize_kv_error(&e)),
        })),
    }
}

async fn handle_cas(
    ctx: &ClientProtocolContext,
    key: String,
    expected: Option<Vec<u8>>,
    new_value: Vec<u8>,
) -> anyhow::Result<ClientRpcResponse> {
    let expected_str = expected.map(|v| String::from_utf8_lossy(&v).to_string());
    let new_value_str = String::from_utf8_lossy(&new_value).to_string();
    let req = WriteRequest {
        command: WriteCommand::CompareAndSwap {
            key: key.clone(),
            expected: expected_str,
            new_value: new_value_str,
        },
    };
    match ctx.kv_store.write(req).await {
        // CAS succeeded — the condition matched and the value was written
        Ok(_result) => Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
            is_success: true,
            actual_value: None,
            error: None,
        })),
        // CAS condition didn't match — return conflict with actual value
        Err(KeyValueStoreError::CompareAndSwapFailed { actual, .. }) => {
            Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
                is_success: false,
                actual_value: actual.map(|v| v.into_bytes()),
                error: None,
            }))
        }
        Err(ref e) if is_not_leader_error(e) => Ok(ClientRpcResponse::error("NOT_LEADER", sanitize_kv_error(e))),
        // Other errors (timeout, etc.)
        Err(e) => Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
            is_success: false,
            actual_value: None,
            error: Some(sanitize_kv_error(&e)),
        })),
    }
}

async fn handle_cad(ctx: &ClientProtocolContext, key: String, expected: Vec<u8>) -> anyhow::Result<ClientRpcResponse> {
    let expected_str = String::from_utf8_lossy(&expected).to_string();
    // Use atomic CompareAndDelete via Raft — NOT read-then-delete (race condition!)
    let req = WriteRequest {
        command: WriteCommand::CompareAndDelete {
            key: key.clone(),
            expected: expected_str,
        },
    };
    match ctx.kv_store.write(req).await {
        // CAD succeeded — condition matched and key was deleted
        Ok(_result) => Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
            is_success: true,
            actual_value: None,
            error: None,
        })),
        // CAD condition didn't match — return conflict with actual value
        Err(KeyValueStoreError::CompareAndSwapFailed { actual, .. }) => {
            Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
                is_success: false,
                actual_value: actual.map(|v| v.into_bytes()),
                error: None,
            }))
        }
        Err(ref e) if is_not_leader_error(e) => Ok(ClientRpcResponse::error("NOT_LEADER", sanitize_kv_error(e))),
        // Other errors
        Err(e) => Ok(ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
            is_success: false,
            actual_value: None,
            error: Some(sanitize_kv_error(&e)),
        })),
    }
}

async fn handle_batch_read(ctx: &ClientProtocolContext, keys: Vec<String>) -> anyhow::Result<ClientRpcResponse> {
    let mut values = Vec::with_capacity(keys.len());
    for key in &keys {
        let req = ReadRequest {
            key: key.clone(),
            consistency: ReadConsistency::Linearizable,
        };
        let value = match ctx.kv_store.read(req).await {
            Ok(result) => result.kv.map(|kv| kv.value.into_bytes()),
            // NotFound is normal — key doesn't exist, return None for that slot
            Err(KeyValueStoreError::NotFound { .. }) => None,
            // NOT_LEADER on first key means all reads will fail — bail early
            Err(ref e) if is_not_leader_error(e) => {
                return Ok(ClientRpcResponse::error("NOT_LEADER", sanitize_kv_error(e)));
            }
            // Real errors should still surface as None per-key (batch is best-effort per key)
            Err(_) => None,
        };
        values.push(value);
    }
    Ok(ClientRpcResponse::BatchReadResult(BatchReadResultResponse {
        is_success: true,
        values: Some(values),
        error: None,
    }))
}

async fn handle_batch_write(
    ctx: &ClientProtocolContext,
    operations: Vec<BatchWriteOperation>,
) -> anyhow::Result<ClientRpcResponse> {
    let pairs: Vec<(String, String)> = operations
        .iter()
        .filter_map(|op| match op {
            BatchWriteOperation::Set { key, value } => Some((key.clone(), String::from_utf8_lossy(value).to_string())),
            BatchWriteOperation::Delete { .. } => None,
        })
        .collect();
    let delete_keys: Vec<String> = operations
        .iter()
        .filter_map(|op| match op {
            BatchWriteOperation::Delete { key } => Some(key.clone()),
            BatchWriteOperation::Set { .. } => None,
        })
        .collect();

    // Handle sets via SetMulti
    let op_count = operations.len() as u32;
    if !pairs.is_empty() {
        let req = WriteRequest {
            command: WriteCommand::SetMulti { pairs },
        };
        if let Err(e) = ctx.kv_store.write(req).await {
            if is_not_leader_error(&e) {
                return Ok(ClientRpcResponse::error("NOT_LEADER", sanitize_kv_error(&e)));
            }
            return Ok(ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
                is_success: false,
                operations_applied: Some(0),
                error: Some(sanitize_kv_error(&e)),
            }));
        }
    }
    // Handle deletes
    for key in delete_keys {
        let req = DeleteRequest::new(key);
        if let Err(e) = ctx.kv_store.delete(req).await {
            if is_not_leader_error(&e) {
                return Ok(ClientRpcResponse::error("NOT_LEADER", sanitize_kv_error(&e)));
            }
            return Ok(ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
                is_success: false,
                operations_applied: Some(0),
                error: Some(sanitize_kv_error(&e)),
            }));
        }
    }

    Ok(ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
        is_success: true,
        operations_applied: Some(op_count),
        error: None,
    }))
}

async fn handle_conditional_batch_write(
    ctx: &ClientProtocolContext,
    conditions: Vec<BatchCondition>,
    operations: Vec<BatchWriteOperation>,
) -> anyhow::Result<ClientRpcResponse> {
    // Check preconditions
    for (i, condition) in conditions.iter().enumerate() {
        let condition_met = check_condition(ctx, condition).await;
        if !condition_met {
            return Ok(ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
                is_success: false,
                conditions_met: false,
                operations_applied: Some(0),
                failed_condition_index: Some(i as u32),
                failed_condition_reason: Some("condition not met".to_string()),
                error: None,
            }));
        }
    }
    // All conditions met — execute batch
    let pairs: Vec<(String, String)> = operations
        .iter()
        .filter_map(|op| match op {
            BatchWriteOperation::Set { key, value } => Some((key.clone(), String::from_utf8_lossy(value).to_string())),
            BatchWriteOperation::Delete { .. } => None,
        })
        .collect();
    let op_count = operations.len() as u32;
    if !pairs.is_empty() {
        let req = WriteRequest {
            command: WriteCommand::SetMulti { pairs },
        };
        if let Err(e) = ctx.kv_store.write(req).await {
            if is_not_leader_error(&e) {
                return Ok(ClientRpcResponse::error("NOT_LEADER", sanitize_kv_error(&e)));
            }
            return Ok(ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
                is_success: false,
                conditions_met: true,
                operations_applied: Some(0),
                failed_condition_index: None,
                failed_condition_reason: None,
                error: Some(sanitize_kv_error(&e)),
            }));
        }
    }
    Ok(ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
        is_success: true,
        conditions_met: true,
        operations_applied: Some(op_count),
        failed_condition_index: None,
        failed_condition_reason: None,
        error: None,
    }))
}

/// Check a single batch condition against the current store state.
async fn check_condition(ctx: &ClientProtocolContext, condition: &BatchCondition) -> bool {
    match condition {
        BatchCondition::ValueEquals { key, expected } => {
            let req = ReadRequest {
                key: key.clone(),
                consistency: ReadConsistency::Linearizable,
            };
            match ctx.kv_store.read(req).await {
                Ok(result) => result.kv.is_some_and(|kv| kv.value.as_bytes() == expected.as_slice()),
                Err(KeyValueStoreError::NotFound { .. }) => false,
                Err(_) => false,
            }
        }
        BatchCondition::KeyExists { key } => {
            let req = ReadRequest {
                key: key.clone(),
                consistency: ReadConsistency::Linearizable,
            };
            match ctx.kv_store.read(req).await {
                Ok(result) => result.kv.is_some(),
                Err(KeyValueStoreError::NotFound { .. }) => false,
                Err(_) => false,
            }
        }
        BatchCondition::KeyNotExists { key } => {
            let req = ReadRequest {
                key: key.clone(),
                consistency: ReadConsistency::Linearizable,
            };
            match ctx.kv_store.read(req).await {
                Ok(result) => result.kv.is_none(),
                Err(KeyValueStoreError::NotFound { .. }) => true,
                Err(_) => false,
            }
        }
    }
}

use crate::error_utils::is_not_leader_error;
use crate::error_utils::sanitize_kv_error;
