//! Audit Logger plugin — writes an append-only log of KV events.
//!
//! Demonstrates:
//! - Hook subscriptions for event-driven processing
//! - Append-only audit trail pattern using timestamped KV keys
//! - Reading audit entries back via scan
//!
//! ## How It Works
//!
//! On init, subscribes to `hooks.kv.*` events. When a KV write or delete
//! occurs, the hook fires and this plugin writes an audit entry under
//! `audit:log:{timestamp_ms}` with the event details.
//!
//! ## Querying the Audit Log
//!
//! ReadKey { key: "stats" } → returns total event count
//! Ping → health check

use aspen_wasm_guest_sdk::AspenPlugin;
use aspen_wasm_guest_sdk::ClientRpcRequest;
use aspen_wasm_guest_sdk::ClientRpcResponse;
use aspen_wasm_guest_sdk::PluginInfo;
use aspen_wasm_guest_sdk::PluginPermissions;
use aspen_wasm_guest_sdk::ReadResultResponse;
use aspen_wasm_guest_sdk::register_plugin;

const KV_PREFIX: &str = "audit:";
const LOG_PREFIX: &str = "audit:log:";

struct AuditLogger;

impl AspenPlugin for AuditLogger {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "audit-logger".to_string(),
            version: "0.1.0".to_string(),
            handles: vec!["Ping".to_string(), "ReadKey".to_string()],
            priority: 960,
            app_id: None,
            kv_prefixes: vec![KV_PREFIX.to_string()],
            permissions: PluginPermissions {
                kv_read: true,
                kv_write: true,
                hooks: true,
                ..PluginPermissions::default()
            },
        }
    }

    fn init() -> Result<(), String> {
        // Subscribe to all KV write and delete events
        aspen_wasm_guest_sdk::host::subscribe_hook_events("hooks.kv.*")?;
        aspen_wasm_guest_sdk::host::log_info_msg("audit-logger: initialized, subscribed to hooks.kv.*");
        Ok(())
    }

    fn on_hook_event(topic: &str, event: &[u8]) {
        let now = aspen_wasm_guest_sdk::host::current_time_ms();
        let event_str = String::from_utf8_lossy(event);

        // Write audit entry with timestamp key for natural ordering
        let log_key = format!("{LOG_PREFIX}{now}");
        let log_entry = format!(r#"{{"topic":"{topic}","timestamp_ms":{now},"event":{event_str}}}"#);

        if let Err(e) = aspen_wasm_guest_sdk::host::kv_put_value(&log_key, log_entry.as_bytes()) {
            aspen_wasm_guest_sdk::host::log_warn_msg(&format!("audit-logger: write failed: {e}"));
        }
    }

    fn handle(request: ClientRpcRequest) -> ClientRpcResponse {
        match request {
            ClientRpcRequest::Ping => ClientRpcResponse::Pong,

            ClientRpcRequest::ReadKey { ref key } if key == "stats" => {
                // Return count of audit entries
                match aspen_wasm_guest_sdk::host::kv_scan_prefix(LOG_PREFIX, 10_000) {
                    Ok(entries) => ClientRpcResponse::ReadResult(ReadResultResponse {
                        value: Some(format!("{{\"total_events\":{}}}", entries.len()).into_bytes()),
                        was_found: true,
                        error: None,
                    }),
                    Err(e) => ClientRpcResponse::ReadResult(ReadResultResponse {
                        value: None,
                        was_found: false,
                        error: Some(e),
                    }),
                }
            }

            _ => ClientRpcResponse::Error(aspen_wasm_guest_sdk::response::error_response(
                "UNHANDLED",
                "audit-logger does not handle this request type",
            )),
        }
    }
}

register_plugin!(AuditLogger);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_info_matches_manifest() {
        let manifest_bytes = include_bytes!("../plugin.json");
        let manifest: PluginInfo = serde_json::from_slice(manifest_bytes).expect("plugin.json should be valid");
        let info = AuditLogger::info();
        assert_eq!(info.name, manifest.name);
        assert_eq!(info.handles, manifest.handles);
        assert_eq!(info.priority, manifest.priority);
        assert_eq!(info.kv_prefixes, manifest.kv_prefixes);
    }
}
