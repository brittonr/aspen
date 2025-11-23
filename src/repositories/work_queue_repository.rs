//! WorkQueue-based implementation of WorkRepository

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value as JsonValue;

use crate::work_queue::{WorkQueue, WorkItem, WorkQueueStats, WorkStatus};
use crate::repositories::WorkRepository;

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

    async fn claim_work(&self) -> Result<Option<WorkItem>> {
        self.work_queue.claim_work().await
    }

    async fn update_status(&self, job_id: &str, status: WorkStatus) -> Result<()> {
        self.work_queue.update_status(job_id, status).await
    }

    async fn list_work(&self) -> Result<Vec<WorkItem>> {
        self.work_queue.list_work().await
    }

    async fn stats(&self) -> WorkQueueStats {
        self.work_queue.stats().await
    }
}
