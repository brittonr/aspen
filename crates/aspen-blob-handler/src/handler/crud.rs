//! Basic blob CRUD operations: add, get, has, ticket, list, status.

use aspen_blob::IrohBlobStore;
use aspen_blob::prelude::*;
use aspen_client_api::AddBlobResultResponse;
use aspen_client_api::BlobListEntry;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::GetBlobResultResponse;
use aspen_client_api::GetBlobStatusResultResponse;
use aspen_client_api::GetBlobTicketResultResponse;
use aspen_client_api::HasBlobResultResponse;
use aspen_client_api::ListBlobsResultResponse;
use aspen_rpc_core::ClientProtocolContext;
use iroh_blobs::Hash;
use tracing::warn;

use super::error::sanitize_blob_error;

pub(crate) async fn handle_add_blob(
    ctx: &ClientProtocolContext,
    data: Vec<u8>,
    tag: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::AddBlobResult(AddBlobResultResponse {
            success: false,
            hash: None,
            size: None,
            was_new: None,
            error: Some("blob store not enabled".to_string()),
        }));
    };

    match blob_store.add_bytes(&data).await {
        Ok(result) => {
            // Apply tag if provided
            if let Some(tag_name) = tag {
                let tag_name = IrohBlobStore::user_tag(&tag_name);
                if let Err(e) = blob_store.protect(&result.blob_ref.hash, &tag_name).await {
                    warn!(error = %e, "failed to apply tag to blob");
                }
            }

            // Announce to DHT if content discovery is enabled
            #[cfg(feature = "global-discovery")]
            if let Some(ref discovery) = ctx.content_discovery {
                let hash = result.blob_ref.hash;
                let size = result.blob_ref.size_bytes;
                let format = result.blob_ref.format;
                let discovery = discovery.clone();
                tokio::spawn(async move {
                    if let Err(e) = discovery.announce(hash, size, format).await {
                        tracing::debug!(
                            hash = %hash.fmt_short(),
                            error = %e,
                            "DHT announce failed (non-fatal)"
                        );
                    }
                });
            }

            Ok(ClientRpcResponse::AddBlobResult(AddBlobResultResponse {
                success: true,
                hash: Some(result.blob_ref.hash.to_string()),
                size: Some(result.blob_ref.size_bytes),
                was_new: Some(result.was_new),
                error: None,
            }))
        }
        Err(e) => {
            warn!(error = %e, "blob add failed");
            Ok(ClientRpcResponse::AddBlobResult(AddBlobResultResponse {
                success: false,
                hash: None,
                size: None,
                was_new: None,
                error: Some(sanitize_blob_error(&e)),
            }))
        }
    }
}

pub(crate) async fn handle_get_blob(ctx: &ClientProtocolContext, hash: String) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::GetBlobResult(GetBlobResultResponse {
            found: false,
            data: None,
            error: Some("blob store not enabled".to_string()),
        }));
    };

    // Parse hash from string
    let hash = match hash.parse::<Hash>() {
        Ok(h) => h,
        Err(_) => {
            return Ok(ClientRpcResponse::GetBlobResult(GetBlobResultResponse {
                found: false,
                data: None,
                error: Some("invalid hash".to_string()),
            }));
        }
    };

    match blob_store.get_bytes(&hash).await {
        Ok(Some(data)) => Ok(ClientRpcResponse::GetBlobResult(GetBlobResultResponse {
            found: true,
            data: Some(data.to_vec()),
            error: None,
        })),
        Ok(None) => Ok(ClientRpcResponse::GetBlobResult(GetBlobResultResponse {
            found: false,
            data: None,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "blob get failed");
            Ok(ClientRpcResponse::GetBlobResult(GetBlobResultResponse {
                found: false,
                data: None,
                error: Some(sanitize_blob_error(&e)),
            }))
        }
    }
}

pub(crate) async fn handle_has_blob(ctx: &ClientProtocolContext, hash: String) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::HasBlobResult(HasBlobResultResponse {
            exists: false,
            error: Some("blob store not enabled".to_string()),
        }));
    };

    // Parse hash from string
    let hash = match hash.parse::<Hash>() {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::HasBlobResult(HasBlobResultResponse {
                exists: false,
                error: Some(format!("invalid hash: {}", e)),
            }));
        }
    };

    match blob_store.has(&hash).await {
        Ok(exists) => Ok(ClientRpcResponse::HasBlobResult(HasBlobResultResponse { exists, error: None })),
        Err(e) => {
            warn!(error = %e, "blob has check failed");
            Ok(ClientRpcResponse::HasBlobResult(HasBlobResultResponse {
                exists: false,
                error: Some(sanitize_blob_error(&e)),
            }))
        }
    }
}

pub(crate) async fn handle_get_blob_ticket(
    ctx: &ClientProtocolContext,
    hash: String,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::GetBlobTicketResult(GetBlobTicketResultResponse {
            success: false,
            ticket: None,
            error: Some("blob store not enabled".to_string()),
        }));
    };

    // Parse hash from string
    let hash = match hash.parse::<Hash>() {
        Ok(h) => h,
        Err(_) => {
            return Ok(ClientRpcResponse::GetBlobTicketResult(GetBlobTicketResultResponse {
                success: false,
                ticket: None,
                error: Some("invalid hash".to_string()),
            }));
        }
    };

    match blob_store.ticket(&hash).await {
        Ok(ticket) => Ok(ClientRpcResponse::GetBlobTicketResult(GetBlobTicketResultResponse {
            success: true,
            ticket: Some(ticket.to_string()),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "blob ticket generation failed");
            Ok(ClientRpcResponse::GetBlobTicketResult(GetBlobTicketResultResponse {
                success: false,
                ticket: None,
                error: Some(sanitize_blob_error(&e)),
            }))
        }
    }
}

pub(crate) async fn handle_list_blobs(
    ctx: &ClientProtocolContext,
    limit: u32,
    continuation_token: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::ListBlobsResult(ListBlobsResultResponse {
            blobs: vec![],
            count: 0,
            has_more: false,
            continuation_token: None,
            error: Some("blob store not enabled".to_string()),
        }));
    };

    // Tiger Style: Cap limit to prevent unbounded responses
    let limit = limit.min(1000);

    match blob_store.list(limit, continuation_token.as_deref()).await {
        Ok(result) => {
            let count = result.blobs.len() as u32;
            let blobs = result
                .blobs
                .into_iter()
                .map(|entry| BlobListEntry {
                    hash: entry.hash.to_string(),
                    size: entry.size_bytes,
                })
                .collect();

            Ok(ClientRpcResponse::ListBlobsResult(ListBlobsResultResponse {
                blobs,
                count,
                has_more: result.continuation_token.is_some(),
                continuation_token: result.continuation_token,
                error: None,
            }))
        }
        Err(e) => {
            warn!(error = %e, "blob list failed");
            Ok(ClientRpcResponse::ListBlobsResult(ListBlobsResultResponse {
                blobs: vec![],
                count: 0,
                has_more: false,
                continuation_token: None,
                error: Some(sanitize_blob_error(&e)),
            }))
        }
    }
}

pub(crate) async fn handle_get_blob_status(
    ctx: &ClientProtocolContext,
    hash: String,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::GetBlobStatusResult(GetBlobStatusResultResponse {
            found: false,
            hash: None,
            size: None,
            complete: None,
            tags: None,
            error: Some("blob store not enabled".to_string()),
        }));
    };

    // Parse hash from string
    let hash = match hash.parse::<Hash>() {
        Ok(h) => h,
        Err(_) => {
            return Ok(ClientRpcResponse::GetBlobStatusResult(GetBlobStatusResultResponse {
                found: false,
                hash: None,
                size: None,
                complete: None,
                tags: None,
                error: Some("invalid hash".to_string()),
            }));
        }
    };

    match blob_store.status(&hash).await {
        Ok(Some(status)) => Ok(ClientRpcResponse::GetBlobStatusResult(GetBlobStatusResultResponse {
            found: true,
            hash: Some(status.hash.to_string()),
            size: status.size_bytes,
            complete: Some(status.complete),
            tags: Some(status.tags),
            error: None,
        })),
        Ok(None) => Ok(ClientRpcResponse::GetBlobStatusResult(GetBlobStatusResultResponse {
            found: false,
            hash: Some(hash.to_string()),
            size: None,
            complete: None,
            tags: None,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "blob status check failed");
            Ok(ClientRpcResponse::GetBlobStatusResult(GetBlobStatusResultResponse {
                found: false,
                hash: None,
                size: None,
                complete: None,
                tags: None,
                error: Some(sanitize_blob_error(&e)),
            }))
        }
    }
}
