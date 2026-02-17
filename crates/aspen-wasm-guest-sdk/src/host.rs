//! Host function imports for WASM guest plugins.
//!
//! These functions call into the host runtime via primitive-mode FFI.
//! The host registers 16 functions that guests can import to interact
//! with the Aspen cluster (logging, KV store, blob storage, cluster info).

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
    fn blob_has(hash: String) -> bool;
    fn blob_get(hash: String) -> Vec<u8>;
    fn blob_put(data: Vec<u8>) -> String;
    fn node_id() -> u64;
    fn random_bytes(count: u32) -> Vec<u8>;
    fn is_leader() -> bool;
    fn leader_id() -> u64;
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
/// Returns `None` if the key does not exist (empty bytes).
pub fn kv_get_value(key: &str) -> Option<Vec<u8>> {
    let result = unsafe { kv_get(key.to_string()) };
    if result.is_empty() { None } else { Some(result) }
}

/// Write a value to the distributed KV store.
/// Returns `Ok(())` on success or `Err(message)` on failure.
pub fn kv_put_value(key: &str, value: &[u8]) -> Result<(), String> {
    let result = unsafe { kv_put(key.to_string(), value.to_vec()) };
    if result.is_empty() { Ok(()) } else { Err(result) }
}

/// Delete a key from the distributed KV store.
/// Returns `Ok(())` on success or `Err(message)` on failure.
pub fn kv_delete_key(key: &str) -> Result<(), String> {
    let result = unsafe { kv_delete(key.to_string()) };
    if result.is_empty() { Ok(()) } else { Err(result) }
}

/// Scan keys by prefix from the distributed KV store.
/// Returns a list of `(key, value)` pairs, JSON-decoded from the host response.
pub fn kv_scan_prefix(prefix: &str, limit: u32) -> Vec<(String, Vec<u8>)> {
    let result = unsafe { kv_scan(prefix.to_string(), limit) };
    if result.is_empty() {
        return Vec::new();
    }
    serde_json::from_slice(&result).unwrap_or_default()
}

/// Compare-and-swap a value in the distributed KV store.
/// Returns `Ok(())` if the swap succeeded or `Err(message)` on failure.
pub fn kv_compare_and_swap(key: &str, expected: &[u8], new_value: &[u8]) -> Result<(), String> {
    let result = unsafe { kv_cas(key.to_string(), expected.to_vec(), new_value.to_vec()) };
    if result.is_empty() { Ok(()) } else { Err(result) }
}

/// Check whether a blob exists in the content-addressed store.
pub fn blob_exists(hash: &str) -> bool {
    unsafe { blob_has(hash.to_string()) }
}

/// Retrieve a blob by hash. Returns `None` if the blob does not exist.
pub fn blob_get_data(hash: &str) -> Option<Vec<u8>> {
    let result = unsafe { blob_get(hash.to_string()) };
    if result.is_empty() { None } else { Some(result) }
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
