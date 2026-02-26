//! Document synchronization initialization for cluster nodes.
//!
//! This module handles iroh-docs based synchronization including DocsExporter,
//! DocsSyncService, and peer manager setup.

use std::sync::Arc;

#[cfg(feature = "blob")]
use aspen_blob::IrohBlobStore;
#[cfg(feature = "docs")]
use aspen_docs::DocsEventBroadcaster;
use aspen_raft::log_subscriber::LogEntryPayload;
use aspen_raft::node::RaftNode;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::debug as trace_debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::IrohEndpointManager;
use crate::config::NodeConfig;

/// Initialize peer manager if enabled.
///
/// Creates a PeerManager for cluster-to-cluster synchronization using iroh-docs.
/// The peer manager coordinates connections to peer clusters and routes
/// incoming entries through the DocsImporter for conflict resolution.
pub(super) fn initialize_peer_manager(
    config: &NodeConfig,
    raft_node: &Arc<RaftNode>,
) -> Option<Arc<aspen_docs::PeerManager>> {
    if !config.peer_sync.is_enabled {
        return None;
    }

    use aspen_docs::DocsImporter;
    use aspen_docs::PeerManager;

    let importer = Arc::new(DocsImporter::new(config.cookie.clone(), raft_node.clone(), &config.node_id.to_string()));
    let manager = Arc::new(PeerManager::new(config.cookie.clone(), importer));

    info!(node_id = config.node_id, "peer sync initialized");
    Some(manager)
}

/// Initialize DocsExporter and P2P sync if enabled.
///
/// Returns (docs_exporter_cancel, docs_sync).
///
/// If no namespace_secret is configured, derives one from the cluster cookie.
/// This ensures all nodes with the same cookie share the same docs namespace,
/// enabling automatic cross-node replication without explicit configuration.
pub(super) async fn initialize_docs_export(
    config: &NodeConfig,
    data_dir: &std::path::Path,
    log_broadcast: Option<&broadcast::Sender<LogEntryPayload>>,
    #[cfg(feature = "blob")] blob_store: Option<&Arc<IrohBlobStore>>,
    #[cfg(feature = "docs")] docs_broadcaster: Option<Arc<DocsEventBroadcaster>>,
) -> anyhow::Result<(Option<CancellationToken>, Option<Arc<aspen_docs::DocsSyncResources>>)> {
    if !config.docs.is_enabled {
        info!(node_id = config.node_id, "DocsExporter disabled by configuration");
        return Ok((None, None));
    }

    let Some(sender) = log_broadcast else {
        warn!(node_id = config.node_id, "DocsExporter not started - log broadcast channel not available");
        return Ok((None, None));
    };

    use aspen_docs::BlobBackedDocsWriter;
    use aspen_docs::DocsExporter;
    use aspen_docs::DocsSyncResources;
    use aspen_docs::SyncHandleDocsWriter;
    use aspen_docs::init_docs_resources;
    use sha2::Digest;
    use sha2::Sha256;

    // Derive namespace secret from cookie if not explicitly configured.
    // This ensures all nodes with the same cookie share the same docs namespace.
    let namespace_secret = config.docs.namespace_secret.clone().unwrap_or_else(|| {
        let mut hasher = Sha256::new();
        hasher.update(b"aspen-docs-namespace:");
        hasher.update(config.cookie.as_bytes());
        let hash = hasher.finalize();
        let derived = hex::encode(hash);
        info!(
            node_id = config.node_id,
            cookie = %config.cookie,
            "derived docs namespace secret from cluster cookie"
        );
        derived
    });

    let resources = match init_docs_resources(
        data_dir,
        config.docs.in_memory,
        Some(&namespace_secret),
        config.docs.author_secret.as_deref(),
    ) {
        Ok(r) => r,
        Err(err) => {
            error!(
                error = ?err,
                node_id = config.node_id,
                "failed to initialize iroh-docs resources, continuing without docs export"
            );
            return Ok((None, None));
        }
    };

    let namespace_id = resources.namespace_id;
    let in_memory = config.docs.in_memory;

    let docs_sync = DocsSyncResources::from_docs_resources(resources, &format!("node-{}", config.node_id));
    #[cfg(feature = "docs")]
    let docs_sync = match &docs_broadcaster {
        Some(b) => docs_sync.with_event_broadcaster(Arc::clone(b)),
        None => docs_sync,
    };

    if let Err(err) = docs_sync.open_replica().await {
        error!(
            error = ?err,
            node_id = config.node_id,
            namespace_id = %namespace_id,
            "failed to open docs replica"
        );
    }

    #[cfg(feature = "blob")]
    let writer: Arc<dyn aspen_docs::DocsWriter> = match blob_store {
        Some(store) => {
            info!(
                node_id = config.node_id,
                namespace_id = %namespace_id,
                "using BlobBackedDocsWriter for full P2P content transfer"
            );
            Arc::new(BlobBackedDocsWriter::new(
                docs_sync.sync_handle.clone(),
                docs_sync.namespace_id,
                docs_sync.author.clone(),
                (*store).clone(),
            ))
        }
        None => {
            info!(
                node_id = config.node_id,
                namespace_id = %namespace_id,
                "using SyncHandleDocsWriter (metadata sync only, no blob storage)"
            );
            Arc::new(SyncHandleDocsWriter::new(
                docs_sync.sync_handle.clone(),
                docs_sync.namespace_id,
                docs_sync.author.clone(),
            ))
        }
    };

    #[cfg(not(feature = "blob"))]
    let writer: Arc<dyn aspen_docs::DocsWriter> = {
        info!(
            node_id = config.node_id,
            namespace_id = %namespace_id,
            "using SyncHandleDocsWriter (metadata sync only, no blob storage)"
        );
        Arc::new(SyncHandleDocsWriter::new(
            docs_sync.sync_handle.clone(),
            docs_sync.namespace_id,
            docs_sync.author.clone(),
        ))
    };

    let exporter = DocsExporter::new(writer);
    #[cfg(feature = "docs")]
    let exporter = match &docs_broadcaster {
        Some(b) => exporter.with_event_broadcaster(Arc::clone(b)),
        None => exporter,
    };
    let exporter = Arc::new(exporter);
    let receiver = sender.subscribe();
    let cancel_token = exporter.spawn(receiver);

    info!(
        node_id = config.node_id,
        namespace_id = %namespace_id,
        in_memory = in_memory,
        p2p_sync = true,
        "DocsExporter started with P2P sync enabled"
    );

    Ok((Some(cancel_token), Some(Arc::new(docs_sync))))
}

/// Wire up docs sync services (sync event listener and DocsSyncService).
///
/// This function starts the background services that enable full P2P docs sync:
/// 1. Sync Event Listener: Listens for RemoteInsert events from iroh-docs sync and forwards them to
///    DocsImporter for priority-based import.
/// 2. DocsSyncService: Periodically initiates outbound sync to peer clusters.
///
/// Both services require docs_sync, blob_store, and peer_manager to be available.
///
/// # Arguments
/// * `config` - Node configuration
/// * `docs_sync` - DocsSyncResources (if enabled)
/// * `blob_store` - Blob store for content fetching (if enabled)
/// * `peer_manager` - Peer manager for tracking peer connections
/// * `iroh_manager` - Iroh endpoint manager for network access
///
/// # Returns
/// Tuple of (sync_event_listener_cancel, docs_sync_service_cancel)
pub(super) async fn wire_docs_sync_services(
    config: &NodeConfig,
    docs_sync: &Option<Arc<aspen_docs::DocsSyncResources>>,
    #[cfg(feature = "blob")] blob_store: &Option<Arc<IrohBlobStore>>,
    peer_manager: &Option<Arc<aspen_docs::PeerManager>>,
    iroh_manager: &Arc<IrohEndpointManager>,
) -> (Option<CancellationToken>, Option<CancellationToken>) {
    use aspen_docs::DocsSyncService;

    // All three components are required for full docs sync
    #[cfg(feature = "blob")]
    let (Some(docs_sync), Some(blob_store), Some(peer_manager)) =
        (docs_sync.as_ref(), blob_store.as_ref(), peer_manager.as_ref())
    else {
        // Log what's missing for debugging
        if docs_sync.is_none() {
            trace_debug!(node_id = config.node_id, "docs sync services not started: docs_sync not available");
        } else if blob_store.is_none() {
            trace_debug!(node_id = config.node_id, "docs sync services not started: blob_store not available");
        } else {
            trace_debug!(node_id = config.node_id, "docs sync services not started: peer_manager not available");
        }
        return (None, None);
    };

    #[cfg(not(feature = "blob"))]
    {
        trace_debug!(node_id = config.node_id, "docs sync services not started: blob feature not enabled");
        return (None, None);
    }

    // Start sync event listener
    // This listens for RemoteInsert events from iroh-docs sync and forwards them to DocsImporter
    #[cfg(feature = "blob")]
    let sync_event_listener_cancel = match docs_sync
        .spawn_sync_event_listener(peer_manager.importer().clone(), config.cookie.clone(), blob_store.clone())
        .await
    {
        Ok(cancel) => {
            info!(
                node_id = config.node_id,
                namespace = %docs_sync.namespace_id,
                "sync event listener started"
            );
            Some(cancel)
        }
        Err(err) => {
            warn!(
                node_id = config.node_id,
                error = %err,
                "failed to start sync event listener"
            );
            None
        }
    };

    // Start DocsSyncService for periodic outbound sync
    // Uses PeerManager to get known peers - initially empty but peers can be added
    // via PeerManager.add_peer() with AspenDocsTickets for cross-cluster sync.
    #[cfg(feature = "blob")]
    let sync_service = Arc::new(DocsSyncService::new(docs_sync.clone(), iroh_manager.endpoint().clone()));

    // Peer provider that extracts EndpointAddrs from PeerManager's ticket peers
    #[cfg(feature = "blob")]
    let peer_manager_clone = peer_manager.clone();
    #[cfg(feature = "blob")]
    let docs_sync_service_cancel = sync_service.spawn(move || {
        // Get all connected peers from PeerManager
        // Note: get_peer_addresses() is async but spawn() requires sync closure.
        // We use block_in_place to bridge the async/sync gap since this
        // closure is called from within a tokio runtime context.
        let peer_manager_for_closure = peer_manager_clone.clone();
        tokio::task::block_in_place(move || {
            // Use Handle::current() to run the async operation in the current runtime
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async move {
                // Use the public method to get peer addresses
                peer_manager_for_closure.get_peer_addresses().await
            })
        })
    });

    #[cfg(feature = "blob")]
    {
        info!(
            node_id = config.node_id,
            sync_event_listener = sync_event_listener_cancel.is_some(),
            docs_sync_service = true,
            "docs sync services initialized"
        );

        (sync_event_listener_cancel, Some(docs_sync_service_cancel))
    }
}
