//! Blob storage request handler.
//!
//! Handles: AddBlob, GetBlob, HasBlob, GetBlobTicket, ListBlobs, ProtectBlob,
//! UnprotectBlob, DeleteBlob, DownloadBlob, DownloadBlobByHash,
//! DownloadBlobByProvider, GetBlobStatus, BlobReplicatePull,
//! GetBlobReplicationStatus, TriggerBlobReplication.

use std::time::Instant;

use aspen_blob::BlobStore;
use aspen_blob::IrohBlobStore;
use aspen_client_rpc::AddBlobResultResponse;
use aspen_client_rpc::BlobListEntry;
use aspen_client_rpc::BlobReplicatePullResultResponse;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_client_rpc::DeleteBlobResultResponse;
use aspen_client_rpc::DownloadBlobResultResponse;
use aspen_client_rpc::GetBlobReplicationStatusResultResponse;
use aspen_client_rpc::GetBlobResultResponse;
use aspen_client_rpc::GetBlobStatusResultResponse;
use aspen_client_rpc::GetBlobTicketResultResponse;
use aspen_client_rpc::HasBlobResultResponse;
use aspen_client_rpc::ListBlobsResultResponse;
use aspen_client_rpc::ProtectBlobResultResponse;
use aspen_client_rpc::TriggerBlobReplicationResultResponse;
use aspen_client_rpc::UnprotectBlobResultResponse;
use iroh::PublicKey;
use iroh_blobs::Hash;
use tracing::info;
use tracing::warn;

use crate::context::ClientProtocolContext;
#[cfg(feature = "blob")]
use crate::error_sanitization::sanitize_blob_error;
use crate::registry::RequestHandler;

/// Local error sanitization function that works with or without blob feature.
fn sanitize_blob_error_local(err: &aspen_blob::BlobStoreError) -> String {
    #[cfg(feature = "blob")]
    {
        sanitize_blob_error(err)
    }
    #[cfg(not(feature = "blob"))]
    {
        // Fallback when blob feature is not enabled
        format!("blob operation failed: {}", err)
    }
}

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
            ClientRpcRequest::AddBlob { data, tag } => handle_add_blob(ctx, data, tag).await,

            ClientRpcRequest::GetBlob { hash } => handle_get_blob(ctx, hash).await,

            ClientRpcRequest::HasBlob { hash } => handle_has_blob(ctx, hash).await,

            ClientRpcRequest::GetBlobTicket { hash } => handle_get_blob_ticket(ctx, hash).await,

            ClientRpcRequest::ListBlobs {
                limit,
                continuation_token,
            } => handle_list_blobs(ctx, limit, continuation_token).await,

            ClientRpcRequest::ProtectBlob { hash, tag } => handle_protect_blob(ctx, hash, tag).await,

            ClientRpcRequest::UnprotectBlob { tag } => handle_unprotect_blob(ctx, tag).await,

            ClientRpcRequest::DeleteBlob { hash, force } => handle_delete_blob(ctx, hash, force).await,

            ClientRpcRequest::DownloadBlob { ticket, tag } => handle_download_blob(ctx, ticket, tag).await,

            ClientRpcRequest::DownloadBlobByHash { hash, tag } => handle_download_blob_by_hash(ctx, hash, tag).await,

            ClientRpcRequest::DownloadBlobByProvider { hash, provider, tag } => {
                handle_download_blob_by_provider(ctx, hash, provider, tag).await
            }

            ClientRpcRequest::GetBlobStatus { hash } => handle_get_blob_status(ctx, hash).await,

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

// ============================================================================
// Blob Operation Handlers
// ============================================================================

async fn handle_add_blob(
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
                let size = result.blob_ref.size;
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
                size: Some(result.blob_ref.size),
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
                error: Some(sanitize_blob_error_local(&e)),
            }))
        }
    }
}

async fn handle_get_blob(ctx: &ClientProtocolContext, hash: String) -> anyhow::Result<ClientRpcResponse> {
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
                error: Some(sanitize_blob_error_local(&e)),
            }))
        }
    }
}

async fn handle_has_blob(ctx: &ClientProtocolContext, hash: String) -> anyhow::Result<ClientRpcResponse> {
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
                error: Some(sanitize_blob_error_local(&e)),
            }))
        }
    }
}

async fn handle_get_blob_ticket(ctx: &ClientProtocolContext, hash: String) -> anyhow::Result<ClientRpcResponse> {
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
                error: Some(sanitize_blob_error_local(&e)),
            }))
        }
    }
}

async fn handle_list_blobs(
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
                    size: entry.size,
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
                error: Some(sanitize_blob_error_local(&e)),
            }))
        }
    }
}

async fn handle_protect_blob(
    ctx: &ClientProtocolContext,
    hash: String,
    tag: String,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::ProtectBlobResult(ProtectBlobResultResponse {
            success: false,
            error: Some("blob store not enabled".to_string()),
        }));
    };

    // Parse hash from string
    let hash = match hash.parse::<Hash>() {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ProtectBlobResult(ProtectBlobResultResponse {
                success: false,
                error: Some(format!("invalid hash: {}", e)),
            }));
        }
    };

    let tag_name = IrohBlobStore::user_tag(&tag);
    match blob_store.protect(&hash, &tag_name).await {
        Ok(()) => Ok(ClientRpcResponse::ProtectBlobResult(ProtectBlobResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "blob protect failed");
            Ok(ClientRpcResponse::ProtectBlobResult(ProtectBlobResultResponse {
                success: false,
                error: Some(sanitize_blob_error_local(&e)),
            }))
        }
    }
}

async fn handle_unprotect_blob(ctx: &ClientProtocolContext, tag: String) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::UnprotectBlobResult(UnprotectBlobResultResponse {
            success: false,
            error: Some("blob store not enabled".to_string()),
        }));
    };

    let tag_name = IrohBlobStore::user_tag(&tag);
    match blob_store.unprotect(&tag_name).await {
        Ok(()) => Ok(ClientRpcResponse::UnprotectBlobResult(UnprotectBlobResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "blob unprotect failed");
            Ok(ClientRpcResponse::UnprotectBlobResult(UnprotectBlobResultResponse {
                success: false,
                error: Some(sanitize_blob_error_local(&e)),
            }))
        }
    }
}

async fn handle_delete_blob(
    ctx: &ClientProtocolContext,
    hash: String,
    force: bool,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::DeleteBlobResult(DeleteBlobResultResponse {
            success: false,
            error: Some("blob store not enabled".to_string()),
        }));
    };

    // Parse hash from string
    let hash = match hash.parse::<Hash>() {
        Ok(h) => h,
        Err(_) => {
            return Ok(ClientRpcResponse::DeleteBlobResult(DeleteBlobResultResponse {
                success: false,
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
                success: true,
                error: None,
            }))
        }
        Err(e) => {
            warn!(hash = %hash, error = %e, "failed to delete blob tags");
            Ok(ClientRpcResponse::DeleteBlobResult(DeleteBlobResultResponse {
                success: false,
                error: Some(sanitize_blob_error_local(&e)),
            }))
        }
    }
}

async fn handle_download_blob(
    ctx: &ClientProtocolContext,
    ticket: String,
    tag: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use iroh_blobs::ticket::BlobTicket;

    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::DownloadBlobResult(DownloadBlobResultResponse {
            success: false,
            hash: None,
            size: None,
            error: Some("blob store not enabled".to_string()),
        }));
    };

    // Parse the ticket
    let ticket = match ticket.parse::<BlobTicket>() {
        Ok(t) => t,
        Err(_) => {
            return Ok(ClientRpcResponse::DownloadBlobResult(DownloadBlobResultResponse {
                success: false,
                hash: None,
                size: None,
                error: Some("invalid ticket".to_string()),
            }));
        }
    };

    // First try the ticket's provider
    match blob_store.download(&ticket).await {
        Ok(blob_ref) => {
            // Apply protection tag if requested
            if let Some(ref tag_name) = tag {
                let user_tag = IrohBlobStore::user_tag(tag_name);
                if let Err(e) = blob_store.protect(&blob_ref.hash, &user_tag).await {
                    warn!(error = %e, "failed to apply tag to downloaded blob");
                }
            }

            Ok(ClientRpcResponse::DownloadBlobResult(DownloadBlobResultResponse {
                success: true,
                hash: Some(blob_ref.hash.to_string()),
                size: Some(blob_ref.size),
                error: None,
            }))
        }
        Err(ticket_error) => {
            // Ticket provider failed. Try DHT providers if content discovery is enabled.
            #[cfg(feature = "global-discovery")]
            if let Some(ref discovery) = ctx.content_discovery {
                use tracing::debug;
                use tracing::info;

                let hash = ticket.hash();
                let format = ticket.format();

                debug!(
                    hash = %hash.fmt_short(),
                    "ticket provider failed, trying DHT providers"
                );

                // Query DHT for additional providers
                if let Ok(providers) = discovery.find_providers(hash, format).await {
                    // Filter out the ticket provider (already tried)
                    let ticket_provider = ticket.addr().id;
                    let dht_providers: Vec<_> =
                        providers.into_iter().filter(|p| p.node_id != ticket_provider).collect();

                    if !dht_providers.is_empty() {
                        info!(
                            hash = %hash.fmt_short(),
                            provider_count = dht_providers.len(),
                            "found additional DHT providers"
                        );

                        // Try each DHT provider
                        for provider in &dht_providers {
                            debug!(
                                hash = %hash.fmt_short(),
                                provider = %provider.node_id.fmt_short(),
                                "attempting download from DHT provider"
                            );

                            if let Ok(blob_ref) = blob_store.download_from_peer(&hash, provider.node_id).await {
                                // Apply protection tag if requested
                                if let Some(ref tag_name) = tag {
                                    let user_tag = IrohBlobStore::user_tag(tag_name);
                                    if let Err(e) = blob_store.protect(&blob_ref.hash, &user_tag).await {
                                        warn!(error = %e, "failed to apply tag to downloaded blob");
                                    }
                                }

                                info!(
                                    hash = %hash.fmt_short(),
                                    provider = %provider.node_id.fmt_short(),
                                    size = blob_ref.size,
                                    "blob downloaded from DHT provider (after ticket failure)"
                                );

                                return Ok(ClientRpcResponse::DownloadBlobResult(DownloadBlobResultResponse {
                                    success: true,
                                    hash: Some(blob_ref.hash.to_string()),
                                    size: Some(blob_ref.size),
                                    error: None,
                                }));
                            }
                        }
                    }
                }
            }

            // All providers failed (ticket + DHT)
            warn!(error = %ticket_error, "blob download failed from all providers");
            Ok(ClientRpcResponse::DownloadBlobResult(DownloadBlobResultResponse {
                success: false,
                hash: None,
                size: None,
                error: Some(sanitize_blob_error_local(&ticket_error)),
            }))
        }
    }
}

#[cfg(feature = "global-discovery")]
async fn handle_download_blob_by_hash(
    ctx: &ClientProtocolContext,
    hash: String,
    tag: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use tracing::debug;
    use tracing::info;

    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::DownloadBlobByHashResult(DownloadBlobResultResponse {
            success: false,
            hash: None,
            size: None,
            error: Some("blob store not enabled".to_string()),
        }));
    };

    let Some(ref discovery) = ctx.content_discovery else {
        return Ok(ClientRpcResponse::DownloadBlobByHashResult(DownloadBlobResultResponse {
            success: false,
            hash: None,
            size: None,
            error: Some("content discovery not enabled".to_string()),
        }));
    };

    // Parse the hash
    let hash = match hash.parse::<Hash>() {
        Ok(h) => h,
        Err(_) => {
            return Ok(ClientRpcResponse::DownloadBlobByHashResult(DownloadBlobResultResponse {
                success: false,
                hash: None,
                size: None,
                error: Some("invalid hash".to_string()),
            }));
        }
    };

    // Query DHT for providers
    let providers = match discovery.find_providers(hash, iroh_blobs::BlobFormat::Raw).await {
        Ok(p) => p,
        Err(e) => {
            warn!(error = %e, hash = %hash.fmt_short(), "DHT provider lookup failed");
            return Ok(ClientRpcResponse::DownloadBlobByHashResult(DownloadBlobResultResponse {
                success: false,
                hash: Some(hash.to_string()),
                size: None,
                error: Some("provider lookup failed".to_string()),
            }));
        }
    };

    if providers.is_empty() {
        return Ok(ClientRpcResponse::DownloadBlobByHashResult(DownloadBlobResultResponse {
            success: false,
            hash: Some(hash.to_string()),
            size: None,
            error: Some("no providers found".to_string()),
        }));
    }

    info!(
        hash = %hash.fmt_short(),
        provider_count = providers.len(),
        "found DHT providers"
    );

    // Try each provider until one succeeds
    let mut last_error = None;
    for provider in &providers {
        debug!(
            hash = %hash.fmt_short(),
            provider = %provider.node_id.fmt_short(),
            "attempting download from DHT provider"
        );

        match blob_store.download_from_peer(&hash, provider.node_id).await {
            Ok(blob_ref) => {
                // Apply protection tag if requested
                if let Some(ref tag_name) = tag {
                    let user_tag = IrohBlobStore::user_tag(tag_name);
                    if let Err(e) = blob_store.protect(&blob_ref.hash, &user_tag).await {
                        warn!(error = %e, "failed to apply tag to downloaded blob");
                    }
                }

                info!(
                    hash = %hash.fmt_short(),
                    provider = %provider.node_id.fmt_short(),
                    size = blob_ref.size,
                    "blob downloaded from DHT provider"
                );

                return Ok(ClientRpcResponse::DownloadBlobByHashResult(DownloadBlobResultResponse {
                    success: true,
                    hash: Some(blob_ref.hash.to_string()),
                    size: Some(blob_ref.size),
                    error: None,
                }));
            }
            Err(e) => {
                debug!(
                    error = %e,
                    provider = %provider.node_id.fmt_short(),
                    "download from provider failed, trying next"
                );
                last_error = Some(e);
            }
        }
    }

    // All providers failed
    let error_msg = last_error.map(|e| sanitize_blob_error(&e)).unwrap_or_else(|| "all providers failed".to_string());
    warn!(hash = %hash.fmt_short(), error = %error_msg, "blob download failed from all providers");
    Ok(ClientRpcResponse::DownloadBlobByHashResult(DownloadBlobResultResponse {
        success: false,
        hash: Some(hash.to_string()),
        size: None,
        error: Some(error_msg),
    }))
}

#[cfg(not(feature = "global-discovery"))]
async fn handle_download_blob_by_hash(
    _ctx: &ClientProtocolContext,
    _hash: String,
    _tag: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    Ok(ClientRpcResponse::DownloadBlobByHashResult(DownloadBlobResultResponse {
        success: false,
        hash: None,
        size: None,
        error: Some("global-discovery feature not enabled".to_string()),
    }))
}

#[cfg(feature = "global-discovery")]
async fn handle_download_blob_by_provider(
    ctx: &ClientProtocolContext,
    hash: String,
    provider: String,
    tag: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use iroh::PublicKey;
    use iroh_blobs::BlobFormat;
    use tracing::info;

    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
            success: false,
            hash: None,
            size: None,
            error: Some("blob store not enabled".to_string()),
        }));
    };

    let Some(ref discovery) = ctx.content_discovery else {
        return Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
            success: false,
            hash: None,
            size: None,
            error: Some("content discovery not enabled".to_string()),
        }));
    };

    // Parse the hash
    let hash = match hash.parse::<Hash>() {
        Ok(h) => h,
        Err(_) => {
            return Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
                success: false,
                hash: None,
                size: None,
                error: Some("invalid hash".to_string()),
            }));
        }
    };

    // Parse the provider public key
    let provider_key = match provider.parse::<PublicKey>() {
        Ok(k) => k,
        Err(_) => {
            return Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
                success: false,
                hash: Some(hash.to_string()),
                size: None,
                error: Some("invalid provider public key".to_string()),
            }));
        }
    };

    // Look up the provider's DhtNodeAddr in the DHT
    let node_addr = match discovery.find_provider_by_public_key(&provider_key, hash, BlobFormat::Raw).await {
        Ok(Some(addr)) => addr,
        Ok(None) => {
            warn!(
                hash = %hash.fmt_short(),
                provider = %provider_key.fmt_short(),
                "provider not found in DHT mutable items"
            );
            return Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
                success: false,
                hash: Some(hash.to_string()),
                size: None,
                error: Some("provider not found in DHT".to_string()),
            }));
        }
        Err(e) => {
            warn!(
                hash = %hash.fmt_short(),
                provider = %provider_key.fmt_short(),
                error = %e,
                "DHT mutable item lookup failed"
            );
            return Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
                success: false,
                hash: Some(hash.to_string()),
                size: None,
                error: Some(format!("DHT lookup failed: {}", e)),
            }));
        }
    };

    info!(
        hash = %hash.fmt_short(),
        provider = %provider_key.fmt_short(),
        relay_url = ?node_addr.relay_url,
        direct_addrs = node_addr.direct_addrs.len(),
        "found provider in DHT, attempting download"
    );

    // Download from the provider
    match blob_store.download_from_peer(&hash, provider_key).await {
        Ok(blob_ref) => {
            // Apply protection tag if requested
            if let Some(ref tag_name) = tag {
                let user_tag = IrohBlobStore::user_tag(tag_name);
                if let Err(e) = blob_store.protect(&blob_ref.hash, &user_tag).await {
                    warn!(error = %e, "failed to apply tag to downloaded blob");
                }
            }

            info!(
                hash = %hash.fmt_short(),
                provider = %provider_key.fmt_short(),
                size = blob_ref.size,
                "blob downloaded from DHT provider"
            );

            Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
                success: true,
                hash: Some(blob_ref.hash.to_string()),
                size: Some(blob_ref.size),
                error: None,
            }))
        }
        Err(e) => {
            let error_msg = sanitize_blob_error(&e);
            warn!(
                hash = %hash.fmt_short(),
                provider = %provider_key.fmt_short(),
                error = %error_msg,
                "blob download from provider failed"
            );
            Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
                success: false,
                hash: Some(hash.to_string()),
                size: None,
                error: Some(error_msg),
            }))
        }
    }
}

#[cfg(not(feature = "global-discovery"))]
async fn handle_download_blob_by_provider(
    _ctx: &ClientProtocolContext,
    _hash: String,
    _provider: String,
    _tag: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
        success: false,
        hash: None,
        size: None,
        error: Some("global-discovery feature not enabled".to_string()),
    }))
}

async fn handle_get_blob_status(ctx: &ClientProtocolContext, hash: String) -> anyhow::Result<ClientRpcResponse> {
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
            size: status.size,
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
                error: Some(sanitize_blob_error_local(&e)),
            }))
        }
    }
}

// ============================================================================
// Blob Replication Handlers
// ============================================================================

/// Handle BlobReplicatePull request.
///
/// This is the target-side handler for blob replication. When a source node
/// wants to replicate a blob to this node, it sends a BlobReplicatePull request.
/// This node then downloads the blob from the provider using iroh-blobs P2P.
async fn handle_blob_replicate_pull(
    ctx: &ClientProtocolContext,
    hash: String,
    size: u64,
    provider: String,
    tag: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::BlobReplicatePullResult(BlobReplicatePullResultResponse {
            success: false,
            hash: None,
            size: None,
            duration_ms: None,
            error: Some("blob store not enabled".to_string()),
        }));
    };

    // Parse hash
    let hash = match hash.parse::<Hash>() {
        Ok(h) => h,
        Err(_) => {
            return Ok(ClientRpcResponse::BlobReplicatePullResult(BlobReplicatePullResultResponse {
                success: false,
                hash: None,
                size: None,
                duration_ms: None,
                error: Some("invalid hash format".to_string()),
            }));
        }
    };

    // Parse provider public key
    let provider_key = match provider.parse::<PublicKey>() {
        Ok(k) => k,
        Err(_) => {
            return Ok(ClientRpcResponse::BlobReplicatePullResult(BlobReplicatePullResultResponse {
                success: false,
                hash: Some(hash.to_string()),
                size: None,
                duration_ms: None,
                error: Some("invalid provider public key format".to_string()),
            }));
        }
    };

    // Check if we already have this blob
    match blob_store.has(&hash).await {
        Ok(true) => {
            info!(
                hash = %hash.fmt_short(),
                "blob already exists locally, skipping download"
            );
            return Ok(ClientRpcResponse::BlobReplicatePullResult(BlobReplicatePullResultResponse {
                success: true,
                hash: Some(hash.to_string()),
                size: Some(size),
                duration_ms: Some(0),
                error: None,
            }));
        }
        Ok(false) => {}
        Err(e) => {
            warn!(error = %e, "failed to check blob existence");
        }
    }

    // Download from provider
    let start = Instant::now();
    match blob_store.download_from_peer(&hash, provider_key).await {
        Ok(blob_ref) => {
            let duration_ms = start.elapsed().as_millis() as u64;

            info!(
                hash = %hash.fmt_short(),
                size = blob_ref.size,
                provider = %provider_key.fmt_short(),
                duration_ms,
                "blob replicated from peer"
            );

            // Apply protection tag if specified
            if let Some(ref tag_name) = tag {
                let user_tag = IrohBlobStore::user_tag(tag_name);
                if let Err(e) = blob_store.protect(&blob_ref.hash, &user_tag).await {
                    warn!(error = %e, "failed to apply tag to replicated blob");
                }
            }

            // Always apply a replication tag to prevent GC
            let replica_tag = format!("_replica:{}", hash.to_hex());
            if let Err(e) = blob_store.protect(&blob_ref.hash, &replica_tag).await {
                warn!(error = %e, "failed to apply replica tag");
            }

            Ok(ClientRpcResponse::BlobReplicatePullResult(BlobReplicatePullResultResponse {
                success: true,
                hash: Some(blob_ref.hash.to_string()),
                size: Some(blob_ref.size),
                duration_ms: Some(duration_ms),
                error: None,
            }))
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            warn!(
                hash = %hash.fmt_short(),
                provider = %provider_key.fmt_short(),
                duration_ms,
                error = %e,
                "blob replication failed"
            );
            Ok(ClientRpcResponse::BlobReplicatePullResult(BlobReplicatePullResultResponse {
                success: false,
                hash: Some(hash.to_string()),
                size: None,
                duration_ms: Some(duration_ms),
                error: Some(sanitize_blob_error_local(&e)),
            }))
        }
    }
}

/// Handle GetBlobReplicationStatus request.
///
/// Returns the replication metadata for a blob including which nodes have
/// replicas, the policy, and health status.
async fn handle_get_blob_replication_status(
    ctx: &ClientProtocolContext,
    hash: String,
) -> anyhow::Result<ClientRpcResponse> {
    // Parse hash
    let hash = match hash.parse::<Hash>() {
        Ok(h) => h,
        Err(_) => {
            return Ok(ClientRpcResponse::GetBlobReplicationStatusResult(GetBlobReplicationStatusResultResponse {
                found: false,
                hash: None,
                size: None,
                replica_nodes: None,
                replication_factor: None,
                min_replicas: None,
                status: None,
                replicas_needed: None,
                updated_at: None,
                error: Some("invalid hash format".to_string()),
            }));
        }
    };

    // Read replica metadata from KV store
    let replica_key = format!("_system:blob:replica:{}", hash.to_hex());

    match ctx.kv_store.read(aspen_core::kv::ReadRequest::new(&replica_key)).await {
        Ok(result) => {
            if let Some(kv) = result.kv {
                // Parse the replica set JSON
                match serde_json::from_str::<serde_json::Value>(&kv.value) {
                    Ok(json) => {
                        let nodes = json
                            .get("nodes")
                            .and_then(|n| n.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_u64()).collect::<Vec<_>>());

                        let policy = json.get("policy");
                        let replication_factor =
                            policy.and_then(|p| p.get("replication_factor")).and_then(|f| f.as_u64()).map(|f| f as u32);
                        let min_replicas =
                            policy.and_then(|p| p.get("min_replicas")).and_then(|m| m.as_u64()).map(|m| m as u32);

                        let size = json.get("size").and_then(|s| s.as_u64());
                        let updated_at = json.get("updated_at").and_then(|u| u.as_str()).map(String::from);

                        // Calculate status
                        let node_count = nodes.as_ref().map(|n| n.len() as u32).unwrap_or(0);
                        let target = replication_factor.unwrap_or(3);
                        let min = min_replicas.unwrap_or(2);

                        let status = if node_count == 0 {
                            "critical"
                        } else if node_count < min {
                            "under_replicated"
                        } else if node_count < target {
                            "degraded"
                        } else if node_count == target {
                            "healthy"
                        } else {
                            "over_replicated"
                        };

                        let replicas_needed = if node_count < target {
                            Some(target - node_count)
                        } else {
                            Some(0)
                        };

                        Ok(ClientRpcResponse::GetBlobReplicationStatusResult(GetBlobReplicationStatusResultResponse {
                            found: true,
                            hash: Some(hash.to_string()),
                            size,
                            replica_nodes: nodes,
                            replication_factor,
                            min_replicas,
                            status: Some(status.to_string()),
                            replicas_needed,
                            updated_at,
                            error: None,
                        }))
                    }
                    Err(e) => {
                        Ok(ClientRpcResponse::GetBlobReplicationStatusResult(GetBlobReplicationStatusResultResponse {
                            found: false,
                            hash: Some(hash.to_string()),
                            size: None,
                            replica_nodes: None,
                            replication_factor: None,
                            min_replicas: None,
                            status: None,
                            replicas_needed: None,
                            updated_at: None,
                            error: Some(format!("failed to parse replica metadata: {}", e)),
                        }))
                    }
                }
            } else {
                // No replication metadata exists for this blob
                Ok(ClientRpcResponse::GetBlobReplicationStatusResult(GetBlobReplicationStatusResultResponse {
                    found: false,
                    hash: Some(hash.to_string()),
                    size: None,
                    replica_nodes: None,
                    replication_factor: None,
                    min_replicas: None,
                    status: None,
                    replicas_needed: None,
                    updated_at: None,
                    error: None,
                }))
            }
        }
        Err(e) => Ok(ClientRpcResponse::GetBlobReplicationStatusResult(GetBlobReplicationStatusResultResponse {
            found: false,
            hash: Some(hash.to_string()),
            size: None,
            replica_nodes: None,
            replication_factor: None,
            min_replicas: None,
            status: None,
            replicas_needed: None,
            updated_at: None,
            error: Some(format!("failed to read replica metadata: {}", e)),
        })),
    }
}

/// Handle TriggerBlobReplication request.
///
/// Manually triggers replication of a blob to additional nodes.
/// Uses the BlobReplicationManager to coordinate replication across the cluster.
async fn handle_trigger_blob_replication(
    ctx: &ClientProtocolContext,
    hash: String,
    target_nodes: Vec<u64>,
    _replication_factor: u32,
) -> anyhow::Result<ClientRpcResponse> {
    let start = Instant::now();

    // Parse hash
    let hash = match hash.parse::<Hash>() {
        Ok(h) => h,
        Err(_) => {
            return Ok(ClientRpcResponse::TriggerBlobReplicationResult(TriggerBlobReplicationResultResponse {
                success: false,
                hash: None,
                successful_nodes: None,
                failed_nodes: None,
                duration_ms: None,
                error: Some("invalid hash format".to_string()),
            }));
        }
    };

    // Check if blob exists locally and get its size
    let blob_size = match &ctx.blob_store {
        Some(blob_store) => {
            match blob_store.has(&hash).await {
                Ok(false) => {
                    return Ok(ClientRpcResponse::TriggerBlobReplicationResult(TriggerBlobReplicationResultResponse {
                        success: false,
                        hash: Some(hash.to_string()),
                        successful_nodes: None,
                        failed_nodes: None,
                        duration_ms: Some(start.elapsed().as_millis() as u64),
                        error: Some("blob not found locally".to_string()),
                    }));
                }
                Err(e) => {
                    return Ok(ClientRpcResponse::TriggerBlobReplicationResult(TriggerBlobReplicationResultResponse {
                        success: false,
                        hash: Some(hash.to_string()),
                        successful_nodes: None,
                        failed_nodes: None,
                        duration_ms: Some(start.elapsed().as_millis() as u64),
                        error: Some(format!("failed to check blob existence: {}", e)),
                    }));
                }
                Ok(true) => {
                    // Get the size via status() - required for ReplicationRequest
                    match blob_store.status(&hash).await {
                        Ok(Some(status)) => match status.size {
                            Some(size) => size,
                            None => {
                                return Ok(ClientRpcResponse::TriggerBlobReplicationResult(
                                    TriggerBlobReplicationResultResponse {
                                        success: false,
                                        hash: Some(hash.to_string()),
                                        successful_nodes: None,
                                        failed_nodes: None,
                                        duration_ms: Some(start.elapsed().as_millis() as u64),
                                        error: Some("blob exists but size unavailable".to_string()),
                                    },
                                ));
                            }
                        },
                        Ok(None) => {
                            return Ok(ClientRpcResponse::TriggerBlobReplicationResult(
                                TriggerBlobReplicationResultResponse {
                                    success: false,
                                    hash: Some(hash.to_string()),
                                    successful_nodes: None,
                                    failed_nodes: None,
                                    duration_ms: Some(start.elapsed().as_millis() as u64),
                                    error: Some("blob status unavailable".to_string()),
                                },
                            ));
                        }
                        Err(e) => {
                            return Ok(ClientRpcResponse::TriggerBlobReplicationResult(
                                TriggerBlobReplicationResultResponse {
                                    success: false,
                                    hash: Some(hash.to_string()),
                                    successful_nodes: None,
                                    failed_nodes: None,
                                    duration_ms: Some(start.elapsed().as_millis() as u64),
                                    error: Some(format!("failed to get blob status: {}", e)),
                                },
                            ));
                        }
                    }
                }
            }
        }
        None => {
            return Ok(ClientRpcResponse::TriggerBlobReplicationResult(TriggerBlobReplicationResultResponse {
                success: false,
                hash: Some(hash.to_string()),
                successful_nodes: None,
                failed_nodes: None,
                duration_ms: Some(start.elapsed().as_millis() as u64),
                error: Some("blob store not available".to_string()),
            }));
        }
    };

    // Check if replication manager is available
    let replication_manager = match &ctx.blob_replication_manager {
        Some(manager) => manager,
        None => {
            return Ok(ClientRpcResponse::TriggerBlobReplicationResult(TriggerBlobReplicationResultResponse {
                success: false,
                hash: Some(hash.to_string()),
                successful_nodes: None,
                failed_nodes: None,
                duration_ms: Some(start.elapsed().as_millis() as u64),
                error: Some("blob replication not enabled on this node".to_string()),
            }));
        }
    };

    // Create replication request
    // If target_nodes is empty, the placement strategy will select targets automatically
    let request = aspen_blob::ReplicationRequest::new(hash, blob_size, target_nodes).with_ack(true);

    // Trigger replication through the manager
    match replication_manager.replicate(request).await {
        Ok(result) => {
            let success = result.failed.is_empty();
            let failed_nodes: Option<Vec<(u64, String)>> = if result.failed.is_empty() {
                None
            } else {
                Some(result.failed)
            };

            info!(
                hash = %hash.to_hex(),
                successful_count = result.successful.len(),
                failed_count = failed_nodes.as_ref().map(|f| f.len()).unwrap_or(0),
                duration_ms = result.duration_ms,
                "blob replication triggered"
            );

            Ok(ClientRpcResponse::TriggerBlobReplicationResult(TriggerBlobReplicationResultResponse {
                success,
                hash: Some(hash.to_string()),
                successful_nodes: Some(result.successful),
                failed_nodes,
                duration_ms: Some(result.duration_ms),
                error: None,
            }))
        }
        Err(e) => {
            warn!(hash = %hash.to_hex(), error = %e, "blob replication failed");
            Ok(ClientRpcResponse::TriggerBlobReplicationResult(TriggerBlobReplicationResultResponse {
                success: false,
                hash: Some(hash.to_string()),
                successful_nodes: None,
                failed_nodes: None,
                duration_ms: Some(start.elapsed().as_millis() as u64),
                error: Some(format!("replication failed: {}", e)),
            }))
        }
    }
}

/// Handle RunBlobRepairCycle request.
///
/// Manually triggers a full repair cycle across all blobs in the cluster.
/// Scans for under-replicated blobs and repairs them in priority order:
/// 1. Critical (0 replicas)
/// 2. UnderReplicated (below min_replicas)
/// 3. Degraded (below replication_factor)
///
/// Returns immediately - repairs happen asynchronously in the background.
async fn handle_run_blob_repair_cycle(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_rpc::RunBlobRepairCycleResultResponse;

    // Check if replication manager is available
    let replication_manager = match &ctx.blob_replication_manager {
        Some(manager) => manager,
        None => {
            return Ok(ClientRpcResponse::RunBlobRepairCycleResult(RunBlobRepairCycleResultResponse {
                success: false,
                error: Some("blob replication not enabled on this node".to_string()),
            }));
        }
    };

    // Trigger repair cycle (fire and forget)
    match replication_manager.run_repair_cycle().await {
        Ok(()) => {
            info!("blob repair cycle initiated via RPC");
            Ok(ClientRpcResponse::RunBlobRepairCycleResult(RunBlobRepairCycleResultResponse {
                success: true,
                error: None,
            }))
        }
        Err(e) => {
            warn!(error = %e, "failed to initiate blob repair cycle");
            Ok(ClientRpcResponse::RunBlobRepairCycleResult(RunBlobRepairCycleResultResponse {
                success: false,
                error: Some(format!("repair cycle initiation failed: {}", e)),
            }))
        }
    }
}
