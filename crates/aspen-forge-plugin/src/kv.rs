//! Safe wrappers around host-provided KV, blob, and crypto functions.

use aspen_wasm_guest_sdk::host;

pub fn kv_get(key: &str) -> Result<Option<Vec<u8>>, String> {
    host::kv_get_value(key)
}

pub fn kv_put(key: &str, value: &[u8]) -> Result<(), String> {
    host::kv_put_value(key, value)
}

pub fn kv_delete(key: &str) -> Result<(), String> {
    host::kv_delete_key(key)
}

pub fn kv_scan(prefix: &str, limit: u32) -> Result<Vec<(String, Vec<u8>)>, String> {
    host::kv_scan_prefix(prefix, limit)
}

pub fn kv_cas(key: &str, expected: &[u8], new_value: &[u8]) -> Result<(), String> {
    host::kv_compare_and_swap(key, expected, new_value)
}

pub fn kv_batch(ops: &[aspen_wasm_guest_sdk::KvBatchOp]) -> Result<(), String> {
    host::kv_batch_write(ops)
}

pub fn blob_get(hash: &str) -> Result<Option<Vec<u8>>, String> {
    host::blob_get_data(hash)
}

pub fn blob_put(data: &[u8]) -> Result<String, String> {
    host::blob_put_data(data)
}

pub fn hlc_now() -> u64 {
    host::hlc_now_ms()
}

pub fn public_key() -> String {
    host::public_key()
}

pub fn sign(data: &[u8]) -> Vec<u8> {
    host::sign_data(data)
}
