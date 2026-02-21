//! Timer plugin template — periodic scheduled task.

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
    "timers": true
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
//! Registers a periodic timer on init and handles timer callbacks
//! to perform scheduled work (e.g., cleanup, aggregation).

use aspen_wasm_guest_sdk::AspenPlugin;
use aspen_wasm_guest_sdk::ClientRpcRequest;
use aspen_wasm_guest_sdk::ClientRpcResponse;
use aspen_wasm_guest_sdk::PluginInfo;
use aspen_wasm_guest_sdk::PluginPermissions;
use aspen_wasm_guest_sdk::TimerConfig;
use aspen_wasm_guest_sdk::register_plugin;

const CLEANUP_TIMER: &str = "cleanup";
const CLEANUP_INTERVAL_MS: u64 = 60_000; // 1 minute
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
                timers: true,
                ..PluginPermissions::default()
            }},
        }}
    }}

    fn init() -> Result<(), String> {{
        // Register a periodic cleanup timer
        aspen_wasm_guest_sdk::host::schedule_timer_on_host(&TimerConfig {{
            name: CLEANUP_TIMER.to_string(),
            interval_ms: CLEANUP_INTERVAL_MS,
            repeating: true,
        }})
        .map_err(|e| format!("failed to schedule timer: {{e}}"))?;

        aspen_wasm_guest_sdk::host::log_info_msg("{name}: initialized, cleanup timer scheduled");
        Ok(())
    }}

    fn on_timer(name: &str) {{
        if name == CLEANUP_TIMER {{
            // Perform periodic cleanup work here.
            // Example: scan for expired entries and delete them.
            let now = aspen_wasm_guest_sdk::host::current_time_ms();
            aspen_wasm_guest_sdk::host::log_info_msg(
                &format!("{name}: cleanup tick at {{now}}"),
            );

            // Scan for entries and process them
            match aspen_wasm_guest_sdk::host::kv_scan_prefix(KV_PREFIX, 100) {{
                Ok(entries) => {{
                    aspen_wasm_guest_sdk::host::log_info_msg(
                        &format!("{name}: scanned {{}} entries", entries.len()),
                    );
                }}
                Err(e) => {{
                    aspen_wasm_guest_sdk::host::log_warn_msg(
                        &format!("{name}: cleanup scan failed: {{e}}"),
                    );
                }}
            }}
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
