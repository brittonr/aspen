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

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use iroh::EndpointAddr;
use iroh_gossip::proto::TopicId;
use openraft::{Config as RaftConfig, Raft};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::cluster::config::NodeConfig;
use crate::cluster::gossip_discovery::GossipPeerDiscovery;
use crate::cluster::metadata::{MetadataStore, NodeStatus};
use crate::cluster::ticket::AspenClusterTicket;
use crate::cluster::{IrohEndpointConfig, IrohEndpointManager};
use crate::protocol_handlers::RAFT_ALPN;
use crate::raft::StateMachineVariant;
use crate::raft::network::IrpcRaftNetworkFactory;
use crate::raft::node::{RaftNode, RaftNodeHealth};
use crate::raft::server::RaftRpcServer;
use crate::raft::storage::{InMemoryLogStore, InMemoryStateMachine, RedbLogStore, StorageBackend};
use crate::raft::storage_sqlite::SqliteStateMachine;
use crate::raft::supervisor::Supervisor;
use crate::raft::types::NodeId;
use iroh_gossip::net::GOSSIP_ALPN;

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
    /// RPC server for handling incoming Raft RPCs.
    pub rpc_server: RaftRpcServer,
    /// Supervisor for automatic restarts.
    pub supervisor: Arc<Supervisor>,
    /// Health monitor.
    pub health_monitor: Arc<RaftNodeHealth>,
    /// Cancellation token for shutdown.
    pub shutdown_token: CancellationToken,
    /// Gossip topic ID used for peer discovery.
    ///
    /// Derived from the cluster cookie (hash) or provided via a cluster ticket.
    /// Exposed for generating cluster tickets via HTTP API.
    pub gossip_topic_id: Option<TopicId>,
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

        // Stop RPC server
        info!("shutting down RPC server");
        if let Err(err) = self.rpc_server.shutdown().await {
            error!(error = ?err, "failed to shutdown RPC server gracefully");
        }

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
    // IMPORTANT: Configure ALPNs to accept the protocols we serve
    let mut alpns = vec![RAFT_ALPN.to_vec()];
    if config.iroh.enable_gossip {
        alpns.push(GOSSIP_ALPN.to_vec());
    }

    let iroh_config = IrohEndpointConfig::default()
        .with_gossip(config.iroh.enable_gossip)
        .with_mdns(config.iroh.enable_mdns)
        .with_dns_discovery(config.iroh.enable_dns_discovery)
        .with_pkarr(config.iroh.enable_pkarr)
        .with_alpns(alpns);

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

    // Setup gossip discovery if enabled
    //
    // Gossip discovery automatically:
    // 1. Subscribes to a topic derived from the cluster cookie (or ticket)
    // 2. Broadcasts this node's ID and EndpointAddr every 10 seconds
    // 3. Receives peer announcements and adds them to the network factory
    //
    // This enables automatic peer discovery without manual configuration.
    let (gossip_discovery, gossip_topic_id) = if config.iroh.enable_gossip {
        // Determine topic ID: from ticket if provided, otherwise derive from cookie
        let topic_id = if let Some(ref ticket_str) = config.iroh.gossip_ticket {
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

        info!(
            node_id = config.node_id,
            topic_id = %hex::encode(topic_id.as_bytes()),
            "starting gossip discovery"
        );

        match GossipPeerDiscovery::spawn(
            topic_id,
            config.node_id.into(),
            &iroh_manager,
            Some(network_factory.clone()),
        )
        .await
        {
            Ok(discovery) => {
                info!(
                    node_id = config.node_id,
                    topic_id = %hex::encode(topic_id.as_bytes()),
                    "gossip discovery started successfully"
                );
                (Some(discovery), Some(topic_id))
            }
            Err(err) => {
                // Gossip failure is non-fatal - node can still work with manual peers
                warn!(
                    error = %err,
                    node_id = config.node_id,
                    "failed to start gossip discovery, continuing without it"
                );
                (None, None)
            }
        }
    } else {
        info!(
            node_id = config.node_id,
            "gossip discovery disabled by configuration"
        );
        (None, None)
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
    let health_clone = health_monitor.clone();
    let shutdown = CancellationToken::new();
    let shutdown_clone = shutdown.clone();
    let supervisor_for_health = supervisor.clone();
    let node_id_for_health = config.node_id;
    tokio::spawn(async move {
        // Create a channel to communicate failures from the sync callback to async supervisor
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(16);

        // Spawn a task to process health failures and record them with supervisor
        let supervisor_clone = supervisor_for_health.clone();
        tokio::spawn(async move {
            while let Some(reason) = rx.recv().await {
                supervisor_clone.record_health_failure(&reason).await;

                // Check if we should give up (too many failures)
                if !supervisor_clone.should_attempt_recovery().await {
                    error!(
                        node_id = node_id_for_health,
                        "too many health failures, supervisor circuit breaker triggered"
                    );
                    // Cancel the supervisor which will propagate shutdown
                    supervisor_clone.stop();
                    break;
                }
            }
        });

        tokio::select! {
            _ = health_clone.monitor_with_callback(5, move |status| {
                let reason = format!(
                    "health check failed: state={:?}, failures={}",
                    status.state, status.consecutive_failures
                );
                // Non-blocking send - if channel is full, drop the message
                let _ = tx.try_send(reason);
            }) => {}
            _ = shutdown_clone.cancelled() => {}
        }
    });

    // Start RPC server for handling incoming Raft RPCs
    let rpc_server = RaftRpcServer::spawn(iroh_manager.clone(), raft.as_ref().clone());
    info!(node_id = config.node_id, "started Raft RPC server");

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
        rpc_server,
        supervisor,
        health_monitor,
        shutdown_token: shutdown,
        gossip_topic_id,
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
