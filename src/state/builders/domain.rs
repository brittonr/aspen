// Domain Service Builder - Creates business logic layer
//
// Responsibilities:
// - Create all domain services with proper dependency injection
// - Wire up event publishing
// - Configure service parameters (heartbeat timeout, etc.)
//
// All services are independent and can be built in parallel.

use std::sync::Arc;

use crate::domain::{
    ClusterStatusService, HealthService, JobCommandService, JobQueryService,
    LoggingEventPublisher, WorkerManagementService,
};
use crate::repositories::{StateRepository, WorkRepository, WorkerRepository};

use super::components::DomainServiceComponents;

pub struct DomainServiceBuilder {
    state_repo: Arc<dyn StateRepository>,
    work_repo: Arc<dyn WorkRepository>,
    worker_repo: Arc<dyn WorkerRepository>,
    heartbeat_timeout_secs: i64,
}

impl DomainServiceBuilder {
    pub fn new(
        state_repo: Arc<dyn StateRepository>,
        work_repo: Arc<dyn WorkRepository>,
        worker_repo: Arc<dyn WorkerRepository>,
    ) -> Self {
        Self {
            state_repo,
            work_repo,
            worker_repo,
            heartbeat_timeout_secs: 60, // Default: 60 seconds
        }
    }

    pub fn with_heartbeat_timeout(mut self, secs: i64) -> Self {
        self.heartbeat_timeout_secs = secs;
        self
    }

    /// Build all domain service components
    pub fn build(self) -> DomainServiceComponents {
        // Create event publisher for domain events
        let event_publisher = Arc::new(LoggingEventPublisher::new());

        // Create CQRS services
        let job_commands = Arc::new(JobCommandService::with_events(
            self.work_repo.clone(),
            event_publisher,
        ));
        let job_queries = Arc::new(JobQueryService::new(self.work_repo.clone()));

        // Create cluster and health services
        let cluster_status = Arc::new(ClusterStatusService::new(
            self.state_repo.clone(),
            self.work_repo.clone(),
        ));
        let health = Arc::new(HealthService::new(self.state_repo));

        // Create worker management service
        let worker_management = Arc::new(WorkerManagementService::new(
            self.worker_repo,
            self.work_repo,
            Some(self.heartbeat_timeout_secs),
        ));

        DomainServiceComponents {
            cluster_status,
            health,
            job_commands,
            job_queries,
            worker_management,
        }
    }
}
