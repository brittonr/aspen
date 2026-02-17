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

// TODO: Implement full WIT host bindings using hyperlight_component_macro::host_bindgen!()
// when hyperlight-wasm stabilizes. The macro will generate typed bindings from
// wit/aspen-plugin.wit that map to the functions defined in this module.

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
// Sandbox registration
// ---------------------------------------------------------------------------

// TODO: Add register_host_functions(sandbox, ctx) when hyperlight-wasm
// stabilizes and the `UninitializedSandbox` type becomes available.
// The function will register each host function above on the sandbox,
// capturing a cloned Arc<AspenHostContext> in each closure:
//
//   pub fn register_host_functions(
//       sandbox: &mut UninitializedSandbox,
//       ctx: Arc<AspenHostContext>,
//   ) -> crate::error::Result<()> {
//       let c = ctx.clone();
//       sandbox.register("log_info", move |msg: String| {
//           log_info(&c.job_id, &msg);
//           Ok(())
//       }).map_err(|e| ...)?;
//       // ... repeat for all host functions
//   }
