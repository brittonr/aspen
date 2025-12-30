//! Watch request handler.
//!
//! Handles: WatchCreate, WatchCancel, WatchStatus.
//!
//! Note: Watch operations require streaming which is not directly supported
//! through the simple request/response pattern. These are placeholder handlers.

use super::ClientProtocolContext;
use super::RequestHandler;
use crate::client_rpc::ClientRpcRequest;
use crate::client_rpc::ClientRpcResponse;
use crate::client_rpc::WatchCancelResultResponse;
use crate::client_rpc::WatchCreateResultResponse;
use crate::client_rpc::WatchStatusResultResponse;

/// Handler for watch operations.
pub struct WatchHandler;

#[async_trait::async_trait]
impl RequestHandler for WatchHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::WatchCreate { .. } | ClientRpcRequest::WatchCancel { .. } | ClientRpcRequest::WatchStatus { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        _ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::WatchCreate { .. } => {
                // Watch operations require a streaming protocol, not simple request/response.
                // The ClientRpcRequest::WatchCreate is for documentation completeness,
                // but actual watch functionality would need a dedicated streaming handler.
                Ok(ClientRpcResponse::WatchCreateResult(WatchCreateResultResponse {
                    success: false,
                    watch_id: None,
                    current_index: None,
                    error: Some(
                        "Watch operations require streaming protocol. Use iroh-docs for real-time updates.".to_string(),
                    ),
                }))
            }

            ClientRpcRequest::WatchCancel { watch_id } => {
                // Placeholder - watch IDs would be tracked in a streaming context
                Ok(ClientRpcResponse::WatchCancelResult(WatchCancelResultResponse {
                    success: false,
                    watch_id,
                    error: Some("Watch operations require streaming protocol".to_string()),
                }))
            }

            ClientRpcRequest::WatchStatus { .. } => {
                // Placeholder - would return active watch info in a streaming context
                Ok(ClientRpcResponse::WatchStatusResult(WatchStatusResultResponse {
                    success: false,
                    watches: None,
                    error: Some("Watch operations require streaming protocol".to_string()),
                }))
            }

            _ => Err(anyhow::anyhow!("request not handled by WatchHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "WatchHandler"
    }
}
