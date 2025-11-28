//! Work Command Service - Domain Layer
//!
//! This module contains the business logic for write operations on the work queue.
//! It coordinates between the repository layer and domain services.
//!
//! Responsibilities:
//! - Publish work (business logic for creating new jobs)
//! - Claim work (business logic for claiming jobs with worker type compatibility)
//! - Update status (business logic for state transitions)
//! - Emit domain events for cache invalidation
//!
//! This is the Domain Service layer in clean architecture.

use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, info, warn};

use crate::repositories::WorkRepository;
use crate::domain::types::{Job, JobStatus, WorkerType};
use crate::domain::job_claiming::JobClaimingService;
use crate::work_state_machine::WorkStateMachine;
use crate::persistent_store::PersistentStore;

/// Version counter for cache invalidation
#[derive(Debug, Clone)]
pub struct CacheVersion {
    version: Arc<AtomicU64>,
}

impl CacheVersion {
    pub fn new() -> Self {
        Self {
            version: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn increment(&self) -> u64 {
        self.version.fetch_add(1, Ordering::SeqCst)
    }

    pub fn get(&self) -> u64 {
        self.version.load(Ordering::SeqCst)
    }
}

impl Default for CacheVersion {
    fn default() -> Self {
        Self::new()
    }
}

/// Domain service for work command operations (writes)
///
/// This service contains all business logic for modifying work queue state.
/// It uses the repository for persistence and emits cache invalidation events.
pub struct WorkCommandService {
    /// Node identifier for claiming jobs
    node_id: String,
    /// Persistent store for atomic operations
    store: Arc<dyn PersistentStore>,
    /// Cache version for invalidation
    cache_version: CacheVersion,
}

impl WorkCommandService {
    /// Create a new WorkCommandService
    pub fn new(
        node_id: String,
        store: Arc<dyn PersistentStore>,
    ) -> Self {
        Self {
            node_id,
            store,
            cache_version: CacheVersion::new(),
        }
    }

    /// Publish a new work item to the queue
    ///
    /// Business logic:
    /// - Create a Job with Pending status
    /// - Set default requirements
    /// - Persist to store
    /// - Emit cache invalidation event
    pub async fn publish_work(&self, job_id: String, payload: serde_json::Value) -> Result<()> {
        use crate::domain::job_metadata::JobMetadata;
        use crate::domain::job_requirements::JobRequirements;

        let job = Job {
            id: job_id.clone(),
            status: JobStatus::Pending,
            payload: payload.clone(),
            requirements: JobRequirements::default(),
            metadata: JobMetadata::new(),
            error_message: None,
            claimed_by: None,
            assigned_worker_id: None,
            completed_by: None,
        };

        // Persist to store
        self.store.upsert_workflow(&job).await?;

        // Invalidate cache
        self.cache_version.increment();

        info!(job_id = %job_id, "Work item published");
        Ok(())
    }

    /// Claim available work from the queue
    ///
    /// Business logic:
    /// - Load pending jobs
    /// - Filter by worker type compatibility (using JobClaimingService)
    /// - Atomically claim first compatible job
    /// - Emit cache invalidation event on success
    pub async fn claim_work(
        &self,
        worker_id: Option<&str>,
        worker_type: Option<WorkerType>,
    ) -> Result<Option<Job>> {
        // Load all pending jobs
        let all_jobs = self.store.load_all_workflows().await?;
        let pending_jobs: Vec<Job> = all_jobs
            .into_iter()
            .filter(|j| j.status == JobStatus::Pending)
            .collect();

        info!(
            pending = pending_jobs.len(),
            node_id = %self.node_id,
            worker_type = ?worker_type,
            "Searching for work to claim"
        );

        // Try to claim each compatible job
        for job in &pending_jobs {
            // Business logic: Check compatibility using domain service
            if !JobClaimingService::is_compatible_with_worker_type(job, worker_type) {
                debug!(
                    job_id = %job.id,
                    worker_type = ?worker_type,
                    compatible_types = ?job.requirements.compatible_worker_types,
                    "Skipping incompatible job"
                );
                continue;
            }

            // Try to atomically claim this job
            match self.try_claim_job(job, worker_id).await {
                Ok(Some(claimed_job)) => {
                    // Success! Invalidate cache and return
                    self.cache_version.increment();
                    return Ok(Some(claimed_job));
                }
                Ok(None) => continue, // Lost race, try next job
                Err(e) => {
                    warn!(
                        job_id = %job.id,
                        error = %e,
                        "Error claiming job, continuing with next"
                    );
                    continue;
                }
            }
        }

        info!(
            pending_count = pending_jobs.len(),
            worker_type = ?worker_type,
            "No work claimed - all pending jobs incompatible or claimed by others"
        );
        Ok(None)
    }

    /// Try to atomically claim a single job
    async fn try_claim_job(
        &self,
        job: &Job,
        worker_id: Option<&str>,
    ) -> Result<Option<Job>> {
        let now = chrono::Utc::now().timestamp();

        debug!(
            job_id = %job.id,
            node_id = %self.node_id,
            worker_id = ?worker_id,
            "Attempting to claim job"
        );

        // Atomically claim the job in the store
        let rows_affected = self.store
            .claim_workflow(&job.id, &self.node_id, worker_id, now)
            .await?;

        if rows_affected == 0 {
            // Another node claimed it first
            debug!(job_id = %job.id, "Claim race lost - job already claimed");
            return Ok(None);
        }

        // Construct the claimed job
        let mut claimed_job = job.clone();
        claimed_job.status = JobStatus::Claimed;
        claimed_job.claimed_by = Some(self.node_id.clone());
        claimed_job.assigned_worker_id = worker_id.map(|s| s.to_string());
        claimed_job.metadata.updated_at = now;

        info!(
            job_id = %claimed_job.id,
            node_id = %self.node_id,
            worker_id = ?worker_id,
            "Job claimed successfully"
        );

        Ok(Some(claimed_job))
    }

    /// Update work item status
    ///
    /// Business logic:
    /// - Validate state transition (using WorkStateMachine)
    /// - Set completed_by for terminal states
    /// - Persist to store
    /// - Emit cache invalidation event
    pub async fn update_status(&self, job_id: &str, status: JobStatus) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        // Business logic: Determine completed_by using state machine
        let completed_by = if WorkStateMachine::requires_completed_by(&status) {
            Some(self.node_id.as_str())
        } else {
            None
        };

        // Update in store with state machine guards
        let rows_affected = self.store
            .update_workflow_status(job_id, &status, completed_by, now)
            .await?;

        if rows_affected == 0 {
            debug!(
                job_id = %job_id,
                new_status = ?status,
                "Status update rejected by state machine guards or job not found"
            );
            return Ok(());
        }

        // Invalidate cache
        self.cache_version.increment();

        info!(
            job_id = %job_id,
            status = ?status,
            node_id = %self.node_id,
            "Work status updated successfully"
        );

        Ok(())
    }

    /// Get the current cache version (for invalidation coordination)
    pub fn get_cache_version(&self) -> u64 {
        self.cache_version.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hiqlite_persistent_store::HiqlitePersistentStore;
    use crate::hiqlite::HiqliteService;

    async fn create_test_service() -> WorkCommandService {
        let hiqlite = Arc::new(
            HiqliteService::new(None)
                .await
                .expect("Failed to create HiqliteService"),
        );
        let store = Arc::new(HiqlitePersistentStore::new(hiqlite));
        WorkCommandService::new("test-node".to_string(), store)
    }

    #[tokio::test]
    async fn test_publish_work() {
        let service = create_test_service().await;
        let payload = serde_json::json!({"task": "test"});

        let result = service.publish_work("test-job-1".to_string(), payload).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_claim_work() {
        let service = create_test_service().await;
        let payload = serde_json::json!({"task": "test"});

        // Publish a job first
        service.publish_work("test-job-2".to_string(), payload).await.unwrap();

        // Claim it
        let result = service.claim_work(Some("worker-1"), None).await;
        assert!(result.is_ok());
        let job = result.unwrap();
        assert!(job.is_some());

        // Try to claim again - should get None (already claimed)
        let result2 = service.claim_work(Some("worker-2"), None).await;
        assert!(result2.is_ok());
        assert!(result2.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_update_status() {
        let service = create_test_service().await;
        let payload = serde_json::json!({"task": "test"});

        // Publish and claim a job
        service.publish_work("test-job-3".to_string(), payload).await.unwrap();
        let job = service.claim_work(Some("worker-1"), None).await.unwrap().unwrap();

        // Update to InProgress
        let result = service.update_status(&job.id, JobStatus::InProgress).await;
        assert!(result.is_ok());

        // Update to Completed
        let result = service.update_status(&job.id, JobStatus::Completed).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cache_version_increments() {
        let service = create_test_service().await;
        let payload = serde_json::json!({"task": "test"});

        let v1 = service.get_cache_version();

        service.publish_work("test-job-4".to_string(), payload).await.unwrap();
        let v2 = service.get_cache_version();
        assert!(v2 > v1);

        service.claim_work(Some("worker-1"), None).await.unwrap();
        let v3 = service.get_cache_version();
        assert!(v3 > v2);
    }
}
