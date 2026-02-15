//! Worker service shutdown, statistics, and health checks.

use std::sync::Arc;

use aspen_core::KeyValueStore;
use aspen_jobs::DistributedJobRouter;
use aspen_jobs::WorkerPoolStats;
use snafu::ResultExt;
use tracing::info;

use super::types::Result;
use super::types::ShutdownSnafu;
use super::types::WorkerService;

impl WorkerService {
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

        // Close task tracker and wait for all tracked tasks to complete
        self.task_tracker.close();
        self.task_tracker.wait().await;

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
