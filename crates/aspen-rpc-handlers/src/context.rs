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
}

impl std::fmt::Debug for ClientProtocolContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientProtocolContext")
            .field("node_id", &self.node_id)
            .field("cluster_cookie", &self.cluster_cookie)
            .finish_non_exhaustive()
    }
}
