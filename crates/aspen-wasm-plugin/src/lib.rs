//! Host-side WASM plugin runtime for Aspen RPC handlers.
//!
//! Loads WASM handler plugins from the cluster store, executes them in
//! hyperlight-wasm sandboxes, and bridges between the `HandlerRegistry`
//! and guest `handle_request` exports.
//!
//! ## Sandbox Lifecycle
//!
//! 1. `PluginRegistry::load_all` scans the KV store for plugin manifests
//! 2. WASM bytes are fetched from blob storage via the manifest's `wasm_hash`
//! 3. A hyperlight-wasm sandbox is created with host functions registered
//! 4. The guest's `plugin_info` export is called to validate the manifest
//! 5. A `WasmPluginHandler` wraps the loaded sandbox as a `RequestHandler`
//!
//! ## Host Functions
//!
//! Plugins have access to the same host functions as job workers (kv, blob,
//! logging, clock) plus additional identity, randomness, and cluster-state
//! functions. See the `host` module for details.

mod handler;
mod host;
mod marshal;
mod registry;

pub use aspen_plugin_api::PluginHealth;
pub use aspen_plugin_api::PluginState;
pub use handler::WasmPluginHandler;
pub use registry::LivePluginRegistry;
pub use registry::PluginRegistry;

/// Test utilities for loading WASM plugins in integration tests.
///
/// Provides `load_wasm_handler` which encapsulates the full sandbox creation
/// pipeline: build sandbox, register host functions, load runtime, load WASM
/// module, validate `plugin_info`, and construct a `WasmPluginHandler`.
#[cfg(any(test, feature = "testing"))]
pub mod test_support {
    use std::sync::Arc;

    pub use crate::handler::WasmPluginHandler;
    pub use crate::host::PluginHostContext;
    pub use crate::host::register_plugin_host_functions;

    /// Load a WASM handler plugin from raw bytes.
    ///
    /// Creates a hyperlight-wasm sandbox, registers all host functions,
    /// loads the WASM module, calls `plugin_info` to extract the plugin
    /// identity, validates the name matches `expected_name`, and returns
    /// a fully constructed `WasmPluginHandler`.
    ///
    /// Uses `DEFAULT_WASM_EXECUTION_TIMEOUT_SECS` for the execution timeout.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Sandbox creation fails (requires `/dev/kvm`)
    /// - Host function registration fails
    /// - WASM module loading fails (invalid binary)
    /// - `plugin_info` export is missing or returns invalid JSON
    /// - Plugin name from guest does not match `expected_name`
    pub fn load_wasm_handler(
        wasm_bytes: &[u8],
        expected_name: &str,
        ctx: Arc<PluginHostContext>,
        memory_limit: u64,
    ) -> anyhow::Result<WasmPluginHandler> {
        let mut proto = hyperlight_wasm::SandboxBuilder::new()
            .with_guest_heap_size(memory_limit)
            .build()
            .map_err(|e| anyhow::anyhow!("failed to create sandbox: {e}"))?;

        register_plugin_host_functions(&mut proto, ctx)?;

        let wasm_sb = proto.load_runtime().map_err(|e| anyhow::anyhow!("failed to load WASM runtime: {e}"))?;

        let mut loaded = wasm_sb
            .load_module_from_buffer(wasm_bytes)
            .map_err(|e| anyhow::anyhow!("failed to load WASM module: {e}"))?;

        // Call plugin_info to extract identity
        let info_bytes: Vec<u8> = loaded
            .call_guest_function("plugin_info", Vec::<u8>::new())
            .map_err(|e| anyhow::anyhow!("failed to call plugin_info: {e}"))?;
        let info: aspen_plugin_api::PluginInfo =
            serde_json::from_slice(&info_bytes).map_err(|e| anyhow::anyhow!("invalid plugin_info JSON: {e}"))?;

        if info.name != expected_name {
            return Err(anyhow::anyhow!(
                "plugin name mismatch: expected '{}', guest says '{}'",
                expected_name,
                info.name
            ));
        }

        let execution_timeout =
            std::time::Duration::from_secs(aspen_constants::wasm::DEFAULT_WASM_EXECUTION_TIMEOUT_SECS);

        Ok(WasmPluginHandler::new(info.name, info.handles, loaded, execution_timeout))
    }
}
