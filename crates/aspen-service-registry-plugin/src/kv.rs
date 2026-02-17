//! Safe wrappers around host-provided KV and utility functions.
//!
//! These call through to the host FFI. The actual FFI bridge is defined
//! in `lib.rs`; this module provides typed helpers the handlers use.

use crate::host;

/// Read a value from the host KV store.
pub fn kv_get(key: &str) -> Option<Vec<u8>> {
    host::host_kv_get(key)
}

/// Write a value to the host KV store.
pub fn kv_put(key: &str, value: &[u8]) -> Result<(), String> {
    host::host_kv_put(key, value)
}

/// Delete a key from the host KV store.
pub fn kv_delete(key: &str) -> Result<(), String> {
    host::host_kv_delete(key)
}

/// Scan keys by prefix, returning up to `limit` entries.
pub fn kv_scan(prefix: &str, limit: u32) -> Vec<(String, Vec<u8>)> {
    host::host_kv_scan(prefix, limit)
}

/// Get the current time in Unix milliseconds from the host.
pub fn now_ms() -> u64 {
    host::host_now_ms()
}
