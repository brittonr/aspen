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
                size_bytes,
                provider,
                tag,
            } => handle_blob_replicate_pull(ctx, hash, size_bytes, provider, tag).await,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_handle_add_blob() {
        let handler = BlobHandler;
        assert!(handler.can_handle(&ClientRpcRequest::AddBlob {
            data: vec![1, 2, 3],
            tag: None,
        }));
    }

    #[test]
    fn test_can_handle_get_blob() {
        let handler = BlobHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetBlob {
            hash: "abc123".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_has_blob() {
        let handler = BlobHandler;
        assert!(handler.can_handle(&ClientRpcRequest::HasBlob {
            hash: "abc123".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_list_blobs() {
        let handler = BlobHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ListBlobs {
            limit: 10,
            continuation_token: None,
        }));
    }

    #[test]
    fn test_can_handle_protect_blob() {
        let handler = BlobHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ProtectBlob {
            hash: "abc123".to_string(),
            tag: "important".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_delete_blob() {
        let handler = BlobHandler;
        assert!(handler.can_handle(&ClientRpcRequest::DeleteBlob {
            hash: "abc123".to_string(),
            is_force: false,
        }));
    }

    #[test]
    fn test_can_handle_download_blob() {
        let handler = BlobHandler;
        assert!(handler.can_handle(&ClientRpcRequest::DownloadBlob {
            ticket: "ticket123".to_string(),
            tag: Some("backup".to_string()),
        }));
    }

    #[test]
    fn test_can_handle_download_blob_by_hash() {
        let handler = BlobHandler;
        assert!(handler.can_handle(&ClientRpcRequest::DownloadBlobByHash {
            hash: "abc123".to_string(),
            tag: None,
        }));
    }

    #[test]
    fn test_can_handle_get_blob_status() {
        let handler = BlobHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetBlobStatus {
            hash: "abc123".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_trigger_blob_replication() {
        let handler = BlobHandler;
        assert!(handler.can_handle(&ClientRpcRequest::TriggerBlobReplication {
            hash: "abc123".to_string(),
            target_nodes: vec![1, 2, 3],
            replication_factor: 3,
        }));
    }

    #[test]
    fn test_can_handle_run_blob_repair_cycle() {
        let handler = BlobHandler;
        assert!(handler.can_handle(&ClientRpcRequest::RunBlobRepairCycle));
    }

    #[test]
    fn test_rejects_unrelated_requests() {
        let handler = BlobHandler;

        // Core requests
        assert!(!handler.can_handle(&ClientRpcRequest::Ping));
        assert!(!handler.can_handle(&ClientRpcRequest::GetHealth));

        // KV requests
        assert!(!handler.can_handle(&ClientRpcRequest::ReadKey {
            key: "test".to_string(),
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::WriteKey {
            key: "test".to_string(),
            value: vec![1, 2, 3],
        }));

        // Cluster requests
        assert!(!handler.can_handle(&ClientRpcRequest::InitCluster));
        assert!(!handler.can_handle(&ClientRpcRequest::GetClusterState));

        // Coordination requests
        assert!(!handler.can_handle(&ClientRpcRequest::LockAcquire {
            key: "test".to_string(),
            holder_id: "holder".to_string(),
            ttl_ms: 30000,
            timeout_ms: 0,
        }));
    }

    #[test]
    fn test_handler_name() {
        let handler = BlobHandler;
        assert_eq!(handler.name(), "BlobHandler");
    }
}
