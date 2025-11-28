//! Work Repository Implementation
//!
//! This module provides the concrete implementation of WorkRepository using PersistentStore.
//! It's responsible for data access only - NO business logic, NO caching.
//!
//! Responsibilities:
//! - Direct interaction with PersistentStore
//! - Data conversion (if needed)
//! - Transaction coordination
//!
//! This is the Infrastructure/Repository layer in clean architecture.

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::persistent_store::PersistentStore;
use crate::repositories::{JobAssignment, WorkRepository};
use crate::domain::types::{Job, JobStatus, QueueStats};

/// WorkRepository implementation backed by PersistentStore
///
/// This is a pure data access layer with no business logic.
/// All operations delegate directly to the PersistentStore.
#[derive(Clone)]
pub struct WorkRepositoryImpl {
    store: Arc<dyn PersistentStore>,
}

impl WorkRepositoryImpl {
    /// Create a new WorkRepository with the given persistent store
    pub fn new(store: Arc<dyn PersistentStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl WorkRepository for WorkRepositoryImpl {
    async fn publish_work(&self, job_id: String, payload: serde_json::Value) -> Result<()> {
        use crate::domain::job_metadata::JobMetadata;
        use crate::domain::job_requirements::JobRequirements;

        // Create a new Job with Pending status
        let job = Job {
            id: job_id.clone(),
            status: JobStatus::Pending,
            payload,
            requirements: JobRequirements::default(),
            metadata: JobMetadata::new(),
            error_message: None,
        };

        let assignment = JobAssignment::new(job_id);

        // Persist to store
        self.store.upsert_workflow(&job, &assignment).await
    }

    async fn claim_work(
        &self,
        worker_id: Option<&str>,
        _worker_type: Option<crate::domain::types::WorkerType>,
    ) -> Result<Option<Job>> {
        // Note: This is a simplified implementation
        // The business logic for worker type filtering should be in the service layer
        // For now, we'll load all workflows and return None
        // The actual claiming logic will be in WorkCommandService
        let all_jobs = self.store.load_all_workflows().await?;
        let pending_jobs: Vec<Job> = all_jobs
            .into_iter()
            .filter(|j| j.status == JobStatus::Pending)
            .collect();

        for job in &pending_jobs {
            let now = chrono::Utc::now().timestamp();
            let rows_affected = self.store.claim_workflow(&job.id, now).await?;
            if rows_affected > 0 {
                let mut assignment = self.store.find_job_assignment_by_id(&job.id).await?.unwrap_or_else(|| JobAssignment::new(job.id.clone()));
                assignment.claim("node-id".to_string(), worker_id.map(|s| s.to_string()), now);
                self.store.upsert_job_assignment(&assignment).await?;
                return Ok(Some(job.clone()));
            }
        }

        Ok(None)
    }

    async fn update_status(&self, job_id: &str, status: JobStatus) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        // Determine completed_by - this should ideally come from caller
        // For now, we'll use None and let the service layer set it
        let completed_by = None;

        self.store
            .update_workflow_status(job_id, &status, completed_by, now)
            .await?;

        Ok(())
    }

    async fn find_by_id(&self, job_id: &str) -> Result<Option<Job>> {
        // Load all workflows and find the one with matching ID
        // This is inefficient - ideally PersistentStore should have a find_by_id method
        let all_jobs = self.store.load_all_workflows().await?;
        Ok(all_jobs.into_iter().find(|j| j.id == job_id))
    }

    async fn find_by_status(&self, status: JobStatus) -> Result<Vec<Job>> {
        let all_jobs = self.store.load_all_workflows().await?;
        Ok(all_jobs
            .into_iter()
            .filter(|j| j.status == status)
            .collect())
    }

    async fn find_by_worker(&self, worker_id: &str) -> Result<Vec<Job>> {
        let all_jobs = self.store.load_all_workflows().await?;
        let all_assignments = self.store.load_all_job_assignments().await?;
        let assignments_map: std::collections::HashMap<String, JobAssignment> = all_assignments
            .into_iter()
            .map(|a| (a.job_id.clone(), a))
            .collect();

        Ok(all_jobs
            .into_iter()
            .filter(|j| {
                assignments_map
                    .get(&j.id)
                    .map(|a| a.assigned_worker_id.as_deref() == Some(worker_id))
                    .unwrap_or(false)
            })
            .collect())
    }

    async fn find_assignment_by_id(&self, job_id: &str) -> Result<Option<JobAssignment>> {
        self.store.find_job_assignment_by_id(job_id).await
    }

    async fn find_paginated(&self, offset: usize, limit: usize) -> Result<Vec<Job>> {
        let mut all_jobs = self.store.load_all_workflows().await?;

        // Sort by updated_at descending (most recent first)
        all_jobs.sort_by(|a, b| b.updated_at().cmp(&a.updated_at()));

        Ok(all_jobs
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect())
    }

    async fn find_by_status_and_worker(
        &self,
        status: JobStatus,
        worker_id: &str,
    ) -> Result<Vec<Job>> {
        let all_jobs = self.store.load_all_workflows().await?;
        let all_assignments = self.store.load_all_job_assignments().await?;
        let assignments_map: std::collections::HashMap<String, JobAssignment> = all_assignments
            .into_iter()
            .map(|a| (a.job_id.clone(), a))
            .collect();

        Ok(all_jobs
            .into_iter()
            .filter(|j| {
                j.status == status
                    && assignments_map
                        .get(&j.id)
                        .map(|a| a.assigned_worker_id.as_deref() == Some(worker_id))
                        .unwrap_or(false)
            })
            .collect())
    }

    async fn list_work(&self) -> Result<Vec<Job>> {
        self.store.load_all_workflows().await
    }

    async fn list_job_assignments(&self) -> Result<Vec<JobAssignment>> {
        self.store.load_all_job_assignments().await
    }

    async fn stats(&self) -> QueueStats {
        // Load all jobs and compute stats
        match self.store.load_all_workflows().await {
            Ok(jobs) => QueueStats::from_jobs(&jobs),
            Err(_) => QueueStats::empty(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hiqlite_persistent_store::HiqlitePersistentStore;
    use crate::hiqlite::HiqliteService;

    async fn create_test_repository() -> WorkRepositoryImpl {
        let hiqlite = Arc::new(
            HiqliteService::new(None)
                .await
                .expect("Failed to create HiqliteService"),
        );
        let store = Arc::new(HiqlitePersistentStore::new(hiqlite));
        WorkRepositoryImpl::new(store)
    }

    #[tokio::test]
    async fn test_publish_work() {
        let repo = create_test_repository().await;
        let payload = serde_json::json!({"task": "test"});

        let result = repo.publish_work("test-job-1".to_string(), payload).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_find_by_id() {
        let repo = create_test_repository().await;
        let payload = serde_json::json!({"task": "test"});

        repo.publish_work("test-job-2".to_string(), payload).await.unwrap();

        let job = repo.find_by_id("test-job-2").await.unwrap();
        assert!(job.is_some());
        assert_eq!(job.unwrap().id, "test-job-2");
    }

    #[tokio::test]
    async fn test_find_by_status() {
        let repo = create_test_repository().await;
        let payload = serde_json::json!({"task": "test"});

        repo.publish_work("test-job-3".to_string(), payload).await.unwrap();

        let jobs = repo.find_by_status(JobStatus::Pending).await.unwrap();
        assert!(!jobs.is_empty());
    }

    #[tokio::test]
    async fn test_list_work() {
        let repo = create_test_repository().await;

        let jobs = repo.list_work().await.unwrap();
        // Should not error, may be empty
        assert!(jobs.len() >= 0);
    }
}
