//! Host function bindings for WASM Component Model guests.
//!
//! Maps WIT interface functions (logging, clock, kv-store, blob-store) to
//! Aspen's internal traits. These bindings are registered with the
//! hyperlight-wasm sandbox before guest execution.
//!
//! Each host function is a standalone function taking `&AspenHostContext`,
//! bridging async Aspen trait calls to synchronous host callbacks via
//! `tokio::runtime::Handle::current().block_on()`.
//!
//! See `wit/aspen-plugin.wit` for the WIT interface definitions.

use std::sync::Arc;

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;

// TODO: Generate typed WIT bindings when cargo-hyperlight supports Cargo 1.93+.
// Blocked: cargo-hyperlight v0.1.5 uses --build-plan (removed in Cargo 1.93).
//
// When unblocked:
// 1. Compile WIT to binary: `wasm-tools component wit -w -o world.wasm wit/aspen-plugin.wit`
// 2. Generate bindings: mod bindings { hyperlight_component_macro::host_bindgen!("world.wasm"); }
// 3. Implement the generated Host trait on AspenHostContext, delegating to the standalone functions
//    below (kv_get, kv_put, blob_get, etc.)
// 4. In wasm_component.rs, call `bindings::register_host_functions(&mut proto, state)` to register
//    all host functions on the ProtoWasmSandbox.

/// Host context passed to WASM guest host function callbacks.
///
/// Holds references to Aspen services that the guest can interact with
/// through the WIT-defined interfaces (logging, clock, kv-store, blob-store).
pub struct AspenHostContext {
    /// KV store for guest key-value operations.
    pub kv_store: Arc<dyn KeyValueStore>,
    /// Blob store for guest blob operations.
    pub blob_store: Arc<dyn BlobStore>,
    /// Job ID for structured log context.
    pub job_id: String,
    /// Clock baseline (epoch ms at sandbox creation).
    /// Reserved for guest elapsed-time queries (not yet exposed as a host function).
    #[allow(dead_code)]
    pub start_time_ms: u64,
}

impl AspenHostContext {
    /// Create a new host context for a WASM guest.
    pub fn new(
        kv_store: Arc<dyn KeyValueStore>,
        blob_store: Arc<dyn BlobStore>,
        job_id: String,
        start_time_ms: u64,
    ) -> Self {
        Self {
            kv_store,
            blob_store,
            job_id,
            start_time_ms,
        }
    }
}

// ---------------------------------------------------------------------------
// Logging host functions (WIT: aspen:plugin/logging)
// ---------------------------------------------------------------------------

/// Log an informational message from the WASM guest.
pub fn log_info(job_id: &str, message: &str) {
    tracing::info!(job_id, guest_message = %message, "wasm guest log");
}

/// Log a debug message from the WASM guest.
pub fn log_debug(job_id: &str, message: &str) {
    tracing::debug!(job_id, guest_message = %message, "wasm guest log");
}

/// Log a warning message from the WASM guest.
pub fn log_warn(job_id: &str, message: &str) {
    tracing::warn!(job_id, guest_message = %message, "wasm guest log");
}

// ---------------------------------------------------------------------------
// Clock host function (WIT: aspen:plugin/clock)
// ---------------------------------------------------------------------------

/// Return the current wall-clock time as milliseconds since the Unix epoch.
///
/// Uses `unwrap_or_default` to avoid panicking if the system clock is before
/// the epoch (returns 0 in that degenerate case).
pub fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

// ---------------------------------------------------------------------------
// KV Store host functions (WIT: aspen:plugin/kv-store)
// ---------------------------------------------------------------------------

/// Get a value by key from the distributed KV store.
///
/// Returns `None` if the key does not exist. Values are returned as raw
/// UTF-8 bytes since the KV store uses `String` values internally.
pub fn kv_get(ctx: &AspenHostContext, key: &str) -> Option<Vec<u8>> {
    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        let request = aspen_kv_types::ReadRequest::new(key);
        match ctx.kv_store.read(request).await {
            Ok(result) => result.kv.map(|entry| entry.value.into_bytes()),
            Err(e) => {
                tracing::warn!(
                    job_id = %ctx.job_id,
                    key,
                    error = %e,
                    "wasm kv_get failed"
                );
                None
            }
        }
    })
}

/// Put a key-value pair into the distributed KV store.
///
/// The value bytes are interpreted as UTF-8. Returns an error string if the
/// write fails or the value contains invalid UTF-8.
pub fn kv_put(ctx: &AspenHostContext, key: &str, value: &[u8]) -> Result<(), String> {
    let value_str = std::str::from_utf8(value).map_err(|e| format!("value is not valid UTF-8: {e}"))?;

    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        let request = aspen_kv_types::WriteRequest::set(key, value_str);
        ctx.kv_store.write(request).await.map(|_| ()).map_err(|e| {
            tracing::warn!(
                job_id = %ctx.job_id,
                key,
                error = %e,
                "wasm kv_put failed"
            );
            format!("kv_put failed: {e}")
        })
    })
}

/// Delete a key from the distributed KV store.
///
/// Returns an error string if the delete operation fails.
pub fn kv_delete(ctx: &AspenHostContext, key: &str) -> Result<(), String> {
    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        let request = aspen_kv_types::DeleteRequest::new(key);
        ctx.kv_store.delete(request).await.map(|_| ()).map_err(|e| {
            tracing::warn!(
                job_id = %ctx.job_id,
                key,
                error = %e,
                "wasm kv_delete failed"
            );
            format!("kv_delete failed: {e}")
        })
    })
}

/// Scan keys matching a prefix from the distributed KV store.
///
/// Returns a list of `(key, value_bytes)` pairs. The `limit` parameter
/// caps the number of results returned (0 means no limit).
pub fn kv_scan(ctx: &AspenHostContext, prefix: &str, limit: u32) -> Vec<(String, Vec<u8>)> {
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
                    job_id = %ctx.job_id,
                    prefix,
                    error = %e,
                    "wasm kv_scan failed"
                );
                Vec::new()
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Blob Store host functions (WIT: aspen:plugin/blob-store)
// ---------------------------------------------------------------------------

/// Check whether a blob exists in the store.
///
/// The `hash` parameter is the hex-encoded BLAKE3 hash of the blob.
/// Returns `false` if the hash is malformed or the blob does not exist.
pub fn blob_has(ctx: &AspenHostContext, hash: &str) -> bool {
    let blob_hash = match hash.parse::<iroh_blobs::Hash>() {
        Ok(h) => h,
        Err(e) => {
            tracing::warn!(
                job_id = %ctx.job_id,
                hash,
                error = %e,
                "wasm blob_has: invalid hash"
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
                    job_id = %ctx.job_id,
                    hash,
                    error = %e,
                    "wasm blob_has failed"
                );
                false
            }
        }
    })
}

/// Retrieve blob bytes by hash.
///
/// The `hash` parameter is the hex-encoded BLAKE3 hash of the blob.
/// Returns `None` if the hash is malformed or the blob does not exist.
pub fn blob_get(ctx: &AspenHostContext, hash: &str) -> Option<Vec<u8>> {
    let blob_hash = match hash.parse::<iroh_blobs::Hash>() {
        Ok(h) => h,
        Err(e) => {
            tracing::warn!(
                job_id = %ctx.job_id,
                hash,
                error = %e,
                "wasm blob_get: invalid hash"
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
                    job_id = %ctx.job_id,
                    hash,
                    error = %e,
                    "wasm blob_get failed"
                );
                None
            }
        }
    })
}

/// Store bytes in the blob store and return the hex-encoded BLAKE3 hash.
///
/// Returns the hash as a hex string on success, or an error description
/// on failure.
pub fn blob_put(ctx: &AspenHostContext, data: &[u8]) -> Result<String, String> {
    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        match ctx.blob_store.add_bytes(data).await {
            Ok(result) => Ok(result.blob_ref.hash.to_string()),
            Err(e) => {
                tracing::warn!(
                    job_id = %ctx.job_id,
                    data_len = data.len(),
                    error = %e,
                    "wasm blob_put failed"
                );
                Err(format!("blob_put failed: {e}"))
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Sandbox registration (primitive mode)
// ---------------------------------------------------------------------------

// TODO: Replace with generated Component Model bindings (host_bindgen!) when
// cargo-hyperlight drops the --build-plan dependency. The standalone functions
// above will become trait delegation targets.

/// Register all host functions on a `ProtoWasmSandbox` using primitive mode.
///
/// Must be called before `proto.load_runtime()`. Each closure captures a
/// shared `Arc<AspenHostContext>` and delegates to the standalone functions.
///
/// Type adaptations for primitive mode (only String, i32/u32/i64/u64, f32/f64,
/// bool, Vec<u8> are supported):
/// - `Option<Vec<u8>>` -> empty `Vec<u8>` for None (guest checks `.is_empty()`)
/// - `Result<(), String>` -> `String` (empty = success, non-empty = error)
/// - `Vec<(String, Vec<u8>)>` -> JSON-serialized `Vec<u8>`
/// - `Result<String, String>` -> `String` with `\0` prefix for ok, `\x01` for err
pub fn register_host_functions(
    proto: &mut hyperlight_wasm::ProtoWasmSandbox,
    ctx: Arc<AspenHostContext>,
) -> crate::error::Result<()> {
    // -- Logging --
    let ctx_log_info = Arc::clone(&ctx);
    proto
        .register("log_info", move |msg: String| -> () {
            log_info(&ctx_log_info.job_id, &msg);
        })
        .map_err(|e| crate::error::JobError::VmExecutionFailed {
            reason: format!("failed to register log_info: {e}"),
        })?;

    let ctx_log_debug = Arc::clone(&ctx);
    proto
        .register("log_debug", move |msg: String| -> () {
            log_debug(&ctx_log_debug.job_id, &msg);
        })
        .map_err(|e| crate::error::JobError::VmExecutionFailed {
            reason: format!("failed to register log_debug: {e}"),
        })?;

    let ctx_log_warn = Arc::clone(&ctx);
    proto
        .register("log_warn", move |msg: String| -> () {
            log_warn(&ctx_log_warn.job_id, &msg);
        })
        .map_err(|e| crate::error::JobError::VmExecutionFailed {
            reason: format!("failed to register log_warn: {e}"),
        })?;

    // -- Clock --
    proto
        .register("now_ms", || -> u64 { now_ms() })
        .map_err(|e| crate::error::JobError::VmExecutionFailed {
            reason: format!("failed to register now_ms: {e}"),
        })?;

    // -- KV Store --
    // kv_get: returns Vec<u8> (empty = key not found)
    let ctx_kv_get = Arc::clone(&ctx);
    proto
        .register("kv_get", move |key: String| -> Vec<u8> { kv_get(&ctx_kv_get, &key).unwrap_or_default() })
        .map_err(|e| crate::error::JobError::VmExecutionFailed {
            reason: format!("failed to register kv_get: {e}"),
        })?;

    // kv_put: returns String (empty = success, non-empty = error)
    let ctx_kv_put = Arc::clone(&ctx);
    proto
        .register("kv_put", move |key: String, value: Vec<u8>| -> String {
            match kv_put(&ctx_kv_put, &key, &value) {
                Ok(()) => String::new(),
                Err(e) => e,
            }
        })
        .map_err(|e| crate::error::JobError::VmExecutionFailed {
            reason: format!("failed to register kv_put: {e}"),
        })?;

    // kv_delete: returns String (empty = success, non-empty = error)
    let ctx_kv_delete = Arc::clone(&ctx);
    proto
        .register("kv_delete", move |key: String| -> String {
            match kv_delete(&ctx_kv_delete, &key) {
                Ok(()) => String::new(),
                Err(e) => e,
            }
        })
        .map_err(|e| crate::error::JobError::VmExecutionFailed {
            reason: format!("failed to register kv_delete: {e}"),
        })?;

    // kv_scan: returns Vec<u8> (JSON-serialized array of [key, value_bytes] pairs)
    let ctx_kv_scan = Arc::clone(&ctx);
    proto
        .register("kv_scan", move |prefix: String, limit: u32| -> Vec<u8> {
            let results = kv_scan(&ctx_kv_scan, &prefix, limit);
            serde_json::to_vec(&results).unwrap_or_default()
        })
        .map_err(|e| crate::error::JobError::VmExecutionFailed {
            reason: format!("failed to register kv_scan: {e}"),
        })?;

    // -- Blob Store --
    // blob_has: bool is directly supported
    let ctx_blob_has = Arc::clone(&ctx);
    proto
        .register("blob_has", move |hash: String| -> bool { blob_has(&ctx_blob_has, &hash) })
        .map_err(|e| crate::error::JobError::VmExecutionFailed {
            reason: format!("failed to register blob_has: {e}"),
        })?;

    // blob_get: returns Vec<u8> (empty = not found)
    let ctx_blob_get = Arc::clone(&ctx);
    proto
        .register("blob_get", move |hash: String| -> Vec<u8> { blob_get(&ctx_blob_get, &hash).unwrap_or_default() })
        .map_err(|e| crate::error::JobError::VmExecutionFailed {
            reason: format!("failed to register blob_get: {e}"),
        })?;

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
        .map_err(|e| crate::error::JobError::VmExecutionFailed {
            reason: format!("failed to register blob_put: {e}"),
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use aspen_blob::InMemoryBlobStore;
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    /// Create an `AspenHostContext` backed by in-memory stores.
    fn test_context() -> Arc<AspenHostContext> {
        let kv_store: Arc<dyn KeyValueStore> = DeterministicKeyValueStore::new();
        let blob_store: Arc<dyn BlobStore> = Arc::new(InMemoryBlobStore::new());
        Arc::new(AspenHostContext::new(kv_store, blob_store, "test-job-1".to_string(), 0))
    }

    /// Run a closure that calls synchronous host functions (which use
    /// `Handle::current().block_on()`) on a blocking thread so we don't
    /// nest runtimes.
    async fn on_blocking<F, R>(f: F) -> R
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        tokio::task::spawn_blocking(f).await.expect("blocking task panicked")
    }

    #[test]
    fn test_now_ms_returns_reasonable_value() {
        let ts = now_ms();
        // Post-2023 timestamp (2023-11-14 roughly)
        assert!(ts > 1_700_000_000_000, "timestamp {ts} should be post-2023");
        // Before year 2100
        assert!(ts < 4_102_444_800_000, "timestamp {ts} should be before 2100");
    }

    #[test]
    fn test_log_functions_do_not_panic() {
        log_info("job-1", "hello info");
        log_debug("job-2", "hello debug");
        log_warn("job-3", "hello warn");

        // Empty messages
        log_info("job-1", "");
        log_debug("job-2", "");
        log_warn("job-3", "");

        // Long message
        let long_msg = "x".repeat(10_000);
        log_info("job-1", &long_msg);
        log_debug("job-2", &long_msg);
        log_warn("job-3", &long_msg);
    }

    #[test]
    fn test_aspen_host_context_creation() {
        let kv_store: Arc<dyn KeyValueStore> = DeterministicKeyValueStore::new();
        let blob_store: Arc<dyn BlobStore> = Arc::new(InMemoryBlobStore::new());
        let ctx = AspenHostContext::new(kv_store, blob_store, "test-job-1".to_string(), 42);
        assert_eq!(ctx.job_id, "test-job-1");
        assert_eq!(ctx.start_time_ms, 42);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_kv_get_nonexistent_key() {
        let ctx = test_context();
        let result = on_blocking(move || kv_get(&ctx, "nonexistent-key")).await;
        assert!(result.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_kv_put_and_get_roundtrip() {
        let ctx = test_context();
        on_blocking({
            let ctx = Arc::clone(&ctx);
            move || kv_put(&ctx, "my-key", b"hello world").expect("kv_put should succeed")
        })
        .await;

        let got = on_blocking(move || kv_get(&ctx, "my-key")).await;
        assert_eq!(got.expect("key should exist"), b"hello world");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_kv_delete() {
        let ctx = test_context();

        on_blocking({
            let ctx = Arc::clone(&ctx);
            move || kv_put(&ctx, "delete-me", b"value").expect("kv_put should succeed")
        })
        .await;

        let exists = on_blocking({
            let ctx = Arc::clone(&ctx);
            move || kv_get(&ctx, "delete-me").is_some()
        })
        .await;
        assert!(exists);

        on_blocking({
            let ctx = Arc::clone(&ctx);
            move || kv_delete(&ctx, "delete-me").expect("kv_delete should succeed")
        })
        .await;

        let gone = on_blocking(move || kv_get(&ctx, "delete-me").is_none()).await;
        assert!(gone);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_kv_scan_with_prefix() {
        let ctx = test_context();

        on_blocking({
            let ctx = Arc::clone(&ctx);
            move || {
                kv_put(&ctx, "prefix/a", b"1").expect("put a");
                kv_put(&ctx, "prefix/b", b"2").expect("put b");
                kv_put(&ctx, "prefix/c", b"3").expect("put c");
                kv_put(&ctx, "other/x", b"4").expect("put x");
            }
        })
        .await;

        let results = on_blocking(move || kv_scan(&ctx, "prefix/", 0)).await;
        assert_eq!(results.len(), 3);

        let keys: Vec<&str> = results.iter().map(|(k, _)| k.as_str()).collect();
        assert!(keys.contains(&"prefix/a"));
        assert!(keys.contains(&"prefix/b"));
        assert!(keys.contains(&"prefix/c"));
        assert!(!keys.contains(&"other/x"));
    }

    #[test]
    fn test_kv_put_invalid_utf8() {
        // UTF-8 validation happens before block_on, so no runtime needed.
        let kv_store: Arc<dyn KeyValueStore> = DeterministicKeyValueStore::new();
        let blob_store: Arc<dyn BlobStore> = Arc::new(InMemoryBlobStore::new());
        let ctx = AspenHostContext::new(kv_store, blob_store, "test-job-1".to_string(), 0);

        let invalid_utf8: &[u8] = &[0xFF, 0xFE, 0xFD];
        let result = kv_put(&ctx, "bad-value", invalid_utf8);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("UTF-8"), "error should mention UTF-8, got: {err_msg}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_blob_has_nonexistent() {
        let ctx = test_context();
        // Valid 64-hex-char BLAKE3 hash format, but no blob with this hash stored.
        let fake_hash = "a".repeat(64);
        let result = on_blocking(move || blob_has(&ctx, &fake_hash)).await;
        assert!(!result);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_blob_get_nonexistent() {
        let ctx = test_context();
        // Valid hex format but no blob stored with this hash.
        // iroh_blobs::Hash expects a 32-byte (64-hex-char) BLAKE3 hash.
        let fake_hash = "0".repeat(64);
        let result = on_blocking(move || blob_get(&ctx, &fake_hash)).await;
        assert!(result.is_none());
    }
}
