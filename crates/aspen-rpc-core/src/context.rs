//! Client protocol context.
//!
//! Contains the shared context and dependencies needed by RPC handlers.
//! This is the central dependency container that handlers receive to access
//! cluster services.

use std::sync::Arc;
use std::time::Instant;

use aspen_auth::TokenVerifier;
use aspen_client_api::IngestSpan;
use aspen_core::ClusterController;
#[cfg(feature = "global-discovery")]
use aspen_core::ContentDiscovery;
use aspen_core::DocsSyncProvider;
use aspen_core::EndpointProvider;
use aspen_core::KeyValueStore;
use aspen_core::NetworkFactory;
use aspen_core::PeerManager;
use aspen_core::SharedAppRegistry;
use aspen_core::WatchRegistry;
use aspen_raft::StateMachineVariant;
use aspen_raft::connection_pool::ConnectionPoolMetrics;
use aspen_sharding::ShardTopology;

use crate::proxy::ProxyConfig;

/// Provider trait for connection pool metrics.
///
/// Abstracts the concrete `RaftConnectionPool<T>` behind a trait object
/// so the context doesn't need the transport generic parameter.
#[async_trait::async_trait]
pub trait NetworkMetricsProvider: Send + Sync {
    /// Collect current connection pool metrics.
    async fn metrics(&self) -> ConnectionPoolMetrics;

    /// Get the snapshot transfer history, if available.
    fn snapshot_history(&self) -> Vec<aspen_transport::snapshot_history::SnapshotTransferEntry> {
        Vec::new()
    }
}

/// Optional forwarder for exporting trace spans to an external collector.
///
/// When configured, the `TraceIngest` handler sends spans both to KV storage
/// and to this forwarder (e.g., an OTLP trace exporter).
#[async_trait::async_trait]
pub trait SpanForwarder: Send + Sync {
    /// Forward a batch of ingested spans to an external collector.
    ///
    /// Errors are logged but do not fail the ingest — KV storage is the
    /// primary store, OTLP is best-effort.
    async fn forward(&self, spans: &[IngestSpan]);
}

/// Adapter that implements `NetworkMetricsProvider` for any `RaftConnectionPool<T>`.
pub struct PoolMetricsAdapter<T>
where T: aspen_core::NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static
{
    pool: Arc<aspen_raft::connection_pool::RaftConnectionPool<T>>,
    snapshot_history: Option<Arc<aspen_transport::snapshot_history::SnapshotTransferHistory>>,
}

impl<T> PoolMetricsAdapter<T>
where T: aspen_core::NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static
{
    /// Create a new adapter wrapping a connection pool.
    pub fn new(pool: Arc<aspen_raft::connection_pool::RaftConnectionPool<T>>) -> Self {
        Self {
            pool,
            snapshot_history: None,
        }
    }

    /// Attach a snapshot transfer history.
    pub fn with_snapshot_history(
        mut self,
        history: Arc<aspen_transport::snapshot_history::SnapshotTransferHistory>,
    ) -> Self {
        self.snapshot_history = Some(history);
        self
    }
}

#[async_trait::async_trait]
impl<T> NetworkMetricsProvider for PoolMetricsAdapter<T>
where T: aspen_core::NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static
{
    async fn metrics(&self) -> ConnectionPoolMetrics {
        self.pool.metrics().await
    }

    fn snapshot_history(&self) -> Vec<aspen_transport::snapshot_history::SnapshotTransferEntry> {
        self.snapshot_history.as_ref().map(|h| h.recent()).unwrap_or_default()
    }
}

/// Context for Client protocol handler with all dependencies.
///
/// This struct contains all the services and configuration needed by RPC handlers.
/// It is constructed during server initialization and passed to each handler.
///
/// # Feature-Gated Fields
///
/// Many fields are optional and gated behind feature flags:
/// - `sql`: SQL query executor
/// - `blob`: Blob store and replication manager
/// - `forge`: Forge node and federation services
/// - `ci`: CI orchestrator and trigger service
/// - `secrets`: Secrets service
/// - `global-discovery`: Content discovery service
///
/// # Tiger Style
///
/// - Context is immutable after construction (Clone-friendly)
/// - All fields use Arc for shared ownership
/// - Optional fields use Option<Arc<T>> pattern
#[derive(Clone)]
pub struct ClientProtocolContext {
    /// Node identifier.
    pub node_id: u64,
    /// Cluster controller for Raft operations.
    pub controller: Arc<dyn ClusterController>,
    /// Key-value store interface.
    pub kv_store: Arc<dyn KeyValueStore>,
    /// SQL query executor for read-only SQL queries.
    #[cfg(feature = "sql")]
    pub sql_executor: Arc<dyn aspen_sql::SqlQueryExecutor>,
    /// State machine for direct reads (lease queries, etc.).
    pub state_machine: Option<StateMachineVariant>,
    /// Endpoint provider for peer info.
    pub endpoint_manager: Arc<dyn EndpointProvider>,
    /// Blob store for content-addressed storage (optional).
    #[cfg(feature = "blob")]
    pub blob_store: Option<Arc<aspen_blob::IrohBlobStore>>,
    /// Blob replication manager for coordinating blob replication across nodes (optional).
    #[cfg(feature = "blob")]
    pub blob_replication_manager: Option<aspen_blob::BlobReplicationManager>,
    /// Peer manager for cluster-to-cluster sync (optional).
    pub peer_manager: Option<Arc<dyn PeerManager>>,
    /// Docs sync resources for iroh-docs operations (optional).
    pub docs_sync: Option<Arc<dyn DocsSyncProvider>>,
    /// Cluster cookie for ticket generation.
    pub cluster_cookie: String,
    /// Node start time for uptime calculation.
    pub start_time: Instant,
    /// Network factory for dynamic peer addition (optional).
    pub network_factory: Option<Arc<dyn NetworkFactory>>,
    /// Token verifier for capability-based authorization.
    pub token_verifier: Option<Arc<TokenVerifier>>,
    /// Whether to require authentication for all authorized requests.
    pub require_auth: bool,
    /// Shard topology for GetTopology RPC (optional).
    pub topology: Option<Arc<tokio::sync::RwLock<ShardTopology>>>,
    /// Content discovery service for DHT announcements and provider lookup (optional).
    #[cfg(feature = "global-discovery")]
    pub content_discovery: Option<Arc<dyn ContentDiscovery>>,
    /// Forge node for decentralized Git operations (optional).
    #[cfg(feature = "forge")]
    pub forge_node: Option<Arc<aspen_forge::ForgeNode<aspen_blob::IrohBlobStore, dyn aspen_core::KeyValueStore>>>,
    /// Job manager for distributed job queue operations (optional).
    #[cfg(feature = "jobs")]
    pub job_manager: Option<Arc<aspen_jobs::JobManager<dyn KeyValueStore>>>,
    /// Worker service for querying worker status (optional).
    #[cfg(feature = "worker")]
    pub worker_service: Option<Arc<aspen_cluster::worker_service::WorkerService>>,
    /// Distributed worker coordinator for external worker registration (optional).
    #[cfg(feature = "jobs")]
    pub worker_coordinator: Option<Arc<aspen_coordination::DistributedWorkerCoordinator<dyn KeyValueStore>>>,
    /// Watch registry for tracking active watch subscriptions (optional).
    pub watch_registry: Option<Arc<dyn WatchRegistry>>,
    /// Hook service for event-driven automation (optional).
    #[cfg(feature = "hooks")]
    pub hook_service: Option<Arc<aspen_hooks::HookService>>,
    /// Hooks configuration for handler metadata.
    pub hooks_config: aspen_hooks_types::HooksConfig,
    /// Secrets service for Vault-compatible secrets management (optional).
    ///
    /// Type-erased to avoid circular dependency with aspen-rpc-handlers.
    /// Use `secrets_service.as_ref().and_then(|s| s.downcast_ref::<T>())` to access.
    pub secrets_service: Option<Arc<dyn std::any::Any + Send + Sync>>,
    /// Federation cluster identity (optional).
    #[cfg(feature = "forge")]
    pub federation_identity: Option<Arc<aspen_cluster::federation::SignedClusterIdentity>>,
    /// Federation trust manager (optional).
    #[cfg(feature = "forge")]
    pub federation_trust_manager: Option<Arc<aspen_cluster::federation::TrustManager>>,
    /// Cluster identity with signing key for outbound federation handshakes.
    #[cfg(feature = "forge")]
    pub federation_cluster_identity: Option<Arc<aspen_cluster::federation::ClusterIdentity>>,
    /// Iroh endpoint for outbound federation connections.
    #[cfg(feature = "forge")]
    pub iroh_endpoint: Option<Arc<iroh::Endpoint>>,
    /// Federation discovery service (optional).
    #[cfg(all(feature = "forge", feature = "global-discovery"))]
    pub federation_discovery: Option<Arc<aspen_cluster::federation::FederationDiscoveryService>>,
    /// CI pipeline orchestrator for triggering and monitoring pipelines (optional).
    #[cfg(feature = "ci")]
    pub ci_orchestrator: Option<Arc<aspen_ci::PipelineOrchestrator<dyn aspen_core::KeyValueStore>>>,
    /// CI trigger service for watching repositories (optional).
    /// Requires both `ci` and `nickel` features since TriggerService depends on Nickel config.
    #[cfg(all(feature = "ci", feature = "nickel"))]
    pub ci_trigger_service: Option<Arc<aspen_ci::TriggerService>>,
    /// Service executors for WASM plugin host function dispatch.
    ///
    /// Each executor handles a domain (docs, jobs, etc.) and is called
    /// by the `service_execute` host function. Created during node setup
    /// by the handler crates that own the concrete service types.
    pub service_executors: Vec<Arc<dyn aspen_core::ServiceExecutor>>,
    /// Application registry for capability-aware dispatch and federation advertisement.
    ///
    /// Populated automatically during handler registration. Used to:
    /// 1. Return `CapabilityUnavailable` responses with the required app name
    /// 2. Advertise capabilities to federated clusters via gossip
    pub app_registry: SharedAppRegistry,
    /// Configuration for cross-cluster request proxying.
    ///
    /// Controls whether requests for unavailable apps are forwarded to discovered clusters.
    pub proxy_config: ProxyConfig,
    /// Prometheus metrics handle for rendering metrics on `GetMetrics` requests.
    ///
    /// Installed once at node startup via `metrics_init::install_prometheus_recorder()`.
    /// All `metrics::counter!()` / `metrics::gauge!()` / `metrics::histogram!()` calls
    /// across the codebase are captured by this recorder and rendered on demand.
    pub prometheus_handle: Option<Arc<metrics_exporter_prometheus::PrometheusHandle>>,
    /// Connection pool metrics provider for `GetNetworkMetrics` handler.
    ///
    /// Set during node bootstrap when the connection pool is created.
    pub network_metrics: Option<Arc<dyn NetworkMetricsProvider>>,
    /// Optional span forwarder for OTLP trace export.
    ///
    /// When set, `TraceIngest` sends spans here in addition to KV storage.
    pub span_forwarder: Option<Arc<dyn SpanForwarder>>,
    /// Shared drain state for graceful node upgrades (optional).
    ///
    /// When present, the client protocol handler checks this before dispatching RPCs.
    /// During drain, new RPCs are rejected with NOT_LEADER so clients fail over.
    /// The same instance is shared with the NodeUpgradeExecutor so drain waits
    /// for real in-flight client RPCs to complete.
    #[cfg(feature = "deploy")]
    pub drain_state: Option<Arc<aspen_cluster::upgrade::DrainState>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingContextCapabilityError {
    capability: &'static str,
}

impl MissingContextCapabilityError {
    fn new(capability: &'static str) -> Self {
        Self { capability }
    }
}

impl std::fmt::Display for MissingContextCapabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "missing client protocol capability: {}", self.capability)
    }
}

impl std::error::Error for MissingContextCapabilityError {}

pub type ContextCapabilityResult<T> = std::result::Result<T, MissingContextCapabilityError>;

#[cfg(feature = "blob")]
#[derive(Clone)]
pub struct BlobHandlerContext {
    pub controller: Arc<dyn ClusterController>,
    pub kv_store: Arc<dyn KeyValueStore>,
    pub endpoint_manager: Arc<dyn EndpointProvider>,
    pub blob_store: Arc<aspen_blob::IrohBlobStore>,
    pub blob_replication_manager: Option<aspen_blob::BlobReplicationManager>,
    #[cfg(feature = "global-discovery")]
    pub content_discovery: Option<Arc<dyn ContentDiscovery>>,
}

#[derive(Clone)]
pub struct DocsHandlerContext {
    pub docs_sync: Arc<dyn DocsSyncProvider>,
    pub peer_manager: Option<Arc<dyn PeerManager>>,
}

#[cfg(feature = "forge")]
#[derive(Clone)]
pub struct ForgeHandlerContext {
    pub node_id: u64,
    pub forge_node: Arc<aspen_forge::ForgeNode<aspen_blob::IrohBlobStore, dyn aspen_core::KeyValueStore>>,
    #[cfg(feature = "global-discovery")]
    pub content_discovery: Option<Arc<dyn ContentDiscovery>>,
    #[cfg(all(feature = "forge", feature = "global-discovery"))]
    pub federation_discovery: Option<Arc<aspen_cluster::federation::FederationDiscoveryService>>,
    pub federation_identity: Option<Arc<aspen_cluster::federation::SignedClusterIdentity>>,
    pub federation_trust_manager: Option<Arc<aspen_cluster::federation::TrustManager>>,
    pub federation_cluster_identity: Option<Arc<aspen_cluster::federation::ClusterIdentity>>,
    pub iroh_endpoint: Option<Arc<iroh::Endpoint>>,
    #[cfg(all(feature = "hooks", feature = "git-bridge"))]
    pub hook_service: Option<Arc<aspen_hooks::HookService>>,
}

#[cfg(feature = "ci")]
#[derive(Clone)]
pub struct CiHandlerContext {
    pub node_id: u64,
    pub controller: Arc<dyn ClusterController>,
    pub kv_store: Arc<dyn KeyValueStore>,
    pub endpoint_manager: Arc<dyn EndpointProvider>,
    pub ci_orchestrator: Option<Arc<aspen_ci::PipelineOrchestrator<dyn aspen_core::KeyValueStore>>>,
    #[cfg(all(feature = "ci", feature = "nickel"))]
    pub ci_trigger_service: Option<Arc<aspen_ci::TriggerService>>,
    #[cfg(all(feature = "forge", feature = "blob"))]
    pub forge_node: Option<Arc<aspen_forge::ForgeNode<aspen_blob::IrohBlobStore, dyn aspen_core::KeyValueStore>>>,
    #[cfg(feature = "blob")]
    pub blob_store: Option<Arc<aspen_blob::IrohBlobStore>>,
}

#[cfg(feature = "jobs")]
#[derive(Clone)]
pub struct JobsHandlerContext {
    pub node_id: u64,
    pub kv_store: Arc<dyn KeyValueStore>,
    pub job_manager: Arc<aspen_jobs::JobManager<dyn KeyValueStore>>,
    pub worker_service: Option<Arc<aspen_cluster::worker_service::WorkerService>>,
    pub worker_coordinator: Option<Arc<aspen_coordination::DistributedWorkerCoordinator<dyn KeyValueStore>>>,
}

impl std::fmt::Debug for ClientProtocolContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientProtocolContext")
            .field("node_id", &self.node_id)
            .field("cluster_cookie", &self.cluster_cookie)
            .finish_non_exhaustive()
    }
}

impl ClientProtocolContext {
    fn require_some<T: Clone>(value: &Option<T>, capability: &'static str) -> ContextCapabilityResult<T> {
        value.clone().ok_or_else(|| MissingContextCapabilityError::new(capability))
    }

    /// Log security warnings about authentication configuration.
    ///
    /// Call this during server initialization to alert operators about
    /// potentially insecure configurations.
    pub fn log_auth_warnings(&self) {
        if !self.require_auth {
            tracing::warn!(
                target: "aspen_rpc::security",
                node_id = self.node_id,
                "SECURITY WARNING: require_auth=false - unauthenticated RPC requests are allowed. \
                 Set require_auth=true for production deployments."
            );
        }

        if self.token_verifier.is_none() {
            tracing::warn!(
                target: "aspen_rpc::security",
                node_id = self.node_id,
                "SECURITY WARNING: token_verifier not configured - capability-based authorization disabled. \
                 Configure token_verifier for production deployments."
            );
        }

        if self.require_auth && self.token_verifier.is_some() {
            tracing::info!(
                target: "aspen_rpc::security",
                node_id = self.node_id,
                "Authentication configured: require_auth=true, token_verifier=enabled"
            );
        }
    }

    #[cfg(feature = "blob")]
    pub fn blob_handler_context(&self) -> ContextCapabilityResult<BlobHandlerContext> {
        Ok(BlobHandlerContext {
            controller: Arc::clone(&self.controller),
            kv_store: Arc::clone(&self.kv_store),
            endpoint_manager: Arc::clone(&self.endpoint_manager),
            blob_store: Self::require_some(&self.blob_store, "blob_store")?,
            blob_replication_manager: self.blob_replication_manager.clone(),
            #[cfg(feature = "global-discovery")]
            content_discovery: self.content_discovery.clone(),
        })
    }

    pub fn docs_handler_context(&self) -> ContextCapabilityResult<DocsHandlerContext> {
        Ok(DocsHandlerContext {
            docs_sync: Self::require_some(&self.docs_sync, "docs_sync")?,
            peer_manager: self.peer_manager.clone(),
        })
    }

    #[cfg(feature = "forge")]
    pub fn forge_handler_context(&self) -> ContextCapabilityResult<ForgeHandlerContext> {
        Ok(ForgeHandlerContext {
            node_id: self.node_id,
            forge_node: Self::require_some(&self.forge_node, "forge_node")?,
            #[cfg(feature = "global-discovery")]
            content_discovery: self.content_discovery.clone(),
            #[cfg(all(feature = "forge", feature = "global-discovery"))]
            federation_discovery: self.federation_discovery.clone(),
            federation_identity: self.federation_identity.clone(),
            federation_trust_manager: self.federation_trust_manager.clone(),
            federation_cluster_identity: self.federation_cluster_identity.clone(),
            iroh_endpoint: self.iroh_endpoint.clone(),
            #[cfg(all(feature = "hooks", feature = "git-bridge"))]
            hook_service: self.hook_service.clone(),
        })
    }

    #[cfg(feature = "ci")]
    pub fn ci_handler_context(&self) -> ContextCapabilityResult<CiHandlerContext> {
        let has_ci = self.ci_orchestrator.is_some();
        #[cfg(all(feature = "ci", feature = "nickel"))]
        let has_ci = has_ci || self.ci_trigger_service.is_some();
        if !has_ci {
            return Err(MissingContextCapabilityError::new("ci_orchestrator_or_trigger"));
        }

        Ok(CiHandlerContext {
            node_id: self.node_id,
            controller: Arc::clone(&self.controller),
            kv_store: Arc::clone(&self.kv_store),
            endpoint_manager: Arc::clone(&self.endpoint_manager),
            ci_orchestrator: self.ci_orchestrator.clone(),
            #[cfg(all(feature = "ci", feature = "nickel"))]
            ci_trigger_service: self.ci_trigger_service.clone(),
            #[cfg(all(feature = "forge", feature = "blob"))]
            forge_node: self.forge_node.clone(),
            #[cfg(feature = "blob")]
            blob_store: self.blob_store.clone(),
        })
    }

    #[cfg(feature = "jobs")]
    pub fn jobs_handler_context(&self) -> ContextCapabilityResult<JobsHandlerContext> {
        Ok(JobsHandlerContext {
            node_id: self.node_id,
            kv_store: Arc::clone(&self.kv_store),
            job_manager: Self::require_some(&self.job_manager, "job_manager")?,
            worker_service: self.worker_service.clone(),
            worker_coordinator: self.worker_coordinator.clone(),
        })
    }

    pub fn required_typed_service<T>(&self, capability: &'static str) -> ContextCapabilityResult<Arc<T>>
    where T: std::any::Any + Send + Sync + 'static {
        let value =
            self.secrets_service.as_ref().ok_or_else(|| MissingContextCapabilityError::new(capability))?.clone();
        value.downcast::<T>().map_err(|_| MissingContextCapabilityError::new(capability))
    }
}

// =============================================================================
// Test Support
// =============================================================================

#[cfg(feature = "testing")]
pub mod test_support {
    //! Test utilities for creating mock `ClientProtocolContext` instances.
    //!
    //! Provides a builder pattern for constructing test contexts with configurable
    //! mock dependencies. Uses `DeterministicKeyValueStore` and `DeterministicClusterController`
    //! from `aspen_core` for in-memory testing.
    //!
    //! # MockEndpointProvider
    //!
    //! The `MockEndpointProvider` creates a real Iroh endpoint for compatibility
    //! with handler code that needs to access endpoint information, without requiring
    //! actual network connections.
    //!
    //! ```ignore
    //! use aspen_rpc_core::test_support::MockEndpointProvider;
    //!
    //! let provider = MockEndpointProvider::with_seed(12345).await;
    //! let peer_id = provider.peer_id().await;
    //! ```

    use aspen_testing::DeterministicClusterController;
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    // =============================================================================
    // MockEndpointProvider
    // =============================================================================

    /// Mock implementation of `EndpointProvider` for testing.
    ///
    /// Provides deterministic responses without real Iroh networking.
    /// Creates a real Iroh endpoint for compatibility with handler code that
    /// needs to access the endpoint.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use aspen_rpc_core::test_support::MockEndpointProvider;
    ///
    /// #[tokio::test]
    /// async fn test_handler() {
    ///     let provider = MockEndpointProvider::new().await;
    ///     let peer_id = provider.peer_id().await;
    /// }
    /// ```
    pub struct MockEndpointProvider {
        /// Iroh endpoint for mock network operations.
        endpoint: iroh::Endpoint,
        /// Node address for peer discovery.
        node_addr: iroh::EndpointAddr,
        /// Public key bytes.
        public_key: Vec<u8>,
        /// Peer ID string.
        peer_id: String,
    }

    impl MockEndpointProvider {
        /// Create a new mock endpoint provider with a random secret key.
        ///
        /// This creates an isolated Iroh endpoint that won't connect to any real network.
        pub async fn new() -> Self {
            Self::with_seed(0).await
        }

        /// Create a mock endpoint provider with a deterministic seed.
        ///
        /// Using the same seed will produce the same node identity, useful for
        /// reproducible tests.
        pub async fn with_seed(seed: u64) -> Self {
            // Generate deterministic secret key from seed
            let mut key_bytes = [0u8; 32];
            key_bytes[0..8].copy_from_slice(&seed.to_le_bytes());
            let secret_key = iroh::SecretKey::from_bytes(&key_bytes);

            // Build endpoint without discovery (isolated)
            let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                .secret_key(secret_key.clone())
                .bind_addr("127.0.0.1:0".parse::<std::net::SocketAddrV4>().unwrap())
                .expect("failed to bind loopback address")
                .bind()
                .await
                .expect("failed to create mock endpoint");

            let node_addr = endpoint.addr();
            let public_key = secret_key.public().as_bytes().to_vec();
            let peer_id = node_addr.id.fmt_short().to_string();

            Self {
                endpoint,
                node_addr,
                public_key,
                peer_id,
            }
        }

        /// Create a mock endpoint provider for a specific node ID.
        ///
        /// The seed is derived from the node ID for deterministic identity.
        pub async fn for_node(node_id: u64) -> Self {
            Self::with_seed(node_id * 1000).await
        }
    }

    #[async_trait::async_trait]
    impl EndpointProvider for MockEndpointProvider {
        async fn public_key(&self) -> Vec<u8> {
            self.public_key.clone()
        }

        async fn peer_id(&self) -> String {
            self.peer_id.clone()
        }

        async fn addresses(&self) -> Vec<String> {
            vec!["127.0.0.1:0".to_string()]
        }

        fn node_addr(&self) -> &iroh::EndpointAddr {
            &self.node_addr
        }

        fn endpoint(&self) -> &iroh::Endpoint {
            &self.endpoint
        }
    }

    /// Builder for creating test `ClientProtocolContext` instances.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use aspen_rpc_core::test_support::TestContextBuilder;
    ///
    /// let ctx = TestContextBuilder::new()
    ///     .with_node_id(1)
    ///     .with_cookie("test-cookie")
    ///     .build();
    /// ```
    pub struct TestContextBuilder {
        node_id: u64,
        controller: Option<Arc<dyn ClusterController>>,
        kv_store: Option<Arc<dyn KeyValueStore>>,
        endpoint_manager: Option<Arc<dyn EndpointProvider>>,
        cluster_cookie: String,
        watch_registry: Option<Arc<dyn WatchRegistry>>,
        hooks_config: Option<aspen_hooks_types::HooksConfig>,
        docs_sync: Option<Arc<dyn DocsSyncProvider>>,
        peer_manager: Option<Arc<dyn PeerManager>>,
        #[cfg(feature = "blob")]
        blob_store: Option<Arc<aspen_blob::IrohBlobStore>>,
        #[cfg(feature = "blob")]
        blob_replication_manager: Option<aspen_blob::BlobReplicationManager>,
        #[cfg(feature = "sql")]
        sql_executor: Option<Arc<dyn aspen_sql::SqlQueryExecutor>>,
        #[cfg(feature = "forge")]
        forge_node: Option<Arc<aspen_forge::ForgeNode<aspen_blob::IrohBlobStore, dyn KeyValueStore>>>,
        #[cfg(feature = "jobs")]
        job_manager: Option<Arc<aspen_jobs::JobManager<dyn KeyValueStore>>>,
        #[cfg(feature = "worker")]
        worker_service: Option<Arc<aspen_cluster::worker_service::WorkerService>>,
        #[cfg(feature = "jobs")]
        worker_coordinator: Option<Arc<aspen_coordination::DistributedWorkerCoordinator<dyn KeyValueStore>>>,
        #[cfg(feature = "ci")]
        ci_orchestrator: Option<Arc<aspen_ci::PipelineOrchestrator<dyn aspen_core::KeyValueStore>>>,
        #[cfg(all(feature = "ci", feature = "nickel"))]
        ci_trigger_service: Option<Arc<aspen_ci::TriggerService>>,
        secrets_service: Option<Arc<dyn std::any::Any + Send + Sync>>,
    }

    impl Default for TestContextBuilder {
        fn default() -> Self {
            Self::new()
        }
    }

    impl TestContextBuilder {
        /// Create a new test context builder with default values.
        pub fn new() -> Self {
            Self {
                node_id: 1,
                controller: None,
                kv_store: None,
                endpoint_manager: None,
                cluster_cookie: "test-cookie".to_string(),
                watch_registry: None,
                hooks_config: None,
                docs_sync: None,
                peer_manager: None,
                #[cfg(feature = "blob")]
                blob_store: None,
                #[cfg(feature = "blob")]
                blob_replication_manager: None,
                #[cfg(feature = "sql")]
                sql_executor: None,
                #[cfg(feature = "forge")]
                forge_node: None,
                #[cfg(feature = "jobs")]
                job_manager: None,
                #[cfg(feature = "worker")]
                worker_service: None,
                #[cfg(feature = "jobs")]
                worker_coordinator: None,
                #[cfg(feature = "ci")]
                ci_orchestrator: None,
                #[cfg(all(feature = "ci", feature = "nickel"))]
                ci_trigger_service: None,
                secrets_service: None,
            }
        }

        /// Set the node ID.
        pub fn with_node_id(mut self, node_id: u64) -> Self {
            self.node_id = node_id;
            self
        }

        /// Set a custom cluster controller.
        pub fn with_controller(mut self, controller: Arc<dyn ClusterController>) -> Self {
            self.controller = Some(controller);
            self
        }

        /// Set a custom key-value store.
        pub fn with_kv_store(mut self, kv_store: Arc<dyn KeyValueStore>) -> Self {
            self.kv_store = Some(kv_store);
            self
        }

        /// Set a custom endpoint provider.
        pub fn with_endpoint_manager(mut self, endpoint_manager: Arc<dyn EndpointProvider>) -> Self {
            self.endpoint_manager = Some(endpoint_manager);
            self
        }

        /// Set the cluster cookie.
        pub fn with_cookie(mut self, cookie: impl Into<String>) -> Self {
            self.cluster_cookie = cookie.into();
            self
        }

        /// Set a custom watch registry.
        pub fn with_watch_registry(mut self, watch_registry: Arc<dyn WatchRegistry>) -> Self {
            self.watch_registry = Some(watch_registry);
            self
        }

        /// Set hooks configuration.
        pub fn with_hooks_config(mut self, hooks_config: aspen_hooks_types::HooksConfig) -> Self {
            self.hooks_config = Some(hooks_config);
            self
        }

        /// Set docs sync capabilities without unrelated app services.
        pub fn with_docs_sync(mut self, docs_sync: Arc<dyn DocsSyncProvider>) -> Self {
            self.docs_sync = Some(docs_sync);
            self
        }

        /// Set peer manager capabilities without unrelated app services.
        pub fn with_peer_manager(mut self, peer_manager: Arc<dyn PeerManager>) -> Self {
            self.peer_manager = Some(peer_manager);
            self
        }

        /// Set a custom blob store.
        #[cfg(feature = "blob")]
        pub fn with_blob_store(mut self, blob_store: Arc<aspen_blob::IrohBlobStore>) -> Self {
            self.blob_store = Some(blob_store);
            self
        }

        /// Set a blob replication manager.
        #[cfg(feature = "blob")]
        pub fn with_blob_replication_manager(
            mut self,
            blob_replication_manager: aspen_blob::BlobReplicationManager,
        ) -> Self {
            self.blob_replication_manager = Some(blob_replication_manager);
            self
        }

        /// Set a custom SQL executor.
        #[cfg(feature = "sql")]
        pub fn with_sql_executor(mut self, sql_executor: Arc<dyn aspen_sql::SqlQueryExecutor>) -> Self {
            self.sql_executor = Some(sql_executor);
            self
        }

        /// Set the forge node.
        #[cfg(feature = "forge")]
        pub fn with_forge_node(
            mut self,
            forge_node: Arc<aspen_forge::ForgeNode<aspen_blob::IrohBlobStore, dyn KeyValueStore>>,
        ) -> Self {
            self.forge_node = Some(forge_node);
            self
        }

        /// Set a custom job manager.
        #[cfg(feature = "jobs")]
        pub fn with_job_manager(mut self, job_manager: Arc<aspen_jobs::JobManager<dyn KeyValueStore>>) -> Self {
            self.job_manager = Some(job_manager);
            self
        }

        /// Set a custom worker service.
        #[cfg(feature = "worker")]
        pub fn with_worker_service(
            mut self,
            worker_service: Arc<aspen_cluster::worker_service::WorkerService>,
        ) -> Self {
            self.worker_service = Some(worker_service);
            self
        }

        /// Set a custom worker coordinator.
        #[cfg(feature = "jobs")]
        pub fn with_worker_coordinator(
            mut self,
            worker_coordinator: Arc<aspen_coordination::DistributedWorkerCoordinator<dyn KeyValueStore>>,
        ) -> Self {
            self.worker_coordinator = Some(worker_coordinator);
            self
        }

        /// Set a custom CI orchestrator.
        #[cfg(feature = "ci")]
        pub fn with_ci_orchestrator(
            mut self,
            ci_orchestrator: Arc<aspen_ci::PipelineOrchestrator<dyn aspen_core::KeyValueStore>>,
        ) -> Self {
            self.ci_orchestrator = Some(ci_orchestrator);
            self
        }

        /// Set a custom CI trigger service.
        #[cfg(all(feature = "ci", feature = "nickel"))]
        pub fn with_ci_trigger_service(mut self, ci_trigger_service: Arc<aspen_ci::TriggerService>) -> Self {
            self.ci_trigger_service = Some(ci_trigger_service);
            self
        }

        /// Set a typed secrets service payload.
        pub fn with_secrets_service(mut self, secrets_service: Arc<dyn std::any::Any + Send + Sync>) -> Self {
            self.secrets_service = Some(secrets_service);
            self
        }

        /// Build the test context.
        ///
        /// Uses deterministic in-memory implementations for any dependencies
        /// that were not explicitly configured.
        ///
        /// # Panics
        ///
        /// Panics if `endpoint_manager` is not set.
        pub fn build(self) -> ClientProtocolContext {
            let controller = self.controller.unwrap_or_else(|| DeterministicClusterController::new());
            let kv_store = self.kv_store.unwrap_or_else(|| DeterministicKeyValueStore::new());
            let endpoint_manager = self
                .endpoint_manager
                .expect("endpoint_manager is required. Use with_endpoint_manager() or build_with_mock_endpoint()");

            ClientProtocolContext {
                node_id: self.node_id,
                controller,
                kv_store,
                #[cfg(feature = "sql")]
                sql_executor: self.sql_executor.expect("sql_executor required when sql feature enabled"),
                state_machine: None,
                endpoint_manager,
                #[cfg(feature = "blob")]
                blob_store: self.blob_store,
                #[cfg(feature = "blob")]
                blob_replication_manager: self.blob_replication_manager,
                peer_manager: self.peer_manager,
                docs_sync: self.docs_sync,
                cluster_cookie: self.cluster_cookie,
                start_time: Instant::now(),
                network_factory: None,
                token_verifier: None,
                require_auth: false,
                topology: None,
                #[cfg(feature = "global-discovery")]
                content_discovery: None,
                #[cfg(feature = "forge")]
                forge_node: self.forge_node,
                #[cfg(feature = "jobs")]
                job_manager: self.job_manager,
                #[cfg(feature = "worker")]
                worker_service: self.worker_service,
                #[cfg(feature = "jobs")]
                worker_coordinator: self.worker_coordinator,
                watch_registry: self.watch_registry,
                #[cfg(feature = "hooks")]
                hook_service: None,
                hooks_config: self.hooks_config.unwrap_or_default(),
                secrets_service: self.secrets_service,
                #[cfg(feature = "forge")]
                federation_identity: None,
                #[cfg(feature = "forge")]
                federation_trust_manager: None,
                #[cfg(feature = "forge")]
                federation_cluster_identity: None,
                #[cfg(feature = "forge")]
                iroh_endpoint: None,
                #[cfg(all(feature = "forge", feature = "global-discovery"))]
                federation_discovery: None,
                #[cfg(feature = "ci")]
                ci_orchestrator: self.ci_orchestrator,
                #[cfg(all(feature = "ci", feature = "nickel"))]
                ci_trigger_service: self.ci_trigger_service,

                service_executors: Vec::new(),
                app_registry: aspen_core::shared_registry(),
                proxy_config: ProxyConfig::default(),
                prometheus_handle: None,
                network_metrics: None,
                span_forwarder: None,
                #[cfg(feature = "deploy")]
                drain_state: None,
            }
        }
    }

    /// Create a minimal test context with mock endpoint provider.
    pub fn minimal_test_context(mock_endpoint: Arc<dyn EndpointProvider>) -> ClientProtocolContext {
        TestContextBuilder::new().with_endpoint_manager(mock_endpoint).build()
    }

    /// Create a test context with a shared key-value store.
    pub fn test_context_with_kv(
        kv_store: Arc<dyn KeyValueStore>,
        mock_endpoint: Arc<dyn EndpointProvider>,
    ) -> ClientProtocolContext {
        TestContextBuilder::new().with_kv_store(kv_store).with_endpoint_manager(mock_endpoint).build()
    }
}
