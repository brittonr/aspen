//! Sync operations for DocsSyncResources: open_replica, sync_with_peer, spawn_sync_event_listener.

use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use aspen_blob::constants::MAX_CONCURRENT_BLOB_DOWNLOADS;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh_docs::net::ConnectError;
use iroh_docs::net::SyncFinished;
use iroh_docs::net::{self};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::DocsSyncResources;
use super::event_handling::run_sync_event_loop;
use crate::importer::DocsImporter;

impl DocsSyncResources {
    /// Open the replica for reading and writing.
    ///
    /// This must be called before writing entries via `SyncHandleDocsWriter`.
    /// The replica is opened with sync enabled to allow P2P synchronization.
    pub async fn open_replica(&self) -> Result<()> {
        use iroh_docs::actor::OpenOpts;

        self.sync_handle
            .open(self.namespace_id, OpenOpts::default().sync())
            .await
            .context("failed to open replica")?;

        debug!(
            namespace = %self.namespace_id,
            "replica opened for sync"
        );

        Ok(())
    }

    /// Initiate outbound sync to a peer.
    ///
    /// Connects to the specified peer and performs range-based set reconciliation
    /// to sync the namespace. This is the "Alice" side of the sync protocol.
    ///
    /// # Arguments
    /// * `endpoint` - The Iroh endpoint for establishing connections
    /// * `peer` - The peer's endpoint address (node ID + optional direct addresses)
    ///
    /// # Returns
    /// * `Ok(SyncFinished)` - Sync completed successfully with details
    /// * `Err(ConnectError)` - Connection or sync protocol failed
    pub async fn sync_with_peer(&self, endpoint: &Endpoint, peer: EndpointAddr) -> Result<SyncFinished, ConnectError> {
        let peer_id = peer.id.fmt_short().to_string();

        debug!(
            peer = %peer_id,
            namespace = %self.namespace_id,
            "initiating outbound sync"
        );

        // Emit sync started event
        if let Some(broadcaster) = &self.event_broadcaster {
            broadcaster.emit_sync_started(&peer_id, 1);
        }

        let start_time = std::time::Instant::now();

        let result = net::connect_and_sync(
            endpoint,
            &self.sync_handle,
            self.namespace_id,
            peer.clone(),
            None, // No metrics for now
        )
        .await;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        match &result {
            Ok(finished) => {
                let entries_synced = finished.outcome.num_sent + finished.outcome.num_recv;
                info!(
                    peer = %finished.peer.fmt_short(),
                    namespace = %finished.namespace,
                    sent = finished.outcome.num_sent,
                    recv = finished.outcome.num_recv,
                    connect_ms = ?finished.timings.connect.as_millis(),
                    process_ms = ?finished.timings.process.as_millis(),
                    "outbound sync completed"
                );

                // Emit sync completed event
                if let Some(broadcaster) = &self.event_broadcaster {
                    broadcaster.emit_sync_completed(&peer_id, entries_synced as u64, duration_ms);
                }
            }
            Err(err) => {
                warn!(
                    peer = %peer_id,
                    namespace = %self.namespace_id,
                    error = %err,
                    "outbound sync failed"
                );

                // Emit sync completed event with 0 entries for failed sync
                if let Some(broadcaster) = &self.event_broadcaster {
                    broadcaster.emit_sync_completed(&peer_id, 0, duration_ms);
                }
            }
        }

        result
    }

    /// Subscribe to sync events and forward RemoteInsert entries to the DocsImporter.
    ///
    /// This spawns a background task that:
    /// 1. Subscribes to sync events for this namespace
    /// 2. Filters for RemoteInsert events (entries received from peers)
    /// 3. Fetches content from the blob store using the content hash
    /// 4. Downloads content from peer if not available locally and should_download=true
    /// 5. Forwards each entry to the DocsImporter for priority-based import
    ///
    /// # Arguments
    /// * `importer` - The DocsImporter to forward entries to
    /// * `source_cluster_id` - Identifier for the source cluster (for origin tracking)
    /// * `blob_store` - The blob store for fetching content by hash
    ///
    /// # Returns
    /// A CancellationToken that can be used to stop the event listener.
    ///
    /// # Tiger Style
    /// - Bounded concurrent blob downloads (MAX_CONCURRENT_BLOB_DOWNLOADS)
    /// - Non-blocking download spawning to avoid blocking event processing
    pub async fn spawn_sync_event_listener(
        &self,
        importer: Arc<DocsImporter>,
        source_cluster_id: String,
        blob_store: Arc<aspen_blob::store::IrohBlobStore>,
    ) -> Result<CancellationToken> {
        use iroh_docs::sync::Event;

        // Create channel for sync events
        let (tx, rx) = async_channel::bounded::<Event>(1000);

        // Subscribe to sync events for our namespace
        self.sync_handle
            .subscribe(self.namespace_id, tx)
            .await
            .context("failed to subscribe to sync events")?;

        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        let namespace_id = self.namespace_id;

        // Semaphore for bounded concurrent blob downloads (Tiger Style)
        let download_semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_BLOB_DOWNLOADS as usize));

        tokio::spawn(async move {
            info!(
                namespace = %namespace_id,
                source = %source_cluster_id,
                max_concurrent_downloads = MAX_CONCURRENT_BLOB_DOWNLOADS,
                "sync event listener started"
            );

            run_sync_event_loop(
                rx,
                cancel_clone,
                namespace_id,
                source_cluster_id,
                blob_store,
                importer,
                download_semaphore,
            )
            .await;
        });

        Ok(cancel)
    }
}
