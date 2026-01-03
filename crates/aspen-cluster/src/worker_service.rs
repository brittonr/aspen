//! Worker service for managing distributed job execution on Aspen nodes.
//!
//! This module provides the WorkerService that integrates with Aspen nodes to:
//! - Start and manage worker pools based on node configuration
//! - Register workers with the cluster's job manager
//! - Track worker health and report metrics
//! - Handle graceful shutdown during node termination
//!
//! # Tiger Style
//!
//! - Fixed limits on workers and concurrent jobs
//! - Fail-fast on invalid configurations
//! - Graceful shutdown with bounded timeout
//! - Clear separation between worker lifecycle and job execution

use std::sync::Arc;
use std::time::Duration;

use aspen_core::{EndpointProvider, KeyValueStore};
use aspen_jobs::{
    AffinityJobManager, JobManager, Worker, WorkerConfig as JobWorkerConfig, WorkerMetadata,
    WorkerPool, WorkerPoolStats, DistributedWorkerPool, DistributedPoolConfig, DistributedJobRouter,
};
use aspen_coordination::{
    DistributedWorkerCoordinator, WorkerCoordinatorConfig, LoadBalancingStrategy,
};
use iroh::PublicKey as NodeId;
use snafu::{ResultExt, Snafu};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use crate::config::WorkerConfig;

/// Errors that can occur in the worker service.
#[derive(Debug, Snafu)]
pub enum WorkerServiceError {
    /// Failed to initialize worker pool.
    #[snafu(display("failed to initialize worker pool: {}", source))]
    InitializePool {
        source: aspen_jobs::JobError,
    },

    /// Failed to register worker handler.
    #[snafu(display("failed to register worker handler '{}': {}", job_type, source))]
    RegisterHandler {
        job_type: String,
        source: aspen_jobs::JobError,
    },

    /// Failed to start workers.
    #[snafu(display("failed to start {} workers: {}", count, source))]
    StartWorkers {
        count: usize,
        source: aspen_jobs::JobError,
    },

    /// Failed to update worker metadata.
    #[snafu(display("failed to update worker metadata: {}", source))]
    UpdateMetadata {
        source: aspen_jobs::JobError,
    },

    /// Worker configuration is invalid.
    #[snafu(display("invalid worker configuration: {}", reason))]
    InvalidConfig {
        reason: String,
    },

    /// Failed to shutdown workers.
    #[snafu(display("failed to shutdown workers: {}", source))]
    Shutdown {
        source: aspen_jobs::JobError,
    },
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
                reason: format!(
                    "max_concurrent_jobs {} exceeds maximum of 100",
                    config.max_concurrent_jobs
                ),
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
    pub async fn register_handler<W: Worker>(
        &self,
        job_type: &str,
        handler: W,
    ) -> Result<()> {
        info!(node_id = self.node_id, job_type, "registering worker handler");

        self.pool
            .register_handler(job_type, handler)
            .await
            .context(RegisterHandlerSnafu {
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
                    .map_err(|e| WorkerServiceError::InitializePool {
                        source: e,
                    })?;

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

            self.pool
                .spawn_worker(worker_config_copy)
                .await
                .context(StartWorkersSnafu {
                    count: 1usize,
                })?;
        }

        // Update worker metadata for affinity routing
        self.update_worker_metadata().await?;

        // Start monitoring task
        self.start_monitoring();

        info!(
            node_id = self.node_id,
            worker_count = self.config.worker_count,
            "worker service started"
        );

        Ok(())
    }

    /// Update worker metadata for P2P affinity routing.
    async fn update_worker_metadata(&self) -> Result<()> {
        let metadata = WorkerMetadata {
            id: format!("node-{}", self.node_id),
            node_id: self.iroh_node_id,
            tags: self.config.tags.clone(),
            region: None, // Could be configured later
            load: 0.0,   // Will be updated by monitoring
            local_blobs: vec![], // Could query blob store
            latencies: Default::default(),
        };

        self.affinity_manager
            .update_worker_metadata(metadata)
            .await;

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