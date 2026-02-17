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

pub use handler::WasmPluginHandler;
pub use registry::PluginRegistry;
