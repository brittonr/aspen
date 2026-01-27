//! Client protocol context.
//!
//! Contains the shared context and dependencies needed by RPC handlers.

use std::sync::Arc;
use std::time::Instant;

use aspen_auth::TokenVerifier;
use aspen_core::ClusterController;
#[cfg(feature = "global-discovery")]
use aspen_core::ContentDiscovery;
use aspen_core::DocsSyncProvider;
use aspen_core::EndpointProvider;
use aspen_core::KeyValueStore;
use aspen_core::NetworkFactory;
use aspen_core::PeerManager;
use aspen_core::WatchRegistry;
use aspen_raft::StateMachineVariant;
use aspen_sharding::ShardTopology;

/// Context for Client protocol handler with all dependencies.
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
    ///
    /// When present, enables:
    /// - Manual replication triggering via TriggerBlobReplication RPC
    /// - Replication status queries via GetBlobReplicationStatus RPC
    /// - Automatic topology updates for replica placement
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
    ///
    /// When present, enables AddPeer RPC to register peers in the network factory.
    pub network_factory: Option<Arc<dyn NetworkFactory>>,
    /// Token verifier for capability-based authorization.
    ///
    /// Optional during migration period. When `None`, all requests are allowed.
    /// When `Some`, requests that require auth must provide valid tokens.
    pub token_verifier: Option<Arc<TokenVerifier>>,
    /// Whether to require authentication for all authorized requests.
    ///
    /// When `false` (default), missing tokens are allowed during migration.
    /// When `true`, requests without valid tokens are rejected.
    pub require_auth: bool,
    /// Shard topology for GetTopology RPC (optional).
    ///
    /// When present, enables topology queries for shard-aware clients.
    pub topology: Option<Arc<tokio::sync::RwLock<ShardTopology>>>,
    /// Content discovery service for DHT announcements and provider lookup (optional).
    ///
    /// When present, enables:
    /// - Automatic DHT announcements when blobs are added
    /// - DHT provider discovery for hash-only downloads
    /// - Provider aggregation combining ticket + DHT providers
    #[cfg(feature = "global-discovery")]
    pub content_discovery: Option<Arc<dyn ContentDiscovery>>,
    /// Forge node for decentralized Git operations (optional).
    ///
    /// When present, enables Forge RPC operations for:
    /// - Repository management (create, get, list)
    /// - Git object storage (blobs, trees, commits)
    /// - Ref management (branches, tags)
    /// - Collaborative objects (issues, patches)
    #[cfg(feature = "forge")]
    pub forge_node: Option<Arc<aspen_forge::ForgeNode<aspen_blob::IrohBlobStore, dyn aspen_core::KeyValueStore>>>,
    /// Pijul store for patch-based version control (optional).
    ///
    /// When present, enables Pijul RPC operations for:
    /// - Repository management (init, list, info)
    /// - Channel management (list, create, delete, fork)
    /// - Change operations (record, apply, log, checkout)
    #[cfg(feature = "pijul")]
    pub pijul_store: Option<Arc<aspen_pijul::PijulStore<aspen_blob::IrohBlobStore, dyn aspen_core::KeyValueStore>>>,
    /// Job manager for distributed job queue operations (optional).
    ///
    /// When present, enables Job RPC operations for:
    /// - Job submission and management
    /// - Queue statistics and monitoring
    /// - Worker registration and heartbeats
    pub job_manager: Option<Arc<aspen_jobs::JobManager<dyn KeyValueStore>>>,
    /// Worker service for querying worker status (optional).
    ///
    /// When present, enables worker status queries via the WorkerStatus RPC.
    /// Provides access to worker pool statistics and individual worker info.
    pub worker_service: Option<Arc<aspen_cluster::worker_service::WorkerService>>,
    /// Distributed worker coordinator for external worker registration (optional).
    ///
    /// When present, enables external workers to register, send heartbeats, and
    /// deregister via RPC. Provides cluster-wide worker coordination including:
    /// - Worker registration with capabilities and capacity
    /// - Heartbeat-based health monitoring
    /// - Load-based job routing
    /// - Automatic failover on worker timeout
    pub worker_coordinator: Option<Arc<aspen_coordination::DistributedWorkerCoordinator<dyn KeyValueStore>>>,
    /// Watch registry for tracking active watch subscriptions (optional).
    ///
    /// When present, enables the WatchStatus RPC to return information about
    /// active watches created via the streaming protocol (LOG_SUBSCRIBER_ALPN).
    /// This provides observability into watch subscriptions without requiring
    /// clients to use the streaming protocol.
    pub watch_registry: Option<Arc<dyn WatchRegistry>>,
    /// Hook service for event-driven automation (optional).
    ///
    /// When present, enables Hook RPC operations for:
    /// - Listing configured handlers
    /// - Querying execution metrics
    /// - Manual event triggering for testing
    pub hook_service: Option<Arc<aspen_hooks::HookService>>,
    /// Hooks configuration for handler metadata.
    ///
    /// Contains the handler configurations needed for HookList RPC.
    /// This is separate from hook_service to allow listing even when
    /// the service is not initialized.
    pub hooks_config: aspen_hooks::HooksConfig,
    /// Secrets service for Vault-compatible secrets management (optional).
    ///
    /// When present, enables Secrets RPC operations for:
    /// - KV v2: Versioned key-value secrets
    /// - Transit: Encryption-as-a-service
    /// - PKI: Certificate authority
    #[cfg(feature = "secrets")]
    pub secrets_service: Option<std::sync::Arc<crate::handlers::secrets::SecretsService>>,
    /// Federation cluster identity (optional).
    ///
    /// When present, enables federation status queries to return
    /// cluster name and public key information.
    #[cfg(feature = "forge")]
    pub federation_identity: Option<Arc<aspen_cluster::federation::SignedClusterIdentity>>,
    /// Federation trust manager (optional).
    ///
    /// When present, enables trust/untrust cluster operations via RPC.
    #[cfg(feature = "forge")]
    pub federation_trust_manager: Option<Arc<aspen_cluster::federation::TrustManager>>,
    /// Federation discovery service (optional).
    ///
    /// When present, enables federation discovery operations via RPC:
    /// - Get discovered cluster count for federation status
    /// - List discovered clusters
    /// - Get individual cluster details
    ///
    /// Requires both `forge` and `global-discovery` features.
    #[cfg(all(feature = "forge", feature = "global-discovery"))]
    pub federation_discovery: Option<Arc<aspen_cluster::federation::FederationDiscoveryService>>,
    /// CI pipeline orchestrator for triggering and monitoring pipelines (optional).
    ///
    /// When present, enables CI RPC operations for:
    /// - Triggering pipeline runs
    /// - Getting pipeline status
    /// - Listing pipeline runs
    /// - Cancelling pipeline runs
    #[cfg(feature = "ci")]
    pub ci_orchestrator: Option<Arc<aspen_ci::PipelineOrchestrator<dyn aspen_core::KeyValueStore>>>,
    /// CI trigger service for watching repositories (optional).
    ///
    /// When present, enables automatic CI triggering on ref updates:
    /// - Watch repository for CI triggers
    /// - Unwatch repository
    #[cfg(feature = "ci")]
    pub ci_trigger_service: Option<Arc<aspen_ci::TriggerService>>,
    /// Nix cache signer for narinfo signing (optional).
    ///
    /// When present, enables automatic narinfo signing for the Nix binary cache gateway.
    /// Supports both local Ed25519 keys and Transit secrets engine backends.
    #[cfg(feature = "nix-cache-gateway")]
    pub nix_cache_signer: Option<Arc<dyn aspen_nix_cache_gateway::NarinfoSigningProvider>>,
}

impl std::fmt::Debug for ClientProtocolContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientProtocolContext")
            .field("node_id", &self.node_id)
            .field("cluster_cookie", &self.cluster_cookie)
            .finish_non_exhaustive()
    }
}

// =============================================================================
// Test Support
// =============================================================================

#[cfg(any(test, feature = "testing"))]
pub mod test_support {
    //! Test utilities for creating mock `ClientProtocolContext` instances.
    //!
    //! Provides a builder pattern for constructing test contexts with configurable
    //! mock dependencies. Uses `DeterministicKeyValueStore` and `DeterministicClusterController`
    //! from `aspen_core` for in-memory testing.

    use aspen_core::DeterministicClusterController;
    use aspen_core::DeterministicKeyValueStore;

    use super::*;

    /// Builder for creating test `ClientProtocolContext` instances.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use aspen_rpc_handlers::context::test_support::TestContextBuilder;
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
        hooks_config: Option<aspen_hooks::HooksConfig>,
        #[cfg(feature = "sql")]
        sql_executor: Option<Arc<dyn aspen_sql::SqlQueryExecutor>>,
        #[cfg(feature = "pijul")]
        pijul_store: Option<Arc<aspen_pijul::PijulStore<aspen_blob::IrohBlobStore, dyn KeyValueStore>>>,
    }

    impl Default for TestContextBuilder {
        fn default() -> Self {
            Self::new()
        }
    }

    impl TestContextBuilder {
        /// Create a new test context builder with default values.
        ///
        /// Defaults:
        /// - node_id: 1
        /// - cluster_cookie: "test-cookie"
        /// - controller: DeterministicClusterController
        /// - kv_store: DeterministicKeyValueStore
        pub fn new() -> Self {
            Self {
                node_id: 1,
                controller: None,
                kv_store: None,
                endpoint_manager: None,
                cluster_cookie: "test-cookie".to_string(),
                watch_registry: None,
                hooks_config: None,
                #[cfg(feature = "sql")]
                sql_executor: None,
                #[cfg(feature = "pijul")]
                pijul_store: None,
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
        pub fn with_hooks_config(mut self, hooks_config: aspen_hooks::HooksConfig) -> Self {
            self.hooks_config = Some(hooks_config);
            self
        }

        /// Set a custom SQL executor.
        #[cfg(feature = "sql")]
        pub fn with_sql_executor(mut self, sql_executor: Arc<dyn aspen_sql::SqlQueryExecutor>) -> Self {
            self.sql_executor = Some(sql_executor);
            self
        }

        /// Set a custom Pijul store.
        #[cfg(feature = "pijul")]
        pub fn with_pijul_store(
            mut self,
            pijul_store: Arc<aspen_pijul::PijulStore<aspen_blob::IrohBlobStore, dyn KeyValueStore>>,
        ) -> Self {
            self.pijul_store = Some(pijul_store);
            self
        }

        /// Build the test context.
        ///
        /// Uses deterministic in-memory implementations for any dependencies
        /// that were not explicitly configured.
        ///
        /// # Panics
        ///
        /// Panics if `endpoint_manager` is not set. Use `with_endpoint_manager()`
        /// or `build_with_mock_endpoint()` to provide an endpoint.
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
                blob_store: None,
                #[cfg(feature = "blob")]
                blob_replication_manager: None,
                peer_manager: None,
                docs_sync: None,
                cluster_cookie: self.cluster_cookie,
                start_time: Instant::now(),
                network_factory: None,
                token_verifier: None,
                require_auth: false,
                topology: None,
                #[cfg(feature = "global-discovery")]
                content_discovery: None,
                #[cfg(feature = "forge")]
                forge_node: None,
                #[cfg(feature = "pijul")]
                pijul_store: self.pijul_store,
                job_manager: None,
                worker_service: None,
                worker_coordinator: None,
                watch_registry: self.watch_registry,
                hook_service: None,
                hooks_config: self.hooks_config.unwrap_or_default(),
                #[cfg(feature = "secrets")]
                secrets_service: None,
                #[cfg(feature = "forge")]
                federation_identity: None,
                #[cfg(feature = "forge")]
                federation_trust_manager: None,
                #[cfg(all(feature = "forge", feature = "global-discovery"))]
                federation_discovery: None,
                #[cfg(feature = "ci")]
                ci_orchestrator: None,
                #[cfg(feature = "ci")]
                ci_trigger_service: None,
                #[cfg(feature = "nix-cache-gateway")]
                nix_cache_signer: None,
            }
        }
    }

    /// Create a minimal test context with mock endpoint provider.
    ///
    /// This is a convenience function for tests that need a quick context
    /// without configuring individual components.
    ///
    /// # Arguments
    ///
    /// * `mock_endpoint` - A mock endpoint provider (use `MockEndpointProvider` from test_mocks)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use aspen_rpc_handlers::context::test_support::minimal_test_context;
    /// use aspen_rpc_handlers::test_mocks::MockEndpointProvider;
    ///
    /// let ctx = minimal_test_context(Arc::new(MockEndpointProvider::new()));
    /// ```
    pub fn minimal_test_context(mock_endpoint: Arc<dyn EndpointProvider>) -> ClientProtocolContext {
        TestContextBuilder::new().with_endpoint_manager(mock_endpoint).build()
    }

    /// Create a test context with a shared key-value store.
    ///
    /// Useful when tests need to share state between handler calls.
    ///
    /// # Arguments
    ///
    /// * `kv_store` - Shared key-value store instance
    /// * `mock_endpoint` - A mock endpoint provider
    pub fn test_context_with_kv(
        kv_store: Arc<dyn KeyValueStore>,
        mock_endpoint: Arc<dyn EndpointProvider>,
    ) -> ClientProtocolContext {
        TestContextBuilder::new().with_kv_store(kv_store).with_endpoint_manager(mock_endpoint).build()
    }
}
