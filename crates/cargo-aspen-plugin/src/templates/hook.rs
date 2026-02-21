//! Hook plugin template — subscribes to KV write events.

use super::RenderedTemplate;
use super::TemplateContext;

pub fn render(ctx: &TemplateContext<'_>) -> RenderedTemplate {
    let struct_name = ctx.struct_name();
    let prefix = format!("{}:", ctx.name);

    let plugin_json = format!(
        r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "description": "{description}",
  "handles": ["Ping"],
  "priority": {priority},
  "kv_prefixes": ["{prefix}"],
  "permissions": {{
    "kv_read": true,
    "kv_write": true,
    "hooks": true
  }}
}}"#,
        name = ctx.name,
        description = ctx.description,
        priority = ctx.priority,
        prefix = prefix,
    );

    let lib_rs = format!(
        r#"//! {description}
//!
//! Subscribes to hook events on init and processes them in `on_hook_event`.
//! Demonstrates event-driven plugin architecture.

use aspen_wasm_guest_sdk::AspenPlugin;
use aspen_wasm_guest_sdk::ClientRpcRequest;
use aspen_wasm_guest_sdk::ClientRpcResponse;
use aspen_wasm_guest_sdk::PluginInfo;
use aspen_wasm_guest_sdk::PluginPermissions;
use aspen_wasm_guest_sdk::register_plugin;

const KV_PREFIX: &str = "{prefix}";

struct {struct_name};

impl AspenPlugin for {struct_name} {{
    fn info() -> PluginInfo {{
        PluginInfo {{
            name: "{name}".to_string(),
            version: "0.1.0".to_string(),
            handles: vec!["Ping".to_string()],
            priority: {priority},
            app_id: None,
            kv_prefixes: vec![KV_PREFIX.to_string()],
            permissions: PluginPermissions {{
                kv_read: true,
                kv_write: true,
                hooks: true,
                ..PluginPermissions::default()
            }},
        }}
    }}

    fn init() -> Result<(), String> {{
        // Subscribe to all KV write events
        aspen_wasm_guest_sdk::host::subscribe_hook_events("hooks.kv.*")?;

        aspen_wasm_guest_sdk::host::log_info_msg("{name}: initialized, subscribed to hooks.kv.*");
        Ok(())
    }}

    fn on_hook_event(topic: &str, event: &[u8]) {{
        // Log every event we receive
        let event_str = String::from_utf8_lossy(event);
        aspen_wasm_guest_sdk::host::log_info_msg(
            &format!("{name}: event on '{{}}': {{}}", topic, event_str),
        );

        // Write an audit log entry
        let now = aspen_wasm_guest_sdk::host::current_time_ms();
        let log_key = format!("{{}}log:{{}}", KV_PREFIX, now);
        let log_value = format!("{{{{\"topic\":\"{{}}\",\"event\":{{}}}}}}", topic, event_str);

        if let Err(e) = aspen_wasm_guest_sdk::host::kv_put_value(&log_key, log_value.as_bytes()) {{
            aspen_wasm_guest_sdk::host::log_warn_msg(
                &format!("{name}: failed to write audit log: {{}}", e),
            );
        }}
    }}

    fn handle(request: ClientRpcRequest) -> ClientRpcResponse {{
        match request {{
            ClientRpcRequest::Ping => ClientRpcResponse::Pong,
            _ => ClientRpcResponse::Error(aspen_wasm_guest_sdk::response::error_response(
                "UNHANDLED",
                "{name} does not handle this request type",
            )),
        }}
    }}
}}

register_plugin!({struct_name});

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn plugin_info_matches_manifest() {{
        let manifest_bytes = include_bytes!("../plugin.json");
        let manifest: PluginInfo = serde_json::from_slice(manifest_bytes)
            .expect("plugin.json should be valid");
        let info = {struct_name}::info();
        assert_eq!(info.name, manifest.name);
        assert_eq!(info.handles, manifest.handles);
        assert_eq!(info.priority, manifest.priority);
    }}
}}
"#,
        name = ctx.name,
        description = ctx.description,
        struct_name = struct_name,
        priority = ctx.priority,
        prefix = prefix,
    );

    RenderedTemplate {
        cargo_toml: super::cargo_toml(ctx),
        plugin_json,
        lib_rs,
        readme: super::readme(ctx),
    }
}
