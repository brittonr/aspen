//! Safe wrappers around host-provided KV and utility functions.
//!
//! These delegate to the guest SDK host functions which call into
//! the Aspen WASM host runtime via primitive-mode FFI.

use aspen_wasm_guest_sdk::host;

/// Read a value from the host KV store.
pub fn kv_get(key: &str) -> Option<Vec<u8>> {
    host::kv_get_value(key)
}

/// Write a value to the host KV store.
pub fn kv_put(key: &str, value: &[u8]) -> Result<(), String> {
    host::kv_put_value(key, value)
}

/// Scan keys by prefix, returning up to `limit` entries.
pub fn kv_scan(prefix: &str, limit: u32) -> Vec<(String, Vec<u8>)> {
    host::kv_scan_prefix(prefix, limit)
}

/// Get the current time in Unix milliseconds from the host.
pub fn now_ms() -> u64 {
    host::current_time_ms()
}
