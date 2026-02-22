//! KV helper wrappers for coordination primitives.
//!
//! All coordination state is stored under `__coord:` prefix.
//! State is JSON-encoded for debuggability.

use aspen_wasm_guest_sdk::host::kv_compare_and_swap;
use aspen_wasm_guest_sdk::host::kv_delete_key;
use aspen_wasm_guest_sdk::host::kv_get_value;
use aspen_wasm_guest_sdk::host::kv_put_value;
use aspen_wasm_guest_sdk::host::kv_scan_prefix;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Maximum CAS retries before giving up.
const MAX_CAS_RETRIES: u32 = 100;

/// Get a JSON-serialized value from the KV store.
pub fn get_json<T: DeserializeOwned>(key: &str) -> Result<Option<T>, String> {
    match kv_get_value(key) {
        Ok(Some(bytes)) => {
            let s = std::str::from_utf8(&bytes).map_err(|e| format!("invalid UTF-8: {e}"))?;
            serde_json::from_str(s).map(Some).map_err(|e| format!("JSON parse error: {e}"))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Get raw bytes as a UTF-8 string.
pub fn get_string(key: &str) -> Result<Option<String>, String> {
    match kv_get_value(key) {
        Ok(Some(bytes)) => {
            let s = std::str::from_utf8(&bytes).map_err(|e| format!("invalid UTF-8: {e}"))?;
            Ok(Some(s.to_string()))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Put a JSON-serialized value.
pub fn put_json<T: Serialize>(key: &str, value: &T) -> Result<(), String> {
    let json = serde_json::to_string(value).map_err(|e| format!("JSON serialize error: {e}"))?;
    kv_put_value(key, json.as_bytes())
}

/// Put a raw string value.
pub fn put_string(key: &str, value: &str) -> Result<(), String> {
    kv_put_value(key, value.as_bytes())
}

/// Delete a key.
pub fn delete(key: &str) -> Result<(), String> {
    kv_delete_key(key)
}

/// Scan keys by prefix.
pub fn scan(prefix: &str, limit: u32) -> Result<Vec<(String, Vec<u8>)>, String> {
    kv_scan_prefix(prefix, limit)
}

/// Compare-and-swap with JSON serialization.
///
/// `expected` is `None` for create-if-absent (empty expected = key must not exist).
/// Returns `Ok(true)` on success, `Ok(false)` on CAS conflict.
pub fn cas_json<T: Serialize>(key: &str, expected: Option<&str>, new_value: &T) -> Result<bool, String> {
    let new_json = serde_json::to_string(new_value).map_err(|e| format!("JSON serialize error: {e}"))?;
    let expected_bytes = expected.map(|s| s.as_bytes()).unwrap_or(b"");
    match kv_compare_and_swap(key, expected_bytes, new_json.as_bytes()) {
        Ok(()) => Ok(true),
        Err(e) if e.contains("CAS") || e.contains("cas") || e.contains("conflict") || e.contains("mismatch") => {
            Ok(false)
        }
        Err(e) => Err(e),
    }
}

/// Compare-and-swap with raw string values.
///
/// `expected` is `None` for create-if-absent.
/// Returns `Ok(true)` on success, `Ok(false)` on CAS conflict.
pub fn cas_string(key: &str, expected: Option<&str>, new_value: &str) -> Result<bool, String> {
    let expected_bytes = expected.map(|s| s.as_bytes()).unwrap_or(b"");
    match kv_compare_and_swap(key, expected_bytes, new_value.as_bytes()) {
        Ok(()) => Ok(true),
        Err(e) if e.contains("CAS") || e.contains("cas") || e.contains("conflict") || e.contains("mismatch") => {
            Ok(false)
        }
        Err(e) => Err(e),
    }
}

/// CAS retry loop for a JSON-encoded value.
///
/// Reads the current value, applies `transform`, writes back with CAS.
/// Retries up to `MAX_CAS_RETRIES` on conflict.
pub fn cas_loop_json<T, F>(key: &str, transform: F) -> Result<T, String>
where
    T: Serialize + DeserializeOwned + Clone,
    F: Fn(Option<T>) -> Result<T, String>,
{
    for _ in 0..MAX_CAS_RETRIES {
        let current_raw = get_string(key)?;
        let current: Option<T> = match &current_raw {
            Some(s) => Some(serde_json::from_str(s).map_err(|e| format!("JSON parse error: {e}"))?),
            None => None,
        };
        let new_val = transform(current)?;
        if cas_json(key, current_raw.as_deref(), &new_val)? {
            return Ok(new_val);
        }
    }
    Err("max CAS retries exceeded".to_string())
}

/// CAS retry loop for a raw string value.
///
/// Reads the current value, applies `transform`, writes back with CAS.
pub fn cas_loop_string<F>(key: &str, transform: F) -> Result<String, String>
where F: Fn(Option<String>) -> Result<String, String> {
    for _ in 0..MAX_CAS_RETRIES {
        let current = get_string(key)?;
        let new_val = transform(current.clone())?;
        if cas_string(key, current.as_deref(), &new_val)? {
            return Ok(new_val);
        }
    }
    Err("max CAS retries exceeded".to_string())
}
