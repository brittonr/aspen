use std::sync::Arc;

use crate::domain::{
    ClusterStatusService, HealthService, JobLifecycleService, WorkerManagementService,
};

/// Domain services state container
///
/// This is the primary state container for HTTP handlers. It provides
/// access to all business logic services without exposing infrastructure.
///
/// # Usage in handlers
///
/// ```rust
/// pub async fn my_handler(
///     State(domain): State<DomainState>,
/// ) -> impl IntoResponse {
///     let jobs = domain.job_lifecycle().list_jobs(...).await?;
///     // ...
/// }
/// ```
#[derive(Clone)]
pub struct DomainState {
    cluster_status: Arc<ClusterStatusService>,
    health: Arc<HealthService>,
    job_lifecycle: Arc<JobLifecycleService>,
    worker_management: Arc<WorkerManagementService>,
}

impl DomainState {
    /// Create new domain state
    pub fn new(
        cluster_status: Arc<ClusterStatusService>,
        health: Arc<HealthService>,
        job_lifecycle: Arc<JobLifecycleService>,
        worker_management: Arc<WorkerManagementService>,
    ) -> Self {
        Self {
            cluster_status,
            health,
            job_lifecycle,
            worker_management,
        }
    }

    /// Get cluster status service
    pub fn cluster_status(&self) -> Arc<ClusterStatusService> {
        self.cluster_status.clone()
    }

    /// Get health service
    pub fn health(&self) -> Arc<HealthService> {
        self.health.clone()
    }

    /// Get job lifecycle service
    pub fn job_lifecycle(&self) -> Arc<JobLifecycleService> {
        self.job_lifecycle.clone()
    }

    /// Get worker management service
    pub fn worker_management(&self) -> Arc<WorkerManagementService> {
        self.worker_management.clone()
    }
}
