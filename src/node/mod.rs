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
//! Unit tests for NodeBuilder are included in the tests module:
//!       - Builder with all configuration options ✓
//!       - with_storage() for each StorageBackend variant ✓
//!       - start() returning properly configured Node ✓
//!       - Fluent API chaining ✓
//!       - Write batching configuration ✓
//!       Coverage: 100% of public API tested
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

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use iroh::EndpointAddr;
use iroh::protocol::Router;
use parking_lot::RwLock;
use tokio_util::sync::CancellationToken;

pub use self::types::NodeId;
use crate::AuthenticatedRaftProtocolHandler;
use crate::CLIENT_ALPN;
use crate::ClientProtocolContext;
use crate::ClientProtocolHandler;
use crate::RaftProtocolHandler;
use crate::TrustedPeersRegistry;
use crate::api::ClusterController;
use crate::api::KeyValueStore;
use crate::cluster::bootstrap::NodeHandle;
use crate::cluster::bootstrap::bootstrap_node;
use crate::cluster::config::NodeConfig;
use crate::cluster::federation::ClusterIdentity;
use crate::cluster::federation::FEDERATION_ALPN;
use crate::cluster::federation::FederatedId;
use crate::cluster::federation::FederationProtocolHandler;
use crate::cluster::federation::FederationSettings;
use crate::cluster::federation::TrustManager;
use crate::cluster::federation::sync::FederationProtocolContext;
use crate::protocol_adapters::EndpointProviderAdapter;
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

    /// Configure write batching for improved throughput.
    ///
    /// When enabled, multiple write operations are batched together into
    /// a single Raft proposal to amortize fsync costs. This significantly
    /// increases throughput under concurrent load.
    ///
    /// Default: `BatchConfig::default()` (batching enabled)
    pub fn with_write_batching(mut self, config: crate::raft::BatchConfig) -> Self {
        self.config.batch_config = Some(config);
        self
    }

    /// Disable write batching.
    ///
    /// When disabled, each write operation goes directly to Raft without
    /// batching. This may be preferred for low-latency scenarios with
    /// minimal concurrent writes.
    pub fn without_write_batching(mut self) -> Self {
        self.config.batch_config = None;
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
        self.config.validate().context("configuration validation failed")?;

        let handle = bootstrap_node(self.config).await?;
        Ok(Node {
            handle,
            router: None,
            membership_watcher_cancel: None,
            federation_identity: None,
            federation_trust_manager: None,
            federation_resource_settings: None,
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
    /// Cancellation token for the membership watcher task.
    /// Used to gracefully shut down the watcher when the node shuts down.
    membership_watcher_cancel: Option<CancellationToken>,
    /// Federation cluster identity (if federation is enabled).
    federation_identity: Option<ClusterIdentity>,
    /// Federation trust manager (if federation is enabled).
    federation_trust_manager: Option<Arc<TrustManager>>,
    /// Federation resource settings (if federation is enabled).
    federation_resource_settings: Option<Arc<RwLock<HashMap<FederatedId, FederationSettings>>>>,
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
        self.handle.network.iroh_manager.node_addr().clone()
    }

    /// Get the RaftNode for direct Raft and KV operations.
    ///
    /// The RaftNode implements both `ClusterController` and `KeyValueStore` traits,
    /// providing a unified interface for cluster management and key-value operations.
    pub fn raft_node(&self) -> &Arc<RaftNode> {
        &self.handle.storage.raft_node
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
        self.handle.storage.raft_node.as_ref()
    }

    /// Access the KeyValueStore interface for key-value operations.
    pub fn kv_store(&self) -> &dyn KeyValueStore {
        self.handle.storage.raft_node.as_ref()
    }

    /// Get the federation cluster identity (if federation is enabled).
    pub fn federation_identity(&self) -> Option<&ClusterIdentity> {
        self.federation_identity.as_ref()
    }

    /// Get the federation trust manager (if federation is enabled).
    pub fn federation_trust_manager(&self) -> Option<&Arc<TrustManager>> {
        self.federation_trust_manager.as_ref()
    }

    /// Get the federation resource settings (if federation is enabled).
    pub fn federation_resource_settings(&self) -> Option<&Arc<RwLock<HashMap<FederatedId, FederationSettings>>>> {
        self.federation_resource_settings.as_ref()
    }

    /// Create a ClientProtocolContext for the Client RPC handler.
    ///
    /// This context provides all the dependencies needed by the Client RPC handlers.
    /// Many fields are optional and will be None when the corresponding feature
    /// is not enabled or configured.
    fn create_client_protocol_context(&self) -> ClientProtocolContext {
        let raft_node = self.handle.storage.raft_node.clone();

        // Create endpoint provider adapter
        let endpoint_manager: Arc<dyn aspen_core::EndpointProvider> =
            Arc::new(EndpointProviderAdapter::new(self.handle.network.iroh_manager.clone()));

        ClientProtocolContext {
            node_id: self.handle.config.node_id,
            controller: raft_node.clone(),
            kv_store: raft_node.clone(),
            #[cfg(feature = "sql")]
            sql_executor: raft_node.clone(),
            state_machine: Some(self.handle.storage.state_machine.clone()),
            endpoint_manager,
            blob_store: None, // Would need IrohBlobStore passed in
            peer_manager: None,
            docs_sync: None,
            cluster_cookie: self.handle.config.cookie.clone(),
            start_time: std::time::Instant::now(),
            network_factory: None, // Could be added if needed
            token_verifier: None,
            require_auth: false,
            topology: None,
            #[cfg(feature = "global-discovery")]
            content_discovery: None,
            #[cfg(feature = "forge")]
            forge_node: None,
            #[cfg(feature = "pijul")]
            pijul_store: None,
            job_manager: None,
            worker_service: None,
            worker_coordinator: None,
            watch_registry: None,
            hook_service: self.handle.hooks.hook_service.clone(),
            hooks_config: self.handle.config.hooks.clone(),
        }
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
        use crate::RAFT_ALPN;
        use crate::RAFT_AUTH_ALPN;

        let raft_core = self.handle.storage.raft_node.raft().as_ref().clone();
        // SAFETY: aspen_raft::AppTypeConfig and aspen_transport::AppTypeConfig are
        // structurally identical (both use the same types from aspen-raft-types).
        // This transmute is safe because both AppTypeConfig declarations use the exact
        // same component types: AppRequest, AppResponse, NodeId, and RaftMemberInfo.
        let raft_core_transport: openraft::Raft<aspen_transport::rpc::AppTypeConfig> =
            unsafe { std::mem::transmute(raft_core.clone()) };
        let raft_handler = RaftProtocolHandler::new(raft_core_transport.clone());

        let mut builder = Router::builder(self.handle.network.iroh_manager.endpoint().clone());

        // Always register legacy unauthenticated handler for backwards compatibility
        builder = builder.accept(RAFT_ALPN, raft_handler);
        tracing::info!("registered Raft RPC protocol handler (ALPN: raft-rpc)");

        // Register authenticated handler if enabled
        if self.handle.config.iroh.enable_raft_auth {
            use crate::raft::membership_watcher::spawn_membership_watcher;

            // Pre-populate TrustedPeersRegistry with this node's own identity
            // This allows self-connections and provides the starting point for
            // membership-based peer authorization.
            let our_public_key = self.handle.network.iroh_manager.node_addr().id;
            let trusted_peers = TrustedPeersRegistry::with_peers([our_public_key]);

            // Spawn the membership watcher to keep TrustedPeersRegistry in sync with Raft membership.
            // The watcher monitors Raft metrics for membership changes and updates the registry
            // with PublicKeys from RaftMemberInfo.iroh_addr.id.
            let watcher_cancel =
                spawn_membership_watcher(self.handle.storage.raft_node.raft().clone(), trusted_peers.clone());
            self.membership_watcher_cancel = Some(watcher_cancel);

            let auth_handler = AuthenticatedRaftProtocolHandler::new(raft_core_transport, trusted_peers);
            builder = builder.accept(RAFT_AUTH_ALPN, auth_handler);
            tracing::info!(
                our_public_key = %our_public_key,
                "registered authenticated Raft RPC protocol handler with membership sync (ALPN: raft-auth)"
            );
        }

        // Add gossip handler if enabled
        if let Some(gossip) = self.handle.network.iroh_manager.gossip() {
            use iroh_gossip::ALPN as GOSSIP_ALPN;
            builder = builder.accept(GOSSIP_ALPN, gossip.clone());
            tracing::info!("registered Gossip protocol handler");
        }

        // Add Client RPC protocol handler
        // This enables CLI and programmatic clients to connect via CLIENT_ALPN
        let client_context = self.create_client_protocol_context();
        let client_handler = ClientProtocolHandler::new(client_context);
        builder = builder.accept(CLIENT_ALPN, client_handler);
        tracing::info!("registered Client RPC protocol handler (ALPN: aspen-client)");

        // Add federation handler if enabled
        if self.handle.config.federation.enabled {
            let fed_config = &self.handle.config.federation;

            // Load or generate cluster identity
            let cluster_identity = if let Some(ref key_hex) = fed_config.cluster_key {
                // Load from config
                match ClusterIdentity::from_hex_key(key_hex, fed_config.cluster_name.clone()) {
                    Ok(identity) => identity,
                    Err(e) => {
                        tracing::error!("Invalid federation cluster_key: {}", e);
                        tracing::warn!("Federation disabled due to invalid key");
                        // Skip federation registration
                        self.router = Some(builder.spawn());
                        tracing::info!("Iroh Router spawned with ALPN-based protocol dispatching");
                        return;
                    }
                }
            } else {
                // Generate a new identity
                let identity = ClusterIdentity::generate(fed_config.cluster_name.clone());
                tracing::info!(
                    cluster_name = %identity.name(),
                    cluster_key = %identity.public_key(),
                    "Generated new federation cluster identity"
                );
                identity
            };

            // Create trust manager with trusted clusters from config
            let trust_manager = Arc::new(TrustManager::new());
            for key_hex in &fed_config.trusted_clusters {
                if let Ok(bytes) = hex::decode(key_hex)
                    && bytes.len() == 32
                {
                    let mut key_bytes = [0u8; 32];
                    key_bytes.copy_from_slice(&bytes);
                    if let Ok(public_key) = iroh::PublicKey::from_bytes(&key_bytes) {
                        trust_manager.add_trusted(
                            public_key,
                            format!("trusted-{}", &key_hex[..8]),
                            None, // No notes for config-loaded clusters
                        );
                        tracing::debug!(
                            cluster_key = %public_key,
                            "Added trusted cluster from config"
                        );
                    }
                }
            }

            // Create resource settings (starts empty, populated via CLI/API)
            let resource_settings = Arc::new(RwLock::new(HashMap::new()));

            // Create federation protocol context and handler
            let context = FederationProtocolContext {
                cluster_identity: cluster_identity.clone(),
                trust_manager: trust_manager.clone(),
                resource_settings: resource_settings.clone(),
                endpoint: Arc::new(self.handle.network.iroh_manager.endpoint().clone()),
                hlc: Arc::new(aspen_core::hlc::create_hlc(&self.handle.config.node_id.to_string())),
            };
            let federation_handler = FederationProtocolHandler::new(context);

            builder = builder.accept(FEDERATION_ALPN, federation_handler);
            tracing::info!(
                cluster_name = %cluster_identity.name(),
                cluster_key = %cluster_identity.public_key(),
                "registered Federation protocol handler (ALPN: /aspen/federation/1)"
            );

            // Store federation components for later access
            self.federation_identity = Some(cluster_identity);
            self.federation_trust_manager = Some(trust_manager);
            self.federation_resource_settings = Some(resource_settings);
        }

        // Spawn the router and store the handle to keep it alive
        // Dropping the Router would shut down protocol handling!
        self.router = Some(builder.spawn());
        tracing::info!("Iroh Router spawned with ALPN-based protocol dispatching");
    }

    /// Spawn the Iroh Router with Raft and Blobs protocol handlers.
    ///
    /// This is similar to `spawn_router()` but additionally registers the
    /// iroh-blobs protocol for P2P blob transfers. Use this when you need
    /// content-addressed blob storage with P2P download support.
    ///
    /// # Arguments
    ///
    /// * `blobs_handler` - The BlobsProtocol handler from an IrohBlobStore
    ///
    /// # Example
    ///
    /// ```ignore
    /// let blob_store = IrohBlobStore::new(&data_dir, endpoint.clone()).await?;
    /// node.spawn_router_with_blobs(blob_store.protocol_handler());
    /// ```
    pub fn spawn_router_with_blobs(&mut self, blobs_handler: iroh_blobs::BlobsProtocol) {
        use crate::RAFT_ALPN;
        use crate::RAFT_AUTH_ALPN;

        let raft_core = self.handle.storage.raft_node.raft().as_ref().clone();
        // SAFETY: aspen_raft::AppTypeConfig and aspen_transport::AppTypeConfig are
        // structurally identical (both use the same types from aspen-raft-types).
        // This transmute is safe because both AppTypeConfig declarations use the exact
        // same component types: AppRequest, AppResponse, NodeId, and RaftMemberInfo.
        let raft_core_transport: openraft::Raft<aspen_transport::rpc::AppTypeConfig> =
            unsafe { std::mem::transmute(raft_core.clone()) };
        let raft_handler = RaftProtocolHandler::new(raft_core_transport.clone());

        let mut builder = Router::builder(self.handle.network.iroh_manager.endpoint().clone());

        // Always register legacy unauthenticated handler for backwards compatibility
        builder = builder.accept(RAFT_ALPN, raft_handler);
        tracing::info!("registered Raft RPC protocol handler (ALPN: raft-rpc)");

        // Register authenticated handler if enabled
        if self.handle.config.iroh.enable_raft_auth {
            use crate::raft::membership_watcher::spawn_membership_watcher;

            let our_public_key = self.handle.network.iroh_manager.node_addr().id;
            let trusted_peers = TrustedPeersRegistry::with_peers([our_public_key]);

            let watcher_cancel =
                spawn_membership_watcher(self.handle.storage.raft_node.raft().clone(), trusted_peers.clone());
            self.membership_watcher_cancel = Some(watcher_cancel);

            let auth_handler = AuthenticatedRaftProtocolHandler::new(raft_core_transport, trusted_peers);
            builder = builder.accept(RAFT_AUTH_ALPN, auth_handler);
            tracing::info!(
                our_public_key = %our_public_key,
                "registered authenticated Raft RPC protocol handler with membership sync (ALPN: raft-auth)"
            );
        }

        // Add gossip handler if enabled
        if let Some(gossip) = self.handle.network.iroh_manager.gossip() {
            use iroh_gossip::ALPN as GOSSIP_ALPN;
            builder = builder.accept(GOSSIP_ALPN, gossip.clone());
            tracing::info!("registered Gossip protocol handler");
        }

        // Add Client RPC protocol handler
        let client_context = self.create_client_protocol_context();
        let client_handler = ClientProtocolHandler::new(client_context);
        builder = builder.accept(CLIENT_ALPN, client_handler);
        tracing::info!("registered Client RPC protocol handler (ALPN: aspen-client)");

        // Register the blobs protocol handler
        builder = builder.accept(iroh_blobs::ALPN, blobs_handler);
        tracing::info!("registered Blobs protocol handler (ALPN: iroh-blobs/0)");

        // Spawn the router and store the handle to keep it alive
        self.router = Some(builder.spawn());
        tracing::info!("Iroh Router spawned with ALPN-based protocol dispatching (including blobs)");
    }

    /// Gracefully shutdown the node.
    ///
    /// Shuts down all components in reverse order of startup:
    /// 1. Membership watcher (if enabled)
    /// 2. Gossip discovery (if enabled)
    /// 3. IRPC server
    /// 4. Iroh endpoint
    /// 5. RaftNode
    pub async fn shutdown(self) -> Result<()> {
        // Cancel membership watcher if running
        if let Some(cancel) = self.membership_watcher_cancel {
            tracing::info!("cancelling membership watcher");
            cancel.cancel();
        }

        self.handle.shutdown().await
    }
}
