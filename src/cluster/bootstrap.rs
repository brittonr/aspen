use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use iroh::{EndpointAddr, SecretKey};
use openraft::Config as RaftConfig;
use ractor::{Actor, ActorRef};
use tracing::info;

use crate::cluster::config::{ClusterBootstrapConfig, ControlBackend};
use crate::cluster::metadata::{MetadataStore, NodeMetadata, NodeStatus};
use crate::cluster::{IrohEndpointConfig, IrohEndpointManager, NodeServerConfig, NodeServerHandle};
use crate::raft::network::IrpcRaftNetworkFactory;
use crate::raft::server::RaftRpcServer;
use crate::raft::storage::{InMemoryLogStore, StateMachineStore};
use crate::raft::types::{AppTypeConfig, NodeId};
use crate::raft::{RaftActor, RaftActorConfig, RaftActorMessage};

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
    /// State machine store.
    pub state_machine: Arc<StateMachineStore>,
    /// IRPC server handle.
    pub rpc_server: RaftRpcServer,
    /// IRPC network factory for dynamic peer addition.
    pub network_factory: Arc<IrpcRaftNetworkFactory>,
}

impl BootstrapHandle {
    /// Gracefully shutdown the node.
    ///
    /// Shuts down components in reverse order of startup:
    /// 1. IRPC server
    /// 2. Iroh endpoint
    /// 3. Node server
    /// 4. Raft actor
    pub async fn shutdown(self) -> Result<()> {
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
    let metadata_store = Arc::new(
        MetadataStore::new(&metadata_path).context("failed to create metadata store")?,
    );
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
            let relay_url = relay_url_str
                .parse()
                .context("invalid iroh relay URL")?;
            iroh_config = iroh_config.with_relay_url(relay_url)?;
        }

        let manager = IrohEndpointManager::new(iroh_config)
            .await
            .context("failed to create iroh endpoint manager")?;
        let endpoint_id = manager.endpoint().id();
        info!(
            endpoint_id = %endpoint_id,
            node_addr = ?manager.node_addr(),
            "iroh endpoint created"
        );
        Arc::new(manager)
    };

    // Parse peer addresses
    // Note: EndpointAddr doesn't implement FromStr, so we'll need manual construction
    // For now, we'll use an empty map and document this for future implementation
    let peer_addrs: HashMap<NodeId, EndpointAddr> = HashMap::new();

    if !config.peers.is_empty() {
        tracing::warn!(
            peer_count = config.peers.len(),
            "peer address parsing not yet implemented - EndpointAddr requires manual construction"
        );
    }

    info!(peer_count = peer_addrs.len(), "parsed peer addresses");

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

    // Create Raft core and state machine
    let (raft_core, state_machine_store, network_factory) = {
        let mut raft_config = RaftConfig::default();
        raft_config.heartbeat_interval = config.heartbeat_interval_ms;
        raft_config.election_timeout_min = config.election_timeout_min_ms;
        raft_config.election_timeout_max = config.election_timeout_max_ms;
        let validated_config = Arc::new(
            raft_config
                .validate()
                .context("invalid raft configuration")?,
        );

        let log_store = InMemoryLogStore::default();
        let state_machine_store = StateMachineStore::new();
        let network_factory = IrpcRaftNetworkFactory::new(Arc::clone(&iroh_manager), peer_addrs);

        let raft = openraft::Raft::new(
            config.node_id,
            validated_config,
            network_factory.clone(),
            log_store,
            state_machine_store.clone(),
        )
        .await
        .context("failed to initialize raft")?;

        (raft, state_machine_store, Arc::new(network_factory))
    };

    // Spawn Raft actor
    let raft_actor_config = RaftActorConfig {
        node_id: config.node_id,
        raft: raft_core.clone(),
        state_machine: state_machine_store.clone(),
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
    let node_metadata = NodeMetadata {
        node_id: config.node_id,
        endpoint_id: iroh_manager.endpoint().id().to_string(),
        raft_addr: format!("{}:{}", config.host, config.ractor_port),
        status: NodeStatus::Online,
        last_updated_secs: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_secs(),
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
        state_machine: state_machine_store,
        rpc_server,
        network_factory,
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
        };

        let config = load_config(Some(&toml_path), cli_config).unwrap();

        // CLI should win for node_id and ractor_port
        assert_eq!(config.node_id, 2);
        assert_eq!(config.ractor_port, 26002);

        // TOML should override default heartbeat_interval_ms
        assert_eq!(config.heartbeat_interval_ms, 1000);
    }
}
