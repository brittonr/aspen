//! WorkQueue-based implementation of WorkRepository
//!
//! This implementation directly passes through domain types to the WorkQueue
//! infrastructure layer, which now uses domain types natively.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value as JsonValue;

use crate::work_queue::WorkQueue;
use crate::repositories::WorkRepository;
use crate::domain::types::{Job, JobStatus, QueueStats};

/// WorkRepository implementation backed by WorkQueue
#[derive(Clone)]
pub struct WorkQueueWorkRepository {
    work_queue: WorkQueue,
}

impl WorkQueueWorkRepository {
    /// Create a new WorkQueue-backed work repository
    pub fn new(work_queue: WorkQueue) -> Self {
        Self { work_queue }
    }
}

#[async_trait]
impl WorkRepository for WorkQueueWorkRepository {
    async fn publish_work(&self, job_id: String, payload: JsonValue) -> Result<()> {
        self.work_queue.publish_work(job_id, payload).await
    }

    async fn claim_work(&self, worker_id: Option<&str>, worker_type: Option<crate::domain::types::WorkerType>) -> Result<Option<Job>> {
        self.work_queue.claim_work(worker_id, worker_type).await
    }

    async fn update_status(&self, job_id: &str, status: JobStatus) -> Result<()> {
        self.work_queue.update_status(job_id, status).await
    }

    async fn find_by_id(&self, job_id: &str) -> Result<Option<Job>> {
        self.work_queue.get_work_by_id(job_id).await
    }

    async fn find_by_status(&self, status: JobStatus) -> Result<Vec<Job>> {
        self.work_queue.list_work_by_status(status).await
    }

    async fn find_by_worker(&self, worker_id: &str) -> Result<Vec<Job>> {
        self.work_queue.list_work_by_worker(worker_id).await
    }

    async fn find_paginated(&self, offset: usize, limit: usize) -> Result<Vec<Job>> {
        self.work_queue.list_work_paginated(offset, limit).await
    }

    async fn find_by_status_and_worker(&self, status: JobStatus, worker_id: &str) -> Result<Vec<Job>> {
        self.work_queue
            .list_work_by_status_and_worker(status, worker_id)
            .await
    }

    async fn list_work(&self) -> Result<Vec<Job>> {
        self.work_queue.list_work().await
    }

    async fn stats(&self) -> QueueStats {
        self.work_queue.stats().await
    }
}
