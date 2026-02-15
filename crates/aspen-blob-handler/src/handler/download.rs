//! Blob download operations with DHT fallback.

use aspen_blob::IrohBlobStore;
use aspen_blob::prelude::*;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::DownloadBlobResultResponse;
use aspen_rpc_core::ClientProtocolContext;
#[cfg(feature = "global-discovery")]
use iroh_blobs::Hash;
use tracing::warn;

use super::error::sanitize_blob_error;

pub(crate) async fn handle_download_blob(
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
                size: Some(blob_ref.size_bytes),
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
                                    size = blob_ref.size_bytes,
                                    "blob downloaded from DHT provider (after ticket failure)"
                                );

                                return Ok(ClientRpcResponse::DownloadBlobResult(DownloadBlobResultResponse {
                                    success: true,
                                    hash: Some(blob_ref.hash.to_string()),
                                    size: Some(blob_ref.size_bytes),
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
                error: Some(sanitize_blob_error(&ticket_error)),
            }))
        }
    }
}

#[cfg(feature = "global-discovery")]
pub(crate) async fn handle_download_blob_by_hash(
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
                    size = blob_ref.size_bytes,
                    "blob downloaded from DHT provider"
                );

                return Ok(ClientRpcResponse::DownloadBlobByHashResult(DownloadBlobResultResponse {
                    success: true,
                    hash: Some(blob_ref.hash.to_string()),
                    size: Some(blob_ref.size_bytes),
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
pub(crate) async fn handle_download_blob_by_hash(
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
pub(crate) async fn handle_download_blob_by_provider(
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
                size = blob_ref.size_bytes,
                "blob downloaded from DHT provider"
            );

            Ok(ClientRpcResponse::DownloadBlobByProviderResult(DownloadBlobResultResponse {
                success: true,
                hash: Some(blob_ref.hash.to_string()),
                size: Some(blob_ref.size_bytes),
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
pub(crate) async fn handle_download_blob_by_provider(
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
