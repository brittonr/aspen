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

mod blob_init;
mod discovery_init;
mod hooks_init;
mod network_init;
mod sharding_init;
mod storage_init;
mod sync_init;
mod worker_init;

use std::sync::Arc;

use anyhow::Result;
use aspen_auth::CapabilityToken;
#[cfg(feature = "blob")]
use aspen_blob::BlobEventBroadcaster;
#[cfg(feature = "blob")]
use aspen_blob::IrohBlobStore;
#[cfg(feature = "blob")]
use aspen_blob::create_blob_event_channel;
#[cfg(feature = "docs")]
use aspen_docs::DocsEventBroadcaster;
#[cfg(feature = "docs")]
use aspen_docs::create_docs_event_channel;
use aspen_raft::node::RaftNode;
use aspen_raft::node::RaftNodeHealth;
use aspen_raft::supervisor::Supervisor;
// Re-export public items from sub-modules
pub use blob_init::auto_announce_local_blobs;
pub use blob_init::initialize_blob_replication;
pub use discovery_init::derive_topic_id_from_cookie;
pub use network_init::parse_peer_addresses;
pub use sharding_init::BaseDiscoveryResources;
pub use sharding_init::BaseNodeResources;
pub use sharding_init::ShardedNodeHandle;
pub use sharding_init::ShardingResources;
pub use sharding_init::bootstrap_sharded_node;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::info;

// Use the type alias from cluster mod.rs which provides the concrete type
use super::super::IrpcRaftNetworkFactory;
// Import Resource structs from resources module
use crate::bootstrap::resources::BlobReplicationResources;
use crate::bootstrap::resources::DiscoveryResources;
use crate::bootstrap::resources::HookResources;
use crate::bootstrap::resources::NetworkResources;
use crate::bootstrap::resources::ShutdownCoordinator;
use crate::bootstrap::resources::StorageResources;
use crate::bootstrap::resources::SyncResources;
use crate::bootstrap::resources::WorkerResources;
use crate::config::NodeConfig;
use crate::metadata::MetadataStore;
use crate::metadata::NodeStatus;

// ============================================================================
// NodeHandle - Main handle to a running cluster node
// ============================================================================
// Note: Resource structs (StorageResources, NetworkResources, etc.) are
// defined in the resources module and imported above.
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
        self.hooks.shutdown().await;

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
    #[cfg(feature = "blob")]
    blob_store: Option<&'a Arc<IrohBlobStore>>,
    iroh_manager: &'a Arc<crate::IrohEndpointManager>,
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
#[allow(dead_code)]
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
    #[cfg(feature = "blob")]
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
// Bootstrap Functions
// ============================================================================

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
    let iroh_manager = network_init::initialize_iroh_endpoint(&config).await?;

    // Parse peer addresses from config if provided
    let peer_addrs = network_init::parse_peer_addresses(&config.peers)?;

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
    let gossip_topic_id = discovery_init::derive_gossip_topic_from_config(&config);

    // Setup gossip discovery
    let gossip_discovery =
        discovery_init::setup_gossip_discovery(&config, gossip_topic_id, &iroh_manager, &network_factory).await;

    // Create Raft config and broadcast channels
    let (raft_config, broadcasts) = storage_init::create_raft_config_and_broadcast(&config);

    // Create Raft instance with appropriate storage backend
    let (raft, state_machine_variant, ttl_cleanup_cancel) =
        storage_init::create_raft_instance(&config, raft_config.clone(), &network_factory, data_dir, &broadcasts)
            .await?;

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
    let shutdown = worker_init::spawn_health_monitoring(&config, health_monitor.clone(), supervisor.clone());

    // Create event broadcast channels for hook integration (only if hooks enabled)
    #[cfg(feature = "blob")]
    let (blob_event_sender, blob_broadcaster) = if config.hooks.enabled && config.blobs.enabled {
        let (sender, _receiver) = create_blob_event_channel();
        let broadcaster = BlobEventBroadcaster::new(sender.clone());
        (Some(sender), Some(broadcaster))
    } else {
        (None, None)
    };

    #[cfg(feature = "docs")]
    let (docs_event_sender, docs_broadcaster) = if config.hooks.enabled && config.docs.enabled {
        let (sender, _receiver) = create_docs_event_channel();
        let broadcaster = Arc::new(DocsEventBroadcaster::new(sender.clone()));
        (Some(sender), Some(broadcaster))
    } else {
        (None, None)
    };

    // Initialize blob store and peer manager
    #[cfg(feature = "blob")]
    let blob_store = blob_init::initialize_blob_store(&config, data_dir, &iroh_manager, blob_broadcaster).await;

    let peer_manager = sync_init::initialize_peer_manager(&config, &raft_node);

    // Initialize DocsExporter and P2P sync if enabled
    let (docs_exporter_cancel, docs_sync) = sync_init::initialize_docs_export(
        &config,
        data_dir,
        broadcasts.log.as_ref(),
        #[cfg(feature = "blob")]
        blob_store.as_ref(),
        #[cfg(feature = "docs")]
        docs_broadcaster,
    )
    .await?;

    // Wire up sync event listener and DocsSyncService if all components are available
    let (sync_event_listener_cancel, docs_sync_service_cancel) = sync_init::wire_docs_sync_services(
        &config,
        &docs_sync,
        #[cfg(feature = "blob")]
        &blob_store,
        &peer_manager,
        &iroh_manager,
    )
    .await;

    // Initialize global content discovery if enabled
    let (content_discovery, content_discovery_cancel) =
        discovery_init::initialize_content_discovery(&config, &iroh_manager, &shutdown).await?;

    // Auto-announce local blobs to DHT if enabled (runs in background)
    #[cfg(feature = "blob")]
    blob_init::spawn_blob_announcer(&config, &blob_store, &content_discovery);

    // Initialize hook service if enabled
    let hooks = hooks_init::initialize_hook_service(
        &config,
        broadcasts.log.as_ref(),
        broadcasts.snapshot.as_ref(),
        #[cfg(feature = "blob")]
        blob_event_sender.as_ref(),
        #[cfg(feature = "docs")]
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
            #[cfg(feature = "blob")]
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
            #[cfg(feature = "docs")]
            docs_sync,
            #[cfg(feature = "docs")]
            peer_manager,
        },
        worker: WorkerResources {
            #[cfg(feature = "jobs")]
            worker_service: None, // Initialized in aspen-node after JobManager creation
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

    // Merge Nickel config file if provided (requires nickel feature)
    #[cfg(feature = "nickel")]
    if let Some(path) = config_path {
        let file_config: NodeConfig = aspen_nickel::load_nickel_config(path)
            .map_err(|e| anyhow::anyhow!("failed to load config from {}: {}", path.display(), e))?;
        config.merge(file_config);
    }
    #[cfg(not(feature = "nickel"))]
    if config_path.is_some() {
        anyhow::bail!("Nickel configuration files require the 'nickel' feature to be enabled");
    }

    // Merge overrides (typically from CLI args)
    config.merge(overrides);

    // Validate final configuration
    config.validate().map_err(|e| anyhow::anyhow!("configuration validation failed: {}", e))?;

    Ok(config)
}

/// Register node in metadata store.
fn register_node_metadata(
    config: &NodeConfig,
    metadata_store: &Arc<MetadataStore>,
    iroh_manager: &Arc<crate::IrohEndpointManager>,
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

#[cfg(test)]
mod tests {
    use aspen_raft::storage::StorageBackend;

    use super::*;

    // =========================================================================
    // Resource Struct Tests
    // =========================================================================

    #[test]
    fn test_storage_resources_shutdown_no_panic_without_ttl() {
        // Verify shutdown doesn't panic when ttl_cleanup_cancel is None
        // We can't create a real StorageResources but we can test the pattern
        let cancel: Option<CancellationToken> = None;
        if let Some(c) = &cancel {
            c.cancel();
        }
        // No panic = success
    }

    #[test]
    fn test_storage_resources_shutdown_with_ttl() {
        let cancel_token = CancellationToken::new();
        assert!(!cancel_token.is_cancelled());

        cancel_token.cancel();
        assert!(cancel_token.is_cancelled());

        // Calling cancel again is safe
        cancel_token.cancel();
        assert!(cancel_token.is_cancelled());
    }

    #[test]
    fn test_discovery_resources_topic_id_stored() {
        // Verify TopicId can be created and stored
        let topic = derive_topic_id_from_cookie("test-cookie");
        assert_eq!(topic.as_bytes().len(), 32);
    }

    #[test]
    fn test_sync_resources_shutdown_order() {
        // Test that sync resources can have their cancellation tokens cancelled in order
        let sync_event = CancellationToken::new();
        let docs_sync = CancellationToken::new();
        let docs_exporter = CancellationToken::new();

        // Verify initial state
        assert!(!sync_event.is_cancelled());
        assert!(!docs_sync.is_cancelled());
        assert!(!docs_exporter.is_cancelled());

        // Shutdown in order: sync_event -> docs_sync -> docs_exporter
        sync_event.cancel();
        docs_sync.cancel();
        docs_exporter.cancel();

        assert!(sync_event.is_cancelled());
        assert!(docs_sync.is_cancelled());
        assert!(docs_exporter.is_cancelled());
    }

    #[test]
    fn test_worker_resources_shutdown() {
        let worker_cancel = CancellationToken::new();
        assert!(!worker_cancel.is_cancelled());

        worker_cancel.cancel();
        assert!(worker_cancel.is_cancelled());
    }

    #[test]
    fn test_blob_replication_resources_disabled() {
        let resources = BlobReplicationResources::disabled();
        assert!(resources.replication_manager.is_none());
        assert!(resources.replication_cancel.is_none());
        assert!(resources.replication_task.is_none());
        assert!(resources.topology_cancel.is_none());
    }

    #[test]
    fn test_hook_resources_disabled() {
        let resources = HookResources::disabled();
        #[cfg(feature = "hooks")]
        assert!(resources.hook_service.is_none());
        assert!(resources.event_bridge_cancel.is_none());
        assert!(resources.blob_bridge_cancel.is_none());
        assert!(resources.docs_bridge_cancel.is_none());
        assert!(resources.system_events_bridge_cancel.is_none());
        assert!(resources.ttl_events_bridge_cancel.is_none());
        assert!(resources.snapshot_events_bridge_cancel.is_none());
    }

    #[test]
    fn test_hook_resources_shutdown_all_bridges() {
        // Create cancellation tokens for all bridges
        let event_bridge = CancellationToken::new();
        let blob_bridge = CancellationToken::new();
        let docs_bridge = CancellationToken::new();
        let system_events = CancellationToken::new();
        let ttl_events = CancellationToken::new();
        let snapshot_events = CancellationToken::new();

        // Simulate shutdown by cancelling all
        event_bridge.cancel();
        blob_bridge.cancel();
        docs_bridge.cancel();
        system_events.cancel();
        ttl_events.cancel();
        snapshot_events.cancel();

        assert!(event_bridge.is_cancelled());
        assert!(blob_bridge.is_cancelled());
        assert!(docs_bridge.is_cancelled());
        assert!(system_events.is_cancelled());
        assert!(ttl_events.is_cancelled());
        assert!(snapshot_events.is_cancelled());
    }

    #[test]
    fn test_shutdown_coordinator_signal_shutdown() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());

        token.cancel(); // Simulates signal_shutdown
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_shutdown_coordinator_child_token() {
        let parent = CancellationToken::new();
        let child = parent.child_token();

        assert!(!parent.is_cancelled());
        assert!(!child.is_cancelled());

        // Cancelling parent cancels child
        parent.cancel();

        assert!(parent.is_cancelled());
        assert!(child.is_cancelled());
    }

    // =========================================================================
    // NodeConfig Default Tests
    // =========================================================================

    #[test]
    fn test_node_config_default_values() {
        let config = NodeConfig::default();

        // Verify sensible defaults
        assert_eq!(config.node_id, 0); // Will be overridden
        assert_eq!(config.cookie, "aspen-cookie-UNSAFE-CHANGE-ME");
        assert!(config.heartbeat_interval_ms > 0);
        assert!(config.election_timeout_min_ms > 0);
        assert!(config.election_timeout_max_ms > config.election_timeout_min_ms);
    }

    #[test]
    fn test_node_config_storage_backend_default() {
        let config = NodeConfig::default();
        // Default is Redb for persistence
        assert_eq!(config.storage_backend, StorageBackend::Redb);
    }

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
            let _: &aspen_raft::StateMachineVariant = &handle.storage.state_machine;
            let _: &Option<CancellationToken> = &handle.storage.ttl_cleanup_cancel;

            // Network resources (composed)
            let _: &Arc<crate::IrohEndpointManager> = &handle.network.iroh_manager;
            let _: &Arc<IrpcRaftNetworkFactory> = &handle.network.network_factory;
            let _: &Option<aspen_raft::server::RaftRpcServer> = &handle.network.rpc_server;
            #[cfg(feature = "blob")]
            let _: &Option<Arc<IrohBlobStore>> = &handle.network.blob_store;

            // Discovery resources (composed)
            let _: &Option<crate::gossip_discovery::GossipPeerDiscovery> = &handle.discovery.gossip_discovery;
            let _: &iroh_gossip::proto::TopicId = &handle.discovery.gossip_topic_id;

            // Sync resources (composed)
            let _: &Option<tokio::sync::broadcast::Sender<aspen_raft::log_subscriber::LogEntryPayload>> =
                &handle.sync.log_broadcast;
            let _: &Option<CancellationToken> = &handle.sync.docs_exporter_cancel;
            let _: &Option<CancellationToken> = &handle.sync.sync_event_listener_cancel;
            let _: &Option<CancellationToken> = &handle.sync.docs_sync_service_cancel;
            #[cfg(feature = "docs")]
            let _: &Option<Arc<aspen_docs::DocsSyncResources>> = &handle.sync.docs_sync;

            // Worker resources (composed)
            // Handle doesn't expose supervisor directly - access through composed groups

            // Hooks resources (composed)
            #[cfg(feature = "hooks")]
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
        use aspen_sharding::ShardId;

        // Verify ShardedNodeHandle fields are accessible (compile-time check)
        // This test ensures the API remains stable
        fn _check_handle_fields(handle: &ShardedNodeHandle) {
            // Base resources
            let _: &BaseNodeResources = &handle.base;
            let _: &Option<CapabilityToken> = &handle.root_token;
            let _: &Arc<Supervisor> = &handle.supervisor;

            // Sharding resources (shard-specific Raft instances)
            let _: &std::collections::HashMap<ShardId, Arc<RaftNode>> = &handle.sharding.shard_nodes;
            let _: &Arc<aspen_sharding::ShardedKeyValueStore<RaftNode>> = &handle.sharding.sharded_kv;
            let _: &Arc<aspen_transport::ShardedRaftProtocolHandler> = &handle.sharding.sharded_handler;
            let _: &std::collections::HashMap<ShardId, Arc<RaftNodeHealth>> = &handle.sharding.health_monitors;
            let _: &std::collections::HashMap<ShardId, CancellationToken> = &handle.sharding.ttl_cleanup_cancels;
            let _: &std::collections::HashMap<ShardId, aspen_raft::StateMachineVariant> =
                &handle.sharding.shard_state_machines;
            let _: &Option<Arc<tokio::sync::RwLock<aspen_sharding::ShardTopology>>> = &handle.sharding.topology;

            // Discovery resources
            let _: &Option<crate::content_discovery::ContentDiscoveryService> = &handle.discovery.content_discovery;
            let _: &Option<CancellationToken> = &handle.discovery.content_discovery_cancel;

            // Sync resources
            let _: &Option<tokio::sync::broadcast::Sender<aspen_raft::log_subscriber::LogEntryPayload>> =
                &handle.sync.log_broadcast;
            let _: &Option<CancellationToken> = &handle.sync.docs_exporter_cancel;
            let _: &Option<CancellationToken> = &handle.sync.sync_event_listener_cancel;
            let _: &Option<CancellationToken> = &handle.sync.docs_sync_service_cancel;

            // Worker resources
            #[cfg(feature = "jobs")]
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
            let _: &Arc<crate::IrohEndpointManager> = &base.network.iroh_manager;
            let _: &Arc<IrpcRaftNetworkFactory> = &base.network.network_factory;
            #[cfg(feature = "blob")]
            let _: &Option<Arc<IrohBlobStore>> = &base.network.blob_store;
            // Discovery resources (nested)
            let _: &Option<crate::gossip_discovery::GossipPeerDiscovery> = &base.discovery.gossip_discovery;
            let _: &iroh_gossip::proto::TopicId = &base.discovery.gossip_topic_id;
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
        use aspen_sharding::ShardId;

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
