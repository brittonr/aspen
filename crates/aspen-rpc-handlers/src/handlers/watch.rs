//! Watch request handler.
//!
//! Handles: WatchCreate, WatchCancel, WatchStatus.
//!
//! Watch operations require a streaming connection via LOG_SUBSCRIBER_ALPN (aspen-logs),
//! not the simple request/response pattern. These handlers return informative errors
//! directing users to the appropriate streaming protocol.
//!
//! The WatchStatus RPC is an exception - when a `WatchRegistry` is configured in the
//! context, it can return the status of active watches. This enables observability
//! without requiring clients to use the streaming protocol.

use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_client_rpc::WatchCancelResultResponse;
use aspen_client_rpc::WatchCreateResultResponse;
use aspen_client_rpc::WatchInfo as RpcWatchInfo;
use aspen_client_rpc::WatchStatusResultResponse;

use crate::context::ClientProtocolContext;
use crate::registry::RequestHandler;

/// Handler for watch operations.
///
/// Watch operations require streaming connections and cannot be handled through
/// the simple request/response pattern. This handler returns informative errors
/// directing clients to use LOG_SUBSCRIBER_ALPN for real-time key change notifications.
///
/// The WatchStatus RPC is an exception - when a `WatchRegistry` is configured,
/// it returns information about active watches without requiring streaming.
pub struct WatchHandler;

#[async_trait::async_trait]
impl RequestHandler for WatchHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::WatchCreate { .. }
                | ClientRpcRequest::WatchCancel { .. }
                | ClientRpcRequest::WatchStatus { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::WatchCreate { .. } => handle_watch_create().await,
            ClientRpcRequest::WatchCancel { watch_id } => handle_watch_cancel(watch_id).await,
            ClientRpcRequest::WatchStatus { watch_id } => handle_watch_status(watch_id, ctx).await,
            _ => Err(anyhow::anyhow!("request not handled by WatchHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "WatchHandler"
    }
}

// ============================================================================
// Watch Operation Handlers
// ============================================================================

async fn handle_watch_create() -> anyhow::Result<ClientRpcResponse> {
    // Watch operations require a streaming connection via LOG_SUBSCRIBER_ALPN.
    // The ClientRpcRequest::WatchCreate is for documentation completeness,
    // but actual watch functionality is handled by LogSubscriberProtocolHandler.
    Ok(ClientRpcResponse::WatchCreateResult(WatchCreateResultResponse {
        success: false,
        watch_id: None,
        current_index: None,
        error: Some(
            "Watch operations require the streaming protocol. \
             Connect via LOG_SUBSCRIBER_ALPN (aspen-logs) for real-time \
             key change notifications."
                .to_string(),
        ),
    }))
}

async fn handle_watch_cancel(watch_id: u64) -> anyhow::Result<ClientRpcResponse> {
    // Same as WatchCreate - streaming protocol required
    Ok(ClientRpcResponse::WatchCancelResult(WatchCancelResultResponse {
        success: false,
        watch_id,
        error: Some(
            "Watch operations require the streaming protocol. \
             Use LOG_SUBSCRIBER_ALPN (aspen-logs)."
                .to_string(),
        ),
    }))
}

async fn handle_watch_status(
    watch_id: Option<u64>,
    ctx: &ClientProtocolContext,
) -> anyhow::Result<ClientRpcResponse> {
    // If watch registry is configured, query it for active watches
    if let Some(ref registry) = ctx.watch_registry {
        let watches = if let Some(id) = watch_id {
            // Query specific watch
            match registry.get_watch(id).await {
                Some(info) => vec![RpcWatchInfo {
                    watch_id: info.watch_id,
                    prefix: info.prefix,
                    last_sent_index: info.last_sent_index,
                    events_sent: info.events_sent,
                    created_at_ms: info.created_at_ms,
                    include_prev_value: info.include_prev_value,
                }],
                None => vec![],
            }
        } else {
            // Query all watches
            registry
                .get_all_watches()
                .await
                .into_iter()
                .map(|info| RpcWatchInfo {
                    watch_id: info.watch_id,
                    prefix: info.prefix,
                    last_sent_index: info.last_sent_index,
                    events_sent: info.events_sent,
                    created_at_ms: info.created_at_ms,
                    include_prev_value: info.include_prev_value,
                })
                .collect()
        };

        return Ok(ClientRpcResponse::WatchStatusResult(
            WatchStatusResultResponse {
                success: true,
                watches: Some(watches),
                error: None,
            },
        ));
    }

    // No watch registry configured - return informative message
    Ok(ClientRpcResponse::WatchStatusResult(WatchStatusResultResponse {
        success: true,
        watches: Some(vec![]),
        error: Some(
            "Watch registry not configured. Watches are created via the streaming \
             protocol (LOG_SUBSCRIBER_ALPN). This node reports no active watches."
                .to_string(),
        ),
    }))
}
