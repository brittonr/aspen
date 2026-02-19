//! KV store helpers for the automerge plugin.

use aspen_wasm_guest_sdk::host;

/// Read a value from the plugin's KV namespace.
pub fn get(key: &str) -> Result<Option<String>, String> {
    match host::kv_get_value(key)? {
        Some(bytes) => {
            let s = String::from_utf8(bytes).map_err(|e| format!("invalid UTF-8 in KV value: {e}"))?;
            Ok(Some(s))
        }
        None => Ok(None),
    }
}

/// Write a value to the plugin's KV namespace.
pub fn put(key: &str, value: &str) -> Result<(), String> {
    host::kv_put_value(key, value.as_bytes())
}

/// Delete a key from the plugin's KV namespace.
pub fn delete(key: &str) -> Result<(), String> {
    host::kv_delete_key(key)
}

/// Scan keys by prefix.
pub fn scan(prefix: &str, limit: u32) -> Result<Vec<(String, String)>, String> {
    let results = host::kv_scan_prefix(prefix, limit)?;
    let mut entries = Vec::with_capacity(results.len());
    for (key, value_bytes) in results {
        let value = String::from_utf8(value_bytes).map_err(|e| format!("invalid UTF-8 in scan value: {e}"))?;
        entries.push((key, value));
    }
    Ok(entries)
}
