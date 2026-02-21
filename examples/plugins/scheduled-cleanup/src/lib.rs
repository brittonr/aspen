//! Scheduled Cleanup plugin — periodic expiry of old entries.
//!
//! Demonstrates:
//! - Timer scheduling with `schedule_timer_on_host`
//! - Periodic batch operations (scan + delete)
//! - TTL-based data expiry pattern
//!
//! ## How It Works
//!
//! Entries are stored as `cleanup:data:{key}` with a companion TTL entry
//! `cleanup:ttl:{key}` containing the expiry timestamp (Unix ms).
//!
//! Every 60 seconds, the cleanup timer fires and scans for expired TTL
//! entries, deleting both the data and TTL keys.
//!
//! ## Usage
//!
//! WriteKey { key: "mykey", value: "data" } → stores data + 5min TTL
//! ReadKey { key: "mykey" } → reads data (or not-found if expired)
//! Ping → health check

use aspen_wasm_guest_sdk::AspenPlugin;
use aspen_wasm_guest_sdk::ClientRpcRequest;
use aspen_wasm_guest_sdk::ClientRpcResponse;
use aspen_wasm_guest_sdk::KvBatchOp;
use aspen_wasm_guest_sdk::PluginInfo;
use aspen_wasm_guest_sdk::PluginPermissions;
use aspen_wasm_guest_sdk::ReadResultResponse;
use aspen_wasm_guest_sdk::TimerConfig;
use aspen_wasm_guest_sdk::register_plugin;

const DATA_PREFIX: &str = "cleanup:data:";
const TTL_PREFIX: &str = "cleanup:ttl:";
const KV_PREFIX: &str = "cleanup:";
const CLEANUP_TIMER: &str = "cleanup";
const CLEANUP_INTERVAL_MS: u64 = 60_000;
const DEFAULT_TTL_MS: u64 = 300_000; // 5 minutes

struct ScheduledCleanup;

impl AspenPlugin for ScheduledCleanup {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "scheduled-cleanup".to_string(),
            version: "0.1.0".to_string(),
            handles: vec!["Ping".to_string(), "ReadKey".to_string(), "WriteKey".to_string()],
            priority: 945,
            app_id: None,
            kv_prefixes: vec![KV_PREFIX.to_string()],
            permissions: PluginPermissions {
                kv_read: true,
                kv_write: true,
                timers: true,
                ..PluginPermissions::default()
            },
        }
    }

    fn init() -> Result<(), String> {
        aspen_wasm_guest_sdk::host::schedule_timer_on_host(&TimerConfig {
            name: CLEANUP_TIMER.to_string(),
            interval_ms: CLEANUP_INTERVAL_MS,
            repeating: true,
        })
        .map_err(|e| format!("failed to schedule cleanup timer: {e}"))?;

        aspen_wasm_guest_sdk::host::log_info_msg("scheduled-cleanup: initialized, timer set for 60s intervals");
        Ok(())
    }

    fn on_timer(name: &str) {
        if name != CLEANUP_TIMER {
            return;
        }

        let now = aspen_wasm_guest_sdk::host::current_time_ms();

        // Scan all TTL entries
        let ttl_entries = match aspen_wasm_guest_sdk::host::kv_scan_prefix(TTL_PREFIX, 1000) {
            Ok(entries) => entries,
            Err(e) => {
                aspen_wasm_guest_sdk::host::log_warn_msg(&format!("scheduled-cleanup: TTL scan failed: {e}"));
                return;
            }
        };

        let mut expired_ops = Vec::new();
        for (ttl_key, ttl_value) in &ttl_entries {
            let expiry_str = String::from_utf8_lossy(ttl_value);
            let expiry_ms: u64 = expiry_str.parse().unwrap_or(0);

            if expiry_ms > 0 && expiry_ms <= now {
                // Extract the user key from the TTL key
                let user_key = ttl_key.strip_prefix(TTL_PREFIX).unwrap_or(ttl_key);
                let data_key = format!("{DATA_PREFIX}{user_key}");

                expired_ops.push(KvBatchOp::Delete { key: data_key });
                expired_ops.push(KvBatchOp::Delete { key: ttl_key.clone() });
            }
        }

        if !expired_ops.is_empty() {
            let count = expired_ops.len() / 2;
            match aspen_wasm_guest_sdk::host::kv_batch_write(&expired_ops) {
                Ok(()) => {
                    aspen_wasm_guest_sdk::host::log_info_msg(&format!("scheduled-cleanup: expired {count} entries"));
                }
                Err(e) => {
                    aspen_wasm_guest_sdk::host::log_warn_msg(&format!("scheduled-cleanup: batch delete failed: {e}"));
                }
            }
        }
    }

    fn handle(request: ClientRpcRequest) -> ClientRpcResponse {
        match request {
            ClientRpcRequest::Ping => ClientRpcResponse::Pong,

            ClientRpcRequest::WriteKey { ref key, ref value } => {
                let data_key = format!("{DATA_PREFIX}{key}");
                let ttl_key = format!("{TTL_PREFIX}{key}");
                let now = aspen_wasm_guest_sdk::host::current_time_ms();
                let expiry = (now + DEFAULT_TTL_MS).to_string();

                // Batch write: data + TTL together
                let ops = vec![
                    KvBatchOp::Set {
                        key: data_key,
                        value: String::from_utf8_lossy(value).to_string(),
                    },
                    KvBatchOp::Set {
                        key: ttl_key,
                        value: expiry,
                    },
                ];

                match aspen_wasm_guest_sdk::host::kv_batch_write(&ops) {
                    Ok(()) => ClientRpcResponse::Pong,
                    Err(e) => {
                        ClientRpcResponse::Error(aspen_wasm_guest_sdk::response::error_response("WRITE_FAILED", &e))
                    }
                }
            }

            ClientRpcRequest::ReadKey { ref key } => {
                let data_key = format!("{DATA_PREFIX}{key}");
                match aspen_wasm_guest_sdk::host::kv_get_value(&data_key) {
                    Ok(Some(data)) => ClientRpcResponse::ReadResult(ReadResultResponse {
                        value: Some(data),
                        was_found: true,
                        error: None,
                    }),
                    Ok(None) => ClientRpcResponse::ReadResult(ReadResultResponse {
                        value: None,
                        was_found: false,
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
                "scheduled-cleanup does not handle this request type",
            )),
        }
    }
}

register_plugin!(ScheduledCleanup);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_info_matches_manifest() {
        let manifest_bytes = include_bytes!("../plugin.json");
        let manifest: PluginInfo = serde_json::from_slice(manifest_bytes).expect("plugin.json should be valid");
        let info = ScheduledCleanup::info();
        assert_eq!(info.name, manifest.name);
        assert_eq!(info.handles, manifest.handles);
        assert_eq!(info.priority, manifest.priority);
        assert_eq!(info.kv_prefixes, manifest.kv_prefixes);
    }
}
