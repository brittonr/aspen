//! Host function bindings for WASM Component Model guests.
//!
//! Maps WIT interface functions (logging, clock, kv-store, blob-store) to
//! Aspen's internal traits. These bindings are registered with the
//! hyperlight-wasm sandbox before guest execution.
//!
//! See `wit/aspen-plugin.wit` for the WIT interface definitions.

use std::sync::Arc;

use aspen_blob::prelude::*;

// TODO: Implement full WIT host bindings using hyperlight_component_macro::host_bindgen!()
// when hyperlight-wasm stabilizes. The macro will generate typed bindings from
// wit/aspen-plugin.wit that map to the functions defined in this module.
//
// The async-to-sync bridge for KV and blob operations will use:
//   tokio::runtime::Handle::current().block_on(async_operation)

/// Host context passed to WASM guest host function callbacks.
///
/// Holds references to Aspen services that the guest can interact with
/// through the WIT-defined interfaces (logging, clock, kv-store, blob-store).
pub struct AspenHostContext {
    /// Blob store for guest blob operations.
    pub blob_store: Arc<dyn BlobStore>,
    /// Job ID for structured log context.
    pub job_id: String,
    /// Clock baseline (epoch ms at sandbox creation).
    pub start_time_ms: u64,
}

impl AspenHostContext {
    /// Create a new host context for a WASM guest.
    pub fn new(blob_store: Arc<dyn BlobStore>, job_id: String, start_time_ms: u64) -> Self {
        Self {
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
// KV / blob helpers are intentionally left as TODOs until the sandbox can
// actually invoke them.
// ---------------------------------------------------------------------------

// TODO: Add kv_get, kv_put, kv_delete, kv_scan host functions that bridge
// the async KeyValueStore trait to synchronous host callbacks via
// tokio::runtime::Handle::current().block_on().

// TODO: Add blob_has, blob_get, blob_put host functions that bridge the
// async BlobRead / BlobWrite traits to synchronous host callbacks.
