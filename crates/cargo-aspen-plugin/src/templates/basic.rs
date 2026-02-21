//! Basic plugin template — handles Ping → Pong.

use super::RenderedTemplate;
use super::TemplateContext;

pub fn render(ctx: &TemplateContext<'_>) -> RenderedTemplate {
    let struct_name = ctx.struct_name();

    let plugin_json = format!(
        r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "description": "{description}",
  "handles": ["Ping"],
  "priority": {priority},
  "permissions": {{
    "kv_read": true
  }}
}}"#,
        name = ctx.name,
        description = ctx.description,
        priority = ctx.priority,
    );

    let lib_rs = format!(
        r#"//! {description}

use aspen_wasm_guest_sdk::AspenPlugin;
use aspen_wasm_guest_sdk::ClientRpcRequest;
use aspen_wasm_guest_sdk::ClientRpcResponse;
use aspen_wasm_guest_sdk::PluginInfo;
use aspen_wasm_guest_sdk::PluginPermissions;
use aspen_wasm_guest_sdk::register_plugin;

struct {struct_name};

impl AspenPlugin for {struct_name} {{
    fn info() -> PluginInfo {{
        PluginInfo {{
            name: "{name}".to_string(),
            version: "0.1.0".to_string(),
            handles: vec!["Ping".to_string()],
            priority: {priority},
            app_id: None,
            kv_prefixes: vec![],
            permissions: PluginPermissions {{
                kv_read: true,
                ..PluginPermissions::default()
            }},
        }}
    }}

    fn init() -> Result<(), String> {{
        aspen_wasm_guest_sdk::host::log_info_msg("{name}: initialized");
        Ok(())
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
    );

    RenderedTemplate {
        cargo_toml: super::cargo_toml(ctx),
        plugin_json,
        lib_rs,
        readme: super::readme(ctx),
    }
}
