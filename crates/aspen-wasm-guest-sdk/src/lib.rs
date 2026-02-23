//! Guest SDK for building Aspen WASM handler plugins.
//!
//! Plugin authors depend on this crate to implement request handlers that
//! run inside the Aspen WASM sandbox. The typical workflow:
//!
//! 1. Implement the [`AspenPlugin`] trait.
//! 2. Call [`register_plugin!`] with your type to generate the required FFI exports
//!    (`handle_request` and `plugin_info`).
//! 3. Compile to `wasm32-unknown-unknown` and deploy via the plugin manifest.
//!
//! # Example
//!
//! ```rust,ignore
//! use aspen_wasm_guest_sdk::{AspenPlugin, ClientRpcRequest, ClientRpcResponse, PluginInfo};
//!
//! struct Echo;
//!
//! impl AspenPlugin for Echo {
//!     fn info() -> PluginInfo {
//!         PluginInfo {
//!             name: "echo".into(),
//!             version: "0.1.0".into(),
//!             handles: vec!["Echo".into()],
//!             priority: 900,
//!             app_id: None,
//!             kv_prefixes: vec![],
//!             permissions: PluginPermissions::default(),
//!         }
//!     }
//!
//!     fn handle(request: ClientRpcRequest) -> ClientRpcResponse {
//!         ClientRpcResponse::error("ECHO", &format!("{request:?}"))
//!     }
//! }
//!
//! aspen_wasm_guest_sdk::register_plugin!(Echo);
//! ```
//!
//! The [`AspenPlugin`] trait also provides lifecycle hooks (`init`, `shutdown`, `health`)
//! with default implementations.

pub mod host;
pub mod response;

// Re-export types that plugin authors need.
// KV response types for handler plugins.
// Docs response types for handler plugins.
pub use aspen_client_api::AddPeerClusterResultResponse;
pub use aspen_client_api::BatchCondition;
pub use aspen_client_api::BatchReadResultResponse;
pub use aspen_client_api::BatchWriteOperation;
pub use aspen_client_api::BatchWriteResultResponse;
pub use aspen_client_api::ClientRpcRequest;
pub use aspen_client_api::ClientRpcResponse;
pub use aspen_client_api::CompareAndSwapResultResponse;
pub use aspen_client_api::ConditionalBatchWriteResultResponse;
pub use aspen_client_api::DeleteResultResponse;
pub use aspen_client_api::DocsDeleteResultResponse;
pub use aspen_client_api::DocsGetResultResponse;
pub use aspen_client_api::DocsListEntry;
pub use aspen_client_api::DocsListResultResponse;
pub use aspen_client_api::DocsSetResultResponse;
pub use aspen_client_api::DocsStatusResultResponse;
pub use aspen_client_api::ErrorResponse;
// Hook response types for handler plugins.
pub use aspen_client_api::HookHandlerInfo;
pub use aspen_client_api::HookHandlerMetrics;
pub use aspen_client_api::HookListResultResponse;
pub use aspen_client_api::HookMetricsResultResponse;
pub use aspen_client_api::HookTriggerResultResponse;
// Job/Worker response types for handler plugins.
pub use aspen_client_api::JobCancelResultResponse;
pub use aspen_client_api::JobDetails;
pub use aspen_client_api::JobGetResultResponse;
pub use aspen_client_api::JobListResultResponse;
pub use aspen_client_api::JobQueueStatsResultResponse;
pub use aspen_client_api::JobSubmitResultResponse;
pub use aspen_client_api::JobUpdateProgressResultResponse;
pub use aspen_client_api::KeyOriginResultResponse;
pub use aspen_client_api::ListPeerClustersResultResponse;
pub use aspen_client_api::PeerClusterInfo;
pub use aspen_client_api::PeerClusterStatusResponse;
pub use aspen_client_api::ReadResultResponse;
pub use aspen_client_api::RemovePeerClusterResultResponse;
pub use aspen_client_api::ScanEntry;
pub use aspen_client_api::ScanResultResponse;
pub use aspen_client_api::SetPeerClusterEnabledResultResponse;
// SQL response types for handler plugins.
pub use aspen_client_api::SqlCellValue;
pub use aspen_client_api::SqlResultResponse;
pub use aspen_client_api::UpdatePeerClusterFilterResultResponse;
pub use aspen_client_api::UpdatePeerClusterPriorityResultResponse;
pub use aspen_client_api::WorkerDeregisterResultResponse;
pub use aspen_client_api::WorkerHeartbeatResultResponse;
pub use aspen_client_api::WorkerInfo;
pub use aspen_client_api::WorkerRegisterResultResponse;
pub use aspen_client_api::WorkerStatusResultResponse;
pub use aspen_client_api::WriteResultResponse;
pub use aspen_plugin_api::KvBatchOp;
pub use aspen_plugin_api::PluginHealth;
pub use aspen_plugin_api::PluginInfo;
pub use aspen_plugin_api::PluginPermissions;
pub use aspen_plugin_api::PluginState;
pub use aspen_plugin_api::TimerConfig;

/// Trait that WASM plugin authors implement to handle requests.
pub trait AspenPlugin {
    /// Return metadata describing this plugin.
    fn info() -> PluginInfo;

    /// Handle an incoming client RPC request and produce a response.
    fn handle(request: ClientRpcRequest) -> ClientRpcResponse;

    /// Called once after the plugin is loaded. Perform initialization here.
    ///
    /// Return `Ok(())` to signal readiness, or `Err(message)` to indicate
    /// initialization failure. The default implementation succeeds immediately.
    fn init() -> Result<(), String> {
        Ok(())
    }

    /// Called when the plugin is being unloaded. Release resources here.
    ///
    /// The default implementation does nothing.
    fn shutdown() {}

    /// Called periodically by the host to check plugin health.
    ///
    /// Return `Ok(())` if the plugin is healthy, or `Err(message)` if it is
    /// degraded. The default implementation always reports healthy.
    fn health() -> Result<(), String> {
        Ok(())
    }

    /// Called by the host when a scheduled timer fires.
    ///
    /// The `name` parameter identifies which timer fired. Schedule timers
    /// via [`host::schedule_timer_on_host`].
    ///
    /// The default implementation does nothing.
    fn on_timer(_name: &str) {}

    /// Called by the host when a subscribed hook event fires.
    ///
    /// Subscribe to events via [`host::subscribe_hook_events`] (typically
    /// in your `init` method). The `topic` is the full event topic
    /// (e.g., `hooks.kv.write_committed`) and `event` is the JSON-serialized
    /// hook event payload.
    ///
    /// The default implementation does nothing.
    fn on_hook_event(_topic: &str, _event: &[u8]) {}
}

/// Register a plugin type by generating the `handle_request` and `plugin_info`
/// FFI exports that the host expects.
///
/// The macro deserializes the incoming JSON bytes, dispatches to the plugin's
/// `handle` method, and serializes the response back to JSON bytes.
#[macro_export]
macro_rules! register_plugin {
    ($plugin_type:ty) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn handle_request(input: Vec<u8>) -> Vec<u8> {
            let request: $crate::ClientRpcRequest = match serde_json::from_slice(&input) {
                Ok(r) => r,
                Err(e) => {
                    let err = $crate::ClientRpcResponse::Error($crate::response::error_response(
                        "PLUGIN_DESERIALIZE_ERROR",
                        &format!("failed to deserialize request: {e}"),
                    ));
                    return serde_json::to_vec(&err).unwrap_or_default();
                }
            };
            let response = <$plugin_type as $crate::AspenPlugin>::handle(request);
            serde_json::to_vec(&response).unwrap_or_default()
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn plugin_info(_input: Vec<u8>) -> Vec<u8> {
            let info = <$plugin_type as $crate::AspenPlugin>::info();
            serde_json::to_vec(&info).unwrap_or_default()
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn plugin_init(_input: Vec<u8>) -> Vec<u8> {
            match <$plugin_type as $crate::AspenPlugin>::init() {
                Ok(()) => {
                    // Return JSON: {"ok": true}
                    serde_json::to_vec(&serde_json::json!({"ok": true})).unwrap_or_default()
                }
                Err(e) => {
                    // Return JSON: {"ok": false, "error": "message"}
                    serde_json::to_vec(&serde_json::json!({"ok": false, "error": e})).unwrap_or_default()
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn plugin_shutdown(_input: Vec<u8>) -> Vec<u8> {
            <$plugin_type as $crate::AspenPlugin>::shutdown();
            serde_json::to_vec(&serde_json::json!({"ok": true})).unwrap_or_default()
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn plugin_health(_input: Vec<u8>) -> Vec<u8> {
            match <$plugin_type as $crate::AspenPlugin>::health() {
                Ok(()) => {
                    serde_json::to_vec(&serde_json::json!({"ok": true})).unwrap_or_default()
                }
                Err(e) => {
                    serde_json::to_vec(&serde_json::json!({"ok": false, "error": e})).unwrap_or_default()
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn plugin_on_timer(input: Vec<u8>) -> Vec<u8> {
            let name: String = serde_json::from_slice(&input).unwrap_or_default();
            <$plugin_type as $crate::AspenPlugin>::on_timer(&name);
            serde_json::to_vec(&serde_json::json!({"ok": true})).unwrap_or_default()
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn plugin_on_hook_event(input: Vec<u8>) -> Vec<u8> {
            // Input is JSON: {"topic": "...", "event": {...}}
            let parsed: serde_json::Value = serde_json::from_slice(&input).unwrap_or_default();
            let topic = parsed["topic"].as_str().unwrap_or("");
            let event_bytes = serde_json::to_vec(&parsed["event"]).unwrap_or_default();
            <$plugin_type as $crate::AspenPlugin>::on_hook_event(topic, &event_bytes);
            serde_json::to_vec(&serde_json::json!({"ok": true})).unwrap_or_default()
        }
    };
}
