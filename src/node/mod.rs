//! Node orchestration and distributed system API.
//!
//! Provides a high-level API for building and operating Aspen nodes. This module
//! orchestrates the complete node bootstrap sequence, wiring together Raft consensus,
//! Iroh P2P networking, and distributed storage using direct async APIs (no actors).
//!
//! # Key Components
//!
//! - `NodeBuilder`: Fluent builder for node configuration and bootstrap
//! - `Node`: Handle to a running Aspen node with all its subsystems
//! - `NodeId`: Type-safe wrapper for u64 node identifiers
//! - Bootstrap integration: Delegates to cluster::bootstrap for full node startup
//!
//! # Architecture
//!
//! The node module orchestrates the entire distributed system stack:
//! 1. Initializes metadata store for node registry
//! 2. Creates Iroh P2P endpoint for inter-node communication
//! 3. Starts Raft consensus with SQLite/InMemory state machine
//! 4. Provides direct async API to the distributed key-value store via RaftNode
//!
//! # Tiger Style
//!
//! - Explicit types: NodeId wrapper prevents u64 confusion with other IDs
//! - Builder pattern: Fluent API with compile-time validation
//! - Resource bounds: All operations bounded by Raft batch limits
//! - Error handling: Anyhow for application errors, clear context messages
//! - Deterministic testing: Builder supports in-memory mode for tests
//!
//! # Test Coverage
//!
//! TODO: Add unit tests for NodeBuilder:
//!       - Builder with all configuration options
//!       - with_storage() for each StorageBackend variant
//!       - with_http_port() binding validation
//!       - start() returning properly configured Node
//!       Coverage: 0% line coverage (tested via node_builder_integration tests)
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
//! // Use the RaftNode directly for KV operations
//! let raft_node = node.raft_node();
//! // raft_node implements both ClusterController and KeyValueStore traits
//! ```

pub mod types;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use iroh::EndpointAddr;

pub use self::types::NodeId;

use iroh::protocol::Router;

use crate::api::{ClusterController, KeyValueStore};
use crate::cluster::bootstrap::{NodeHandle, bootstrap_node};
use crate::cluster::config::NodeConfig;
use crate::protocol_handlers::RaftProtocolHandler;
use crate::raft::node::RaftNode;
use crate::raft::storage::StorageBackend;

/// Builds an Aspen node with full cluster bootstrap.
///
/// This builder provides a programmatic API for starting Aspen nodes,
/// wiring together all the components: Raft consensus, Iroh P2P networking,
/// gossip discovery, and distributed storage.
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
    ///
    /// All other configuration uses defaults from environment variables or
    /// centralized default functions in config.rs. Override with builder methods.
    pub fn new(node_id: NodeId, data_dir: impl Into<PathBuf>) -> Self {
        let mut config = NodeConfig::from_env();
        config.node_id = node_id.into();
        config.data_dir = Some(data_dir.into());
        // from_env() already sets defaults via default_*() functions:
        // - storage_backend: Sqlite
        // - host: "localhost"
        // - cookie: "aspen-cluster"
        // - http_addr: 127.0.0.1:8080
        // - heartbeat_interval_ms: 1000
        // - election_timeout_min_ms: 3000
        // - election_timeout_max_ms: 6000
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

    /// Set the cluster authentication cookie (default: "aspen-cluster").
    pub fn with_cookie(mut self, cookie: impl Into<String>) -> Self {
        self.config.cookie = cookie.into();
        self
    }

    /// Set the HTTP API address (default: 127.0.0.1:8080).
    pub fn with_http_addr(mut self, addr: std::net::SocketAddr) -> Self {
        self.config.http_addr = addr;
        self
    }

    /// Set the Iroh secret key (hex-encoded, 64 characters).
    ///
    /// If not set, a new key is generated on startup.
    pub fn with_iroh_secret_key(mut self, secret_key: impl Into<String>) -> Self {
        self.config.iroh.secret_key = Some(secret_key.into());
        self
    }

    /// Set a cluster ticket for bootstrap peer discovery.
    pub fn with_gossip_ticket(mut self, ticket: impl Into<String>) -> Self {
        self.config.iroh.gossip_ticket = Some(ticket.into());
        self
    }

    /// Enable/disable DNS discovery (default: false).
    pub fn with_dns_discovery(mut self, enable: bool) -> Self {
        self.config.iroh.enable_dns_discovery = enable;
        self
    }

    /// Enable/disable Pkarr DHT discovery (default: false).
    pub fn with_pkarr(mut self, enable: bool) -> Self {
        self.config.iroh.enable_pkarr = enable;
        self
    }

    /// Start the node by bootstrapping a full Aspen cluster node.
    ///
    /// This initializes all components:
    /// - Metadata store
    /// - Iroh P2P endpoint
    /// - RaftNode with direct async API
    /// - Optional gossip discovery
    ///
    /// Returns a handle that can be used to shut down the node gracefully.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration validation fails or bootstrap fails.
    pub async fn start(self) -> Result<Node> {
        use anyhow::Context;

        // Validate configuration before expensive bootstrap
        self.config
            .validate()
            .context("configuration validation failed")?;

        let handle = bootstrap_node(self.config).await?;
        Ok(Node {
            handle,
            router: None,
        })
    }
}

/// Handle returned by [`NodeBuilder::start`].
///
/// Wraps a [`NodeHandle`] and provides convenient access to the
/// node's components for integration testing and programmatic usage.
pub struct Node {
    handle: NodeHandle,
    /// Router for handling incoming protocol connections.
    /// Stored to keep it alive - dropping the Router shuts down protocol handling.
    router: Option<Router>,
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

    /// Get the RaftNode for direct Raft and KV operations.
    ///
    /// The RaftNode implements both `ClusterController` and `KeyValueStore` traits,
    /// providing a unified interface for cluster management and key-value operations.
    pub fn raft_node(&self) -> &Arc<RaftNode> {
        &self.handle.raft_node
    }

    /// Get the node handle for advanced operations.
    ///
    /// This provides access to all internal components including metadata store,
    /// network factory, health monitor, etc.
    pub fn handle(&self) -> &NodeHandle {
        &self.handle
    }

    /// Access the ClusterController interface for cluster management operations.
    pub fn cluster_controller(&self) -> &dyn ClusterController {
        self.handle.raft_node.as_ref()
    }

    /// Access the KeyValueStore interface for key-value operations.
    pub fn kv_store(&self) -> &dyn KeyValueStore {
        self.handle.raft_node.as_ref()
    }

    /// Spawn the Iroh Router with the Raft protocol handler.
    ///
    /// This must be called after `NodeBuilder::start()` to enable inter-node
    /// communication. The Router registers the `RaftProtocolHandler` which
    /// handles incoming Raft RPC connections.
    ///
    /// # Why This Is Separate
    ///
    /// The Router is not spawned automatically by `NodeBuilder::start()` because:
    /// - `aspen-node.rs` needs to add additional handlers (Client, LogSubscriber)
    /// - The Router configuration varies by deployment scenario
    ///
    /// For integration tests, call this immediately after `start()`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut node = NodeBuilder::new(NodeId(1), &temp_dir)
    ///     .with_storage(StorageBackend::InMemory)
    ///     .start()
    ///     .await?;
    /// node.spawn_router();
    /// ```
    pub fn spawn_router(&mut self) {
        use crate::protocol_handlers::RAFT_ALPN;

        let raft_handler = RaftProtocolHandler::new(self.handle.raft_node.raft().as_ref().clone());

        let mut builder = Router::builder(self.handle.iroh_manager.endpoint().clone());
        builder = builder.accept(RAFT_ALPN, raft_handler);

        // Add gossip handler if enabled
        if let Some(gossip) = self.handle.iroh_manager.gossip() {
            use iroh_gossip::ALPN as GOSSIP_ALPN;
            builder = builder.accept(GOSSIP_ALPN, gossip.clone());
        }

        // Spawn the router and store the handle to keep it alive
        // Dropping the Router would shut down protocol handling!
        self.router = Some(builder.spawn());
    }

    /// Gracefully shutdown the node.
    ///
    /// Shuts down all components in reverse order of startup:
    /// 1. Gossip discovery (if enabled)
    /// 2. IRPC server
    /// 3. Iroh endpoint
    /// 4. RaftNode
    pub async fn shutdown(self) -> Result<()> {
        self.handle.shutdown().await
    }
}
