//! Domain services container
//!
//! Pre-constructed domain services ready for use by handlers.

use std::sync::Arc;

use crate::domain::{ClusterStatusService, JobLifecycleService};
use crate::repositories::{HiqliteStateRepository, WorkQueueWorkRepository};
use crate::state::InfrastructureState;

/// Container for pre-constructed domain services
///
/// Domain services are created once at startup (not per-request) and injected
/// with repository dependencies, following the dependency injection pattern.
#[derive(Clone)]
pub struct DomainServices {
    cluster_status: Arc<ClusterStatusService>,
    job_lifecycle: Arc<JobLifecycleService>,
}

impl DomainServices {
    /// Create domain services with repository dependencies
    pub fn new(infra: &InfrastructureState) -> Self {
        // Create repository implementations
        let state_repo = Arc::new(HiqliteStateRepository::new(
            Arc::new(infra.hiqlite().clone())
        ));
        let work_repo = Arc::new(WorkQueueWorkRepository::new(infra.work_queue().clone()));

        // Create domain services with injected repositories
        let cluster_status = Arc::new(ClusterStatusService::new(
            state_repo.clone(),
            work_repo.clone(),
        ));

        let job_lifecycle = Arc::new(JobLifecycleService::new(work_repo.clone()));

        Self {
            cluster_status,
            job_lifecycle,
        }
    }

    /// Get the cluster status service
    pub fn cluster_status(&self) -> Arc<ClusterStatusService> {
        self.cluster_status.clone()
    }

    /// Get the job lifecycle service
    pub fn job_lifecycle(&self) -> Arc<JobLifecycleService> {
        self.job_lifecycle.clone()
    }
}
