//! Domain services container
//!
//! Pre-constructed domain services ready for use by handlers.

use std::sync::Arc;

use crate::domain::{ClusterStatusService, HealthService, JobLifecycleService};
use crate::repositories::{HiqliteStateRepository, StateRepository, WorkQueueWorkRepository, WorkRepository};
use crate::state::InfrastructureState;

/// Container for pre-constructed domain services
///
/// Domain services are created once at startup (not per-request) and injected
/// with repository dependencies, following the dependency injection pattern.
#[derive(Clone)]
pub struct DomainServices {
    cluster_status: Arc<ClusterStatusService>,
    health: Arc<HealthService>,
    job_lifecycle: Arc<JobLifecycleService>,
}

impl DomainServices {
    /// Create domain services with repository dependencies from infrastructure
    ///
    /// This is a convenience constructor that creates concrete repository implementations
    /// from the infrastructure state. For testing with mock repositories, use
    /// `from_repositories` instead.
    pub fn new(infra: &InfrastructureState) -> Self {
        // Create repository implementations
        let state_repo = Arc::new(HiqliteStateRepository::new(
            Arc::new(infra.hiqlite().clone())
        ));
        let work_repo = Arc::new(WorkQueueWorkRepository::new(infra.work_queue().clone()));

        Self::from_repositories(state_repo, work_repo)
    }

    /// Create domain services with injected repository abstractions
    ///
    /// This constructor enables dependency injection by accepting repository trait objects,
    /// allowing tests to inject mock implementations.
    ///
    /// # Arguments
    /// * `state_repo` - Repository for cluster state operations
    /// * `work_repo` - Repository for work queue operations
    pub fn from_repositories(
        state_repo: Arc<dyn StateRepository>,
        work_repo: Arc<dyn WorkRepository>,
    ) -> Self {
        // Create domain services with injected repositories
        let cluster_status = Arc::new(ClusterStatusService::new(
            state_repo.clone(),
            work_repo.clone(),
        ));

        let health = Arc::new(HealthService::new(state_repo.clone()));

        let job_lifecycle = Arc::new(JobLifecycleService::new(work_repo.clone()));

        Self {
            cluster_status,
            health,
            job_lifecycle,
        }
    }

    /// Get the cluster status service
    pub fn cluster_status(&self) -> Arc<ClusterStatusService> {
        self.cluster_status.clone()
    }

    /// Get the health service
    pub fn health(&self) -> Arc<HealthService> {
        self.health.clone()
    }

    /// Get the job lifecycle service
    pub fn job_lifecycle(&self) -> Arc<JobLifecycleService> {
        self.job_lifecycle.clone()
    }
}
