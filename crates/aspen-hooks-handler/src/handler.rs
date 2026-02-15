//! Hook system RPC handlers.
//!
//! Handles hook listing, metrics, and manual triggering through
//! the event-driven hook system.

use std::sync::Arc;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::HookHandlerInfo;
use aspen_client_api::HookHandlerMetrics;
use aspen_client_api::HookListResultResponse;
use aspen_client_api::HookMetricsResultResponse;
use aspen_client_api::HookTriggerResultResponse;
use aspen_hooks::HookEvent;
use aspen_hooks::HookEventType;
use aspen_hooks::HookService;
use aspen_hooks::config::ExecutionMode;
use aspen_hooks::service::DispatchResult;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use async_trait::async_trait;
use tracing::debug;
use tracing::info;

/// Handler for hook system operations.
///
/// Processes HookList, HookGetMetrics, and HookTrigger RPC requests.
///
/// # Tiger Style
///
/// - Read-only operations for listing and metrics
/// - Bounded event dispatch for triggers
/// - Clear error reporting for configuration issues
pub struct HooksHandler;

#[async_trait]
impl RequestHandler for HooksHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::HookList | ClientRpcRequest::HookGetMetrics { .. } | ClientRpcRequest::HookTrigger { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        // Check if hook service is available
        let hook_service = ctx.hook_service.as_ref();

        match request {
            ClientRpcRequest::HookList => handle_hook_list(hook_service, ctx),
            ClientRpcRequest::HookGetMetrics { handler_name } => handle_hook_metrics(hook_service, handler_name),
            ClientRpcRequest::HookTrigger {
                event_type,
                payload_json,
            } => {
                // Parse JSON payload from string (PostCard-compatible transport)
                let payload: serde_json::Value =
                    serde_json::from_str(&payload_json).unwrap_or_else(|_| serde_json::json!({}));
                handle_hook_trigger(hook_service, ctx.node_id, event_type, payload).await
            }
            _ => Err(anyhow::anyhow!("request not handled by HooksHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "HooksHandler"
    }
}

/// Handle HookList request.
///
/// Returns information about all configured handlers.
fn handle_hook_list(
    hook_service: Option<&Arc<HookService>>,
    ctx: &ClientProtocolContext,
) -> anyhow::Result<ClientRpcResponse> {
    // If no hook service, return disabled status with empty handlers
    let Some(service) = hook_service else {
        return Ok(ClientRpcResponse::HookListResult(HookListResultResponse {
            is_enabled: false,
            handlers: vec![],
        }));
    };

    // Build handler info from HooksConfig in context
    let hooks_config = &ctx.hooks_config;
    let handlers: Vec<HookHandlerInfo> = hooks_config
        .handlers
        .iter()
        .map(|cfg| HookHandlerInfo {
            name: cfg.name.clone(),
            pattern: cfg.pattern.clone(),
            handler_type: cfg.handler_type.type_name().to_string(),
            execution_mode: match cfg.execution_mode {
                ExecutionMode::Direct => "direct".to_string(),
                ExecutionMode::Job => "job".to_string(),
            },
            is_enabled: cfg.is_enabled,
            timeout_ms: cfg.timeout_ms,
            retry_count: cfg.retry_count,
        })
        .collect();

    debug!(handler_count = handlers.len(), enabled = service.is_enabled(), "listed hook handlers");

    Ok(ClientRpcResponse::HookListResult(HookListResultResponse {
        is_enabled: service.is_enabled(),
        handlers,
    }))
}

/// Handle HookGetMetrics request.
///
/// Returns execution metrics for handlers.
fn handle_hook_metrics(
    hook_service: Option<&Arc<HookService>>,
    handler_name: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(service) = hook_service else {
        return Ok(ClientRpcResponse::HookMetricsResult(HookMetricsResultResponse {
            is_enabled: false,
            total_events_processed: 0,
            handlers: vec![],
        }));
    };

    let snapshot = service.metrics().snapshot();

    // Filter by handler name if specified
    let handlers: Vec<HookHandlerMetrics> = if let Some(ref name) = handler_name {
        snapshot
            .handlers
            .iter()
            .filter(|(n, _)| *n == name)
            .map(|(name, m): (&String, &aspen_hooks::metrics::HandlerMetricsSnapshot)| HookHandlerMetrics {
                name: name.clone(),
                success_count: m.successes,
                failure_count: m.failures,
                dropped_count: m.dropped,
                jobs_submitted: m.jobs_submitted,
                avg_duration_us: m.avg_latency_us,
                max_duration_us: 0, // Not tracked currently
            })
            .collect()
    } else {
        snapshot
            .handlers
            .iter()
            .map(|(name, m): (&String, &aspen_hooks::metrics::HandlerMetricsSnapshot)| HookHandlerMetrics {
                name: name.clone(),
                success_count: m.successes,
                failure_count: m.failures,
                dropped_count: m.dropped,
                jobs_submitted: m.jobs_submitted,
                avg_duration_us: m.avg_latency_us,
                max_duration_us: 0, // Not tracked currently
            })
            .collect()
    };

    let total = snapshot.global.successes + snapshot.global.failures;

    debug!(
        total_events = total,
        handler_count = handlers.len(),
        filter = ?handler_name,
        "returned hook metrics"
    );

    Ok(ClientRpcResponse::HookMetricsResult(HookMetricsResultResponse {
        is_enabled: service.is_enabled(),
        total_events_processed: total,
        handlers,
    }))
}

/// Handle HookTrigger request.
///
/// Manually triggers a hook event for testing purposes.
async fn handle_hook_trigger(
    hook_service: Option<&Arc<HookService>>,
    node_id: u64,
    event_type: String,
    payload: serde_json::Value,
) -> anyhow::Result<ClientRpcResponse> {
    // Validate event type first - invalid types should always fail
    // regardless of whether hooks are enabled
    let hook_event_type = match event_type.as_str() {
        "write_committed" => HookEventType::WriteCommitted,
        "delete_committed" => HookEventType::DeleteCommitted,
        "membership_changed" => HookEventType::MembershipChanged,
        "leader_elected" => HookEventType::LeaderElected,
        "snapshot_created" => HookEventType::SnapshotCreated,
        _ => {
            return Ok(ClientRpcResponse::HookTriggerResult(HookTriggerResultResponse {
                success: false,
                dispatched_count: 0,
                error: Some(format!("unknown event type: {event_type}")),
                handler_failures: vec![],
            }));
        }
    };

    let Some(service) = hook_service else {
        // When hook service is unavailable, return failure with clear error
        return Ok(ClientRpcResponse::HookTriggerResult(HookTriggerResultResponse {
            success: false,
            dispatched_count: 0,
            error: Some("hooks not enabled".to_string()),
            handler_failures: vec![],
        }));
    };

    if !service.is_enabled() {
        // When hook service is disabled, return failure with clear error
        return Ok(ClientRpcResponse::HookTriggerResult(HookTriggerResultResponse {
            success: false,
            dispatched_count: 0,
            error: Some("hooks not enabled".to_string()),
            handler_failures: vec![],
        }));
    }

    // Create synthetic event
    let event = HookEvent::new(hook_event_type, node_id, payload);

    info!(
        event_type = ?event_type,
        node_id,
        "manually triggering hook event"
    );

    // Dispatch the event
    let dispatch_result = service.dispatch(&event).await;

    match dispatch_result {
        Ok(result) => {
            let dispatched_count = result.handler_count();
            let handler_failures: Vec<(String, String)> = match result {
                DispatchResult::Disabled => vec![],
                DispatchResult::Dispatched { direct_results, .. } => direct_results
                    .into_iter()
                    .filter_map(|(name, r): (String, aspen_hooks::error::Result<()>)| {
                        r.err().map(|e| (name, e.to_string()))
                    })
                    .collect(),
            };

            let success = handler_failures.is_empty();

            Ok(ClientRpcResponse::HookTriggerResult(HookTriggerResultResponse {
                success,
                dispatched_count,
                error: None,
                handler_failures,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::HookTriggerResult(HookTriggerResultResponse {
            success: false,
            dispatched_count: 0,
            error: Some(e.to_string()),
            handler_failures: vec![],
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_can_handle() {
        let handler = HooksHandler;

        assert!(handler.can_handle(&ClientRpcRequest::HookList));
        assert!(handler.can_handle(&ClientRpcRequest::HookGetMetrics { handler_name: None }));
        assert!(handler.can_handle(&ClientRpcRequest::HookTrigger {
            event_type: "write_committed".to_string(),
            payload_json: "{}".to_string(),
        }));

        // Should not handle other requests
        assert!(!handler.can_handle(&ClientRpcRequest::Ping));
        assert!(!handler.can_handle(&ClientRpcRequest::GetHealth));
    }
}
