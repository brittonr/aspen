//! Worker management business logic
//!
//! This service handles worker lifecycle operations including registration,
//! heartbeats, health monitoring, and orphaned job recovery.

use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;

use crate::domain::types::{Worker, WorkerHeartbeat, WorkerRegistration, WorkerStats, WorkerStatus, JobStatus};
use crate::repositories::{WorkerRepository, WorkRepository};

/// Service managing worker lifecycle and health
///
/// Responsibilities:
/// - Worker registration
/// - Heartbeat processing
/// - Health monitoring (background task)
/// - Orphaned job recovery
/// - Worker statistics
pub struct WorkerManagementService {
    worker_repo: Arc<dyn WorkerRepository>,
    work_repo: Arc<dyn WorkRepository>,
    heartbeat_timeout_secs: i64,
}

impl WorkerManagementService {
    /// Create a new worker management service
    ///
    /// # Arguments
    /// * `worker_repo` - Repository for worker state
    /// * `work_repo` - Repository for job state (needed for orphaned job recovery)
    /// * `heartbeat_timeout_secs` - Seconds before a worker is considered offline (default: 60)
    pub fn new(
        worker_repo: Arc<dyn WorkerRepository>,
        work_repo: Arc<dyn WorkRepository>,
        heartbeat_timeout_secs: Option<i64>,
    ) -> Self {
        Self {
            worker_repo,
            work_repo,
            heartbeat_timeout_secs: heartbeat_timeout_secs.unwrap_or(60),
        }
    }

    /// Register a new worker
    ///
    /// Creates a new worker record with a unique ID and sets status to Online.
    ///
    /// # Arguments
    /// * `registration` - Worker registration information
    ///
    /// # Returns
    /// The newly created Worker with generated ID
    pub async fn register_worker(&self, registration: WorkerRegistration) -> Result<Worker> {
        let worker = self.worker_repo.register(registration).await?;

        tracing::info!(
            worker_id = %worker.id,
            worker_type = %worker.worker_type,
            endpoint_id = %worker.endpoint_id,
            "Worker registered successfully"
        );

        Ok(worker)
    }

    /// Process a worker heartbeat
    ///
    /// Updates the worker's last_heartbeat timestamp and optionally updates
    /// resource information.
    ///
    /// # Arguments
    /// * `heartbeat` - Heartbeat update information
    pub async fn handle_heartbeat(&self, heartbeat: WorkerHeartbeat) -> Result<()> {
        self.worker_repo.heartbeat(heartbeat).await?;
        Ok(())
    }

    /// Mark a worker as draining (graceful shutdown)
    ///
    /// Sets worker status to Draining, which prevents new job assignments
    /// while allowing current jobs to complete.
    ///
    /// # Arguments
    /// * `worker_id` - The worker to drain
    pub async fn mark_worker_draining(&self, worker_id: &str) -> Result<()> {
        self.worker_repo.mark_draining(worker_id).await?;

        tracing::info!(
            worker_id = %worker_id,
            "Worker marked as draining (graceful shutdown)"
        );

        Ok(())
    }

    /// Get a specific worker by ID
    ///
    /// # Arguments
    /// * `worker_id` - The worker identifier
    ///
    /// # Returns
    /// The worker if found, None otherwise
    pub async fn get_worker(&self, worker_id: &str) -> Result<Option<Worker>> {
        self.worker_repo.find_by_id(worker_id).await
    }

    /// List all online workers
    pub async fn list_online_workers(&self) -> Result<Vec<Worker>> {
        self.worker_repo.list_online_workers().await
    }

    /// List all workers regardless of status
    pub async fn list_all_workers(&self) -> Result<Vec<Worker>> {
        self.worker_repo.list_all_workers().await
    }

    /// Get worker pool statistics
    pub async fn get_worker_stats(&self) -> Result<WorkerStats> {
        self.worker_repo.get_stats().await
    }

    /// Health monitor loop (run as background task)
    ///
    /// This should be spawned as a tokio task on startup. It periodically:
    /// 1. Checks all workers for stale heartbeats
    /// 2. Marks stale workers as offline
    /// 3. Requeues their orphaned jobs
    ///
    /// # Arguments
    /// * `check_interval_secs` - How often to check worker health (default: 30)
    ///
    /// # Example
    /// ```no_run
    /// tokio::spawn(async move {
    ///     service.health_monitor_loop(None).await;
    /// });
    /// ```
    pub async fn health_monitor_loop(&self, check_interval_secs: Option<u64>) {
        let interval = Duration::from_secs(check_interval_secs.unwrap_or(30));

        tracing::info!(
            check_interval_secs = interval.as_secs(),
            heartbeat_timeout_secs = self.heartbeat_timeout_secs,
            "Starting worker health monitor loop"
        );

        loop {
            tokio::time::sleep(interval).await;

            if let Err(e) = self.check_worker_health().await {
                tracing::error!(error = %e, "Error in worker health check");
            }
        }
    }

    /// Check all workers for stale heartbeats and handle failures
    ///
    /// Called by health_monitor_loop. Can also be called manually for testing.
    pub async fn check_worker_health(&self) -> Result<()> {
        let workers = self.worker_repo.list_all_workers().await?;

        for worker in workers {
            // Only check online or draining workers
            if !matches!(worker.status, WorkerStatus::Online | WorkerStatus::Draining) {
                continue;
            }

            // Check if heartbeat is stale
            if !worker.is_healthy(self.heartbeat_timeout_secs) {
                let age = worker.heartbeat_age_seconds();

                tracing::warn!(
                    worker_id = %worker.id,
                    worker_type = %worker.worker_type,
                    heartbeat_age_secs = age,
                    "Worker heartbeat timeout - marking offline"
                );

                // Mark worker as offline
                self.worker_repo.mark_offline(&worker.id).await?;

                // Requeue orphaned jobs
                if let Err(e) = self.requeue_orphaned_jobs(&worker.id).await {
                    tracing::error!(
                        worker_id = %worker.id,
                        error = %e,
                        "Failed to requeue orphaned jobs"
                    );
                }
            }
        }

        Ok(())
    }

    /// Requeue jobs orphaned by a failed worker
    ///
    /// Finds all in-progress jobs for the given worker and resets them to pending.
    ///
    /// # Arguments
    /// * `worker_id` - The failed worker whose jobs need requeuing
    async fn requeue_orphaned_jobs(&self, worker_id: &str) -> Result<()> {
        // Find all in-progress jobs for this worker
        let orphaned_jobs = self
            .work_repo
            .find_by_status_and_worker(JobStatus::InProgress, worker_id)
            .await?;

        if orphaned_jobs.is_empty() {
            tracing::debug!(
                worker_id = %worker_id,
                "No orphaned jobs to requeue"
            );
            return Ok(());
        }

        tracing::info!(
            worker_id = %worker_id,
            orphaned_job_count = orphaned_jobs.len(),
            "Requeuing orphaned jobs from failed worker"
        );

        for job in orphaned_jobs {
            // Reset job to pending status
            match self.work_repo.update_status(&job.id, JobStatus::Pending).await {
                Ok(_) => {
                    tracing::info!(
                        job_id = %job.id,
                        worker_id = %worker_id,
                        "Orphaned job requeued successfully"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        job_id = %job.id,
                        worker_id = %worker_id,
                        error = %e,
                        "Failed to requeue orphaned job"
                    );
                }
            }
        }

        Ok(())
    }

    /// Increment job count when worker claims a job
    ///
    /// # Arguments
    /// * `worker_id` - The worker that claimed the job
    pub async fn increment_worker_job_count(&self, worker_id: &str) -> Result<()> {
        self.worker_repo.increment_job_count(worker_id).await
    }

    /// Decrement job count when worker completes a job
    ///
    /// # Arguments
    /// * `worker_id` - The worker that completed the job
    pub async fn decrement_worker_job_count(&self, worker_id: &str) -> Result<()> {
        self.worker_repo.decrement_job_count(worker_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::mocks::{MockWorkRepository, MockWorkerRepository};
    use crate::domain::types::{Job, WorkerType};
    use serde_json::json;

    fn create_test_worker(id: &str, last_heartbeat: i64) -> Worker {
        Worker {
            id: id.to_string(),
            worker_type: WorkerType::Wasm,
            status: WorkerStatus::Online,
            endpoint_id: "test-endpoint".to_string(),
            registered_at: 1000,
            last_heartbeat,
            cpu_cores: Some(4),
            memory_mb: Some(8192),
            active_jobs: 1,
            total_jobs_completed: 10,
            metadata: json!({}),
        }
    }

    fn create_test_job(id: &str, worker_id: &str) -> Job {
        Job {
            id: id.to_string(),
            status: JobStatus::InProgress,
            claimed_by: Some(worker_id.to_string()),
            completed_by: None,
            created_at: 1000,
            updated_at: 1100,
            started_at: Some(1050),
            error_message: None,
            retry_count: 0,
            assigned_worker_id: None,
            compatible_worker_types: Vec::new(),
            payload: json!({"url": "https://example.com"}),
        }
    }

    #[tokio::test]
    async fn test_register_worker() {
        let worker_repo = Arc::new(MockWorkerRepository::new());
        let work_repo = Arc::new(MockWorkRepository::new());
        let service = WorkerManagementService::new(worker_repo, work_repo, Some(60));

        let registration = WorkerRegistration {
            worker_type: WorkerType::Wasm,
            endpoint_id: "test-endpoint".to_string(),
            cpu_cores: Some(4),
            memory_mb: Some(8192),
            metadata: json!({"version": "0.1.0"}),
        };

        let worker = service.register_worker(registration).await.unwrap();

        assert!(worker.id.starts_with("worker-"));
        assert_eq!(worker.worker_type, WorkerType::Wasm);
        assert_eq!(worker.status, WorkerStatus::Online);
    }

    #[tokio::test]
    async fn test_handle_heartbeat() {
        let worker_repo = Arc::new(MockWorkerRepository::new());
        let work_repo = Arc::new(MockWorkRepository::new());

        // Pre-populate with a worker
        let worker = create_test_worker("worker-1", 1000);
        worker_repo.add_workers(vec![worker]).await;

        let service = WorkerManagementService::new(worker_repo, work_repo, Some(60));

        let heartbeat = WorkerHeartbeat {
            worker_id: "worker-1".to_string(),
            active_jobs: 2,
            cpu_cores: Some(4),
            memory_mb: Some(8192),
        };

        service.handle_heartbeat(heartbeat).await.unwrap();

        let worker = service.get_worker("worker-1").await.unwrap().unwrap();
        assert_eq!(worker.active_jobs, 2);
    }

    #[tokio::test]
    async fn test_requeue_orphaned_jobs() {
        let worker_repo = Arc::new(MockWorkerRepository::new());
        let work_repo = Arc::new(MockWorkRepository::new());

        // Pre-populate with jobs
        let job1 = create_test_job("job-1", "worker-1");
        let job2 = create_test_job("job-2", "worker-1");
        work_repo.add_jobs(vec![job1, job2]).await;

        let service = WorkerManagementService::new(worker_repo, work_repo.clone(), Some(60));

        // Trigger orphaned job recovery
        service.requeue_orphaned_jobs("worker-1").await.unwrap();

        // Verify jobs were requeued
        let job1 = work_repo.find_by_id("job-1").await.unwrap().unwrap();
        let job2 = work_repo.find_by_id("job-2").await.unwrap().unwrap();

        assert_eq!(job1.status, JobStatus::Pending);
        assert_eq!(job2.status, JobStatus::Pending);
    }
}
