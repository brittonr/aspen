//! KV Counter plugin — distributed counter backed by the KV store.
//!
//! Demonstrates:
//! - KV read/write with namespaced prefixes
//! - Compare-and-swap for safe concurrent increments
//! - Proper error handling patterns
//!
//! ## Usage
//!
//! Write a counter value:
//!   WriteKey { key: "mycounter", value: "increment" }  → increments by 1
//!   WriteKey { key: "mycounter", value: "42" }         → sets to 42
//!
//! Read current value:
//!   ReadKey { key: "mycounter" } → returns current count as string

use aspen_wasm_guest_sdk::AspenPlugin;
use aspen_wasm_guest_sdk::ClientRpcRequest;
use aspen_wasm_guest_sdk::ClientRpcResponse;
use aspen_wasm_guest_sdk::PluginInfo;
use aspen_wasm_guest_sdk::PluginPermissions;
use aspen_wasm_guest_sdk::ReadResultResponse;
use aspen_wasm_guest_sdk::register_plugin;

const KV_PREFIX: &str = "counter:";

struct KvCounter;

impl AspenPlugin for KvCounter {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "kv-counter".to_string(),
            version: "0.1.0".to_string(),
            handles: vec!["WriteKey".to_string(), "ReadKey".to_string()],
            priority: 940,
            app_id: None,
            kv_prefixes: vec![KV_PREFIX.to_string()],
            permissions: PluginPermissions {
                kv_read: true,
                kv_write: true,
                ..PluginPermissions::default()
            },
        }
    }

    fn init() -> Result<(), String> {
        aspen_wasm_guest_sdk::host::log_info_msg("kv-counter: initialized");
        Ok(())
    }

    fn handle(request: ClientRpcRequest) -> ClientRpcResponse {
        match request {
            ClientRpcRequest::WriteKey { ref key, ref value } => {
                let prefixed = format!("{KV_PREFIX}{key}");
                let value_str = String::from_utf8_lossy(value);

                if value_str == "increment" {
                    // CAS-based increment: read current, increment, write back
                    match increment_counter(&prefixed) {
                        Ok(new_val) => {
                            let resp = aspen_wasm_guest_sdk::response::error_response("OK", &format!("{new_val}"));
                            ClientRpcResponse::Error(resp)
                        }
                        Err(e) => ClientRpcResponse::Error(aspen_wasm_guest_sdk::response::error_response(
                            "INCREMENT_FAILED",
                            &e,
                        )),
                    }
                } else {
                    // Direct set
                    match aspen_wasm_guest_sdk::host::kv_put_value(&prefixed, value) {
                        Ok(()) => ClientRpcResponse::Pong, // success
                        Err(e) => {
                            ClientRpcResponse::Error(aspen_wasm_guest_sdk::response::error_response("WRITE_FAILED", &e))
                        }
                    }
                }
            }

            ClientRpcRequest::ReadKey { ref key } => {
                let prefixed = format!("{KV_PREFIX}{key}");
                match aspen_wasm_guest_sdk::host::kv_get_value(&prefixed) {
                    Ok(Some(data)) => ClientRpcResponse::ReadResult(ReadResultResponse {
                        value: Some(data),
                        was_found: true,
                        error: None,
                    }),
                    Ok(None) => ClientRpcResponse::ReadResult(ReadResultResponse {
                        value: Some(b"0".to_vec()),
                        was_found: true,
                        error: None,
                    }),
                    Err(e) => ClientRpcResponse::ReadResult(ReadResultResponse {
                        value: None,
                        was_found: false,
                        error: Some(e),
                    }),
                }
            }

            _ => ClientRpcResponse::Error(aspen_wasm_guest_sdk::response::error_response(
                "UNHANDLED",
                "kv-counter does not handle this request type",
            )),
        }
    }
}

/// Increment a counter using compare-and-swap. Retries up to 5 times on conflict.
fn increment_counter(key: &str) -> Result<u64, String> {
    const MAX_RETRIES: u32 = 5;

    for _ in 0..MAX_RETRIES {
        let current = aspen_wasm_guest_sdk::host::kv_get_value(key).map_err(|e| format!("read failed: {e}"))?;

        let (current_val, expected) = match current {
            Some(ref data) => {
                let s = String::from_utf8_lossy(data);
                let val: u64 = s.parse().unwrap_or(0);
                (val, data.clone())
            }
            None => (0, Vec::new()),
        };

        let new_val = current_val + 1;
        let new_bytes = new_val.to_string().into_bytes();

        match aspen_wasm_guest_sdk::host::kv_compare_and_swap(key, &expected, &new_bytes) {
            Ok(()) => return Ok(new_val),
            Err(_) => continue, // CAS conflict, retry
        }
    }

    Err("CAS conflict after max retries".to_string())
}

register_plugin!(KvCounter);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_info_matches_manifest() {
        let manifest_bytes = include_bytes!("../plugin.json");
        let manifest: PluginInfo = serde_json::from_slice(manifest_bytes).expect("plugin.json should be valid");
        let info = KvCounter::info();
        assert_eq!(info.name, manifest.name);
        assert_eq!(info.handles, manifest.handles);
        assert_eq!(info.priority, manifest.priority);
        assert_eq!(info.kv_prefixes, manifest.kv_prefixes);
    }
}
