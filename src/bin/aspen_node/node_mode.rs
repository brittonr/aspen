//! Node mode abstraction for single vs sharded Raft operation.
//!
//! `NodeMode` wraps both `NodeHandle` and `ShardedNodeHandle` to allow
//! the main function to work uniformly with both bootstrap modes.

use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use aspen::api::ClusterController;
use aspen::api::KeyValueStore;
use aspen::auth::CapabilityToken;
use aspen::cluster::bootstrap::NodeHandle;
use aspen::cluster::bootstrap::ShardedNodeHandle;
use aspen::cluster::bootstrap::bootstrap_node;
use aspen::cluster::bootstrap::bootstrap_sharded_node;
use aspen::cluster::config::NodeConfig;
use aspen_raft::node::RaftNode;
use tracing::info;

use crate::args::Args;

/// Unified node handle that wraps both sharded and non-sharded modes.
///
/// This enum allows main() to work uniformly with both bootstrap modes.
pub enum NodeMode {
    /// Single Raft node (legacy/default mode).
    Single(Box<NodeHandle>),
    /// Sharded node with multiple Raft instances.
    Sharded(Box<ShardedNodeHandle>),
}

impl NodeMode {
    pub fn iroh_manager(&self) -> &Arc<aspen::cluster::IrohEndpointManager> {
        match self {
            NodeMode::Single(h) => &h.network.iroh_manager,
            NodeMode::Sharded(h) => &h.base.network.iroh_manager,
        }
    }

    pub fn blob_store(&self) -> Option<&Arc<aspen::blob::IrohBlobStore>> {
        match self {
            NodeMode::Single(h) => h.network.blob_store.as_ref(),
            NodeMode::Sharded(h) => h.base.network.blob_store.as_ref(),
        }
    }

    pub fn docs_sync(&self) -> Option<&Arc<aspen::docs::DocsSyncResources>> {
        match self {
            NodeMode::Single(h) => h.sync.docs_sync.as_ref(),
            NodeMode::Sharded(h) => h.sync.docs_sync.as_ref(),
        }
    }

    pub fn peer_manager(&self) -> Option<&Arc<aspen::docs::PeerManager>> {
        match self {
            NodeMode::Single(h) => h.sync.peer_manager.as_ref(),
            NodeMode::Sharded(h) => h.sync.peer_manager.as_ref(),
        }
    }

    pub fn log_broadcast(
        &self,
    ) -> Option<&tokio::sync::broadcast::Sender<aspen::raft::log_subscriber::LogEntryPayload>> {
        match self {
            NodeMode::Single(h) => h.sync.log_broadcast.as_ref(),
            NodeMode::Sharded(h) => h.sync.log_broadcast.as_ref(),
        }
    }

    #[cfg(feature = "global-discovery")]
    pub fn content_discovery(&self) -> Option<aspen::cluster::content_discovery::ContentDiscoveryService> {
        match self {
            NodeMode::Single(h) => h.discovery.content_discovery.clone(),
            NodeMode::Sharded(h) => h.discovery.content_discovery.clone(),
        }
    }

    pub fn topology(&self) -> &Option<Arc<tokio::sync::RwLock<aspen_sharding::ShardTopology>>> {
        match self {
            NodeMode::Single(_) => &None,
            NodeMode::Sharded(h) => &h.sharding.topology,
        }
    }

    pub async fn shutdown(self) -> Result<()> {
        match self {
            NodeMode::Single(h) => h.shutdown().await,
            NodeMode::Sharded(h) => h.shutdown().await,
        }
    }

    /// Get the database handle from the state machine (if using Redb storage).
    ///
    /// For sharded mode, returns the database from shard 0 (primary shard).
    /// Returns None if using in-memory storage.
    pub fn db(&self) -> Option<std::sync::Arc<redb::Database>> {
        match self {
            NodeMode::Single(h) => h.storage.state_machine.db(),
            NodeMode::Sharded(h) => {
                // Use shard 0 as the primary shard for maintenance operations
                h.sharding.shard_state_machines.get(&0).and_then(|sm| sm.db())
            }
        }
    }

    /// Get the hook service for event-driven automation (if enabled).
    #[cfg(feature = "hooks")]
    pub fn hook_service(&self) -> Option<Arc<aspen_hooks::HookService>> {
        match self {
            NodeMode::Single(h) => h.hooks.hook_service.clone(),
            NodeMode::Sharded(h) => h.hooks.hook_service.clone(),
        }
    }

    /// Get the hooks configuration.
    pub fn hooks_config(&self) -> aspen_hooks_types::HooksConfig {
        match self {
            NodeMode::Single(h) => h.config.hooks.clone(),
            NodeMode::Sharded(h) => h.base.config.hooks.clone(),
        }
    }

    /// Get the shutdown token.
    pub fn shutdown_token(&self) -> tokio_util::sync::CancellationToken {
        match self {
            NodeMode::Single(h) => h.shutdown.shutdown_token.clone(),
            NodeMode::Sharded(h) => h.base.shutdown_token.clone(),
        }
    }

    /// Get mutable reference to blob replication resources (non-sharded mode only).
    pub fn blob_replication_mut(&mut self) -> Option<&mut aspen::cluster::bootstrap::BlobReplicationResources> {
        match self {
            NodeMode::Single(h) => Some(&mut h.blob_replication),
            NodeMode::Sharded(_) => None, // Blob replication not supported in sharded mode
        }
    }

    /// Get the blob replication manager (non-sharded mode only).
    #[allow(dead_code)]
    pub fn blob_replication_manager(&self) -> Option<aspen_blob::BlobReplicationManager> {
        match self {
            NodeMode::Single(h) => h.blob_replication.replication_manager.clone(),
            NodeMode::Sharded(_) => None, // Blob replication not supported in sharded mode
        }
    }

    /// Get the node configuration.
    pub fn config(&self) -> &aspen::cluster::config::NodeConfig {
        match self {
            NodeMode::Single(h) => &h.config,
            NodeMode::Sharded(h) => &h.base.config,
        }
    }
}

/// Components extracted from a node, regardless of mode.
pub type NodeComponents = (
    Arc<dyn ClusterController>,
    Arc<dyn KeyValueStore>,
    Arc<RaftNode>,
    Arc<aspen::cluster::IrpcRaftNetworkFactory>,
);

/// Extract node components based on mode (single vs sharded).
pub fn extract_node_components(config: &NodeConfig, node_mode: &NodeMode) -> Result<NodeComponents> {
    match node_mode {
        NodeMode::Single(handle) => {
            let (controller, kv_store) = setup_controllers(config, handle);
            let primary_raft_node = handle.storage.raft_node.clone();
            let network_factory = handle.network.network_factory.clone();
            Ok((controller, kv_store, primary_raft_node, network_factory))
        }
        NodeMode::Sharded(handle) => {
            let kv_store: Arc<dyn KeyValueStore> = handle.sharding.sharded_kv.clone();
            let primary_shard = handle
                .primary_shard()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("shard 0 must be present in sharded mode"))?;
            let controller: Arc<dyn ClusterController> = primary_shard.clone();
            let network_factory = handle.base.network.network_factory.clone();
            Ok((controller, kv_store, primary_shard, network_factory))
        }
    }
}

/// Bootstrap the node and generate root token if requested.
pub async fn bootstrap_node_and_generate_token(args: &Args, config: &NodeConfig) -> Result<NodeMode> {
    if config.sharding.is_enabled {
        // Sharded mode: multiple Raft instances
        let mut sharded_handle = bootstrap_sharded_node(config.clone()).await?;

        // Generate and output root token if requested
        if let Some(ref token_path) = args.output_root_token {
            generate_and_write_root_token(
                token_path,
                sharded_handle.base.network.iroh_manager.endpoint().secret_key(),
                &mut |token| sharded_handle.root_token = Some(token),
            )
            .await?;
        }

        info!(
            num_shards = sharded_handle.shard_count(),
            local_shards = ?sharded_handle.local_shard_ids(),
            "sharded node bootstrap complete"
        );

        Ok(NodeMode::Sharded(Box::new(sharded_handle)))
    } else {
        // Non-sharded mode: single Raft instance
        let mut handle = bootstrap_node(config.clone()).await?;

        // Generate and output root token if requested
        if let Some(ref token_path) = args.output_root_token {
            generate_and_write_root_token(
                token_path,
                handle.network.iroh_manager.endpoint().secret_key(),
                &mut |token| handle.root_token = Some(token),
            )
            .await?;
        }

        Ok(NodeMode::Single(Box::new(handle)))
    }
}

/// Generate and write root token to file.
async fn generate_and_write_root_token<F>(
    token_path: &std::path::Path,
    secret_key: &iroh::SecretKey,
    store_token: &mut F,
) -> Result<()>
where
    F: FnMut(CapabilityToken),
{
    let token = aspen::auth::generate_root_token(secret_key, std::time::Duration::from_secs(365 * 24 * 60 * 60))
        .context("failed to generate root token")?;

    let token_base64 = token.to_base64().context("failed to encode root token")?;
    std::fs::write(token_path, &token_base64)
        .with_context(|| format!("failed to write token to {}", token_path.display()))?;

    info!(
        token_path = %token_path.display(),
        issuer = %token.issuer,
        "root token written to file"
    );

    store_token(token);
    Ok(())
}

/// Setup cluster and key-value store controllers based on configuration.
#[cfg(feature = "testing")]
fn setup_controllers(config: &NodeConfig, handle: &NodeHandle) -> (Arc<dyn ClusterController>, Arc<dyn KeyValueStore>) {
    use aspen::cluster::config::ControlBackend;
    use aspen::testing::DeterministicClusterController;
    use aspen::testing::DeterministicKeyValueStore;

    match config.control_backend {
        ControlBackend::Deterministic => {
            (Arc::new(DeterministicClusterController::new()), Arc::new(DeterministicKeyValueStore::new()))
        }
        ControlBackend::Raft => {
            let raft_node = handle.storage.raft_node.clone();
            (raft_node.clone(), raft_node)
        }
    }
}

/// Setup cluster and key-value store controllers (Raft only without testing feature).
#[cfg(not(feature = "testing"))]
fn setup_controllers(
    _config: &NodeConfig,
    handle: &NodeHandle,
) -> (Arc<dyn ClusterController>, Arc<dyn KeyValueStore>) {
    let raft_node = handle.storage.raft_node.clone();
    (raft_node.clone(), raft_node)
}
