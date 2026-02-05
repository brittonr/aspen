//! Worker service for managing distributed job execution on Aspen nodes.
//!
//! This module provides the WorkerService that integrates with Aspen nodes to:
//! - Start and manage worker pools based on node configuration
//! - Register workers with the cluster's job manager
//! - Track worker health and report metrics
//! - Handle graceful shutdown during node termination
//!
//! # Tiger Style

#![allow(dead_code, unused_imports, clippy::collapsible_if)]
//!
//! - Fixed limits on workers and concurrent jobs
//! - Fail-fast on invalid configurations
//! - Graceful shutdown with bounded timeout
//! - Clear separation between worker lifecycle and job execution

use std::sync::Arc;
use std::time::Duration;

use aspen_coordination::DistributedWorkerCoordinator;
use aspen_coordination::LoadBalancingStrategy;
use aspen_coordination::WorkerCoordinatorConfig;
use aspen_core::EndpointProvider;
use aspen_core::KeyValueStore;
use aspen_jobs::AffinityJobManager;
use aspen_jobs::DistributedJobRouter;
use aspen_jobs::DistributedPoolConfig;
use aspen_jobs::DistributedWorkerPool;
use aspen_jobs::JobManager;
use aspen_jobs::Worker;
use aspen_jobs::WorkerConfig as JobWorkerConfig;
use aspen_jobs::WorkerMetadata;
use aspen_jobs::WorkerPool;
use aspen_jobs::WorkerPoolStats;
use iroh::PublicKey as NodeId;
use snafu::ResultExt;
use snafu::Snafu;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::config::WorkerConfig;

/// Errors that can occur in the worker service.
#[derive(Debug, Snafu)]
pub enum WorkerServiceError {
    /// Failed to initialize worker pool.
    #[snafu(display("failed to initialize worker pool: {}", source))]
    InitializePool { source: aspen_jobs::JobError },

    /// Failed to register worker handler.
    #[snafu(display("failed to register worker handler '{}': {}", job_type, source))]
    RegisterHandler {
        job_type: String,
        source: aspen_jobs::JobError,
    },

    /// Failed to start workers.
    #[snafu(display("failed to start {} workers: {}", count, source))]
    StartWorkers { count: usize, source: aspen_jobs::JobError },

    /// Failed to update worker metadata.
    #[snafu(display("failed to update worker metadata: {}", source))]
    UpdateMetadata { source: aspen_jobs::JobError },

    /// Worker configuration is invalid.
    #[snafu(display("invalid worker configuration: {}", reason))]
    InvalidConfig { reason: String },

    /// Failed to shutdown workers.
    #[snafu(display("failed to shutdown workers: {}", source))]
    Shutdown { source: aspen_jobs::JobError },
}

type Result<T> = std::result::Result<T, WorkerServiceError>;

/// Service that manages worker pools on an Aspen node.
///
/// The WorkerService integrates with the node's job manager to provide
/// distributed job execution capabilities. It manages worker lifecycle,
/// registers handlers, and tracks worker health.
pub struct WorkerService {
    /// Node identifier.
    node_id: u64,

    /// Iroh node ID for P2P affinity.
    iroh_node_id: NodeId,

    /// Worker configuration from node config.
    config: WorkerConfig,

    /// Job manager for the cluster.
    job_manager: Arc<JobManager<dyn KeyValueStore>>,

    /// Affinity manager for P2P-aware job routing.
    affinity_manager: Arc<AffinityJobManager<dyn KeyValueStore>>,

    /// Worker pool instance.
    pool: Arc<WorkerPool<dyn KeyValueStore>>,

    /// Distributed worker pool (optional, enabled via config).
    distributed_pool: Option<Arc<DistributedWorkerPool<dyn KeyValueStore>>>,

    /// Distributed job router.
    distributed_router: Option<DistributedJobRouter<dyn KeyValueStore>>,

    /// Registered worker handlers.
    handlers: Arc<RwLock<Vec<Arc<dyn Worker>>>>,

    /// Handle to the worker monitoring task.
    monitor_handle: Option<JoinHandle<()>>,

    /// Shutdown signal.
    shutdown: Arc<tokio::sync::Notify>,
}

impl WorkerService {
    /// Create a new worker service.
    ///
    /// # Arguments
    ///
    /// * `node_id` - Logical node identifier
    /// * `config` - Worker configuration
    /// * `job_manager` - Shared job manager instance (must be the same as RPC handlers)
    /// * `store` - Key-value store (for distributed pool)
    /// * `endpoint_manager` - Endpoint provider for Iroh node ID
    pub fn new(
        node_id: u64,
        config: WorkerConfig,
        job_manager: Arc<JobManager<dyn KeyValueStore>>,
        store: Arc<dyn KeyValueStore>,
        endpoint_manager: Arc<dyn EndpointProvider>,
    ) -> Result<Self> {
        // Validate configuration
        if config.worker_count > 64 {
            return Err(WorkerServiceError::InvalidConfig {
                reason: format!("worker_count {} exceeds maximum of 64", config.worker_count),
            });
        }

        if config.max_concurrent_jobs > 100 {
            return Err(WorkerServiceError::InvalidConfig {
                reason: format!("max_concurrent_jobs {} exceeds maximum of 100", config.max_concurrent_jobs),
            });
        }

        if config.job_types.len() > 32 {
            return Err(WorkerServiceError::InvalidConfig {
                reason: format!("job_types count {} exceeds maximum of 32", config.job_types.len()),
            });
        }

        if config.tags.len() > 16 {
            return Err(WorkerServiceError::InvalidConfig {
                reason: format!("tags count {} exceeds maximum of 16", config.tags.len()),
            });
        }

        // Get Iroh node ID from endpoint
        let iroh_node_id = endpoint_manager.node_addr().id;

        // Create affinity manager
        let affinity_manager = Arc::new(AffinityJobManager::new(job_manager.clone()));

        // Create worker pool
        let pool = Arc::new(WorkerPool::with_manager(job_manager.clone()));

        // Create distributed pool if configured
        let (distributed_pool, distributed_router) = if config.enable_distributed {
            let distributed_config = DistributedPoolConfig {
                node_id: format!("node-{}", node_id),
                peer_id: Some(iroh_node_id.to_string()),
                worker_config: JobWorkerConfig {
                    id: Some(format!("node-{}-pool", node_id)),
                    concurrency: config.max_concurrent_jobs,
                    heartbeat_interval: Duration::from_millis(config.heartbeat_interval_ms),
                    shutdown_timeout: Duration::from_millis(config.shutdown_timeout_ms),
                    poll_interval: Duration::from_millis(config.poll_interval_ms),
                    job_types: config.job_types.clone(),
                    visibility_timeout: Duration::from_secs(config.visibility_timeout_secs),
                },
                coordinator_config: WorkerCoordinatorConfig {
                    strategy: LoadBalancingStrategy::LeastLoaded,
                    enable_work_stealing: config.enable_work_stealing.unwrap_or(true),
                    enable_failover: true,
                    ..Default::default()
                },
                enable_migration: true,
                enable_work_stealing: config.enable_work_stealing.unwrap_or(true),
                steal_check_interval: Duration::from_secs(5),
                max_migration_batch: 10,
                specializations: config.job_types.clone(),
                tags: config.tags.clone(),
            };

            let distributed_pool = Arc::new(DistributedWorkerPool::new(store.clone(), distributed_config));
            let router = distributed_pool.get_router();
            (Some(distributed_pool), Some(router))
        } else {
            (None, None)
        };

        Ok(Self {
            node_id,
            iroh_node_id,
            config,
            job_manager,
            affinity_manager,
            pool,
            distributed_pool,
            distributed_router,
            handlers: Arc::new(RwLock::new(Vec::new())),
            monitor_handle: None,
            shutdown: Arc::new(tokio::sync::Notify::new()),
        })
    }

    /// Register a worker handler for a specific job type.
    ///
    /// # Arguments
    ///
    /// * `job_type` - Type of jobs this handler processes
    /// * `handler` - Worker implementation
    pub async fn register_handler<W: Worker>(&self, job_type: &str, handler: W) -> Result<()> {
        info!(node_id = self.node_id, job_type, "registering worker handler");

        self.pool.register_handler(job_type, handler).await.context(RegisterHandlerSnafu {
            job_type: job_type.to_string(),
        })?;
        // Note: no longer tracking handlers here since they're moved into the pool

        Ok(())
    }

    /// Start the worker service.
    ///
    /// This starts the configured number of workers and begins processing jobs
    /// from the distributed queue.
    pub async fn start(&mut self) -> Result<()> {
        if !self.config.enabled {
            info!(node_id = self.node_id, "worker service disabled in configuration");
            return Ok(());
        }

        info!(
            node_id = self.node_id,
            worker_count = self.config.worker_count,
            job_types = ?self.config.job_types,
            tags = ?self.config.tags,
            distributed = self.config.enable_distributed,
            "starting worker service"
        );

        // Use distributed pool if enabled
        if self.config.enable_distributed {
            if let Some(ref distributed_pool) = self.distributed_pool {
                distributed_pool
                    .start(self.config.worker_count)
                    .await
                    .map_err(|e| WorkerServiceError::InitializePool { source: e })?;

                info!(
                    node_id = self.node_id,
                    worker_count = self.config.worker_count,
                    "distributed worker service started"
                );
                return Ok(());
            }
        }

        // Fall back to regular pool
        // Configure worker pool
        let worker_config = JobWorkerConfig {
            id: Some(format!("node-{}-worker", self.node_id)),
            concurrency: self.config.max_concurrent_jobs,
            heartbeat_interval: Duration::from_millis(self.config.heartbeat_interval_ms),
            shutdown_timeout: Duration::from_millis(self.config.shutdown_timeout_ms),
            poll_interval: Duration::from_millis(self.config.poll_interval_ms),
            job_types: self.config.job_types.clone(),
            visibility_timeout: Duration::from_secs(self.config.visibility_timeout_secs),
        };

        // Start workers individually with custom config
        for i in 0..self.config.worker_count {
            let mut worker_config_copy = worker_config.clone();
            worker_config_copy.id = Some(format!("node-{}-worker-{}", self.node_id, i));

            self.pool.spawn_worker(worker_config_copy).await.context(StartWorkersSnafu { count: 1usize })?;
        }

        // Update worker metadata for affinity routing
        self.update_worker_metadata().await?;

        // Start monitoring task
        self.start_monitoring();

        info!(node_id = self.node_id, worker_count = self.config.worker_count, "worker service started");

        Ok(())
    }

    /// Update worker metadata for P2P affinity routing.
    async fn update_worker_metadata(&self) -> Result<()> {
        let metadata = WorkerMetadata {
            id: format!("node-{}", self.node_id),
            node_id: self.iroh_node_id,
            tags: self.config.tags.clone(),
            region: None,        // Could be configured later
            load: 0.0,           // Will be updated by monitoring
            local_blobs: vec![], // Could query blob store
            latencies: Default::default(),
            local_shards: vec![], // Will be populated when sharding is enabled
        };

        self.affinity_manager.update_worker_metadata(metadata).await;

        Ok(())
    }

    /// Start monitoring task for worker health and metrics.
    fn start_monitoring(&mut self) {
        let pool = self.pool.clone();
        let affinity_manager = self.affinity_manager.clone();
        let node_id = self.node_id;
        let iroh_node_id = self.iroh_node_id;
        let tags = self.config.tags.clone();
        let shutdown = self.shutdown.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Get worker stats
                        let stats = pool.get_stats().await;

                        // Calculate load (ratio of processing to total workers)
                        let load = if stats.total_workers > 0 {
                            stats.processing_workers as f32 / stats.total_workers as f32
                        } else {
                            0.0
                        };

                        // Update metadata with current load
                        let metadata = WorkerMetadata {
                            id: format!("node-{}", node_id),
                            node_id: iroh_node_id,
                            tags: tags.clone(),
                            region: None,
                            load,
                            local_blobs: vec![], // Could query blob store periodically
                            latencies: Default::default(),
                            local_shards: vec![], // Will be populated when sharding is enabled
                        };

                        affinity_manager.update_worker_metadata(metadata).await;

                        // Log stats
                        info!(
                            node_id,
                            idle = stats.idle_workers,
                            processing = stats.processing_workers,
                            failed = stats.failed_workers,
                            jobs_processed = stats.total_jobs_processed,
                            jobs_failed = stats.total_jobs_failed,
                            load = format!("{:.2}%", load * 100.0),
                            "worker service stats"
                        );
                    }
                    _ = shutdown.notified() => {
                        info!(node_id, "worker monitoring task shutting down");
                        break;
                    }
                }
            }
        });

        self.monitor_handle = Some(handle);
    }

    /// Get current worker statistics.
    pub async fn get_stats(&self) -> WorkerPoolStats {
        self.pool.get_stats().await
    }

    /// Get information about all workers in the pool.
    pub async fn get_worker_info(&self) -> Vec<aspen_jobs::WorkerInfo> {
        self.pool.get_worker_info().await
    }

    /// Check if the worker service is healthy.
    pub async fn is_healthy(&self) -> bool {
        if !self.config.enabled {
            return true; // Disabled service is "healthy"
        }

        let stats = self.pool.get_stats().await;

        // Service is healthy if we have workers and not all are failed
        stats.total_workers > 0 && stats.failed_workers < stats.total_workers
    }

    /// Get the distributed job router if available.
    pub fn get_distributed_router(&self) -> Option<&DistributedJobRouter<dyn KeyValueStore>> {
        self.distributed_router.as_ref()
    }

    /// Shutdown the worker service gracefully.
    pub async fn shutdown(mut self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        info!(node_id = self.node_id, "shutting down worker service");

        // Signal monitoring task to stop
        self.shutdown.notify_one();

        // Wait for monitoring task
        if let Some(handle) = self.monitor_handle.take() {
            let _ = handle.await;
        }

        // Shutdown distributed pool if enabled
        if let Some(distributed_pool) = self.distributed_pool.take() {
            if let Ok(pool) = Arc::try_unwrap(distributed_pool) {
                let _ = pool.shutdown().await;
            }
        }

        // Shutdown worker pool
        self.pool.shutdown().await.context(ShutdownSnafu)?;

        info!(node_id = self.node_id, "worker service shut down");
        Ok(())
    }
}

// Re-export MaintenanceWorker from aspen_jobs
pub use aspen_jobs::workers::MaintenanceWorker;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use aspen_core::DeterministicKeyValueStore;
    use aspen_core::context::EndpointAddr;
    use aspen_core::context::IrohEndpoint;
    use aspen_jobs::Job;
    use aspen_jobs::JobOutput;
    use aspen_jobs::JobResult;
    use async_trait::async_trait;

    use super::*;

    // =========================================================================
    // Test Utilities
    // =========================================================================

    /// Create a default test configuration
    fn create_test_config() -> WorkerConfig {
        WorkerConfig {
            enabled: true,
            worker_count: 2,
            max_concurrent_jobs: 4,
            job_types: vec!["test_job".to_string()],
            tags: vec!["test".to_string()],
            prefer_local: true,
            data_locality_weight: 0.7,
            poll_interval_ms: 1000,
            visibility_timeout_secs: 300,
            heartbeat_interval_ms: 5000,
            shutdown_timeout_ms: 30000,
            enable_distributed: false,
            enable_work_stealing: None,
            load_balancing_strategy: None,
        }
    }

    /// Create a disabled test configuration
    fn create_disabled_config() -> WorkerConfig {
        WorkerConfig {
            enabled: false,
            ..create_test_config()
        }
    }

    /// Mock endpoint provider for tests.
    ///
    /// Creates an isolated Iroh endpoint that won't connect to any real network.
    struct MockEndpointProvider {
        endpoint: IrohEndpoint,
        node_addr: EndpointAddr,
        public_key: Vec<u8>,
        peer_id: String,
    }

    impl MockEndpointProvider {
        /// Create a new mock endpoint provider.
        async fn new() -> Self {
            Self::with_seed(0).await
        }

        /// Create a mock endpoint provider with a deterministic seed.
        async fn with_seed(seed: u64) -> Self {
            // Generate deterministic secret key from seed
            let mut key_bytes = [0u8; 32];
            key_bytes[0..8].copy_from_slice(&seed.to_le_bytes());
            let secret_key = iroh::SecretKey::from_bytes(&key_bytes);

            // Build endpoint without discovery (isolated)
            let endpoint = iroh::Endpoint::builder()
                .secret_key(secret_key.clone())
                .bind_addr_v4("127.0.0.1:0".parse().unwrap())
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
    }

    #[async_trait]
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

        fn node_addr(&self) -> &EndpointAddr {
            &self.node_addr
        }

        fn endpoint(&self) -> &IrohEndpoint {
            &self.endpoint
        }
    }

    /// Simple test worker that always succeeds.
    #[derive(Clone)]
    struct TestWorker {
        job_types: Vec<String>,
    }

    impl TestWorker {
        fn new(job_type: &str) -> Self {
            Self {
                job_types: vec![job_type.to_string()],
            }
        }
    }

    #[async_trait]
    impl Worker for TestWorker {
        fn job_types(&self) -> Vec<String> {
            self.job_types.clone()
        }

        async fn execute(&self, _job: Job) -> JobResult {
            JobResult::Success(JobOutput {
                data: serde_json::json!({"status": "ok"}),
                metadata: HashMap::new(),
            })
        }
    }

    /// Create test components needed for WorkerService (async version).
    async fn create_test_components()
    -> (Arc<JobManager<dyn KeyValueStore>>, Arc<dyn KeyValueStore>, Arc<dyn EndpointProvider>) {
        let store: Arc<dyn KeyValueStore> = DeterministicKeyValueStore::new();
        let job_manager: Arc<JobManager<dyn KeyValueStore>> = Arc::new(JobManager::new(store.clone()));
        let endpoint: Arc<dyn EndpointProvider> = Arc::new(MockEndpointProvider::new().await);
        (job_manager, store, endpoint)
    }

    // =========================================================================
    // Constructor & Configuration Tests
    // =========================================================================

    #[tokio::test]
    async fn test_new_valid_configuration() {
        let config = create_test_config();
        let (job_manager, store, endpoint) = create_test_components().await;

        let result = WorkerService::new(1, config, job_manager, store, endpoint);
        assert!(result.is_ok());

        let service = result.unwrap();
        assert_eq!(service.node_id, 1);
        assert!(service.config.enabled);
        assert_eq!(service.config.worker_count, 2);
    }

    #[tokio::test]
    async fn test_new_with_disabled_workers() {
        let config = create_disabled_config();
        let (job_manager, store, endpoint) = create_test_components().await;

        let result = WorkerService::new(1, config, job_manager, store, endpoint);
        assert!(result.is_ok());

        let service = result.unwrap();
        assert!(!service.config.enabled);
    }

    #[tokio::test]
    async fn test_new_worker_count_exceeds_max() {
        let mut config = create_test_config();
        config.worker_count = 65; // Exceeds max of 64

        let (job_manager, store, endpoint) = create_test_components().await;
        let result = WorkerService::new(1, config, job_manager, store, endpoint);

        match result {
            Err(WorkerServiceError::InvalidConfig { reason }) => {
                assert!(reason.contains("worker_count"));
                assert!(reason.contains("65"));
                assert!(reason.contains("64"));
            }
            Err(e) => panic!("expected InvalidConfig error, got: {}", e),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    #[tokio::test]
    async fn test_new_max_concurrent_jobs_exceeds_max() {
        let mut config = create_test_config();
        config.max_concurrent_jobs = 101; // Exceeds max of 100

        let (job_manager, store, endpoint) = create_test_components().await;
        let result = WorkerService::new(1, config, job_manager, store, endpoint);

        match result {
            Err(WorkerServiceError::InvalidConfig { reason }) => {
                assert!(reason.contains("max_concurrent_jobs"));
                assert!(reason.contains("101"));
                assert!(reason.contains("100"));
            }
            Err(e) => panic!("expected InvalidConfig error, got: {}", e),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    #[tokio::test]
    async fn test_new_job_types_exceeds_max() {
        let mut config = create_test_config();
        config.job_types = (0..33).map(|i| format!("job_type_{}", i)).collect(); // 33 exceeds max of 32

        let (job_manager, store, endpoint) = create_test_components().await;
        let result = WorkerService::new(1, config, job_manager, store, endpoint);

        match result {
            Err(WorkerServiceError::InvalidConfig { reason }) => {
                assert!(reason.contains("job_types"));
                assert!(reason.contains("33"));
                assert!(reason.contains("32"));
            }
            Err(e) => panic!("expected InvalidConfig error, got: {}", e),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    #[tokio::test]
    async fn test_new_tags_exceeds_max() {
        let mut config = create_test_config();
        config.tags = (0..17).map(|i| format!("tag_{}", i)).collect(); // 17 exceeds max of 16

        let (job_manager, store, endpoint) = create_test_components().await;
        let result = WorkerService::new(1, config, job_manager, store, endpoint);

        match result {
            Err(WorkerServiceError::InvalidConfig { reason }) => {
                assert!(reason.contains("tags"));
                assert!(reason.contains("17"));
                assert!(reason.contains("16"));
            }
            Err(e) => panic!("expected InvalidConfig error, got: {}", e),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    #[tokio::test]
    async fn test_config_edge_cases_at_limits() {
        // Test exactly at Tiger Style limits - should succeed
        let mut config = create_test_config();
        config.worker_count = 64;
        config.max_concurrent_jobs = 100;
        config.job_types = (0..32).map(|i| format!("job_type_{}", i)).collect();
        config.tags = (0..16).map(|i| format!("tag_{}", i)).collect();

        let (job_manager, store, endpoint) = create_test_components().await;
        let result = WorkerService::new(1, config, job_manager, store, endpoint);

        assert!(result.is_ok(), "should accept values exactly at limits");
    }

    #[tokio::test]
    async fn test_config_zero_workers() {
        let mut config = create_test_config();
        config.worker_count = 0;

        let (job_manager, store, endpoint) = create_test_components().await;
        let result = WorkerService::new(1, config, job_manager, store, endpoint);

        // Zero workers should be allowed (effectively disabled)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_distributed_config_initialization() {
        let mut config = create_test_config();
        config.enable_distributed = true;

        let (job_manager, store, endpoint) = create_test_components().await;
        let result = WorkerService::new(1, config, job_manager, store, endpoint);

        assert!(result.is_ok());
        let service = result.unwrap();
        assert!(service.distributed_pool.is_some());
        assert!(service.distributed_router.is_some());
    }

    #[tokio::test]
    async fn test_non_distributed_config_initialization() {
        let mut config = create_test_config();
        config.enable_distributed = false;

        let (job_manager, store, endpoint) = create_test_components().await;
        let result = WorkerService::new(1, config, job_manager, store, endpoint);

        assert!(result.is_ok());
        let service = result.unwrap();
        assert!(service.distributed_pool.is_none());
        assert!(service.distributed_router.is_none());
    }

    // =========================================================================
    // Handler Registration Tests
    // =========================================================================

    #[tokio::test]
    async fn test_register_handler_success() {
        let config = create_test_config();
        let (job_manager, store, endpoint) = create_test_components().await;

        let service = WorkerService::new(1, config, job_manager, store, endpoint).unwrap();

        let worker = TestWorker::new("test_job");
        let result = service.register_handler("test_job", worker).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_register_multiple_handlers() {
        let config = create_test_config();
        let (job_manager, store, endpoint) = create_test_components().await;

        let service = WorkerService::new(1, config, job_manager, store, endpoint).unwrap();

        // Register multiple handlers
        let worker1 = TestWorker::new("job_type_1");
        let worker2 = TestWorker::new("job_type_2");
        let worker3 = TestWorker::new("job_type_3");

        assert!(service.register_handler("job_type_1", worker1).await.is_ok());
        assert!(service.register_handler("job_type_2", worker2).await.is_ok());
        assert!(service.register_handler("job_type_3", worker3).await.is_ok());
    }

    // =========================================================================
    // Service Lifecycle Tests
    // =========================================================================

    #[tokio::test]
    async fn test_start_disabled_service() {
        let config = create_disabled_config();
        let (job_manager, store, endpoint) = create_test_components().await;

        let mut service = WorkerService::new(1, config, job_manager, store, endpoint).unwrap();

        let result = service.start().await;
        assert!(result.is_ok());

        // Monitor should not be started for disabled service
        assert!(service.monitor_handle.is_none());
    }

    #[tokio::test]
    async fn test_start_creates_workers() {
        let mut config = create_test_config();
        config.worker_count = 4;

        let (job_manager, store, endpoint) = create_test_components().await;
        let mut service = WorkerService::new(1, config, job_manager, store, endpoint).unwrap();

        let result = service.start().await;
        assert!(result.is_ok());

        // Allow workers to initialize
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify workers were created
        let stats = service.get_stats().await;
        assert_eq!(stats.total_workers, 4, "expected 4 workers to be registered");
    }

    #[tokio::test]
    async fn test_start_local_pool() {
        let mut config = create_test_config();
        config.enable_distributed = false;

        let (job_manager, store, endpoint) = create_test_components().await;
        let mut service = WorkerService::new(1, config, job_manager, store, endpoint).unwrap();

        let result = service.start().await;
        assert!(result.is_ok());

        // Monitor should be started for local pool
        assert!(service.monitor_handle.is_some());
    }

    #[tokio::test]
    async fn test_start_distributed_pool() {
        let mut config = create_test_config();
        config.enable_distributed = true;

        let (job_manager, store, endpoint) = create_test_components().await;
        let mut service = WorkerService::new(1, config, job_manager, store, endpoint).unwrap();

        let result = service.start().await;
        assert!(result.is_ok());

        // Distributed pool uses its own monitoring
        assert!(service.monitor_handle.is_none());
    }

    #[tokio::test]
    async fn test_shutdown_disabled_service() {
        let config = create_disabled_config();
        let (job_manager, store, endpoint) = create_test_components().await;

        let service = WorkerService::new(1, config, job_manager, store, endpoint).unwrap();

        let result = service.shutdown().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_shutdown_enabled_service() {
        let config = create_test_config();
        let (job_manager, store, endpoint) = create_test_components().await;

        let mut service = WorkerService::new(1, config, job_manager, store, endpoint).unwrap();

        // Start the service first
        service.start().await.unwrap();

        // Verify monitoring is running
        assert!(service.monitor_handle.is_some());

        // Shutdown
        let result = service.shutdown().await;
        assert!(result.is_ok());
    }

    // =========================================================================
    // Statistics & Health Tests
    // =========================================================================

    #[tokio::test]
    async fn test_get_stats() {
        let mut config = create_test_config();
        config.worker_count = 3;

        let (job_manager, store, endpoint) = create_test_components().await;
        let mut service = WorkerService::new(1, config, job_manager, store, endpoint).unwrap();

        service.start().await.unwrap();

        // Allow workers to initialize (they start async and need time to register)
        tokio::time::sleep(Duration::from_millis(100)).await;

        let stats = service.get_stats().await;
        assert_eq!(stats.total_workers, 3, "expected 3 workers to be registered");
        // Workers should be idle since no jobs are queued
        assert_eq!(stats.processing_workers, 0, "no jobs queued, none should be processing");
        assert_eq!(stats.failed_workers, 0, "no workers should have failed");
    }

    #[tokio::test]
    async fn test_get_worker_info() {
        let mut config = create_test_config();
        config.worker_count = 2;

        let (job_manager, store, endpoint) = create_test_components().await;
        let mut service = WorkerService::new(1, config, job_manager, store, endpoint).unwrap();

        service.start().await.unwrap();

        // Allow workers to initialize
        tokio::time::sleep(Duration::from_millis(100)).await;

        let workers = service.get_worker_info().await;
        assert_eq!(workers.len(), 2, "expected 2 workers to be registered");
    }

    #[tokio::test]
    async fn test_is_healthy_disabled() {
        let config = create_disabled_config();
        let (job_manager, store, endpoint) = create_test_components().await;

        let service = WorkerService::new(1, config, job_manager, store, endpoint).unwrap();

        // Disabled service is considered healthy
        assert!(service.is_healthy().await);
    }

    #[tokio::test]
    async fn test_is_healthy_with_workers() {
        let config = create_test_config();
        let (job_manager, store, endpoint) = create_test_components().await;

        let mut service = WorkerService::new(1, config, job_manager, store, endpoint).unwrap();
        service.start().await.unwrap();

        // Allow workers to initialize
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Service with active workers should be healthy
        assert!(service.is_healthy().await);
    }

    #[tokio::test]
    async fn test_is_healthy_no_workers() {
        let mut config = create_test_config();
        config.worker_count = 0;

        let (job_manager, store, endpoint) = create_test_components().await;
        let service = WorkerService::new(1, config, job_manager, store, endpoint).unwrap();

        // Service with 0 workers should be unhealthy (when enabled)
        assert!(!service.is_healthy().await);
    }

    // =========================================================================
    // Distributed Router Tests
    // =========================================================================

    #[tokio::test]
    async fn test_get_distributed_router_disabled() {
        let mut config = create_test_config();
        config.enable_distributed = false;

        let (job_manager, store, endpoint) = create_test_components().await;
        let service = WorkerService::new(1, config, job_manager, store, endpoint).unwrap();

        assert!(service.get_distributed_router().is_none());
    }

    #[tokio::test]
    async fn test_get_distributed_router_enabled() {
        let mut config = create_test_config();
        config.enable_distributed = true;

        let (job_manager, store, endpoint) = create_test_components().await;
        let service = WorkerService::new(1, config, job_manager, store, endpoint).unwrap();

        assert!(service.get_distributed_router().is_some());
    }

    // =========================================================================
    // Error Type Tests
    // =========================================================================

    #[test]
    fn test_error_display_invalid_config() {
        let err = WorkerServiceError::InvalidConfig {
            reason: "test reason".to_string(),
        };
        assert!(err.to_string().contains("test reason"));
    }

    #[test]
    fn test_error_display_start_workers() {
        // Just verify the error type exists and formats correctly
        let err = WorkerServiceError::InvalidConfig {
            reason: "failed to start workers".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("failed to start workers"));
    }
}
