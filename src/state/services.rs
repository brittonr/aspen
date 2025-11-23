//! Domain services container
//!
//! Pre-constructed domain services ready for use by handlers.

use std::sync::Arc;

use crate::domain::{ClusterStatusService, HealthService, JobLifecycleService, JobCommandService, JobQueryService};
use crate::domain::{EventPublisher, LoggingEventPublisher};
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
    /// Uses LoggingEventPublisher by default for production observability.
    ///
    /// # Arguments
    /// * `state_repo` - Repository for cluster state operations
    /// * `work_repo` - Repository for work queue operations
    pub fn from_repositories(
        state_repo: Arc<dyn StateRepository>,
        work_repo: Arc<dyn WorkRepository>,
    ) -> Self {
        // Use LoggingEventPublisher for production observability
        let event_publisher = Arc::new(LoggingEventPublisher::new());
        Self::from_repositories_with_events(state_repo, work_repo, event_publisher)
    }

    /// Create domain services with custom event publisher (for testing)
    ///
    /// This constructor provides full control over event publishing, useful for:
    /// - Testing with InMemoryEventPublisher to verify events
    /// - Disabling events with NoOpEventPublisher
    /// - Using custom event publishers (external systems, metrics)
    ///
    /// # Arguments
    /// * `state_repo` - Repository for cluster state operations
    /// * `work_repo` - Repository for work queue operations
    /// * `event_publisher` - Publisher for domain events
    pub fn from_repositories_with_events(
        state_repo: Arc<dyn StateRepository>,
        work_repo: Arc<dyn WorkRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        // Create command and query services (CQRS pattern)
        let commands = Arc::new(JobCommandService::with_events(
            work_repo.clone(),
            event_publisher.clone(),
        ));
        let queries = Arc::new(JobQueryService::new(work_repo.clone()));

        // Create domain services with injected repositories
        let cluster_status = Arc::new(ClusterStatusService::new(
            state_repo.clone(),
            work_repo.clone(),
        ));

        let health = Arc::new(HealthService::new(state_repo.clone()));

        // JobLifecycleService wraps command and query services
        let job_lifecycle = Arc::new(JobLifecycleService::from_services(commands, queries));

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
