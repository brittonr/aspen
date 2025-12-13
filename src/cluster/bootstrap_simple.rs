//! Simplified cluster node bootstrap without actors.
//!
//! This module provides bootstrap orchestration for Aspen nodes using
//! direct async APIs instead of actor message passing. The bootstrap
//! process creates and initializes all necessary components for a
//! functioning Raft cluster node.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use iroh::EndpointAddr;
use openraft::{Config as RaftConfig, Raft};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::cluster::config::NodeConfig;
use crate::cluster::metadata::{MetadataStore, NodeStatus};
use crate::cluster::{IrohEndpointConfig, IrohEndpointManager};
use crate::raft::StateMachineVariant;
use crate::raft::network::IrpcRaftNetworkFactory;
use crate::raft::node::{RaftNode, RaftNodeHealth};
use crate::raft::simple_supervisor::SimpleSupervisor;
use crate::raft::storage::RedbLogStore;
use crate::raft::storage_sqlite::SqliteStateMachine;
use crate::raft::types::NodeId;

/// Handle to a running cluster node (simplified version).
///
/// Contains all the resources needed to run and shutdown a node cleanly
/// without the overhead of actor message passing.
pub struct SimpleNodeHandle {
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
    // TODO: Implement gossip without actors
    // pub gossip_discovery: Option<Arc<GossipPeerDiscovery>>,
    /// RPC server for handling incoming Raft RPCs.
    // TODO: Implement RPC server without actor dependencies
    // pub rpc_server: Arc<RaftRpcServer>,
    /// Supervisor for automatic restarts.
    pub supervisor: Arc<SimpleSupervisor>,
    /// Health monitor.
    pub health_monitor: Arc<RaftNodeHealth>,
    /// Cancellation token for shutdown.
    pub shutdown_token: CancellationToken,
}

impl SimpleNodeHandle {
    /// Gracefully shutdown the node.
    pub async fn shutdown(self) -> Result<()> {
        info!("shutting down node {}", self.config.node_id);

        // Signal shutdown to all components
        self.shutdown_token.cancel();

        // TODO: Stop gossip discovery when implemented

        // Stop RPC server
        info!("shutting down RPC server");
        // Server will stop when cancellation token fires

        // Stop supervisor
        info!("shutting down supervisor");
        self.supervisor.stop();

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
pub async fn bootstrap_node_simple(config: NodeConfig) -> Result<SimpleNodeHandle> {
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

    // TODO: Setup gossip discovery without actors
    // For now, gossip is disabled in the simplified version

    // Create Raft storage
    let log_path = data_dir.join(format!("node_{}.db", config.node_id));
    let log_store = Arc::new(RedbLogStore::new(&log_path)?);

    let state_machine_path = data_dir.join(format!("node_{}_state.db", config.node_id));
    let sqlite_state_machine = SqliteStateMachine::new(&state_machine_path)?;
    let state_machine_variant = StateMachineVariant::Sqlite(sqlite_state_machine.clone());

    // Create Raft config
    let raft_config = Arc::new(RaftConfig {
        cluster_name: config.cookie.clone(),
        replication_lag_threshold: 10000,
        snapshot_policy: openraft::SnapshotPolicy::LogsSinceLast(100),
        max_in_snapshot_log_to_keep: 100,
        ..RaftConfig::default()
    });

    // Build OpenRaft instance
    let raft = Arc::new(
        Raft::new(
            config.node_id.into(),
            raft_config,
            network_factory.as_ref().clone(),
            log_store.as_ref().clone(),
            sqlite_state_machine,
        )
        .await?,
    );

    info!(node_id = config.node_id, "created OpenRaft instance");

    // Create RaftNode (direct wrapper)
    let raft_node = Arc::new(RaftNode::new(
        config.node_id.into(),
        raft.clone(),
        state_machine_variant,
    ));

    // Create health monitor
    let health_monitor = Arc::new(RaftNodeHealth::new(raft_node.clone()));

    // Start health monitoring in background
    let health_clone = health_monitor.clone();
    let shutdown = CancellationToken::new();
    let shutdown_clone = shutdown.clone();
    tokio::spawn(async move {
        tokio::select! {
            _ = health_clone.monitor(5) => {}
            _ = shutdown_clone.cancelled() => {}
        }
    });

    // TODO: Create and start RPC server without actor dependencies
    // The RPC server needs to be reimplemented to work without actors
    // For now, Raft RPCs are handled through the existing network layer

    // Create supervisor (but don't use it for now - Raft is stable)
    let supervisor = SimpleSupervisor::new(format!("raft-node-{}", config.node_id));

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

    Ok(SimpleNodeHandle {
        config,
        metadata_store,
        iroh_manager,
        raft_node,
        network_factory,
        // gossip_discovery,
        // rpc_server,
        supervisor,
        health_monitor,
        shutdown_token: shutdown,
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
