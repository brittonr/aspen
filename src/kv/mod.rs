pub mod client;
pub mod types;

use std::path::PathBuf;

use anyhow::Result;
use iroh::EndpointAddr;

pub use self::client::KvClient;
pub use self::types::NodeId;

use crate::cluster::bootstrap::{BootstrapHandle, bootstrap_node};
use crate::cluster::config::ClusterBootstrapConfig;
use crate::raft::storage::StorageBackend;

/// Builds a KV service with full cluster bootstrap.
///
/// This builder provides a programmatic API for starting Aspen nodes,
/// wiring together all the components: Raft consensus, Iroh P2P networking,
/// gossip discovery, and actor supervision.
///
/// # Example
///
/// ```no_run
/// use aspen::kv::KvServiceBuilder;
/// use aspen::raft::storage::StorageBackend;
///
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let service = KvServiceBuilder::new(1, "./data/node-1")
///     .with_storage(StorageBackend::InMemory)
///     .with_gossip(true)
///     .start()
///     .await?;
///
/// // Use the service...
///
/// service.shutdown().await?;
/// # Ok(())
/// # }
/// ```
pub struct KvServiceBuilder {
    config: ClusterBootstrapConfig,
}

impl KvServiceBuilder {
    /// Create a new builder for the given node ID and data directory.
    pub fn new(node_id: NodeId, data_dir: impl Into<PathBuf>) -> Self {
        let config = ClusterBootstrapConfig {
            node_id: node_id.into(),
            data_dir: Some(data_dir.into()),
            storage_backend: StorageBackend::Sqlite,
            redb_log_path: None,
            redb_sm_path: None,
            sqlite_log_path: None,
            sqlite_sm_path: None,
            host: "localhost".to_string(),
            ractor_port: 0,
            cookie: "aspen-cluster".to_string(),
            http_addr: "127.0.0.1:8080".parse().unwrap(),
            control_backend: Default::default(),
            heartbeat_interval_ms: 1000,
            election_timeout_min_ms: 3000,
            election_timeout_max_ms: 6000,
            iroh: Default::default(),
            peers: Vec::new(),
            supervision_config: Default::default(),
            raft_mailbox_capacity: 1000,
        };
        Self { config }
    }

    /// Set the storage backend (default: Sqlite).
    pub fn with_storage(mut self, backend: StorageBackend) -> Self {
        self.config.storage_backend = backend;
        self
    }

    /// Configure peer node addresses.
    ///
    /// Format: "node_id@endpoint_id:relay_url:direct_addrs"
    pub fn with_peers(mut self, peers: Vec<String>) -> Self {
        self.config.peers = peers;
        self
    }

    /// Enable/disable gossip-based peer discovery (default: false).
    pub fn with_gossip(mut self, enable: bool) -> Self {
        self.config.iroh.enable_gossip = enable;
        self
    }

    /// Enable/disable mDNS discovery (default: false).
    pub fn with_mdns(mut self, enable: bool) -> Self {
        self.config.iroh.enable_mdns = enable;
        self
    }

    /// Set the Raft heartbeat interval in milliseconds (default: 1000).
    pub fn with_heartbeat_interval_ms(mut self, interval_ms: u64) -> Self {
        self.config.heartbeat_interval_ms = interval_ms;
        self
    }

    /// Set the Raft election timeout range in milliseconds (default: 3000-6000).
    pub fn with_election_timeout_ms(mut self, min_ms: u64, max_ms: u64) -> Self {
        self.config.election_timeout_min_ms = min_ms;
        self.config.election_timeout_max_ms = max_ms;
        self
    }

    /// Start the KV service by bootstrapping a full cluster node.
    ///
    /// This initializes all components:
    /// - Metadata store
    /// - Iroh P2P endpoint
    /// - Ractor cluster node server
    /// - Raft actor with supervision
    /// - IRPC server for Raft RPCs
    /// - Optional gossip discovery
    ///
    /// Returns a handle that can be used to shut down the service gracefully.
    pub async fn start(self) -> Result<KvService> {
        let handle = bootstrap_node(self.config).await?;
        Ok(KvService { handle })
    }
}

/// Handle returned by [`KvServiceBuilder::start`].
///
/// Wraps a [`BootstrapHandle`] and provides convenient access to the
/// node's components for integration testing and programmatic usage.
pub struct KvService {
    handle: BootstrapHandle,
}

impl KvService {
    /// Get the node ID.
    pub fn node_id(&self) -> NodeId {
        self.handle.config.node_id.into()
    }

    /// Get the data directory path.
    pub fn data_dir(&self) -> PathBuf {
        self.handle.config.data_dir()
    }

    /// Get the Iroh endpoint address for P2P communication.
    pub fn endpoint_addr(&self) -> EndpointAddr {
        self.handle.iroh_manager.node_addr().clone()
    }

    /// Get a reference to the Raft core for direct Raft operations.
    pub fn raft_core(&self) -> &openraft::Raft<crate::raft::types::AppTypeConfig> {
        &self.handle.raft_core
    }

    /// Get the bootstrap handle for advanced operations.
    ///
    /// This provides access to all internal components including metadata store,
    /// network factory, health monitor, etc.
    pub fn handle(&self) -> &BootstrapHandle {
        &self.handle
    }

    /// Create a KV client for this node.
    ///
    /// The client can be used to perform read/write operations through
    /// the Raft consensus layer.
    pub fn client(&self) -> KvClient {
        KvClient::new(self.handle.raft_actor.clone())
    }

    /// Gracefully shutdown the KV service.
    ///
    /// Shuts down all components in reverse order of startup:
    /// 1. Gossip discovery (if enabled)
    /// 2. IRPC server
    /// 3. Iroh endpoint
    /// 4. Node server
    /// 5. Raft actor
    pub async fn shutdown(self) -> Result<()> {
        self.handle.shutdown().await
    }
}
