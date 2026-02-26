//! Handler registration and worker pool start logic.

use std::time::Duration;

use aspen_jobs::Worker;
use aspen_jobs::WorkerConfig as JobWorkerConfig;
use snafu::ResultExt;
use tracing::info;

use super::types::RegisterHandlerSnafu;
use super::types::Result;
use super::types::StartWorkersSnafu;
use super::types::WorkerService;
use super::types::WorkerServiceError;

impl WorkerService {
    /// Register a worker handler for a specific job type.
    ///
    /// # Arguments
    ///
    /// * `job_type` - Type of jobs this handler processes
    /// * `handler` - Worker implementation
    pub async fn register_handler<W: Worker>(&self, job_type: &str, handler: W) -> Result<()> {
        // Tiger Style: argument validation
        debug_assert!(!job_type.is_empty(), "WORKER_SERVICE: job_type must not be empty");
        debug_assert!(self.node_id > 0, "WORKER_SERVICE: node_id must be positive");

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
        // Tiger Style: validate config before starting
        debug_assert!(
            self.config.worker_count > 0 || !self.config.is_enabled,
            "WORKER_SERVICE: worker_count must be positive when enabled"
        );

        if !self.config.is_enabled {
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

            self.pool.spawn_worker(worker_config_copy).await.context(StartWorkersSnafu { count: 1u32 })?;
        }

        // Update worker metadata for affinity routing
        self.update_worker_metadata().await?;

        // Start monitoring task
        self.start_monitoring();

        info!(node_id = self.node_id, worker_count = self.config.worker_count, "worker service started");

        Ok(())
    }
}
