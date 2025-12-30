//! SQL query request handler.
//!
//! Handles: ExecuteSql.

use super::ClientProtocolContext;
use super::RequestHandler;
use crate::client_rpc::ClientRpcRequest;
use crate::client_rpc::ClientRpcResponse;

/// Handler for SQL query operations.
pub struct SqlHandler;

#[async_trait::async_trait]
impl RequestHandler for SqlHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(request, ClientRpcRequest::ExecuteSql { .. })
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        _ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        // The actual SQL implementation requires type alignment between
        // the internal SQL executor types and client_rpc response types.
        // For now, we delegate to a placeholder that indicates the handler
        // exists but the implementation remains in client.rs until the full
        // extraction is completed.
        //
        // TODO: Complete extraction of SQL handler once the API type alignment
        // is completed.
        match request {
            ClientRpcRequest::ExecuteSql { .. } => Ok(ClientRpcResponse::error(
                "NOT_IMPLEMENTED",
                "SQL handler extraction in progress - operation handled by legacy path",
            )),
            _ => Err(anyhow::anyhow!("request not handled by SqlHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "SqlHandler"
    }
}
