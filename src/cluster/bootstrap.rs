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
use crate::cluster::gossip_discovery::GossipPeerDiscovery;
use crate::cluster::metadata::{MetadataStore, NodeMetadata, NodeStatus};
use crate::cluster::ticket::AspenClusterTicket;
use crate::cluster::{IrohEndpointConfig, IrohEndpointManager, NodeServerConfig, NodeServerHandle};
use crate::raft::network::IrpcRaftNetworkFactory;
use crate::raft::server::RaftRpcServer;
use crate::raft::storage::{
    InMemoryLogStore, RedbLogStore, RedbStateMachine, StateMachineStore, StorageBackend,
};
use crate::raft::types::{AppTypeConfig, NodeId};
use crate::raft::{RaftActor, RaftActorConfig, RaftActorMessage, StateMachineVariant};

/// Handle to a bootstrapped cluster node.
///
/// Contains all the resources needed to run and shutdown a node cleanly.
pub struct BootstrapHandle {
    /// Node configuration.
    pub config: ClusterBootstrapConfig,
    /// Metadata store for cluster nodes.
    pub metadata_store: Arc<MetadataStore>,
    /// Iroh endpoint manager.
    pub iroh_manager: Arc<IrohEndpointManager>,
    /// Ractor node server handle.
    pub node_server: NodeServerHandle,
    /// Raft actor reference.
    pub raft_actor: ActorRef<RaftActorMessage>,
    /// Raft actor join handle.
    pub raft_task: tokio::task::JoinHandle<()>,
    /// Raft core for direct access.
    pub raft_core: openraft::Raft<AppTypeConfig>,
    /// State machine variant (actual state machine used by Raft).
    pub state_machine: StateMachineVariant,
    /// IRPC server handle.
    pub rpc_server: RaftRpcServer,
    /// IRPC network factory for dynamic peer addition.
    pub network_factory: Arc<IrpcRaftNetworkFactory>,
    /// Gossip-based peer discovery (None if disabled).
    pub gossip_discovery: Option<GossipPeerDiscovery>,
}

impl BootstrapHandle {
    /// Gracefully shutdown the node.
    ///
    /// Shuts down components in reverse order of startup:
    /// 1. Gossip discovery (if enabled)
    /// 2. IRPC server
    /// 3. Iroh endpoint
    /// 4. Node server
    /// 5. Raft actor
    pub async fn shutdown(self) -> Result<()> {
        // Shutdown gossip discovery first
        if let Some(gossip_discovery) = self.gossip_discovery {
            info!("shutting down gossip discovery");
            gossip_discovery.shutdown().await?;
        }

        info!("shutting down IRPC server");
        self.rpc_server.shutdown().await?;

        info!("shutting down Iroh endpoint");
        self.iroh_manager.shutdown().await?;

        info!("shutting down node server");
        self.node_server.shutdown().await?;

        info!("shutting down Raft actor");
        self.raft_actor.stop(Some("bootstrap-shutdown".into()));
        self.raft_task.await?;

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
    let metadata_path = config.data_dir().join("metadata.redb");
    let metadata_store =
        Arc::new(MetadataStore::new(&metadata_path).context("failed to create metadata store")?);
    info!(
        path = %metadata_path.display(),
        "metadata store initialized"
    );

    // Create Iroh endpoint manager
    let iroh_manager = {
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
        Arc::new(manager)
    };

    // Parse peer addresses
    let peer_addrs =
        parse_peer_addresses(&config.peers).context("failed to parse peer addresses")?;
    info!(peer_count = peer_addrs.len(), "parsed peer addresses");

    // Create network factory for Raft
    let network_factory = Arc::new(IrpcRaftNetworkFactory::new(
        Arc::clone(&iroh_manager),
        peer_addrs,
    ));

    // Spawn gossip peer discovery if enabled
    let gossip_discovery = if config.iroh.enable_gossip {
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

        // Spawn gossip discovery with network factory for automatic peer connection
        let discovery = GossipPeerDiscovery::spawn(
            topic_id,
            config.node_id,
            &iroh_manager,
            Some(Arc::clone(&network_factory)),
        )
        .await
        .context("failed to spawn gossip peer discovery")?;
        info!("gossip peer discovery spawned with automatic peer connection");
        Some(discovery)
    } else {
        info!("gossip peer discovery disabled");
        None
    };

    // Launch NodeServer
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

    // Create Raft core and capture the actual state machine
    let (raft_core, state_machine_variant) = {
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
                info!("Using in-memory storage backend (non-durable)");
                let log_store = InMemoryLogStore::default();
                let state_machine_store = StateMachineStore::new();

                let raft = openraft::Raft::new(
                    config.node_id,
                    validated_config,
                    (*network_factory).clone(),
                    log_store,
                    state_machine_store.clone(),
                )
                .await
                .context("failed to initialize raft with in-memory storage")?;

                (raft, StateMachineVariant::InMemory(state_machine_store))
            }
            StorageBackend::Redb => {
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

                let log_store =
                    RedbLogStore::new(&log_path).context("failed to create redb log store")?;
                let state_machine = RedbStateMachine::new(&sm_path)
                    .context("failed to create redb state machine")?;

                let raft = openraft::Raft::new(
                    config.node_id,
                    validated_config,
                    (*network_factory).clone(),
                    log_store,
                    state_machine.clone(),
                )
                .await
                .context("failed to initialize raft with redb storage")?;

                (raft, StateMachineVariant::Redb(state_machine))
            }
        }
    };

    // Spawn Raft actor with the actual state machine
    let raft_actor_config = RaftActorConfig {
        node_id: config.node_id,
        raft: raft_core.clone(),
        state_machine: state_machine_variant.clone(),
    };
    let (raft_actor, raft_task) = Actor::spawn(
        Some(format!("raft-{}", config.node_id)),
        RaftActor,
        raft_actor_config,
    )
    .await
    .context("failed to spawn raft actor")?;

    info!(node_id = config.node_id, "raft actor spawned");

    // Spawn IRPC server for Raft RPC
    let rpc_server = RaftRpcServer::spawn(Arc::clone(&iroh_manager), raft_core.clone());
    info!("irpc server spawned for raft rpc");

    // Register node in metadata store
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

    Ok(BootstrapHandle {
        config,
        metadata_store,
        iroh_manager,
        node_server,
        raft_actor,
        raft_task,
        raft_core,
        state_machine: state_machine_variant,
        rpc_server,
        network_factory,
        gossip_discovery,
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

    // Merge overrides (typically CLI args)
    config.merge(overrides);

    // Validate final configuration
    config
        .validate()
        .context("configuration validation failed")?;

    Ok(config)
}

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
