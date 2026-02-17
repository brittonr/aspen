//! WASM plugin handler that delegates to a sandboxed guest.
//!
//! `WasmPluginHandler` wraps a loaded hyperlight-wasm sandbox and
//! implements `RequestHandler` so the `HandlerRegistry` can dispatch
//! matching requests to the WASM guest's `handle_request` export.
//!
//! Sandbox calls are executed via `spawn_blocking` since hyperlight
//! operations are CPU-bound and `LoadedWasmSandbox` is not `Send`.

use std::sync::Arc;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;

use crate::marshal;

/// A request handler backed by a WASM plugin running in a hyperlight-wasm sandbox.
///
/// The sandbox is wrapped in a `std::sync::Mutex` because `call_guest_function`
/// requires `&mut self`. All sandbox calls go through `spawn_blocking` to avoid
/// blocking the async executor.
pub struct WasmPluginHandler {
    /// Plugin name (leaked for 'static lifetime requirement of `RequestHandler::name`).
    name: &'static str,
    /// Request variant names this plugin handles.
    handles: Vec<String>,
    /// The loaded WASM sandbox. Mutex because `call_guest_function` takes `&mut`.
    sandbox: Arc<std::sync::Mutex<hyperlight_wasm::LoadedWasmSandbox>>,
}

impl WasmPluginHandler {
    /// Create a new WASM plugin handler.
    ///
    /// # Arguments
    ///
    /// * `name` - Plugin name (will be leaked for 'static lifetime)
    /// * `handles` - Request variant names this plugin handles
    /// * `sandbox` - The loaded hyperlight-wasm sandbox
    pub fn new(name: String, handles: Vec<String>, sandbox: hyperlight_wasm::LoadedWasmSandbox) -> Self {
        Self {
            name: Box::leak(name.into_boxed_str()),
            handles,
            sandbox: Arc::new(std::sync::Mutex::new(sandbox)),
        }
    }
}

#[async_trait::async_trait]
impl RequestHandler for WasmPluginHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        marshal::extract_variant_name(request)
            .map(|name| self.handles.iter().any(|h| h == &name))
            .unwrap_or(false)
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        _ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        let input = marshal::serialize_request(&request)?;
        let sandbox = Arc::clone(&self.sandbox);
        let handler_name = self.name;

        let output = tokio::task::spawn_blocking(move || {
            let mut guard = sandbox.lock().map_err(|e| anyhow::anyhow!("sandbox mutex poisoned: {e}"))?;
            guard
                .call_guest_function::<Vec<u8>>("handle_request", input)
                .map_err(|e| anyhow::anyhow!("WASM plugin '{handler_name}' execution failed: {e}"))
        })
        .await
        .map_err(|e| anyhow::anyhow!("WASM plugin task panicked: {e}"))??;

        marshal::deserialize_response(&output)
    }

    fn name(&self) -> &'static str {
        self.name
    }
}
