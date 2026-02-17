//! WASM guest plugin for the Aspen service registry handler.
//!
//! This crate compiles to `wasm32-wasip2` and exports `handle_request`
//! and `plugin_info` for the Aspen plugin runtime. It reimplements the
//! native `ServiceRegistryHandler` using host-provided KV operations.

mod handlers;
mod kv;
mod types;

/// Host function stubs.
///
/// The actual implementations are provided by the Aspen WASM host runtime
/// via `call_host_function`. This module provides the typed bridge that
/// `kv.rs` and handlers call through.
///
/// In a real hyperlight-wasm guest build, these would use the guest SDK's
/// `call_host_function(name, args)` mechanism. For now they are implemented
/// as no-op stubs so the crate compiles on the native target.
pub(crate) mod host {
    /// Read a value from the host KV store.
    pub fn host_kv_get(_key: &str) -> Option<Vec<u8>> {
        // TODO: Wire up to hyperlight-wasm guest SDK host call
        None
    }

    /// Write a value to the host KV store.
    pub fn host_kv_put(_key: &str, _value: &[u8]) -> Result<(), String> {
        // TODO: Wire up to hyperlight-wasm guest SDK host call
        Err("host_kv_put not wired".to_string())
    }

    /// Delete a key from the host KV store.
    pub fn host_kv_delete(_key: &str) -> Result<(), String> {
        // TODO: Wire up to hyperlight-wasm guest SDK host call
        Err("host_kv_delete not wired".to_string())
    }

    /// Scan keys by prefix.
    pub fn host_kv_scan(_prefix: &str, _limit: u32) -> Vec<(String, Vec<u8>)> {
        // TODO: Wire up to hyperlight-wasm guest SDK host call
        Vec::new()
    }

    /// Get current time in Unix milliseconds.
    pub fn host_now_ms() -> u64 {
        // TODO: Wire up to hyperlight-wasm guest SDK host call
        0
    }
}

/// Dispatch map from operation name to handler.
const HANDLED_OPS: &[&str] = &[
    "ServiceRegister",
    "ServiceDeregister",
    "ServiceDiscover",
    "ServiceList",
    "ServiceGetInstance",
    "ServiceHeartbeat",
    "ServiceUpdateHealth",
    "ServiceUpdateMetadata",
];

/// Entry point called by the host with a JSON-encoded request.
///
/// The input is expected to be a JSON object with a top-level key indicating
/// the operation (e.g., `{"ServiceRegister": {...}}`).
#[unsafe(no_mangle)]
pub extern "C" fn handle_request(input: *const u8, input_len: u32, output: *mut u8, output_len: *mut u32) -> i32 {
    let input_slice = unsafe { core::slice::from_raw_parts(input, input_len as usize) };
    let result = handle_request_inner(input_slice);
    let result_bytes = result.as_bytes();

    // Write output length
    unsafe { *output_len = result_bytes.len() as u32 };

    // Copy result into output buffer
    let out_slice = unsafe { core::slice::from_raw_parts_mut(output, result_bytes.len()) };
    out_slice.copy_from_slice(result_bytes);

    0 // success
}

/// Inner dispatch logic, separated for testability.
fn handle_request_inner(input: &[u8]) -> String {
    let req: serde_json::Value = match serde_json::from_slice(input) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::to_string(&serde_json::json!({
                "error": format!("failed to parse request: {}", e)
            }))
            .unwrap_or_default();
        }
    };

    // The request is an object with a single key naming the operation
    let (op, body) = match req.as_object().and_then(|m| m.iter().next()) {
        Some((k, v)) => (k.as_str(), v),
        None => {
            return serde_json::to_string(&serde_json::json!({
                "error": "request must be a JSON object with an operation key"
            }))
            .unwrap_or_default();
        }
    };

    let response = match op {
        "ServiceRegister" => handlers::handle_register(body),
        "ServiceDeregister" => handlers::handle_deregister(body),
        "ServiceDiscover" => handlers::handle_discover(body),
        "ServiceList" => handlers::handle_list(body),
        "ServiceGetInstance" => handlers::handle_get_instance(body),
        "ServiceHeartbeat" => handlers::handle_heartbeat(body),
        "ServiceUpdateHealth" => handlers::handle_update_health(body),
        "ServiceUpdateMetadata" => handlers::handle_update_metadata(body),
        _ => serde_json::json!({ "error": format!("unknown operation: {}", op) }),
    };

    serde_json::to_string(&response).unwrap_or_default()
}

/// Return plugin metadata as JSON bytes.
#[unsafe(no_mangle)]
pub extern "C" fn plugin_info(output: *mut u8, output_len: *mut u32) -> i32 {
    let info = serde_json::json!({
        "name": "service-registry",
        "version": "0.1.0",
        "handles": HANDLED_OPS,
        "priority": 950
    });

    let bytes = serde_json::to_string(&info).unwrap_or_default();
    let result_bytes = bytes.as_bytes();

    unsafe { *output_len = result_bytes.len() as u32 };
    let out_slice = unsafe { core::slice::from_raw_parts_mut(output, result_bytes.len()) };
    out_slice.copy_from_slice(result_bytes);

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_info_deserializes() {
        let info = serde_json::json!({
            "name": "service-registry",
            "version": "0.1.0",
            "handles": HANDLED_OPS,
            "priority": 950
        });
        assert_eq!(info["name"], "service-registry");
        assert_eq!(info["handles"].as_array().map(|a| a.len()), Some(8));
    }

    #[test]
    fn test_handle_request_unknown_op() {
        let input = br#"{"UnknownOp": {}}"#;
        let result = handle_request_inner(input);
        assert!(result.contains("unknown operation"));
    }

    #[test]
    fn test_handle_request_invalid_json() {
        let input = b"not json";
        let result = handle_request_inner(input);
        assert!(result.contains("failed to parse request"));
    }

    #[test]
    fn test_handle_request_register_stub() {
        // With stubbed host functions, register will fail at kv_put
        let input = br#"{"ServiceRegister": {"service_name": "test", "instance_id": "i1", "address": "127.0.0.1:80", "version": "1.0", "tags": "[]", "weight": 100, "custom_metadata": "{}", "ttl_ms": 30000}}"#;
        let result = handle_request_inner(input);
        // Should return a ServiceRegisterResult (with error since stubs fail)
        assert!(result.contains("ServiceRegisterResult"));
    }
}
