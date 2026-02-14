//! Background sync service for periodic peer synchronization.

use std::sync::Arc;
use std::time::Duration;

use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh_docs::net::ConnectError;
use iroh_docs::net::SyncFinished;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::BACKGROUND_SYNC_INTERVAL_SECS;
use super::DocsSyncResources;
use super::MAX_OUTBOUND_SYNCS;

/// Background sync service that periodically syncs with discovered peers.
///
/// This service runs in the background and initiates outbound sync to peers
/// discovered via gossip or other mechanisms. It complements the inbound
/// sync handled by DocsProtocolHandler.
///
/// # Tiger Style
///
/// - Bounded concurrent syncs (MAX_OUTBOUND_SYNCS)
/// - Configurable sync interval
/// - Graceful shutdown via CancellationToken
pub struct DocsSyncService {
    /// Docs sync resources (SyncHandle, namespace, etc.)
    docs_sync: Arc<DocsSyncResources>,
    /// Iroh endpoint for outbound connections
    endpoint: Endpoint,
    /// Cancellation token for shutdown
    cancel: CancellationToken,
    /// Semaphore for limiting concurrent outbound syncs
    sync_semaphore: Arc<Semaphore>,
}

impl DocsSyncService {
    /// Create a new background sync service.
    ///
    /// # Arguments
    /// * `docs_sync` - The DocsSyncResources to use for syncing
    /// * `endpoint` - The Iroh endpoint for connections
    pub fn new(docs_sync: Arc<DocsSyncResources>, endpoint: Endpoint) -> Self {
        Self {
            docs_sync,
            endpoint,
            cancel: CancellationToken::new(),
            sync_semaphore: Arc::new(Semaphore::new(MAX_OUTBOUND_SYNCS)),
        }
    }

    /// Spawn the background sync service.
    ///
    /// Returns a cancellation token that can be used to stop the service.
    /// The service will periodically sync with provided peers.
    ///
    /// # Arguments
    /// * `peer_provider` - A function that returns the current list of peers to sync with
    pub fn spawn<F>(self: Arc<Self>, peer_provider: F) -> CancellationToken
    where F: Fn() -> Vec<EndpointAddr> + Send + Sync + 'static {
        let cancel = self.cancel.clone();
        let service = self.clone();

        tokio::spawn(async move {
            info!(
                namespace = %service.docs_sync.namespace_id,
                interval_secs = BACKGROUND_SYNC_INTERVAL_SECS,
                "docs sync service started"
            );

            let mut interval = tokio::time::interval(Duration::from_secs(BACKGROUND_SYNC_INTERVAL_SECS));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = service.cancel.cancelled() => {
                        info!("docs sync service shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        let peers = peer_provider();
                        if peers.is_empty() {
                            debug!("no peers available for docs sync");
                            continue;
                        }

                        debug!(peer_count = peers.len(), "starting background sync round");
                        service.sync_with_peers(peers).await;
                    }
                }
            }
        });

        cancel
    }

    /// Sync with a list of peers concurrently (bounded by semaphore).
    async fn sync_with_peers(&self, peers: Vec<EndpointAddr>) {
        let mut handles = Vec::new();

        for peer in peers {
            let docs_sync = self.docs_sync.clone();
            let endpoint = self.endpoint.clone();
            let semaphore = self.sync_semaphore.clone();

            let handle = tokio::spawn(async move {
                // Acquire permit before starting sync
                let _permit = match semaphore.acquire().await {
                    Ok(permit) => permit,
                    Err(_) => {
                        warn!("sync semaphore closed");
                        return;
                    }
                };

                // Perform sync (errors are logged inside sync_with_peer)
                let _ = docs_sync.sync_with_peer(&endpoint, peer).await;
            });

            handles.push(handle);
        }

        // Wait for all sync tasks to complete
        for handle in handles {
            let _ = handle.await;
        }
    }

    /// Trigger an immediate sync with a specific peer.
    ///
    /// This bypasses the background interval and syncs immediately.
    pub async fn sync_now(&self, peer: EndpointAddr) -> Result<SyncFinished, ConnectError> {
        self.docs_sync.sync_with_peer(&self.endpoint, peer).await
    }

    /// Shutdown the sync service.
    pub fn shutdown(&self) {
        self.cancel.cancel();
    }
}
