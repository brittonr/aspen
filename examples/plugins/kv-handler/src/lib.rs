//! Key-Value handler WASM plugin.
//!
//! Migrated from the native `aspen-kv-handler` crate. Handles all KV
//! operations: ReadKey, WriteKey, DeleteKey, ScanKeys, BatchRead,
//! BatchWrite, ConditionalBatchWrite, CompareAndSwapKey, CompareAndDeleteKey.
//!
//! Uses the `kv_execute` host function for full protocol fidelity:
//! - NOT_LEADER error propagation for client-side leader rotation
//! - CAS failure returns actual value for retry logic
//! - Scan results include version/revision metadata
//! - Conditional batch write with precondition evaluation
//!
//! Client-facing validation (key prefix restrictions, error sanitization)
//! is performed in the plugin. Store operations are delegated to the host.

use aspen_wasm_guest_sdk::host;
use aspen_wasm_guest_sdk::*;

// =============================================================================
// Constants
// =============================================================================

/// Reserved key prefix for internal system keys.
const SYSTEM_KEY_PREFIX: &str = "_system:";

/// Maximum number of keys in a batch operation.
const MAX_BATCH_KEYS: usize = 1000;

// =============================================================================
// Plugin Registration
// =============================================================================

struct KvHandlerPlugin;

impl AspenPlugin for KvHandlerPlugin {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "aspen-kv-handler".to_string(),
            version: "0.1.0".to_string(),
            handles: vec![
                "ReadKey".to_string(),
                "WriteKey".to_string(),
                "DeleteKey".to_string(),
                "ScanKeys".to_string(),
                "BatchRead".to_string(),
                "BatchWrite".to_string(),
                "ConditionalBatchWrite".to_string(),
                "CompareAndSwapKey".to_string(),
                "CompareAndDeleteKey".to_string(),
            ],
            priority: 110,
            app_id: None,
            kv_prefixes: vec!["".to_string()], // Unrestricted KV access
            permissions: PluginPermissions {
                kv_read: true,
                kv_write: true,
                ..PluginPermissions::default()
            },
        }
    }

    fn init() -> Result<(), String> {
        host::log_info_msg("aspen-kv-handler: initialized");
        Ok(())
    }

    fn handle(request: ClientRpcRequest) -> ClientRpcResponse {
        match request {
            ClientRpcRequest::ReadKey { key } => handle_read_key(key),
            ClientRpcRequest::WriteKey { key, value } => handle_write_key(key, value),
            ClientRpcRequest::DeleteKey { key } => handle_delete_key(key),
            ClientRpcRequest::ScanKeys {
                prefix,
                limit,
                continuation_token,
            } => handle_scan_keys(prefix, limit, continuation_token),
            ClientRpcRequest::BatchRead { keys } => handle_batch_read(keys),
            ClientRpcRequest::BatchWrite { operations } => handle_batch_write(operations),
            ClientRpcRequest::ConditionalBatchWrite { conditions, operations } => {
                handle_conditional_batch_write(conditions, operations)
            }
            ClientRpcRequest::CompareAndSwapKey {
                key,
                expected,
                new_value,
            } => handle_compare_and_swap(key, expected, new_value),
            ClientRpcRequest::CompareAndDeleteKey { key, expected } => handle_compare_and_delete(key, expected),
            _ => ClientRpcResponse::Error(response::error_response(
                "UNHANDLED",
                "aspen-kv-handler does not handle this request type",
            )),
        }
    }
}

// =============================================================================
// Key Validation
// =============================================================================

/// Validate a client key. Rejects empty keys and keys with the reserved
/// `_system:` prefix.
fn validate_client_key(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("key cannot be empty".to_string());
    }
    if key.starts_with(SYSTEM_KEY_PREFIX) {
        return Err(format!("key prefix '{}' is reserved for system use", SYSTEM_KEY_PREFIX));
    }
    Ok(())
}

// =============================================================================
// Error Sanitization
// =============================================================================

/// Sanitize an error string for client consumption.
/// Preserves safe error categories but removes internal implementation details.
fn sanitize_error(err: &str) -> String {
    // Preserve known safe patterns
    if err.contains("key not found") {
        return "key not found".to_string();
    }
    if err.contains("key too large") || err.contains("KeyTooLarge") {
        return "key too large".to_string();
    }
    if err.contains("value too large") || err.contains("ValueTooLarge") {
        return "value too large".to_string();
    }
    if err.contains("batch too large") || err.contains("BatchTooLarge") {
        return "batch too large".to_string();
    }
    if err.contains("timed out") || err.contains("Timeout") {
        return "operation timed out".to_string();
    }
    // Default: generic message
    "operation failed".to_string()
}

// =============================================================================
// Base64 Helpers
// =============================================================================

/// Encode bytes to base64 for host function transport.
fn to_b64(bytes: &[u8]) -> String {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((n >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(n & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Decode base64 to bytes.
fn from_b64(input: &str) -> Result<Vec<u8>, String> {
    const DECODE: [u8; 128] = {
        let mut table = [0xFFu8; 128];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0;
        while i < 64 {
            table[chars[i] as usize] = i as u8;
            i += 1;
        }
        table
    };

    let input = input.trim_end_matches('=');
    let mut output = Vec::with_capacity(input.len() * 3 / 4);
    let bytes = input.as_bytes();

    for chunk in bytes.chunks(4) {
        let mut n: u32 = 0;
        for (i, &b) in chunk.iter().enumerate() {
            if b >= 128 || DECODE[b as usize] == 0xFF {
                return Err(format!("invalid base64 character: {}", b as char));
            }
            n |= (DECODE[b as usize] as u32) << (18 - i * 6);
        }
        output.push((n >> 16) as u8);
        if chunk.len() > 2 {
            output.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            output.push(n as u8);
        }
    }
    Ok(output)
}

// =============================================================================
// Request Handlers
// =============================================================================

fn handle_read_key(key: String) -> ClientRpcResponse {
    let result = host::kv_read_full(&key);

    if let Some(ref err) = result.error {
        return ClientRpcResponse::ReadResult(ReadResultResponse {
            value: None,
            was_found: false,
            error: Some(sanitize_error(err)),
        });
    }

    let value = result.value.and_then(|b64| from_b64(&b64).ok());
    ClientRpcResponse::ReadResult(ReadResultResponse {
        value,
        was_found: result.was_found,
        error: None,
    })
}

fn handle_write_key(key: String, value: Vec<u8>) -> ClientRpcResponse {
    if let Err(e) = validate_client_key(&key) {
        return ClientRpcResponse::WriteResult(WriteResultResponse {
            is_success: false,
            error: Some(e),
        });
    }

    let value_b64 = to_b64(&value);
    let result = host::kv_write_full(&key, &value_b64);

    // Propagate NOT_LEADER as top-level error for client leader rotation
    if result.error_code.as_deref() == Some("NOT_LEADER") {
        let msg = result.error.unwrap_or_else(|| "not leader; leader unknown".to_string());
        return ClientRpcResponse::error("NOT_LEADER", msg);
    }

    ClientRpcResponse::WriteResult(WriteResultResponse {
        is_success: result.is_success,
        error: result.error.map(|e| sanitize_error(&e)),
    })
}

fn handle_delete_key(key: String) -> ClientRpcResponse {
    if let Err(e) = validate_client_key(&key) {
        return ClientRpcResponse::DeleteResult(DeleteResultResponse {
            key,
            was_deleted: false,
            error: Some(e),
        });
    }

    let result = host::kv_delete_full(&key);

    if result.error_code.as_deref() == Some("NOT_LEADER") {
        let msg = result.error.unwrap_or_else(|| "not leader; leader unknown".to_string());
        return ClientRpcResponse::error("NOT_LEADER", msg);
    }

    ClientRpcResponse::DeleteResult(DeleteResultResponse {
        key: result.key,
        was_deleted: result.was_deleted,
        error: result.error.map(|e| sanitize_error(&e)),
    })
}

fn handle_scan_keys(prefix: String, limit: Option<u32>, continuation_token: Option<String>) -> ClientRpcResponse {
    let result = host::kv_scan_full(&prefix, limit, continuation_token.as_deref());

    if let Some(ref err) = result.error {
        return ClientRpcResponse::ScanResult(ScanResultResponse {
            entries: vec![],
            count: 0,
            is_truncated: false,
            continuation_token: None,
            error: Some(sanitize_error(err)),
        });
    }

    let entries: Vec<ScanEntry> = result
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

    ClientRpcResponse::ScanResult(ScanResultResponse {
        entries,
        count: result.count,
        is_truncated: result.is_truncated,
        continuation_token: result.continuation_token,
        error: None,
    })
}

fn handle_batch_read(keys: Vec<String>) -> ClientRpcResponse {
    if keys.len() > MAX_BATCH_KEYS {
        return ClientRpcResponse::BatchReadResult(BatchReadResultResponse {
            is_success: false,
            values: None,
            error: Some(format!("batch too large; max {} keys", MAX_BATCH_KEYS)),
        });
    }

    // Validate all keys
    for key in &keys {
        if let Err(e) = validate_client_key(key) {
            return ClientRpcResponse::BatchReadResult(BatchReadResultResponse {
                is_success: false,
                values: None,
                error: Some(e),
            });
        }
    }

    let result = host::kv_batch_read_full(&keys);

    if !result.is_success {
        return ClientRpcResponse::BatchReadResult(BatchReadResultResponse {
            is_success: false,
            values: None,
            error: result.error.map(|e| sanitize_error(&e)),
        });
    }

    // Decode base64 values
    let values: Option<Vec<Option<Vec<u8>>>> =
        result.values.map(|vals| vals.into_iter().map(|v| v.and_then(|b64| from_b64(&b64).ok())).collect());

    ClientRpcResponse::BatchReadResult(BatchReadResultResponse {
        is_success: true,
        values,
        error: None,
    })
}

fn handle_batch_write(operations: Vec<BatchWriteOperation>) -> ClientRpcResponse {
    if operations.len() > MAX_BATCH_KEYS {
        return ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
            is_success: false,
            operations_applied: None,
            error: Some(format!("batch too large; max {} keys", MAX_BATCH_KEYS)),
        });
    }

    // Validate all keys
    for op in &operations {
        let key = match op {
            BatchWriteOperation::Set { key, .. } => key,
            BatchWriteOperation::Delete { key } => key,
        };
        if let Err(e) = validate_client_key(key) {
            return ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
                is_success: false,
                operations_applied: None,
                error: Some(e),
            });
        }
    }

    // Serialize operations for the host function
    let ops_json: Vec<serde_json::Value> = operations
        .iter()
        .map(|op| match op {
            BatchWriteOperation::Set { key, value } => {
                serde_json::json!({"Set": {"key": key, "value": to_b64(value)}})
            }
            BatchWriteOperation::Delete { key } => {
                serde_json::json!({"Delete": {"key": key}})
            }
        })
        .collect();

    let result = host::kv_batch_write_full(&serde_json::Value::Array(ops_json));

    if result.error_code.as_deref() == Some("NOT_LEADER") {
        let msg = result.error.unwrap_or_else(|| "not leader; leader unknown".to_string());
        return ClientRpcResponse::error("NOT_LEADER", msg);
    }

    ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
        is_success: result.is_success,
        operations_applied: result.operations_applied,
        error: result.error.map(|e| sanitize_error(&e)),
    })
}

fn handle_conditional_batch_write(
    conditions: Vec<BatchCondition>,
    operations: Vec<BatchWriteOperation>,
) -> ClientRpcResponse {
    // Validate condition keys
    for cond in &conditions {
        let key = match cond {
            BatchCondition::ValueEquals { key, .. } => key,
            BatchCondition::KeyExists { key } => key,
            BatchCondition::KeyNotExists { key } => key,
        };
        if let Err(e) = validate_client_key(key) {
            return ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
                is_success: false,
                conditions_met: false,
                operations_applied: None,
                failed_condition_index: None,
                failed_condition_reason: Some(e),
                error: None,
            });
        }
    }

    // Validate operation keys
    for op in &operations {
        let key = match op {
            BatchWriteOperation::Set { key, .. } => key,
            BatchWriteOperation::Delete { key } => key,
        };
        if let Err(e) = validate_client_key(key) {
            return ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
                is_success: false,
                conditions_met: false,
                operations_applied: None,
                failed_condition_index: None,
                failed_condition_reason: Some(e),
                error: None,
            });
        }
    }

    // Serialize conditions
    let conditions_json: Vec<serde_json::Value> = conditions
        .iter()
        .map(|c| match c {
            BatchCondition::ValueEquals { key, expected } => {
                serde_json::json!({"ValueEquals": {"key": key, "expected": to_b64(expected)}})
            }
            BatchCondition::KeyExists { key } => {
                serde_json::json!({"KeyExists": {"key": key}})
            }
            BatchCondition::KeyNotExists { key } => {
                serde_json::json!({"KeyNotExists": {"key": key}})
            }
        })
        .collect();

    // Serialize operations
    let ops_json: Vec<serde_json::Value> = operations
        .iter()
        .map(|op| match op {
            BatchWriteOperation::Set { key, value } => {
                serde_json::json!({"Set": {"key": key, "value": to_b64(value)}})
            }
            BatchWriteOperation::Delete { key } => {
                serde_json::json!({"Delete": {"key": key}})
            }
        })
        .collect();

    let result = host::kv_conditional_batch_full(
        &serde_json::Value::Array(conditions_json),
        &serde_json::Value::Array(ops_json),
    );

    if result.error_code.as_deref() == Some("NOT_LEADER") {
        let msg = result.error.unwrap_or_else(|| "not leader; leader unknown".to_string());
        return ClientRpcResponse::error("NOT_LEADER", msg);
    }

    ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
        is_success: result.is_success,
        conditions_met: result.conditions_met,
        operations_applied: result.operations_applied,
        failed_condition_index: result.failed_condition_index,
        failed_condition_reason: result.failed_condition_reason,
        error: result.error.map(|e| sanitize_error(&e)),
    })
}

fn handle_compare_and_swap(key: String, expected: Option<Vec<u8>>, new_value: Vec<u8>) -> ClientRpcResponse {
    if let Err(e) = validate_client_key(&key) {
        return ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
            is_success: false,
            actual_value: None,
            error: Some(e),
        });
    }

    let expected_b64 = expected.as_ref().map(|v| to_b64(v));
    let new_value_b64 = to_b64(&new_value);

    let result = host::kv_cas_full(&key, expected_b64.as_deref(), &new_value_b64);

    if result.error_code.as_deref() == Some("NOT_LEADER") {
        let msg = result.error.unwrap_or_else(|| "not leader; leader unknown".to_string());
        return ClientRpcResponse::error("NOT_LEADER", msg);
    }

    if result.error_code.as_deref() == Some("CAS_FAILED") {
        let actual = result.actual_value.and_then(|b64| from_b64(&b64).ok());
        return ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
            is_success: false,
            actual_value: actual,
            error: None,
        });
    }

    ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
        is_success: result.is_success,
        actual_value: None,
        error: result.error.map(|e| sanitize_error(&e)),
    })
}

fn handle_compare_and_delete(key: String, expected: Vec<u8>) -> ClientRpcResponse {
    if let Err(e) = validate_client_key(&key) {
        return ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
            is_success: false,
            actual_value: None,
            error: Some(e),
        });
    }

    let expected_b64 = to_b64(&expected);
    let result = host::kv_cad_full(&key, &expected_b64);

    if result.error_code.as_deref() == Some("NOT_LEADER") {
        let msg = result.error.unwrap_or_else(|| "not leader; leader unknown".to_string());
        return ClientRpcResponse::error("NOT_LEADER", msg);
    }

    if result.error_code.as_deref() == Some("CAS_FAILED") {
        let actual = result.actual_value.and_then(|b64| from_b64(&b64).ok());
        return ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
            is_success: false,
            actual_value: actual,
            error: None,
        });
    }

    ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
        is_success: result.is_success,
        actual_value: None,
        error: result.error.map(|e| sanitize_error(&e)),
    })
}

register_plugin!(KvHandlerPlugin);
