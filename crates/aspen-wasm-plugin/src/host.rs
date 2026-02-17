//! Host function bindings for WASM handler plugins.
//!
//! Registers host functions on a `ProtoWasmSandbox` in primitive mode.
//! Extends the base host functions (logging, clock, kv-store, blob-store)
//! with identity, randomness, and cluster-state queries.
//!
//! ## Primitive Mode Type Encoding
//!
//! Only `String`, `i32`/`u32`/`i64`/`u64`, `f32`/`f64`, `bool`, and
//! `Vec<u8>` are supported. Complex types are encoded as follows:
//!
//! - `Option<Vec<u8>>` -> empty `Vec<u8>` for `None` (guest checks `.is_empty()`)
//! - `Result<(), String>` -> `String` (empty = success, non-empty = error)
//! - `Vec<(String, Vec<u8>)>` -> JSON-serialized `Vec<u8>`
//! - `Result<String, String>` -> `String` with `\0` prefix for ok, `\x01` for err

use std::sync::Arc;

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;
use aspen_traits::ClusterController;

/// Host context for WASM handler plugin callbacks.
///
/// Holds references to cluster services that the guest can interact with
/// through registered host functions.
pub struct PluginHostContext {
    /// KV store for guest key-value operations.
    pub kv_store: Arc<dyn KeyValueStore>,
    /// Blob store for guest blob operations.
    pub blob_store: Arc<dyn BlobStore>,
    /// Cluster controller for leader queries.
    pub controller: Arc<dyn ClusterController>,
    /// Node ID of the host node.
    pub node_id: u64,
    /// Plugin name for structured log context.
    pub plugin_name: String,
}

impl PluginHostContext {
    /// Create a new host context for a WASM handler plugin.
    pub fn new(
        kv_store: Arc<dyn KeyValueStore>,
        blob_store: Arc<dyn BlobStore>,
        controller: Arc<dyn ClusterController>,
        node_id: u64,
        plugin_name: String,
    ) -> Self {
        Self {
            kv_store,
            blob_store,
            controller,
            node_id,
            plugin_name,
        }
    }
}

// ---------------------------------------------------------------------------
// Logging host functions
// ---------------------------------------------------------------------------

/// Log an informational message from a WASM plugin.
pub fn log_info(plugin_name: &str, message: &str) {
    tracing::info!(plugin = plugin_name, guest_message = %message, "wasm plugin log");
}

/// Log a debug message from a WASM plugin.
pub fn log_debug(plugin_name: &str, message: &str) {
    tracing::debug!(plugin = plugin_name, guest_message = %message, "wasm plugin log");
}

/// Log a warning message from a WASM plugin.
pub fn log_warn(plugin_name: &str, message: &str) {
    tracing::warn!(plugin = plugin_name, guest_message = %message, "wasm plugin log");
}

// ---------------------------------------------------------------------------
// Clock host function
// ---------------------------------------------------------------------------

/// Return the current wall-clock time as milliseconds since the Unix epoch.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

// ---------------------------------------------------------------------------
// KV Store host functions
// ---------------------------------------------------------------------------

/// Get a value by key from the distributed KV store.
///
/// Returns `None` if the key does not exist.
pub fn kv_get(ctx: &PluginHostContext, key: &str) -> Option<Vec<u8>> {
    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        let request = aspen_kv_types::ReadRequest::new(key);
        match ctx.kv_store.read(request).await {
            Ok(result) => result.kv.map(|entry| entry.value.into_bytes()),
            Err(e) => {
                tracing::warn!(
                    plugin = %ctx.plugin_name,
                    key,
                    error = %e,
                    "wasm plugin kv_get failed"
                );
                None
            }
        }
    })
}

/// Put a key-value pair into the distributed KV store.
///
/// The value bytes must be valid UTF-8.
pub fn kv_put(ctx: &PluginHostContext, key: &str, value: &[u8]) -> Result<(), String> {
    let value_str = std::str::from_utf8(value).map_err(|e| format!("value is not valid UTF-8: {e}"))?;

    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        let request = aspen_kv_types::WriteRequest::set(key, value_str);
        ctx.kv_store.write(request).await.map(|_| ()).map_err(|e| {
            tracing::warn!(
                plugin = %ctx.plugin_name,
                key,
                error = %e,
                "wasm plugin kv_put failed"
            );
            format!("kv_put failed: {e}")
        })
    })
}

/// Delete a key from the distributed KV store.
pub fn kv_delete(ctx: &PluginHostContext, key: &str) -> Result<(), String> {
    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        let request = aspen_kv_types::DeleteRequest::new(key);
        ctx.kv_store.delete(request).await.map(|_| ()).map_err(|e| {
            tracing::warn!(
                plugin = %ctx.plugin_name,
                key,
                error = %e,
                "wasm plugin kv_delete failed"
            );
            format!("kv_delete failed: {e}")
        })
    })
}

/// Scan keys matching a prefix from the distributed KV store.
///
/// Returns a list of `(key, value_bytes)` pairs. A `limit` of 0 means no limit.
pub fn kv_scan(ctx: &PluginHostContext, prefix: &str, limit: u32) -> Vec<(String, Vec<u8>)> {
    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        let request = aspen_kv_types::ScanRequest {
            prefix: prefix.to_string(),
            limit_results: if limit == 0 { None } else { Some(limit) },
            continuation_token: None,
        };
        match ctx.kv_store.scan(request).await {
            Ok(result) => result.entries.into_iter().map(|entry| (entry.key, entry.value.into_bytes())).collect(),
            Err(e) => {
                tracing::warn!(
                    plugin = %ctx.plugin_name,
                    prefix,
                    error = %e,
                    "wasm plugin kv_scan failed"
                );
                Vec::new()
            }
        }
    })
}

/// Compare-and-swap a key in the distributed KV store.
///
/// If `expected` is empty, the key must not exist (create-if-absent).
/// Both `expected` and `new_value` must be valid UTF-8.
pub fn kv_cas(ctx: &PluginHostContext, key: &str, expected: &[u8], new_value: &[u8]) -> Result<(), String> {
    let expected_str = if expected.is_empty() {
        None
    } else {
        Some(std::str::from_utf8(expected).map_err(|e| format!("expected is not valid UTF-8: {e}"))?.to_string())
    };
    let new_value_str = std::str::from_utf8(new_value).map_err(|e| format!("new_value is not valid UTF-8: {e}"))?;

    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        let request = aspen_kv_types::WriteRequest::compare_and_swap(key, expected_str, new_value_str);
        ctx.kv_store.write(request).await.map(|_| ()).map_err(|e| {
            tracing::warn!(
                plugin = %ctx.plugin_name,
                key,
                error = %e,
                "wasm plugin kv_cas failed"
            );
            format!("kv_cas failed: {e}")
        })
    })
}

// ---------------------------------------------------------------------------
// Blob Store host functions
// ---------------------------------------------------------------------------

/// Check whether a blob exists in the store.
///
/// The `hash` parameter is the hex-encoded BLAKE3 hash of the blob.
pub fn blob_has(ctx: &PluginHostContext, hash: &str) -> bool {
    let blob_hash = match hash.parse::<iroh_blobs::Hash>() {
        Ok(h) => h,
        Err(e) => {
            tracing::warn!(
                plugin = %ctx.plugin_name,
                hash,
                error = %e,
                "wasm plugin blob_has: invalid hash"
            );
            return false;
        }
    };

    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        match ctx.blob_store.has(&blob_hash).await {
            Ok(exists) => exists,
            Err(e) => {
                tracing::warn!(
                    plugin = %ctx.plugin_name,
                    hash,
                    error = %e,
                    "wasm plugin blob_has failed"
                );
                false
            }
        }
    })
}

/// Retrieve blob bytes by hash.
///
/// The `hash` parameter is the hex-encoded BLAKE3 hash.
/// Returns `None` if the blob does not exist.
pub fn blob_get(ctx: &PluginHostContext, hash: &str) -> Option<Vec<u8>> {
    let blob_hash = match hash.parse::<iroh_blobs::Hash>() {
        Ok(h) => h,
        Err(e) => {
            tracing::warn!(
                plugin = %ctx.plugin_name,
                hash,
                error = %e,
                "wasm plugin blob_get: invalid hash"
            );
            return None;
        }
    };

    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        match ctx.blob_store.get_bytes(&blob_hash).await {
            Ok(Some(bytes)) => Some(bytes.to_vec()),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!(
                    plugin = %ctx.plugin_name,
                    hash,
                    error = %e,
                    "wasm plugin blob_get failed"
                );
                None
            }
        }
    })
}

/// Store bytes in the blob store and return the hex-encoded BLAKE3 hash.
pub fn blob_put(ctx: &PluginHostContext, data: &[u8]) -> Result<String, String> {
    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        match ctx.blob_store.add_bytes(data).await {
            Ok(result) => Ok(result.blob_ref.hash.to_string()),
            Err(e) => {
                tracing::warn!(
                    plugin = %ctx.plugin_name,
                    data_len = data.len(),
                    error = %e,
                    "wasm plugin blob_put failed"
                );
                Err(format!("blob_put failed: {e}"))
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Identity host functions
// ---------------------------------------------------------------------------

/// Return the node ID of the host.
pub fn node_id(ctx: &PluginHostContext) -> u64 {
    ctx.node_id
}

// ---------------------------------------------------------------------------
// Randomness host functions
// ---------------------------------------------------------------------------

/// Generate `count` random bytes using the OS CSPRNG.
pub fn random_bytes(count: u32) -> Vec<u8> {
    let count = count.min(4096) as usize; // Cap at 4KB per call
    let mut buf = vec![0u8; count];
    getrandom::getrandom(&mut buf).unwrap_or_else(|e| {
        tracing::warn!(error = %e, "getrandom failed, returning zeroed bytes");
    });
    buf
}

// ---------------------------------------------------------------------------
// Cluster host functions
// ---------------------------------------------------------------------------

/// Check if the current node is the Raft leader.
pub fn is_leader(ctx: &PluginHostContext) -> bool {
    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        match ctx.controller.get_leader().await {
            Ok(Some(leader_id)) => leader_id == ctx.node_id,
            _ => false,
        }
    })
}

/// Get the current Raft leader's node ID, or 0 if unknown.
pub fn leader_id(ctx: &PluginHostContext) -> u64 {
    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        match ctx.controller.get_leader().await {
            Ok(Some(id)) => id,
            _ => 0,
        }
    })
}

// ---------------------------------------------------------------------------
// Sandbox registration (primitive mode)
// ---------------------------------------------------------------------------

/// Register all host functions on a `ProtoWasmSandbox` for a WASM handler plugin.
///
/// Must be called before `proto.load_runtime()`. Each closure captures a
/// shared `Arc<PluginHostContext>` and delegates to the standalone functions.
pub fn register_plugin_host_functions(
    proto: &mut hyperlight_wasm::ProtoWasmSandbox,
    ctx: Arc<PluginHostContext>,
) -> anyhow::Result<()> {
    // -- Logging --
    let ctx_log_info = Arc::clone(&ctx);
    proto
        .register("log_info", move |msg: String| -> () {
            log_info(&ctx_log_info.plugin_name, &msg);
        })
        .map_err(|e| anyhow::anyhow!("failed to register log_info: {e}"))?;

    let ctx_log_debug = Arc::clone(&ctx);
    proto
        .register("log_debug", move |msg: String| -> () {
            log_debug(&ctx_log_debug.plugin_name, &msg);
        })
        .map_err(|e| anyhow::anyhow!("failed to register log_debug: {e}"))?;

    let ctx_log_warn = Arc::clone(&ctx);
    proto
        .register("log_warn", move |msg: String| -> () {
            log_warn(&ctx_log_warn.plugin_name, &msg);
        })
        .map_err(|e| anyhow::anyhow!("failed to register log_warn: {e}"))?;

    // -- Clock --
    proto
        .register("now_ms", || -> u64 { now_ms() })
        .map_err(|e| anyhow::anyhow!("failed to register now_ms: {e}"))?;

    // -- KV Store --
    // kv_get: returns Vec<u8> (empty = key not found)
    let ctx_kv_get = Arc::clone(&ctx);
    proto
        .register("kv_get", move |key: String| -> Vec<u8> { kv_get(&ctx_kv_get, &key).unwrap_or_default() })
        .map_err(|e| anyhow::anyhow!("failed to register kv_get: {e}"))?;

    // kv_put: returns String (empty = success, non-empty = error)
    let ctx_kv_put = Arc::clone(&ctx);
    proto
        .register("kv_put", move |key: String, value: Vec<u8>| -> String {
            match kv_put(&ctx_kv_put, &key, &value) {
                Ok(()) => String::new(),
                Err(e) => e,
            }
        })
        .map_err(|e| anyhow::anyhow!("failed to register kv_put: {e}"))?;

    // kv_delete: returns String (empty = success, non-empty = error)
    let ctx_kv_delete = Arc::clone(&ctx);
    proto
        .register("kv_delete", move |key: String| -> String {
            match kv_delete(&ctx_kv_delete, &key) {
                Ok(()) => String::new(),
                Err(e) => e,
            }
        })
        .map_err(|e| anyhow::anyhow!("failed to register kv_delete: {e}"))?;

    // kv_scan: returns Vec<u8> (JSON-serialized array of [key, value_bytes] pairs)
    let ctx_kv_scan = Arc::clone(&ctx);
    proto
        .register("kv_scan", move |prefix: String, limit: u32| -> Vec<u8> {
            let results = kv_scan(&ctx_kv_scan, &prefix, limit);
            serde_json::to_vec(&results).unwrap_or_default()
        })
        .map_err(|e| anyhow::anyhow!("failed to register kv_scan: {e}"))?;

    // kv_cas: returns String (empty = success, non-empty = error)
    let ctx_kv_cas = Arc::clone(&ctx);
    proto
        .register("kv_cas", move |key: String, expected: Vec<u8>, new_value: Vec<u8>| -> String {
            match kv_cas(&ctx_kv_cas, &key, &expected, &new_value) {
                Ok(()) => String::new(),
                Err(e) => e,
            }
        })
        .map_err(|e| anyhow::anyhow!("failed to register kv_cas: {e}"))?;

    // -- Blob Store --
    // blob_has: bool is directly supported
    let ctx_blob_has = Arc::clone(&ctx);
    proto
        .register("blob_has", move |hash: String| -> bool { blob_has(&ctx_blob_has, &hash) })
        .map_err(|e| anyhow::anyhow!("failed to register blob_has: {e}"))?;

    // blob_get: returns Vec<u8> (empty = not found)
    let ctx_blob_get = Arc::clone(&ctx);
    proto
        .register("blob_get", move |hash: String| -> Vec<u8> { blob_get(&ctx_blob_get, &hash).unwrap_or_default() })
        .map_err(|e| anyhow::anyhow!("failed to register blob_get: {e}"))?;

    // blob_put: returns String with first byte as ok/err tag
    // '\0' + hash = success, '\x01' + error = failure
    let ctx_blob_put = Arc::clone(&ctx);
    proto
        .register("blob_put", move |data: Vec<u8>| -> String {
            match blob_put(&ctx_blob_put, &data) {
                Ok(hash) => format!("\0{hash}"),
                Err(e) => format!("\x01{e}"),
            }
        })
        .map_err(|e| anyhow::anyhow!("failed to register blob_put: {e}"))?;

    // -- Identity --
    let ctx_node_id = Arc::clone(&ctx);
    proto
        .register("node_id", move || -> u64 { node_id(&ctx_node_id) })
        .map_err(|e| anyhow::anyhow!("failed to register node_id: {e}"))?;

    // -- Randomness --
    proto
        .register("random_bytes", move |count: u32| -> Vec<u8> { random_bytes(count) })
        .map_err(|e| anyhow::anyhow!("failed to register random_bytes: {e}"))?;

    // -- Cluster --
    let ctx_is_leader = Arc::clone(&ctx);
    proto
        .register("is_leader", move || -> bool { is_leader(&ctx_is_leader) })
        .map_err(|e| anyhow::anyhow!("failed to register is_leader: {e}"))?;

    let ctx_leader_id = Arc::clone(&ctx);
    proto
        .register("leader_id", move || -> u64 { leader_id(&ctx_leader_id) })
        .map_err(|e| anyhow::anyhow!("failed to register leader_id: {e}"))?;

    Ok(())
}
