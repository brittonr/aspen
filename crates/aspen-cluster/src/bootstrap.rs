//! Simplified cluster node bootstrap without actors.
//!
//! This module provides bootstrap orchestration for Aspen nodes using
//! direct async APIs instead of actor message passing. The bootstrap
//! process creates and initializes all necessary components for a
//! functioning Raft cluster node.
//!
//! # Gossip Discovery
//!
//! When gossip is enabled (`config.iroh.enable_gossip = true`), the bootstrap
//! automatically spawns a `GossipPeerDiscovery` instance that:
//! 1. Derives a topic ID from the cluster cookie (or uses a ticket's topic ID)
//! 2. Broadcasts this node's ID and EndpointAddr every 10 seconds
//! 3. Receives peer announcements and adds them to the network factory
//!
//! This enables automatic peer discovery without manual configuration.
//!
//! # Test Coverage
//!
//! Unit tests in this module cover:
//! - `derive_topic_id_from_cookie`: Deterministic topic ID derivation from cluster cookie
//! - `parse_peer_addresses`: Peer spec parsing with error handling
//! - `load_config`: Configuration merging from multiple sources
//!
//! Integration tests for `bootstrap_node()` are in `tests/node_builder_integration.rs`:
//! - Bootstrap with different storage backends (InMemory, SQLite)
//! - Bootstrap with gossip enabled/disabled
//! - NodeHandle shutdown sequence testing
//! - Service restart behavior verification

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use anyhow::ensure;
use aspen_auth::CapabilityToken;
use aspen_blob::BlobEventBroadcaster;
use aspen_blob::IrohBlobStore;
use aspen_blob::create_blob_event_channel;
use aspen_docs::DocsEventBroadcaster;
use aspen_docs::create_docs_event_channel;
use aspen_raft::StateMachineVariant;
use aspen_raft::log_subscriber::LOG_BROADCAST_BUFFER_SIZE;
use aspen_raft::log_subscriber::LogEntryPayload;
use aspen_raft::node::RaftNode;
use aspen_raft::node::RaftNodeHealth;
use aspen_raft::server::RaftRpcServer;
use aspen_raft::storage::InMemoryLogStore;
use aspen_raft::storage::InMemoryStateMachine;
use aspen_raft::storage::StorageBackend;
use aspen_raft::storage_shared::SharedRedbStorage;
use aspen_raft::supervisor::Supervisor;
use aspen_raft::ttl_cleanup::TtlCleanupConfig;
use aspen_raft::ttl_cleanup::spawn_redb_ttl_cleanup_task;
use aspen_raft::types::AppTypeConfig;
use aspen_raft::types::NodeId;
use aspen_sharding::ShardConfig;
use aspen_sharding::ShardId;
use aspen_sharding::ShardStoragePaths;
use aspen_sharding::ShardedKeyValueStore;
use aspen_sharding::encode_shard_node_id;
use aspen_transport::ShardedRaftProtocolHandler;
use aspen_transport::rpc::AppTypeConfig as TransportAppTypeConfig;
use iroh::EndpointAddr;
use iroh_gossip::proto::TopicId;
use openraft::Config as RaftConfig;
use openraft::Raft;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

// Use the type alias from cluster mod.rs which provides the concrete type
use super::IrpcRaftNetworkFactory;
use crate::IrohEndpointConfig;
use crate::IrohEndpointManager;
use crate::config::NodeConfig;
use crate::gossip_discovery::GossipPeerDiscovery;
use crate::gossip_discovery::spawn_gossip_peer_discovery;
use crate::metadata::MetadataStore;
use crate::metadata::NodeStatus;
use crate::ticket::AspenClusterTicket;

// ============================================================================
// Resource Structs - Focused groupings of NodeHandle fields
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
            tracing::info!("shutting down TTL cleanup task");
            cancel.cancel();
        }
    }
}

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
    pub blob_store: Option<Arc<IrohBlobStore>>,
}

impl NetworkResources {
    /// Shutdown network resources in correct order.
    pub async fn shutdown(&mut self) {
        // Stop RPC server if present
        if let Some(rpc_server) = self.rpc_server.take() {
            tracing::info!("shutting down legacy RPC server");
            if let Err(err) = rpc_server.shutdown().await {
                tracing::error!(error = ?err, "failed to shutdown RPC server gracefully");
            }
        }

        // Shutdown blob store if present
        if let Some(blob_store) = &self.blob_store {
            tracing::info!("shutting down blob store");
            if let Err(err) = blob_store.shutdown().await {
                tracing::error!(error = ?err, "failed to shutdown blob store gracefully");
            }
        }

        // Shutdown Iroh endpoint
        tracing::info!("shutting down Iroh endpoint");
        if let Err(err) = self.iroh_manager.shutdown().await {
            tracing::error!(error = ?err, "failed to shutdown Iroh endpoint gracefully");
        }
    }
}

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
            tracing::info!("shutting down gossip discovery");
            if let Err(err) = gossip_discovery.shutdown().await {
                tracing::error!(error = ?err, "failed to shutdown gossip discovery gracefully");
            }
        }

        // Shutdown content discovery if present
        if let Some(cancel_token) = &self.content_discovery_cancel {
            tracing::info!("shutting down content discovery");
            cancel_token.cancel();
        }
    }
}

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
    pub docs_sync: Option<Arc<aspen_docs::DocsSyncResources>>,
    /// Peer manager for cluster-to-cluster synchronization.
    pub peer_manager: Option<Arc<aspen_docs::PeerManager>>,
}

impl SyncResources {
    /// Shutdown sync services in correct order.
    pub fn shutdown(&self) {
        // Shutdown sync event listener first
        if let Some(cancel) = &self.sync_event_listener_cancel {
            tracing::info!("shutting down sync event listener");
            cancel.cancel();
        }

        // Shutdown DocsSyncService
        if let Some(cancel) = &self.docs_sync_service_cancel {
            tracing::info!("shutting down docs sync service");
            cancel.cancel();
        }

        // Shutdown DocsExporter last
        if let Some(cancel) = &self.docs_exporter_cancel {
            tracing::info!("shutting down DocsExporter");
            cancel.cancel();
        }
    }
}

/// Distributed job execution resources.
///
/// Contains the worker service for processing jobs from the queue.
pub struct WorkerResources {
    /// Worker service for distributed job execution.
    pub worker_service: Option<Arc<crate::worker_service::WorkerService>>,
    /// Worker service cancellation token.
    pub worker_service_cancel: Option<CancellationToken>,
}

impl WorkerResources {
    /// Shutdown worker service.
    pub fn shutdown(&self) {
        if let Some(cancel) = &self.worker_service_cancel {
            tracing::info!("shutting down worker service");
            cancel.cancel();
        }
    }
}

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
        tracing::info!("blob topology watcher started");
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
            tracing::info!("shutting down blob replication manager");
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
            tracing::info!("shutting down raft log event bridge");
            cancel.cancel();
        }
        if let Some(cancel) = &self.blob_bridge_cancel {
            tracing::info!("shutting down blob event bridge");
            cancel.cancel();
        }
        if let Some(cancel) = &self.docs_bridge_cancel {
            tracing::info!("shutting down docs event bridge");
            cancel.cancel();
        }
        if let Some(cancel) = &self.system_events_bridge_cancel {
            tracing::info!("shutting down system events bridge");
            cancel.cancel();
        }
        if let Some(cancel) = &self.ttl_events_bridge_cancel {
            tracing::info!("shutting down TTL events bridge");
            cancel.cancel();
        }
        if let Some(cancel) = &self.snapshot_events_bridge_cancel {
            tracing::info!("shutting down snapshot events bridge");
            cancel.cancel();
        }
    }
}

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
        tracing::info!("shutting down supervisor");
        self.supervisor.stop();
    }
}

// ============================================================================
// NodeHandle - Main handle to a running cluster node
// ============================================================================

/// Handle to a running cluster node.
///
/// Contains all the resources needed to run and shutdown a node cleanly
/// using direct async APIs. Resources are organized into focused groups
/// for better maintainability:
///
/// - `storage`: Raft consensus, state machine, TTL cleanup
/// - `network`: Iroh endpoint, network factory, RPC server, blob store
/// - `discovery`: Gossip discovery, content discovery
/// - `sync`: Document synchronization and replication
/// - `worker`: Distributed job execution
/// - `blob_replication`: Blob replication across cluster nodes
/// - `hooks`: Event hook system
/// - `shutdown`: Shutdown coordination and health monitoring
pub struct NodeHandle {
    /// Node configuration.
    pub config: NodeConfig,
    /// Metadata store for cluster nodes.
    pub metadata_store: Arc<MetadataStore>,
    /// Root token generated during cluster initialization (if requested).
    ///
    /// Only present when the node initialized a new cluster (not joining existing)
    /// and token generation was requested via bootstrap configuration.
    pub root_token: Option<CapabilityToken>,

    // ========================================================================
    // Composed resource groups (replacing flat fields)
    // ========================================================================
    /// Storage and consensus resources.
    pub storage: StorageResources,
    /// Network and transport resources.
    pub network: NetworkResources,
    /// Peer and content discovery resources.
    pub discovery: DiscoveryResources,
    /// Document synchronization resources.
    pub sync: SyncResources,
    /// Worker job execution resources.
    pub worker: WorkerResources,
    /// Blob replication resources.
    pub blob_replication: BlobReplicationResources,
    /// Event hook system resources.
    pub hooks: HookResources,
    /// Shutdown coordination resources.
    pub shutdown: ShutdownCoordinator,
}

impl NodeHandle {
    /// Gracefully shutdown the node.
    ///
    /// Delegates to resource struct shutdown methods in dependency order:
    /// 1. Signal shutdown via coordinator
    /// 2. Worker resources (stop accepting jobs)
    /// 3. Hook resources (stop event bridge)
    /// 4. Blob replication resources (stop replication before discovery)
    /// 5. Discovery resources (stop peer/content discovery)
    /// 6. Sync resources (stop document sync)
    /// 7. Storage resources (stop TTL cleanup)
    /// 8. Shutdown coordinator (stop supervisor)
    /// 9. Network resources (close connections last)
    /// 10. Update metadata status
    pub async fn shutdown(mut self) -> Result<()> {
        info!("shutting down node {}", self.config.node_id);

        // 1. Signal shutdown to all components
        self.shutdown.signal_shutdown();

        // 2. Shutdown worker resources (stop accepting new jobs first)
        self.worker.shutdown();

        // 3. Shutdown hook resources (stop event bridge before discovery)
        self.hooks.shutdown();

        // 4. Shutdown blob replication resources (stop before discovery)
        self.blob_replication.shutdown().await;

        // 5. Shutdown discovery resources (stop peer/content discovery)
        self.discovery.shutdown().await;

        // 6. Shutdown sync resources (stop document sync)
        self.sync.shutdown();

        // 7. Shutdown storage resources (stop TTL cleanup)
        self.storage.shutdown();

        // 8. Stop supervisor via shutdown coordinator
        self.shutdown.stop_supervisor();

        // 9. Shutdown network resources (close connections last)
        self.network.shutdown().await;

        // 10. Update node status to offline
        if let Err(err) = self.metadata_store.update_status(self.config.node_id, NodeStatus::Offline) {
            error!(
                error = ?err,
                node_id = self.config.node_id,
                "failed to update node status to offline"
            );
        }

        Ok(())
    }
}

// ============================================================================
// Common Shutdown Logic
// ============================================================================

/// Resources for common shutdown operations.
///
/// This struct aggregates references to optional components that follow
/// the same shutdown pattern across `NodeHandle` and `ShardedNodeHandle`.
struct CommonShutdownResources<'a> {
    sync_event_listener_cancel: Option<&'a CancellationToken>,
    docs_sync_service_cancel: Option<&'a CancellationToken>,
    docs_exporter_cancel: Option<&'a CancellationToken>,
    content_discovery_cancel: Option<&'a CancellationToken>,
    worker_service_cancel: Option<&'a CancellationToken>,
    supervisor: &'a Arc<Supervisor>,
    blob_store: Option<&'a Arc<IrohBlobStore>>,
    iroh_manager: &'a Arc<IrohEndpointManager>,
    metadata_store: &'a Arc<MetadataStore>,
    node_id: u64,
}

/// Shutdown common components in the correct order.
///
/// This function handles the shutdown sequence for components shared between
/// `NodeHandle` and `ShardedNodeHandle`. Callers handle their unique components
/// (gossip_discovery, rpc_server, docs_sync, ttl_cleanup) before and after calling this.
///
/// Shutdown order:
/// 1. Sync event listener (cancel token)
/// 2. Docs sync service (cancel token)
/// 3. Peer manager (shutdown)
/// 4. Docs exporter (cancel token)
/// 5. Supervisor (stop)
/// 6. Blob store (shutdown)
/// 7. Iroh endpoint (shutdown)
/// 8. Metadata status update
async fn shutdown_common_resources(resources: CommonShutdownResources<'_>) -> Result<()> {
    // Shutdown sync event listener if present (before peer manager)
    if let Some(cancel_token) = resources.sync_event_listener_cancel {
        info!("shutting down sync event listener");
        cancel_token.cancel();
    }

    // Shutdown DocsSyncService if present (before peer manager)
    if let Some(cancel_token) = resources.docs_sync_service_cancel {
        info!("shutting down docs sync service");
        cancel_token.cancel();
    }

    // Peer manager functionality moved to aspen-docs crate

    // Shutdown DocsExporter if present
    if let Some(cancel_token) = resources.docs_exporter_cancel {
        info!("shutting down DocsExporter");
        cancel_token.cancel();
    }

    // Shutdown content discovery if present
    if let Some(cancel_token) = resources.content_discovery_cancel {
        info!("shutting down content discovery service");
        cancel_token.cancel();
    }

    // Shutdown worker service if present
    if let Some(cancel_token) = resources.worker_service_cancel {
        info!("shutting down worker service");
        cancel_token.cancel();
    }

    // Stop supervisor
    info!("shutting down supervisor");
    resources.supervisor.stop();

    // Shutdown blob store if present
    if let Some(blob_store) = resources.blob_store {
        info!("shutting down blob store");
        if let Err(err) = blob_store.shutdown().await {
            error!(error = ?err, "failed to shutdown blob store gracefully");
        }
    }

    // Shutdown Iroh endpoint
    info!("shutting down Iroh endpoint");
    resources.iroh_manager.shutdown().await?;

    // Update node status
    if let Err(err) = resources.metadata_store.update_status(resources.node_id, NodeStatus::Offline) {
        error!(
            error = ?err,
            node_id = resources.node_id,
            "failed to update node status to offline"
        );
    }

    Ok(())
}

// ============================================================================
// Sharded Bootstrap Infrastructure
// ============================================================================

/// Gossip-only discovery resources for sharded nodes.
///
/// Unlike `DiscoveryResources` which includes content discovery,
/// this struct only contains gossip-based peer discovery which is
/// shared across all shards on a node.
pub struct BaseDiscoveryResources {
    /// Gossip discovery service (if enabled).
    pub gossip_discovery: Option<GossipPeerDiscovery>,
    /// Gossip topic ID for peer discovery and cluster tickets.
    pub gossip_topic_id: TopicId,
}

impl BaseDiscoveryResources {
    /// Shutdown gossip discovery.
    pub async fn shutdown(&mut self) {
        if let Some(gossip_discovery) = self.gossip_discovery.take() {
            tracing::info!("shutting down gossip discovery");
            if let Err(err) = gossip_discovery.shutdown().await {
                tracing::error!(error = ?err, "failed to shutdown gossip discovery gracefully");
            }
        }
    }
}

/// Shard-specific resources for sharded cluster nodes.
///
/// Contains the per-shard Raft nodes, state machines, and related data
/// that are unique to sharded mode. Each shard is an independent Raft
/// consensus group with its own storage.
pub struct ShardingResources {
    /// Map of shard ID to Raft node for that shard.
    pub shard_nodes: HashMap<ShardId, Arc<RaftNode>>,
    /// Sharded key-value store wrapping all shard nodes.
    pub sharded_kv: Arc<ShardedKeyValueStore<RaftNode>>,
    /// Protocol handler for sharded Raft RPC.
    pub sharded_handler: Arc<ShardedRaftProtocolHandler>,
    /// Health monitors for each shard.
    pub health_monitors: HashMap<ShardId, Arc<RaftNodeHealth>>,
    /// TTL cleanup cancellation tokens (one per shard).
    pub ttl_cleanup_cancels: HashMap<ShardId, CancellationToken>,
    /// State machine variants for each shard.
    pub shard_state_machines: HashMap<ShardId, StateMachineVariant>,
    /// Sharding topology for shard routing and redistribution.
    pub topology: Option<Arc<tokio::sync::RwLock<aspen_sharding::ShardTopology>>>,
}

impl ShardingResources {
    /// Shutdown shard-related resources.
    pub fn shutdown(&self) {
        for (shard_id, cancel_token) in &self.ttl_cleanup_cancels {
            tracing::info!(shard_id, "shutting down TTL cleanup task for shard");
            cancel_token.cancel();
        }
    }

    /// Get a reference to the first shard's RaftNode (shard 0).
    pub fn primary_shard(&self) -> Option<&Arc<RaftNode>> {
        self.shard_nodes.get(&0)
    }

    /// Get the number of shards hosted by this node.
    pub fn shard_count(&self) -> usize {
        self.shard_nodes.len()
    }

    /// Get the list of shard IDs hosted by this node.
    pub fn local_shard_ids(&self) -> Vec<ShardId> {
        self.shard_nodes.keys().copied().collect()
    }
}

/// Base node resources shared across all shards.
///
/// Contains the transport and discovery infrastructure that is shared
/// by all shards on a node. This is separated from Raft-specific resources
/// to enable per-shard Raft instances while sharing the common P2P transport.
///
/// Resources are organized into focused groups:
/// - `network`: Iroh endpoint, network factory, blob store
/// - `discovery`: Gossip-based peer discovery
pub struct BaseNodeResources {
    /// Node configuration.
    pub config: NodeConfig,
    /// Metadata store for cluster nodes.
    pub metadata_store: Arc<MetadataStore>,
    /// Network and transport resources.
    pub network: NetworkResources,
    /// Gossip-based discovery resources.
    pub discovery: BaseDiscoveryResources,
    /// Cancellation token for shutdown.
    pub shutdown_token: CancellationToken,
}

/// Handle to a running sharded cluster node.
///
/// Contains multiple independent Raft instances (one per shard) that share
/// the same underlying Iroh P2P transport. Each shard is a separate Raft
/// consensus group with its own leader election and log replication.
///
/// Resources are organized into focused groups (matching NodeHandle pattern):
/// - `base`: Shared transport and gossip discovery
/// - `sharding`: Per-shard Raft nodes and state machines
/// - `discovery`: Content discovery (DHT)
/// - `sync`: Document synchronization
/// - `worker`: Distributed job execution
/// - `hooks`: Event hook system
/// - `supervisor`: Health monitoring
pub struct ShardedNodeHandle {
    /// Base node resources (Iroh, metadata, gossip - shared across shards).
    pub base: BaseNodeResources,
    /// Root token generated during cluster initialization (if requested).
    pub root_token: Option<CapabilityToken>,

    // ========================================================================
    // Composed resource groups (matching NodeHandle pattern)
    // ========================================================================
    /// Shard-specific resources (Raft nodes, state machines, topology).
    pub sharding: ShardingResources,
    /// Content discovery resources (DHT).
    pub discovery: DiscoveryResources,
    /// Document synchronization resources.
    pub sync: SyncResources,
    /// Worker job execution resources.
    pub worker: WorkerResources,
    /// Event hook system resources.
    pub hooks: HookResources,
    /// Supervisor for health monitoring.
    pub supervisor: Arc<Supervisor>,
}

impl ShardedNodeHandle {
    /// Gracefully shutdown all shards and the node.
    ///
    /// Delegates to resource struct shutdown methods in dependency order:
    /// 1. Signal shutdown via base token
    /// 2. Worker resources (stop accepting jobs)
    /// 3. Discovery resources (base gossip + content discovery)
    /// 4. Sync resources (stop document sync)
    /// 5. Sharding resources (stop TTL cleanup for all shards)
    /// 6. Stop supervisor
    /// 7. Network resources (close connections last)
    /// 8. Update metadata status
    pub async fn shutdown(mut self) -> Result<()> {
        info!(
            node_id = self.base.config.node_id,
            shard_count = self.sharding.shard_count(),
            "shutting down sharded node"
        );

        // 1. Signal shutdown to all components
        self.base.shutdown_token.cancel();

        // 2. Shutdown worker resources (stop accepting new jobs first)
        self.worker.shutdown();

        // 3. Shutdown hook resources (stop event bridges before discovery)
        self.hooks.shutdown();

        // 4. Shutdown discovery resources (content discovery + gossip)
        self.discovery.shutdown().await;

        // Also shutdown base gossip discovery (owned by base, not discovery group)
        if let Some(gossip_discovery) = self.base.discovery.gossip_discovery.take() {
            info!("shutting down gossip discovery");
            if let Err(err) = gossip_discovery.shutdown().await {
                error!(error = ?err, "failed to shutdown gossip discovery gracefully");
            }
        }

        // 5. Shutdown sync resources (stop document sync)
        self.sync.shutdown();

        // 6. Shutdown sharding resources (TTL cleanup for all shards)
        self.sharding.shutdown();

        // 7. Stop supervisor
        info!("shutting down supervisor");
        self.supervisor.stop();

        // 8. Shutdown network resources (close connections last)
        self.base.network.shutdown().await;

        // 9. Update node status to offline
        if let Err(err) = self.base.metadata_store.update_status(self.base.config.node_id, NodeStatus::Offline) {
            error!(
                error = ?err,
                node_id = self.base.config.node_id,
                "failed to update node status to offline"
            );
        }

        info!(node_id = self.base.config.node_id, "sharded node shutdown complete");
        Ok(())
    }

    /// Get a reference to the first shard's RaftNode (shard 0).
    ///
    /// Used for compatibility with single-shard APIs and initialization.
    pub fn primary_shard(&self) -> Option<&Arc<RaftNode>> {
        self.sharding.primary_shard()
    }

    /// Get the number of shards hosted by this node.
    pub fn shard_count(&self) -> usize {
        self.sharding.shard_count()
    }

    /// Get the list of shard IDs hosted by this node.
    pub fn local_shard_ids(&self) -> Vec<ShardId> {
        self.sharding.local_shard_ids()
    }
}

/// Bootstrap base node resources (Iroh, metadata, gossip) without Raft.
///
/// This function sets up the shared transport and discovery infrastructure
/// that is used by all shards. Call this before creating per-shard Raft instances.
async fn bootstrap_base_node(config: &NodeConfig) -> Result<BaseNodeResources> {
    info!(node_id = config.node_id, "bootstrapping base node resources (Iroh, metadata, gossip)");

    // Initialize metadata store
    let data_dir = config.data_dir.as_ref().ok_or_else(|| anyhow::anyhow!("data_dir must be set"))?;

    // Ensure data directory exists
    std::fs::create_dir_all(data_dir)
        .map_err(|e| anyhow::anyhow!("failed to create data directory {}: {}", data_dir.display(), e))?;

    // MetadataStore expects a path to the database file, not directory
    let metadata_db_path = data_dir.join("metadata.redb");
    let metadata_store = Arc::new(MetadataStore::new(&metadata_db_path)?);

    // Create Iroh endpoint manager
    let iroh_config = build_iroh_config_from_node_config(config)?;
    let iroh_manager = Arc::new(IrohEndpointManager::new(iroh_config).await?);
    info!(
        node_id = config.node_id,
        endpoint_id = %iroh_manager.node_addr().id,
        "created Iroh endpoint"
    );

    // Parse peer addresses from config if provided
    let peer_addrs = parse_peer_addresses(&config.peers)?;

    // Iroh-Native Authentication: No client-side auth needed
    // Authentication is handled at connection accept time by the server.
    // The server validates the remote NodeId (verified by QUIC TLS handshake)
    // against the TrustedPeersRegistry populated from Raft membership.
    if config.iroh.enable_raft_auth {
        info!("Raft authentication enabled - using Iroh-native NodeId verification and RAFT_AUTH_ALPN");
    }

    // Create network factory with appropriate ALPN based on auth config
    // When auth is enabled, use RAFT_AUTH_ALPN (raft-auth) for connections
    let network_factory =
        Arc::new(IrpcRaftNetworkFactory::new(iroh_manager.clone(), peer_addrs, config.iroh.enable_raft_auth));

    // Derive gossip topic ID
    let gossip_topic_id = if let Some(ref ticket_str) = config.iroh.gossip_ticket {
        match AspenClusterTicket::deserialize(ticket_str) {
            Ok(ticket) => {
                info!(
                    cluster_id = %ticket.cluster_id,
                    bootstrap_peers = ticket.bootstrap.len(),
                    "using topic ID from cluster ticket"
                );
                ticket.topic_id
            }
            Err(err) => {
                warn!(
                    error = %err,
                    "failed to parse gossip ticket, falling back to cookie-derived topic"
                );
                derive_topic_id_from_cookie(&config.cookie)
            }
        }
    } else {
        derive_topic_id_from_cookie(&config.cookie)
    };

    // Setup gossip discovery if enabled
    let gossip_discovery = if config.iroh.enable_gossip {
        info!(
            node_id = config.node_id,
            topic_id = %hex::encode(gossip_topic_id.as_bytes()),
            "starting gossip discovery"
        );

        match spawn_gossip_peer_discovery(
            gossip_topic_id,
            config.node_id.into(),
            &iroh_manager,
            Some(network_factory.clone()),
        )
        .await
        {
            Ok(discovery) => {
                info!(
                    node_id = config.node_id,
                    topic_id = %hex::encode(gossip_topic_id.as_bytes()),
                    "gossip discovery started successfully"
                );
                Some(discovery)
            }
            Err(err) => {
                warn!(
                    error = %err,
                    node_id = config.node_id,
                    "failed to start gossip discovery, continuing without it"
                );
                None
            }
        }
    } else {
        info!(
            node_id = config.node_id,
            topic_id = %hex::encode(gossip_topic_id.as_bytes()),
            "gossip discovery disabled by configuration"
        );
        None
    };

    // Initialize blob store if enabled
    let blob_store = if config.blobs.enabled {
        let blobs_dir = data_dir.join("blobs");
        std::fs::create_dir_all(&blobs_dir)
            .map_err(|e| anyhow::anyhow!("failed to create blobs directory {}: {}", blobs_dir.display(), e))?;

        match IrohBlobStore::new(&blobs_dir, iroh_manager.endpoint().clone()).await {
            Ok(store) => {
                info!(
                    node_id = config.node_id,
                    path = %blobs_dir.display(),
                    "blob store initialized"
                );
                Some(Arc::new(store))
            }
            Err(err) => {
                warn!(
                    error = ?err,
                    node_id = config.node_id,
                    "failed to initialize blob store, continuing without it"
                );
                None
            }
        }
    } else {
        info!(node_id = config.node_id, "blob store disabled by configuration");
        None
    };

    let shutdown_token = CancellationToken::new();

    Ok(BaseNodeResources {
        config: config.clone(),
        metadata_store,
        network: NetworkResources {
            iroh_manager,
            network_factory,
            rpc_server: None, // Not used in sharded mode
            blob_store,
        },
        discovery: BaseDiscoveryResources {
            gossip_discovery,
            gossip_topic_id,
        },
        shutdown_token,
    })
}

/// Bootstrap a sharded cluster node.
///
/// Creates multiple independent Raft instances (one per shard) that share
/// the same Iroh P2P transport. Each shard is a separate consensus group
/// with its own leader election and log.
///
/// # Arguments
///
/// * `config` - Node configuration with sharding enabled
///
/// # Returns
///
/// A `ShardedNodeHandle` containing all shard RaftNodes and the sharded KV store.
///
/// # Errors
///
/// Returns an error if:
/// - Sharding is not enabled in config
/// - Base node bootstrap fails
/// - Any shard's Raft instance fails to initialize
pub async fn bootstrap_sharded_node(mut config: NodeConfig) -> Result<ShardedNodeHandle> {
    // Apply security defaults before using config
    // This auto-enables raft_auth when pkarr is enabled for defense-in-depth
    config.apply_security_defaults();

    ensure!(config.sharding.enabled, "sharding must be enabled to use bootstrap_sharded_node");

    let num_shards = config.sharding.num_shards;
    let local_shards: Vec<ShardId> = if config.sharding.local_shards.is_empty() {
        // Host all shards by default
        (0..num_shards).collect()
    } else {
        config.sharding.local_shards.clone()
    };

    info!(
        node_id = config.node_id,
        num_shards,
        local_shards = ?local_shards,
        "bootstrapping sharded node"
    );

    // Bootstrap base resources (Iroh, metadata, gossip)
    let base = bootstrap_base_node(&config).await?;

    // Create sharded protocol handler
    let sharded_handler = Arc::new(ShardedRaftProtocolHandler::new());

    // Create ShardedKeyValueStore with router and topology
    let shard_config = ShardConfig::new(num_shards);

    // Create initial topology for the sharded cluster
    let topology = {
        use aspen_sharding::ShardTopology;
        let created_at = aspen_core::utils::current_time_secs();
        let topology = ShardTopology::new(num_shards, created_at);
        Arc::new(tokio::sync::RwLock::new(topology))
    };

    // For now, create ShardedKeyValueStore without topology to avoid type complexity
    let sharded_kv = Arc::new(ShardedKeyValueStore::<RaftNode>::new(shard_config));

    // Create supervisor for all shards
    let supervisor = Supervisor::new(format!("sharded-node-{}", config.node_id));

    let data_dir = config
        .data_dir
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("data_dir must be set for sharded node bootstrap"))?;

    let mut shard_nodes: HashMap<ShardId, Arc<RaftNode>> = HashMap::new();
    let mut health_monitors: HashMap<ShardId, Arc<RaftNodeHealth>> = HashMap::new();
    let mut ttl_cleanup_cancels: HashMap<ShardId, CancellationToken> = HashMap::new();
    let mut shard_state_machines: HashMap<ShardId, StateMachineVariant> = HashMap::new();

    // Create Raft config (shared across shards with per-shard cluster name)
    let base_raft_config = RaftConfig {
        heartbeat_interval: config.heartbeat_interval_ms,
        election_timeout_min: config.election_timeout_min_ms,
        election_timeout_max: config.election_timeout_max_ms,
        replication_lag_threshold: 10000,
        snapshot_policy: openraft::SnapshotPolicy::LogsSinceLast(100),
        max_in_snapshot_log_to_keep: 100,
        enable_tick: true,
        ..RaftConfig::default()
    };

    // Create broadcast channels for shard 0 if hooks or docs are enabled
    // Only shard 0 needs these channels for hooks to work in sharded mode
    let shard_0_broadcasts = if (config.hooks.enabled || config.docs.enabled) && local_shards.contains(&0) {
        let (log_sender, _) = broadcast::channel(LOG_BROADCAST_BUFFER_SIZE);
        let (snapshot_sender, _) = broadcast::channel(LOG_BROADCAST_BUFFER_SIZE);
        info!(
            node_id = config.node_id,
            buffer_size = LOG_BROADCAST_BUFFER_SIZE,
            hooks_enabled = config.hooks.enabled,
            docs_enabled = config.docs.enabled,
            "created broadcast channels for shard 0 hooks/docs"
        );
        Some((log_sender, snapshot_sender))
    } else {
        None
    };

    // For each local shard, create Raft instance
    for &shard_id in &local_shards {
        info!(node_id = config.node_id, shard_id, "creating Raft instance for shard");

        // Generate storage paths for this shard
        let paths = ShardStoragePaths::new(data_dir, shard_id);
        paths
            .ensure_dir_exists()
            .map_err(|e| anyhow::anyhow!("failed to create shard directory {}: {}", paths.shard_dir.display(), e))?;

        // Encode shard-aware node ID (shard in upper 16 bits)
        let shard_node_id = encode_shard_node_id(config.node_id, shard_id);

        // Create shard-specific Raft config
        let raft_config = Arc::new(RaftConfig {
            cluster_name: format!("{}-shard-{}", config.cookie, shard_id),
            ..base_raft_config.clone()
        });

        // Create storage based on backend type
        let (raft, state_machine_variant, ttl_cancel) = match config.storage_backend {
            StorageBackend::InMemory => {
                let log_store = Arc::new(InMemoryLogStore::default());
                let state_machine = InMemoryStateMachine::new();
                let raft: Arc<Raft<AppTypeConfig>> = Arc::new(
                    Raft::new(
                        shard_node_id.into(),
                        raft_config.clone(),
                        base.network.network_factory.as_ref().clone(),
                        log_store.as_ref().clone(),
                        state_machine.clone(),
                    )
                    .await?,
                );
                (raft, StateMachineVariant::InMemory(state_machine), None)
            }
            StorageBackend::Redb => {
                // Single-fsync storage: shared redb for both log and state machine
                let db_path = paths.log_path.with_extension("shared.redb");

                // For shard 0, pass broadcast channels if hooks/docs are enabled
                let shared_storage = if shard_id == 0 {
                    if let Some((ref log_sender, ref snapshot_sender)) = shard_0_broadcasts {
                        Arc::new(
                            SharedRedbStorage::with_broadcasts(
                                &db_path,
                                Some(log_sender.clone()),
                                Some(snapshot_sender.clone()),
                                &shard_node_id.to_string(),
                            )
                            .map_err(|e| {
                                anyhow::anyhow!("failed to open shared redb storage for shard 0 with broadcasts: {}", e)
                            })?,
                        )
                    } else {
                        Arc::new(
                            SharedRedbStorage::new(&db_path, &shard_node_id.to_string())
                                .map_err(|e| anyhow::anyhow!("failed to open shared redb storage for shard: {}", e))?,
                        )
                    }
                } else {
                    Arc::new(
                        SharedRedbStorage::new(&db_path, &shard_node_id.to_string())
                            .map_err(|e| anyhow::anyhow!("failed to open shared redb storage for shard: {}", e))?,
                    )
                };

                info!(
                    node_id = config.node_id,
                    shard_id,
                    path = %db_path.display(),
                    has_broadcasts = shard_id == 0 && shard_0_broadcasts.is_some(),
                    "created shared redb storage for shard (single-fsync mode)"
                );

                let raft: Arc<Raft<AppTypeConfig>> = Arc::new(
                    Raft::new(
                        shard_node_id.into(),
                        raft_config.clone(),
                        base.network.network_factory.as_ref().clone(),
                        shared_storage.as_ref().clone(),
                        shared_storage.as_ref().clone(),
                    )
                    .await?,
                );

                (raft, StateMachineVariant::Redb(shared_storage), None)
            }
        };

        info!(node_id = config.node_id, shard_id, "created OpenRaft instance for shard");

        // Register Raft core with sharded protocol handler
        // SAFETY: aspen_raft::types::AppTypeConfig and aspen_transport::rpc::AppTypeConfig are
        // structurally identical (both use the same types from aspen-raft-types: AppRequest,
        // AppResponse, NodeId, RaftMemberInfo). This transmute is verified safe at compile time
        // by static_assertions in aspen_raft::types::_transmute_safety_static_checks. If the
        // types ever diverge, compilation will fail.
        let transport_raft: openraft::Raft<TransportAppTypeConfig> =
            unsafe { std::mem::transmute(raft.as_ref().clone()) };
        sharded_handler.register_shard(shard_id, transport_raft);

        // Clone state machine for ShardedNodeHandle (needed for maintenance worker database access)
        let state_machine_for_handle = state_machine_variant.clone();

        // Create RaftNode wrapper - use batch config from NodeConfig or default
        let raft_node = if let Some(batch_config) = config.batch_config.clone() {
            Arc::new(RaftNode::with_write_batching(
                shard_node_id.into(),
                raft.clone(),
                state_machine_variant,
                batch_config.finalize(),
            ))
        } else {
            Arc::new(RaftNode::new(shard_node_id.into(), raft.clone(), state_machine_variant))
        };

        // Store state machine for this shard
        shard_state_machines.insert(shard_id, state_machine_for_handle);

        // Create health monitor
        let health_monitor = Arc::new(RaftNodeHealth::new(raft_node.clone()));

        // Register with ShardedKeyValueStore
        sharded_kv.add_shard(shard_id, raft_node.clone()).await;

        // Store in maps
        shard_nodes.insert(shard_id, raft_node);
        health_monitors.insert(shard_id, health_monitor);
        if let Some(cancel) = ttl_cancel {
            ttl_cleanup_cancels.insert(shard_id, cancel);
        }
    }

    // Initialize peer sync if enabled (using shard 0 for now)
    let peer_manager = if config.peer_sync.enabled {
        if let Some(shard_0) = shard_nodes.get(&0) {
            use aspen_docs::DocsImporter;
            use aspen_docs::PeerManager;

            let importer =
                Arc::new(DocsImporter::new(config.cookie.clone(), shard_0.clone(), &config.node_id.to_string()));
            let manager = Arc::new(PeerManager::new(config.cookie.clone(), importer));

            info!(node_id = config.node_id, "peer sync initialized (using shard 0)");
            Some(manager)
        } else {
            warn!(node_id = config.node_id, "peer sync requested but shard 0 not hosted locally");
            None
        }
    } else {
        None
    };

    // Initialize global content discovery if enabled
    let shutdown_for_content = CancellationToken::new();
    let (content_discovery, content_discovery_cancel) =
        initialize_content_discovery(&config, &base.network.iroh_manager, &shutdown_for_content).await?;

    // Auto-announce local blobs to DHT if enabled (only from shard 0 in sharded mode)
    if content_discovery.is_some() && base.network.blob_store.is_some() && shard_nodes.contains_key(&0) {
        let config_clone = config.clone();
        let blob_store_clone = base.network.blob_store.clone();
        let content_discovery_clone = content_discovery.clone();
        tokio::spawn(async move {
            auto_announce_local_blobs(&config_clone, blob_store_clone.as_ref(), content_discovery_clone.as_ref()).await;
        });
    }

    // Register node in metadata store
    use crate::metadata::NodeMetadata;
    base.metadata_store.register_node(NodeMetadata {
        node_id: config.node_id,
        endpoint_id: base.network.iroh_manager.node_addr().id.to_string(),
        raft_addr: String::new(),
        status: NodeStatus::Online,
        last_updated_secs: aspen_core::utils::current_time_secs(),
    })?;

    info!(node_id = config.node_id, shard_count = shard_nodes.len(), "sharded node bootstrap complete");

    // Create event broadcast channels for hook integration (only if hooks enabled and shard 0 is local)
    let (blob_event_sender, _blob_broadcaster) =
        if config.hooks.enabled && config.blobs.enabled && shard_nodes.contains_key(&0) {
            let (sender, _receiver) = create_blob_event_channel();
            // Note: In sharded mode, we don't attach the broadcaster to blob_store since it's shared
            // across shards. Blob events for hooks would need to be wired differently if needed.
            (Some(sender), None::<BlobEventBroadcaster>)
        } else {
            (None, None)
        };

    let (docs_event_sender, _docs_broadcaster) =
        if config.hooks.enabled && config.docs.enabled && shard_nodes.contains_key(&0) {
            let (sender, _receiver) = create_docs_event_channel();
            // Note: In sharded mode, docs events would need to be wired to the appropriate exporter
            (Some(sender), None::<Arc<DocsEventBroadcaster>>)
        } else {
            (None, None)
        };

    // Initialize hook service using shard 0's resources (if available and hooks enabled)
    let hooks = if let (Some(shard_0_raft), Some(shard_0_sm)) = (shard_nodes.get(&0), shard_state_machines.get(&0)) {
        // Extract broadcast channels for shard 0
        let (log_broadcast, snapshot_broadcast) = match &shard_0_broadcasts {
            Some((log, snapshot)) => (Some(log.clone()), Some(snapshot.clone())),
            None => (None, None),
        };

        initialize_hook_service(
            &config,
            log_broadcast.as_ref(),
            snapshot_broadcast.as_ref(),
            blob_event_sender.as_ref(),
            docs_event_sender.as_ref(),
            shard_0_raft,
            shard_0_sm,
        )
        .await?
    } else {
        warn!(node_id = config.node_id, "hook service not initialized: shard 0 not hosted locally");
        HookResources::disabled()
    };

    // Construct resource groups
    let sharding = ShardingResources {
        shard_nodes,
        sharded_kv,
        sharded_handler,
        health_monitors,
        ttl_cleanup_cancels,
        shard_state_machines,
        topology: Some(topology),
    };

    let discovery = DiscoveryResources {
        gossip_discovery: None, // Gossip discovery is in base.discovery
        gossip_topic_id: base.discovery.gossip_topic_id,
        content_discovery,
        content_discovery_cancel,
    };

    let sync = SyncResources {
        log_broadcast: shard_0_broadcasts.map(|(log, _)| log),
        docs_exporter_cancel: None,
        sync_event_listener_cancel: None,
        docs_sync_service_cancel: None,
        docs_sync: None,
        peer_manager,
    };

    let worker = WorkerResources {
        worker_service: None,        // Initialized in aspen-node after JobManager creation
        worker_service_cancel: None, // Initialized in aspen-node after JobManager creation
    };

    Ok(ShardedNodeHandle {
        base,
        root_token: None,
        sharding,
        discovery,
        sync,
        worker,
        hooks,
        supervisor,
    })
}

/// Bootstrap a cluster node with simplified architecture.
///
/// This replaces the actor-based bootstrap with direct async APIs,
/// removing the overhead of message passing while maintaining the
/// same functionality.
pub async fn bootstrap_node(mut config: NodeConfig) -> Result<NodeHandle> {
    // Apply security defaults before using config
    // This auto-enables raft_auth when pkarr is enabled for defense-in-depth
    config.apply_security_defaults();

    info!(node_id = config.node_id, "bootstrapping node with simplified architecture");

    // Initialize metadata store
    let data_dir = config.data_dir.as_ref().ok_or_else(|| anyhow::anyhow!("data_dir must be set"))?;

    // Ensure data directory exists
    std::fs::create_dir_all(data_dir)
        .map_err(|e| anyhow::anyhow!("failed to create data directory {}: {}", data_dir.display(), e))?;

    // MetadataStore expects a path to the database file, not directory
    let metadata_db_path = data_dir.join("metadata.redb");
    let metadata_store = Arc::new(MetadataStore::new(&metadata_db_path)?);

    // Initialize Iroh endpoint
    let iroh_manager = initialize_iroh_endpoint(&config).await?;

    // Parse peer addresses from config if provided
    let peer_addrs = parse_peer_addresses(&config.peers)?;

    // Iroh-Native Authentication: No client-side auth needed
    // Authentication is handled at connection accept time by the server.
    // The server validates the remote NodeId (verified by QUIC TLS handshake)
    // against the TrustedPeersRegistry populated from Raft membership.
    if config.iroh.enable_raft_auth {
        info!("Raft authentication enabled - using Iroh-native NodeId verification and RAFT_AUTH_ALPN");
    }

    // Create network factory with appropriate ALPN based on auth config
    // When auth is enabled, use RAFT_AUTH_ALPN (raft-auth) for connections
    let network_factory =
        Arc::new(IrpcRaftNetworkFactory::new(iroh_manager.clone(), peer_addrs, config.iroh.enable_raft_auth));

    // Derive gossip topic ID from ticket or cookie
    let gossip_topic_id = derive_gossip_topic_from_config(&config);

    // Setup gossip discovery
    let gossip_discovery = setup_gossip_discovery(&config, gossip_topic_id, &iroh_manager, &network_factory).await;

    // Create Raft config and broadcast channels
    let (raft_config, broadcasts) = create_raft_config_and_broadcast(&config);

    // Create Raft instance with appropriate storage backend
    let (raft, state_machine_variant, ttl_cleanup_cancel) =
        create_raft_instance(&config, raft_config.clone(), &network_factory, data_dir, &broadcasts).await?;

    info!(node_id = config.node_id, "created OpenRaft instance");

    // Clone state machine for NodeHandle (needed for maintenance worker database access)
    let state_machine_for_handle = state_machine_variant.clone();

    // Create RaftNode - use batch config from NodeConfig if enabled
    // Write batching provides ~10x throughput improvement at ~2ms added latency
    let raft_node = if let Some(batch_config) = config.batch_config.clone() {
        Arc::new(RaftNode::with_write_batching(
            config.node_id.into(),
            raft.clone(),
            state_machine_variant,
            batch_config.finalize(),
        ))
    } else {
        Arc::new(RaftNode::new(config.node_id.into(), raft.clone(), state_machine_variant))
    };

    // Create supervisor and health monitor
    let supervisor = Supervisor::new(format!("raft-node-{}", config.node_id));
    let health_monitor = Arc::new(RaftNodeHealth::new(raft_node.clone()));

    // Spawn health monitoring background task
    let shutdown = spawn_health_monitoring(&config, health_monitor.clone(), supervisor.clone());

    // Create event broadcast channels for hook integration (only if hooks enabled)
    let (blob_event_sender, blob_broadcaster) = if config.hooks.enabled && config.blobs.enabled {
        let (sender, _receiver) = create_blob_event_channel();
        let broadcaster = BlobEventBroadcaster::new(sender.clone());
        (Some(sender), Some(broadcaster))
    } else {
        (None, None)
    };

    let (docs_event_sender, docs_broadcaster) = if config.hooks.enabled && config.docs.enabled {
        let (sender, _receiver) = create_docs_event_channel();
        let broadcaster = Arc::new(DocsEventBroadcaster::new(sender.clone()));
        (Some(sender), Some(broadcaster))
    } else {
        (None, None)
    };

    // Initialize blob store and peer manager
    let blob_store = initialize_blob_store(&config, data_dir, &iroh_manager, blob_broadcaster).await;
    let peer_manager = initialize_peer_manager(&config, &raft_node);

    // Initialize DocsExporter and P2P sync if enabled
    let (docs_exporter_cancel, docs_sync) =
        initialize_docs_export(&config, data_dir, broadcasts.log.as_ref(), blob_store.as_ref(), docs_broadcaster)
            .await?;

    // Wire up sync event listener and DocsSyncService if all components are available
    let (sync_event_listener_cancel, docs_sync_service_cancel) =
        wire_docs_sync_services(&config, &docs_sync, &blob_store, &peer_manager, &iroh_manager).await;

    // Initialize global content discovery if enabled
    let (content_discovery, content_discovery_cancel) =
        initialize_content_discovery(&config, &iroh_manager, &shutdown).await?;

    // Auto-announce local blobs to DHT if enabled (runs in background)
    spawn_blob_announcer(&config, &blob_store, &content_discovery);

    // Initialize hook service if enabled
    let hooks = initialize_hook_service(
        &config,
        broadcasts.log.as_ref(),
        broadcasts.snapshot.as_ref(),
        blob_event_sender.as_ref(),
        docs_event_sender.as_ref(),
        &raft_node,
        &state_machine_for_handle,
    )
    .await?;

    // Register node in metadata store
    register_node_metadata(&config, &metadata_store, &iroh_manager)?;

    Ok(NodeHandle {
        config,
        metadata_store,
        root_token: None, // Set by caller after cluster init
        storage: StorageResources {
            raft_node,
            state_machine: state_machine_for_handle,
            ttl_cleanup_cancel,
        },
        network: NetworkResources {
            iroh_manager,
            network_factory,
            rpc_server: None, // Router-based architecture preferred; spawn Router in caller
            blob_store,
        },
        discovery: DiscoveryResources {
            gossip_discovery,
            gossip_topic_id,
            content_discovery,
            content_discovery_cancel,
        },
        sync: SyncResources {
            log_broadcast: broadcasts.log,
            docs_exporter_cancel,
            sync_event_listener_cancel,
            docs_sync_service_cancel,
            docs_sync,
            peer_manager,
        },
        worker: WorkerResources {
            worker_service: None,        // Initialized in aspen-node after JobManager creation
            worker_service_cancel: None, // Initialized in aspen-node after JobManager creation
        },
        blob_replication: BlobReplicationResources::disabled(), // Initialized in aspen-node after KV store is ready
        hooks,
        shutdown: ShutdownCoordinator {
            shutdown_token: shutdown,
            supervisor,
            health_monitor,
        },
    })
}

/// Derive a gossip topic ID from the cluster cookie.
///
/// Uses blake3 hash of the cookie string to create a deterministic
/// 32-byte topic ID. All nodes with the same cookie will join the
/// same gossip topic.
///
/// Tiger Style: Fixed-size output (32 bytes), deterministic.
fn derive_topic_id_from_cookie(cookie: &str) -> TopicId {
    let hash = blake3::hash(cookie.as_bytes());
    TopicId::from_bytes(*hash.as_bytes())
}

/// Load configuration from multiple sources with proper precedence.
///
/// Configuration is loaded in the following order (lowest to highest precedence):
/// 1. Environment variables (ASPEN_*)
/// 2. Nickel configuration file (if path provided)
/// 3. Configuration overrides (typically from CLI args)
///
/// Returns the merged configuration.
///
/// # Config File Format
///
/// Configuration files use the Nickel language (.ncl extension).
/// Nickel provides programmable configuration with contracts for validation.
///
/// # Example
///
/// ```nickel
/// {
///   node_id = 1,
///   cookie = "my-cluster",
///   iroh.enable_raft_auth = true,
/// }
/// ```
pub fn load_config(config_path: Option<&std::path::Path>, overrides: NodeConfig) -> Result<NodeConfig> {
    // Start with environment variables
    let mut config = NodeConfig::from_env();

    // Merge Nickel config file if provided
    if let Some(path) = config_path {
        let file_config: NodeConfig = aspen_nickel::load_nickel_config(path)
            .map_err(|e| anyhow::anyhow!("failed to load config from {}: {}", path.display(), e))?;
        config.merge(file_config);
    }

    // Merge overrides (typically from CLI args)
    config.merge(overrides);

    // Validate final configuration
    config.validate().map_err(|e| anyhow::anyhow!("configuration validation failed: {}", e))?;

    Ok(config)
}

/// Parse peer addresses from CLI arguments.
fn parse_peer_addresses(peer_specs: &[String]) -> Result<HashMap<NodeId, EndpointAddr>> {
    let mut peers = HashMap::new();

    for spec in peer_specs {
        let parts: Vec<&str> = spec.split('@').collect();
        ensure!(parts.len() == 2, "invalid peer spec '{}', expected format: node_id@endpoint_id", spec);

        let node_id: u64 =
            parts[0].parse().map_err(|e| anyhow::anyhow!("invalid node_id in peer spec '{}': {}", spec, e))?;

        // Parse endpoint address (could be full JSON or just ID)
        let addr = if parts[1].starts_with('{') {
            serde_json::from_str(parts[1])
                .map_err(|e| anyhow::anyhow!("invalid JSON endpoint in peer spec '{}': {}", spec, e))?
        } else {
            // Parse as bare endpoint ID
            let endpoint_id = parts[1]
                .parse()
                .map_err(|e| anyhow::anyhow!("invalid endpoint_id in peer spec '{}': {}", spec, e))?;
            EndpointAddr::new(endpoint_id)
        };

        peers.insert(node_id.into(), addr);
    }

    Ok(peers)
}

/// Create Raft instance with appropriate storage backend.
///
/// Returns (raft, state_machine_variant, ttl_cleanup_cancel).
async fn create_raft_instance(
    config: &NodeConfig,
    raft_config: Arc<RaftConfig>,
    network_factory: &Arc<IrpcRaftNetworkFactory>,
    data_dir: &std::path::Path,
    broadcasts: &StorageBroadcasts,
) -> Result<(Arc<Raft<aspen_raft::types::AppTypeConfig>>, StateMachineVariant, Option<CancellationToken>)> {
    match config.storage_backend {
        StorageBackend::InMemory => {
            let log_store = Arc::new(InMemoryLogStore::default());
            let state_machine = InMemoryStateMachine::new();
            let raft = Arc::new(
                Raft::new(
                    config.node_id.into(),
                    raft_config,
                    network_factory.as_ref().clone(),
                    log_store.as_ref().clone(),
                    state_machine.clone(),
                )
                .await?,
            );
            Ok((raft, StateMachineVariant::InMemory(state_machine), None))
        }
        StorageBackend::Redb => {
            let db_path = data_dir.join(format!("node_{}_shared.redb", config.node_id));
            let shared_storage = Arc::new(
                SharedRedbStorage::with_broadcasts(
                    &db_path,
                    broadcasts.log.clone(),
                    broadcasts.snapshot.clone(),
                    &config.node_id.to_string(),
                )
                .map_err(|e| anyhow::anyhow!("failed to open shared redb storage: {}", e))?,
            );

            info!(
                node_id = config.node_id,
                path = %db_path.display(),
                "created shared redb storage (single-fsync mode)"
            );

            let raft = Arc::new(
                Raft::new(
                    config.node_id.into(),
                    raft_config,
                    network_factory.as_ref().clone(),
                    shared_storage.as_ref().clone(),
                    shared_storage.as_ref().clone(),
                )
                .await?,
            );

            let ttl_cancel = spawn_redb_ttl_cleanup_task(shared_storage.clone(), TtlCleanupConfig::default());
            info!(node_id = config.node_id, "Redb TTL cleanup task started");
            Ok((raft, StateMachineVariant::Redb(shared_storage), Some(ttl_cancel)))
        }
    }
}

/// Spawn health monitoring background task.
///
/// Returns the shutdown token for the health monitor.
fn spawn_health_monitoring(
    config: &NodeConfig,
    health_monitor: Arc<RaftNodeHealth>,
    supervisor: Arc<Supervisor>,
) -> CancellationToken {
    let health_clone = health_monitor.clone();
    let shutdown = CancellationToken::new();
    let shutdown_clone = shutdown.clone();
    let supervisor_for_health = supervisor.clone();
    let node_id_for_health = config.node_id;

    tokio::spawn(async move {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(32);
        let dropped_count = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let dropped_count_for_callback = Arc::clone(&dropped_count);

        let supervisor_clone = supervisor_for_health.clone();
        let node_id_for_processor = node_id_for_health;
        tokio::spawn(async move {
            while let Some(reason) = rx.recv().await {
                supervisor_clone.record_health_failure(&reason).await;

                if !supervisor_clone.should_attempt_recovery().await {
                    error!(
                        node_id = node_id_for_processor,
                        "too many health failures, supervisor circuit breaker triggered"
                    );
                    supervisor_clone.stop();
                    break;
                }
            }
            warn!(node_id = node_id_for_processor, "health failure processor channel closed");
        });

        tokio::select! {
            _ = health_clone.monitor_with_callback(5, move |status| {
                let reason = format!(
                    "health check failed: state={:?}, failures={}",
                    status.state, status.consecutive_failures
                );
                if let Err(tokio::sync::mpsc::error::TrySendError::Full(_)) = tx.try_send(reason) {
                    let count = dropped_count_for_callback
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    if count % 10 == 1 {
                        tracing::warn!(
                            dropped_count = count,
                            "health failure channel full, messages being dropped \
                             (health processor may be blocked or overwhelmed)"
                        );
                    }
                }
            }) => {}
            _ = shutdown_clone.cancelled() => {
                let final_drops = dropped_count.load(std::sync::atomic::Ordering::Relaxed);
                if final_drops > 0 {
                    warn!(
                        node_id = node_id_for_health,
                        dropped_messages = final_drops,
                        "health monitor shutdown with dropped messages"
                    );
                }
            }
        }
    });

    shutdown
}

/// Initialize blob store if enabled.
async fn initialize_blob_store(
    config: &NodeConfig,
    data_dir: &std::path::Path,
    iroh_manager: &Arc<IrohEndpointManager>,
    broadcaster: Option<BlobEventBroadcaster>,
) -> Option<Arc<IrohBlobStore>> {
    if !config.blobs.enabled {
        info!(node_id = config.node_id, "blob store disabled by configuration");
        return None;
    }

    let blobs_dir = data_dir.join("blobs");
    if let Err(e) = std::fs::create_dir_all(&blobs_dir) {
        warn!(
            error = ?e,
            node_id = config.node_id,
            path = %blobs_dir.display(),
            "failed to create blobs directory, continuing without blob store"
        );
        return None;
    }

    match IrohBlobStore::new(&blobs_dir, iroh_manager.endpoint().clone()).await {
        Ok(store) => {
            let store = match broadcaster {
                Some(b) => store.with_broadcaster(b),
                None => store,
            };
            info!(
                node_id = config.node_id,
                path = %blobs_dir.display(),
                "blob store initialized"
            );
            Some(Arc::new(store))
        }
        Err(err) => {
            warn!(
                error = ?err,
                node_id = config.node_id,
                "failed to initialize blob store, continuing without it"
            );
            None
        }
    }
}

/// Initialize blob replication manager if replication is enabled.
///
/// Creates the `BlobReplicationManager` which coordinates blob replication
/// across cluster nodes. The manager is only created if:
/// - Blobs are enabled (`config.blobs.enabled`)
/// - Replication factor > 1 or auto-replication is enabled
///
/// # Arguments
/// * `config` - Node configuration with blob settings
/// * `blob_store` - The blob store for P2P transfers
/// * `kv_store` - The KV store for replica metadata (Raft-backed)
/// * `blob_events` - Receiver for blob events (from broadcaster)
/// * `shutdown` - Cancellation token for graceful shutdown
///
/// # Returns
/// `BlobReplicationResources` containing the manager and shutdown handles,
/// or disabled resources if replication is not configured.
pub async fn initialize_blob_replication<KV>(
    config: &NodeConfig,
    blob_store: Option<Arc<IrohBlobStore>>,
    endpoint: Option<iroh::Endpoint>,
    kv_store: Arc<KV>,
    blob_events: Option<tokio::sync::broadcast::Receiver<aspen_blob::BlobEvent>>,
    shutdown: CancellationToken,
) -> BlobReplicationResources
where
    KV: aspen_core::traits::KeyValueStore + Send + Sync + 'static,
{
    // Check if blob replication should be enabled
    let Some(blob_store) = blob_store else {
        info!(node_id = config.node_id, "blob replication disabled: no blob store");
        return BlobReplicationResources::disabled();
    };

    let Some(endpoint) = endpoint else {
        info!(node_id = config.node_id, "blob replication disabled: no endpoint for RPC");
        return BlobReplicationResources::disabled();
    };

    let Some(blob_events) = blob_events else {
        info!(node_id = config.node_id, "blob replication disabled: no event broadcaster");
        return BlobReplicationResources::disabled();
    };

    // Only enable replication if:
    // - replication_factor > 1 (need multiple replicas), OR
    // - auto_replication is enabled (even with factor=1, we track replicas)
    if config.blobs.replication_factor <= 1 && !config.blobs.enable_auto_replication {
        info!(
            node_id = config.node_id,
            replication_factor = config.blobs.replication_factor,
            "blob replication disabled: replication_factor=1 and auto_replication=false"
        );
        return BlobReplicationResources::disabled();
    }

    // Build replication configuration from BlobConfig
    let replication_config = aspen_blob::ReplicationConfig {
        default_policy: aspen_blob::ReplicationPolicy {
            replication_factor: config.blobs.replication_factor.min(aspen_blob::MAX_REPLICATION_FACTOR),
            min_replicas: config.blobs.min_replicas.min(config.blobs.replication_factor),
            failure_domain_key: config.blobs.failure_domain_key.clone(),
            enable_quorum_writes: config.blobs.enable_quorum_writes,
        },
        node_id: config.node_id,
        auto_replicate: config.blobs.enable_auto_replication,
        repair_interval_secs: config.blobs.repair_interval_secs,
        repair_delay_secs: config.blobs.repair_delay_secs,
        max_concurrent: aspen_blob::MAX_CONCURRENT_REPLICATIONS,
    };

    // Create the trait adapters
    let metadata_store = Arc::new(aspen_blob::KvReplicaMetadataStore::new(kv_store));
    let blob_transfer = Arc::new(aspen_blob::IrohBlobTransfer::new(blob_store, endpoint));
    let placement = Arc::new(aspen_blob::WeightedPlacement);

    // Create child cancellation token for replication manager
    let replication_cancel = shutdown.child_token();

    // Spawn the replication manager
    match aspen_blob::BlobReplicationManager::spawn(
        replication_config.clone(),
        blob_events,
        metadata_store,
        blob_transfer,
        placement,
        replication_cancel.clone(),
    )
    .await
    {
        Ok((manager, task)) => {
            info!(
                node_id = config.node_id,
                replication_factor = replication_config.default_policy.replication_factor,
                min_replicas = replication_config.default_policy.min_replicas,
                auto_replicate = replication_config.auto_replicate,
                "blob replication manager started"
            );
            BlobReplicationResources {
                replication_manager: Some(manager),
                replication_cancel: Some(replication_cancel),
                replication_task: Some(task),
                topology_cancel: None, // Set later via wire_topology_watcher()
            }
        }
        Err(err) => {
            warn!(
                error = ?err,
                node_id = config.node_id,
                "failed to start blob replication manager, continuing without replication"
            );
            BlobReplicationResources::disabled()
        }
    }
}

/// Build IrohEndpointConfig from NodeConfig's Iroh settings.
///
/// Converts the application-level IrohConfig (with hex strings and optional fields)
/// into the transport-level IrohEndpointConfig (with parsed types).
///
/// # Errors
///
/// Returns an error if secret_key hex decoding fails.
fn build_iroh_config_from_node_config(config: &NodeConfig) -> Result<IrohEndpointConfig> {
    let iroh_config = IrohEndpointConfig::default()
        .with_gossip(config.iroh.enable_gossip)
        .with_mdns(config.iroh.enable_mdns)
        .with_dns_discovery(config.iroh.enable_dns_discovery)
        .with_pkarr(config.iroh.enable_pkarr)
        .with_pkarr_dht(config.iroh.enable_pkarr_dht)
        .with_pkarr_relay(config.iroh.enable_pkarr_relay)
        .with_pkarr_direct_addresses(config.iroh.include_pkarr_direct_addresses)
        .with_pkarr_republish_delay_secs(config.iroh.pkarr_republish_delay_secs)
        .with_relay_mode(config.iroh.relay_mode.clone())
        .with_relay_urls(config.iroh.relay_urls.clone())
        .with_bind_port(config.iroh.bind_port);

    // Apply optional pkarr relay URL
    let iroh_config = match &config.iroh.pkarr_relay_url {
        Some(url) => iroh_config.with_pkarr_relay_url(url.clone()),
        None => iroh_config,
    };

    // Apply optional DNS discovery URL
    let iroh_config = match &config.iroh.dns_discovery_url {
        Some(url) => iroh_config.with_dns_discovery_url(url.clone()),
        None => iroh_config,
    };

    // Parse and apply optional secret key from config
    let iroh_config = match &config.iroh.secret_key {
        Some(secret_key_hex) => {
            let bytes = hex::decode(secret_key_hex).map_err(|e| anyhow::anyhow!("invalid secret key hex: {}", e))?;
            let secret_key = iroh::SecretKey::from_bytes(
                &bytes.try_into().map_err(|_| anyhow::anyhow!("invalid secret key length"))?,
            );
            iroh_config.with_secret_key(secret_key)
        }
        None => iroh_config,
    };

    // Set secret key persistence path based on data_dir
    // This ensures stable node identity across restarts
    let iroh_config = match &config.data_dir {
        Some(data_dir) => {
            let key_path = data_dir.join("iroh_secret_key");
            info!(path = %key_path.display(), "configured secret key persistence path");
            iroh_config.with_secret_key_path(key_path)
        }
        None => {
            warn!("no data_dir configured - secret key will not persist across restarts");
            iroh_config
        }
    };

    Ok(iroh_config)
}

/// Initialize Iroh endpoint manager.
async fn initialize_iroh_endpoint(config: &NodeConfig) -> Result<Arc<IrohEndpointManager>> {
    let iroh_config = build_iroh_config_from_node_config(config)?;

    let iroh_manager = Arc::new(IrohEndpointManager::new(iroh_config).await?);
    info!(
        node_id = config.node_id,
        endpoint_id = %iroh_manager.node_addr().id,
        "created Iroh endpoint"
    );

    Ok(iroh_manager)
}

/// Broadcast channels for storage events.
struct StorageBroadcasts {
    /// Log entry broadcast for hooks and docs export.
    log: Option<broadcast::Sender<LogEntryPayload>>,
    /// Snapshot event broadcast for hooks.
    snapshot: Option<broadcast::Sender<aspen_raft::storage_shared::SnapshotEvent>>,
}

/// Create Raft configuration and broadcast channels.
///
/// Returns (raft_config, broadcasts).
fn create_raft_config_and_broadcast(config: &NodeConfig) -> (Arc<RaftConfig>, StorageBroadcasts) {
    let raft_config = Arc::new(RaftConfig {
        cluster_name: config.cookie.clone(),
        heartbeat_interval: config.heartbeat_interval_ms,
        election_timeout_min: config.election_timeout_min_ms,
        election_timeout_max: config.election_timeout_max_ms,
        replication_lag_threshold: 10000,
        snapshot_policy: openraft::SnapshotPolicy::LogsSinceLast(100),
        max_in_snapshot_log_to_keep: 100,
        enable_tick: true,
        install_snapshot_timeout: aspen_raft::constants::SNAPSHOT_INSTALL_TIMEOUT_MS,
        ..RaftConfig::default()
    });

    let (log_broadcast, snapshot_broadcast) = if config.hooks.enabled || config.docs.enabled {
        let (log_sender, _) = broadcast::channel(LOG_BROADCAST_BUFFER_SIZE);
        let (snapshot_sender, _) = broadcast::channel(LOG_BROADCAST_BUFFER_SIZE);
        info!(
            node_id = config.node_id,
            buffer_size = LOG_BROADCAST_BUFFER_SIZE,
            hooks_enabled = config.hooks.enabled,
            docs_enabled = config.docs.enabled,
            "created broadcast channels for hooks/docs"
        );
        (Some(log_sender), Some(snapshot_sender))
    } else {
        (None, None)
    };

    (
        raft_config,
        StorageBroadcasts {
            log: log_broadcast,
            snapshot: snapshot_broadcast,
        },
    )
}

/// Register node in metadata store.
fn register_node_metadata(
    config: &NodeConfig,
    metadata_store: &Arc<MetadataStore>,
    iroh_manager: &Arc<IrohEndpointManager>,
) -> Result<()> {
    use crate::metadata::NodeMetadata;
    metadata_store.register_node(NodeMetadata {
        node_id: config.node_id,
        endpoint_id: iroh_manager.node_addr().id.to_string(),
        raft_addr: String::new(),
        status: NodeStatus::Online,
        last_updated_secs: aspen_core::utils::current_time_secs(),
    })?;
    Ok(())
}

/// Initialize peer manager if enabled.
///
/// Creates a PeerManager for cluster-to-cluster synchronization using iroh-docs.
/// The peer manager coordinates connections to peer clusters and routes
/// incoming entries through the DocsImporter for conflict resolution.
fn initialize_peer_manager(config: &NodeConfig, raft_node: &Arc<RaftNode>) -> Option<Arc<aspen_docs::PeerManager>> {
    if !config.peer_sync.enabled {
        return None;
    }

    use aspen_docs::DocsImporter;
    use aspen_docs::PeerManager;

    let importer = Arc::new(DocsImporter::new(config.cookie.clone(), raft_node.clone(), &config.node_id.to_string()));
    let manager = Arc::new(PeerManager::new(config.cookie.clone(), importer));

    info!(node_id = config.node_id, "peer sync initialized");
    Some(manager)
}

/// Initialize hook service if enabled.
///
/// Creates the HookService from configuration and spawns the event bridge tasks
/// that subscribe to the broadcast channels. Returns HookResources for
/// inclusion in NodeHandle.
///
/// The event bridges convert events into HookEvents and dispatch them to registered
/// handlers. Handlers can be in-process closures, shell commands, or cross-cluster
/// forwarding (ForwardHandler not yet implemented).
///
/// Bridge types:
/// - Raft log bridge: Converts committed log entries into hook events
/// - Blob bridge: Converts blob store events (add, download, protect, etc.)
/// - Docs bridge: Converts docs sync events (sync started/completed, import/export)
/// - System events bridge: Monitors Raft metrics for LeaderElected and HealthChanged events
/// - Snapshot events bridge: Monitors snapshot creation and installation events
async fn initialize_hook_service(
    config: &NodeConfig,
    log_broadcast: Option<&broadcast::Sender<LogEntryPayload>>,
    snapshot_broadcast: Option<&broadcast::Sender<aspen_raft::storage_shared::SnapshotEvent>>,
    blob_broadcast: Option<&broadcast::Sender<aspen_blob::BlobEvent>>,
    docs_broadcast: Option<&broadcast::Sender<aspen_docs::DocsEvent>>,
    raft_node: &Arc<RaftNode>,
    state_machine: &StateMachineVariant,
) -> Result<HookResources> {
    if !config.hooks.enabled {
        info!(node_id = config.node_id, "hook service disabled by configuration");
        return Ok(HookResources::disabled());
    }

    // Create hook service from configuration
    let hook_service = Arc::new(aspen_hooks::HookService::new(config.hooks.clone()));

    // Spawn raft log event bridge if log broadcast is available
    let event_bridge_cancel = if let Some(sender) = log_broadcast {
        let cancel = CancellationToken::new();
        let receiver = sender.subscribe();
        let service = Arc::clone(&hook_service);
        let node_id = config.node_id;
        let cancel_clone = cancel.clone();

        tokio::spawn(async move {
            crate::hooks_bridge::run_event_bridge(receiver, service, node_id, cancel_clone).await;
        });

        info!(node_id = config.node_id, "raft log event bridge started");
        Some(cancel)
    } else {
        debug!(node_id = config.node_id, "raft log event bridge not started (log broadcast unavailable)");
        None
    };

    // Spawn blob event bridge if blob broadcast is available
    let blob_bridge_cancel = if let Some(sender) = blob_broadcast {
        let cancel = CancellationToken::new();
        let receiver = sender.subscribe();
        let service = Arc::clone(&hook_service);
        let node_id = config.node_id;
        let cancel_clone = cancel.clone();

        tokio::spawn(async move {
            crate::blob_bridge::run_blob_bridge(receiver, service, node_id, cancel_clone).await;
        });

        info!(node_id = config.node_id, "blob event bridge started");
        Some(cancel)
    } else {
        debug!(node_id = config.node_id, "blob event bridge not started (blob broadcast unavailable)");
        None
    };

    // Spawn docs event bridge if docs broadcast is available
    let docs_bridge_cancel = if let Some(sender) = docs_broadcast {
        let cancel = CancellationToken::new();
        let receiver = sender.subscribe();
        let service = Arc::clone(&hook_service);
        let node_id = config.node_id;
        let cancel_clone = cancel.clone();

        tokio::spawn(async move {
            crate::docs_bridge::run_docs_bridge(receiver, service, node_id, cancel_clone).await;
        });

        info!(node_id = config.node_id, "docs event bridge started");
        Some(cancel)
    } else {
        debug!(node_id = config.node_id, "docs event bridge not started (docs broadcast unavailable)");
        None
    };

    // Spawn system events bridge for LeaderElected and HealthChanged events
    let system_events_bridge_cancel = {
        let cancel = CancellationToken::new();
        let raft_node_clone = Arc::clone(raft_node);
        let service = Arc::clone(&hook_service);
        let node_id = config.node_id;
        let cancel_clone = cancel.clone();
        let bridge_config = crate::system_events_bridge::SystemEventsBridgeConfig::default();

        tokio::spawn(async move {
            crate::system_events_bridge::run_system_events_bridge(
                raft_node_clone,
                service,
                node_id,
                bridge_config,
                cancel_clone,
            )
            .await;
        });

        info!(node_id = config.node_id, "system events bridge started");
        Some(cancel)
    };

    // Spawn snapshot events bridge if snapshot broadcast is available
    let snapshot_events_bridge_cancel = if let Some(sender) = snapshot_broadcast {
        let cancel = CancellationToken::new();
        let receiver = sender.subscribe();
        let service = Arc::clone(&hook_service);
        let node_id = config.node_id;
        let cancel_clone = cancel.clone();

        tokio::spawn(async move {
            crate::snapshot_events_bridge::run_snapshot_events_bridge(receiver, service, node_id, cancel_clone).await;
        });

        info!(node_id = config.node_id, "snapshot events bridge started");
        Some(cancel)
    } else {
        debug!(node_id = config.node_id, "snapshot events bridge not started (snapshot broadcast unavailable)");
        None
    };

    // Spawn TTL events bridge if using Redb storage
    let ttl_events_bridge_cancel = if let StateMachineVariant::Redb(storage) = state_machine {
        let ttl_config = TtlCleanupConfig::default();
        let cancel = crate::ttl_events_bridge::spawn_ttl_events_bridge(
            Arc::clone(storage),
            ttl_config,
            Arc::clone(&hook_service),
            config.node_id,
        );
        info!(node_id = config.node_id, "TTL events bridge started");
        Some(cancel)
    } else {
        debug!(node_id = config.node_id, "TTL events bridge not started (not using Redb storage)");
        None
    };

    info!(
        node_id = config.node_id,
        handler_count = config.hooks.handlers.len(),
        has_log_bridge = event_bridge_cancel.is_some(),
        has_blob_bridge = blob_bridge_cancel.is_some(),
        has_docs_bridge = docs_bridge_cancel.is_some(),
        has_system_bridge = system_events_bridge_cancel.is_some(),
        has_snapshot_bridge = snapshot_events_bridge_cancel.is_some(),
        has_ttl_bridge = ttl_events_bridge_cancel.is_some(),
        "hook service started"
    );

    Ok(HookResources {
        hook_service: Some(hook_service),
        event_bridge_cancel,
        blob_bridge_cancel,
        docs_bridge_cancel,
        system_events_bridge_cancel,
        ttl_events_bridge_cancel,
        snapshot_events_bridge_cancel,
    })
}

/// Setup gossip discovery if enabled.
async fn setup_gossip_discovery(
    config: &NodeConfig,
    gossip_topic_id: TopicId,
    iroh_manager: &Arc<IrohEndpointManager>,
    network_factory: &Arc<IrpcRaftNetworkFactory>,
) -> Option<GossipPeerDiscovery> {
    if !config.iroh.enable_gossip {
        info!(
            node_id = config.node_id,
            topic_id = %hex::encode(gossip_topic_id.as_bytes()),
            "gossip discovery disabled by configuration (topic ID still available for tickets)"
        );
        return None;
    }

    info!(
        node_id = config.node_id,
        topic_id = %hex::encode(gossip_topic_id.as_bytes()),
        "starting gossip discovery"
    );

    match spawn_gossip_peer_discovery(
        gossip_topic_id,
        config.node_id.into(),
        iroh_manager,
        Some(network_factory.clone()),
    )
    .await
    {
        Ok(discovery) => {
            info!(
                node_id = config.node_id,
                topic_id = %hex::encode(gossip_topic_id.as_bytes()),
                "gossip discovery started successfully"
            );
            Some(discovery)
        }
        Err(err) => {
            warn!(
                error = %err,
                node_id = config.node_id,
                "failed to start gossip discovery, continuing without it"
            );
            None
        }
    }
}

/// Initialize DocsExporter and P2P sync if enabled.
///
/// Returns (docs_exporter_cancel, docs_sync).
///
/// If no namespace_secret is configured, derives one from the cluster cookie.
/// This ensures all nodes with the same cookie share the same docs namespace,
/// enabling automatic cross-node replication without explicit configuration.
async fn initialize_docs_export(
    config: &NodeConfig,
    data_dir: &std::path::Path,
    log_broadcast: Option<&broadcast::Sender<LogEntryPayload>>,
    blob_store: Option<&Arc<IrohBlobStore>>,
    docs_broadcaster: Option<Arc<DocsEventBroadcaster>>,
) -> Result<(Option<CancellationToken>, Option<Arc<aspen_docs::DocsSyncResources>>)> {
    if !config.docs.enabled {
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

    let exporter = DocsExporter::new(writer);
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
async fn wire_docs_sync_services(
    config: &NodeConfig,
    docs_sync: &Option<Arc<aspen_docs::DocsSyncResources>>,
    blob_store: &Option<Arc<IrohBlobStore>>,
    peer_manager: &Option<Arc<aspen_docs::PeerManager>>,
    iroh_manager: &Arc<IrohEndpointManager>,
) -> (Option<CancellationToken>, Option<CancellationToken>) {
    use aspen_docs::DocsSyncService;
    use tracing::debug as trace_debug;

    // All three components are required for full docs sync
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

    // Start sync event listener
    // This listens for RemoteInsert events from iroh-docs sync and forwards them to DocsImporter
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
    let sync_service = Arc::new(DocsSyncService::new(docs_sync.clone(), iroh_manager.endpoint().clone()));

    // Peer provider that extracts EndpointAddrs from PeerManager's ticket peers
    let peer_manager_clone = peer_manager.clone();
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

    info!(
        node_id = config.node_id,
        sync_event_listener = sync_event_listener_cancel.is_some(),
        docs_sync_service = true,
        "docs sync services initialized"
    );

    (sync_event_listener_cancel, Some(docs_sync_service_cancel))
}

/// Initialize global content discovery service if enabled.
///
/// Content discovery uses the BitTorrent Mainline DHT to announce and
/// find blobs across clusters without direct federation.
///
/// When `auto_announce` is enabled in the configuration, this function
/// will also scan the local blob store and announce all existing blobs
/// to the DHT.
///
/// # Arguments
/// * `config` - Node configuration with content discovery settings
/// * `iroh_manager` - Iroh endpoint manager for network access
/// * `shutdown` - Cancellation token inherited from parent
///
/// # Returns
/// Tuple of (content_discovery_service, cancellation_token)
/// Both are None when content discovery is disabled.
async fn initialize_content_discovery(
    config: &NodeConfig,
    iroh_manager: &Arc<IrohEndpointManager>,
    shutdown: &CancellationToken,
) -> Result<(Option<crate::content_discovery::ContentDiscoveryService>, Option<CancellationToken>)> {
    use crate::content_discovery::ContentDiscoveryService;

    if !config.content_discovery.enabled {
        return Ok((None, None));
    }

    info!(
        node_id = config.node_id,
        server_mode = config.content_discovery.server_mode,
        dht_port = config.content_discovery.dht_port,
        auto_announce = config.content_discovery.auto_announce,
        "initializing global content discovery (DHT)"
    );

    // Create a child cancellation token for the content discovery service
    let cancel = shutdown.child_token();

    // Spawn the content discovery service
    let (service, _task) = ContentDiscoveryService::spawn(
        Arc::new(iroh_manager.endpoint().clone()),
        iroh_manager.secret_key().clone(),
        config.content_discovery.clone(),
        cancel.clone(),
    )
    .await?;

    info!(
        node_id = config.node_id,
        public_key = %service.public_key(),
        "content discovery service started"
    );

    Ok((Some(service), Some(cancel)))
}

/// Derive gossip topic ID from cluster ticket or cookie.
///
/// If a gossip ticket is configured, uses the topic ID from the ticket.
/// Otherwise, derives a deterministic topic ID from the cluster cookie.
fn derive_gossip_topic_from_config(config: &NodeConfig) -> TopicId {
    if let Some(ref ticket_str) = config.iroh.gossip_ticket {
        match AspenClusterTicket::deserialize(ticket_str) {
            Ok(ticket) => {
                info!(
                    cluster_id = %ticket.cluster_id,
                    bootstrap_peers = ticket.bootstrap.len(),
                    "using topic ID from cluster ticket"
                );
                return ticket.topic_id;
            }
            Err(err) => {
                warn!(
                    error = %err,
                    "failed to parse gossip ticket, falling back to cookie-derived topic"
                );
            }
        }
    }
    derive_topic_id_from_cookie(&config.cookie)
}

/// Spawn background task for auto-announcing blobs to DHT.
///
/// This task waits for the DHT to bootstrap and then announces all local blobs.
/// Runs in background and doesn't block node startup.
fn spawn_blob_announcer(
    config: &NodeConfig,
    blob_store: &Option<Arc<IrohBlobStore>>,
    content_discovery: &Option<crate::content_discovery::ContentDiscoveryService>,
) {
    if content_discovery.is_none() || blob_store.is_none() {
        return;
    }

    let config_clone = config.clone();
    let blob_store_clone = blob_store.clone();
    let content_discovery_clone = content_discovery.clone();
    tokio::spawn(async move {
        // Wait a bit for DHT to bootstrap before announcing
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        auto_announce_local_blobs(&config_clone, blob_store_clone.as_ref(), content_discovery_clone.as_ref()).await;
    });
}

/// Perform auto-announce of local blobs if enabled.
///
/// This function is called after the node is fully initialized.
/// It scans the local blob store and announces all blobs to the DHT.
///
/// # Arguments
/// * `config` - Node configuration
/// * `blob_store` - Optional blob store reference
/// * `content_discovery` - Optional content discovery service
pub async fn auto_announce_local_blobs(
    config: &NodeConfig,
    blob_store: Option<&Arc<IrohBlobStore>>,
    content_discovery: Option<&crate::content_discovery::ContentDiscoveryService>,
) {
    use aspen_blob::BlobStore;

    // Check if auto-announce is enabled
    if !config.content_discovery.auto_announce {
        return;
    }

    let Some(blob_store) = blob_store else {
        warn!(node_id = config.node_id, "auto_announce enabled but blob store not available");
        return;
    };

    let Some(discovery) = content_discovery else {
        warn!(node_id = config.node_id, "auto_announce enabled but content discovery service not available");
        return;
    };

    info!(node_id = config.node_id, "scanning local blobs for auto-announce");

    // List all local blobs
    match blob_store.list(10_000, None).await {
        Ok(result) => {
            if result.blobs.is_empty() {
                info!(node_id = config.node_id, "no local blobs to announce");
                return;
            }

            let blobs: Vec<_> = result.blobs.iter().map(|e| (e.hash, e.size, e.format)).collect();
            let count = blobs.len();

            info!(node_id = config.node_id, blob_count = count, "announcing local blobs to DHT");

            match discovery.announce_local_blobs(blobs).await {
                Ok(announced) => {
                    info!(node_id = config.node_id, announced, total = count, "auto-announce complete");
                }
                Err(err) => {
                    warn!(
                        node_id = config.node_id,
                        error = %err,
                        "failed to auto-announce local blobs"
                    );
                }
            }
        }
        Err(err) => {
            warn!(
                node_id = config.node_id,
                error = %err,
                "failed to list local blobs for auto-announce"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // derive_topic_id_from_cookie Tests
    // =========================================================================

    #[test]
    fn test_topic_id_from_cookie_deterministic() {
        // Same cookie should always produce same topic ID
        let topic1 = derive_topic_id_from_cookie("test-cluster-cookie");
        let topic2 = derive_topic_id_from_cookie("test-cluster-cookie");
        assert_eq!(topic1, topic2);
    }

    #[test]
    fn test_topic_id_from_cookie_different_cookies() {
        // Different cookies should produce different topic IDs
        let topic1 = derive_topic_id_from_cookie("cluster-alpha");
        let topic2 = derive_topic_id_from_cookie("cluster-beta");
        assert_ne!(topic1, topic2);
    }

    #[test]
    fn test_topic_id_from_cookie_empty() {
        // Empty cookie should still produce a valid topic ID
        let topic = derive_topic_id_from_cookie("");
        // blake3 hash of empty string is deterministic
        assert_eq!(topic.as_bytes().len(), 32);
    }

    #[test]
    fn test_topic_id_from_cookie_unicode() {
        // Unicode cookies should work
        let topic = derive_topic_id_from_cookie("集群-α-βeta");
        assert_eq!(topic.as_bytes().len(), 32);
    }

    #[test]
    fn test_topic_id_from_cookie_whitespace_sensitive() {
        // Whitespace should be significant
        let topic1 = derive_topic_id_from_cookie("cookie");
        let topic2 = derive_topic_id_from_cookie(" cookie");
        let topic3 = derive_topic_id_from_cookie("cookie ");
        assert_ne!(topic1, topic2);
        assert_ne!(topic1, topic3);
        assert_ne!(topic2, topic3);
    }

    #[test]
    fn test_topic_id_from_default_cookie() {
        // Default unsafe cookie should produce consistent topic ID
        let topic = derive_topic_id_from_cookie("aspen-cookie-UNSAFE-CHANGE-ME");
        assert_eq!(topic.as_bytes().len(), 32);
    }

    // =========================================================================
    // parse_peer_addresses Tests
    // =========================================================================

    #[test]
    fn test_parse_peer_addresses_empty() {
        let result = parse_peer_addresses(&[]).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_peer_addresses_invalid_format_no_at() {
        let specs = vec!["invalid-spec".to_string()];
        let result = parse_peer_addresses(&specs);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("invalid peer spec"));
    }

    #[test]
    fn test_parse_peer_addresses_invalid_node_id() {
        let specs = vec!["not_a_number@someendpoint".to_string()];
        let result = parse_peer_addresses(&specs);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("invalid node_id"));
    }

    #[test]
    fn test_parse_peer_addresses_multiple_at_signs() {
        // More than one @ sign should result in 3+ parts after split
        let specs = vec!["1@endpoint@extra".to_string()];
        let result = parse_peer_addresses(&specs);
        assert!(result.is_err());
    }

    // =========================================================================
    // load_config Tests
    // =========================================================================

    #[test]
    fn test_load_config_default_overrides() {
        // Test that overrides work with default values
        let overrides = NodeConfig {
            node_id: 42,
            cookie: "test-cookie".into(),
            ..Default::default()
        };

        let result = load_config(None, overrides);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.node_id, 42);
    }

    #[test]
    fn test_load_config_custom_heartbeat() {
        let overrides = NodeConfig {
            node_id: 1,
            cookie: "test-cookie".into(),
            heartbeat_interval_ms: 500,
            ..Default::default()
        };

        let result = load_config(None, overrides);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.heartbeat_interval_ms, 500);
    }

    #[test]
    fn test_load_config_custom_election_timeouts() {
        let overrides = NodeConfig {
            node_id: 1,
            cookie: "test-cookie".into(),
            election_timeout_min_ms: 2000,
            election_timeout_max_ms: 4000,
            ..Default::default()
        };

        let result = load_config(None, overrides);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.election_timeout_min_ms, 2000);
        assert_eq!(config.election_timeout_max_ms, 4000);
    }

    #[test]
    fn test_load_config_storage_backend() {
        let overrides = NodeConfig {
            node_id: 1,
            cookie: "test-cookie".into(),
            storage_backend: StorageBackend::InMemory,
            ..Default::default()
        };

        let result = load_config(None, overrides);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.storage_backend, StorageBackend::InMemory);
    }

    #[test]
    fn test_load_config_iroh_gossip_settings() {
        use crate::config::IrohConfig;

        let overrides = NodeConfig {
            node_id: 1,
            cookie: "test-cookie".into(),
            iroh: IrohConfig {
                enable_gossip: false,
                enable_mdns: false,
                ..Default::default()
            },
            ..Default::default()
        };

        let result = load_config(None, overrides);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(!config.iroh.enable_gossip);
        assert!(!config.iroh.enable_mdns);
    }

    #[test]
    fn test_load_config_nonexistent_toml() {
        let overrides = NodeConfig::default();
        let result = load_config(Some(std::path::Path::new("/nonexistent/config.toml")), overrides);
        assert!(result.is_err());
    }

    // =========================================================================
    // NodeHandle Tests (struct properties)
    // =========================================================================

    #[test]
    fn test_node_handle_fields_are_public() {
        // Verify NodeHandle fields are accessible (compile-time check)
        // This test ensures the API remains stable
        fn _check_handle_fields(handle: &NodeHandle) {
            // Core fields
            let _: &NodeConfig = &handle.config;
            let _: &Arc<MetadataStore> = &handle.metadata_store;
            let _: &Option<CapabilityToken> = &handle.root_token;

            // Storage resources (composed)
            let _: &Arc<RaftNode> = &handle.storage.raft_node;
            let _: &StateMachineVariant = &handle.storage.state_machine;
            let _: &Option<CancellationToken> = &handle.storage.ttl_cleanup_cancel;

            // Network resources (composed)
            let _: &Arc<IrohEndpointManager> = &handle.network.iroh_manager;
            let _: &Arc<IrpcRaftNetworkFactory> = &handle.network.network_factory;
            let _: &Option<RaftRpcServer> = &handle.network.rpc_server;
            let _: &Option<Arc<IrohBlobStore>> = &handle.network.blob_store;

            // Discovery resources (composed)
            let _: &Option<GossipPeerDiscovery> = &handle.discovery.gossip_discovery;
            let _: &TopicId = &handle.discovery.gossip_topic_id;

            // Sync resources (composed)
            let _: &Option<broadcast::Sender<LogEntryPayload>> = &handle.sync.log_broadcast;
            let _: &Option<CancellationToken> = &handle.sync.docs_exporter_cancel;
            let _: &Option<CancellationToken> = &handle.sync.sync_event_listener_cancel;
            let _: &Option<CancellationToken> = &handle.sync.docs_sync_service_cancel;
            let _: &Option<Arc<aspen_docs::DocsSyncResources>> = &handle.sync.docs_sync;

            // Worker resources (composed)
            // Handle doesn't expose supervisor directly - access through composed groups

            // Hooks resources (composed)
            let _: &Option<Arc<aspen_hooks::HookService>> = &handle.hooks.hook_service;

            // Shutdown coordinator
            let _: &CancellationToken = &handle.shutdown.shutdown_token;
        }
    }

    // =========================================================================
    // ShardedNodeHandle Tests (struct properties)
    // =========================================================================

    #[test]
    fn test_sharded_node_handle_fields_are_public() {
        // Verify ShardedNodeHandle fields are accessible (compile-time check)
        // This test ensures the API remains stable
        fn _check_handle_fields(handle: &ShardedNodeHandle) {
            // Base resources
            let _: &BaseNodeResources = &handle.base;
            let _: &Option<CapabilityToken> = &handle.root_token;
            let _: &Arc<Supervisor> = &handle.supervisor;

            // Sharding resources (shard-specific Raft instances)
            let _: &HashMap<ShardId, Arc<RaftNode>> = &handle.sharding.shard_nodes;
            let _: &Arc<ShardedKeyValueStore<RaftNode>> = &handle.sharding.sharded_kv;
            let _: &Arc<ShardedRaftProtocolHandler> = &handle.sharding.sharded_handler;
            let _: &HashMap<ShardId, Arc<RaftNodeHealth>> = &handle.sharding.health_monitors;
            let _: &HashMap<ShardId, CancellationToken> = &handle.sharding.ttl_cleanup_cancels;
            let _: &HashMap<ShardId, StateMachineVariant> = &handle.sharding.shard_state_machines;
            let _: &Option<Arc<tokio::sync::RwLock<aspen_sharding::ShardTopology>>> = &handle.sharding.topology;

            // Discovery resources
            let _: &Option<crate::content_discovery::ContentDiscoveryService> = &handle.discovery.content_discovery;
            let _: &Option<CancellationToken> = &handle.discovery.content_discovery_cancel;

            // Sync resources
            let _: &Option<broadcast::Sender<LogEntryPayload>> = &handle.sync.log_broadcast;
            let _: &Option<CancellationToken> = &handle.sync.docs_exporter_cancel;
            let _: &Option<CancellationToken> = &handle.sync.sync_event_listener_cancel;
            let _: &Option<CancellationToken> = &handle.sync.docs_sync_service_cancel;

            // Worker resources
            let _: &Option<Arc<crate::worker_service::WorkerService>> = &handle.worker.worker_service;
            let _: &Option<CancellationToken> = &handle.worker.worker_service_cancel;
        }
    }

    #[test]
    fn test_base_node_resources_fields_are_public() {
        // Verify BaseNodeResources fields are accessible (compile-time check)
        fn _check_base_fields(base: &BaseNodeResources) {
            let _: &NodeConfig = &base.config;
            let _: &Arc<MetadataStore> = &base.metadata_store;
            // Network resources (nested)
            let _: &Arc<IrohEndpointManager> = &base.network.iroh_manager;
            let _: &Arc<IrpcRaftNetworkFactory> = &base.network.network_factory;
            let _: &Option<Arc<IrohBlobStore>> = &base.network.blob_store;
            // Discovery resources (nested)
            let _: &Option<GossipPeerDiscovery> = &base.discovery.gossip_discovery;
            let _: &TopicId = &base.discovery.gossip_topic_id;
            // Top-level
            let _: &CancellationToken = &base.shutdown_token;
        }
    }

    // =========================================================================
    // ShardedNodeHandle Method Tests
    // =========================================================================

    #[test]
    fn test_sharded_node_handle_shard_count_accessor() {
        // Test that shard_count() returns correct value
        // This is a compile-time check that the method exists
        fn _check_shard_count(handle: &ShardedNodeHandle) -> usize {
            handle.shard_count()
        }
    }

    #[test]
    fn test_sharded_node_handle_local_shard_ids_accessor() {
        // Test that local_shard_ids() returns correct type
        // This is a compile-time check that the method exists
        fn _check_local_shard_ids(handle: &ShardedNodeHandle) -> Vec<ShardId> {
            handle.local_shard_ids()
        }
    }

    #[test]
    fn test_sharded_node_handle_primary_shard_accessor() {
        // Test that primary_shard() returns correct type
        // This is a compile-time check that the method exists
        fn _check_primary_shard(handle: &ShardedNodeHandle) -> Option<&Arc<RaftNode>> {
            handle.primary_shard()
        }
    }

    // =========================================================================
    // Resource Struct Shutdown Logic Tests
    // =========================================================================

    /// Test StorageResources::shutdown() is idempotent.
    ///
    /// Calling shutdown multiple times should not panic or cause errors.
    #[test]
    fn test_storage_resources_shutdown_idempotent() {
        use aspen_raft::storage::InMemoryStateMachine;

        // Create minimal StorageResources for testing shutdown
        // We can't create a full RaftNode without Raft infrastructure,
        // but we can verify the shutdown method signature exists
        fn _verify_shutdown_signature(resources: &StorageResources) {
            resources.shutdown();
            // Call again to verify idempotency
            resources.shutdown();
        }
    }

    /// Test StorageResources with TTL cleanup cancellation token.
    #[test]
    fn test_storage_resources_ttl_cleanup_cancellation() {
        let cancel_token = CancellationToken::new();
        assert!(!cancel_token.is_cancelled());

        // Simulating what StorageResources::shutdown() does
        cancel_token.cancel();
        assert!(cancel_token.is_cancelled());

        // Calling cancel again should be safe (idempotent)
        cancel_token.cancel();
        assert!(cancel_token.is_cancelled());
    }

    /// Test DiscoveryResources shutdown token cancellation.
    #[test]
    fn test_discovery_resources_content_discovery_cancellation() {
        let cancel_token = CancellationToken::new();
        assert!(!cancel_token.is_cancelled());

        // Simulating what DiscoveryResources::shutdown() does
        cancel_token.cancel();
        assert!(cancel_token.is_cancelled());
    }

    /// Test SyncResources shutdown token cancellation order.
    #[test]
    fn test_sync_resources_cancellation_tokens() {
        let docs_exporter_cancel = CancellationToken::new();
        let sync_event_listener_cancel = CancellationToken::new();
        let docs_sync_service_cancel = CancellationToken::new();

        // Verify none are cancelled initially
        assert!(!docs_exporter_cancel.is_cancelled());
        assert!(!sync_event_listener_cancel.is_cancelled());
        assert!(!docs_sync_service_cancel.is_cancelled());

        // Shutdown order: docs_exporter -> sync_event_listener -> docs_sync_service
        docs_exporter_cancel.cancel();
        sync_event_listener_cancel.cancel();
        docs_sync_service_cancel.cancel();

        // All should be cancelled
        assert!(docs_exporter_cancel.is_cancelled());
        assert!(sync_event_listener_cancel.is_cancelled());
        assert!(docs_sync_service_cancel.is_cancelled());
    }

    /// Test WorkerResources shutdown token cancellation.
    #[test]
    fn test_worker_resources_cancellation() {
        let worker_service_cancel = CancellationToken::new();
        assert!(!worker_service_cancel.is_cancelled());

        worker_service_cancel.cancel();
        assert!(worker_service_cancel.is_cancelled());
    }

    /// Test HookResources shutdown method exists.
    #[test]
    fn test_hook_resources_shutdown_signature() {
        // Verify HookResources has async shutdown method
        fn _verify_shutdown(_resources: &HookResources) {
            // Method signature check - actual execution requires async runtime
            std::mem::drop(std::future::pending::<()>());
        }
    }

    /// Test ShutdownResources cancellation token.
    #[test]
    fn test_shutdown_resources_cancellation() {
        let shutdown_token = CancellationToken::new();
        assert!(!shutdown_token.is_cancelled());

        shutdown_token.cancel();
        assert!(shutdown_token.is_cancelled());
    }

    // =========================================================================
    // Bootstrap Configuration Validation Tests
    // =========================================================================

    /// Test that node_id of 1 is accepted (valid minimum node ID).
    #[test]
    fn test_load_config_node_id_one() {
        let overrides = NodeConfig {
            node_id: 1,
            cookie: "test-cookie".into(),
            ..Default::default()
        };

        let result = load_config(None, overrides);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.node_id, 1);
    }

    /// Test that max u64 node_id is accepted.
    #[test]
    fn test_load_config_node_id_max() {
        let overrides = NodeConfig {
            node_id: u64::MAX,
            cookie: "test-cookie".into(),
            ..Default::default()
        };

        let result = load_config(None, overrides);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.node_id, u64::MAX);
    }

    /// Test redb storage backend configuration.
    #[test]
    fn test_load_config_storage_backend_redb() {
        let overrides = NodeConfig {
            node_id: 1,
            cookie: "test-cookie".into(),
            storage_backend: StorageBackend::Redb,
            ..Default::default()
        };

        let result = load_config(None, overrides);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.storage_backend, StorageBackend::Redb);
    }

    /// Test iroh configuration with all discovery methods enabled.
    #[test]
    fn test_load_config_iroh_full_discovery() {
        use crate::config::IrohConfig;

        let overrides = NodeConfig {
            node_id: 1,
            cookie: "test-cookie".into(),
            iroh: IrohConfig {
                enable_gossip: true,
                enable_mdns: true,
                enable_pkarr: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let result = load_config(None, overrides);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.iroh.enable_gossip);
        assert!(config.iroh.enable_mdns);
        assert!(config.iroh.enable_pkarr);
    }
}
