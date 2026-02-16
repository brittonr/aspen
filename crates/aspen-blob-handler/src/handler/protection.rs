//! Blob protection operations: protect, unprotect, delete.

use aspen_blob::IrohBlobStore;
use aspen_blob::prelude::*;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::DeleteBlobResultResponse;
use aspen_client_api::ProtectBlobResultResponse;
use aspen_client_api::UnprotectBlobResultResponse;
use aspen_rpc_core::ClientProtocolContext;
use iroh_blobs::Hash;
use tracing::info;
use tracing::warn;

use super::error::sanitize_blob_error;

pub(crate) async fn handle_protect_blob(
    ctx: &ClientProtocolContext,
    hash: String,
    tag: String,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::ProtectBlobResult(ProtectBlobResultResponse {
            is_success: false,
            error: Some("blob store not enabled".to_string()),
        }));
    };

    // Parse hash from string
    let hash = match hash.parse::<Hash>() {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ProtectBlobResult(ProtectBlobResultResponse {
                is_success: false,
                error: Some(format!("invalid hash: {}", e)),
            }));
        }
    };

    let tag_name = IrohBlobStore::user_tag(&tag);
    match blob_store.protect(&hash, &tag_name).await {
        Ok(()) => Ok(ClientRpcResponse::ProtectBlobResult(ProtectBlobResultResponse {
            is_success: true,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "blob protect failed");
            Ok(ClientRpcResponse::ProtectBlobResult(ProtectBlobResultResponse {
                is_success: false,
                error: Some(sanitize_blob_error(&e)),
            }))
        }
    }
}

pub(crate) async fn handle_unprotect_blob(
    ctx: &ClientProtocolContext,
    tag: String,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::UnprotectBlobResult(UnprotectBlobResultResponse {
            is_success: false,
            error: Some("blob store not enabled".to_string()),
        }));
    };

    let tag_name = IrohBlobStore::user_tag(&tag);
    match blob_store.unprotect(&tag_name).await {
        Ok(()) => Ok(ClientRpcResponse::UnprotectBlobResult(UnprotectBlobResultResponse {
            is_success: true,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "blob unprotect failed");
            Ok(ClientRpcResponse::UnprotectBlobResult(UnprotectBlobResultResponse {
                is_success: false,
                error: Some(sanitize_blob_error(&e)),
            }))
        }
    }
}

pub(crate) async fn handle_delete_blob(
    ctx: &ClientProtocolContext,
    hash: String,
    force: bool,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::DeleteBlobResult(DeleteBlobResultResponse {
            is_success: false,
            error: Some("blob store not enabled".to_string()),
        }));
    };

    // Parse hash from string
    let hash = match hash.parse::<Hash>() {
        Ok(h) => h,
        Err(_) => {
            return Ok(ClientRpcResponse::DeleteBlobResult(DeleteBlobResultResponse {
                is_success: false,
                error: Some("invalid hash".to_string()),
            }));
        }
    };

    // Delete user tags for this blob
    // iroh-blobs uses tags to protect blobs from GC. When all tags are removed,
    // the blob becomes eligible for garbage collection.
    //
    // Behavior:
    // - force=true: Remove all user-created tags (user:*) for this hash
    // - force=false: Same behavior (we don't remove KV tags - those are managed by KV operations)
    //
    // Note: KV-referenced blobs (kv:* tags) are NOT deleted here. Those tags are
    // managed by KV delete operations. This only affects explicitly protected blobs.
    match blob_store.delete_user_tags_for_hash(&hash).await {
        Ok(deleted_count) => {
            if deleted_count > 0 {
                info!(hash = %hash, deleted_tags = deleted_count, force, "blob user tags deleted, blob eligible for GC");
            } else {
                // No user tags found - blob may already be unprotected or only has KV tags
                info!(hash = %hash, "no user tags found for blob (may have KV tags or be unprotected)");
            }
            Ok(ClientRpcResponse::DeleteBlobResult(DeleteBlobResultResponse {
                is_success: true,
                error: None,
            }))
        }
        Err(e) => {
            warn!(hash = %hash, error = %e, "failed to delete blob tags");
            Ok(ClientRpcResponse::DeleteBlobResult(DeleteBlobResultResponse {
                is_success: false,
                error: Some(sanitize_blob_error(&e)),
            }))
        }
    }
}
