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
use iroh::EndpointAddr;
use iroh_gossip::proto::TopicId;
use openraft::Config as RaftConfig;
use openraft::Raft;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::info;
use tracing::warn;

use aspen_auth::CapabilityToken;
use aspen_blob::IrohBlobStore;
use crate::IrohEndpointConfig;
use crate::IrohEndpointManager;
use crate::config::NodeConfig;
use crate::gossip_discovery::{spawn_gossip_peer_discovery, GossipPeerDiscovery};
use crate::metadata::MetadataStore;
use crate::metadata::NodeStatus;
use crate::ticket::AspenClusterTicket;
use aspen_transport::ShardedRaftProtocolHandler;
use aspen_raft::StateMachineVariant;
use aspen_raft::log_subscriber::LOG_BROADCAST_BUFFER_SIZE;
use aspen_raft::log_subscriber::LogEntryPayload;
// Use the type alias from cluster mod.rs which provides the concrete type
use super::IrpcRaftNetworkFactory;
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
use aspen_raft::types::NodeId;
use aspen_raft::types::AppTypeConfig;
use aspen_transport::rpc::AppTypeConfig as TransportAppTypeConfig;
use aspen_sharding::ShardConfig;
use aspen_sharding::ShardId;
use aspen_sharding::ShardStoragePaths;
use aspen_sharding::ShardedKeyValueStore;
use aspen_sharding::encode_shard_node_id;

/// Handle to a running cluster node.
///
/// Contains all the resources needed to run and shutdown a node cleanly
/// using direct async APIs.
pub struct NodeHandle {
    /// Node configuration.
    pub config: NodeConfig,
    /// Metadata store for cluster nodes.
    pub metadata_store: Arc<MetadataStore>,
    /// Iroh endpoint manager.
    pub iroh_manager: Arc<IrohEndpointManager>,
    /// Raft node (direct wrapper around OpenRaft).
    pub raft_node: Arc<RaftNode>,
    /// IRPC network factory for dynamic peer addition.
    pub network_factory: Arc<IrpcRaftNetworkFactory>,
    /// Gossip discovery service (if enabled).
    ///
    /// Automatically broadcasts this node's presence and discovers peers
    /// via iroh-gossip. Discovered peers are added to the network factory
    /// for Raft RPC routing.
    pub gossip_discovery: Option<GossipPeerDiscovery>,
    /// Legacy RPC server for handling incoming Raft RPCs.
    ///
    /// When using Iroh Router with RaftProtocolHandler (preferred), this should be None.
    /// The Router handles ALPN-based dispatching directly.
    /// Only used when Router is not available (e.g., testing scenarios).
    pub rpc_server: Option<RaftRpcServer>,
    /// Supervisor for automatic restarts.
    pub supervisor: Arc<Supervisor>,
    /// Health monitor.
    pub health_monitor: Arc<RaftNodeHealth>,
    /// Cancellation token for shutdown.
    pub shutdown_token: CancellationToken,
    /// Gossip topic ID used for peer discovery and cluster tickets.
    ///
    /// Always available (derived from cluster cookie or provided via ticket).
    /// Used for:
    /// - Gossip peer discovery (when enabled)
    /// - Generating cluster tickets via HTTP API (GET /cluster-ticket)
    ///
    /// Tiger Style: Always computed to enable ticket generation even when
    /// gossip discovery is disabled.
    pub gossip_topic_id: TopicId,
    /// Blob store for content-addressed storage.
    ///
    /// Provides large value offloading and P2P blob distribution.
    /// None when blobs are disabled in configuration.
    pub blob_store: Option<Arc<IrohBlobStore>>,
    /// Peer manager for cluster-to-cluster sync.
    ///
    /// Manages connections to peer Aspen clusters for iroh-docs
    /// based data synchronization with priority-based conflict resolution.
    /// None when peer sync is disabled in configuration.
    // Note: aspen-docs module should be extracted from main crate for modularity
    // pub peer_manager: Option<Arc<aspen_docs::PeerManager>>,
    /// Log broadcast sender for DocsExporter and other subscribers.
    ///
    /// When docs export is enabled, committed KV entries are broadcast on this
    /// channel for real-time synchronization to iroh-docs.
    /// None when docs export is disabled.
    pub log_broadcast: Option<broadcast::Sender<LogEntryPayload>>,
    /// DocsExporter cancellation token.
    ///
    /// Used to gracefully shutdown the DocsExporter background task.
    /// None when docs export is disabled.
    pub docs_exporter_cancel: Option<CancellationToken>,
    /// Docs sync resources for P2P CRDT replication.
    ///
    /// Contains the SyncHandle and NamespaceId for accepting incoming
    /// sync connections. Wrapped in Arc to allow sharing with DocsSyncService.
    /// None when docs P2P sync is disabled.
    // Note: aspen-docs module should be extracted from main crate for modularity
    // pub docs_sync: Option<Arc<aspen_docs::DocsSyncResources>>,
    /// TTL cleanup task cancellation token.
    ///
    /// Used to gracefully shutdown the background TTL cleanup task.
    /// None when using InMemory storage (TTL cleanup only applies to SQLite).
    pub ttl_cleanup_cancel: Option<CancellationToken>,
    /// Sync event listener cancellation token.
    ///
    /// Used to gracefully shutdown the background task that listens for
    /// iroh-docs sync events and forwards RemoteInsert entries to DocsImporter.
    /// None when docs sync or peer manager is disabled.
    pub sync_event_listener_cancel: Option<CancellationToken>,
    /// DocsSyncService cancellation token.
    ///
    /// Used to gracefully shutdown the background sync service that
    /// periodically syncs with peer clusters.
    /// None when docs sync is disabled.
    pub docs_sync_service_cancel: Option<CancellationToken>,
    /// Root token generated during cluster initialization (if requested).
    ///
    /// Only present when the node initialized a new cluster (not joining existing)
    /// and token generation was requested via bootstrap configuration.
    pub root_token: Option<CapabilityToken>,
    /// Global content discovery service.
    ///
    /// Enables announcing and finding blobs via the BitTorrent Mainline DHT
    /// for cross-cluster discovery without direct federation.
    /// None when content discovery is disabled in configuration.
    pub content_discovery: Option<crate::content_discovery::ContentDiscoveryService>,
    /// Content discovery service cancellation token.
    ///
    /// Used to gracefully shutdown the background DHT service.
    /// None when content discovery is disabled.
    pub content_discovery_cancel: Option<CancellationToken>,
}

impl NodeHandle {
    /// Gracefully shutdown the node.
    pub async fn shutdown(self) -> Result<()> {
        info!("shutting down node {}", self.config.node_id);

        // Signal shutdown to all components
        self.shutdown_token.cancel();

        // Stop gossip discovery if enabled (NodeHandle-specific, owned)
        if let Some(gossip_discovery) = self.gossip_discovery {
            info!("shutting down gossip discovery");
            if let Err(err) = gossip_discovery.shutdown().await {
                error!(error = ?err, "failed to shutdown gossip discovery gracefully");
            }
        }

        // Stop RPC server if present (NodeHandle-specific)
        if let Some(rpc_server) = self.rpc_server {
            info!("shutting down legacy RPC server");
            if let Err(err) = rpc_server.shutdown().await {
                error!(error = ?err, "failed to shutdown RPC server gracefully");
            }
        }

        // Docs sync resources have been extracted to aspen-docs crate
        // No direct field cleanup needed as docs functionality is managed separately

        // Shutdown TTL cleanup task if present (NodeHandle-specific, single task)
        if let Some(cancel_token) = &self.ttl_cleanup_cancel {
            info!("shutting down TTL cleanup task");
            cancel_token.cancel();
        }

        // Shutdown content discovery if present (NodeHandle-specific)
        if let Some(cancel_token) = &self.content_discovery_cancel {
            info!("shutting down content discovery");
            cancel_token.cancel();
        }

        // Shutdown common resources (sync services, peer manager, supervisor, blob store, iroh, metadata)
        shutdown_common_resources(CommonShutdownResources {
            sync_event_listener_cancel: self.sync_event_listener_cancel.as_ref(),
            docs_sync_service_cancel: self.docs_sync_service_cancel.as_ref(),
            docs_exporter_cancel: self.docs_exporter_cancel.as_ref(),
            content_discovery_cancel: self.content_discovery_cancel.as_ref(),
            supervisor: &self.supervisor,
            blob_store: self.blob_store.as_ref(),
            iroh_manager: &self.iroh_manager,
            metadata_store: &self.metadata_store,
            node_id: self.config.node_id,
        })
        .await
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

/// Base node resources shared across all shards.
///
/// Contains the transport and discovery infrastructure that is shared
/// by all shards on a node. This is separated from Raft-specific resources
/// to enable per-shard Raft instances while sharing the common P2P transport.
pub struct BaseNodeResources {
    /// Node configuration.
    pub config: NodeConfig,
    /// Metadata store for cluster nodes.
    pub metadata_store: Arc<MetadataStore>,
    /// Iroh endpoint manager for P2P transport.
    pub iroh_manager: Arc<IrohEndpointManager>,
    /// IRPC network factory for dynamic peer addition.
    pub network_factory: Arc<IrpcRaftNetworkFactory>,
    /// Gossip discovery service (if enabled).
    pub gossip_discovery: Option<GossipPeerDiscovery>,
    /// Gossip topic ID for peer discovery and cluster tickets.
    pub gossip_topic_id: TopicId,
    /// Cancellation token for shutdown.
    pub shutdown_token: CancellationToken,
    /// Blob store for content-addressed storage (optional).
    pub blob_store: Option<Arc<IrohBlobStore>>,
}

/// Handle to a running sharded cluster node.
///
/// Contains multiple independent Raft instances (one per shard) that share
/// the same underlying Iroh P2P transport. Each shard is a separate Raft
/// consensus group with its own leader election and log replication.
///
/// # Architecture
///
/// ```text
/// ShardedNodeHandle
///     ├── base (BaseNodeResources)
///     │     └── iroh_manager (shared P2P transport)
///     │
///     ├── shard_nodes
///     │     ├── shard 0 → RaftNode (shard-0/raft-log.db, shard-0/state-machine.db)
///     │     ├── shard 1 → RaftNode (shard-1/raft-log.db, shard-1/state-machine.db)
///     │     └── shard N → RaftNode (shard-N/raft-log.db, shard-N/state-machine.db)
///     │
///     ├── sharded_kv (routes keys to correct shard)
///     │
///     └── sharded_handler (ALPN: raft-shard)
/// ```
pub struct ShardedNodeHandle {
    /// Base node resources (Iroh, metadata, gossip - shared across shards).
    pub base: BaseNodeResources,
    /// Map of shard ID to Raft node for that shard.
    pub shard_nodes: HashMap<ShardId, Arc<RaftNode>>,
    /// Sharded key-value store wrapping all shard nodes.
    pub sharded_kv: Arc<ShardedKeyValueStore<RaftNode>>,
    /// Protocol handler for sharded Raft RPC.
    pub sharded_handler: Arc<ShardedRaftProtocolHandler>,
    /// Supervisor for health monitoring.
    pub supervisor: Arc<Supervisor>,
    /// Health monitors for each shard.
    pub health_monitors: HashMap<ShardId, Arc<RaftNodeHealth>>,
    /// TTL cleanup cancellation tokens (one per shard using SQLite).
    pub ttl_cleanup_cancels: HashMap<ShardId, CancellationToken>,
    /// Peer manager for cluster-to-cluster sync (optional).
    // Note: aspen-docs module should be extracted from main crate for modularity
    // pub peer_manager: Option<Arc<aspen_docs::PeerManager>>,
    /// Log broadcast sender for DocsExporter (optional).
    /// Note: In sharded mode, only shard 0 exports to docs for simplicity.
    pub log_broadcast: Option<broadcast::Sender<LogEntryPayload>>,
    /// DocsExporter cancellation token (optional).
    pub docs_exporter_cancel: Option<CancellationToken>,
    /// Docs sync resources for P2P CRDT replication (optional).
    // Note: aspen-docs module should be extracted from main crate for modularity
    // pub docs_sync: Option<Arc<aspen_docs::DocsSyncResources>>,
    /// Sync event listener cancellation token (optional).
    pub sync_event_listener_cancel: Option<CancellationToken>,
    /// DocsSyncService cancellation token (optional).
    pub docs_sync_service_cancel: Option<CancellationToken>,
    /// Root token generated during cluster initialization (if requested).
    pub root_token: Option<CapabilityToken>,
    /// Sharding topology for shard routing and redistribution (optional).
    pub topology: Option<Arc<tokio::sync::RwLock<aspen_sharding::ShardTopology>>>,
    /// Global content discovery service.
    ///
    /// Enables announcing and finding blobs via the BitTorrent Mainline DHT
    /// for cross-cluster discovery without direct federation.
    /// None when content discovery is disabled in configuration.
    pub content_discovery: Option<crate::content_discovery::ContentDiscoveryService>,
    /// Content discovery service cancellation token.
    ///
    /// Used to gracefully shutdown the background DHT service.
    /// None when content discovery is disabled.
    pub content_discovery_cancel: Option<CancellationToken>,
}

impl ShardedNodeHandle {
    /// Gracefully shutdown all shards and the node.
    pub async fn shutdown(self) -> Result<()> {
        info!(
            node_id = self.base.config.node_id,
            shard_count = self.shard_nodes.len(),
            "shutting down sharded node"
        );

        // Signal shutdown to all components
        self.base.shutdown_token.cancel();

        // Stop gossip discovery if enabled (ShardedNodeHandle-specific, owned via base)
        if let Some(gossip_discovery) = self.base.gossip_discovery {
            info!("shutting down gossip discovery");
            if let Err(err) = gossip_discovery.shutdown().await {
                error!(error = ?err, "failed to shutdown gossip discovery gracefully");
            }
        }

        // Docs sync resources have been extracted to aspen-docs crate
        // No direct field cleanup needed as docs functionality is managed separately

        // Shutdown TTL cleanup tasks for all shards (ShardedNodeHandle-specific, loop)
        for (shard_id, cancel_token) in &self.ttl_cleanup_cancels {
            info!(shard_id, "shutting down TTL cleanup task for shard");
            cancel_token.cancel();
        }

        // Shutdown common resources (sync services, peer manager, supervisor, blob store, iroh, metadata)
        shutdown_common_resources(CommonShutdownResources {
            sync_event_listener_cancel: self.sync_event_listener_cancel.as_ref(),
            docs_sync_service_cancel: self.docs_sync_service_cancel.as_ref(),
            docs_exporter_cancel: self.docs_exporter_cancel.as_ref(),
            content_discovery_cancel: self.content_discovery_cancel.as_ref(),
            supervisor: &self.supervisor,
            blob_store: self.base.blob_store.as_ref(),
            iroh_manager: &self.base.iroh_manager,
            metadata_store: &self.base.metadata_store,
            node_id: self.base.config.node_id,
        })
        .await?;

        info!(node_id = self.base.config.node_id, "sharded node shutdown complete");
        Ok(())
    }

    /// Get a reference to the first shard's RaftNode (shard 0).
    ///
    /// Used for compatibility with single-shard APIs and initialization.
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
        info!("Raft authentication enabled - using Iroh-native NodeId verification");
    }

    // Create network factory (no auth context needed - server handles auth)
    let network_factory = Arc::new(IrpcRaftNetworkFactory::new(iroh_manager.clone(), peer_addrs));

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
        iroh_manager,
        network_factory,
        gossip_discovery,
        gossip_topic_id,
        shutdown_token,
        blob_store,
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
                        base.network_factory.as_ref().clone(),
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
                let shared_storage = Arc::new(
                    SharedRedbStorage::new(&db_path)
                        .map_err(|e| anyhow::anyhow!("failed to open shared redb storage for shard: {}", e))?,
                );

                info!(
                    node_id = config.node_id,
                    shard_id,
                    path = %db_path.display(),
                    "created shared redb storage for shard (single-fsync mode)"
                );

                let raft: Arc<Raft<AppTypeConfig>> = Arc::new(
                    Raft::new(
                        shard_node_id.into(),
                        raft_config.clone(),
                        base.network_factory.as_ref().clone(),
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
        // SAFETY: aspen_raft::types::AppTypeConfig and aspen_transport::rpc::AppTypeConfig are identical
        // but Rust treats them as different types. We transmute to convert between them.
        let transport_raft: openraft::Raft<TransportAppTypeConfig> =
            unsafe { std::mem::transmute(raft.as_ref().clone()) };
        sharded_handler.register_shard(shard_id, transport_raft);

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
    let _peer_manager = if config.peer_sync.enabled {
        if let Some(shard_0) = shard_nodes.get(&0) {
            use aspen_docs::DocsImporter;
            use aspen_docs::PeerManager;

            let importer = Arc::new(DocsImporter::new(config.cookie.clone(), shard_0.clone()));
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
        initialize_content_discovery(&config, &base.iroh_manager, &shutdown_for_content).await?;

    // Auto-announce local blobs to DHT if enabled (only from shard 0 in sharded mode)
    if content_discovery.is_some() && base.blob_store.is_some() && shard_nodes.contains_key(&0) {
        let config_clone = config.clone();
        let blob_store_clone = base.blob_store.clone();
        let content_discovery_clone = content_discovery.clone();
        tokio::spawn(async move {
            auto_announce_local_blobs(&config_clone, blob_store_clone.as_ref(), content_discovery_clone.as_ref())
                .await;
        });
    }

    // Register node in metadata store
    use crate::metadata::NodeMetadata;
    base.metadata_store.register_node(NodeMetadata {
        node_id: config.node_id,
        endpoint_id: base.iroh_manager.node_addr().id.to_string(),
        raft_addr: String::new(),
        status: NodeStatus::Online,
        last_updated_secs: aspen_core::utils::current_time_secs(),
    })?;

    info!(node_id = config.node_id, shard_count = shard_nodes.len(), "sharded node bootstrap complete");

    Ok(ShardedNodeHandle {
        base,
        shard_nodes,
        sharded_kv,
        sharded_handler,
        supervisor,
        health_monitors,
        ttl_cleanup_cancels,
        log_broadcast: None,
        docs_exporter_cancel: None,
        sync_event_listener_cancel: None,
        docs_sync_service_cancel: None,
        root_token: None,
        topology: Some(topology),
        content_discovery,
        content_discovery_cancel,
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
        info!("Raft authentication enabled - using Iroh-native NodeId verification");
    }

    // Create network factory (no auth context needed - server handles auth)
    let network_factory = Arc::new(IrpcRaftNetworkFactory::new(iroh_manager.clone(), peer_addrs));

    // Derive gossip topic ID from ticket or cookie
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

    // Setup gossip discovery
    let gossip_discovery = setup_gossip_discovery(&config, gossip_topic_id, &iroh_manager, &network_factory).await;

    // Create Raft config and log broadcast channel
    let (raft_config, log_broadcast) = create_raft_config_and_broadcast(&config);

    // Create Raft instance with appropriate storage backend
    let (raft, state_machine_variant, ttl_cleanup_cancel) =
        create_raft_instance(&config, raft_config.clone(), &network_factory, data_dir, log_broadcast.clone()).await?;

    info!(node_id = config.node_id, "created OpenRaft instance");

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

    // Initialize blob store and peer manager
    let blob_store = initialize_blob_store(&config, data_dir, &iroh_manager).await;
    let peer_manager = initialize_peer_manager(&config, &raft_node);

    // Initialize DocsExporter and P2P sync if enabled
    let (docs_exporter_cancel, docs_sync) =
        initialize_docs_export(&config, data_dir, log_broadcast.as_ref(), blob_store.as_ref()).await?;

    // Wire up sync event listener and DocsSyncService if all components are available
    let (sync_event_listener_cancel, docs_sync_service_cancel) =
        wire_docs_sync_services(&config, &docs_sync, &blob_store, &peer_manager, &iroh_manager).await;

    // Initialize global content discovery if enabled
    let (content_discovery, content_discovery_cancel) =
        initialize_content_discovery(&config, &iroh_manager, &shutdown).await?;

    // Auto-announce local blobs to DHT if enabled
    // This runs in background and doesn't block bootstrap
    if content_discovery.is_some() && blob_store.is_some() {
        let config_clone = config.clone();
        let blob_store_clone = blob_store.clone();
        let content_discovery_clone = content_discovery.clone();
        tokio::spawn(async move {
            // Wait a bit for DHT to bootstrap before announcing
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            auto_announce_local_blobs(&config_clone, blob_store_clone.as_ref(), content_discovery_clone.as_ref()).await;
        });
    }

    // Register node in metadata store
    register_node_metadata(&config, &metadata_store, &iroh_manager)?;

    Ok(NodeHandle {
        config,
        metadata_store,
        iroh_manager,
        raft_node,
        network_factory,
        gossip_discovery,
        rpc_server: None, // Router-based architecture preferred; spawn Router in caller
        supervisor,
        health_monitor,
        shutdown_token: shutdown,
        gossip_topic_id,
        blob_store,
        log_broadcast,
        docs_exporter_cancel,
        ttl_cleanup_cancel,
        sync_event_listener_cancel,
        docs_sync_service_cancel,
        root_token: None, // Set by caller after cluster init
        content_discovery,
        content_discovery_cancel,
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
/// 2. TOML configuration file (if path provided)
/// 3. Configuration overrides (typically from CLI args)
///
/// Returns the merged configuration.
pub fn load_config(toml_path: Option<&std::path::Path>, overrides: NodeConfig) -> Result<NodeConfig> {
    // Start with environment variables
    let mut config = NodeConfig::from_env();

    // Merge TOML file if provided
    if let Some(path) = toml_path {
        let toml_config = NodeConfig::from_toml_file(path)
            .map_err(|e| anyhow::anyhow!("failed to load config from {}: {}", path.display(), e))?;
        config.merge(toml_config);
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
    log_broadcast: Option<broadcast::Sender<LogEntryPayload>>,
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
                SharedRedbStorage::with_broadcast(&db_path, log_broadcast)
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
        .with_relay_urls(config.iroh.relay_urls.clone());

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

    // Parse and apply optional secret key
    let iroh_config = match &config.iroh.secret_key {
        Some(secret_key_hex) => {
            let bytes = hex::decode(secret_key_hex)
                .map_err(|e| anyhow::anyhow!("invalid secret key hex: {}", e))?;
            let secret_key = iroh::SecretKey::from_bytes(
                &bytes
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("invalid secret key length"))?,
            );
            iroh_config.with_secret_key(secret_key)
        }
        None => iroh_config,
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

/// Create Raft configuration and log broadcast channel.
///
/// Returns (raft_config, log_broadcast).
fn create_raft_config_and_broadcast(
    config: &NodeConfig,
) -> (Arc<RaftConfig>, Option<broadcast::Sender<LogEntryPayload>>) {
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

    let log_broadcast = if config.docs.enabled {
        let (sender, _) = broadcast::channel(LOG_BROADCAST_BUFFER_SIZE);
        info!(
            node_id = config.node_id,
            buffer_size = LOG_BROADCAST_BUFFER_SIZE,
            "created log broadcast channel for docs export"
        );
        Some(sender)
    } else {
        None
    };

    (raft_config, log_broadcast)
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
fn initialize_peer_manager(_config: &NodeConfig, _raft_node: &Arc<RaftNode>) -> Option<Arc<aspen_docs::PeerManager>> {
    // Peer manager functionality has been extracted to aspen-docs crate
    // Return None to disable docs functionality in aspen-cluster
    None
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

    let exporter = Arc::new(DocsExporter::new(writer));
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
/// 1. Sync Event Listener: Listens for RemoteInsert events from iroh-docs sync
///    and forwards them to DocsImporter for priority-based import.
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
            let _: &NodeConfig = &handle.config;
            let _: &Arc<MetadataStore> = &handle.metadata_store;
            let _: &Arc<IrohEndpointManager> = &handle.iroh_manager;
            let _: &Arc<RaftNode> = &handle.raft_node;
            let _: &Arc<IrpcRaftNetworkFactory> = &handle.network_factory;
            let _: &Option<GossipPeerDiscovery> = &handle.gossip_discovery;
            let _: &Option<RaftRpcServer> = &handle.rpc_server;
            let _: &Arc<Supervisor> = &handle.supervisor;
            let _: &Arc<RaftNodeHealth> = &handle.health_monitor;
            let _: &CancellationToken = &handle.shutdown_token;
            let _: &TopicId = &handle.gossip_topic_id;
            let _: &Option<CancellationToken> = &handle.ttl_cleanup_cancel;
            let _: &Option<Arc<crate::blob::IrohBlobStore>> = &handle.blob_store;
            let _: &Option<Arc<aspen_docs::PeerManager>> = &handle.peer_manager;
            let _: &Option<broadcast::Sender<LogEntryPayload>> = &handle.log_broadcast;
            let _: &Option<CancellationToken> = &handle.docs_exporter_cancel;
            let _: &Option<Arc<aspen_docs::DocsSyncResources>> = &handle.docs_sync;
            let _: &Option<CapabilityToken> = &handle.root_token;
            let _: &Option<CancellationToken> = &handle.sync_event_listener_cancel;
            let _: &Option<CancellationToken> = &handle.docs_sync_service_cancel;
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
            let _: &BaseNodeResources = &handle.base;
            let _: &HashMap<ShardId, Arc<RaftNode>> = &handle.shard_nodes;
            let _: &Arc<ShardedKeyValueStore<RaftNode>> = &handle.sharded_kv;
            let _: &Arc<crate::protocol_handlers::ShardedRaftProtocolHandler> = &handle.sharded_handler;
            let _: &Arc<Supervisor> = &handle.supervisor;
            let _: &HashMap<ShardId, Arc<RaftNodeHealth>> = &handle.health_monitors;
            let _: &HashMap<ShardId, CancellationToken> = &handle.ttl_cleanup_cancels;
            let _: &Option<Arc<aspen_docs::PeerManager>> = &handle.peer_manager;
            let _: &Option<broadcast::Sender<LogEntryPayload>> = &handle.log_broadcast;
            let _: &Option<CancellationToken> = &handle.docs_exporter_cancel;
            let _: &Option<Arc<aspen_docs::DocsSyncResources>> = &handle.docs_sync;
            let _: &Option<CapabilityToken> = &handle.root_token;
            let _: &Option<CancellationToken> = &handle.sync_event_listener_cancel;
            let _: &Option<CancellationToken> = &handle.docs_sync_service_cancel;
        }
    }

    #[test]
    fn test_base_node_resources_fields_are_public() {
        // Verify BaseNodeResources fields are accessible (compile-time check)
        fn _check_base_fields(base: &BaseNodeResources) {
            let _: &NodeConfig = &base.config;
            let _: &Arc<MetadataStore> = &base.metadata_store;
            let _: &Arc<IrohEndpointManager> = &base.iroh_manager;
            let _: &Arc<IrpcRaftNetworkFactory> = &base.network_factory;
            let _: &Option<GossipPeerDiscovery> = &base.gossip_discovery;
            let _: &TopicId = &base.gossip_topic_id;
            let _: &CancellationToken = &base.shutdown_token;
            let _: &Option<Arc<crate::blob::IrohBlobStore>> = &base.blob_store;
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
}
