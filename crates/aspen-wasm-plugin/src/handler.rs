//! WASM plugin handler that delegates to a sandboxed guest.
//!
//! `WasmPluginHandler` wraps a loaded hyperlight-wasm sandbox and
//! implements `RequestHandler` so the `HandlerRegistry` can dispatch
//! matching requests to the WASM guest's `handle_request` export.
//!
//! Sandbox calls are executed via `spawn_blocking` since hyperlight
//! operations are CPU-bound and `LoadedWasmSandbox` is not `Send`.

use std::sync::Arc;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering;
use std::time::Duration;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_plugin_api::PluginHealth;
use aspen_plugin_api::PluginState;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;

use crate::marshal;

/// A request handler backed by a WASM plugin running in a hyperlight-wasm sandbox.
///
/// The sandbox is wrapped in a `std::sync::Mutex` because `call_guest_function`
/// requires `&mut self`. All sandbox calls go through `spawn_blocking` to avoid
/// blocking the async executor, with a wall-clock timeout to prevent runaway
/// guest execution.
pub struct WasmPluginHandler {
    /// Plugin name (leaked for 'static lifetime requirement of `RequestHandler::name`).
    name: &'static str,
    /// Request variant names this plugin handles.
    handles: Vec<String>,
    /// The loaded WASM sandbox. Mutex because `call_guest_function` takes `&mut`.
    sandbox: Arc<std::sync::Mutex<hyperlight_wasm::LoadedWasmSandbox>>,
    /// Wall-clock execution timeout for a single guest call.
    ///
    /// Tiger Style: Bounded execution prevents runaway plugins from blocking
    /// the handler indefinitely.
    execution_timeout: Duration,
    /// Plugin lifecycle state.
    ///
    /// Encoded as u8: 0=Loading, 1=Initializing, 2=Ready, 3=Degraded,
    /// 4=Stopping, 5=Stopped, 6=Failed.
    ///
    /// Tiger Style: Atomic state tracking enables concurrent health checks
    /// and graceful shutdown without blocking request processing.
    state: Arc<AtomicU8>,
}

impl WasmPluginHandler {
    /// Create a new WASM plugin handler.
    ///
    /// # Arguments
    ///
    /// * `name` - Plugin name (will be leaked for 'static lifetime)
    /// * `handles` - Request variant names this plugin handles
    /// * `sandbox` - The loaded hyperlight-wasm sandbox
    /// * `execution_timeout` - Wall-clock timeout for guest calls
    pub fn new(
        name: String,
        handles: Vec<String>,
        sandbox: hyperlight_wasm::LoadedWasmSandbox,
        execution_timeout: Duration,
    ) -> Self {
        Self {
            name: Box::leak(name.into_boxed_str()),
            handles,
            sandbox: Arc::new(std::sync::Mutex::new(sandbox)),
            execution_timeout,
            state: Arc::new(AtomicU8::new(0)), // Loading
        }
    }

    /// Get the current plugin state.
    pub fn state(&self) -> PluginState {
        match self.state.load(Ordering::SeqCst) {
            0 => PluginState::Loading,
            1 => PluginState::Initializing,
            2 => PluginState::Ready,
            3 => PluginState::Degraded,
            4 => PluginState::Stopping,
            5 => PluginState::Stopped,
            6 => PluginState::Failed,
            _ => PluginState::Failed,
        }
    }

    /// Set the plugin state.
    pub fn set_state(&self, state: PluginState) {
        let value = match state {
            PluginState::Loading => 0,
            PluginState::Initializing => 1,
            PluginState::Ready => 2,
            PluginState::Degraded => 3,
            PluginState::Stopping => 4,
            PluginState::Stopped => 5,
            PluginState::Failed => 6,
        };
        self.state.store(value, Ordering::SeqCst);
    }

    /// Get the plugin name.
    pub fn plugin_name(&self) -> &str {
        self.name
    }

    /// Call the plugin's `plugin_init` export.
    ///
    /// Sets state to Initializing, calls the guest export, and transitions
    /// to Ready on success or Failed on error.
    ///
    /// Tiger Style: Explicit initialization contract lets plugins perform
    /// setup logic before handling requests.
    pub async fn call_init(&self) -> anyhow::Result<()> {
        self.set_state(PluginState::Initializing);

        let sandbox = Arc::clone(&self.sandbox);
        let handler_name = self.name;
        let timeout = self.execution_timeout;

        let result = tokio::time::timeout(
            timeout,
            tokio::task::spawn_blocking(move || {
                let mut guard = sandbox.lock().map_err(|e| anyhow::anyhow!("sandbox mutex poisoned: {e}"))?;
                guard
                    .call_guest_function::<Vec<u8>>("plugin_init", Vec::<u8>::new())
                    .map_err(|e| anyhow::anyhow!("WASM plugin '{handler_name}' init failed: {e}"))
            }),
        )
        .await;

        match result {
            Ok(Ok(Ok(output))) => {
                // Parse JSON response: {"ok": true/false, "error": "..."}
                let response: serde_json::Value = serde_json::from_slice(&output)
                    .map_err(|e| anyhow::anyhow!("Failed to parse init response: {e}"))?;

                if response["ok"].as_bool().unwrap_or(false) {
                    self.set_state(PluginState::Ready);
                    Ok(())
                } else {
                    let error = response["error"].as_str().unwrap_or("unknown error").to_string();
                    self.set_state(PluginState::Failed);
                    anyhow::bail!("Plugin init failed: {}", error);
                }
            }
            Ok(Ok(Err(e))) => {
                self.set_state(PluginState::Failed);
                Err(e)
            }
            Ok(Err(e)) => {
                self.set_state(PluginState::Failed);
                anyhow::bail!("WASM plugin task panicked: {e}")
            }
            Err(_) => {
                self.set_state(PluginState::Failed);
                tracing::warn!(
                    plugin = handler_name,
                    timeout_secs = timeout.as_secs(),
                    "WASM plugin init exceeded execution timeout"
                );
                anyhow::bail!(
                    "WASM plugin '{}' init exceeded execution timeout of {}s",
                    handler_name,
                    timeout.as_secs()
                )
            }
        }
    }

    /// Call the plugin's `plugin_shutdown` export.
    ///
    /// Sets state to Stopping, calls the guest export, and transitions
    /// to Stopped.
    ///
    /// Tiger Style: Explicit shutdown contract enables graceful cleanup
    /// and resource deallocation.
    pub async fn call_shutdown(&self) -> anyhow::Result<()> {
        self.set_state(PluginState::Stopping);

        let sandbox = Arc::clone(&self.sandbox);
        let handler_name = self.name;
        let timeout = self.execution_timeout;

        let result = tokio::time::timeout(
            timeout,
            tokio::task::spawn_blocking(move || {
                let mut guard = sandbox.lock().map_err(|e| anyhow::anyhow!("sandbox mutex poisoned: {e}"))?;
                guard
                    .call_guest_function::<Vec<u8>>("plugin_shutdown", Vec::<u8>::new())
                    .map_err(|e| anyhow::anyhow!("WASM plugin '{handler_name}' shutdown failed: {e}"))
            }),
        )
        .await;

        self.set_state(PluginState::Stopped);

        match result {
            Ok(Ok(Ok(_))) => Ok(()),
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(e)) => anyhow::bail!("WASM plugin task panicked: {e}"),
            Err(_) => {
                tracing::warn!(
                    plugin = handler_name,
                    timeout_secs = timeout.as_secs(),
                    "WASM plugin shutdown exceeded execution timeout"
                );
                // Still set to Stopped even if timeout
                Ok(())
            }
        }
    }

    /// Call the plugin's `plugin_health` export.
    ///
    /// Returns PluginHealth::healthy on success, PluginHealth::degraded
    /// on error or timeout (and sets state to Degraded).
    ///
    /// Tiger Style: Health checks enable proactive monitoring and degraded
    /// operation detection without taking the plugin offline.
    pub async fn call_health(&self) -> PluginHealth {
        let sandbox = Arc::clone(&self.sandbox);
        let handler_name = self.name;
        let timeout = Duration::from_secs(5);

        let result = tokio::time::timeout(
            timeout,
            tokio::task::spawn_blocking(move || {
                let mut guard = sandbox.lock().map_err(|e| anyhow::anyhow!("sandbox mutex poisoned: {e}"))?;
                guard
                    .call_guest_function::<Vec<u8>>("plugin_health", Vec::<u8>::new())
                    .map_err(|e| anyhow::anyhow!("WASM plugin '{handler_name}' health check failed: {e}"))
            }),
        )
        .await;

        match result {
            Ok(Ok(Ok(output))) => {
                // Parse JSON response for health message
                if let Ok(response) = serde_json::from_slice::<serde_json::Value>(&output)
                    && response["ok"].as_bool().unwrap_or(false)
                {
                    let message = response["message"].as_str().unwrap_or("healthy").to_string();
                    return PluginHealth::healthy(message);
                }
                self.set_state(PluginState::Degraded);
                PluginHealth::degraded("health check returned non-ok response".to_string())
            }
            Ok(Ok(Err(e))) => {
                self.set_state(PluginState::Degraded);
                PluginHealth::degraded(format!("health check failed: {}", e))
            }
            Ok(Err(e)) => {
                self.set_state(PluginState::Degraded);
                PluginHealth::degraded(format!("health check task panicked: {}", e))
            }
            Err(_) => {
                self.set_state(PluginState::Degraded);
                PluginHealth::degraded(format!("health check exceeded timeout of {}s", timeout.as_secs()))
            }
        }
    }
}

#[async_trait::async_trait]
impl RequestHandler for WasmPluginHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        let name = marshal::extract_variant_name(request);
        self.handles.iter().any(|h| h == name)
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        _ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        // Check state before dispatching
        let current_state = self.state();
        match current_state {
            PluginState::Ready | PluginState::Degraded => {
                // OK to process
            }
            _ => {
                anyhow::bail!("Plugin '{}' is not ready (state: {:?})", self.name, current_state);
            }
        }

        let input = marshal::serialize_request(&request)?;
        let sandbox = Arc::clone(&self.sandbox);
        let handler_name = self.name;
        let timeout = self.execution_timeout;

        let output = tokio::time::timeout(
            timeout,
            tokio::task::spawn_blocking(move || {
                let mut guard = sandbox.lock().map_err(|e| anyhow::anyhow!("sandbox mutex poisoned: {e}"))?;
                guard
                    .call_guest_function::<Vec<u8>>("handle_request", input)
                    .map_err(|e| anyhow::anyhow!("WASM plugin '{handler_name}' execution failed: {e}"))
            }),
        )
        .await
        .map_err(|_| {
            tracing::warn!(
                plugin = handler_name,
                timeout_secs = timeout.as_secs(),
                "WASM plugin exceeded execution timeout"
            );
            anyhow::anyhow!("WASM plugin '{}' exceeded execution timeout of {}s", handler_name, timeout.as_secs())
        })?
        .map_err(|e| anyhow::anyhow!("WASM plugin task panicked: {e}"))??;

        marshal::deserialize_response(&output)
    }

    fn name(&self) -> &'static str {
        self.name
    }
}
