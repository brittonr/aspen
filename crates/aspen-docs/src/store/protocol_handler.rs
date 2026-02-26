//! Protocol handler for incoming docs sync connections.

use std::sync::Arc;

use iroh::endpoint::Connection;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use iroh_docs::NamespaceId;
use iroh_docs::actor::SyncHandle;
use iroh_docs::net::AcceptOutcome;
use iroh_docs::net::SyncFinished;
use iroh_docs::net::{self};
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::constants::MAX_DOCS_CONNECTIONS;
use crate::events::DocsEventBroadcaster;

/// Protocol handler for iroh-docs P2P sync connections.
///
/// Implements `ProtocolHandler` to accept incoming sync connections from
/// remote peers. Uses `iroh_docs::net::handle_connection` for the actual
/// sync protocol implementation (range-based set reconciliation).
///
/// # Tiger Style
///
/// - Bounded connection count via semaphore (MAX_DOCS_CONNECTIONS)
/// - Explicit access control via accept callback
/// - Clean shutdown via ProtocolHandler::shutdown()
pub struct DocsProtocolHandler {
    /// Sync handle for coordinating with the replica store.
    sync_handle: SyncHandle,
    /// Our namespace ID (used for access control decisions).
    namespace_id: NamespaceId,
    /// Connection semaphore for bounded resources.
    connection_semaphore: Arc<Semaphore>,
    /// Optional event broadcaster for hook integration.
    event_broadcaster: Option<Arc<DocsEventBroadcaster>>,
}

impl std::fmt::Debug for DocsProtocolHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocsProtocolHandler")
            .field("namespace_id", &self.namespace_id)
            .field("has_event_broadcaster", &self.event_broadcaster.is_some())
            .finish()
    }
}

impl DocsProtocolHandler {
    /// Create a new docs protocol handler.
    ///
    /// # Arguments
    /// * `sync_handle` - Handle to the sync actor for replica coordination
    /// * `namespace_id` - The namespace ID this node is serving
    /// * `event_broadcaster` - Optional event broadcaster for hook integration
    pub fn new(
        sync_handle: SyncHandle,
        namespace_id: NamespaceId,
        event_broadcaster: Option<Arc<DocsEventBroadcaster>>,
    ) -> Self {
        Self {
            sync_handle,
            namespace_id,
            connection_semaphore: Arc::new(Semaphore::new(MAX_DOCS_CONNECTIONS as usize)),
            event_broadcaster,
        }
    }
}

#[allow(refining_impl_trait)]
impl ProtocolHandler for DocsProtocolHandler {
    fn accept(&self, connection: Connection) -> impl std::future::Future<Output = Result<(), AcceptError>> + Send + '_ {
        let sync_handle = self.sync_handle.clone();
        let namespace_id = self.namespace_id;
        let semaphore = self.connection_semaphore.clone();
        let event_broadcaster = self.event_broadcaster.clone();

        async move {
            let remote_peer = connection.remote_id();
            let peer_id = remote_peer.fmt_short().to_string();

            // Acquire permit or reject if at connection limit
            let permit = acquire_connection_permit(&semaphore, &peer_id)?;

            debug!(peer = %peer_id, namespace = %namespace_id, "accepting docs sync connection");

            // Emit sync started event
            if let Some(broadcaster) = &event_broadcaster {
                broadcaster.emit_sync_started(&peer_id, 1);
            }

            let start_time = std::time::Instant::now();

            // Handle the connection using iroh-docs sync protocol
            let result = run_sync_protocol(sync_handle, connection, namespace_id).await;

            let duration_ms = start_time.elapsed().as_millis() as u64;
            drop(permit); // Release permit

            // Process result and emit completion event
            handle_sync_result(result, &peer_id, duration_ms, &event_broadcaster)
        }
    }

    async fn shutdown(&self) {
        info!("docs protocol handler shutting down");
        self.connection_semaphore.close();
    }
}

/// Attempt to acquire a connection permit from the semaphore.
fn acquire_connection_permit(
    semaphore: &Arc<Semaphore>,
    peer_id: &str,
) -> Result<tokio::sync::OwnedSemaphorePermit, AcceptError> {
    semaphore.clone().try_acquire_owned().map_err(|_| {
        warn!(peer = %peer_id, max = MAX_DOCS_CONNECTIONS, "docs sync connection limit reached, rejecting");
        AcceptError::from_err(std::io::Error::other("connection limit reached"))
    })
}

/// Run the iroh-docs sync protocol with namespace access control.
async fn run_sync_protocol(
    sync_handle: iroh_docs::actor::SyncHandle,
    connection: Connection,
    namespace_id: NamespaceId,
) -> Result<SyncFinished, net::AcceptError> {
    net::handle_connection(
        sync_handle,
        connection,
        |requested_namespace, peer| {
            async move {
                if requested_namespace == namespace_id {
                    debug!(peer = %peer.fmt_short(), namespace = %requested_namespace, "accepting sync request");
                    AcceptOutcome::Allow
                } else {
                    warn!(peer = %peer.fmt_short(), requested = %requested_namespace, our_namespace = %namespace_id, "rejecting sync request for unknown namespace");
                    AcceptOutcome::Reject(net::AbortReason::NotFound)
                }
            }
        },
        None,
    )
    .await
}

/// Handle the sync protocol result and emit completion event.
fn handle_sync_result(
    result: Result<SyncFinished, net::AcceptError>,
    peer_id: &str,
    duration_ms: u64,
    event_broadcaster: &Option<Arc<DocsEventBroadcaster>>,
) -> Result<(), AcceptError> {
    match result {
        Ok(finished) => {
            let entries_synced = finished.outcome.num_sent + finished.outcome.num_recv;
            info!(
                peer = %finished.peer.fmt_short(),
                namespace = %finished.namespace,
                sent = finished.outcome.num_sent,
                recv = finished.outcome.num_recv,
                connect_ms = ?finished.timings.connect.as_millis(),
                process_ms = ?finished.timings.process.as_millis(),
                "docs sync completed"
            );
            if let Some(broadcaster) = event_broadcaster {
                broadcaster.emit_sync_completed(peer_id, entries_synced as u64, duration_ms);
            }
            Ok(())
        }
        Err(err) => {
            warn!(peer = ?err.peer(), namespace = ?err.namespace(), error = %err, "docs sync failed");
            if let Some(broadcaster) = event_broadcaster {
                broadcaster.emit_sync_completed(peer_id, 0, duration_ms);
            }
            Err(AcceptError::from_err(std::io::Error::other(err.to_string())))
        }
    }
}
