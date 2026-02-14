//! Blob content fetching and downloading from peers.

use std::sync::Arc;

use iroh_docs::NamespaceId;
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::is_tombstone_marker;
use crate::importer::DocsImporter;

/// Fetch entry content from local blob store or download from peer.
///
/// Returns `Some(content)` if content is available, `None` if unavailable or deferred.
#[allow(clippy::too_many_arguments)]
pub(super) async fn fetch_entry_content(
    content_hash: &iroh_blobs::Hash,
    from: &[u8; 32],
    should_download: bool,
    namespace_id: NamespaceId,
    key: &[u8],
    blob_store: &Arc<aspen_blob::store::IrohBlobStore>,
    importer: &Arc<DocsImporter>,
    source_cluster_id: &str,
    download_semaphore: &Arc<Semaphore>,
) -> Option<Vec<u8>> {
    use aspen_blob::prelude::*;

    // Try local blob store first
    match blob_store.get_bytes(content_hash).await {
        Ok(Some(bytes)) => {
            debug!(namespace = %namespace_id, hash = %content_hash.fmt_short(), size = bytes.len(), "fetched content from blob store");
            return Some(bytes.to_vec());
        }
        Ok(None) => {
            // Content not available locally - continue to download logic
        }
        Err(e) => {
            warn!(namespace = %namespace_id, hash = %content_hash.fmt_short(), error = %e, "failed to fetch content from blob store");
            return None;
        }
    }

    // Content not available locally
    if !should_download {
        debug!(namespace = %namespace_id, hash = %content_hash.fmt_short(), key = %String::from_utf8_lossy(key), "content not available locally and should_download=false, skipping");
        return None;
    }

    // Parse provider from peer ID bytes
    let provider = match iroh::PublicKey::from_bytes(from) {
        Ok(pk) => pk,
        Err(e) => {
            warn!(namespace = %namespace_id, hash = %content_hash.fmt_short(), from = %hex::encode(&from[..8]), error = %e, "invalid peer ID bytes, cannot download blob");
            return None;
        }
    };

    // Download from peer with bounded concurrency
    download_blob_content(
        content_hash,
        provider,
        namespace_id,
        key,
        blob_store,
        importer,
        source_cluster_id,
        download_semaphore,
    )
    .await
}

/// Download blob content from a peer with semaphore-bounded concurrency.
///
/// If semaphore is full, spawns a deferred background download and returns `None`.
#[allow(clippy::too_many_arguments)]
async fn download_blob_content(
    content_hash: &iroh_blobs::Hash,
    provider: iroh::PublicKey,
    namespace_id: NamespaceId,
    key: &[u8],
    blob_store: &Arc<aspen_blob::store::IrohBlobStore>,
    importer: &Arc<DocsImporter>,
    source_cluster_id: &str,
    download_semaphore: &Arc<Semaphore>,
) -> Option<Vec<u8>> {
    use aspen_blob::prelude::*;

    // Try to acquire semaphore permit
    let permit = match download_semaphore.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            // Semaphore full - spawn background download
            spawn_deferred_download(
                *content_hash,
                provider,
                key.to_vec(),
                blob_store.clone(),
                importer.clone(),
                source_cluster_id.to_string(),
                download_semaphore.clone(),
            );
            return None;
        }
    };

    // Download blob with permit
    info!(namespace = %namespace_id, hash = %content_hash.fmt_short(), provider = %provider.fmt_short(), "downloading blob from peer");

    match blob_store.download_from_peer(content_hash, provider).await {
        Ok(blob_ref) => {
            drop(permit); // Release permit early
            info!(namespace = %namespace_id, hash = %content_hash.fmt_short(), size = blob_ref.size, provider = %provider.fmt_short(), "blob downloaded from peer");

            // Fetch the downloaded content
            match blob_store.get_bytes(content_hash).await {
                Ok(Some(bytes)) => Some(bytes.to_vec()),
                Ok(None) => {
                    warn!(namespace = %namespace_id, hash = %content_hash.fmt_short(), "blob disappeared after download");
                    None
                }
                Err(e) => {
                    warn!(namespace = %namespace_id, hash = %content_hash.fmt_short(), error = %e, "failed to read downloaded blob");
                    None
                }
            }
        }
        Err(e) => {
            drop(permit);
            warn!(namespace = %namespace_id, hash = %content_hash.fmt_short(), provider = %provider.fmt_short(), error = %e, "failed to download blob from peer");
            None
        }
    }
}

/// Spawn a deferred background download when semaphore is full.
fn spawn_deferred_download(
    content_hash: iroh_blobs::Hash,
    provider: iroh::PublicKey,
    key: Vec<u8>,
    blob_store: Arc<aspen_blob::store::IrohBlobStore>,
    importer: Arc<DocsImporter>,
    source_cluster_id: String,
    semaphore: Arc<Semaphore>,
) {
    use aspen_blob::prelude::*;

    tokio::spawn(async move {
        // Wait for permit
        let _permit = match semaphore.acquire().await {
            Ok(p) => p,
            Err(_) => {
                warn!(hash = %content_hash.fmt_short(), "download semaphore closed");
                return;
            }
        };

        // Download blob
        match blob_store.download_from_peer(&content_hash, provider).await {
            Ok(blob_ref) => {
                info!(hash = %content_hash.fmt_short(), size = blob_ref.size, provider = %provider.fmt_short(), "blob downloaded from peer (deferred)");

                // Fetch and process the downloaded content
                if let Ok(Some(bytes)) = blob_store.get_bytes(&content_hash).await {
                    let content = bytes.to_vec();
                    if is_tombstone_marker(&content) {
                        return;
                    }
                    if let Err(e) = importer.process_remote_entry(&source_cluster_id, &key, &content).await {
                        warn!(key = %String::from_utf8_lossy(&key), error = %e, "failed to import deferred remote entry");
                    }
                }
            }
            Err(e) => {
                warn!(hash = %content_hash.fmt_short(), provider = %provider.fmt_short(), error = %e, "failed to download blob from peer (deferred)");
            }
        }
    });
}
