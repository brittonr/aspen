//! Sharded node bootstrap infrastructure.
//!
//! This module handles bootstrapping nodes with multiple Raft shards,
//! where each shard is an independent Raft consensus group sharing
//! the same underlying Iroh P2P transport.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use anyhow::ensure;
#[cfg(feature = "blob")]
use aspen_blob::IrohBlobStore;
#[cfg(feature = "blob")]
use aspen_blob::create_blob_event_channel;
#[cfg(feature = "docs")]
use aspen_docs::create_docs_event_channel;
use aspen_raft::StateMachineVariant;
use aspen_raft::log_subscriber::LOG_BROADCAST_BUFFER_SIZE;
use aspen_raft::log_subscriber::LogEntryPayload;
use aspen_raft::node::RaftNode;
use aspen_raft::node::RaftNodeHealth;
use aspen_raft::storage::InMemoryLogStore;
use aspen_raft::storage::InMemoryStateMachine;
use aspen_raft::storage::StorageBackend;
use aspen_raft::storage_shared::SharedRedbStorage;
use aspen_raft::supervisor::Supervisor;
use aspen_raft::types::AppTypeConfig;
use aspen_sharding::ShardConfig;
use aspen_sharding::ShardId;
use aspen_sharding::ShardStoragePaths;
use aspen_sharding::ShardedKeyValueStore;
use aspen_sharding::encode_shard_node_id;
use aspen_transport::ShardedRaftProtocolHandler;
use aspen_transport::rpc::AppTypeConfig as TransportAppTypeConfig;
use iroh_gossip::proto::TopicId;
use openraft::Config as RaftConfig;
use openraft::Raft;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing::warn;

use super::IrpcRaftNetworkFactory;
use super::discovery_init::derive_topic_id_from_cookie;
use super::discovery_init::initialize_content_discovery;
use super::hooks_init::initialize_hook_service;
use crate::IrohEndpointConfig;
use crate::IrohEndpointManager;
use crate::bootstrap::resources::DiscoveryResources;
use crate::bootstrap::resources::HookResources;
use crate::bootstrap::resources::NetworkResources;
use crate::bootstrap::resources::SyncResources;
use crate::bootstrap::resources::WorkerResources;
use crate::config::NodeConfig;
use crate::gossip_discovery::GossipPeerDiscovery;
use crate::gossip_discovery::spawn_gossip_peer_discovery;
use crate::metadata::MetadataStore;
use crate::metadata::NodeMetadata;
use crate::metadata::NodeStatus;
use crate::ticket::AspenClusterTicket;

// ============================================================================
// Type Aliases and Resource Structs
// ============================================================================

/// Broadcast channels for shard 0 (used for hooks and docs).
pub(super) type Shard0Broadcasts =
    Option<(broadcast::Sender<LogEntryPayload>, broadcast::Sender<aspen_raft::storage_shared::SnapshotEvent>)>;

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
    pub root_token: Option<aspen_auth::CapabilityToken>,

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
                tracing::error!(error = ?err, "failed to shutdown gossip discovery gracefully");
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
            tracing::error!(
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

// ============================================================================
// Internal Helper Structs
// ============================================================================

/// Resources for sharded context initialization.
///
/// Contains the shared components needed by all shards.
struct ShardContextResources {
    sharded_handler: Arc<ShardedRaftProtocolHandler>,
    sharded_kv: Arc<ShardedKeyValueStore<RaftNode>>,
    topology: Arc<tokio::sync::RwLock<aspen_sharding::ShardTopology>>,
    supervisor: Arc<Supervisor>,
    base_raft_config: RaftConfig,
}

/// Result of creating a single shard's Raft instance.
struct ShardRaftResult {
    raft_node: Arc<RaftNode>,
    state_machine: StateMachineVariant,
    health_monitor: Arc<RaftNodeHealth>,
    ttl_cancel: Option<CancellationToken>,
}

/// Collected shard resources from the per-shard loop.
struct ShardLoopResults {
    shard_nodes: HashMap<ShardId, Arc<RaftNode>>,
    health_monitors: HashMap<ShardId, Arc<RaftNodeHealth>>,
    ttl_cleanup_cancels: HashMap<ShardId, CancellationToken>,
    shard_state_machines: HashMap<ShardId, StateMachineVariant>,
}

// ============================================================================
// Bootstrap Functions
// ============================================================================

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
    config.apply_security_defaults();
    ensure!(config.sharding.enabled, "sharding must be enabled to use bootstrap_sharded_node");

    let num_shards = config.sharding.num_shards;
    let local_shards: Vec<ShardId> = if config.sharding.local_shards.is_empty() {
        (0..num_shards).collect()
    } else {
        config.sharding.local_shards.clone()
    };

    info!(node_id = config.node_id, num_shards, local_shards = ?local_shards, "bootstrapping sharded node");

    let base = bootstrap_base_node(&config).await?;
    let ctx = create_shard_context_resources(&config, num_shards);
    let ShardContextResources {
        sharded_handler,
        sharded_kv,
        topology,
        supervisor,
        base_raft_config,
    } = ctx;

    let data_dir = config
        .data_dir
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("data_dir must be set for sharded node bootstrap"))?;

    let shard_0_broadcasts = create_shard_0_broadcasts(&config, &local_shards);

    // Create all shard Raft instances
    let shard_results = create_all_shard_instances(
        &config,
        &local_shards,
        &base_raft_config,
        &base.network.network_factory,
        &sharded_handler,
        &sharded_kv,
        data_dir,
        &shard_0_broadcasts,
    )
    .await?;
    let ShardLoopResults {
        shard_nodes,
        health_monitors,
        ttl_cleanup_cancels,
        shard_state_machines,
    } = shard_results;

    let peer_manager = initialize_sharded_peer_sync(&config, &shard_nodes);

    let shutdown_for_content = CancellationToken::new();
    let (content_discovery, content_discovery_cancel) =
        initialize_content_discovery(&config, &base.network.iroh_manager, &shutdown_for_content).await?;

    #[cfg(feature = "blob")]
    spawn_sharded_blob_announce(&config, &base.network.blob_store, &content_discovery, &shard_nodes);
    register_sharded_node_metadata(&config, &base.metadata_store, &base.network.iroh_manager)?;

    info!(node_id = config.node_id, shard_count = shard_nodes.len(), "sharded node bootstrap complete");

    let (blob_event_sender, docs_event_sender) = create_sharded_event_channels(&config, &shard_nodes);
    let hooks = initialize_sharded_hooks(
        &config,
        &shard_nodes,
        &shard_state_machines,
        &shard_0_broadcasts,
        blob_event_sender,
        docs_event_sender,
    )
    .await?;

    // Extract gossip_topic_id before moving base
    let gossip_topic_id = base.discovery.gossip_topic_id;

    Ok(ShardedNodeHandle {
        base,
        root_token: None,
        sharding: ShardingResources {
            shard_nodes,
            sharded_kv,
            sharded_handler,
            health_monitors,
            ttl_cleanup_cancels,
            shard_state_machines,
            topology: Some(topology),
        },
        discovery: DiscoveryResources {
            gossip_discovery: None,
            gossip_topic_id,
            content_discovery,
            content_discovery_cancel,
        },
        sync: SyncResources {
            log_broadcast: shard_0_broadcasts.map(|(log, _)| log),
            docs_exporter_cancel: None,
            sync_event_listener_cancel: None,
            docs_sync_service_cancel: None,
            #[cfg(feature = "docs")]
            docs_sync: None,
            #[cfg(feature = "docs")]
            peer_manager,
        },
        worker: WorkerResources {
            #[cfg(feature = "jobs")]
            worker_service: None,
            worker_service_cancel: None,
        },
        hooks,
        supervisor,
    })
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
    let iroh_config = super::network_init::build_iroh_config_from_node_config(config)?;
    let iroh_manager = Arc::new(IrohEndpointManager::new(iroh_config).await?);
    info!(
        node_id = config.node_id,
        endpoint_id = %iroh_manager.node_addr().id,
        "created Iroh endpoint"
    );

    // Parse peer addresses from config if provided
    let peer_addrs = super::network_init::parse_peer_addresses(&config.peers)?;

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
    #[cfg(feature = "blob")]
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
            #[cfg(feature = "blob")]
            blob_store,
        },
        discovery: BaseDiscoveryResources {
            gossip_discovery,
            gossip_topic_id,
        },
        shutdown_token,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create shared context resources for sharded bootstrap.
///
/// This function creates the components shared across all shards:
/// - ShardedRaftProtocolHandler for ALPN routing
/// - ShardedKeyValueStore for shard-aware routing
/// - ShardTopology for shard placement
/// - Supervisor for health monitoring
/// - Base RaftConfig template
fn create_shard_context_resources(config: &NodeConfig, num_shards: u32) -> ShardContextResources {
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

    // Create ShardedKeyValueStore
    let sharded_kv = Arc::new(ShardedKeyValueStore::<RaftNode>::new(shard_config));

    // Create supervisor for all shards
    // Note: Supervisor::new() returns Arc<Supervisor>
    let supervisor = Supervisor::new(format!("sharded-node-{}", config.node_id));

    // Create base Raft config (shared across shards with per-shard cluster name)
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

    ShardContextResources {
        sharded_handler,
        sharded_kv,
        topology,
        supervisor,
        base_raft_config,
    }
}

/// Create broadcast channels for shard 0 if hooks or docs are enabled.
///
/// Only shard 0 needs these channels for hooks to work in sharded mode.
pub(super) fn create_shard_0_broadcasts(config: &NodeConfig, local_shards: &[ShardId]) -> Shard0Broadcasts {
    if (config.hooks.enabled || config.docs.enabled) && local_shards.contains(&0) {
        let (log_sender, _) = broadcast::channel::<LogEntryPayload>(LOG_BROADCAST_BUFFER_SIZE);
        let (snapshot_sender, _) =
            broadcast::channel::<aspen_raft::storage_shared::SnapshotEvent>(LOG_BROADCAST_BUFFER_SIZE);
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
    }
}

/// Create a Raft instance for a single shard.
///
/// This handles storage creation, Raft initialization, and registration
/// with the sharded protocol handler.
#[allow(clippy::too_many_arguments)]
async fn create_shard_raft_instance(
    config: &NodeConfig,
    shard_id: ShardId,
    base_raft_config: &RaftConfig,
    network_factory: &Arc<IrpcRaftNetworkFactory>,
    sharded_handler: &Arc<ShardedRaftProtocolHandler>,
    sharded_kv: &Arc<ShardedKeyValueStore<RaftNode>>,
    data_dir: &std::path::Path,
    shard_0_broadcasts: &Shard0Broadcasts,
) -> Result<ShardRaftResult> {
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
                    network_factory.as_ref().clone(),
                    log_store.as_ref().clone(),
                    state_machine.clone(),
                )
                .await?,
            );
            (raft, StateMachineVariant::InMemory(state_machine), None)
        }
        StorageBackend::Redb => {
            let db_path = paths.log_path.with_extension("shared.redb");

            // For shard 0, pass broadcast channels if hooks/docs are enabled
            let shared_storage = if shard_id == 0 {
                if let Some((log_sender, snapshot_sender)) = shard_0_broadcasts {
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
                    network_factory.as_ref().clone(),
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
    // SAFETY: See safety comment in bootstrap_sharded_node for transmute justification
    let transport_raft: openraft::Raft<TransportAppTypeConfig> = unsafe { std::mem::transmute(raft.as_ref().clone()) };
    sharded_handler.register_shard(shard_id, transport_raft);

    // Create RaftNode wrapper - use batch config from NodeConfig or default
    let raft_node = if let Some(batch_config) = config.batch_config.clone() {
        Arc::new(RaftNode::with_write_batching(
            shard_node_id.into(),
            raft.clone(),
            state_machine_variant.clone(),
            batch_config.finalize(),
        ))
    } else {
        Arc::new(RaftNode::new(shard_node_id.into(), raft.clone(), state_machine_variant.clone()))
    };

    // Create health monitor
    let health_monitor = Arc::new(RaftNodeHealth::new(raft_node.clone()));

    // Register with ShardedKeyValueStore
    sharded_kv.add_shard(shard_id, raft_node.clone()).await;

    Ok(ShardRaftResult {
        raft_node,
        state_machine: state_machine_variant,
        health_monitor,
        ttl_cancel,
    })
}

/// Create all shard Raft instances and collect results.
#[allow(clippy::too_many_arguments)]
async fn create_all_shard_instances(
    config: &NodeConfig,
    local_shards: &[ShardId],
    base_raft_config: &RaftConfig,
    network_factory: &Arc<IrpcRaftNetworkFactory>,
    sharded_handler: &Arc<ShardedRaftProtocolHandler>,
    sharded_kv: &Arc<ShardedKeyValueStore<RaftNode>>,
    data_dir: &std::path::Path,
    shard_0_broadcasts: &Shard0Broadcasts,
) -> Result<ShardLoopResults> {
    let mut shard_nodes: HashMap<ShardId, Arc<RaftNode>> = HashMap::new();
    let mut health_monitors: HashMap<ShardId, Arc<RaftNodeHealth>> = HashMap::new();
    let mut ttl_cleanup_cancels: HashMap<ShardId, CancellationToken> = HashMap::new();
    let mut shard_state_machines: HashMap<ShardId, StateMachineVariant> = HashMap::new();

    for &shard_id in local_shards {
        let result = create_shard_raft_instance(
            config,
            shard_id,
            base_raft_config,
            network_factory,
            sharded_handler,
            sharded_kv,
            data_dir,
            shard_0_broadcasts,
        )
        .await?;

        shard_state_machines.insert(shard_id, result.state_machine);
        shard_nodes.insert(shard_id, result.raft_node);
        health_monitors.insert(shard_id, result.health_monitor);
        if let Some(cancel) = result.ttl_cancel {
            ttl_cleanup_cancels.insert(shard_id, cancel);
        }
    }

    Ok(ShardLoopResults {
        shard_nodes,
        health_monitors,
        ttl_cleanup_cancels,
        shard_state_machines,
    })
}

/// Initialize peer sync for sharded mode using shard 0.
#[cfg(feature = "docs")]
fn initialize_sharded_peer_sync(
    config: &NodeConfig,
    shard_nodes: &HashMap<ShardId, Arc<RaftNode>>,
) -> Option<Arc<aspen_docs::PeerManager>> {
    if !config.peer_sync.enabled {
        return None;
    }

    if let Some(shard_0) = shard_nodes.get(&0) {
        use aspen_docs::DocsImporter;
        use aspen_docs::PeerManager;

        let importer = Arc::new(DocsImporter::new(config.cookie.clone(), shard_0.clone(), &config.node_id.to_string()));
        let manager = Arc::new(PeerManager::new(config.cookie.clone(), importer));

        info!(node_id = config.node_id, "peer sync initialized (using shard 0)");
        Some(manager)
    } else {
        warn!(node_id = config.node_id, "peer sync requested but shard 0 not hosted locally");
        None
    }
}

#[cfg(not(feature = "docs"))]
fn initialize_sharded_peer_sync(_config: &NodeConfig, _shard_nodes: &HashMap<ShardId, Arc<RaftNode>>) -> Option<()> {
    None
}

/// Create event broadcast channels for hooks in sharded mode.
///
/// Returns (blob_event_sender, docs_event_sender) if enabled and shard 0 is local.
#[cfg(all(feature = "blob", feature = "docs"))]
fn create_sharded_event_channels(
    config: &NodeConfig,
    shard_nodes: &HashMap<ShardId, Arc<RaftNode>>,
) -> (Option<broadcast::Sender<aspen_blob::BlobEvent>>, Option<broadcast::Sender<aspen_docs::DocsEvent>>) {
    let blob_event_sender = if config.hooks.enabled && config.blobs.enabled && shard_nodes.contains_key(&0) {
        let (sender, _receiver) = create_blob_event_channel();
        Some(sender)
    } else {
        None
    };

    let docs_event_sender = if config.hooks.enabled && config.docs.enabled && shard_nodes.contains_key(&0) {
        let (sender, _receiver) = create_docs_event_channel();
        Some(sender)
    } else {
        None
    };

    (blob_event_sender, docs_event_sender)
}

#[cfg(not(all(feature = "blob", feature = "docs")))]
fn create_sharded_event_channels(
    _config: &NodeConfig,
    _shard_nodes: &HashMap<ShardId, Arc<RaftNode>>,
) -> (Option<broadcast::Sender<aspen_blob::BlobEvent>>, Option<broadcast::Sender<aspen_docs::DocsEvent>>) {
    (None, None)
}

/// Initialize hook service for sharded mode using shard 0's resources.
async fn initialize_sharded_hooks(
    config: &NodeConfig,
    shard_nodes: &HashMap<ShardId, Arc<RaftNode>>,
    shard_state_machines: &HashMap<ShardId, StateMachineVariant>,
    shard_0_broadcasts: &Shard0Broadcasts,
    blob_event_sender: Option<broadcast::Sender<aspen_blob::BlobEvent>>,
    docs_event_sender: Option<broadcast::Sender<aspen_docs::DocsEvent>>,
) -> Result<HookResources> {
    if let (Some(shard_0_raft), Some(shard_0_sm)) = (shard_nodes.get(&0), shard_state_machines.get(&0)) {
        let (log_broadcast, snapshot_broadcast) = match shard_0_broadcasts {
            Some((log, snapshot)) => (Some(log.clone()), Some(snapshot.clone())),
            None => (None, None),
        };

        initialize_hook_service(
            config,
            log_broadcast.as_ref(),
            snapshot_broadcast.as_ref(),
            blob_event_sender.as_ref(),
            docs_event_sender.as_ref(),
            shard_0_raft,
            shard_0_sm,
        )
        .await
    } else {
        warn!(node_id = config.node_id, "hook service not initialized: shard 0 not hosted locally");
        Ok(HookResources::disabled())
    }
}

/// Register a sharded node in the metadata store.
fn register_sharded_node_metadata(
    config: &NodeConfig,
    metadata_store: &Arc<MetadataStore>,
    iroh_manager: &Arc<IrohEndpointManager>,
) -> Result<()> {
    Ok(metadata_store.register_node(NodeMetadata {
        node_id: config.node_id,
        endpoint_id: iroh_manager.node_addr().id.to_string(),
        raft_addr: String::new(),
        status: NodeStatus::Online,
        last_updated_secs: aspen_core::utils::current_time_secs(),
    })?)
}

/// Spawn blob auto-announce task for sharded mode.
#[cfg(feature = "blob")]
fn spawn_sharded_blob_announce(
    config: &NodeConfig,
    blob_store: &Option<Arc<IrohBlobStore>>,
    content_discovery: &Option<crate::content_discovery::ContentDiscoveryService>,
    shard_nodes: &HashMap<ShardId, Arc<RaftNode>>,
) {
    if content_discovery.is_some() && blob_store.is_some() && shard_nodes.contains_key(&0) {
        let config_clone = config.clone();
        let blob_store_clone = blob_store.clone();
        let content_discovery_clone = content_discovery.clone();
        tokio::spawn(async move {
            super::blob_init::auto_announce_local_blobs(
                &config_clone,
                blob_store_clone.as_ref(),
                content_discovery_clone.as_ref(),
            )
            .await;
        });
    }
}
