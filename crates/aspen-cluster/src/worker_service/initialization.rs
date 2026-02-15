//! WorkerService constructor and configuration validation.

use std::sync::Arc;
use std::time::Duration;

use aspen_coordination::LoadBalancingStrategy;
use aspen_coordination::WorkerCoordinatorConfig;
use aspen_core::EndpointProvider;
use aspen_core::KeyValueStore;
use aspen_jobs::AffinityJobManager;
use aspen_jobs::DistributedPoolConfig;
use aspen_jobs::DistributedWorkerPool;
use aspen_jobs::JobManager;
use aspen_jobs::WorkerConfig as JobWorkerConfig;
use aspen_jobs::WorkerPool;
use tokio::sync::RwLock;
use tokio_util::task::TaskTracker;

use super::types::Result;
use super::types::WorkerService;
use super::types::WorkerServiceError;
use crate::config::WorkerConfig;

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
            task_tracker: TaskTracker::new(),
            shutdown: Arc::new(tokio::sync::Notify::new()),
        })
    }
}
