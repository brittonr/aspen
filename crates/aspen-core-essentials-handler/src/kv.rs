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
use aspen_client_api::IndexCreateResultResponse;
use aspen_client_api::IndexDefinitionWire;
use aspen_client_api::IndexDropResultResponse;
use aspen_client_api::IndexListResultResponse;
use aspen_client_api::IndexScanResultResponse;
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
use aspen_core::layer::IndexDefinition;
use aspen_core::layer::IndexFieldType;
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
                | ClientRpcRequest::IndexCreate { .. }
                | ClientRpcRequest::IndexDrop { .. }
                | ClientRpcRequest::IndexScan { .. }
                | ClientRpcRequest::IndexList
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
            ClientRpcRequest::IndexCreate {
                name,
                field,
                field_type,
                is_unique,
                should_index_nulls,
            } => handle_index_create(ctx, name, field, field_type, is_unique, should_index_nulls).await,
            ClientRpcRequest::IndexDrop { name } => handle_index_drop(ctx, name).await,
            ClientRpcRequest::IndexScan {
                index_name,
                mode,
                value,
                end_value,
                limit,
            } => handle_index_scan(ctx, index_name, mode, value, end_value, limit),
            ClientRpcRequest::IndexList => handle_index_list(ctx),
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

// =============================================================================
// Index handler functions
// =============================================================================

async fn handle_index_create(
    ctx: &ClientProtocolContext,
    name: String,
    field: String,
    field_type: String,
    is_unique: bool,
    should_index_nulls: bool,
) -> anyhow::Result<ClientRpcResponse> {
    // Validate name length
    if name.len() > aspen_constants::MAX_INDEX_NAME_SIZE as usize {
        return Ok(ClientRpcResponse::IndexCreateResult(IndexCreateResultResponse {
            is_success: false,
            name,
            error: Some(format!("index name exceeds {} bytes", aspen_constants::MAX_INDEX_NAME_SIZE)),
        }));
    }

    // Reject builtin-reserved names
    let builtins = IndexDefinition::builtins();
    if builtins.iter().any(|b| b.name == name) {
        return Ok(ClientRpcResponse::IndexCreateResult(IndexCreateResultResponse {
            is_success: false,
            name,
            error: Some("cannot create index with built-in name".to_string()),
        }));
    }

    // Parse field type
    let parsed_type = match field_type.as_str() {
        "integer" => IndexFieldType::Integer,
        "unsignedinteger" => IndexFieldType::UnsignedInteger,
        "string" => IndexFieldType::String,
        _ => {
            return Ok(ClientRpcResponse::IndexCreateResult(IndexCreateResultResponse {
                is_success: false,
                name,
                error: Some(format!(
                    "invalid field_type '{}': expected 'integer', 'unsignedinteger', or 'string'",
                    field_type
                )),
            }));
        }
    };

    // Store index definition in KV (persisted via Raft)
    let def = IndexDefinition::custom(&name, &field, parsed_type)
        .with_unique(is_unique)
        .with_index_nulls(should_index_nulls);

    let key = def.system_key();
    let value_json = serde_json::to_string(&def).map_err(|e| anyhow::anyhow!("serialize index definition: {}", e))?;

    let write_req = WriteRequest::set(key, value_json);
    match ctx.kv_store.write(write_req).await {
        Ok(_) => Ok(ClientRpcResponse::IndexCreateResult(IndexCreateResultResponse {
            is_success: true,
            name,
            error: None,
        })),
        Err(ref e) if is_not_leader_error(e) => Ok(ClientRpcResponse::error("NOT_LEADER", sanitize_kv_error(e))),
        Err(e) => Ok(ClientRpcResponse::IndexCreateResult(IndexCreateResultResponse {
            is_success: false,
            name,
            error: Some(sanitize_kv_error(&e)),
        })),
    }
}

async fn handle_index_drop(ctx: &ClientProtocolContext, name: String) -> anyhow::Result<ClientRpcResponse> {
    // Reject dropping builtin indexes
    let builtins = IndexDefinition::builtins();
    if builtins.iter().any(|b| b.name == name) {
        return Ok(ClientRpcResponse::IndexDropResult(IndexDropResultResponse {
            is_success: false,
            name,
            was_dropped: false,
            error: Some("cannot drop built-in index".to_string()),
        }));
    }

    // Delete the index definition from KV using the canonical system_key
    let key = format!("{}{}", aspen_core::layer::INDEX_METADATA_PREFIX, name);
    let delete_req = DeleteRequest { key: key.clone() };
    match ctx.kv_store.delete(delete_req).await {
        Ok(result) => Ok(ClientRpcResponse::IndexDropResult(IndexDropResultResponse {
            is_success: true,
            name,
            was_dropped: result.is_deleted,
            error: None,
        })),
        Err(ref e) if is_not_leader_error(e) => Ok(ClientRpcResponse::error("NOT_LEADER", sanitize_kv_error(e))),
        Err(e) => Ok(ClientRpcResponse::IndexDropResult(IndexDropResultResponse {
            is_success: false,
            name,
            was_dropped: false,
            error: Some(sanitize_kv_error(&e)),
        })),
    }
}

fn handle_index_scan(
    ctx: &ClientProtocolContext,
    index_name: String,
    mode: String,
    value: String,
    end_value: Option<String>,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_core::layer::IndexQueryExecutor;
    use aspen_raft::StateMachineVariant;

    let sm = match &ctx.state_machine {
        Some(StateMachineVariant::Redb(sm)) => sm,
        _ => {
            return Ok(ClientRpcResponse::IndexScanResult(IndexScanResultResponse {
                is_success: false,
                primary_keys: vec![],
                has_more: false,
                count: 0,
                error: Some("index scan requires redb state machine".to_string()),
            }));
        }
    };

    // Decode hex values
    let value_bytes = hex::decode(&value).unwrap_or_else(|_| value.as_bytes().to_vec());
    let scan_limit = limit.unwrap_or(aspen_constants::DEFAULT_SCAN_LIMIT).min(aspen_constants::MAX_SCAN_RESULTS);

    let result = match mode.as_str() {
        "exact" => sm.scan_by_index(&index_name, &value_bytes, scan_limit),
        "range" => {
            let end_bytes = end_value
                .as_deref()
                .map(|e| hex::decode(e).unwrap_or_else(|_| e.as_bytes().to_vec()))
                .unwrap_or_default();
            sm.range_by_index(&index_name, &value_bytes, &end_bytes, scan_limit)
        }
        "lt" => sm.scan_index_lt(&index_name, &value_bytes, scan_limit),
        other => {
            return Ok(ClientRpcResponse::IndexScanResult(IndexScanResultResponse {
                is_success: false,
                primary_keys: vec![],
                has_more: false,
                count: 0,
                error: Some(format!("invalid scan mode '{}': expected 'exact', 'range', or 'lt'", other)),
            }));
        }
    };

    match result {
        Ok(scan_result) => {
            let primary_keys: Vec<String> = scan_result.primary_keys.iter().map(hex::encode).collect();
            let count = primary_keys.len() as u32;
            Ok(ClientRpcResponse::IndexScanResult(IndexScanResultResponse {
                is_success: true,
                primary_keys,
                has_more: scan_result.has_more,
                count,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::IndexScanResult(IndexScanResultResponse {
            is_success: false,
            primary_keys: vec![],
            has_more: false,
            count: 0,
            error: Some(format!("{}", e)),
        })),
    }
}

fn handle_index_list(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    use aspen_raft::StateMachineVariant;

    // Get definitions from the state machine registry if available
    let definitions = match &ctx.state_machine {
        Some(StateMachineVariant::Redb(sm)) => {
            let registry = sm.index_registry();
            registry.definitions()
        }
        _ => IndexDefinition::builtins(),
    };

    let indexes: Vec<IndexDefinitionWire> = definitions
        .iter()
        .map(|def| IndexDefinitionWire {
            name: def.name.clone(),
            field: def.field.clone(),
            field_type: match def.field_type {
                IndexFieldType::Integer => "integer".to_string(),
                IndexFieldType::UnsignedInteger => "unsignedinteger".to_string(),
                IndexFieldType::String => "string".to_string(),
            },
            builtin: def.builtin,
            is_unique: def.options.unique,
            should_index_nulls: def.options.index_nulls,
        })
        .collect();

    let count = indexes.len() as u32;
    Ok(ClientRpcResponse::IndexListResult(IndexListResultResponse {
        is_success: true,
        indexes,
        count,
        error: None,
    }))
}

use crate::error_utils::is_not_leader_error;
use crate::error_utils::sanitize_kv_error;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aspen_rpc_core::test_support::MockEndpointProvider;
    use aspen_rpc_core::test_support::TestContextBuilder;
    use aspen_testing::DeterministicClusterController;
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    async fn setup_test_context() -> ClientProtocolContext {
        let controller = Arc::new(DeterministicClusterController::new());
        let kv_store = Arc::new(DeterministicKeyValueStore::new());
        let mock_endpoint = Arc::new(MockEndpointProvider::with_seed(12345).await);

        TestContextBuilder::new()
            .with_node_id(1)
            .with_controller(controller)
            .with_kv_store(kv_store)
            .with_endpoint_manager(mock_endpoint)
            .with_cookie("test_cluster")
            .build()
    }

    // =========================================================================
    // Routing tests
    // =========================================================================

    #[test]
    fn test_can_handle_kv_operations() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ReadKey { key: "k".to_string() }));
        assert!(handler.can_handle(&ClientRpcRequest::WriteKey {
            key: "k".to_string(),
            value: vec![],
        }));
        assert!(handler.can_handle(&ClientRpcRequest::DeleteKey { key: "k".to_string() }));
        assert!(handler.can_handle(&ClientRpcRequest::ScanKeys {
            prefix: "p".to_string(),
            limit: None,
            continuation_token: None,
        }));
        assert!(handler.can_handle(&ClientRpcRequest::CompareAndSwapKey {
            key: "k".to_string(),
            expected: None,
            new_value: vec![],
        }));
        assert!(handler.can_handle(&ClientRpcRequest::CompareAndDeleteKey {
            key: "k".to_string(),
            expected: vec![],
        }));
        assert!(handler.can_handle(&ClientRpcRequest::BatchRead { keys: vec![] }));
        assert!(handler.can_handle(&ClientRpcRequest::BatchWrite { operations: vec![] }));
        assert!(handler.can_handle(&ClientRpcRequest::ConditionalBatchWrite {
            conditions: vec![],
            operations: vec![],
        }));
    }

    #[test]
    fn test_can_handle_index_operations() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::IndexCreate {
            name: "idx".to_string(),
            field: "f".to_string(),
            field_type: "string".to_string(),
            is_unique: false,
            should_index_nulls: false,
        }));
        assert!(handler.can_handle(&ClientRpcRequest::IndexDrop {
            name: "idx".to_string()
        }));
        assert!(handler.can_handle(&ClientRpcRequest::IndexScan {
            index_name: "idx".to_string(),
            mode: "exact".to_string(),
            value: "v".to_string(),
            end_value: None,
            limit: None,
        }));
        assert!(handler.can_handle(&ClientRpcRequest::IndexList));
    }

    #[test]
    fn test_rejects_unrelated_requests() {
        let handler = KvHandler;
        assert!(!handler.can_handle(&ClientRpcRequest::Ping));
        assert!(!handler.can_handle(&ClientRpcRequest::GetHealth));
        assert!(!handler.can_handle(&ClientRpcRequest::InitCluster));
    }

    #[test]
    fn test_handler_name() {
        let handler = KvHandler;
        assert_eq!(handler.name(), "KvHandler");
    }

    // =========================================================================
    // KV CRUD integration tests
    // =========================================================================

    #[tokio::test]
    async fn test_write_then_read() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        // Write
        let result = handler
            .handle(
                ClientRpcRequest::WriteKey {
                    key: "mykey".to_string(),
                    value: b"myvalue".to_vec(),
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::WriteResult(r) => assert!(r.is_success),
            other => panic!("expected WriteResult, got {:?}", other),
        }

        // Read
        let result = handler
            .handle(
                ClientRpcRequest::ReadKey {
                    key: "mykey".to_string(),
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::ReadResult(r) => {
                assert!(r.was_found);
                assert_eq!(r.value, Some(b"myvalue".to_vec()));
                assert!(r.error.is_none());
            }
            other => panic!("expected ReadResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_read_nonexistent_key() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        let result = handler
            .handle(
                ClientRpcRequest::ReadKey {
                    key: "nosuchkey".to_string(),
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::ReadResult(r) => {
                assert!(!r.was_found);
                assert!(r.value.is_none());
                assert!(r.error.is_none());
            }
            other => panic!("expected ReadResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_delete_key() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        // Write then delete
        handler
            .handle(
                ClientRpcRequest::WriteKey {
                    key: "delme".to_string(),
                    value: b"val".to_vec(),
                },
                &ctx,
            )
            .await
            .unwrap();

        let result = handler
            .handle(
                ClientRpcRequest::DeleteKey {
                    key: "delme".to_string(),
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::DeleteResult(r) => {
                assert!(r.was_deleted);
                assert_eq!(r.key, "delme");
            }
            other => panic!("expected DeleteResult, got {:?}", other),
        }

        // Verify deleted
        let result = handler
            .handle(
                ClientRpcRequest::ReadKey {
                    key: "delme".to_string(),
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::ReadResult(r) => assert!(!r.was_found),
            other => panic!("expected ReadResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_scan_with_prefix() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        // Write several keys
        for key in &["app:a", "app:b", "app:c", "other:x"] {
            handler
                .handle(
                    ClientRpcRequest::WriteKey {
                        key: key.to_string(),
                        value: b"v".to_vec(),
                    },
                    &ctx,
                )
                .await
                .unwrap();
        }

        let result = handler
            .handle(
                ClientRpcRequest::ScanKeys {
                    prefix: "app:".to_string(),
                    limit: None,
                    continuation_token: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::ScanResult(r) => {
                assert_eq!(r.count, 3);
                for entry in &r.entries {
                    assert!(entry.key.starts_with("app:"), "key={} should start with app:", entry.key);
                }
                assert!(r.error.is_none());
            }
            other => panic!("expected ScanResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_scan_with_limit() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        for i in 0..5 {
            handler
                .handle(
                    ClientRpcRequest::WriteKey {
                        key: format!("lim:{:02}", i),
                        value: b"v".to_vec(),
                    },
                    &ctx,
                )
                .await
                .unwrap();
        }

        let result = handler
            .handle(
                ClientRpcRequest::ScanKeys {
                    prefix: "lim:".to_string(),
                    limit: Some(2),
                    continuation_token: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::ScanResult(r) => {
                assert_eq!(r.count, 2);
                assert!(r.is_truncated);
            }
            other => panic!("expected ScanResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_batch_read() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        handler
            .handle(
                ClientRpcRequest::WriteKey {
                    key: "br:a".to_string(),
                    value: b"va".to_vec(),
                },
                &ctx,
            )
            .await
            .unwrap();
        handler
            .handle(
                ClientRpcRequest::WriteKey {
                    key: "br:c".to_string(),
                    value: b"vc".to_vec(),
                },
                &ctx,
            )
            .await
            .unwrap();

        let result = handler
            .handle(
                ClientRpcRequest::BatchRead {
                    keys: vec!["br:a".to_string(), "br:missing".to_string(), "br:c".to_string()],
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::BatchReadResult(r) => {
                assert!(r.is_success);
                let values = r.values.unwrap();
                assert_eq!(values.len(), 3);
                assert_eq!(values[0], Some(b"va".to_vec()));
                assert!(values[1].is_none(), "missing key should be None");
                assert_eq!(values[2], Some(b"vc".to_vec()));
            }
            other => panic!("expected BatchReadResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_batch_write() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        let result = handler
            .handle(
                ClientRpcRequest::BatchWrite {
                    operations: vec![
                        BatchWriteOperation::Set {
                            key: "bw:a".to_string(),
                            value: b"1".to_vec(),
                        },
                        BatchWriteOperation::Set {
                            key: "bw:b".to_string(),
                            value: b"2".to_vec(),
                        },
                    ],
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::BatchWriteResult(r) => {
                assert!(r.is_success);
                assert_eq!(r.operations_applied, Some(2));
            }
            other => panic!("expected BatchWriteResult, got {:?}", other),
        }

        // Verify both written
        let result = handler
            .handle(
                ClientRpcRequest::ReadKey {
                    key: "bw:a".to_string(),
                },
                &ctx,
            )
            .await
            .unwrap();
        match result {
            ClientRpcResponse::ReadResult(r) => assert!(r.was_found),
            other => panic!("expected ReadResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_compare_and_swap_success() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        // Write initial value
        handler
            .handle(
                ClientRpcRequest::WriteKey {
                    key: "cas:k".to_string(),
                    value: b"old".to_vec(),
                },
                &ctx,
            )
            .await
            .unwrap();

        // CAS: old → new
        let result = handler
            .handle(
                ClientRpcRequest::CompareAndSwapKey {
                    key: "cas:k".to_string(),
                    expected: Some(b"old".to_vec()),
                    new_value: b"new".to_vec(),
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::CompareAndSwapResult(r) => {
                assert!(r.is_success);
                assert!(r.error.is_none());
            }
            other => panic!("expected CompareAndSwapResult, got {:?}", other),
        }

        // Verify new value
        let result = handler
            .handle(
                ClientRpcRequest::ReadKey {
                    key: "cas:k".to_string(),
                },
                &ctx,
            )
            .await
            .unwrap();
        match result {
            ClientRpcResponse::ReadResult(r) => {
                assert!(r.was_found);
                assert_eq!(r.value, Some(b"new".to_vec()));
            }
            other => panic!("expected ReadResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_compare_and_swap_conflict() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        handler
            .handle(
                ClientRpcRequest::WriteKey {
                    key: "cas:conflict".to_string(),
                    value: b"actual".to_vec(),
                },
                &ctx,
            )
            .await
            .unwrap();

        // CAS with wrong expected value
        let result = handler
            .handle(
                ClientRpcRequest::CompareAndSwapKey {
                    key: "cas:conflict".to_string(),
                    expected: Some(b"wrong".to_vec()),
                    new_value: b"new".to_vec(),
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::CompareAndSwapResult(r) => {
                assert!(!r.is_success);
            }
            other => panic!("expected CompareAndSwapResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_unhandled_request() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        let result = handler.handle(ClientRpcRequest::Ping, &ctx).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not handled"));
    }

    // =========================================================================
    // Index operation integration tests
    // =========================================================================

    #[tokio::test]
    async fn test_index_create_success() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        let result = handler
            .handle(
                ClientRpcRequest::IndexCreate {
                    name: "idx_user_email".to_string(),
                    field: "email".to_string(),
                    field_type: "string".to_string(),
                    is_unique: true,
                    should_index_nulls: false,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::IndexCreateResult(r) => {
                assert!(r.is_success);
                assert_eq!(r.name, "idx_user_email");
                assert!(r.error.is_none());
            }
            other => panic!("expected IndexCreateResult, got {:?}", other),
        }

        // Verify persisted in KV using the canonical system_key
        let expected_key = format!("{}idx_user_email", aspen_core::layer::INDEX_METADATA_PREFIX);
        let read_result = handler.handle(ClientRpcRequest::ReadKey { key: expected_key }, &ctx).await.unwrap();

        match read_result {
            ClientRpcResponse::ReadResult(r) => {
                assert!(r.was_found, "index definition should be stored in KV");
            }
            other => panic!("expected ReadResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_index_create_builtin_name_rejected() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        // Get a builtin name
        let builtins = IndexDefinition::builtins();
        if builtins.is_empty() {
            return; // No builtins to test against
        }
        let builtin_name = &builtins[0].name;

        let result = handler
            .handle(
                ClientRpcRequest::IndexCreate {
                    name: builtin_name.clone(),
                    field: "f".to_string(),
                    field_type: "string".to_string(),
                    is_unique: false,
                    should_index_nulls: false,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::IndexCreateResult(r) => {
                assert!(!r.is_success);
                assert!(r.error.as_deref().unwrap().contains("built-in"));
            }
            other => panic!("expected IndexCreateResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_index_create_name_too_long() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        let long_name = "x".repeat(aspen_constants::MAX_INDEX_NAME_SIZE as usize + 1);
        let result = handler
            .handle(
                ClientRpcRequest::IndexCreate {
                    name: long_name,
                    field: "f".to_string(),
                    field_type: "string".to_string(),
                    is_unique: false,
                    should_index_nulls: false,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::IndexCreateResult(r) => {
                assert!(!r.is_success);
                assert!(r.error.as_deref().unwrap().contains("exceeds"));
            }
            other => panic!("expected IndexCreateResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_index_create_invalid_field_type() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        let result = handler
            .handle(
                ClientRpcRequest::IndexCreate {
                    name: "idx_bad_type".to_string(),
                    field: "f".to_string(),
                    field_type: "boolean".to_string(),
                    is_unique: false,
                    should_index_nulls: false,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::IndexCreateResult(r) => {
                assert!(!r.is_success);
                assert!(r.error.as_deref().unwrap().contains("invalid field_type"));
            }
            other => panic!("expected IndexCreateResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_index_create_all_field_types() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        for (name, field_type) in &[
            ("idx_int", "integer"),
            ("idx_uint", "unsignedinteger"),
            ("idx_str", "string"),
        ] {
            let result = handler
                .handle(
                    ClientRpcRequest::IndexCreate {
                        name: name.to_string(),
                        field: "f".to_string(),
                        field_type: field_type.to_string(),
                        is_unique: false,
                        should_index_nulls: false,
                    },
                    &ctx,
                )
                .await
                .unwrap();

            match result {
                ClientRpcResponse::IndexCreateResult(r) => {
                    assert!(r.is_success, "field_type '{}' should succeed", field_type);
                }
                other => panic!("expected IndexCreateResult for {}, got {:?}", field_type, other),
            }
        }
    }

    #[tokio::test]
    async fn test_index_drop_success() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        // Create first
        handler
            .handle(
                ClientRpcRequest::IndexCreate {
                    name: "idx_to_drop".to_string(),
                    field: "f".to_string(),
                    field_type: "string".to_string(),
                    is_unique: false,
                    should_index_nulls: false,
                },
                &ctx,
            )
            .await
            .unwrap();

        // Drop
        let result = handler
            .handle(
                ClientRpcRequest::IndexDrop {
                    name: "idx_to_drop".to_string(),
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::IndexDropResult(r) => {
                assert!(r.is_success);
                assert!(r.was_dropped);
                assert_eq!(r.name, "idx_to_drop");
                assert!(r.error.is_none());
            }
            other => panic!("expected IndexDropResult, got {:?}", other),
        }

        // Verify removed from KV using the canonical system_key
        let expected_key = format!("{}idx_to_drop", aspen_core::layer::INDEX_METADATA_PREFIX);
        let read_result = handler.handle(ClientRpcRequest::ReadKey { key: expected_key }, &ctx).await.unwrap();

        match read_result {
            ClientRpcResponse::ReadResult(r) => {
                assert!(!r.was_found, "index should be gone after drop");
            }
            other => panic!("expected ReadResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_index_drop_builtin_rejected() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        let builtins = IndexDefinition::builtins();
        if builtins.is_empty() {
            return;
        }
        let builtin_name = &builtins[0].name;

        let result = handler
            .handle(
                ClientRpcRequest::IndexDrop {
                    name: builtin_name.clone(),
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::IndexDropResult(r) => {
                assert!(!r.is_success);
                assert!(!r.was_dropped);
                assert!(r.error.as_deref().unwrap().contains("built-in"));
            }
            other => panic!("expected IndexDropResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_index_drop_nonexistent() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        let result = handler
            .handle(
                ClientRpcRequest::IndexDrop {
                    name: "no_such_index".to_string(),
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::IndexDropResult(r) => {
                assert!(r.is_success, "drop of nonexistent is not an error");
                // was_dropped may be false since key didn't exist
            }
            other => panic!("expected IndexDropResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_index_scan_no_state_machine() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        // Test context has no redb state machine → should return error
        let result = handler
            .handle(
                ClientRpcRequest::IndexScan {
                    index_name: "idx_test".to_string(),
                    mode: "exact".to_string(),
                    value: hex::encode(b"test"),
                    end_value: None,
                    limit: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::IndexScanResult(r) => {
                assert!(!r.is_success);
                assert!(r.error.as_deref().unwrap().contains("redb state machine"));
            }
            other => panic!("expected IndexScanResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_index_scan_invalid_mode() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        // Without redb state machine, scan with any mode returns the "requires redb" error.
        // The invalid mode error would only surface with a redb state machine present.
        let result = handler
            .handle(
                ClientRpcRequest::IndexScan {
                    index_name: "idx_test".to_string(),
                    mode: "fuzzy".to_string(),
                    value: "v".to_string(),
                    end_value: None,
                    limit: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::IndexScanResult(r) => {
                assert!(!r.is_success);
                assert!(r.error.is_some());
            }
            other => panic!("expected IndexScanResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_index_list_no_state_machine() {
        let ctx = setup_test_context().await;
        let handler = KvHandler;

        // Without redb state machine, should return builtins
        let result = handler.handle(ClientRpcRequest::IndexList, &ctx).await.unwrap();

        match result {
            ClientRpcResponse::IndexListResult(r) => {
                assert!(r.is_success);
                // Should return at least the builtin indexes
                let builtin_count = IndexDefinition::builtins().len() as u32;
                assert_eq!(r.count, builtin_count);
            }
            other => panic!("expected IndexListResult, got {:?}", other),
        }
    }
}
