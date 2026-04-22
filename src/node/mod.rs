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

#[cfg(all(feature = "node-runtime", feature = "federation"))]
use std::collections::HashMap;
use std::path::PathBuf;
#[cfg(feature = "node-runtime")]
use std::sync::Arc;

#[cfg(feature = "node-runtime")]
use anyhow::Result;
// Secrets support — used in create_client_protocol_context (requires full node feature set)
#[cfg(all(
    feature = "node-runtime",
    feature = "secrets",
    feature = "jobs",
    feature = "docs",
    feature = "hooks",
    feature = "federation"
))]
use aspen_rpc_handlers::SecretsService;
#[cfg(feature = "node-runtime")]
use iroh::EndpointAddr;
#[cfg(feature = "node-runtime")]
use iroh::protocol::Router;
#[cfg(feature = "node-runtime")]
use tokio_util::sync::CancellationToken;

pub use self::types::NodeId;
#[cfg(feature = "node-runtime")]
use crate::AuthenticatedRaftProtocolHandler;
#[cfg(feature = "node-runtime")]
use crate::CLIENT_ALPN;
#[cfg(feature = "node-runtime")]
use crate::ClientProtocolContext;
#[cfg(feature = "node-runtime")]
use crate::ClientProtocolHandler;
#[cfg(feature = "node-runtime")]
use crate::RaftProtocolHandler;
#[cfg(feature = "node-runtime")]
use crate::TrustedPeersRegistry;
#[cfg(feature = "node-runtime")]
use crate::api::ClusterController;
#[cfg(feature = "node-runtime")]
use crate::api::KeyValueStore;
#[cfg(feature = "node-runtime")]
use crate::cluster::bootstrap::NodeHandle;
#[cfg(feature = "node-runtime")]
use crate::cluster::bootstrap::bootstrap_node;
use crate::cluster::config::NodeConfig;
#[cfg(all(feature = "node-runtime", feature = "federation"))]
use crate::cluster::federation::ClusterIdentity;
#[cfg(all(feature = "node-runtime", feature = "federation"))]
use crate::cluster::federation::DirectResourceResolver;
#[cfg(all(feature = "node-runtime", feature = "federation"))]
use crate::cluster::federation::FEDERATION_ALPN;
#[cfg(all(feature = "node-runtime", feature = "federation"))]
use crate::cluster::federation::FederatedId;
#[cfg(all(feature = "node-runtime", feature = "federation"))]
use crate::cluster::federation::FederationProtocolHandler;
#[cfg(all(feature = "node-runtime", feature = "federation"))]
use crate::cluster::federation::FederationResourceResolver;
#[cfg(all(feature = "node-runtime", feature = "federation"))]
use crate::cluster::federation::FederationSettings;
#[cfg(all(feature = "node-runtime", feature = "federation"))]
use crate::cluster::federation::TrustManager;
#[cfg(all(feature = "node-runtime", feature = "federation"))]
use crate::cluster::federation::sync::FederationProtocolContext;
#[cfg(feature = "node-runtime")]
use crate::protocol_adapters::EndpointProviderAdapter;
#[cfg(feature = "node-runtime")]
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
/// ```ignore
/// use aspen::node::NodeBuilder;
/// use aspen::raft::storage::StorageBackend;
/// use aspen::raft::types::NodeId;
///
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let node = NodeBuilder::new(NodeId(1), "./data/node-1")
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

    /// Enable/disable secrets management (default: false).
    ///
    /// When enabled, the node provides KV, Transit, and PKI secrets engines
    /// through the Client RPC interface.
    ///
    /// # Feature
    ///
    /// Requires the `secrets` feature.
    #[cfg(feature = "secrets")]
    pub fn with_secrets(mut self, enable: bool) -> Self {
        self.config.secrets.is_enabled = enable;
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
    ///
    /// # Feature Requirements
    ///
    /// This method requires the `jobs`, `docs`, and `hooks` features to be enabled.
    #[cfg(feature = "node-runtime")]
    pub async fn start(self) -> Result<Node> {
        use anyhow::Context;

        // Validate configuration before expensive bootstrap
        self.config.validate().context("configuration validation failed")?;

        let handle = bootstrap_node(self.config).await?;
        Ok(Node {
            handle,
            router: None,
            membership_watcher_cancel: None,
            #[cfg(feature = "federation")]
            federation_identity: None,
            #[cfg(feature = "federation")]
            federation_trust_manager: None,
            #[cfg(feature = "federation")]
            federation_resource_settings: None,
            #[cfg(feature = "hooks")]
            ephemeral_broker: None,
        })
    }
}

/// Handle returned by [`NodeBuilder::start`].
///
/// Wraps a [`NodeHandle`] and provides convenient access to the
/// node's components for integration testing and programmatic usage.
///
/// # Feature Requirements
///
/// This struct requires the `jobs`, `docs`, `hooks`, and `federation` features to be enabled.
#[cfg(feature = "node-runtime")]
pub struct Node {
    handle: NodeHandle,
    /// Router for handling incoming protocol connections.
    /// Stored to keep it alive - dropping the Router shuts down protocol handling.
    router: Option<Router>,
    /// Cancellation token for the membership watcher task.
    /// Used to gracefully shut down the watcher when the node shuts down.
    membership_watcher_cancel: Option<CancellationToken>,
    /// Federation cluster identity (if federation is enabled).
    #[cfg(feature = "federation")]
    federation_identity: Option<ClusterIdentity>,
    /// Federation trust manager (if federation is enabled).
    #[cfg(feature = "federation")]
    federation_trust_manager: Option<Arc<TrustManager>>,
    /// Federation resource settings (if federation is enabled).
    /// Uses tokio::sync::RwLock for async compatibility with FederationResourceResolver.
    #[cfg(feature = "federation")]
    federation_resource_settings: Option<Arc<tokio::sync::RwLock<HashMap<FederatedId, FederationSettings>>>>,
    /// Ephemeral pub/sub broker for fire-and-forget event streaming.
    /// Created during router spawn, shared with EphemeralProtocolHandler.
    #[cfg(feature = "hooks")]
    ephemeral_broker: Option<Arc<aspen_hooks::pubsub::EphemeralBroker>>,
}

#[cfg(feature = "node-runtime")]
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

    /// Get the ephemeral pub/sub broker.
    ///
    /// Available after `spawn_router()` or `spawn_router_with_blobs()` is called.
    /// Use this to publish ephemeral events that are streamed to connected subscribers
    /// without Raft consensus.
    #[cfg(feature = "hooks")]
    pub fn ephemeral_broker(&self) -> Option<&Arc<aspen_hooks::pubsub::EphemeralBroker>> {
        self.ephemeral_broker.as_ref()
    }

    /// Get the federation cluster identity (if federation is enabled).
    #[cfg(feature = "federation")]
    pub fn federation_identity(&self) -> Option<&ClusterIdentity> {
        self.federation_identity.as_ref()
    }

    /// Get the federation trust manager (if federation is enabled).
    #[cfg(feature = "federation")]
    pub fn federation_trust_manager(&self) -> Option<&Arc<TrustManager>> {
        self.federation_trust_manager.as_ref()
    }

    /// Get the federation resource settings (if federation is enabled).
    #[cfg(feature = "federation")]
    pub fn federation_resource_settings(
        &self,
    ) -> Option<&Arc<tokio::sync::RwLock<HashMap<FederatedId, FederationSettings>>>> {
        self.federation_resource_settings.as_ref()
    }

    #[cfg(feature = "secrets")]
    fn finish_secrets_service_setup(
        mount_registry: Arc<aspen_secrets::MountRegistry>,
        provider: Result<Option<Arc<dyn aspen_secrets::SecretsEncryptionProvider>>, String>,
    ) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
        #[cfg(feature = "trust")]
        {
            match provider {
                Ok(Some(provider)) => {
                    mount_registry.set_encryption_provider(provider);
                    tracing::info!("Secrets service initialized with trust-aware secrets-at-rest support");
                    Some(Arc::new(SecretsService::new(mount_registry)) as Arc<dyn std::any::Any + Send + Sync>)
                }
                Ok(None) => {
                    tracing::info!("Secrets service initialized without trust-aware secrets-at-rest provider");
                    Some(Arc::new(SecretsService::new(mount_registry)) as Arc<dyn std::any::Any + Send + Sync>)
                }
                Err(error) => {
                    tracing::error!(
                        error,
                        "failed to initialize trust-aware secrets-at-rest provider; disabling secrets service"
                    );
                    None
                }
            }
        }

        #[cfg(not(feature = "trust"))]
        {
            let _ = provider;
            tracing::info!("Secrets service initialized with multi-mount support");
            Some(Arc::new(SecretsService::new(mount_registry)) as Arc<dyn std::any::Any + Send + Sync>)
        }
    }

    /// Create a ClientProtocolContext for the Client RPC handler.
    ///
    /// This context provides all the dependencies needed by the Client RPC handlers.
    /// Many fields are optional and will be None when the corresponding feature
    /// is not enabled or configured.
    fn create_native_handler_plan(&self) -> crate::NativeHandlerPlan {
        let mut plan = crate::NativeHandlerPlan::core_only();
        plan.set_net_enabled(true);
        plan.set_blob_enabled(false);
        plan.set_docs_enabled(false);
        #[cfg(feature = "federation")]
        plan.set_forge_enabled(self.federation_identity.is_some());
        #[cfg(not(feature = "federation"))]
        plan.set_forge_enabled(false);
        plan.set_jobs_enabled(false);
        plan.set_ci_enabled(false);
        plan.set_cache_enabled(false);
        plan.set_secrets_enabled(false);
        plan.set_snix_enabled(true);
        plan
    }

    fn create_client_protocol_context(&self) -> ClientProtocolContext {
        let raft_node = self.handle.storage.raft_node.clone();

        // Create endpoint provider adapter
        let endpoint_manager: Arc<dyn aspen_core::EndpointProvider> =
            Arc::new(EndpointProviderAdapter::new(self.handle.network.iroh_manager.clone()));

        // Create secrets service — always available when the feature is compiled in.
        // The secrets engine (KV, Transit, PKI) uses the Raft KV store as its
        // backend and does not require SOPS configuration.
        #[cfg(feature = "secrets")]
        let secrets_service = {
            let mount_registry =
                Arc::new(aspen_secrets::MountRegistry::new(raft_node.clone() as Arc<dyn aspen_core::KeyValueStore>));

            #[cfg(feature = "trust")]
            let provider = if aspen_raft::secrets_at_rest::has_runtime_trust_configuration(raft_node.as_ref()) {
                aspen_raft::secrets_at_rest::build_trust_aware_secrets_provider(
                    raft_node.clone(),
                    raft_node.clone() as Arc<dyn aspen_core::KeyValueStore>,
                )
                .map(Some)
            } else {
                Ok(None)
            };

            #[cfg(not(feature = "trust"))]
            let provider: Result<Option<Arc<dyn aspen_secrets::SecretsEncryptionProvider>>, String> = Ok(None);

            Self::finish_secrets_service_setup(mount_registry, provider)
        };

        ClientProtocolContext {
            node_id: self.handle.config.node_id,
            controller: raft_node.clone(),
            kv_store: raft_node.clone(),
            #[cfg(feature = "sql")]
            sql_executor: raft_node.clone(),
            state_machine: Some(self.handle.storage.state_machine.clone()),
            endpoint_manager,
            #[cfg(feature = "blob")]
            blob_store: None, // Would need IrohBlobStore passed in
            #[cfg(feature = "blob")]
            blob_replication_manager: self.handle.blob_replication.replication_manager.clone(),
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
            // Job-related fields are only available when jobs feature is enabled
            // The jobs feature enables aspen-rpc-core's jobs and worker features
            #[cfg(feature = "jobs")]
            job_manager: None,
            // worker_service requires aspen-rpc-core's worker feature (enabled by jobs)
            #[cfg(feature = "jobs")]
            worker_service: None,
            #[cfg(feature = "jobs")]
            worker_coordinator: None,
            watch_registry: None,
            #[cfg(feature = "hooks")]
            hook_service: self.handle.hooks.hook_service.clone(),
            hooks_config: self.handle.config.hooks.clone(),
            #[cfg(feature = "secrets")]
            secrets_service,
            #[cfg(not(feature = "secrets"))]
            secrets_service: None,
            #[cfg(all(feature = "forge", feature = "federation"))]
            federation_identity: self.federation_identity.as_ref().map(|id| Arc::new(id.to_signed())),
            #[cfg(all(feature = "forge", not(feature = "federation")))]
            federation_identity: None,
            #[cfg(all(feature = "forge", feature = "federation"))]
            federation_trust_manager: self.federation_trust_manager.clone(),
            #[cfg(all(feature = "forge", not(feature = "federation")))]
            federation_trust_manager: None,
            #[cfg(all(feature = "forge", feature = "federation"))]
            federation_cluster_identity: self.federation_identity.as_ref().map(|id| Arc::new(id.clone())),
            #[cfg(all(feature = "forge", not(feature = "federation")))]
            federation_cluster_identity: None,
            #[cfg(feature = "forge")]
            iroh_endpoint: Some(Arc::new(self.handle.network.iroh_manager.endpoint().clone())),
            #[cfg(all(feature = "forge", feature = "global-discovery"))]
            federation_discovery: None,
            #[cfg(feature = "ci")]
            ci_orchestrator: None,
            #[cfg(feature = "ci")]
            ci_trigger_service: None,
            #[cfg(feature = "nix-cache-gateway")]
            nix_cache_signer: None,
            service_executors: Vec::new(),
            app_registry: aspen_core::shared_registry(),
            proxy_config: aspen_rpc_handlers::aspen_rpc_core::ProxyConfig::default(),
            prometheus_handle: None,
            network_metrics: None,
            span_forwarder: None,
            #[cfg(feature = "deploy")]
            drain_state: None,
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
    #[allow(deprecated)] // Legacy method supports both auth and non-auth handlers
    pub fn spawn_router(&mut self) -> Result<()> {
        use crate::RAFT_ALPN;
        use crate::RAFT_AUTH_ALPN;

        let raft_core_transport = self.handle.storage.raft_node.raft().as_ref().clone();

        let mut builder = Router::builder(self.handle.network.iroh_manager.endpoint().clone());

        // Tiger Style: Only clone if we need both handlers. If auth is enabled,
        // we need two copies (one for each handler). Otherwise, just use the original.
        let auth_enabled = self.handle.config.iroh.enable_raft_auth;

        // Create legacy handler - clone only if we also need the auth handler
        let (raft_handler, raft_for_auth) = if auth_enabled {
            (RaftProtocolHandler::new(raft_core_transport.clone()), Some(raft_core_transport))
        } else {
            (RaftProtocolHandler::new(raft_core_transport), None)
        };

        // Always register legacy unauthenticated handler for backwards compatibility
        builder = builder.accept(RAFT_ALPN, raft_handler);
        tracing::info!("registered Raft RPC protocol handler (ALPN: raft-rpc)");

        // Register authenticated handler if enabled
        if let Some(raft_core_for_auth) = raft_for_auth {
            use crate::raft::membership_watcher::spawn_membership_watcher;

            // Pre-populate TrustedPeersRegistry with this node's own identity
            // This allows self-connections and provides the starting point for
            // membership-based peer authorization.
            let our_public_key = self.handle.network.iroh_manager.node_addr().id;
            let trusted_peers = TrustedPeersRegistry::with_peers([our_public_key]);

            // Spawn the membership watcher to keep TrustedPeersRegistry in sync with Raft membership.
            // The watcher monitors Raft metrics for membership changes and updates the registry
            // with PublicKeys parsed from each RaftMemberInfo endpoint identifier.
            let watcher_cancel =
                spawn_membership_watcher(self.handle.storage.raft_node.raft().clone(), trusted_peers.clone());
            self.membership_watcher_cancel = Some(watcher_cancel);

            #[cfg(feature = "trust")]
            let auth_handler = AuthenticatedRaftProtocolHandler::new(raft_core_for_auth, trusted_peers)
                .with_expunged_flag(self.handle.storage.raft_node.expunged_flag().clone());
            #[cfg(not(feature = "trust"))]
            let auth_handler = AuthenticatedRaftProtocolHandler::new(raft_core_for_auth, trusted_peers);
            builder = builder.accept(RAFT_AUTH_ALPN, auth_handler);
            tracing::info!(
                our_public_key = %our_public_key,
                "registered authenticated Raft RPC protocol handler with membership sync (ALPN: raft-auth)"
            );
        }

        // Add gossip handler if enabled
        if let Some(gossip) = self.handle.network.iroh_manager.gossip() {
            use iroh_gossip::ALPN as GOSSIP_ALPN;
            let gossip_handler = gossip.clone();
            builder = builder.accept(GOSSIP_ALPN, gossip_handler);
            tracing::info!("registered Gossip protocol handler");
        }

        // Add federation handler if enabled.
        // IMPORTANT: Must run before creating the client protocol context so
        // that self.federation_identity is populated when the context is built.
        #[cfg(feature = "federation")]
        if self.handle.config.federation.is_enabled {
            let fed_config = &self.handle.config.federation;

            // Load or generate cluster identity
            let cluster_identity = if let Some(key_hex) = fed_config.cluster_key.as_ref() {
                // Load from config
                match ClusterIdentity::from_hex_key(key_hex, fed_config.cluster_name.clone()) {
                    Ok(identity) => identity,
                    Err(e) => {
                        tracing::error!("Invalid federation cluster_key: {}", e);
                        tracing::warn!("Federation disabled due to invalid key");
                        // Skip federation registration
                        self.router = Some(builder.spawn());
                        tracing::info!("Iroh Router spawned with ALPN-based protocol dispatching");
                        return Ok(());
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
            // Note: Use tokio::sync::RwLock as required by DirectResourceResolver
            let resource_settings = Arc::new(tokio::sync::RwLock::new(HashMap::new()));

            // Create resource resolver using RaftNode (implements KeyValueStore)
            let resource_resolver: Option<Arc<dyn FederationResourceResolver>> = {
                let raft_node = self.handle.storage.raft_node.clone();
                Some(Arc::new(DirectResourceResolver::new(raft_node, resource_settings.clone())))
            };

            // Create federation protocol context and handler
            let context = FederationProtocolContext {
                cluster_identity: cluster_identity.clone(),
                trust_manager: trust_manager.clone(),
                resource_settings: resource_settings.clone(),
                endpoint: Arc::new(self.handle.network.iroh_manager.endpoint().clone()),
                hlc: Arc::new(aspen_core::hlc::create_hlc(&self.handle.config.node_id.to_string())),
                resource_resolver,
                session_credential: std::sync::Mutex::new(None),
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

        // Add Client RPC protocol handler.
        // Must come after federation setup so the context captures federation_identity.
        let client_context = self.create_client_protocol_context();
        let client_handler = ClientProtocolHandler::new(client_context, self.create_native_handler_plan())?;
        builder = builder.accept(CLIENT_ALPN, client_handler);
        tracing::info!("registered Client RPC protocol handler (ALPN: aspen-client)");

        // Add ephemeral pub/sub protocol handler
        // Enables fire-and-forget event streaming without Raft consensus
        #[cfg(feature = "hooks")]
        {
            use aspen_hooks::pubsub::ephemeral::handler::EPHEMERAL_ALPN;
            use aspen_hooks::pubsub::ephemeral::handler::EphemeralProtocolHandler;

            let broker = Arc::new(aspen_hooks::pubsub::EphemeralBroker::new());
            let ephemeral_handler = EphemeralProtocolHandler::new(Arc::clone(&broker));
            builder = builder.accept(EPHEMERAL_ALPN, ephemeral_handler);
            self.ephemeral_broker = Some(broker);
            tracing::info!("registered ephemeral pub/sub protocol handler (ALPN: aspen-ephemeral/0)");
        }

        // Spawn the router and store the handle to keep it alive
        // Dropping the Router would shut down protocol handling!
        self.router = Some(builder.spawn());
        tracing::info!("Iroh Router spawned with ALPN-based protocol dispatching");
        Ok(())
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
    #[cfg(feature = "blob")]
    #[allow(deprecated)] // Legacy method supports both auth and non-auth handlers
    pub fn spawn_router_with_blobs(&mut self, blobs_handler: iroh_blobs::BlobsProtocol) -> Result<()> {
        use crate::RAFT_ALPN;
        use crate::RAFT_AUTH_ALPN;

        let raft_core_transport = self.handle.storage.raft_node.raft().as_ref().clone();
        let auth_enabled = self.handle.config.iroh.enable_raft_auth;

        // Tiger Style: Only clone when auth is enabled (we need raft_core_transport twice).
        // When auth is disabled, we move raft_core_transport directly into the handler.
        let (raft_handler, raft_for_auth) = if auth_enabled {
            (RaftProtocolHandler::new(raft_core_transport.clone()), Some(raft_core_transport))
        } else {
            (RaftProtocolHandler::new(raft_core_transport), None)
        };

        let mut builder = Router::builder(self.handle.network.iroh_manager.endpoint().clone());

        // Always register legacy unauthenticated handler for backwards compatibility
        builder = builder.accept(RAFT_ALPN, raft_handler);
        tracing::info!("registered Raft RPC protocol handler (ALPN: raft-rpc)");

        // Register authenticated handler if enabled
        if let Some(raft_core_transport) = raft_for_auth {
            use crate::raft::membership_watcher::spawn_membership_watcher;

            let our_public_key = self.handle.network.iroh_manager.node_addr().id;
            let trusted_peers = TrustedPeersRegistry::with_peers([our_public_key]);

            let watcher_cancel =
                spawn_membership_watcher(self.handle.storage.raft_node.raft().clone(), trusted_peers.clone());
            self.membership_watcher_cancel = Some(watcher_cancel);

            #[cfg(feature = "trust")]
            let auth_handler = AuthenticatedRaftProtocolHandler::new(raft_core_transport, trusted_peers)
                .with_expunged_flag(self.handle.storage.raft_node.expunged_flag().clone());
            #[cfg(not(feature = "trust"))]
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
            let gossip_handler = gossip.clone();
            builder = builder.accept(GOSSIP_ALPN, gossip_handler);
            tracing::info!("registered Gossip protocol handler");
        }

        // Add federation handler if enabled.
        // IMPORTANT: Must run before creating the client protocol context so
        // that self.federation_identity is populated when the context is built.
        #[cfg(feature = "federation")]
        if self.handle.config.federation.is_enabled {
            let hlc = Arc::new(aspen_core::hlc::create_hlc(&self.handle.config.node_id.to_string()));

            // Create Forge resource resolver if forge feature is enabled.
            // NOTE: Created without git exporter because blob store isn't
            // available yet at federation init time. The c2e index for
            // federation DAG dedup is populated during git push import instead.
            #[cfg(feature = "forge")]
            let resolver: Option<Arc<dyn aspen_cluster::federation::FederationResourceResolver>> = {
                let kv = self.handle.storage.raft_node.clone();
                Some(Arc::new(aspen_forge::resolver::ForgeResourceResolver::new(kv)))
            };
            #[cfg(not(feature = "forge"))]
            let resolver: Option<Arc<dyn aspen_cluster::federation::FederationResourceResolver>> = None;

            match aspen_cluster::bootstrap::setup_federation(
                &self.handle.config,
                self.handle.network.iroh_manager.endpoint(),
                &hlc,
                resolver,
            ) {
                Some(fed) => {
                    builder = builder.accept(FEDERATION_ALPN, fed.handler);
                    tracing::info!(
                        cluster_name = %fed.identity.name(),
                        cluster_key = %fed.identity.public_key(),
                        "registered Federation protocol handler (ALPN: /aspen/federation/1)"
                    );

                    // Store on NodeHandle for RPC handler access
                    self.handle.federation = aspen_cluster::bootstrap::resources::FederationResources {
                        identity: Some(fed.identity.clone()),
                        trust_manager: Some(fed.trust_manager.clone()),
                    };

                    // Also store on Node for backward compatibility with existing accessors
                    self.federation_identity = Some((*fed.identity).clone());
                    self.federation_trust_manager = Some(fed.trust_manager);
                }
                None => {
                    tracing::warn!("federation enabled but initialization failed, skipping");
                }
            }
        }

        // Add Client RPC protocol handler.
        // Must come after federation setup so the context captures federation_identity.
        let client_context = self.create_client_protocol_context();
        let client_handler = ClientProtocolHandler::new(client_context, self.create_native_handler_plan())?;
        builder = builder.accept(CLIENT_ALPN, client_handler);
        tracing::info!("registered Client RPC protocol handler (ALPN: aspen-client)");

        // Add ephemeral pub/sub protocol handler
        #[cfg(feature = "hooks")]
        {
            use aspen_hooks::pubsub::ephemeral::handler::EPHEMERAL_ALPN;
            use aspen_hooks::pubsub::ephemeral::handler::EphemeralProtocolHandler;

            let broker = Arc::new(aspen_hooks::pubsub::EphemeralBroker::new());
            let ephemeral_handler = EphemeralProtocolHandler::new(Arc::clone(&broker));
            builder = builder.accept(EPHEMERAL_ALPN, ephemeral_handler);
            self.ephemeral_broker = Some(broker);
            tracing::info!("registered ephemeral pub/sub protocol handler (ALPN: aspen-ephemeral/0)");
        }

        // Register the blobs protocol handler
        builder = builder.accept(iroh_blobs::ALPN, blobs_handler);
        tracing::info!("registered Blobs protocol handler (ALPN: iroh-blobs/0)");

        // Spawn the router and store the handle to keep it alive
        self.router = Some(builder.spawn());
        tracing::info!("Iroh Router spawned with ALPN-based protocol dispatching (including blobs)");
        Ok(())
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
