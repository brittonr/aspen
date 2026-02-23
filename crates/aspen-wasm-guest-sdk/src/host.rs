//! Host function imports for WASM guest plugins.
//!
//! These functions call into the host runtime via primitive-mode FFI.
//! The host registers functions that guests can import to interact
//! with the Aspen cluster (logging, KV store, blob storage, cluster info,
//! batch operations, and timer scheduling).

// Hyperlight primitive-mode handles the ABI translation for these types.
#[allow(improper_ctypes)]
unsafe extern "C" {
    fn log_info(msg: String);
    fn log_debug(msg: String);
    fn log_warn(msg: String);
    fn now_ms() -> u64;
    fn kv_get(key: String) -> Vec<u8>;
    fn kv_put(key: String, value: Vec<u8>) -> String;
    fn kv_delete(key: String) -> String;
    fn kv_scan(prefix: String, limit: u32) -> Vec<u8>;
    fn kv_cas(key: String, expected: Vec<u8>, new_value: Vec<u8>) -> String;
    fn kv_batch(ops: Vec<u8>) -> String;
    fn blob_has(hash: String) -> bool;
    fn blob_get(hash: String) -> Vec<u8>;
    fn blob_put(data: Vec<u8>) -> String;
    fn node_id() -> u64;
    fn random_bytes(count: u32) -> Vec<u8>;
    fn is_leader() -> bool;
    fn leader_id() -> u64;
    fn sign(data: Vec<u8>) -> Vec<u8>;
    fn verify(key: String, data: Vec<u8>, sig: Vec<u8>) -> bool;
    fn public_key_hex() -> String;
    fn hlc_now() -> u64;
    fn schedule_timer(config: Vec<u8>) -> String;
    fn cancel_timer(name: String) -> String;
    fn hook_subscribe(pattern: String) -> String;
    fn hook_unsubscribe(pattern: String) -> String;
    fn sql_query(request_json: String) -> String;
    fn kv_execute(request_json: String) -> String;
    fn hook_list(unused: String) -> String;
    fn hook_metrics(handler_name: String) -> String;
    fn hook_trigger(request_json: String) -> String;
}

// ---------------------------------------------------------------------------
// Safe wrappers
// ---------------------------------------------------------------------------

/// Log an info-level message on the host.
pub fn log_info_msg(msg: &str) {
    unsafe { log_info(msg.to_string()) }
}

/// Log a debug-level message on the host.
pub fn log_debug_msg(msg: &str) {
    unsafe { log_debug(msg.to_string()) }
}

/// Log a warn-level message on the host.
pub fn log_warn_msg(msg: &str) {
    unsafe { log_warn(msg.to_string()) }
}

/// Get the current wall-clock time in milliseconds from the host.
pub fn current_time_ms() -> u64 {
    unsafe { now_ms() }
}

/// Read a value from the distributed KV store.
///
/// Returns `Ok(Some(data))` if the key exists, `Ok(None)` if not found,
/// or `Err(message)` on error.
///
/// Host encoding: `[0x00] ++ value` = found, `[0x01]` = not-found,
/// `[0x02] ++ error_msg` = error.
pub fn kv_get_value(key: &str) -> Result<Option<Vec<u8>>, String> {
    let result = unsafe { kv_get(key.to_string()) };
    decode_tagged_option_result(&result)
}

/// Write a value to the distributed KV store.
/// Returns `Ok(())` on success or `Err(message)` on failure.
///
/// The host uses the `\0`/`\x01` tag prefix convention:
/// `\0` = success, `\x01` + message = error.
pub fn kv_put_value(key: &str, value: &[u8]) -> Result<(), String> {
    let result = unsafe { kv_put(key.to_string(), value.to_vec()) };
    decode_tagged_unit_result(&result)
}

/// Delete a key from the distributed KV store.
/// Returns `Ok(())` on success or `Err(message)` on failure.
///
/// The host uses the `\0`/`\x01` tag prefix convention.
pub fn kv_delete_key(key: &str) -> Result<(), String> {
    let result = unsafe { kv_delete(key.to_string()) };
    decode_tagged_unit_result(&result)
}

/// Scan keys by prefix from the distributed KV store.
/// Returns a list of `(key, value)` pairs, JSON-decoded from the host response.
///
/// Host encoding: `[0x00] ++ json_bytes` = ok, `[0x01] ++ error_msg` = error.
pub fn kv_scan_prefix(prefix: &str, limit: u32) -> Result<Vec<(String, Vec<u8>)>, String> {
    let result = unsafe { kv_scan(prefix.to_string(), limit) };
    if result.is_empty() {
        return Ok(Vec::new());
    }
    match result[0] {
        0x00 => Ok(serde_json::from_slice(&result[1..]).unwrap_or_default()),
        0x01 => {
            let msg = String::from_utf8_lossy(&result[1..]).to_string();
            Err(msg)
        }
        _ => {
            // Backwards compat: no tag byte, try raw JSON decode
            Ok(serde_json::from_slice(&result).unwrap_or_default())
        }
    }
}

/// Compare-and-swap a value in the distributed KV store.
/// Returns `Ok(())` if the swap succeeded or `Err(message)` on failure.
///
/// The host uses the `\0`/`\x01` tag prefix convention.
pub fn kv_compare_and_swap(key: &str, expected: &[u8], new_value: &[u8]) -> Result<(), String> {
    let result = unsafe { kv_cas(key.to_string(), expected.to_vec(), new_value.to_vec()) };
    decode_tagged_unit_result(&result)
}

/// Check whether a blob exists in the content-addressed store.
pub fn blob_exists(hash: &str) -> bool {
    unsafe { blob_has(hash.to_string()) }
}

/// Retrieve a blob by hash. Returns `Ok(None)` if the blob does not exist,
/// or `Err(message)` on error.
///
/// Host encoding: `[0x00] ++ data` = found, `[0x01]` = not-found,
/// `[0x02] ++ error_msg` = error.
pub fn blob_get_data(hash: &str) -> Result<Option<Vec<u8>>, String> {
    let result = unsafe { blob_get(hash.to_string()) };
    decode_tagged_option_result(&result)
}

/// Store a blob and return its content hash.
/// The host uses a convention where the first byte of the result string
/// signals success (`\0` prefix -> ok, hash follows) or error (`\x01` prefix).
pub fn blob_put_data(data: &[u8]) -> Result<String, String> {
    let result = unsafe { blob_put(data.to_vec()) };
    if let Some(stripped) = result.strip_prefix('\x01') {
        Err(stripped.to_string())
    } else if let Some(stripped) = result.strip_prefix('\0') {
        Ok(stripped.to_string())
    } else {
        // No prefix -- treat entire string as the hash (backwards compat).
        Ok(result)
    }
}

/// Get the numeric node ID of the host node.
pub fn get_node_id() -> u64 {
    unsafe { node_id() }
}

/// Get cryptographically random bytes from the host.
pub fn get_random_bytes(count: u32) -> Vec<u8> {
    unsafe { random_bytes(count) }
}

/// Check whether the host node is currently the Raft leader.
pub fn is_current_leader() -> bool {
    unsafe { is_leader() }
}

/// Get the numeric node ID of the current Raft leader.
pub fn get_leader_id() -> u64 {
    unsafe { leader_id() }
}

/// Sign data with the host node's Ed25519 secret key.
pub fn sign_data(data: &[u8]) -> Vec<u8> {
    unsafe { sign(data.to_vec()) }
}

/// Verify an Ed25519 signature using a hex-encoded public key.
pub fn verify_signature(public_key_hex: &str, data: &[u8], signature: &[u8]) -> bool {
    unsafe { verify(public_key_hex.to_string(), data.to_vec(), signature.to_vec()) }
}

/// Get the host node's Ed25519 public key as a hex string.
pub fn public_key() -> String {
    unsafe { public_key_hex() }
}

/// Get the current HLC timestamp as milliseconds.
pub fn hlc_now_ms() -> u64 {
    unsafe { hlc_now() }
}

// ---------------------------------------------------------------------------
// KV Batch Operations
// ---------------------------------------------------------------------------

/// Execute a batch of KV operations atomically.
///
/// All operations are validated and executed on the host side. Keys are
/// checked against the plugin's namespace prefixes before any operation
/// executes.
///
/// Tiger Style: Batch operations enable atomic multi-key updates,
/// preventing inconsistent intermediate states.
pub fn kv_batch_write(ops: &[aspen_plugin_api::KvBatchOp]) -> Result<(), String> {
    let json = serde_json::to_vec(ops).map_err(|e| format!("failed to serialize batch: {e}"))?;
    let result = unsafe { kv_batch(json) };
    decode_tagged_unit_result(&result)
}

// ---------------------------------------------------------------------------
// Timer / Scheduler
// ---------------------------------------------------------------------------

/// Schedule a timer on the host.
///
/// The host will call the plugin's `on_timer` method when the timer fires.
/// If a timer with the same name already exists, it is replaced.
///
/// Intervals are clamped to \[1s, 24h\]. Maximum 16 active timers per plugin.
pub fn schedule_timer_on_host(config: &aspen_plugin_api::TimerConfig) -> Result<(), String> {
    let json = serde_json::to_vec(config).map_err(|e| format!("failed to serialize timer config: {e}"))?;
    let result = unsafe { schedule_timer(json) };
    decode_tagged_unit_result(&result)
}

/// Cancel a named timer on the host.
pub fn cancel_timer_on_host(name: &str) -> Result<(), String> {
    let result = unsafe { cancel_timer(name.to_string()) };
    decode_tagged_unit_result(&result)
}

// ---------------------------------------------------------------------------
// Hook Event Subscriptions
// ---------------------------------------------------------------------------

/// Subscribe to hook events matching a NATS-style topic pattern.
///
/// The host will call the plugin's `on_hook_event` method when matching
/// events occur. Patterns use dot-delimited segments with wildcards:
///
/// - `hooks.kv.*` — matches `hooks.kv.write_committed`, `hooks.kv.delete_committed`, etc.
/// - `hooks.>` — matches all hook events
/// - `hooks.cluster.*` — matches cluster events (leader_elected, membership_changed, etc.)
///
/// Subscriptions are idempotent — subscribing to the same pattern twice is a no-op.
/// Maximum subscriptions per plugin: `MAX_HOOK_SUBSCRIPTIONS_PER_PLUGIN`.
pub fn subscribe_hook_events(pattern: &str) -> Result<(), String> {
    let result = unsafe { hook_subscribe(pattern.to_string()) };
    decode_tagged_unit_result(&result)
}

/// Unsubscribe from a previously registered hook event pattern.
///
/// The pattern must exactly match a previously subscribed pattern.
pub fn unsubscribe_hook_events(pattern: &str) -> Result<(), String> {
    let result = unsafe { hook_unsubscribe(pattern.to_string()) };
    decode_tagged_unit_result(&result)
}

// ---------------------------------------------------------------------------
// Hook management
// ---------------------------------------------------------------------------

/// Handler info returned by `list_hooks`.
#[derive(serde::Deserialize)]
pub struct HookHandlerInfo {
    /// Handler name.
    pub name: String,
    /// Topic pattern this handler subscribes to.
    pub pattern: String,
    /// Handler type: "in_process", "shell", or "forward".
    pub handler_type: String,
    /// Execution mode: "direct" or "job".
    pub execution_mode: String,
    /// Whether the handler is enabled.
    pub enabled: bool,
    /// Timeout in milliseconds.
    pub timeout_ms: u64,
    /// Number of retries on failure.
    pub retry_count: u32,
}

/// Hook list result from the host.
#[derive(serde::Deserialize)]
pub struct HookListResult {
    /// Whether the hook service is enabled.
    pub is_enabled: bool,
    /// List of configured handlers.
    pub handlers: Vec<HookHandlerInfo>,
}

/// Metrics for a single hook handler.
#[derive(serde::Deserialize)]
pub struct HookHandlerMetricsInfo {
    /// Handler name.
    pub name: String,
    /// Total successful executions.
    pub success_count: u64,
    /// Total failed executions.
    pub failure_count: u64,
    /// Total dropped events.
    pub dropped_count: u64,
    /// Total jobs submitted (for job mode handlers).
    pub jobs_submitted: u64,
    /// Average execution duration in microseconds.
    pub avg_duration_us: u64,
    /// Maximum execution duration in microseconds.
    pub max_duration_us: u64,
}

/// Hook metrics result from the host.
#[derive(serde::Deserialize)]
pub struct HookMetricsResult {
    /// Whether the hook service is enabled.
    pub is_enabled: bool,
    /// Global total events processed.
    pub total_events_processed: u64,
    /// Per-handler metrics.
    pub handlers: Vec<HookHandlerMetricsInfo>,
}

/// Hook trigger result from the host.
#[derive(serde::Deserialize)]
pub struct HookTriggerResult {
    /// Whether the trigger was successful.
    pub is_success: bool,
    /// Number of handlers dispatched to.
    pub dispatched_count: u32,
    /// Error message if failed.
    pub error: Option<String>,
    /// Handler failures (each is [name, error]).
    pub handler_failures: Vec<Vec<String>>,
}

/// List configured hook handlers and their enabled status.
///
/// # Errors
///
/// Returns an error if the `hooks` permission is not granted.
pub fn list_hooks() -> Result<HookListResult, String> {
    let result = unsafe { hook_list(String::new()) };
    decode_tagged_json_result(&result)
}

/// Get execution metrics for hook handlers.
///
/// Pass an empty string to get all handlers, or a handler name to filter.
///
/// # Errors
///
/// Returns an error if the `hooks` permission is not granted.
pub fn get_hook_metrics(handler_name: &str) -> Result<HookMetricsResult, String> {
    let result = unsafe { hook_metrics(handler_name.to_string()) };
    decode_tagged_json_result(&result)
}

/// Manually trigger a hook event.
///
/// # Arguments
///
/// * `event_type` - One of: "write_committed", "delete_committed", "membership_changed",
///   "leader_elected", "snapshot_created"
/// * `payload` - JSON payload for the event
///
/// # Errors
///
/// Returns an error if the `hooks` permission is not granted,
/// or the event type is invalid.
pub fn trigger_hook(event_type: &str, payload: &serde_json::Value) -> Result<HookTriggerResult, String> {
    let request = serde_json::json!({
        "event_type": event_type,
        "payload": payload,
    });
    let request_json = serde_json::to_string(&request).map_err(|e| format!("serialize failed: {e}"))?;
    let result = unsafe { hook_trigger(request_json) };
    decode_tagged_json_result(&result)
}

/// Decode a tagged JSON result string.
///
/// `\0{json}` = success, `\x01{error}` = error.
fn decode_tagged_json_result<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, String> {
    if let Some(json) = s.strip_prefix('\0') {
        serde_json::from_str(json).map_err(|e| format!("parse result failed: {e}"))
    } else if let Some(err) = s.strip_prefix('\x01') {
        Err(err.to_string())
    } else {
        Err("unexpected host response format".to_string())
    }
}

// ---------------------------------------------------------------------------
// SQL query
// ---------------------------------------------------------------------------

/// SQL query result from the host.
#[derive(serde::Deserialize)]
pub struct SqlQueryResult {
    /// Column names.
    pub columns: Vec<String>,
    /// Result rows (each cell is a JSON value).
    pub rows: Vec<Vec<serde_json::Value>>,
    /// Number of rows returned.
    pub row_count: u32,
    /// Whether more rows exist beyond the limit.
    pub is_truncated: bool,
    /// Execution time in milliseconds.
    pub execution_time_ms: u64,
}

/// Execute a read-only SQL query against the node's state machine.
///
/// # Arguments
///
/// * `query` - SQL SELECT or WITH...SELECT query string
/// * `params_json` - JSON-serialized parameter array (empty string for no params)
/// * `consistency` - `"linearizable"` (default) or `"stale"`
/// * `limit` - Maximum rows to return
/// * `timeout_ms` - Query timeout in milliseconds
///
/// # Errors
///
/// Returns an error if SQL is not supported, the query is invalid,
/// or the `sql_query` permission is not granted.
pub fn execute_sql(
    query: &str,
    params_json: &str,
    consistency: &str,
    limit: Option<u32>,
    timeout_ms: Option<u32>,
) -> Result<SqlQueryResult, String> {
    let request = serde_json::json!({
        "query": query,
        "params_json": params_json,
        "consistency": consistency,
        "limit": limit,
        "timeout_ms": timeout_ms,
    });

    let request_json = serde_json::to_string(&request).map_err(|e| format!("failed to serialize SQL request: {e}"))?;

    let result = unsafe { sql_query(request_json) };

    if let Some(json) = result.strip_prefix('\0') {
        serde_json::from_str(json).map_err(|e| format!("failed to parse SQL result: {e}"))
    } else if let Some(msg) = result.strip_prefix('\x01') {
        Err(msg.to_string())
    } else {
        Err(result)
    }
}

// ---------------------------------------------------------------------------
// Full-fidelity KV operations (for handler plugins)
// ---------------------------------------------------------------------------

/// Result from a full-fidelity KV read operation.
#[derive(serde::Deserialize)]
pub struct KvReadResult {
    /// The value bytes (base64-encoded in JSON transport).
    pub value: Option<String>,
    /// Whether the key was found.
    pub was_found: bool,
    /// Error message, if any.
    pub error: Option<String>,
}

/// Result from a full-fidelity KV write operation.
#[derive(serde::Deserialize)]
pub struct KvWriteResult {
    /// Whether the write succeeded.
    pub is_success: bool,
    /// Error message, if any.
    pub error: Option<String>,
    /// Structured error code (e.g., "NOT_LEADER").
    pub error_code: Option<String>,
    /// Leader node ID hint when error_code is NOT_LEADER.
    pub leader_id: Option<u64>,
}

/// Result from a full-fidelity KV delete operation.
#[derive(serde::Deserialize)]
pub struct KvDeleteResult {
    /// The key that was deleted.
    pub key: String,
    /// Whether the key was actually deleted.
    pub was_deleted: bool,
    /// Error message, if any.
    pub error: Option<String>,
    /// Structured error code (e.g., "NOT_LEADER").
    pub error_code: Option<String>,
    /// Leader node ID hint when error_code is NOT_LEADER.
    pub leader_id: Option<u64>,
}

/// A single entry in a full-fidelity scan result.
#[derive(serde::Deserialize)]
pub struct KvScanEntry {
    /// Key name.
    pub key: String,
    /// Value (base64-encoded in JSON transport).
    pub value: String,
    /// Version number.
    pub version: u64,
    /// Revision at which the key was created.
    pub create_revision: u64,
    /// Revision at which the key was last modified.
    pub mod_revision: u64,
}

/// Result from a full-fidelity KV scan operation.
#[derive(serde::Deserialize)]
pub struct KvScanResult {
    /// Matching entries.
    pub entries: Vec<KvScanEntry>,
    /// Total count of entries returned.
    pub count: u32,
    /// Whether more entries exist beyond the limit.
    pub is_truncated: bool,
    /// Opaque token for fetching the next page.
    pub continuation_token: Option<String>,
    /// Error message, if any.
    pub error: Option<String>,
}

/// Result from a full-fidelity KV batch read operation.
#[derive(serde::Deserialize)]
pub struct KvBatchReadResult {
    /// Whether the batch read succeeded.
    pub is_success: bool,
    /// Values for each requested key (None = key not found).
    pub values: Option<Vec<Option<String>>>,
    /// Error message, if any.
    pub error: Option<String>,
}

/// Result from a full-fidelity KV batch write operation.
#[derive(serde::Deserialize)]
pub struct KvBatchWriteResult {
    /// Whether the batch write succeeded.
    pub is_success: bool,
    /// Number of operations applied.
    pub operations_applied: Option<u32>,
    /// Error message, if any.
    pub error: Option<String>,
    /// Structured error code (e.g., "NOT_LEADER").
    pub error_code: Option<String>,
    /// Leader node ID hint when error_code is NOT_LEADER.
    pub leader_id: Option<u64>,
}

/// Result from a full-fidelity KV compare-and-swap operation.
#[derive(serde::Deserialize)]
pub struct KvCasResult {
    /// Whether the CAS succeeded.
    pub is_success: bool,
    /// Actual value on CAS failure (base64-encoded).
    pub actual_value: Option<String>,
    /// Error message, if any.
    pub error: Option<String>,
    /// Structured error code (e.g., "NOT_LEADER", "CAS_FAILED").
    pub error_code: Option<String>,
    /// Leader node ID hint when error_code is NOT_LEADER.
    pub leader_id: Option<u64>,
}

/// Result from a full-fidelity KV conditional batch write.
#[derive(serde::Deserialize)]
pub struct KvConditionalBatchResult {
    /// Whether the conditional batch succeeded.
    pub is_success: bool,
    /// Whether all conditions were met.
    pub conditions_met: bool,
    /// Number of operations applied.
    pub operations_applied: Option<u32>,
    /// Index of the first failed condition.
    pub failed_condition_index: Option<u32>,
    /// Reason the condition failed.
    pub failed_condition_reason: Option<String>,
    /// Error message, if any.
    pub error: Option<String>,
    /// Structured error code (e.g., "NOT_LEADER").
    pub error_code: Option<String>,
    /// Leader node ID hint when error_code is NOT_LEADER.
    pub leader_id: Option<u64>,
}

/// Execute a full-fidelity KV operation via the host.
///
/// This host function provides complete KV protocol support including
/// structured error codes (NOT_LEADER, CAS_FAILED), version metadata
/// in scan results, batch operations, and conditional writes.
///
/// # Arguments
///
/// * `request` - JSON value describing the operation. Must have an "op" field.
///
/// # Supported operations
///
/// - `{"op":"read","key":"..."}`
/// - `{"op":"write","key":"...","value":"base64..."}`
/// - `{"op":"delete","key":"..."}`
/// - `{"op":"scan","prefix":"...","limit":N,"continuation_token":null}`
/// - `{"op":"batch_read","keys":["k1","k2"]}`
/// - `{"op":"batch_write","operations":[{"Set":{"key":"k","value":"base64:v"}},{"Delete":{"key":"k"
///   }}]}`
/// - `{"op":"cas","key":"...","expected":null,"new_value":"base64..."}`
/// - `{"op":"cad","key":"...","expected":"base64..."}`
/// - `{"op":"conditional_batch","conditions":[...],"operations":[...]}`
fn kv_execute_raw(request_json: &str) -> Result<serde_json::Value, String> {
    let result = unsafe { kv_execute(request_json.to_string()) };
    if let Some(json_str) = result.strip_prefix('\0') {
        serde_json::from_str(json_str).map_err(|e| format!("failed to parse kv_execute result: {e}"))
    } else if let Some(err) = result.strip_prefix('\x01') {
        Err(err.to_string())
    } else {
        Err(result)
    }
}

/// Read a key with full result metadata.
pub fn kv_read_full(key: &str) -> KvReadResult {
    let req = serde_json::json!({"op": "read", "key": key});
    match kv_execute_raw(&req.to_string()) {
        Ok(v) => serde_json::from_value(v).unwrap_or(KvReadResult {
            value: None,
            was_found: false,
            error: Some("failed to parse read result".to_string()),
        }),
        Err(e) => KvReadResult {
            value: None,
            was_found: false,
            error: Some(e),
        },
    }
}

/// Write a key with structured error codes (including NOT_LEADER).
pub fn kv_write_full(key: &str, value_b64: &str) -> KvWriteResult {
    let req = serde_json::json!({"op": "write", "key": key, "value": value_b64});
    match kv_execute_raw(&req.to_string()) {
        Ok(v) => serde_json::from_value(v).unwrap_or(KvWriteResult {
            is_success: false,
            error: Some("failed to parse write result".to_string()),
            error_code: None,
            leader_id: None,
        }),
        Err(e) => KvWriteResult {
            is_success: false,
            error: Some(e),
            error_code: None,
            leader_id: None,
        },
    }
}

/// Delete a key with structured error codes (including NOT_LEADER).
pub fn kv_delete_full(key: &str) -> KvDeleteResult {
    let req = serde_json::json!({"op": "delete", "key": key});
    match kv_execute_raw(&req.to_string()) {
        Ok(v) => serde_json::from_value(v).unwrap_or(KvDeleteResult {
            key: key.to_string(),
            was_deleted: false,
            error: Some("failed to parse delete result".to_string()),
            error_code: None,
            leader_id: None,
        }),
        Err(e) => KvDeleteResult {
            key: key.to_string(),
            was_deleted: false,
            error: Some(e),
            error_code: None,
            leader_id: None,
        },
    }
}

/// Scan keys with full metadata (version, revision, continuation token).
pub fn kv_scan_full(prefix: &str, limit: Option<u32>, continuation_token: Option<&str>) -> KvScanResult {
    let req = serde_json::json!({
        "op": "scan",
        "prefix": prefix,
        "limit": limit,
        "continuation_token": continuation_token,
    });
    match kv_execute_raw(&req.to_string()) {
        Ok(v) => serde_json::from_value(v).unwrap_or(KvScanResult {
            entries: vec![],
            count: 0,
            is_truncated: false,
            continuation_token: None,
            error: Some("failed to parse scan result".to_string()),
        }),
        Err(e) => KvScanResult {
            entries: vec![],
            count: 0,
            is_truncated: false,
            continuation_token: None,
            error: Some(e),
        },
    }
}

/// Batch read multiple keys.
pub fn kv_batch_read_full(keys: &[String]) -> KvBatchReadResult {
    let req = serde_json::json!({"op": "batch_read", "keys": keys});
    match kv_execute_raw(&req.to_string()) {
        Ok(v) => serde_json::from_value(v).unwrap_or(KvBatchReadResult {
            is_success: false,
            values: None,
            error: Some("failed to parse batch read result".to_string()),
        }),
        Err(e) => KvBatchReadResult {
            is_success: false,
            values: None,
            error: Some(e),
        },
    }
}

/// Batch write with atomic semantics and structured error codes.
pub fn kv_batch_write_full(operations: &serde_json::Value) -> KvBatchWriteResult {
    let req = serde_json::json!({"op": "batch_write", "operations": operations});
    match kv_execute_raw(&req.to_string()) {
        Ok(v) => serde_json::from_value(v).unwrap_or(KvBatchWriteResult {
            is_success: false,
            operations_applied: None,
            error: Some("failed to parse batch write result".to_string()),
            error_code: None,
            leader_id: None,
        }),
        Err(e) => KvBatchWriteResult {
            is_success: false,
            operations_applied: None,
            error: Some(e),
            error_code: None,
            leader_id: None,
        },
    }
}

/// Compare-and-swap with actual value on failure.
pub fn kv_cas_full(key: &str, expected: Option<&str>, new_value: &str) -> KvCasResult {
    let req = serde_json::json!({
        "op": "cas",
        "key": key,
        "expected": expected,
        "new_value": new_value,
    });
    match kv_execute_raw(&req.to_string()) {
        Ok(v) => serde_json::from_value(v).unwrap_or(KvCasResult {
            is_success: false,
            actual_value: None,
            error: Some("failed to parse CAS result".to_string()),
            error_code: None,
            leader_id: None,
        }),
        Err(e) => KvCasResult {
            is_success: false,
            actual_value: None,
            error: Some(e),
            error_code: None,
            leader_id: None,
        },
    }
}

/// Compare-and-delete with actual value on failure.
pub fn kv_cad_full(key: &str, expected: &str) -> KvCasResult {
    let req = serde_json::json!({
        "op": "cad",
        "key": key,
        "expected": expected,
    });
    match kv_execute_raw(&req.to_string()) {
        Ok(v) => serde_json::from_value(v).unwrap_or(KvCasResult {
            is_success: false,
            actual_value: None,
            error: Some("failed to parse CAD result".to_string()),
            error_code: None,
            leader_id: None,
        }),
        Err(e) => KvCasResult {
            is_success: false,
            actual_value: None,
            error: Some(e),
            error_code: None,
            leader_id: None,
        },
    }
}

/// Conditional batch write with condition evaluation and structured errors.
pub fn kv_conditional_batch_full(
    conditions: &serde_json::Value,
    operations: &serde_json::Value,
) -> KvConditionalBatchResult {
    let req = serde_json::json!({
        "op": "conditional_batch",
        "conditions": conditions,
        "operations": operations,
    });
    match kv_execute_raw(&req.to_string()) {
        Ok(v) => serde_json::from_value(v).unwrap_or(KvConditionalBatchResult {
            is_success: false,
            conditions_met: false,
            operations_applied: None,
            failed_condition_index: None,
            failed_condition_reason: None,
            error: Some("failed to parse conditional batch result".to_string()),
            error_code: None,
            leader_id: None,
        }),
        Err(e) => KvConditionalBatchResult {
            is_success: false,
            conditions_met: false,
            operations_applied: None,
            failed_condition_index: None,
            failed_condition_reason: None,
            error: Some(e),
            error_code: None,
            leader_id: None,
        },
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Decode a tagged `Result<Option<Vec<u8>>, String>` from a host function.
///
/// The host encodes results as:
/// - `[0x00]` + data = found (returns `Ok(Some(data))`)
/// - `[0x01]` = not found (returns `Ok(None)`)
/// - `[0x02]` + error message = error (returns `Err(message)`)
/// - Empty vec = not found (backwards compatibility)
///
/// Tiger Style: All option result decoding goes through one function.
fn decode_tagged_option_result(result: &[u8]) -> Result<Option<Vec<u8>>, String> {
    if result.is_empty() {
        return Ok(None);
    }
    match result[0] {
        0x00 => Ok(Some(result[1..].to_vec())),
        0x01 => Ok(None),
        0x02 => {
            let msg = String::from_utf8_lossy(&result[1..]).to_string();
            Err(msg)
        }
        _ => {
            // Backwards compat: no tag byte, treat entire vec as data
            Ok(Some(result.to_vec()))
        }
    }
}

/// Decode a tagged `Result<(), String>` from a host function.
///
/// The host encodes results as:
/// - `\0` (or `\0` + ignored payload) = success
/// - `\x01` + message = error
/// - Empty string = success (backwards compatibility)
///
/// Tiger Style: All result decoding goes through one function.
fn decode_tagged_unit_result(result: &str) -> Result<(), String> {
    if result.is_empty() || result.starts_with('\0') {
        Ok(())
    } else if let Some(msg) = result.strip_prefix('\x01') {
        Err(msg.to_string())
    } else {
        // No tag prefix — treat entire string as error message (backwards compat)
        Err(result.to_string())
    }
}
