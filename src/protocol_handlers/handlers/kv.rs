//! Key-Value request handler.
//!
//! Handles: ReadKey, WriteKey, DeleteKey, ScanKeys, BatchRead, BatchWrite,
//! ConditionalBatchWrite, CompareAndSwapKey, CompareAndDeleteKey.

use super::ClientProtocolContext;
use super::RequestHandler;
use crate::client_rpc::ClientRpcRequest;
use crate::client_rpc::ClientRpcResponse;

/// Handler for key-value operations.
pub struct KvHandler;

#[async_trait::async_trait]
impl RequestHandler for KvHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::ReadKey { .. }
                | ClientRpcRequest::WriteKey { .. }
                | ClientRpcRequest::DeleteKey { .. }
                | ClientRpcRequest::ScanKeys { .. }
                | ClientRpcRequest::BatchRead { .. }
                | ClientRpcRequest::BatchWrite { .. }
                | ClientRpcRequest::ConditionalBatchWrite { .. }
                | ClientRpcRequest::CompareAndSwapKey { .. }
                | ClientRpcRequest::CompareAndDeleteKey { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        _ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        // The actual KV implementation requires type alignment between
        // the internal API types and client_rpc response types. For now, we
        // delegate to a placeholder that indicates the handler exists but the
        // implementation remains in client.rs until the full extraction is completed.
        //
        // TODO: Complete extraction of KV handlers once the API type alignment
        // is completed.
        match request {
            ClientRpcRequest::ReadKey { .. }
            | ClientRpcRequest::WriteKey { .. }
            | ClientRpcRequest::DeleteKey { .. }
            | ClientRpcRequest::ScanKeys { .. }
            | ClientRpcRequest::BatchRead { .. }
            | ClientRpcRequest::BatchWrite { .. }
            | ClientRpcRequest::ConditionalBatchWrite { .. }
            | ClientRpcRequest::CompareAndSwapKey { .. }
            | ClientRpcRequest::CompareAndDeleteKey { .. } => Ok(ClientRpcResponse::error(
                "NOT_IMPLEMENTED",
                "KV handler extraction in progress - operation handled by legacy path",
            )),
            _ => Err(anyhow::anyhow!("request not handled by KvHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "KvHandler"
    }
}
