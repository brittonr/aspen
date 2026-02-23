//! Hooks handler WASM plugin.
//!
//! Migrated from the native `aspen-hooks-handler` crate. Handles hook
//! system operations: listing configured handlers, getting execution
//! metrics, and manually triggering events.
//!
//! Uses host functions `hook_list`, `hook_metrics`, and `hook_trigger`
//! to communicate with the native HookService via the plugin host context.

use aspen_wasm_guest_sdk::*;

struct HooksHandlerPlugin;

impl AspenPlugin for HooksHandlerPlugin {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "aspen-hooks-handler".to_string(),
            version: "0.1.0".to_string(),
            handles: vec![
                "HookList".to_string(),
                "HookGetMetrics".to_string(),
                "HookTrigger".to_string(),
            ],
            priority: 570,
            app_id: Some("hooks".to_string()),
            kv_prefixes: vec![],
            permissions: PluginPermissions {
                hooks: true,
                ..PluginPermissions::default()
            },
        }
    }

    fn init() -> Result<(), String> {
        host::log_info_msg("aspen-hooks-handler: initialized");
        Ok(())
    }

    fn handle(request: ClientRpcRequest) -> ClientRpcResponse {
        match request {
            ClientRpcRequest::HookList => handle_hook_list(),
            ClientRpcRequest::HookGetMetrics { handler_name } => handle_hook_metrics(handler_name),
            ClientRpcRequest::HookTrigger {
                event_type,
                payload_json,
            } => handle_hook_trigger(event_type, payload_json),

            _ => ClientRpcResponse::Error(response::error_response(
                "UNHANDLED",
                "aspen-hooks-handler does not handle this request type",
            )),
        }
    }
}

/// Handle HookList request.
///
/// Returns information about all configured hook handlers.
fn handle_hook_list() -> ClientRpcResponse {
    match host::list_hooks() {
        Ok(result) => {
            let handlers: Vec<HookHandlerInfo> = result
                .handlers
                .into_iter()
                .map(|h| HookHandlerInfo {
                    name: h.name,
                    pattern: h.pattern,
                    handler_type: h.handler_type,
                    execution_mode: h.execution_mode,
                    is_enabled: h.enabled,
                    timeout_ms: h.timeout_ms,
                    retry_count: h.retry_count,
                })
                .collect();

            ClientRpcResponse::HookListResult(HookListResultResponse {
                is_enabled: result.is_enabled,
                handlers,
            })
        }
        Err(_e) => ClientRpcResponse::HookListResult(HookListResultResponse {
            is_enabled: false,
            handlers: vec![],
        }),
    }
}

/// Handle HookGetMetrics request.
///
/// Returns execution metrics for hook handlers, optionally filtered
/// by handler name.
fn handle_hook_metrics(handler_name: Option<String>) -> ClientRpcResponse {
    let filter = handler_name.as_deref().unwrap_or("");
    match host::get_hook_metrics(filter) {
        Ok(result) => {
            let handlers: Vec<HookHandlerMetrics> = result
                .handlers
                .into_iter()
                .map(|m| HookHandlerMetrics {
                    name: m.name,
                    success_count: m.success_count,
                    failure_count: m.failure_count,
                    dropped_count: m.dropped_count,
                    jobs_submitted: m.jobs_submitted,
                    avg_duration_us: m.avg_duration_us,
                    max_duration_us: m.max_duration_us,
                })
                .collect();

            ClientRpcResponse::HookMetricsResult(HookMetricsResultResponse {
                is_enabled: result.is_enabled,
                total_events_processed: result.total_events_processed,
                handlers,
            })
        }
        Err(_e) => ClientRpcResponse::HookMetricsResult(HookMetricsResultResponse {
            is_enabled: false,
            total_events_processed: 0,
            handlers: vec![],
        }),
    }
}

/// Handle HookTrigger request.
///
/// Manually triggers a hook event for testing purposes.
fn handle_hook_trigger(event_type: String, payload_json: String) -> ClientRpcResponse {
    let payload: serde_json::Value = serde_json::from_str(&payload_json).unwrap_or_else(|_| serde_json::json!({}));

    match host::trigger_hook(&event_type, &payload) {
        Ok(result) => {
            let handler_failures: Vec<(String, String)> = result
                .handler_failures
                .into_iter()
                .filter_map(|pair| {
                    if pair.len() >= 2 {
                        Some((pair[0].clone(), pair[1].clone()))
                    } else {
                        None
                    }
                })
                .collect();

            ClientRpcResponse::HookTriggerResult(HookTriggerResultResponse {
                is_success: result.is_success,
                dispatched_count: result.dispatched_count,
                error: result.error,
                handler_failures,
            })
        }
        Err(e) => ClientRpcResponse::HookTriggerResult(HookTriggerResultResponse {
            is_success: false,
            dispatched_count: 0,
            error: Some(e),
            handler_failures: vec![],
        }),
    }
}

register_plugin!(HooksHandlerPlugin);
