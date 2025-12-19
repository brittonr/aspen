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

use anyhow::{Context, Result, ensure};
use iroh::EndpointAddr;
use iroh_gossip::proto::TopicId;
use openraft::{Config as RaftConfig, Raft};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::blob::IrohBlobStore;
use crate::cluster::config::NodeConfig;
use crate::cluster::gossip_discovery::GossipPeerDiscovery;
use crate::cluster::metadata::{MetadataStore, NodeStatus};
use crate::cluster::ticket::AspenClusterTicket;
use crate::cluster::{IrohEndpointConfig, IrohEndpointManager};
use crate::raft::StateMachineVariant;
use crate::raft::log_subscriber::{LOG_BROADCAST_BUFFER_SIZE, LogEntryPayload};
use crate::raft::network::IrpcRaftNetworkFactory;
use crate::raft::node::{RaftNode, RaftNodeHealth};
use crate::raft::server::RaftRpcServer;
use crate::raft::storage::{InMemoryLogStore, InMemoryStateMachine, RedbLogStore, StorageBackend};
use crate::raft::storage_sqlite::SqliteStateMachine;
use crate::raft::supervisor::Supervisor;
use crate::raft::types::NodeId;

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
    pub peer_manager: Option<Arc<crate::docs::PeerManager>>,
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
    /// sync connections. None when docs P2P sync is disabled.
    pub docs_sync: Option<crate::docs::DocsSyncResources>,
}

impl NodeHandle {
    /// Gracefully shutdown the node.
    pub async fn shutdown(self) -> Result<()> {
        info!("shutting down node {}", self.config.node_id);

        // Signal shutdown to all components
        self.shutdown_token.cancel();

        // Stop gossip discovery if enabled
        if let Some(gossip_discovery) = self.gossip_discovery {
            info!("shutting down gossip discovery");
            if let Err(err) = gossip_discovery.shutdown().await {
                error!(error = ?err, "failed to shutdown gossip discovery gracefully");
            }
        }

        // Stop RPC server if present (only used when not using Router)
        if let Some(rpc_server) = self.rpc_server {
            info!("shutting down legacy RPC server");
            if let Err(err) = rpc_server.shutdown().await {
                error!(error = ?err, "failed to shutdown RPC server gracefully");
            }
        }

        // Shutdown peer manager if present
        if let Some(peer_manager) = &self.peer_manager {
            info!("shutting down peer manager");
            peer_manager.shutdown();
        }

        // Shutdown DocsExporter if present
        if let Some(cancel_token) = &self.docs_exporter_cancel {
            info!("shutting down DocsExporter");
            cancel_token.cancel();
        }

        // Shutdown docs sync if present
        // Note: SyncHandle shutdown is handled by dropping it
        if self.docs_sync.is_some() {
            info!("shutting down docs sync");
            drop(self.docs_sync);
        }

        // Stop supervisor
        info!("shutting down supervisor");
        self.supervisor.stop();

        // Shutdown blob store if present
        if let Some(blob_store) = &self.blob_store {
            info!("shutting down blob store");
            if let Err(err) = blob_store.shutdown().await {
                error!(error = ?err, "failed to shutdown blob store gracefully");
            }
        }

        // Shutdown Iroh endpoint
        info!("shutting down Iroh endpoint");
        self.iroh_manager.shutdown().await?;

        // Update node status
        if let Err(err) = self
            .metadata_store
            .update_status(self.config.node_id, NodeStatus::Offline)
        {
            error!(
                error = ?err,
                node_id = self.config.node_id,
                "failed to update node status to offline"
            );
        }

        Ok(())
    }
}

/// Bootstrap a cluster node with simplified architecture.
///
/// This replaces the actor-based bootstrap with direct async APIs,
/// removing the overhead of message passing while maintaining the
/// same functionality.
pub async fn bootstrap_node(config: NodeConfig) -> Result<NodeHandle> {
    info!(
        node_id = config.node_id,
        "bootstrapping node with simplified architecture"
    );

    // Initialize metadata store
    let data_dir = config
        .data_dir
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("data_dir must be set"))?;

    // Ensure data directory exists
    std::fs::create_dir_all(data_dir)
        .with_context(|| format!("failed to create data directory: {}", data_dir.display()))?;

    // MetadataStore expects a path to the database file, not directory
    let metadata_db_path = data_dir.join("metadata.redb");
    let metadata_store = Arc::new(MetadataStore::new(&metadata_db_path)?);

    // Create Iroh endpoint configuration
    // NOTE: Do NOT set ALPNs here! When using Router (which is the standard pattern),
    // ALPNs are registered automatically via Router::builder().accept(). Setting ALPNs
    // on the endpoint directly conflicts with Router-based ALPN handling.
    // The Router is spawned by the caller (aspen-node.rs or Node::spawn_router()).

    let iroh_config = IrohEndpointConfig::default()
        .with_gossip(config.iroh.enable_gossip)
        .with_mdns(config.iroh.enable_mdns)
        .with_dns_discovery(config.iroh.enable_dns_discovery)
        .with_pkarr(config.iroh.enable_pkarr);

    let iroh_config = if let Some(dns_url) = &config.iroh.dns_discovery_url {
        iroh_config.with_dns_discovery_url(dns_url.clone())
    } else {
        iroh_config
    };

    // Parse secret key if provided
    let iroh_config = if let Some(secret_key_hex) = &config.iroh.secret_key {
        let bytes = hex::decode(secret_key_hex).context("invalid secret key hex")?;
        let secret_key = iroh::SecretKey::from_bytes(
            &bytes
                .try_into()
                .map_err(|_| anyhow::anyhow!("invalid secret key length"))?,
        );
        iroh_config.with_secret_key(secret_key)
    } else {
        iroh_config
    };

    // Create Iroh endpoint manager
    let iroh_manager = Arc::new(IrohEndpointManager::new(iroh_config).await?);

    info!(
        node_id = config.node_id,
        endpoint_id = %iroh_manager.node_addr().id,
        "created Iroh endpoint"
    );

    // Parse peer addresses from config if provided
    let peer_addrs = parse_peer_addresses(&config.peers)?;

    // Create network factory
    let network_factory = Arc::new(IrpcRaftNetworkFactory::new(
        iroh_manager.clone(),
        peer_addrs,
    ));

    // Derive gossip topic ID - always computed for cluster ticket generation,
    // even when gossip discovery is disabled
    //
    // Tiger Style: Topic ID is derived deterministically from:
    // 1. Cluster ticket (if provided) - ensures joining nodes use same topic
    // 2. Cluster cookie hash (fallback) - deterministic for all nodes with same cookie
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
    //
    // Gossip discovery automatically:
    // 1. Subscribes to a topic derived from the cluster cookie (or ticket)
    // 2. Broadcasts this node's ID and EndpointAddr every 10 seconds
    // 3. Receives peer announcements and adds them to the network factory
    //
    // This enables automatic peer discovery without manual configuration.
    let gossip_discovery = if config.iroh.enable_gossip {
        info!(
            node_id = config.node_id,
            topic_id = %hex::encode(gossip_topic_id.as_bytes()),
            "starting gossip discovery"
        );

        match GossipPeerDiscovery::spawn(
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
                // Gossip failure is non-fatal - node can still work with manual peers
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
            "gossip discovery disabled by configuration (topic ID still available for tickets)"
        );
        None
    };

    // Create Raft config with custom timeouts from NodeConfig
    let raft_config = Arc::new(RaftConfig {
        cluster_name: config.cookie.clone(),
        heartbeat_interval: config.heartbeat_interval_ms,
        election_timeout_min: config.election_timeout_min_ms,
        election_timeout_max: config.election_timeout_max_ms,
        replication_lag_threshold: 10000,
        snapshot_policy: openraft::SnapshotPolicy::LogsSinceLast(100),
        max_in_snapshot_log_to_keep: 100,
        enable_tick: true, // Ensure automatic elections are enabled
        ..RaftConfig::default()
    });

    // Create broadcast channel for log entry notifications (when docs export is enabled)
    // This channel is used by DocsExporter to receive committed KV operations in real-time
    let log_broadcast: Option<broadcast::Sender<LogEntryPayload>> = if config.docs.enabled {
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

    // Build OpenRaft instance and state machine variant based on selected storage
    let (raft, state_machine_variant) = match config.storage_backend {
        StorageBackend::InMemory => {
            let log_store = Arc::new(InMemoryLogStore::default());
            let state_machine = InMemoryStateMachine::new();
            let raft = Arc::new(
                Raft::new(
                    config.node_id.into(),
                    raft_config.clone(),
                    network_factory.as_ref().clone(),
                    log_store.as_ref().clone(),
                    state_machine.clone(),
                )
                .await?,
            );
            (raft, StateMachineVariant::InMemory(state_machine))
        }
        StorageBackend::Sqlite => {
            let log_path = data_dir.join(format!("node_{}.db", config.node_id));
            let log_store = Arc::new(RedbLogStore::new(&log_path)?);

            let state_machine_path = data_dir.join(format!("node_{}_state.db", config.node_id));
            let sqlite_state_machine = SqliteStateMachine::new(&state_machine_path)?;

            // Wire up log broadcast channel to state machine if docs export is enabled
            let sqlite_state_machine = if let Some(ref sender) = log_broadcast {
                sqlite_state_machine.with_log_broadcast(sender.clone())
            } else {
                sqlite_state_machine
            };

            let raft = Arc::new(
                Raft::new(
                    config.node_id.into(),
                    raft_config.clone(),
                    network_factory.as_ref().clone(),
                    log_store.as_ref().clone(),
                    sqlite_state_machine.clone(),
                )
                .await?,
            );

            (raft, StateMachineVariant::Sqlite(sqlite_state_machine))
        }
    };

    info!(node_id = config.node_id, "created OpenRaft instance");

    // Create RaftNode (direct wrapper)
    let raft_node = Arc::new(RaftNode::new(
        config.node_id.into(),
        raft.clone(),
        state_machine_variant,
    ));

    // Create supervisor for tracking health failures
    let supervisor = Supervisor::new(format!("raft-node-{}", config.node_id));

    // Create health monitor with supervisor integration
    let health_monitor = Arc::new(RaftNodeHealth::new(raft_node.clone()));

    // Start health monitoring in background, wired to supervisor
    //
    // Tiger Style: Bounded channel (32 slots) with explicit drop tracking.
    // Health callbacks fire at 5-second intervals, so 32 slots provides ~2.5 minutes
    // of buffer before drops occur (which would indicate serious issues anyway).
    let health_clone = health_monitor.clone();
    let shutdown = CancellationToken::new();
    let shutdown_clone = shutdown.clone();
    let supervisor_for_health = supervisor.clone();
    let node_id_for_health = config.node_id;
    tokio::spawn(async move {
        // Create a channel to communicate failures from the sync callback to async supervisor
        // Tiger Style: Bounded channel size = 32 (allows ~2.5 min backlog at 5s intervals)
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(32);

        // Track dropped messages for observability using Arc for safe sharing
        let dropped_count = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let dropped_count_for_callback = Arc::clone(&dropped_count);

        // Spawn a task to process health failures and record them with supervisor
        let supervisor_clone = supervisor_for_health.clone();
        let node_id_for_processor = node_id_for_health;
        tokio::spawn(async move {
            while let Some(reason) = rx.recv().await {
                supervisor_clone.record_health_failure(&reason).await;

                // Check if we should give up (too many failures)
                if !supervisor_clone.should_attempt_recovery().await {
                    error!(
                        node_id = node_id_for_processor,
                        "too many health failures, supervisor circuit breaker triggered"
                    );
                    // Cancel the supervisor which will propagate shutdown
                    supervisor_clone.stop();
                    break;
                }
            }
            // Log channel closure for debugging
            warn!(
                node_id = node_id_for_processor,
                "health failure processor channel closed"
            );
        });

        tokio::select! {
            _ = health_clone.monitor_with_callback(5, move |status| {
                let reason = format!(
                    "health check failed: state={:?}, failures={}",
                    status.state, status.consecutive_failures
                );
                // Non-blocking send with explicit drop handling
                // Tiger Style: Log dropped messages for observability
                if let Err(tokio::sync::mpsc::error::TrySendError::Full(_)) = tx.try_send(reason) {
                    let count = dropped_count_for_callback
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    // Log every 10th drop to avoid log spam
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
                // Log final drop count on shutdown if any messages were dropped
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

    // NOTE: RPC server is NOT spawned here anymore.
    // When using Iroh Router (preferred), the RaftProtocolHandler handles Raft RPCs.
    // The Router is spawned in aspen-node.rs after bootstrap completes.
    // This eliminates the race condition where both RaftRpcServer and Router
    // compete for incoming connections on the same endpoint.
    //
    // For testing scenarios without Router, call RaftRpcServer::spawn() separately.
    info!(
        node_id = config.node_id,
        "bootstrap complete - Router should be spawned by caller for RPC handling"
    );

    // Initialize blob store if enabled
    let blob_store = if config.blobs.enabled {
        let blobs_dir = data_dir.join("blobs");
        std::fs::create_dir_all(&blobs_dir).with_context(|| {
            format!("failed to create blobs directory: {}", blobs_dir.display())
        })?;

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
                // Blob store failure is non-fatal - node can still work without blobs
                warn!(
                    error = ?err,
                    node_id = config.node_id,
                    "failed to initialize blob store, continuing without it"
                );
                None
            }
        }
    } else {
        info!(
            node_id = config.node_id,
            "blob store disabled by configuration"
        );
        None
    };

    // Initialize peer sync (DocsImporter + PeerManager) if enabled
    let peer_manager = if config.peer_sync.enabled {
        use crate::docs::{DocsImporter, PeerManager};

        // Create DocsImporter with raft_node as the KV store
        // DocsImporter uses the KV store to write imported entries
        let importer = Arc::new(DocsImporter::new(config.cookie.clone(), raft_node.clone()));

        // Create PeerManager which coordinates peer connections
        let manager = Arc::new(PeerManager::new(config.cookie.clone(), importer));

        info!(
            node_id = config.node_id,
            max_subscriptions = config.peer_sync.max_subscriptions,
            default_priority = config.peer_sync.default_priority,
            "peer sync initialized"
        );
        Some(manager)
    } else {
        info!(
            node_id = config.node_id,
            "peer sync disabled by configuration"
        );
        None
    };

    // Initialize DocsExporter and P2P sync for real-time KV export/sync if enabled
    // This spawns a background task that listens to the log broadcast channel
    // and exports committed KV operations to iroh-docs for CRDT-based sync
    let (docs_exporter_cancel, docs_sync) = if config.docs.enabled {
        use crate::docs::{
            BlobBackedDocsWriter, DocsExporter, DocsSyncResources, SyncHandleDocsWriter,
            init_docs_resources,
        };

        if let Some(ref sender) = log_broadcast {
            // Initialize iroh-docs resources (Store, NamespaceId, Author)
            match init_docs_resources(
                data_dir,
                config.docs.in_memory,
                config.docs.namespace_secret.as_deref(),
                config.docs.author_secret.as_deref(),
            ) {
                Ok(resources) => {
                    let namespace_id = resources.namespace_id;
                    let in_memory = config.docs.in_memory;

                    // Create DocsSyncResources for P2P sync (consumes Store, spawns SyncHandle)
                    let docs_sync = DocsSyncResources::from_docs_resources(
                        resources,
                        &format!("node-{}", config.node_id),
                    );

                    // Open the replica for reading/writing (required before using SyncHandleDocsWriter)
                    if let Err(err) = docs_sync.open_replica().await {
                        error!(
                            error = ?err,
                            node_id = config.node_id,
                            namespace_id = %namespace_id,
                            "failed to open docs replica"
                        );
                    }

                    // Create the appropriate DocsWriter implementation:
                    // - BlobBackedDocsWriter when blob_store is available (full P2P content transfer)
                    // - SyncHandleDocsWriter otherwise (metadata sync only)
                    let writer: Arc<dyn crate::docs::DocsWriter> = match &blob_store {
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
                                store.clone(),
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

                    // Create the exporter
                    let exporter = Arc::new(DocsExporter::new(writer));

                    // Subscribe to the broadcast channel
                    let receiver = sender.subscribe();

                    // Spawn the exporter background task
                    let cancel_token = exporter.spawn(receiver);

                    info!(
                        node_id = config.node_id,
                        namespace_id = %namespace_id,
                        in_memory = in_memory,
                        p2p_sync = true,
                        "DocsExporter started with P2P sync enabled"
                    );
                    (Some(cancel_token), Some(docs_sync))
                }
                Err(err) => {
                    // Docs initialization failure is non-fatal - node can still work without docs
                    error!(
                        error = ?err,
                        node_id = config.node_id,
                        "failed to initialize iroh-docs resources, continuing without docs export"
                    );
                    (None, None)
                }
            }
        } else {
            warn!(
                node_id = config.node_id,
                "DocsExporter not started - log broadcast channel not available \
                 (docs enabled but InMemory storage backend doesn't support broadcast yet)"
            );
            (None, None)
        }
    } else {
        info!(
            node_id = config.node_id,
            "DocsExporter disabled by configuration"
        );
        (None, None)
    };

    // Register node in metadata store
    use crate::cluster::metadata::NodeMetadata;
    metadata_store.register_node(NodeMetadata {
        node_id: config.node_id,
        endpoint_id: iroh_manager.node_addr().id.to_string(),
        raft_addr: String::new(), // No separate raft_addr in simplified version
        status: NodeStatus::Online,
        last_updated_secs: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    })?;

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
        peer_manager,
        log_broadcast,
        docs_exporter_cancel,
        docs_sync,
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
pub fn load_config(
    toml_path: Option<&std::path::Path>,
    overrides: NodeConfig,
) -> Result<NodeConfig> {
    // Start with environment variables
    let mut config = NodeConfig::from_env();

    // Merge TOML file if provided
    if let Some(path) = toml_path {
        let toml_config = NodeConfig::from_toml_file(path)
            .with_context(|| format!("failed to load config from {}", path.display()))?;
        config.merge(toml_config);
    }

    // Merge overrides (typically from CLI args)
    config.merge(overrides);

    // Validate final configuration
    config
        .validate()
        .context("configuration validation failed")?;

    Ok(config)
}

/// Parse peer addresses from CLI arguments.
fn parse_peer_addresses(peer_specs: &[String]) -> Result<HashMap<NodeId, EndpointAddr>> {
    let mut peers = HashMap::new();

    for spec in peer_specs {
        let parts: Vec<&str> = spec.split('@').collect();
        ensure!(
            parts.len() == 2,
            "invalid peer spec '{}', expected format: node_id@endpoint_id",
            spec
        );

        let node_id: u64 = parts[0]
            .parse()
            .context(format!("invalid node_id in peer spec '{}'", spec))?;

        // Parse endpoint address (could be full JSON or just ID)
        let addr = if parts[1].starts_with('{') {
            serde_json::from_str(parts[1])
                .context(format!("invalid JSON endpoint in peer spec '{}'", spec))?
        } else {
            // Parse as bare endpoint ID
            let endpoint_id = parts[1]
                .parse()
                .context(format!("invalid endpoint_id in peer spec '{}'", spec))?;
            EndpointAddr::new(endpoint_id)
        };

        peers.insert(node_id.into(), addr);
    }

    Ok(peers)
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
        use crate::cluster::config::IrohConfig;

        let overrides = NodeConfig {
            node_id: 1,
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
        let result = load_config(
            Some(std::path::Path::new("/nonexistent/config.toml")),
            overrides,
        );
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
        }
    }
}
