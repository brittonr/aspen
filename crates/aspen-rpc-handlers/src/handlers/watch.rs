//! Watch request handler.
//!
//! Handles: WatchCreate, WatchCancel, WatchStatus.
//!
//! Watch operations require a streaming connection via LOG_SUBSCRIBER_ALPN (aspen-logs),
//! not the simple request/response pattern. These handlers return informative errors
//! directing users to the appropriate streaming protocol.

use crate::context::ClientProtocolContext;
use crate::registry::RequestHandler;
use aspen_client::ClientRpcRequest;
use aspen_client::ClientRpcResponse;
use aspen_client::WatchCancelResultResponse;
use aspen_client::WatchCreateResultResponse;
use aspen_client::WatchStatusResultResponse;

/// Handler for watch operations.
///
/// Watch operations require streaming connections and cannot be handled through
/// the simple request/response pattern. This handler returns informative errors
/// directing clients to use LOG_SUBSCRIBER_ALPN for real-time key change notifications.
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
        _ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::WatchCreate { .. } => handle_watch_create().await,
            ClientRpcRequest::WatchCancel { watch_id } => handle_watch_cancel(watch_id).await,
            ClientRpcRequest::WatchStatus { .. } => handle_watch_status().await,
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

async fn handle_watch_status() -> anyhow::Result<ClientRpcResponse> {
    // TODO: Could implement this to query LogSubscriberProtocolHandler state
    // For now, redirect to streaming protocol
    Ok(ClientRpcResponse::WatchStatusResult(WatchStatusResultResponse {
        success: false,
        watches: None,
        error: Some(
            "Watch operations require the streaming protocol. \
             Use LOG_SUBSCRIBER_ALPN (aspen-logs)."
                .to_string(),
        ),
    }))
}
