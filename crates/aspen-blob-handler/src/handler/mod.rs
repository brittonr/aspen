//! Blob storage request handler.
//!
//! Handles: AddBlob, GetBlob, HasBlob, GetBlobTicket, ListBlobs, ProtectBlob,
//! UnprotectBlob, DeleteBlob, DownloadBlob, DownloadBlobByHash,
//! DownloadBlobByProvider, GetBlobStatus, BlobReplicatePull,
//! GetBlobReplicationStatus, TriggerBlobReplication.

mod crud;
mod download;
mod error;
mod protection;
mod replication;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use crud::*;
use download::*;
use protection::*;
use replication::*;

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
                | ClientRpcRequest::BlobReplicatePull { .. }
                | ClientRpcRequest::GetBlobReplicationStatus { .. }
                | ClientRpcRequest::TriggerBlobReplication { .. }
                | ClientRpcRequest::RunBlobRepairCycle
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            // CRUD operations
            ClientRpcRequest::AddBlob { data, tag } => handle_add_blob(ctx, data, tag).await,
            ClientRpcRequest::GetBlob { hash } => handle_get_blob(ctx, hash).await,
            ClientRpcRequest::HasBlob { hash } => handle_has_blob(ctx, hash).await,
            ClientRpcRequest::GetBlobTicket { hash } => handle_get_blob_ticket(ctx, hash).await,
            ClientRpcRequest::ListBlobs {
                limit,
                continuation_token,
            } => handle_list_blobs(ctx, limit, continuation_token).await,
            ClientRpcRequest::GetBlobStatus { hash } => handle_get_blob_status(ctx, hash).await,

            // Protection operations
            ClientRpcRequest::ProtectBlob { hash, tag } => handle_protect_blob(ctx, hash, tag).await,
            ClientRpcRequest::UnprotectBlob { tag } => handle_unprotect_blob(ctx, tag).await,
            ClientRpcRequest::DeleteBlob { hash, is_force } => handle_delete_blob(ctx, hash, is_force).await,

            // Download operations
            ClientRpcRequest::DownloadBlob { ticket, tag } => handle_download_blob(ctx, ticket, tag).await,
            ClientRpcRequest::DownloadBlobByHash { hash, tag } => handle_download_blob_by_hash(ctx, hash, tag).await,
            ClientRpcRequest::DownloadBlobByProvider { hash, provider, tag } => {
                handle_download_blob_by_provider(ctx, hash, provider, tag).await
            }

            // Replication operations
            ClientRpcRequest::BlobReplicatePull {
                hash,
                size,
                provider,
                tag,
            } => handle_blob_replicate_pull(ctx, hash, size, provider, tag).await,
            ClientRpcRequest::GetBlobReplicationStatus { hash } => handle_get_blob_replication_status(ctx, hash).await,
            ClientRpcRequest::TriggerBlobReplication {
                hash,
                target_nodes,
                replication_factor,
            } => handle_trigger_blob_replication(ctx, hash, target_nodes, replication_factor).await,
            ClientRpcRequest::RunBlobRepairCycle => handle_run_blob_repair_cycle(ctx).await,

            _ => Err(anyhow::anyhow!("request not handled by BlobHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "BlobHandler"
    }
}
