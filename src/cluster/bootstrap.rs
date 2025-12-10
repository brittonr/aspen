//! Node bootstrap orchestration for Aspen clusters.
//!
//! Coordinates the startup sequence for cluster nodes, wiring together Raft consensus,
//! Iroh P2P networking, actor supervision, gossip discovery, and metadata storage.
//! This module is the primary entry point for launching Aspen nodes, handling both
//! initial cluster formation and joining existing clusters.
//!
//! # Key Components
//!
//! - `bootstrap_node`: Main orchestration function for node startup
//! - `BootstrapHandle`: Resource handle for graceful shutdown and monitoring
//! - `load_config`: Multi-layer configuration loading (env, TOML, CLI)
//! - Component initialization: Metadata store, Iroh endpoint, Raft actor, RPC server
//! - Supervision tree: Raft supervisor with health monitoring
//!
//! # Bootstrap Sequence
//!
//! 1. Load configuration from environment, TOML file, and CLI args
//! 2. Initialize metadata store (redb) for node registry
//! 3. Create Iroh P2P endpoint with optional ticket-based discovery
//! 4. Initialize storage backends (redb log + SQLite state machine)
//! 5. Start ractor_cluster node server for actor communication
//! 6. Create Raft actor with configured supervision
//! 7. Start gossip peer discovery (if enabled)
//! 8. Register RPC endpoints for Raft communication
//! 9. Return handle with all resources for coordinated shutdown
//!
//! # Tiger Style
//!
//! - Explicit types: u64 for node IDs, Duration for timeouts (portable)
//! - Resource management: Handle owns all resources with explicit Drop
//! - Error context: Anyhow errors with actionable messages for operators
//! - Configuration layers: Clear precedence (env < TOML < CLI)
//! - Fail fast: Initialization errors prevent partial startup
//! - Clean shutdown: All components gracefully stopped via handle
//!
//! # Example
//!
//! ```ignore
//! use aspen::cluster::bootstrap::bootstrap_node;
//! use aspen::cluster::config::ClusterBootstrapConfig;
//!
//! let config = ClusterBootstrapConfig {
//!     node_id: 1,
//!     data_dir: Some("./data/node-1".into()),
//!     raft_addr: "127.0.0.1:5301".parse()?,
//!     ..Default::default()
//! };
//!
//! let handle = bootstrap_node(config).await?;
//! // Node is running, handle can be used to monitor or shutdown
//! handle.shutdown().await?;
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use iroh::{EndpointAddr, EndpointId, SecretKey};
use openraft::Config as RaftConfig;
use ractor::{Actor, ActorRef};
use tracing::{info, instrument};

use crate::cluster::config::ClusterBootstrapConfig;
use crate::cluster::gossip_actor::{GossipActor, GossipActorArgs, GossipMessage};
use crate::cluster::metadata::{MetadataStore, NodeMetadata, NodeStatus};
use crate::cluster::ticket::AspenClusterTicket;
use crate::cluster::{IrohEndpointConfig, IrohEndpointManager, NodeServerConfig, NodeServerHandle};
use crate::raft::network::IrpcRaftNetworkFactory;
use crate::raft::server_actor::{RaftRpcServerActor, RaftRpcServerActorArgs, RaftRpcServerMessage};
use crate::raft::storage::{InMemoryLogStore, RedbLogStore, StateMachineStore, StorageBackend};
use crate::raft::storage_sqlite::SqliteStateMachine;
use crate::raft::supervision::{
    HealthMonitor, RaftSupervisor, SupervisorArguments, SupervisorMessage,
};
use crate::raft::types::{AppTypeConfig, NodeId};
use crate::raft::{RaftActorConfig, RaftActorMessage, StateMachineVariant};

/// Handle to a bootstrapped cluster node.
///
/// Contains all the resources needed to run and shutdown a node cleanly.
/// Uses actor references for RPC server and gossip discovery instead of raw handles.
pub struct BootstrapHandle {
    /// Node configuration.
    pub config: ClusterBootstrapConfig,
    /// Metadata store for cluster nodes.
    pub metadata_store: Arc<MetadataStore>,
    /// Iroh endpoint manager.
    pub iroh_manager: Arc<IrohEndpointManager>,
    /// Ractor node server handle.
    pub node_server: NodeServerHandle,
    /// Raft actor reference (managed by supervisor).
    pub raft_actor: ActorRef<RaftActorMessage>,
    /// Raft supervisor reference.
    pub raft_supervisor: ActorRef<SupervisorMessage>,
    /// Raft supervisor join handle.
    pub supervisor_task: tokio::task::JoinHandle<()>,
    /// Raft core for direct access.
    pub raft_core: openraft::Raft<AppTypeConfig>,
    /// State machine variant (actual state machine used by Raft).
    pub state_machine: StateMachineVariant,
    /// IRPC server actor reference.
    pub rpc_server_actor: ActorRef<RaftRpcServerMessage>,
    /// IRPC server actor join handle.
    pub rpc_server_task: tokio::task::JoinHandle<()>,
    /// IRPC network factory for dynamic peer addition.
    pub network_factory: Arc<IrpcRaftNetworkFactory>,
    /// Gossip discovery actor reference (None if disabled).
    pub gossip_actor: Option<ActorRef<GossipMessage>>,
    /// Gossip actor join handle (None if disabled).
    pub gossip_task: Option<tokio::task::JoinHandle<()>>,
    /// Health monitor for the RaftActor.
    pub health_monitor: Option<Arc<HealthMonitor>>,
}

impl BootstrapHandle {
    /// Gracefully shutdown the node.
    ///
    /// Shuts down components in reverse order of startup:
    /// 1. Gossip discovery actor (if enabled)
    /// 2. IRPC server actor
    /// 3. Iroh endpoint
    /// 4. Node server
    /// 5. Raft supervisor
    pub async fn shutdown(self) -> Result<()> {
        // Shutdown gossip discovery actor first
        if let Some(gossip_actor) = self.gossip_actor {
            info!("shutting down gossip discovery actor");
            // Send shutdown message to actor
            if let Err(e) = gossip_actor.cast(GossipMessage::Shutdown) {
                tracing::warn!(error = %e, "failed to send shutdown to gossip actor");
            }
        }
        // Wait for gossip actor task to complete
        if let Some(gossip_task) = self.gossip_task
            && let Err(e) = gossip_task.await
        {
            tracing::warn!(error = %e, "gossip actor task panicked during shutdown");
        }

        info!("shutting down IRPC server actor");
        // Send shutdown message to RPC server actor
        if let Err(e) = self.rpc_server_actor.cast(RaftRpcServerMessage::Shutdown) {
            tracing::warn!(error = %e, "failed to send shutdown to RPC server actor");
        }
        // Wait for RPC server actor task to complete
        if let Err(e) = self.rpc_server_task.await {
            tracing::warn!(error = %e, "RPC server actor task panicked during shutdown");
        }

        info!("shutting down Iroh endpoint");
        self.iroh_manager.shutdown().await?;

        info!("shutting down node server");
        self.node_server.shutdown().await?;

        info!("shutting down Raft supervisor");
        self.raft_supervisor.stop(Some("bootstrap-shutdown".into()));
        self.supervisor_task.await?;

        // Update node status to offline
        if let Err(err) = self
            .metadata_store
            .update_status(self.config.node_id, NodeStatus::Offline)
        {
            tracing::warn!(
                error = ?err,
                node_id = self.config.node_id,
                "failed to update node status to offline"
            );
        }

        Ok(())
    }
}

/// Parse peer address specifications from CLI args.
///
/// Parses peer specs in the format:
/// - "node_id@endpoint_id" (bare endpoint ID)
/// - "node_id@{json}" (full JSON EndpointAddr)
///
/// # Examples
///
/// ```text
/// "1@12D3KooWGdBx..."
/// "1@{\"node_id\":\"12D3KooWGdBx...\",\"relay_url\":\"https://relay.example.com\"}"
/// ```
///
/// Returns a HashMap mapping NodeId to EndpointAddr.
fn parse_peer_addresses(peer_specs: &[String]) -> Result<HashMap<NodeId, EndpointAddr>> {
    let mut peer_addrs = HashMap::new();

    for spec in peer_specs {
        let (node_id_str, addr_str) = spec.split_once('@').with_context(|| {
            format!("invalid peer spec '{spec}', expected format 'node_id@endpoint_addr'")
        })?;

        let node_id: NodeId = node_id_str
            .parse()
            .with_context(|| format!("invalid node_id '{node_id_str}' in peer spec '{spec}'"))?;

        // Try parsing as JSON first, then fall back to bare endpoint ID
        let endpoint_addr = if addr_str.starts_with('{') {
            serde_json::from_str::<EndpointAddr>(addr_str).with_context(|| {
                format!("failed to parse EndpointAddr JSON in peer spec '{spec}'")
            })?
        } else {
            let endpoint_id = EndpointId::from_str(addr_str).with_context(|| {
                format!("invalid endpoint_id '{addr_str}' in peer spec '{spec}'")
            })?;
            EndpointAddr::new(endpoint_id)
        };

        peer_addrs.insert(node_id, endpoint_addr);
    }

    Ok(peer_addrs)
}

/// Bootstrap a cluster node from configuration.
///
/// This function orchestrates the entire node startup process:
/// 1. Validates configuration
/// 2. Initializes metadata store
/// 3. Creates Iroh endpoint
/// 4. Spawns NodeServer
/// 5. Launches Raft actor
/// 6. Registers node in metadata store
///
/// Returns a `BootstrapHandle` that can be used to access node resources
/// and perform graceful shutdown.
#[instrument(skip(config), fields(node_id = config.node_id, storage_backend = ?config.storage_backend))]
pub async fn bootstrap_node(config: ClusterBootstrapConfig) -> Result<BootstrapHandle> {
    // Validate configuration
    config.validate().context("invalid configuration")?;

    info!(
        node_id = config.node_id,
        data_dir = %config.data_dir().display(),
        "bootstrapping node"
    );

    // Initialize metadata store
    let metadata_store = setup_metadata_store(&config)?;

    // Create Iroh endpoint manager
    let iroh_manager = setup_iroh_endpoint(&config).await?;

    // Parse peer addresses and create network factory
    let peer_addrs =
        parse_peer_addresses(&config.peers).context("failed to parse peer addresses")?;
    info!(peer_count = peer_addrs.len(), "parsed peer addresses");

    let network_factory = Arc::new(IrpcRaftNetworkFactory::new(
        Arc::clone(&iroh_manager),
        peer_addrs,
    ));

    // Spawn gossip actor if enabled
    let (gossip_actor, gossip_task) =
        setup_gossip_discovery(&config, &iroh_manager, &network_factory).await?;

    // Launch NodeServer
    let node_server = setup_node_server(&config).await?;

    // Create Raft storage backend and core
    let (raft_core, state_machine_variant, log_store_opt) =
        setup_raft_storage(&config, &network_factory).await?;

    // Spawn Raft supervisor and retrieve actor references
    let (raft_supervisor, supervisor_task, raft_actor, health_monitor) = spawn_raft_supervisor(
        &config,
        raft_core.clone(),
        state_machine_variant.clone(),
        log_store_opt,
    )
    .await?;

    // Spawn IRPC server actor for Raft RPC
    // Note: use_router=false means the actor will spawn its own accept loop
    // for handling Raft RPC connections. This is simpler than setting up an
    // Iroh Router with protocol handlers for ALPN dispatching.
    let rpc_server_args = RaftRpcServerActorArgs {
        endpoint_manager: Arc::clone(&iroh_manager),
        raft_core: raft_core.clone(),
        use_router: false,
    };
    let (rpc_server_actor, rpc_server_task) = Actor::spawn(
        Some(format!("raft-rpc-server-{}", config.node_id)),
        RaftRpcServerActor,
        rpc_server_args,
    )
    .await
    .context("failed to spawn Raft RPC server actor")?;
    info!("irpc server actor spawned for raft rpc (using internal accept loop)");

    // Register node in metadata store
    register_node_metadata(&config, &metadata_store, &iroh_manager)?;

    Ok(BootstrapHandle {
        config,
        metadata_store,
        iroh_manager,
        node_server,
        raft_actor,
        raft_supervisor,
        supervisor_task,
        raft_core,
        state_machine: state_machine_variant,
        rpc_server_actor,
        rpc_server_task,
        network_factory,
        gossip_actor,
        gossip_task,
        health_monitor,
    })
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
    toml_path: Option<&Path>,
    overrides: ClusterBootstrapConfig,
) -> Result<ClusterBootstrapConfig> {
    // Start with environment variables
    let mut config = ClusterBootstrapConfig::from_env();

    // Merge TOML file if provided
    if let Some(path) = toml_path {
        let toml_config = ClusterBootstrapConfig::from_toml_file(path)
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

// Helper functions placed after tests for better readability (high-level code first)
#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cluster::config::ControlBackend;
    use tempfile::TempDir;

    #[test]
    fn test_load_config_precedence() {
        let temp_dir = TempDir::new().unwrap();
        let toml_path = temp_dir.path().join("config.toml");

        // Create a TOML config file
        let toml_content = r#"
            node_id = 1
            ractor_port = 26001
            heartbeat_interval_ms = 1000
        "#;
        std::fs::write(&toml_path, toml_content).unwrap();

        // CLI overrides
        let cli_config = ClusterBootstrapConfig {
            node_id: 2, // This should override TOML
            data_dir: None,
            host: "127.0.0.1".into(),
            ractor_port: 26002, // This should override TOML
            cookie: "test-cookie".into(),
            http_addr: "127.0.0.1:8080".parse().unwrap(),
            control_backend: ControlBackend::RaftActor,
            heartbeat_interval_ms: 500, // Default, should be overridden by TOML
            election_timeout_min_ms: 1500,
            election_timeout_max_ms: 3000,
            iroh: crate::cluster::config::IrohConfig::default(),
            peers: vec![],
            storage_backend: crate::raft::storage::StorageBackend::default(),
            redb_log_path: None,
            redb_sm_path: None,
            sqlite_log_path: None,
            sqlite_sm_path: None,
            supervision_config: crate::raft::supervision::SupervisionConfig::default(),
            raft_mailbox_capacity: 1000,
        };

        let config = load_config(Some(&toml_path), cli_config).unwrap();

        // CLI should win for node_id and ractor_port
        assert_eq!(config.node_id, 2);
        assert_eq!(config.ractor_port, 26002);

        // TOML should override default heartbeat_interval_ms
        assert_eq!(config.heartbeat_interval_ms, 1000);
    }

    #[test]
    fn test_parse_peer_addresses_bare_endpoint_id() {
        // Generate valid endpoint IDs for testing
        fn generate_endpoint_id(node_id: u64) -> EndpointId {
            let mut seed = [0u8; 32];
            seed[..8].copy_from_slice(&node_id.to_le_bytes());
            SecretKey::from(seed).public()
        }

        let id1 = generate_endpoint_id(1);
        let id2 = generate_endpoint_id(2);

        let peers = vec![format!("1@{}", id1), format!("2@{}", id2)];

        let result = parse_peer_addresses(&peers);
        assert!(result.is_ok());

        let addrs = result.unwrap();
        assert_eq!(addrs.len(), 2);
        assert!(addrs.contains_key(&1));
        assert!(addrs.contains_key(&2));
    }

    #[test]
    fn test_parse_peer_addresses_json_format() {
        // Generate valid endpoint ID for testing
        fn generate_endpoint_id(node_id: u64) -> EndpointId {
            let mut seed = [0u8; 32];
            seed[..8].copy_from_slice(&node_id.to_le_bytes());
            SecretKey::from(seed).public()
        }

        let id1 = generate_endpoint_id(1);
        let endpoint_addr = EndpointAddr::new(id1);

        // Serialize to get the actual JSON format
        let actual_json = serde_json::to_string(&endpoint_addr).unwrap();

        let peers = vec![format!("1@{}", actual_json)];

        let result = parse_peer_addresses(&peers);
        assert!(result.is_ok());

        let addrs = result.unwrap();
        assert_eq!(addrs.len(), 1);
        assert!(addrs.contains_key(&1));
    }

    #[test]
    fn test_parse_peer_addresses_invalid_format() {
        // Missing '@' separator
        let peers = vec!["1-invalid".to_string()];
        let result = parse_peer_addresses(&peers);
        assert!(result.is_err());

        // Invalid node_id
        let peers =
            vec!["not_a_number@12D3KooWGdBx9YQp3LTZKHKqPmx1ypXwSWxrRqjATd8EgwPPjE9F".to_string()];
        let result = parse_peer_addresses(&peers);
        assert!(result.is_err());

        // Invalid endpoint_id
        let peers = vec!["1@invalid_endpoint_id".to_string()];
        let result = parse_peer_addresses(&peers);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_peer_addresses_empty() {
        let peers: Vec<String> = vec![];
        let result = parse_peer_addresses(&peers);
        assert!(result.is_ok());

        let addrs = result.unwrap();
        assert_eq!(addrs.len(), 0);
    }
}

// Helper functions for bootstrap_node (Tiger Style: high-level before low-level)

/// Initialize the metadata store for the cluster node.
///
/// Creates a redb-backed metadata store at `<data_dir>/metadata.redb`.
fn setup_metadata_store(config: &ClusterBootstrapConfig) -> Result<Arc<MetadataStore>> {
    let metadata_path = config.data_dir().join("metadata.redb");
    let metadata_store =
        Arc::new(MetadataStore::new(&metadata_path).context("failed to create metadata store")?);
    info!(
        path = %metadata_path.display(),
        "metadata store initialized"
    );
    Ok(metadata_store)
}

/// Create and configure the Iroh P2P endpoint manager.
///
/// Configures the endpoint with:
/// - Secret key (if provided)
/// - Relay URL (if provided)
/// - Gossip protocol
/// - Discovery services (mDNS, DNS, pkarr)
async fn setup_iroh_endpoint(config: &ClusterBootstrapConfig) -> Result<Arc<IrohEndpointManager>> {
    let mut iroh_config = IrohEndpointConfig::new();

    // Parse secret key if provided
    if let Some(key_hex) = &config.iroh.secret_key {
        let key_bytes = hex::decode(key_hex).context("invalid hex in iroh secret key")?;
        if key_bytes.len() != 32 {
            anyhow::bail!("iroh secret key must be 32 bytes (64 hex characters)");
        }
        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(&key_bytes);
        let key = SecretKey::from_bytes(&key_array);
        iroh_config = iroh_config.with_secret_key(key);
    }

    // Parse relay URL if provided
    if let Some(relay_url_str) = &config.iroh.relay_url {
        let relay_url = relay_url_str.parse().context("invalid iroh relay URL")?;
        iroh_config = iroh_config.with_relay_url(relay_url)?;
    }

    // Configure gossip if enabled
    iroh_config = iroh_config.with_gossip(config.iroh.enable_gossip);

    // Configure discovery services
    iroh_config = iroh_config.with_mdns(config.iroh.enable_mdns);
    iroh_config = iroh_config.with_dns_discovery(config.iroh.enable_dns_discovery);
    if let Some(ref dns_url) = config.iroh.dns_discovery_url {
        iroh_config = iroh_config.with_dns_discovery_url(dns_url.clone());
    }
    iroh_config = iroh_config.with_pkarr(config.iroh.enable_pkarr);
    if let Some(ref pkarr_url) = config.iroh.pkarr_relay_url {
        iroh_config = iroh_config.with_pkarr_relay_url(pkarr_url.clone());
    }

    // Configure ALPNs for the endpoint.
    // Since we use use_router=false in the RPC server actor, we must configure
    // ALPNs here so the endpoint knows what protocols to accept.
    // The raft-rpc ALPN is required for Raft RPC communication.
    use crate::protocol_handlers::RAFT_ALPN;
    iroh_config = iroh_config.with_alpn(RAFT_ALPN.to_vec());
    info!(alpn = ?String::from_utf8_lossy(RAFT_ALPN), "configured raft-rpc ALPN");

    let manager = IrohEndpointManager::new(iroh_config)
        .await
        .context("failed to create iroh endpoint manager")?;
    let endpoint_id = manager.endpoint().id();
    info!(
        endpoint_id = %endpoint_id,
        node_addr = ?manager.node_addr(),
        gossip_enabled = config.iroh.enable_gossip,
        "iroh endpoint created"
    );
    Ok(Arc::new(manager))
}

/// Set up gossip-based peer discovery actor if enabled.
///
/// Determines the gossip topic ID from either:
/// 1. Cluster ticket (if provided)
/// 2. Cluster cookie (derived using blake3)
///
/// Returns (None, None) if gossip is disabled.
async fn setup_gossip_discovery(
    config: &ClusterBootstrapConfig,
    iroh_manager: &Arc<IrohEndpointManager>,
    network_factory: &Arc<IrpcRaftNetworkFactory>,
) -> Result<(
    Option<ActorRef<GossipMessage>>,
    Option<tokio::task::JoinHandle<()>>,
)> {
    if !config.iroh.enable_gossip {
        info!("gossip peer discovery disabled");
        return Ok((None, None));
    }

    use iroh_gossip::proto::TopicId;

    // Determine topic ID: from ticket or derive from cookie
    let topic_id = if let Some(ticket_str) = &config.iroh.gossip_ticket {
        info!("parsing cluster ticket for gossip topic");
        let ticket = AspenClusterTicket::deserialize(ticket_str)
            .context("failed to parse cluster ticket")?;
        info!(
            topic_id = ?ticket.topic_id,
            cluster_id = %ticket.cluster_id,
            bootstrap_peers = ticket.bootstrap.len(),
            "cluster ticket parsed"
        );
        ticket.topic_id
    } else {
        // Derive topic ID from cluster cookie using blake3
        let hash = blake3::hash(config.cookie.as_bytes());
        let topic_id = TopicId::from_bytes(*hash.as_bytes());
        info!(
            topic_id = ?topic_id,
            "derived gossip topic from cluster cookie"
        );
        topic_id
    };

    // Spawn gossip actor with network factory for automatic peer connection
    let gossip_args = GossipActorArgs {
        topic_id,
        node_id: config.node_id,
        endpoint_manager: Arc::clone(iroh_manager),
        network_factory: Some(Arc::clone(network_factory)),
    };
    let (gossip_actor, gossip_task) = Actor::spawn(
        Some(format!("gossip-actor-{}", config.node_id)),
        GossipActor,
        gossip_args,
    )
    .await
    .context("failed to spawn gossip actor")?;
    info!("gossip actor spawned with automatic peer connection");
    Ok((Some(gossip_actor), Some(gossip_task)))
}

/// Launch the ractor NodeServer for distributed actor communication.
///
/// The NodeServer provides the transport layer for ractor_cluster.
async fn setup_node_server(config: &ClusterBootstrapConfig) -> Result<NodeServerHandle> {
    let node_server = NodeServerConfig::new(
        format!("node-{}", config.node_id),
        config.host.clone(),
        config.ractor_port,
        config.cookie.clone(),
    )
    .launch()
    .await
    .context("failed to launch node server")?;
    info!(
        label = %node_server.label(),
        addr = %node_server.addr(),
        "node server online"
    );
    Ok(node_server)
}

/// Create Raft storage backend and initialize the Raft core.
///
/// Supports three storage backends:
/// - InMemory: Non-durable, for testing
/// - Redb: Persistent redb log + SQLite state machine (deprecated)
/// - Sqlite: Redb log + SQLite state machine (default)
///
/// Returns (Raft core, state machine variant, optional log store).
async fn setup_raft_storage(
    config: &ClusterBootstrapConfig,
    network_factory: &Arc<IrpcRaftNetworkFactory>,
) -> Result<(
    openraft::Raft<AppTypeConfig>,
    StateMachineVariant,
    Option<RedbLogStore>,
)> {
    let raft_config = RaftConfig {
        heartbeat_interval: config.heartbeat_interval_ms,
        election_timeout_min: config.election_timeout_min_ms,
        election_timeout_max: config.election_timeout_max_ms,
        ..Default::default()
    };
    let validated_config = Arc::new(
        raft_config
            .validate()
            .context("invalid raft configuration")?,
    );

    // Select storage backend based on configuration
    match config.storage_backend {
        StorageBackend::InMemory => {
            create_inmemory_storage(config.node_id, validated_config, network_factory).await
        }
        #[allow(deprecated)]
        StorageBackend::Redb => {
            create_redb_storage(config, validated_config, network_factory).await
        }
        StorageBackend::Sqlite => {
            create_sqlite_storage(config, validated_config, network_factory).await
        }
    }
}

/// Spawn the Raft supervisor and retrieve actor references.
///
/// The supervisor manages the RaftActor lifecycle and provides:
/// - Automatic actor restart on failure
/// - Health monitoring
/// - Graceful shutdown coordination
///
/// Returns (supervisor, supervisor task, raft actor, health monitor).
async fn spawn_raft_supervisor(
    config: &ClusterBootstrapConfig,
    raft_core: openraft::Raft<AppTypeConfig>,
    state_machine: StateMachineVariant,
    log_store_opt: Option<RedbLogStore>,
) -> Result<(
    ActorRef<SupervisorMessage>,
    tokio::task::JoinHandle<()>,
    ActorRef<RaftActorMessage>,
    Option<Arc<HealthMonitor>>,
)> {
    let raft_actor_config = RaftActorConfig {
        node_id: config.node_id,
        raft: raft_core,
        state_machine,
        log_store: log_store_opt,
    };

    let supervisor_args = SupervisorArguments {
        raft_actor_config,
        supervision_config: config.supervision_config.clone(),
    };

    let (raft_supervisor, supervisor_task) = Actor::spawn(
        Some(format!("raft-supervisor-{}", config.node_id)),
        RaftSupervisor,
        supervisor_args,
    )
    .await
    .context("failed to spawn raft supervisor")?;

    info!(
        node_id = config.node_id,
        "raft supervisor spawned (will spawn raft actor)"
    );

    // Retrieve the RaftActor reference from the supervisor
    use ractor::call_t;
    let raft_actor = call_t!(raft_supervisor, SupervisorMessage::GetRaftActor, 100)
        .context("failed to get raft actor from supervisor")?
        .context("raft actor not yet spawned by supervisor")?;

    // Retrieve the HealthMonitor reference from the supervisor
    let health_monitor = call_t!(raft_supervisor, SupervisorMessage::GetHealthMonitor, 100)
        .context("failed to get health monitor from supervisor")?;

    info!(
        node_id = config.node_id,
        health_monitor_available = health_monitor.is_some(),
        "health monitor retrieved from supervisor"
    );

    Ok((raft_supervisor, supervisor_task, raft_actor, health_monitor))
}

/// Register the node in the metadata store.
///
/// Records node information including:
/// - Node ID
/// - Endpoint ID (Iroh P2P address)
/// - Raft address (host:port)
/// - Online status
/// - Last updated timestamp
fn register_node_metadata(
    config: &ClusterBootstrapConfig,
    metadata_store: &Arc<MetadataStore>,
    iroh_manager: &Arc<IrohEndpointManager>,
) -> Result<()> {
    let last_updated_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("system time is set before Unix epoch (1970-01-01)")?
        .as_secs();

    let node_metadata = NodeMetadata {
        node_id: config.node_id,
        endpoint_id: iroh_manager.endpoint().id().to_string(),
        raft_addr: format!("{}:{}", config.host, config.ractor_port),
        status: NodeStatus::Online,
        last_updated_secs,
    };

    metadata_store
        .register_node(node_metadata)
        .context("failed to register node in metadata store")?;

    info!(
        node_id = config.node_id,
        endpoint_id = %iroh_manager.endpoint().id(),
        "node registered in metadata store"
    );

    Ok(())
}

/// Create in-memory storage backend for Raft (non-durable).
///
/// Used for testing and development. All data is lost on restart.
async fn create_inmemory_storage(
    node_id: NodeId,
    validated_config: Arc<RaftConfig>,
    network_factory: &Arc<IrpcRaftNetworkFactory>,
) -> Result<(
    openraft::Raft<AppTypeConfig>,
    StateMachineVariant,
    Option<RedbLogStore>,
)> {
    info!("Using in-memory storage backend (non-durable)");
    let log_store = InMemoryLogStore::default();
    let state_machine_store = StateMachineStore::new();

    let raft = openraft::Raft::new(
        node_id,
        validated_config,
        network_factory.as_ref().clone(),
        log_store,
        state_machine_store.clone(),
    )
    .await
    .context("failed to initialize raft with in-memory storage")?;

    Ok((
        raft,
        StateMachineVariant::InMemory(state_machine_store),
        None,
    ))
}

/// Create redb persistent storage backend for Raft (deprecated).
///
/// Uses redb for log storage and SQLite for state machine.
/// This backend is deprecated in favor of Sqlite.
async fn create_redb_storage(
    config: &ClusterBootstrapConfig,
    validated_config: Arc<RaftConfig>,
    network_factory: &Arc<IrpcRaftNetworkFactory>,
) -> Result<(
    openraft::Raft<AppTypeConfig>,
    StateMachineVariant,
    Option<RedbLogStore>,
)> {
    info!("Using redb persistent storage backend");

    // Determine paths for redb files
    let data_dir = config.data_dir();
    let log_path = config
        .redb_log_path
        .clone()
        .unwrap_or_else(|| data_dir.join("raft-log.redb"));
    let sm_path = config
        .redb_sm_path
        .clone()
        .unwrap_or_else(|| data_dir.join("state-machine.redb"));

    info!(
        log_path = %log_path.display(),
        sm_path = %sm_path.display(),
        "Initializing redb storage"
    );

    let log_store = RedbLogStore::new(&log_path).context("failed to create redb log store")?;
    let state_machine =
        SqliteStateMachine::new(&sm_path).context("failed to create sqlite state machine")?;

    let raft = openraft::Raft::new(
        config.node_id,
        validated_config,
        network_factory.as_ref().clone(),
        log_store,
        state_machine.clone(),
    )
    .await
    .context("failed to initialize raft with redb storage")?;

    Ok((raft, StateMachineVariant::Redb(state_machine), None))
}

/// Create sqlite persistent storage backend for Raft (default).
///
/// Uses redb for log storage and SQLite for state machine.
/// This is the recommended production storage backend.
async fn create_sqlite_storage(
    config: &ClusterBootstrapConfig,
    validated_config: Arc<RaftConfig>,
    network_factory: &Arc<IrpcRaftNetworkFactory>,
) -> Result<(
    openraft::Raft<AppTypeConfig>,
    StateMachineVariant,
    Option<RedbLogStore>,
)> {
    info!("Using sqlite persistent storage backend");

    // Determine paths for sqlite files
    let data_dir = config.data_dir();
    let log_path = config
        .sqlite_log_path
        .clone()
        .unwrap_or_else(|| data_dir.join("raft-log.db"));
    let sm_path = config
        .sqlite_sm_path
        .clone()
        .unwrap_or_else(|| data_dir.join("state-machine.db"));

    info!(
        log_path = %log_path.display(),
        sm_path = %sm_path.display(),
        "Initializing sqlite storage"
    );

    // For now, we're only migrating the state machine to SQLite
    // The log store still uses redb (will be migrated later if needed)
    let log_store = RedbLogStore::new(&log_path).context("failed to create redb log store")?;
    let state_machine =
        SqliteStateMachine::new(&sm_path).context("failed to create sqlite state machine")?;

    let raft = openraft::Raft::new(
        config.node_id,
        validated_config,
        network_factory.as_ref().clone(),
        log_store.clone(),
        state_machine.clone(),
    )
    .await
    .context("failed to initialize raft with sqlite storage")?;

    Ok((
        raft,
        StateMachineVariant::Sqlite(state_machine),
        Some(log_store),
    ))
}
