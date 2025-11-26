//! Mock repository implementations for testing

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::repositories::{StateRepository, WorkRepository, WorkerRepository};
use crate::domain::types::{Job, JobStatus, QueueStats, HealthStatus, Worker, WorkerRegistration, WorkerHeartbeat, WorkerStats, WorkerStatus};

/// Mock implementation of StateRepository for testing
#[derive(Clone)]
pub struct MockStateRepository {
    health: Arc<Mutex<HealthStatus>>,
}

impl MockStateRepository {
    /// Create a new mock repository with default healthy cluster
    pub fn new() -> Self {
        Self {
            health: Arc::new(Mutex::new(HealthStatus {
                is_healthy: true,
                node_count: 3,
                has_leader: true,
            })),
        }
    }

    /// Set the cluster health to return
    pub async fn set_health(&self, health: HealthStatus) {
        *self.health.lock().await = health;
    }
}

#[async_trait]
impl StateRepository for MockStateRepository {
    async fn health_check(&self) -> Result<HealthStatus> {
        Ok(self.health.lock().await.clone())
    }
}

/// Mock implementation of WorkRepository for testing
#[derive(Clone)]
pub struct MockWorkRepository {
    jobs: Arc<Mutex<Vec<Job>>>,
    stats: Arc<Mutex<QueueStats>>,
}

impl MockWorkRepository {
    /// Create a new mock repository with empty queue
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(Mutex::new(QueueStats {
                total: 0,
                pending: 0,
                claimed: 0,
                in_progress: 0,
                completed: 0,
                failed: 0,
            })),
        }
    }

    /// Add jobs for testing
    pub async fn add_jobs(&self, jobs: Vec<Job>) {
        let mut job_list = self.jobs.lock().await;
        job_list.extend(jobs);
    }

    /// Set the stats to return
    pub async fn set_stats(&self, stats: QueueStats) {
        *self.stats.lock().await = stats;
    }

    /// Get current timestamp for testing
    fn current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("System time is before UNIX epoch")
            .as_secs() as i64
    }
}

#[async_trait]
impl WorkRepository for MockWorkRepository {
    async fn publish_work(&self, job_id: String, payload: JsonValue) -> Result<()> {
        let now = Self::current_timestamp();
        let job = Job {
            id: job_id,
            status: JobStatus::Pending,
            claimed_by: None,
            assigned_worker_id: None,
            completed_by: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            error_message: None,
            retry_count: 0,
            payload,
            compatible_worker_types: Vec::new(),
        };
        self.jobs.lock().await.push(job);
        Ok(())
    }

    async fn claim_work(&self, worker_id: Option<&str>, worker_type: Option<crate::domain::types::WorkerType>) -> Result<Option<Job>> {
        let mut jobs = self.jobs.lock().await;
        let now = Self::current_timestamp();

        // Find first pending job that matches worker type (if specified)
        let claimed_job = jobs.iter_mut()
            .find(|job| {
                if job.status != JobStatus::Pending {
                    return false;
                }

                // Check worker type compatibility
                match worker_type {
                    Some(wt) => {
                        // If job has no worker type constraints, it's compatible with all workers
                        if job.compatible_worker_types.is_empty() {
                            true
                        } else {
                            // Otherwise, check if worker type is in the list
                            job.compatible_worker_types.contains(&wt)
                        }
                    }
                    // If no worker type specified, allow claiming any job
                    None => true,
                }
            })
            .map(|job| {
                job.status = JobStatus::Claimed;
                job.assigned_worker_id = worker_id.map(|s| s.to_string());
                job.updated_at = now;
                job.clone()
            });

        Ok(claimed_job)
    }

    async fn update_status(&self, job_id: &str, status: JobStatus) -> Result<()> {
        let mut jobs = self.jobs.lock().await;
        let now = Self::current_timestamp();
        if let Some(job) = jobs.iter_mut().find(|job| job.id == job_id) {
            job.status = status;
            job.updated_at = now;
        }
        Ok(())
    }

    async fn find_by_id(&self, job_id: &str) -> Result<Option<Job>> {
        let jobs = self.jobs.lock().await;
        Ok(jobs.iter().find(|job| job.id == job_id).cloned())
    }

    async fn find_by_status(&self, status: JobStatus) -> Result<Vec<Job>> {
        let jobs = self.jobs.lock().await;
        Ok(jobs
            .iter()
            .filter(|job| job.status == status)
            .cloned()
            .collect())
    }

    async fn find_by_worker(&self, worker_id: &str) -> Result<Vec<Job>> {
        let jobs = self.jobs.lock().await;
        Ok(jobs
            .iter()
            .filter(|job| {
                job.claimed_by
                    .as_ref()
                    .map(|id| id == worker_id)
                    .unwrap_or(false)
            })
            .cloned()
            .collect())
    }

    async fn find_paginated(&self, offset: usize, limit: usize) -> Result<Vec<Job>> {
        let mut jobs = self.jobs.lock().await.clone();

        // Sort by updated_at (most recent first)
        jobs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        // Apply pagination
        Ok(jobs
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect())
    }

    async fn find_by_status_and_worker(&self, status: JobStatus, worker_id: &str) -> Result<Vec<Job>> {
        let jobs = self.jobs.lock().await;
        Ok(jobs
            .iter()
            .filter(|job| {
                job.status == status
                    && job
                        .claimed_by
                        .as_ref()
                        .map(|id| id == worker_id)
                        .unwrap_or(false)
            })
            .cloned()
            .collect())
    }

    async fn list_work(&self) -> Result<Vec<Job>> {
        Ok(self.jobs.lock().await.clone())
    }

    async fn stats(&self) -> QueueStats {
        self.stats.lock().await.clone()
    }
}

/// Mock implementation of WorkerRepository for testing
#[derive(Clone)]
pub struct MockWorkerRepository {
    workers: Arc<Mutex<Vec<Worker>>>,
}

impl MockWorkerRepository {
    /// Create a new mock repository with empty worker pool
    pub fn new() -> Self {
        Self {
            workers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add workers for testing
    pub async fn add_workers(&self, workers: Vec<Worker>) {
        let mut worker_list = self.workers.lock().await;
        worker_list.extend(workers);
    }

    /// Get current timestamp for testing
    fn current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("System time is before UNIX epoch")
            .as_secs() as i64
    }
}

#[async_trait]
impl WorkerRepository for MockWorkerRepository {
    async fn register(&self, registration: WorkerRegistration) -> Result<Worker> {
        let now = Self::current_timestamp();
        let worker_id = format!("worker-{}-{}", registration.worker_type, uuid::Uuid::new_v4());

        let worker = Worker {
            id: worker_id,
            worker_type: registration.worker_type,
            status: WorkerStatus::Online,
            endpoint_id: registration.endpoint_id,
            registered_at: now,
            last_heartbeat: now,
            cpu_cores: registration.cpu_cores,
            memory_mb: registration.memory_mb,
            active_jobs: 0,
            total_jobs_completed: 0,
            metadata: registration.metadata,
        };

        self.workers.lock().await.push(worker.clone());
        Ok(worker)
    }

    async fn heartbeat(&self, heartbeat: WorkerHeartbeat) -> Result<()> {
        let mut workers = self.workers.lock().await;
        let now = Self::current_timestamp();

        if let Some(worker) = workers.iter_mut().find(|w| w.id == heartbeat.worker_id) {
            worker.last_heartbeat = now;
            worker.active_jobs = heartbeat.active_jobs;
            if let Some(cpu_cores) = heartbeat.cpu_cores {
                worker.cpu_cores = Some(cpu_cores);
            }
            if let Some(memory_mb) = heartbeat.memory_mb {
                worker.memory_mb = Some(memory_mb);
            }
        }

        Ok(())
    }

    async fn find_by_id(&self, worker_id: &str) -> Result<Option<Worker>> {
        let workers = self.workers.lock().await;
        Ok(workers.iter().find(|w| w.id == worker_id).cloned())
    }

    async fn list_online_workers(&self) -> Result<Vec<Worker>> {
        let workers = self.workers.lock().await;
        Ok(workers
            .iter()
            .filter(|w| w.status == WorkerStatus::Online)
            .cloned()
            .collect())
    }

    async fn list_all_workers(&self) -> Result<Vec<Worker>> {
        Ok(self.workers.lock().await.clone())
    }

    async fn mark_offline(&self, worker_id: &str) -> Result<()> {
        let mut workers = self.workers.lock().await;
        if let Some(worker) = workers.iter_mut().find(|w| w.id == worker_id) {
            worker.status = WorkerStatus::Offline;
        }
        Ok(())
    }

    async fn mark_draining(&self, worker_id: &str) -> Result<()> {
        let mut workers = self.workers.lock().await;
        if let Some(worker) = workers.iter_mut().find(|w| w.id == worker_id) {
            worker.status = WorkerStatus::Draining;
        }
        Ok(())
    }

    async fn increment_job_count(&self, worker_id: &str) -> Result<()> {
        let mut workers = self.workers.lock().await;
        if let Some(worker) = workers.iter_mut().find(|w| w.id == worker_id) {
            worker.active_jobs += 1;
        }
        Ok(())
    }

    async fn decrement_job_count(&self, worker_id: &str) -> Result<()> {
        let mut workers = self.workers.lock().await;
        if let Some(worker) = workers.iter_mut().find(|w| w.id == worker_id) {
            worker.active_jobs = worker.active_jobs.saturating_sub(1);
            worker.total_jobs_completed += 1;
        }
        Ok(())
    }

    async fn get_stats(&self) -> Result<WorkerStats> {
        let workers = self.workers.lock().await;
        Ok(WorkerStats::from_workers(&workers))
    }
}
