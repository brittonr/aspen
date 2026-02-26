//! Sync event loop and remote insert event handling.

use std::sync::Arc;

use iroh_docs::NamespaceId;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::blob_download::fetch_entry_content;
use super::is_tombstone_marker;
use crate::importer::DocsImporter;

/// Run the sync event processing loop.
///
/// This function handles the main event loop for processing sync events from iroh-docs.
/// It is extracted from `spawn_sync_event_listener` for Tiger Style compliance (70 line limit).
pub(super) async fn run_sync_event_loop(
    rx: async_channel::Receiver<iroh_docs::sync::Event>,
    cancel: CancellationToken,
    namespace_id: NamespaceId,
    source_cluster_id: String,
    blob_store: Arc<aspen_blob::store::IrohBlobStore>,
    importer: Arc<DocsImporter>,
    download_semaphore: Arc<Semaphore>,
) {
    use iroh_docs::sync::Event;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!(namespace = %namespace_id, "sync event listener shutting down");
                break;
            }
            event = rx.recv() => {
                match event {
                    Ok(Event::RemoteInsert { entry, from, should_download, remote_content_status, .. }) => {
                        handle_remote_insert_event(
                            &entry,
                            &from,
                            should_download,
                            remote_content_status,
                            namespace_id,
                            &source_cluster_id,
                            &blob_store,
                            &importer,
                            &download_semaphore,
                        ).await;
                    }
                    Ok(Event::LocalInsert { .. }) => {
                        // Ignore local inserts - we only care about remote
                    }
                    Err(e) => {
                        warn!(namespace = %namespace_id, error = %e, "sync event channel error, stopping listener");
                        break;
                    }
                }
            }
        }
    }
}

/// Handle a remote insert event from the sync stream.
///
/// This function processes a single remote entry: fetches content locally or from peer,
/// validates it, and forwards to the importer for priority-based import.
#[allow(clippy::too_many_arguments)]
async fn handle_remote_insert_event(
    entry: &iroh_docs::SignedEntry,
    from: &[u8; 32],
    should_download: bool,
    remote_content_status: iroh_docs::sync::ContentStatus,
    namespace_id: NamespaceId,
    source_cluster_id: &str,
    blob_store: &Arc<aspen_blob::store::IrohBlobStore>,
    importer: &Arc<DocsImporter>,
    download_semaphore: &Arc<Semaphore>,
) {
    let key = entry.key().to_vec();
    let content_hash = entry.content_hash();
    let content_len = entry.content_len();

    debug!(
        namespace = %namespace_id,
        key_len = key.len(),
        from = %hex::encode(&from[..8]),
        hash = %content_hash.fmt_short(),
        len = content_len,
        should_download = should_download,
        remote_status = ?remote_content_status,
        "received remote entry"
    );

    // Skip tombstones (empty content)
    if content_len == 0 {
        debug!(namespace = %namespace_id, key = %String::from_utf8_lossy(&key), "skipping tombstone entry");
        return;
    }

    // Fetch content (local or remote)
    let content = match fetch_entry_content(
        &content_hash,
        from,
        should_download,
        namespace_id,
        &key,
        blob_store,
        importer,
        source_cluster_id,
        download_semaphore,
    )
    .await
    {
        Some(c) => c,
        None => return, // Content unavailable or deferred to background task
    };

    // Check for tombstone marker (single null byte indicates deletion)
    if is_tombstone_marker(&content) {
        debug!(namespace = %namespace_id, key = %String::from_utf8_lossy(&key), "skipping tombstone marker entry");
        return;
    }

    // Forward to importer for priority-based import
    process_content_with_importer(namespace_id, source_cluster_id, &key, &content, importer).await;
}

/// Process fetched content with the importer.
async fn process_content_with_importer(
    namespace_id: NamespaceId,
    source_cluster_id: &str,
    key: &[u8],
    content: &[u8],
    importer: &Arc<DocsImporter>,
) {
    match importer.process_remote_entry(source_cluster_id, key, content).await {
        Ok(result) => {
            debug!(namespace = %namespace_id, key = %String::from_utf8_lossy(key), result = ?result, "processed remote entry");
        }
        Err(e) => {
            warn!(namespace = %namespace_id, key = %String::from_utf8_lossy(key), error = %e, "failed to import remote entry");
        }
    }
}
