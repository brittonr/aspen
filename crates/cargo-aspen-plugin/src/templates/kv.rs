//! KV counter plugin template — distributed counter using KV store.

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
  "handles": ["WriteKey", "ReadKey", "DeleteKey"],
  "priority": {priority},
  "kv_prefixes": ["{prefix}"],
  "permissions": {{
    "kv_read": true,
    "kv_write": true
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
//! Stores data under the `{prefix}` KV prefix. Handles read, write, and delete
//! operations with automatic key prefixing.

use aspen_wasm_guest_sdk::AspenPlugin;
use aspen_wasm_guest_sdk::ClientRpcRequest;
use aspen_wasm_guest_sdk::ClientRpcResponse;
use aspen_wasm_guest_sdk::PluginInfo;
use aspen_wasm_guest_sdk::PluginPermissions;
use aspen_wasm_guest_sdk::ReadResultResponse;
use aspen_wasm_guest_sdk::register_plugin;

const KV_PREFIX: &str = "{prefix}";

struct {struct_name};

impl AspenPlugin for {struct_name} {{
    fn info() -> PluginInfo {{
        PluginInfo {{
            name: "{name}".to_string(),
            version: "0.1.0".to_string(),
            handles: vec![
                "WriteKey".to_string(),
                "ReadKey".to_string(),
                "DeleteKey".to_string(),
            ],
            priority: {priority},
            app_id: None,
            kv_prefixes: vec![KV_PREFIX.to_string()],
            permissions: PluginPermissions {{
                kv_read: true,
                kv_write: true,
                ..PluginPermissions::default()
            }},
        }}
    }}

    fn init() -> Result<(), String> {{
        aspen_wasm_guest_sdk::host::log_info_msg("{name}: initialized with prefix '{prefix}'");
        Ok(())
    }}

    fn handle(request: ClientRpcRequest) -> ClientRpcResponse {{
        match request {{
            ClientRpcRequest::WriteKey {{ ref key, ref value }} => {{
                let prefixed = format!("{{}}{{}}", KV_PREFIX, key);
                match aspen_wasm_guest_sdk::host::kv_put_value(&prefixed, value) {{
                    Ok(()) => ClientRpcResponse::Pong, // success
                    Err(e) => ClientRpcResponse::Error(
                        aspen_wasm_guest_sdk::response::error_response("KV_WRITE_ERROR", &e),
                    ),
                }}
            }}

            ClientRpcRequest::ReadKey {{ ref key }} => {{
                let prefixed = format!("{{}}{{}}", KV_PREFIX, key);
                match aspen_wasm_guest_sdk::host::kv_get_value(&prefixed) {{
                    Ok(Some(data)) => ClientRpcResponse::ReadResult(ReadResultResponse {{
                        value: Some(data),
                        was_found: true,
                        error: None,
                    }}),
                    Ok(None) => ClientRpcResponse::ReadResult(ReadResultResponse {{
                        value: None,
                        was_found: false,
                        error: None,
                    }}),
                    Err(e) => ClientRpcResponse::ReadResult(ReadResultResponse {{
                        value: None,
                        was_found: false,
                        error: Some(e),
                    }}),
                }}
            }}

            ClientRpcRequest::DeleteKey {{ ref key }} => {{
                let prefixed = format!("{{}}{{}}", KV_PREFIX, key);
                match aspen_wasm_guest_sdk::host::kv_delete_key(&prefixed) {{
                    Ok(()) => ClientRpcResponse::Pong, // success
                    Err(e) => ClientRpcResponse::Error(
                        aspen_wasm_guest_sdk::response::error_response("KV_DELETE_ERROR", &e),
                    ),
                }}
            }}

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
