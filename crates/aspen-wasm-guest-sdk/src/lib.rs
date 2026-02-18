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
pub use aspen_client_api::ClientRpcRequest;
pub use aspen_client_api::ClientRpcResponse;
pub use aspen_client_api::ErrorResponse;
pub use aspen_client_api::ReadResultResponse;
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
    };
}
