//! WASM guest plugin for the Aspen hooks handler.
//!
//! This crate compiles to `wasm32-wasip2` and exports `handle_request`
//! and `plugin_info` for the Aspen plugin runtime. It reimplements the
//! native `HooksHandler` using host-provided KV operations
//! through the `aspen-wasm-guest-sdk`.
//!
//! # Handled Request Types
//!
//! - `HookList` — List configured hook handlers (from KV-stored config)
//! - `HookGetMetrics` — Get execution metrics for handlers (from KV-stored metrics)
//! - `HookTrigger` — Manually trigger a hook event (writes to KV for native dispatch)
//!
//! # KV Layout
//!
//! | Key | Value |
//! |-----|-------|
//! | `__hooks:config` | JSON-serialized `HooksConfig` |
//! | `__hooks:metrics` | JSON-serialized `HooksMetrics` |
//! | `__hooks:trigger:{ts}:{type}` | JSON-serialized `HookEvent` (pending triggers) |

mod handlers;
mod kv;
mod types;

use aspen_wasm_guest_sdk::AspenPlugin;
use aspen_wasm_guest_sdk::ClientRpcRequest;
use aspen_wasm_guest_sdk::ClientRpcResponse;
use aspen_wasm_guest_sdk::PluginInfo;
use aspen_wasm_guest_sdk::register_plugin;

struct HooksPlugin;

impl AspenPlugin for HooksPlugin {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "hooks".to_string(),
            version: "0.1.0".to_string(),
            handles: vec![
                "HookList".to_string(),
                "HookGetMetrics".to_string(),
                "HookTrigger".to_string(),
            ],
            priority: 950,
            app_id: Some("hooks".to_string()),
        }
    }

    fn handle(request: ClientRpcRequest) -> ClientRpcResponse {
        match request {
            ClientRpcRequest::HookList => handlers::handle_hook_list(),

            ClientRpcRequest::HookGetMetrics { handler_name } => handlers::handle_hook_metrics(handler_name),

            ClientRpcRequest::HookTrigger {
                event_type,
                payload_json,
            } => handlers::handle_hook_trigger(event_type, payload_json),

            _ => ClientRpcResponse::Error(aspen_wasm_guest_sdk::response::error_response(
                "UNHANDLED",
                "hooks plugin does not handle this request type",
            )),
        }
    }
}

register_plugin!(HooksPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Plugin manifest consistency
    // ========================================================================

    #[test]
    fn plugin_info_matches_manifest() {
        let manifest_bytes = include_bytes!("../plugin.json");
        let manifest: PluginInfo = serde_json::from_slice(manifest_bytes).expect("plugin.json should be valid");
        let info = HooksPlugin::info();
        assert_eq!(info.name, manifest.name, "name mismatch between code and plugin.json");
        assert_eq!(info.handles, manifest.handles, "handles mismatch between code and plugin.json");
        assert_eq!(info.priority, manifest.priority, "priority mismatch between code and plugin.json");
        assert_eq!(info.version, manifest.version, "version mismatch between code and plugin.json");
        assert_eq!(info.app_id, manifest.app_id, "app_id mismatch between code and plugin.json");
    }

    // ========================================================================
    // Plugin metadata
    // ========================================================================

    #[test]
    fn plugin_info_name() {
        assert_eq!(HooksPlugin::info().name, "hooks");
    }

    #[test]
    fn plugin_info_version() {
        assert_eq!(HooksPlugin::info().version, "0.1.0");
    }

    #[test]
    fn plugin_info_handles_three_request_types() {
        let info = HooksPlugin::info();
        assert_eq!(info.handles.len(), 3);
        assert!(info.handles.contains(&"HookList".to_string()));
        assert!(info.handles.contains(&"HookGetMetrics".to_string()));
        assert!(info.handles.contains(&"HookTrigger".to_string()));
    }

    #[test]
    fn plugin_info_priority_in_plugin_range() {
        let info = HooksPlugin::info();
        assert!(
            (900..=999).contains(&info.priority),
            "plugin priority {} should be in WASM plugin range 900-999",
            info.priority
        );
    }

    #[test]
    fn plugin_info_has_app_id() {
        assert_eq!(HooksPlugin::info().app_id, Some("hooks".to_string()));
    }

    // ========================================================================
    // Manifest JSON is valid
    // ========================================================================

    #[test]
    fn plugin_json_is_valid_json() {
        let bytes = include_bytes!("../plugin.json");
        let value: serde_json::Value = serde_json::from_slice(bytes).expect("plugin.json should be valid JSON");
        assert!(value.is_object());
    }

    #[test]
    fn plugin_json_has_required_fields() {
        let bytes = include_bytes!("../plugin.json");
        let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        assert!(value["name"].is_string(), "missing 'name'");
        assert!(value["version"].is_string(), "missing 'version'");
        assert!(value["handles"].is_array(), "missing 'handles'");
        assert!(value["priority"].is_number(), "missing 'priority'");
    }

    #[test]
    fn plugin_json_handles_non_empty() {
        let bytes = include_bytes!("../plugin.json");
        let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        let handles = value["handles"].as_array().unwrap();
        assert!(!handles.is_empty(), "handles should not be empty");
    }
}
