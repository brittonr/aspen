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
    use crate::repositories::mocks::{MockWorkerRepository, MockWorkRepository};
    use crate::domain::types::{WorkerType, JobStatus};
    use crate::domain::job::types::Job;
    use crate::domain::job_metadata::JobMetadata;
    use crate::domain::job_requirements::JobRequirements;
    use serde_json::json;

    // ========================================================================
    // Test Helpers
    // ========================================================================

    fn create_test_service() -> (
        WorkerManagementService,
        Arc<MockWorkerRepository>,
        Arc<MockWorkRepository>,
    ) {
        let worker_repo = Arc::new(MockWorkerRepository::new());
        let work_repo = Arc::new(MockWorkRepository::new());
        let service = WorkerManagementService::new(
            worker_repo.clone() as Arc<dyn WorkerRepository>,
            work_repo.clone() as Arc<dyn WorkRepository>,
            Some(60), // 60 second heartbeat timeout
        );
        (service, worker_repo, work_repo)
    }

    fn create_test_registration(worker_type: WorkerType, endpoint_id: &str) -> WorkerRegistration {
        WorkerRegistration {
            worker_type,
            endpoint_id: endpoint_id.to_string(),
            cpu_cores: Some(8),
            memory_mb: Some(16384),
            metadata: json!({"version": "1.0.0", "hostname": "test-host"}),
        }
    }

    fn create_test_worker(id: &str, worker_type: WorkerType, status: WorkerStatus) -> Worker {
        let now = crate::common::current_timestamp_or_zero();
        Worker {
            id: id.to_string(),
            worker_type,
            status,
            endpoint_id: "endpoint-123".to_string(),
            registered_at: now,
            last_heartbeat: now,
            cpu_cores: Some(8),
            memory_mb: Some(16384),
            active_jobs: 0,
            total_jobs_completed: 0,
            metadata: json!({}),
        }
    }

    fn create_test_job(id: &str, status: JobStatus, worker_id: Option<&str>) -> Job {
        Job {
            id: id.to_string(),
            status,
            payload: json!({"test": "data"}),
            requirements: JobRequirements::default(),
            metadata: JobMetadata::new(),
            error_message: None,
            claimed_by: worker_id.map(|s| s.to_string()),
            assigned_worker_id: worker_id.map(|s| s.to_string()),
            completed_by: None,
        }
    }

    // ========================================================================
    // Worker Registration Tests
    // ========================================================================

    #[tokio::test]
    async fn test_register_worker_firecracker() {
        let (service, worker_repo, _) = create_test_service();

        let registration = create_test_registration(WorkerType::Firecracker, "endpoint-firecracker");
        let worker = service
            .register_worker(registration)
            .await
            .expect("Registration should succeed");

        assert_eq!(worker.worker_type, WorkerType::Firecracker);
        assert_eq!(worker.status, WorkerStatus::Online);
        assert_eq!(worker.endpoint_id, "endpoint-firecracker");
        assert_eq!(worker.cpu_cores, Some(8));
        assert_eq!(worker.memory_mb, Some(16384));
        assert_eq!(worker.active_jobs, 0);
        assert_eq!(worker.total_jobs_completed, 0);

        // Verify worker is in repository
        let found_worker = worker_repo
            .find_by_id(&worker.id)
            .await
            .expect("Should find worker")
            .expect("Worker should exist");
        assert_eq!(found_worker.id, worker.id);
    }

    #[tokio::test]
    async fn test_register_worker_wasm() {
        let (service, _, _) = create_test_service();

        let registration = create_test_registration(WorkerType::Wasm, "endpoint-wasm");
        let worker = service
            .register_worker(registration)
            .await
            .expect("Registration should succeed");

        assert_eq!(worker.worker_type, WorkerType::Wasm);
        assert_eq!(worker.status, WorkerStatus::Online);
    }

    #[tokio::test]
    async fn test_register_worker_minimal_registration() {
        let (service, _, _) = create_test_service();

        let registration = WorkerRegistration {
            worker_type: WorkerType::Firecracker,
            endpoint_id: "endpoint-minimal".to_string(),
            cpu_cores: None,
            memory_mb: None,
            metadata: json!({}),
        };

        let worker = service
            .register_worker(registration)
            .await
            .expect("Registration should succeed");

        assert_eq!(worker.cpu_cores, None);
        assert_eq!(worker.memory_mb, None);
    }

    #[tokio::test]
    async fn test_register_multiple_workers() {
        let (service, worker_repo, _) = create_test_service();

        let reg1 = create_test_registration(WorkerType::Firecracker, "endpoint-1");
        let reg2 = create_test_registration(WorkerType::Wasm, "endpoint-2");

        let worker1 = service.register_worker(reg1).await.expect("Should succeed");
        let worker2 = service.register_worker(reg2).await.expect("Should succeed");

        // Workers should have unique IDs
        assert_ne!(worker1.id, worker2.id);

        // Both should be in repository
        let all_workers = worker_repo.list_all_workers().await.expect("Should list");
        assert_eq!(all_workers.len(), 2);
    }

    // ========================================================================
    // Heartbeat Handling Tests
    // ========================================================================

    #[tokio::test]
    async fn test_handle_heartbeat_updates_timestamp() {
        let (service, worker_repo, _) = create_test_service();

        let registration = create_test_registration(WorkerType::Firecracker, "endpoint-1");
        let worker = service.register_worker(registration).await.expect("Should succeed");
        let initial_heartbeat = worker.last_heartbeat;

        // Wait a bit to ensure timestamp changes
        tokio::time::sleep(Duration::from_millis(10)).await;

        let heartbeat = WorkerHeartbeat {
            worker_id: worker.id.clone(),
            active_jobs: 2,
            cpu_cores: None,
            memory_mb: None,
        };

        service
            .handle_heartbeat(heartbeat)
            .await
            .expect("Heartbeat should succeed");

        let updated_worker = worker_repo
            .find_by_id(&worker.id)
            .await
            .expect("Should find worker")
            .expect("Worker should exist");

        assert!(updated_worker.last_heartbeat >= initial_heartbeat);
        assert_eq!(updated_worker.active_jobs, 2);
    }

    #[tokio::test]
    async fn test_handle_heartbeat_updates_resources() {
        let (service, worker_repo, _) = create_test_service();

        let registration = create_test_registration(WorkerType::Firecracker, "endpoint-1");
        let worker = service.register_worker(registration).await.expect("Should succeed");

        let heartbeat = WorkerHeartbeat {
            worker_id: worker.id.clone(),
            active_jobs: 1,
            cpu_cores: Some(16), // Updated from 8
            memory_mb: Some(32768), // Updated from 16384
        };

        service
            .handle_heartbeat(heartbeat)
            .await
            .expect("Heartbeat should succeed");

        let updated_worker = worker_repo
            .find_by_id(&worker.id)
            .await
            .expect("Should find worker")
            .expect("Worker should exist");

        assert_eq!(updated_worker.cpu_cores, Some(16));
        assert_eq!(updated_worker.memory_mb, Some(32768));
        assert_eq!(updated_worker.active_jobs, 1);
    }

    #[tokio::test]
    async fn test_handle_heartbeat_nonexistent_worker() {
        let (service, _, _) = create_test_service();

        let heartbeat = WorkerHeartbeat {
            worker_id: "nonexistent-worker".to_string(),
            active_jobs: 0,
            cpu_cores: None,
            memory_mb: None,
        };

        // Should succeed but have no effect (mock doesn't fail)
        service
            .handle_heartbeat(heartbeat)
            .await
            .expect("Heartbeat should succeed");
    }

    // ========================================================================
    // Worker Draining Tests
    // ========================================================================

    #[tokio::test]
    async fn test_mark_worker_draining() {
        let (service, worker_repo, _) = create_test_service();

        let registration = create_test_registration(WorkerType::Firecracker, "endpoint-1");
        let worker = service.register_worker(registration).await.expect("Should succeed");

        service
            .mark_worker_draining(&worker.id)
            .await
            .expect("Marking draining should succeed");

        let updated_worker = worker_repo
            .find_by_id(&worker.id)
            .await
            .expect("Should find worker")
            .expect("Worker should exist");

        assert_eq!(updated_worker.status, WorkerStatus::Draining);
    }

    #[tokio::test]
    async fn test_mark_draining_prevents_job_acceptance() {
        let (service, worker_repo, _) = create_test_service();

        let registration = create_test_registration(WorkerType::Firecracker, "endpoint-1");
        let worker = service.register_worker(registration).await.expect("Should succeed");

        service
            .mark_worker_draining(&worker.id)
            .await
            .expect("Should succeed");

        let draining_worker = worker_repo
            .find_by_id(&worker.id)
            .await
            .expect("Should find")
            .expect("Should exist");

        assert!(!draining_worker.can_accept_jobs());
    }

    #[tokio::test]
    async fn test_mark_draining_nonexistent_worker() {
        let (service, _, _) = create_test_service();

        // Should succeed but have no effect
        service
            .mark_worker_draining("nonexistent-worker")
            .await
            .expect("Should succeed");
    }

    // ========================================================================
    // Worker Query Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_worker_by_id() {
        let (service, _, _) = create_test_service();

        let registration = create_test_registration(WorkerType::Firecracker, "endpoint-1");
        let worker = service.register_worker(registration).await.expect("Should succeed");

        let found_worker = service
            .get_worker(&worker.id)
            .await
            .expect("Query should succeed")
            .expect("Worker should exist");

        assert_eq!(found_worker.id, worker.id);
        assert_eq!(found_worker.worker_type, WorkerType::Firecracker);
    }

    #[tokio::test]
    async fn test_get_worker_nonexistent() {
        let (service, _, _) = create_test_service();

        let result = service
            .get_worker("nonexistent-worker")
            .await
            .expect("Query should succeed");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_list_online_workers() {
        let (service, worker_repo, _) = create_test_service();

        // Register 3 workers
        for i in 1..=3 {
            let reg = create_test_registration(WorkerType::Firecracker, &format!("endpoint-{}", i));
            service.register_worker(reg).await.expect("Should succeed");
        }

        // Mark one as draining and one as offline
        let all = worker_repo.list_all_workers().await.expect("Should list");
        worker_repo.mark_draining(&all[1].id).await.expect("Should succeed");
        worker_repo.mark_offline(&all[2].id).await.expect("Should succeed");

        let online = service.list_online_workers().await.expect("Should succeed");
        assert_eq!(online.len(), 1);
        assert_eq!(online[0].status, WorkerStatus::Online);
    }

    #[tokio::test]
    async fn test_list_all_workers() {
        let (service, worker_repo, _) = create_test_service();

        // Register 3 workers with different statuses
        let reg1 = create_test_registration(WorkerType::Firecracker, "endpoint-1");
        let reg2 = create_test_registration(WorkerType::Wasm, "endpoint-2");
        let reg3 = create_test_registration(WorkerType::Firecracker, "endpoint-3");

        let _w1 = service.register_worker(reg1).await.expect("Should succeed");
        let w2 = service.register_worker(reg2).await.expect("Should succeed");
        let w3 = service.register_worker(reg3).await.expect("Should succeed");

        worker_repo.mark_draining(&w2.id).await.expect("Should succeed");
        worker_repo.mark_offline(&w3.id).await.expect("Should succeed");

        let all = service.list_all_workers().await.expect("Should succeed");
        assert_eq!(all.len(), 3);

        // Verify different statuses
        let statuses: Vec<_> = all.iter().map(|w| w.status).collect();
        assert!(statuses.contains(&WorkerStatus::Online));
        assert!(statuses.contains(&WorkerStatus::Draining));
        assert!(statuses.contains(&WorkerStatus::Offline));
    }

    #[tokio::test]
    async fn test_list_workers_empty() {
        let (service, _, _) = create_test_service();

        let online = service.list_online_workers().await.expect("Should succeed");
        assert_eq!(online.len(), 0);

        let all = service.list_all_workers().await.expect("Should succeed");
        assert_eq!(all.len(), 0);
    }

    // ========================================================================
    // Worker Statistics Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_worker_stats_empty() {
        let (service, _, _) = create_test_service();

        let stats = service.get_worker_stats().await.expect("Should succeed");
        assert_eq!(stats.total, 0);
        assert_eq!(stats.online, 0);
        assert_eq!(stats.draining, 0);
        assert_eq!(stats.offline, 0);
        assert_eq!(stats.total_active_jobs, 0);
        assert_eq!(stats.total_completed_jobs, 0);
    }

    #[tokio::test]
    async fn test_get_worker_stats_with_workers() {
        let (service, worker_repo, _) = create_test_service();

        // Create workers with different statuses
        let w1 = create_test_worker("worker-1", WorkerType::Firecracker, WorkerStatus::Online);
        let w2 = create_test_worker("worker-2", WorkerType::Wasm, WorkerStatus::Online);
        let mut w3 = create_test_worker("worker-3", WorkerType::Firecracker, WorkerStatus::Draining);
        let mut w4 = create_test_worker("worker-4", WorkerType::Firecracker, WorkerStatus::Offline);

        // Set job counts
        w3.active_jobs = 2;
        w3.total_jobs_completed = 10;
        w4.total_jobs_completed = 5;

        worker_repo.add_workers(vec![w1, w2, w3, w4]).await;

        let stats = service.get_worker_stats().await.expect("Should succeed");
        assert_eq!(stats.total, 4);
        assert_eq!(stats.online, 2);
        assert_eq!(stats.draining, 1);
        assert_eq!(stats.offline, 1);
        assert_eq!(stats.total_active_jobs, 2);
        assert_eq!(stats.total_completed_jobs, 15);
    }

    // ========================================================================
    // Worker Health Monitoring Tests
    // ========================================================================

    #[tokio::test]
    async fn test_check_worker_health_all_healthy() {
        let (service, worker_repo, _) = create_test_service();

        let w1 = create_test_worker("worker-1", WorkerType::Firecracker, WorkerStatus::Online);
        let w2 = create_test_worker("worker-2", WorkerType::Wasm, WorkerStatus::Online);
        worker_repo.add_workers(vec![w1, w2]).await;

        service
            .check_worker_health()
            .await
            .expect("Health check should succeed");

        // All workers should still be online
        let workers = worker_repo.list_online_workers().await.expect("Should list");
        assert_eq!(workers.len(), 2);
    }

    #[tokio::test]
    async fn test_check_worker_health_marks_stale_offline() {
        let (service, worker_repo, _) = create_test_service();

        let now = crate::common::current_timestamp_or_zero();
        let mut stale_worker = create_test_worker("worker-1", WorkerType::Firecracker, WorkerStatus::Online);
        stale_worker.last_heartbeat = now - 120; // 2 minutes ago (stale)

        let fresh_worker = create_test_worker("worker-2", WorkerType::Wasm, WorkerStatus::Online);

        worker_repo.add_workers(vec![stale_worker, fresh_worker]).await;

        service
            .check_worker_health()
            .await
            .expect("Health check should succeed");

        // Stale worker should be marked offline
        let w1 = worker_repo
            .find_by_id("worker-1")
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(w1.status, WorkerStatus::Offline);

        // Fresh worker should still be online
        let w2 = worker_repo
            .find_by_id("worker-2")
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(w2.status, WorkerStatus::Online);
    }

    #[tokio::test]
    async fn test_check_worker_health_draining_worker_can_become_offline() {
        let (service, worker_repo, _) = create_test_service();

        let now = crate::common::current_timestamp_or_zero();
        let mut stale_draining = create_test_worker("worker-1", WorkerType::Firecracker, WorkerStatus::Draining);
        stale_draining.last_heartbeat = now - 120; // Stale

        worker_repo.add_workers(vec![stale_draining]).await;

        service
            .check_worker_health()
            .await
            .expect("Health check should succeed");

        let worker = worker_repo
            .find_by_id("worker-1")
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(worker.status, WorkerStatus::Offline);
    }

    #[tokio::test]
    async fn test_check_worker_health_ignores_already_offline() {
        let (service, worker_repo, _) = create_test_service();

        let now = crate::common::current_timestamp_or_zero();
        let mut offline_worker = create_test_worker("worker-1", WorkerType::Firecracker, WorkerStatus::Offline);
        offline_worker.last_heartbeat = now - 3600; // 1 hour ago

        worker_repo.add_workers(vec![offline_worker]).await;

        service
            .check_worker_health()
            .await
            .expect("Health check should succeed");

        // Worker should remain offline (not reprocessed)
        let worker = worker_repo
            .find_by_id("worker-1")
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(worker.status, WorkerStatus::Offline);
    }

    #[tokio::test]
    async fn test_check_worker_health_custom_timeout() {
        let (_, worker_repo, work_repo) = create_test_service();

        // Create service with 30 second timeout
        let service = WorkerManagementService::new(
            worker_repo.clone() as Arc<dyn WorkerRepository>,
            work_repo.clone() as Arc<dyn WorkRepository>,
            Some(30),
        );

        let now = crate::common::current_timestamp_or_zero();
        let mut stale_worker = create_test_worker("worker-1", WorkerType::Firecracker, WorkerStatus::Online);
        stale_worker.last_heartbeat = now - 45; // 45 seconds ago (stale with 30s timeout)

        worker_repo.add_workers(vec![stale_worker]).await;

        service
            .check_worker_health()
            .await
            .expect("Health check should succeed");

        let worker = worker_repo
            .find_by_id("worker-1")
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(worker.status, WorkerStatus::Offline);
    }

    // ========================================================================
    // Orphaned Job Recovery Tests
    // ========================================================================

    #[tokio::test]
    async fn test_check_worker_health_requeues_orphaned_jobs() {
        let (service, worker_repo, work_repo) = create_test_service();

        let now = crate::common::current_timestamp_or_zero();
        let mut stale_worker = create_test_worker("worker-1", WorkerType::Firecracker, WorkerStatus::Online);
        stale_worker.last_heartbeat = now - 120; // Stale

        worker_repo.add_workers(vec![stale_worker]).await;

        // Add orphaned jobs for this worker
        let job1 = create_test_job("job-1", JobStatus::InProgress, Some("worker-1"));
        let job2 = create_test_job("job-2", JobStatus::InProgress, Some("worker-1"));
        let job3 = create_test_job("job-3", JobStatus::Completed, Some("worker-1")); // Should not be requeued
        work_repo.add_jobs(vec![job1, job2, job3]).await;

        service
            .check_worker_health()
            .await
            .expect("Health check should succeed");

        // Worker should be offline
        let worker = worker_repo
            .find_by_id("worker-1")
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(worker.status, WorkerStatus::Offline);

        // Orphaned jobs should be requeued
        let job1_updated = work_repo
            .find_by_id("job-1")
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(job1_updated.status, JobStatus::Pending);

        let job2_updated = work_repo
            .find_by_id("job-2")
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(job2_updated.status, JobStatus::Pending);

        // Completed job should remain completed
        let job3_updated = work_repo
            .find_by_id("job-3")
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(job3_updated.status, JobStatus::Completed);
    }

    #[tokio::test]
    async fn test_check_worker_health_no_orphaned_jobs() {
        let (service, worker_repo, _work_repo) = create_test_service();

        let now = crate::common::current_timestamp_or_zero();
        let mut stale_worker = create_test_worker("worker-1", WorkerType::Firecracker, WorkerStatus::Online);
        stale_worker.last_heartbeat = now - 120; // Stale

        worker_repo.add_workers(vec![stale_worker]).await;

        // No jobs for this worker
        service
            .check_worker_health()
            .await
            .expect("Health check should succeed");

        // Worker should be offline
        let worker = worker_repo
            .find_by_id("worker-1")
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(worker.status, WorkerStatus::Offline);
    }

    #[tokio::test]
    async fn test_orphaned_jobs_from_different_workers_not_affected() {
        let (service, worker_repo, work_repo) = create_test_service();

        let now = crate::common::current_timestamp_or_zero();
        let mut stale_worker = create_test_worker("worker-1", WorkerType::Firecracker, WorkerStatus::Online);
        stale_worker.last_heartbeat = now - 120; // Stale

        let healthy_worker = create_test_worker("worker-2", WorkerType::Wasm, WorkerStatus::Online);

        worker_repo.add_workers(vec![stale_worker, healthy_worker]).await;

        // Add jobs for both workers
        let job1 = create_test_job("job-1", JobStatus::InProgress, Some("worker-1"));
        let job2 = create_test_job("job-2", JobStatus::InProgress, Some("worker-2"));
        work_repo.add_jobs(vec![job1, job2]).await;

        service
            .check_worker_health()
            .await
            .expect("Health check should succeed");

        // Only worker-1's job should be requeued
        let job1_updated = work_repo
            .find_by_id("job-1")
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(job1_updated.status, JobStatus::Pending);

        // Worker-2's job should remain in progress
        let job2_updated = work_repo
            .find_by_id("job-2")
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(job2_updated.status, JobStatus::InProgress);
    }

    // ========================================================================
    // Job Count Management Tests
    // ========================================================================

    #[tokio::test]
    async fn test_increment_worker_job_count() {
        let (service, worker_repo, _) = create_test_service();

        let registration = create_test_registration(WorkerType::Firecracker, "endpoint-1");
        let worker = service.register_worker(registration).await.expect("Should succeed");

        service
            .increment_worker_job_count(&worker.id)
            .await
            .expect("Increment should succeed");

        let updated_worker = worker_repo
            .find_by_id(&worker.id)
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(updated_worker.active_jobs, 1);
    }

    #[tokio::test]
    async fn test_decrement_worker_job_count() {
        let (service, worker_repo, _) = create_test_service();

        let mut worker = create_test_worker("worker-1", WorkerType::Firecracker, WorkerStatus::Online);
        worker.active_jobs = 3;
        worker.total_jobs_completed = 10;
        worker_repo.add_workers(vec![worker]).await;

        service
            .decrement_worker_job_count("worker-1")
            .await
            .expect("Decrement should succeed");

        let updated_worker = worker_repo
            .find_by_id("worker-1")
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(updated_worker.active_jobs, 2);
        assert_eq!(updated_worker.total_jobs_completed, 11);
    }

    #[tokio::test]
    async fn test_decrement_worker_job_count_saturates_at_zero() {
        let (service, worker_repo, _) = create_test_service();

        let worker = create_test_worker("worker-1", WorkerType::Firecracker, WorkerStatus::Online);
        worker_repo.add_workers(vec![worker]).await;

        service
            .decrement_worker_job_count("worker-1")
            .await
            .expect("Decrement should succeed");

        let updated_worker = worker_repo
            .find_by_id("worker-1")
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(updated_worker.active_jobs, 0); // Should not underflow
        assert_eq!(updated_worker.total_jobs_completed, 1);
    }

    #[tokio::test]
    async fn test_increment_decrement_job_count_lifecycle() {
        let (service, worker_repo, _) = create_test_service();

        let registration = create_test_registration(WorkerType::Firecracker, "endpoint-1");
        let worker = service.register_worker(registration).await.expect("Should succeed");

        // Simulate worker claiming and completing 3 jobs
        for _ in 0..3 {
            service
                .increment_worker_job_count(&worker.id)
                .await
                .expect("Increment should succeed");
        }

        let mid_worker = worker_repo
            .find_by_id(&worker.id)
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(mid_worker.active_jobs, 3);
        assert_eq!(mid_worker.total_jobs_completed, 0);

        // Complete 2 jobs
        for _ in 0..2 {
            service
                .decrement_worker_job_count(&worker.id)
                .await
                .expect("Decrement should succeed");
        }

        let final_worker = worker_repo
            .find_by_id(&worker.id)
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(final_worker.active_jobs, 1);
        assert_eq!(final_worker.total_jobs_completed, 2);
    }

    // ========================================================================
    // Service Construction Tests
    // ========================================================================

    #[tokio::test]
    async fn test_service_new_with_default_timeout() {
        let worker_repo = Arc::new(MockWorkerRepository::new());
        let work_repo = Arc::new(MockWorkRepository::new());

        let service = WorkerManagementService::new(
            worker_repo.clone() as Arc<dyn WorkerRepository>,
            work_repo.clone() as Arc<dyn WorkRepository>,
            None, // Use default
        );

        assert_eq!(service.heartbeat_timeout_secs, 60);
    }

    #[tokio::test]
    async fn test_service_new_with_custom_timeout() {
        let worker_repo = Arc::new(MockWorkerRepository::new());
        let work_repo = Arc::new(MockWorkRepository::new());

        let service = WorkerManagementService::new(
            worker_repo.clone() as Arc<dyn WorkerRepository>,
            work_repo.clone() as Arc<dyn WorkRepository>,
            Some(120),
        );

        assert_eq!(service.heartbeat_timeout_secs, 120);
    }

    // ========================================================================
    // Integration Tests (Full Lifecycle)
    // ========================================================================

    #[tokio::test]
    async fn test_full_worker_lifecycle_healthy() {
        let (service, worker_repo, _) = create_test_service();

        // 1. Register worker
        let registration = create_test_registration(WorkerType::Firecracker, "endpoint-1");
        let worker = service.register_worker(registration).await.expect("Should succeed");
        assert_eq!(worker.status, WorkerStatus::Online);

        // 2. Send heartbeats
        for i in 0..3 {
            let heartbeat = WorkerHeartbeat {
                worker_id: worker.id.clone(),
                active_jobs: i,
                cpu_cores: None,
                memory_mb: None,
            };
            service.handle_heartbeat(heartbeat).await.expect("Should succeed");
        }

        // 3. Check health (should remain healthy)
        service.check_worker_health().await.expect("Should succeed");

        let healthy_worker = worker_repo
            .find_by_id(&worker.id)
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(healthy_worker.status, WorkerStatus::Online);

        // 4. Initiate graceful shutdown
        service.mark_worker_draining(&worker.id).await.expect("Should succeed");

        let draining_worker = worker_repo
            .find_by_id(&worker.id)
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(draining_worker.status, WorkerStatus::Draining);
        assert!(!draining_worker.can_accept_jobs());
    }

    #[tokio::test]
    async fn test_full_worker_lifecycle_failure_recovery() {
        let (service, worker_repo, work_repo) = create_test_service();

        // 1. Register worker
        let registration = create_test_registration(WorkerType::Firecracker, "endpoint-1");
        let worker = service.register_worker(registration).await.expect("Should succeed");

        // 2. Worker claims job
        let job = create_test_job("job-1", JobStatus::InProgress, Some(&worker.id));
        work_repo.add_jobs(vec![job]).await;

        service
            .increment_worker_job_count(&worker.id)
            .await
            .expect("Should succeed");

        // 3. Simulate worker failure (no heartbeat, manual timestamp manipulation)
        let now = crate::common::current_timestamp_or_zero();
        let mut failed_worker = worker_repo
            .find_by_id(&worker.id)
            .await
            .expect("Should find")
            .expect("Should exist");
        failed_worker.last_heartbeat = now - 120; // Stale
        worker_repo.add_workers(vec![failed_worker]).await; // Replace with stale version

        // 4. Health monitor detects failure
        service.check_worker_health().await.expect("Should succeed");

        // 5. Worker should be offline
        let offline_worker = worker_repo
            .find_by_id(&worker.id)
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(offline_worker.status, WorkerStatus::Offline);

        // 6. Job should be requeued
        let requeued_job = work_repo
            .find_by_id("job-1")
            .await
            .expect("Should find")
            .expect("Should exist");
        assert_eq!(requeued_job.status, JobStatus::Pending);
    }

    #[tokio::test]
    async fn test_multiple_workers_concurrent_operations() {
        let (service, _worker_repo, _) = create_test_service();

        // Register multiple workers
        let mut worker_ids = Vec::new();
        for i in 1..=5 {
            let reg = create_test_registration(
                if i % 2 == 0 { WorkerType::Wasm } else { WorkerType::Firecracker },
                &format!("endpoint-{}", i),
            );
            let worker = service.register_worker(reg).await.expect("Should succeed");
            worker_ids.push(worker.id);
        }

        // Send heartbeats for all
        for worker_id in &worker_ids {
            let heartbeat = WorkerHeartbeat {
                worker_id: worker_id.clone(),
                active_jobs: 1,
                cpu_cores: None,
                memory_mb: None,
            };
            service.handle_heartbeat(heartbeat).await.expect("Should succeed");
        }

        // Mark some as draining
        service.mark_worker_draining(&worker_ids[1]).await.expect("Should succeed");
        service.mark_worker_draining(&worker_ids[3]).await.expect("Should succeed");

        // Check statistics
        let stats = service.get_worker_stats().await.expect("Should succeed");
        assert_eq!(stats.total, 5);
        assert_eq!(stats.online, 3);
        assert_eq!(stats.draining, 2);
        assert_eq!(stats.offline, 0);

        // Verify online workers list
        let online = service.list_online_workers().await.expect("Should succeed");
        assert_eq!(online.len(), 3);

        // Verify all workers list
        let all = service.list_all_workers().await.expect("Should succeed");
        assert_eq!(all.len(), 5);
    }

    #[tokio::test]
    async fn test_worker_health_calculation() {
        let now = crate::common::current_timestamp_or_zero();

        let mut healthy_worker = create_test_worker("worker-1", WorkerType::Firecracker, WorkerStatus::Online);
        healthy_worker.last_heartbeat = now - 30; // 30 seconds ago

        let mut stale_worker = create_test_worker("worker-2", WorkerType::Wasm, WorkerStatus::Online);
        stale_worker.last_heartbeat = now - 90; // 90 seconds ago

        // With 60 second timeout
        assert!(healthy_worker.is_healthy(60));
        assert!(!stale_worker.is_healthy(60));

        // With 120 second timeout
        assert!(healthy_worker.is_healthy(120));
        assert!(stale_worker.is_healthy(120));
    }
}
