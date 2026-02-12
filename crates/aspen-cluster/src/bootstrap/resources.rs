//! Resource structs for organizing NodeHandle fields.
//!
//! Each resource struct groups related components that share a lifecycle
//! and shutdown sequence. This improves maintainability and makes the
//! shutdown ordering explicit.

use std::sync::Arc;

#[cfg(feature = "blob")]
use aspen_blob::IrohBlobStore;
#[cfg(feature = "docs")]
use aspen_docs::DocsEventBroadcaster;
use aspen_raft::StateMachineVariant;
use aspen_raft::log_subscriber::LogEntryPayload;
use aspen_raft::node::RaftNode;
use aspen_raft::node::RaftNodeHealth;
use aspen_raft::server::RaftRpcServer;
use aspen_raft::supervisor::Supervisor;
use iroh_gossip::proto::TopicId;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::info;

use crate::IrohEndpointManager;
use crate::IrpcRaftNetworkFactory;
use crate::gossip_discovery::GossipPeerDiscovery;

// ============================================================================
// Storage Resources
// ============================================================================

/// Storage and consensus resources for the node.
///
/// Contains the Raft node, state machine, and maintenance tasks.
/// These resources are tightly coupled and share the same lifecycle.
pub struct StorageResources {
    /// Raft node (direct wrapper around OpenRaft).
    pub raft_node: Arc<RaftNode>,
    /// State machine variant holding the storage backend.
    pub state_machine: StateMachineVariant,
    /// TTL cleanup task cancellation token.
    /// None when using InMemory storage.
    pub ttl_cleanup_cancel: Option<CancellationToken>,
}

impl StorageResources {
    /// Shutdown storage-related background tasks.
    pub fn shutdown(&self) {
        if let Some(cancel) = &self.ttl_cleanup_cancel {
            info!("shutting down TTL cleanup task");
            cancel.cancel();
        }
    }
}

// ============================================================================
// Network Resources
// ============================================================================

/// P2P networking resources for the node.
///
/// Contains the Iroh endpoint, network factory, and optional RPC server.
/// These resources manage all inter-node communication.
pub struct NetworkResources {
    /// Iroh endpoint manager.
    pub iroh_manager: Arc<IrohEndpointManager>,
    /// IRPC network factory for dynamic peer addition.
    pub network_factory: Arc<IrpcRaftNetworkFactory>,
    /// Legacy RPC server (None when using Router-based architecture).
    pub rpc_server: Option<RaftRpcServer>,
    /// Blob store for content-addressed storage (optional).
    #[cfg(feature = "blob")]
    pub blob_store: Option<Arc<IrohBlobStore>>,
}

impl NetworkResources {
    /// Shutdown network resources in correct order.
    pub async fn shutdown(&mut self) {
        // Stop RPC server if present
        if let Some(rpc_server) = self.rpc_server.take() {
            info!("shutting down legacy RPC server");
            if let Err(err) = rpc_server.shutdown().await {
                error!(error = ?err, "failed to shutdown RPC server gracefully");
            }
        }

        // Shutdown blob store if present
        #[cfg(feature = "blob")]
        if let Some(blob_store) = &self.blob_store {
            info!("shutting down blob store");
            if let Err(err) = blob_store.shutdown().await {
                error!(error = ?err, "failed to shutdown blob store gracefully");
            }
        }

        // Shutdown Iroh endpoint
        info!("shutting down Iroh endpoint");
        if let Err(err) = self.iroh_manager.shutdown().await {
            error!(error = ?err, "failed to shutdown Iroh endpoint gracefully");
        }
    }
}

// ============================================================================
// Discovery Resources
// ============================================================================

/// Peer and content discovery resources.
///
/// Contains gossip-based discovery and global DHT discovery.
/// These are optional features that help nodes find each other.
pub struct DiscoveryResources {
    /// Gossip discovery service (if enabled).
    pub gossip_discovery: Option<GossipPeerDiscovery>,
    /// Gossip topic ID for peer discovery and cluster tickets.
    pub gossip_topic_id: TopicId,
    /// Global content discovery service (optional).
    pub content_discovery: Option<crate::content_discovery::ContentDiscoveryService>,
    /// Content discovery service cancellation token.
    pub content_discovery_cancel: Option<CancellationToken>,
}

impl DiscoveryResources {
    /// Shutdown discovery services.
    pub async fn shutdown(&mut self) {
        // Stop gossip discovery if enabled
        if let Some(gossip_discovery) = self.gossip_discovery.take() {
            info!("shutting down gossip discovery");
            if let Err(err) = gossip_discovery.shutdown().await {
                error!(error = ?err, "failed to shutdown gossip discovery gracefully");
            }
        }

        // Shutdown content discovery if present
        if let Some(cancel_token) = &self.content_discovery_cancel {
            info!("shutting down content discovery");
            cancel_token.cancel();
        }
    }
}

// ============================================================================
// Sync Resources
// ============================================================================

/// Document synchronization resources.
///
/// Contains log broadcast and docs sync lifecycle management.
/// These enable real-time data replication via iroh-docs.
pub struct SyncResources {
    /// Log broadcast sender for DocsExporter and other subscribers.
    pub log_broadcast: Option<broadcast::Sender<LogEntryPayload>>,
    /// DocsExporter cancellation token.
    pub docs_exporter_cancel: Option<CancellationToken>,
    /// Sync event listener cancellation token.
    pub sync_event_listener_cancel: Option<CancellationToken>,
    /// DocsSyncService cancellation token.
    pub docs_sync_service_cancel: Option<CancellationToken>,
    /// Docs sync resources for iroh-docs operations.
    #[cfg(feature = "docs")]
    pub docs_sync: Option<Arc<aspen_docs::DocsSyncResources>>,
    /// Peer manager for cluster-to-cluster synchronization.
    #[cfg(feature = "docs")]
    pub peer_manager: Option<Arc<aspen_docs::PeerManager>>,
}

impl SyncResources {
    /// Shutdown sync services in correct order.
    pub fn shutdown(&self) {
        // Shutdown sync event listener first
        if let Some(cancel) = &self.sync_event_listener_cancel {
            info!("shutting down sync event listener");
            cancel.cancel();
        }

        // Shutdown DocsSyncService
        if let Some(cancel) = &self.docs_sync_service_cancel {
            info!("shutting down docs sync service");
            cancel.cancel();
        }

        // Shutdown DocsExporter last
        if let Some(cancel) = &self.docs_exporter_cancel {
            info!("shutting down DocsExporter");
            cancel.cancel();
        }
    }
}

// ============================================================================
// Worker Resources
// ============================================================================

/// Distributed job execution resources.
///
/// Contains the worker service for processing jobs from the queue.
pub struct WorkerResources {
    /// Worker service for distributed job execution.
    #[cfg(feature = "jobs")]
    pub worker_service: Option<Arc<crate::worker_service::WorkerService>>,
    /// Worker service cancellation token.
    pub worker_service_cancel: Option<CancellationToken>,
}

impl WorkerResources {
    /// Shutdown worker service.
    pub fn shutdown(&self) {
        if let Some(cancel) = &self.worker_service_cancel {
            info!("shutting down worker service");
            cancel.cancel();
        }
    }
}

// ============================================================================
// Blob Replication Resources
// ============================================================================

/// Blob replication resources.
///
/// Contains the BlobReplicationManager for coordinating blob replication
/// across cluster nodes with configurable replication factor and failure
/// domain awareness.
pub struct BlobReplicationResources {
    /// Blob replication manager (None if replication disabled or replication_factor=1).
    pub replication_manager: Option<aspen_blob::BlobReplicationManager>,
    /// Cancellation token for the replication manager background task.
    pub replication_cancel: Option<CancellationToken>,
    /// JoinHandle for the replication manager task.
    pub replication_task: Option<tokio::task::JoinHandle<()>>,
    /// Cancellation token for the topology watcher task.
    pub topology_cancel: Option<CancellationToken>,
}

impl BlobReplicationResources {
    /// Create disabled replication resources.
    pub fn disabled() -> Self {
        Self {
            replication_manager: None,
            replication_cancel: None,
            replication_task: None,
            topology_cancel: None,
        }
    }

    /// Wire up topology updates from Raft membership changes.
    ///
    /// This spawns a background task that watches the Raft metrics channel
    /// for membership changes and updates the BlobReplicationManager's
    /// topology accordingly.
    ///
    /// # Arguments
    ///
    /// * `metrics_rx` - Watch receiver for Raft metrics
    /// * `extractor` - Function to extract `Vec<NodeInfo>` from metrics
    ///
    /// # Returns
    ///
    /// Returns true if the watcher was successfully started, false if
    /// replication is disabled.
    pub fn wire_topology_watcher<M>(
        &mut self,
        metrics_rx: tokio::sync::watch::Receiver<M>,
        extractor: aspen_blob::replication::topology_watcher::NodeInfoExtractor<M>,
    ) -> bool
    where
        M: Send + Sync + 'static,
    {
        let Some(ref manager) = self.replication_manager else {
            tracing::debug!("topology watcher not started: replication disabled");
            return false;
        };

        let cancel = aspen_blob::spawn_topology_watcher(metrics_rx, extractor, manager.clone());
        self.topology_cancel = Some(cancel);
        info!("blob topology watcher started");
        true
    }

    /// Shutdown blob replication resources.
    pub async fn shutdown(&mut self) {
        // Stop topology watcher first
        if let Some(cancel) = self.topology_cancel.take() {
            tracing::debug!("stopping blob topology watcher");
            cancel.cancel();
        }

        if let Some(cancel) = self.replication_cancel.take() {
            info!("shutting down blob replication manager");
            cancel.cancel();
        }

        // Wait for the task to complete
        if let Some(task) = self.replication_task.take() {
            if let Err(e) = task.await {
                tracing::warn!(error = ?e, "blob replication task failed during shutdown");
            }
        }
    }
}

// ============================================================================
// Hook Resources
// ============================================================================

/// Event hook system resources.
///
/// Contains the HookService for dispatching events to registered handlers.
/// The event bridges subscribe to broadcast channels and convert events
/// into hook events:
/// - Raft log bridge: Converts committed log entries into hook events
/// - Blob bridge: Converts blob store events (add, download, protect, etc.)
/// - Docs bridge: Converts docs sync events (sync started/completed, import/export)
/// - System events bridge: Monitors Raft metrics for LeaderElected and HealthChanged events
/// - TTL events bridge: Emits TtlExpired events when keys expire
pub struct HookResources {
    /// Hook service for event dispatch (None if hooks disabled).
    #[cfg(feature = "hooks")]
    pub hook_service: Option<Arc<aspen_hooks::HookService>>,
    /// Cancellation token for raft log event bridge task.
    pub event_bridge_cancel: Option<CancellationToken>,
    /// Cancellation token for blob event bridge task.
    pub blob_bridge_cancel: Option<CancellationToken>,
    /// Cancellation token for docs event bridge task.
    pub docs_bridge_cancel: Option<CancellationToken>,
    /// Cancellation token for system events bridge task.
    pub system_events_bridge_cancel: Option<CancellationToken>,
    /// Cancellation token for TTL events bridge task.
    pub ttl_events_bridge_cancel: Option<CancellationToken>,
    /// Cancellation token for snapshot events bridge task.
    pub snapshot_events_bridge_cancel: Option<CancellationToken>,
}

impl HookResources {
    /// Create disabled hook resources.
    pub fn disabled() -> Self {
        Self {
            #[cfg(feature = "hooks")]
            hook_service: None,
            event_bridge_cancel: None,
            blob_bridge_cancel: None,
            docs_bridge_cancel: None,
            system_events_bridge_cancel: None,
            ttl_events_bridge_cancel: None,
            snapshot_events_bridge_cancel: None,
        }
    }

    /// Shutdown hook service resources.
    pub fn shutdown(&self) {
        if let Some(cancel) = &self.event_bridge_cancel {
            info!("shutting down raft log event bridge");
            cancel.cancel();
        }
        if let Some(cancel) = &self.blob_bridge_cancel {
            info!("shutting down blob event bridge");
            cancel.cancel();
        }
        if let Some(cancel) = &self.docs_bridge_cancel {
            info!("shutting down docs event bridge");
            cancel.cancel();
        }
        if let Some(cancel) = &self.system_events_bridge_cancel {
            info!("shutting down system events bridge");
            cancel.cancel();
        }
        if let Some(cancel) = &self.ttl_events_bridge_cancel {
            info!("shutting down TTL events bridge");
            cancel.cancel();
        }
        if let Some(cancel) = &self.snapshot_events_bridge_cancel {
            info!("shutting down snapshot events bridge");
            cancel.cancel();
        }
    }
}

// ============================================================================
// Shutdown Coordinator
// ============================================================================

/// Coordinates graceful shutdown of all node resources.
///
/// Ensures resources are shut down in the correct order to prevent
/// data loss and resource leaks.
pub struct ShutdownCoordinator {
    /// Master cancellation token for the node.
    pub shutdown_token: CancellationToken,
    /// Supervisor for automatic restarts.
    pub supervisor: Arc<Supervisor>,
    /// Health monitor for Raft node.
    pub health_monitor: Arc<RaftNodeHealth>,
}

impl ShutdownCoordinator {
    /// Signal shutdown to all components.
    pub fn signal_shutdown(&self) {
        self.shutdown_token.cancel();
    }

    /// Create a child cancellation token.
    ///
    /// Child tokens are automatically cancelled when the parent is cancelled,
    /// but can also be cancelled independently for targeted shutdown.
    pub fn child_token(&self) -> CancellationToken {
        self.shutdown_token.child_token()
    }

    /// Stop the supervisor.
    pub fn stop_supervisor(&self) {
        info!("shutting down supervisor");
        self.supervisor.stop();
    }
}
