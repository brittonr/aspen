//! Mock repository implementations for testing

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::repositories::{StateRepository, WorkRepository};
use crate::domain::types::{Job, JobStatus, QueueStats, HealthStatus};

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
            .unwrap()
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
            completed_by: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            error_message: None,
            retry_count: 0,
            payload,
        };
        self.jobs.lock().await.push(job);
        Ok(())
    }

    async fn claim_work(&self) -> Result<Option<Job>> {
        let mut jobs = self.jobs.lock().await;
        let now = Self::current_timestamp();
        Ok(jobs.iter_mut()
            .find(|job| job.status == JobStatus::Pending)
            .map(|job| {
                job.status = JobStatus::Claimed;
                job.updated_at = now;
                job.clone()
            }))
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
