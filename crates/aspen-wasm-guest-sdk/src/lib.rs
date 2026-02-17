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

pub mod host;
pub mod response;

// Re-export types that plugin authors need.
pub use aspen_client_api::ClientRpcRequest;
pub use aspen_client_api::ClientRpcResponse;
pub use aspen_client_api::ErrorResponse;
pub use aspen_client_api::ReadResultResponse;
pub use aspen_plugin_api::PluginInfo;

/// Trait that WASM plugin authors implement to handle requests.
pub trait AspenPlugin {
    /// Return metadata describing this plugin.
    fn info() -> PluginInfo;

    /// Handle an incoming client RPC request and produce a response.
    fn handle(request: ClientRpcRequest) -> ClientRpcResponse;
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
    };
}
