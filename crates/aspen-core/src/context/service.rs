//! Generic service executor for WASM plugin host functions.
//!
//! Defines a trait that allows domain services (docs, jobs, CI, etc.)
//! to be invoked from WASM plugins via a single `service_execute` host
//! function without requiring the WASM runtime to depend on each
//! domain crate directly.
//!
//! # Architecture
//!
//! ```text
//! WASM Plugin  →  service_execute(json)  →  ServiceExecutor::execute(json)
//!                                              ↓
//!                                         DocsServiceExecutor (has docs_sync)
//!                                         JobServiceExecutor  (has job_manager)
//!                                         CiServiceExecutor   (has orchestrator)
//! ```
//!
//! Concrete `ServiceExecutor` implementations live in their respective
//! handler/service crates and capture the native service references.
//! The node startup code creates executors and passes them through
//! `ClientProtocolContext` → `PluginHostContext`.

use async_trait::async_trait;

/// Generic executor for domain-specific service operations.
///
/// Each domain (docs, jobs, CI, etc.) implements this trait to handle
/// JSON-encoded requests from WASM plugins. The `service_execute` host
/// function dispatches to the right executor by service name.
///
/// # Request Format
///
/// Requests are JSON objects with at minimum an `"op"` field:
///
/// ```json
/// {"op": "set", "key": "my-key", "value": "base64data"}
/// ```
///
/// # Response Format
///
/// Responses use the standard tagged-string encoding:
/// - `\0{json}` for success
/// - `\x01{error}` for failure
#[async_trait]
pub trait ServiceExecutor: Send + Sync {
    /// The service name used for dispatch (e.g., "docs", "jobs", "ci").
    fn service_name(&self) -> &str;

    /// Execute a JSON-encoded operation.
    ///
    /// Returns a tagged string: `\0{json}` on success, `\x01{error}` on failure.
    async fn execute(&self, request_json: &str) -> String;
}
