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

mod handlers;
mod initialization;
mod monitoring;
mod shutdown;
mod types;

// Re-export MaintenanceWorker from aspen-jobs-worker-maintenance (requires jobs + blob features)
#[cfg(all(feature = "jobs", feature = "blob"))]
pub use aspen_jobs_worker_maintenance::MaintenanceWorker;
pub use types::WorkerService;
pub use types::WorkerServiceError;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    use aspen_core::EndpointProvider;
    use aspen_core::KeyValueStore;
    use aspen_core::context::EndpointAddr;
    use aspen_core::context::IrohEndpoint;
    use aspen_jobs::AffinityJobManager;
    use aspen_jobs::Job;
    use aspen_jobs::JobManager;
    use aspen_jobs::JobOutput;
    use aspen_jobs::JobResult;
    use aspen_jobs::Worker;
    use aspen_testing::DeterministicKeyValueStore;
    use async_trait::async_trait;

    use super::*;

    // =========================================================================
    // Test Utilities
    // =========================================================================

    /// Create a default test configuration
    fn create_test_config() -> crate::config::WorkerConfig {
        crate::config::WorkerConfig {
            is_enabled: true,
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
    fn create_disabled_config() -> crate::config::WorkerConfig {
        crate::config::WorkerConfig {
            is_enabled: false,
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
        assert!(service.config.is_enabled);
        assert_eq!(service.config.worker_count, 2);
    }

    #[tokio::test]
    async fn test_new_with_disabled_workers() {
        let config = create_disabled_config();
        let (job_manager, store, endpoint) = create_test_components().await;

        let result = WorkerService::new(1, config, job_manager, store, endpoint);
        assert!(result.is_ok());

        let service = result.unwrap();
        assert!(!service.config.is_enabled);
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
