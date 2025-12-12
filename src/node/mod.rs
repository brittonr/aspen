//! Node orchestration and distributed system client API.
//!
//! Provides a high-level API for building and operating Aspen nodes. This module
//! orchestrates the complete node bootstrap sequence, wiring together Raft consensus,
//! Iroh P2P networking, actor supervision, and distributed storage. The builder
//! (NodeBuilder) handles node configuration and startup, while the client (NodeClient)
//! provides access to the distributed key-value store and cluster operations.
//!
//! # Key Components
//!
//! - `NodeBuilder`: Fluent builder for node configuration and bootstrap
//! - `Node`: Handle to a running Aspen node with all its subsystems
//! - `NodeClient`: Client API for key-value operations (read, write, scan, delete)
//! - `NodeId`: Type-safe wrapper for u64 node identifiers
//! - Bootstrap integration: Delegates to cluster::bootstrap for full node startup
//!
//! # Architecture
//!
//! The node module orchestrates the entire distributed system stack:
//! 1. Initializes metadata store for node registry
//! 2. Creates Iroh P2P endpoint for inter-node communication
//! 3. Starts Raft consensus with SQLite/InMemory state machine
//! 4. Establishes actor supervision trees for fault tolerance
//! 5. Provides client interface to the distributed key-value store
//!
//! # Tiger Style
//!
//! - Explicit types: NodeId wrapper prevents u64 confusion with other IDs
//! - Builder pattern: Fluent API with compile-time validation
//! - Resource bounds: All operations bounded by Raft batch limits
//! - Error handling: Anyhow for application errors, clear context messages
//! - Deterministic testing: Builder supports in-memory mode for tests
//!
//! # Example
//!
//! ```ignore
//! use aspen::node::NodeBuilder;
//! use aspen::raft::storage::StorageBackend;
//!
//! // Start a node
//! let node = NodeBuilder::new(1, "./data/node-1")
//!     .with_storage(StorageBackend::Sqlite)
//!     .start()
//!     .await?;
//!
//! // Use the client
//! let client = node.client();
//! client.write("key".to_string(), b"value".to_vec()).await?;
//! let value = client.read("key".to_string()).await?;
//! ```

pub mod client;
pub mod types;

use std::path::PathBuf;

use anyhow::Result;
use iroh::EndpointAddr;

pub use self::client::NodeClient;
pub use self::types::NodeId;

use crate::cluster::bootstrap::{NodeHandle, bootstrap_node};
use crate::cluster::config::NodeConfig;
use crate::raft::storage::StorageBackend;

/// Builds an Aspen node with full cluster bootstrap.
///
/// This builder provides a programmatic API for starting Aspen nodes,
/// wiring together all the components: Raft consensus, Iroh P2P networking,
/// gossip discovery, actor supervision, and distributed storage.
///
/// # Example
///
/// ```no_run
/// use aspen::node::NodeBuilder;
/// use aspen::raft::storage::StorageBackend;
///
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let node = NodeBuilder::new(1, "./data/node-1")
///     .with_storage(StorageBackend::InMemory)
///     .with_gossip(true)
///     .start()
///     .await?;
///
/// // Use the node...
///
/// node.shutdown().await?;
/// # Ok(())
/// # }
/// ```
pub struct NodeBuilder {
    config: NodeConfig,
}

impl NodeBuilder {
    /// Create a new builder for the given node ID and data directory.
    pub fn new(node_id: NodeId, data_dir: impl Into<PathBuf>) -> Self {
        let config = NodeConfig {
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

    /// Start the node by bootstrapping a full Aspen cluster node.
    ///
    /// This initializes all components:
    /// - Metadata store
    /// - Iroh P2P endpoint
    /// - Ractor cluster node server
    /// - Raft actor with supervision
    /// - IRPC server for Raft RPCs
    /// - Optional gossip discovery
    ///
    /// Returns a handle that can be used to shut down the node gracefully.
    pub async fn start(self) -> Result<Node> {
        let handle = bootstrap_node(self.config).await?;
        Ok(Node { handle })
    }
}

/// Handle returned by [`NodeBuilder::start`].
///
/// Wraps a [`NodeHandle`] and provides convenient access to the
/// node's components for integration testing and programmatic usage.
pub struct Node {
    handle: NodeHandle,
}

impl Node {
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

    /// Get the node handle for advanced operations.
    ///
    /// This provides access to all internal components including metadata store,
    /// network factory, health monitor, etc.
    pub fn handle(&self) -> &NodeHandle {
        &self.handle
    }

    /// Create a client for this node.
    ///
    /// The client can be used to perform distributed key-value operations
    /// and cluster management through the Raft consensus layer.
    pub fn client(&self) -> NodeClient {
        NodeClient::new(self.handle.raft_actor.clone())
    }

    /// Gracefully shutdown the node.
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
