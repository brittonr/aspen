//! Blob storage request handler.
//!
//! Handles: AddBlob, GetBlob, HasBlob, GetBlobTicket, ListBlobs, ProtectBlob,
//! UnprotectBlob, DeleteBlob, DownloadBlob, DownloadBlobByHash,
//! DownloadBlobByProvider, GetBlobStatus.

use super::ClientProtocolContext;
use super::RequestHandler;
use crate::client_rpc::ClientRpcRequest;
use crate::client_rpc::ClientRpcResponse;

/// Handler for blob storage operations.
pub struct BlobHandler;

#[async_trait::async_trait]
impl RequestHandler for BlobHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::AddBlob { .. }
                | ClientRpcRequest::GetBlob { .. }
                | ClientRpcRequest::HasBlob { .. }
                | ClientRpcRequest::GetBlobTicket { .. }
                | ClientRpcRequest::ListBlobs { .. }
                | ClientRpcRequest::ProtectBlob { .. }
                | ClientRpcRequest::UnprotectBlob { .. }
                | ClientRpcRequest::DeleteBlob { .. }
                | ClientRpcRequest::DownloadBlob { .. }
                | ClientRpcRequest::DownloadBlobByHash { .. }
                | ClientRpcRequest::DownloadBlobByProvider { .. }
                | ClientRpcRequest::GetBlobStatus { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        // Check if blob store is available
        if ctx.blob_store.is_none() {
            return Ok(ClientRpcResponse::error(
                "BLOB_UNAVAILABLE",
                "Blob store not configured on this node",
            ));
        }

        // The actual blob implementation requires complex API mapping between
        // IrohBlobStore methods and client_rpc response types. For now, we
        // delegate to a placeholder that indicates the handler exists but the
        // implementation remains in client.rs until the full extraction is completed.
        //
        // TODO: Complete extraction of Blob handlers once IrohBlobStore API
        // alignment is completed.
        match request {
            ClientRpcRequest::AddBlob { .. }
            | ClientRpcRequest::GetBlob { .. }
            | ClientRpcRequest::HasBlob { .. }
            | ClientRpcRequest::GetBlobTicket { .. }
            | ClientRpcRequest::ListBlobs { .. }
            | ClientRpcRequest::ProtectBlob { .. }
            | ClientRpcRequest::UnprotectBlob { .. }
            | ClientRpcRequest::DeleteBlob { .. }
            | ClientRpcRequest::DownloadBlob { .. }
            | ClientRpcRequest::DownloadBlobByHash { .. }
            | ClientRpcRequest::DownloadBlobByProvider { .. }
            | ClientRpcRequest::GetBlobStatus { .. } => Ok(ClientRpcResponse::error(
                "NOT_IMPLEMENTED",
                "Blob handler extraction in progress - operation handled by legacy path",
            )),
            _ => Err(anyhow::anyhow!("request not handled by BlobHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "BlobHandler"
    }
}
