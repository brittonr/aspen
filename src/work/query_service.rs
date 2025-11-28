//! Work Query Service - Domain Layer
//!
//! This module contains the business logic for read operations on the work queue.
//! It provides various query methods without any caching logic.
//!
//! Responsibilities:
//! - List/find operations
//! - Filtering by status, worker, etc.
//! - Pagination
//! - Statistics computation
//! - NO caching (caching is done by CachedWorkQueryService decorator)
//!
//! This is the Domain Service layer in clean architecture.

use anyhow::Result;
use std::sync::Arc;

use crate::repositories::{JobAssignment, WorkRepository};
use crate::domain::types::{Job, JobStatus, QueueStats};

/// Domain service for work query operations (reads)
///
/// This service provides all read operations on the work queue.
/// It delegates to the repository layer and contains no caching logic.
pub struct WorkQueryService {
    repository: Arc<dyn WorkRepository>,
}

impl WorkQueryService {
    /// Create a new WorkQueryService
    pub fn new(repository: Arc<dyn WorkRepository>) -> Self {
        Self { repository }
    }

    /// List all work items
    pub async fn list_work(&self) -> Result<Vec<Job>> {
        self.repository.list_work().await
    }

    /// List all job assignments
    pub async fn list_job_assignments(&self) -> Result<Vec<JobAssignment>> {
        self.repository.list_job_assignments().await
    }

    /// Get a specific work item by ID
    pub async fn get_work_by_id(&self, job_id: &str) -> Result<Option<Job>> {
        self.repository.find_by_id(job_id).await
    }

    /// List work items filtered by status
    pub async fn list_work_by_status(&self, status: JobStatus) -> Result<Vec<Job>> {
        self.repository.find_by_status(status).await
    }

    /// List work items claimed by a specific worker
    pub async fn list_work_by_worker(&self, worker_id: &str) -> Result<Vec<Job>> {
        self.repository.find_by_worker(worker_id).await
    }

    /// List work items with pagination
    pub async fn list_work_paginated(&self, offset: usize, limit: usize) -> Result<Vec<Job>> {
        self.repository.find_paginated(offset, limit).await
    }

    /// List work items filtered by status AND worker (composite query)
    pub async fn list_work_by_status_and_worker(
        &self,
        status: JobStatus,
        worker_id: &str,
    ) -> Result<Vec<Job>> {
        self.repository
            .find_by_status_and_worker(status, worker_id)
            .await
    }

    /// Get work queue statistics
    pub async fn stats(&self) -> QueueStats {
        self.repository.stats().await
    }
}

impl Clone for WorkQueryService {
    fn clone(&self) -> Self {
        Self {
            repository: Arc::clone(&self.repository),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work::repository::WorkRepositoryImpl;
    use crate::hiqlite_persistent_store::HiqlitePersistentStore;
    use crate::hiqlite::HiqliteService;

    async fn create_test_service() -> WorkQueryService {
        let hiqlite = Arc::new(
            HiqliteService::new(None)
                .await
                .expect("Failed to create HiqliteService"),
        );
        let store = Arc::new(HiqlitePersistentStore::new(hiqlite));
        let repository = Arc::new(WorkRepositoryImpl::new(store));
        WorkQueryService::new(repository)
    }

    #[tokio::test]
    async fn test_list_work() {
        let service = create_test_service().await;

        let result = service.list_work().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_work_by_id_not_found() {
        let service = create_test_service().await;

        let result = service.get_work_by_id("nonexistent").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_list_work_by_status() {
        let service = create_test_service().await;

        let result = service.list_work_by_status(JobStatus::Pending).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stats() {
        let service = create_test_service().await;

        let stats = service.stats().await;
        // Should return valid stats (may be all zeros)
        assert!(stats.total >= 0);
    }

    #[tokio::test]
    async fn test_pagination() {
        let service = create_test_service().await;

        let result = service.list_work_paginated(0, 10).await;
        assert!(result.is_ok());
        let jobs = result.unwrap();
        assert!(jobs.len() <= 10);
    }
}
