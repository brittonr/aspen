//! WorkQueue-based implementation of WorkRepository
//!
//! This implementation maps between infrastructure types (WorkItem, WorkStatus)
//! and domain types (Job, JobStatus), maintaining proper layer separation.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value as JsonValue;

use crate::work_queue::{WorkQueue, WorkItem, WorkQueueStats, WorkStatus};
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

    /// Map infrastructure WorkItem to domain Job
    fn to_domain_job(work_item: WorkItem) -> Job {
        Job {
            id: work_item.job_id,
            status: Self::to_domain_status(work_item.status),
            claimed_by: work_item.claimed_by,
            completed_by: work_item.completed_by,
            created_at: work_item.created_at,
            updated_at: work_item.updated_at,
            payload: work_item.payload,
        }
    }

    /// Map domain JobStatus to infrastructure WorkStatus
    fn to_infra_status(status: JobStatus) -> WorkStatus {
        match status {
            JobStatus::Pending => WorkStatus::Pending,
            JobStatus::Claimed => WorkStatus::Claimed,
            JobStatus::InProgress => WorkStatus::InProgress,
            JobStatus::Completed => WorkStatus::Completed,
            JobStatus::Failed => WorkStatus::Failed,
        }
    }

    /// Map infrastructure WorkStatus to domain JobStatus
    fn to_domain_status(status: WorkStatus) -> JobStatus {
        match status {
            WorkStatus::Pending => JobStatus::Pending,
            WorkStatus::Claimed => JobStatus::Claimed,
            WorkStatus::InProgress => JobStatus::InProgress,
            WorkStatus::Completed => JobStatus::Completed,
            WorkStatus::Failed => JobStatus::Failed,
        }
    }

    /// Map infrastructure WorkQueueStats to domain QueueStats
    fn to_domain_stats(stats: WorkQueueStats) -> QueueStats {
        // Calculate total from component counts
        let total = stats.pending + stats.claimed + stats.in_progress + stats.completed + stats.failed;

        QueueStats {
            total,
            pending: stats.pending,
            claimed: stats.claimed,
            in_progress: stats.in_progress,
            completed: stats.completed,
            failed: stats.failed,
        }
    }
}

#[async_trait]
impl WorkRepository for WorkQueueWorkRepository {
    async fn publish_work(&self, job_id: String, payload: JsonValue) -> Result<()> {
        // Publish is the same - no need to map
        self.work_queue.publish_work(job_id, payload).await
    }

    async fn claim_work(&self) -> Result<Option<Job>> {
        // Claim infrastructure work item and map to domain Job
        let work_item = self.work_queue.claim_work().await?;
        Ok(work_item.map(Self::to_domain_job))
    }

    async fn update_status(&self, job_id: &str, status: JobStatus) -> Result<()> {
        // Map domain status to infrastructure status
        let infra_status = Self::to_infra_status(status);
        self.work_queue.update_status(job_id, infra_status).await
    }

    async fn list_work(&self) -> Result<Vec<Job>> {
        // List infrastructure work items and map to domain Jobs
        let work_items = self.work_queue.list_work().await?;
        Ok(work_items.into_iter().map(Self::to_domain_job).collect())
    }

    async fn stats(&self) -> QueueStats {
        // Get infrastructure stats and map to domain stats
        let infra_stats = self.work_queue.stats().await;
        Self::to_domain_stats(infra_stats)
    }
}
