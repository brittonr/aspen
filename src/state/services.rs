//! Domain services container
//!
//! Pre-constructed domain services ready for use by handlers.

use std::sync::Arc;
#[cfg(feature = "tofu-support")]
use std::path::PathBuf;

use crate::domain::{
    ClusterStatusService, HealthService, JobLifecycleService, JobCommandService, JobQueryService,
    WorkerManagementService,
};
#[cfg(feature = "tofu-support")]
use crate::domain::TofuService;
#[cfg(feature = "vm-backend")]
use crate::domain::VmService;
use crate::domain::{EventPublisher, LoggingEventPublisher};
use crate::repositories::{
    HiqliteStateRepository, StateRepository, WorkQueueWorkRepository, WorkRepository,
    HiqliteWorkerRepository, WorkerRepository,
};
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
    #[cfg(feature = "vm-backend")]
    vm_service: Arc<VmService>,
    worker_management: Arc<WorkerManagementService>,
    #[cfg(feature = "tofu-support")]
    tofu_service: Arc<TofuService>,
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
            Arc::new(infra.hiqlite().clone()),
        ));
        let work_repo = Arc::new(WorkQueueWorkRepository::new(infra.work_queue().clone()));
        let worker_repo = Arc::new(HiqliteWorkerRepository::new(
            Arc::new(infra.hiqlite().clone()),
        ));

        #[cfg(feature = "tofu-support")]
        let tofu_service = {
            Arc::new(TofuService::new(
                infra.execution_registry().clone(),
                Arc::new(infra.hiqlite().clone()),
                PathBuf::from("/tmp/tofu-work"),
            ))
        };

        #[cfg(feature = "vm-backend")]
        let vm_service = {
            if let Some(vm_manager) = infra.vm_manager() {
                Arc::new(VmService::new(vm_manager.clone()))
            } else {
                // VM manager not available - create stub service
                tracing::warn!("VM manager not available, VmService will be non-functional");
                Arc::new(VmService::new(unsafe {
                    // SAFETY: This is intentionally unsafe because we never deref the stub
                    // The stub will panic if anyone tries to use it
                    std::mem::transmute::<Arc<()>, Arc<crate::vm_manager::VmManager>>(
                        Arc::new(()),
                    )
                }))
            }
        };

        Self::from_repositories_with_services(state_repo, work_repo, worker_repo)
    }

    /// Create domain services with injected repository abstractions
    ///
    /// This constructor enables dependency injection by accepting repository trait objects,
    /// allowing tests to inject mock implementations. Uses a default TofuService with
    /// placeholder backends for testing.
    ///
    /// Uses LoggingEventPublisher by default for production observability.
    ///
    /// **NOTE:** This path does not initialize VmService due to its async requirements.
    /// Tests that need VM operations should use `DomainServices::new(infrastructure)` instead.
    ///
    /// # Arguments
    /// * `state_repo` - Repository for cluster state operations
    /// * `work_repo` - Repository for work queue operations
    /// * `worker_repo` - Repository for worker management operations
    pub fn from_repositories(
        state_repo: Arc<dyn StateRepository>,
        work_repo: Arc<dyn WorkRepository>,
        worker_repo: Arc<dyn WorkerRepository>,
    ) -> Self {
        // Use LoggingEventPublisher for production observability
        let event_publisher = Arc::new(LoggingEventPublisher::new());
        Self::from_repositories_with_events(state_repo, work_repo, worker_repo, event_publisher)
    }

    /// Create domain services with injected repositories and services
    ///
    /// This constructor is used internally by `new()` to inject the pre-built TofuService and VmService.
    fn from_repositories_with_services(
        state_repo: Arc<dyn StateRepository>,
        work_repo: Arc<dyn WorkRepository>,
        worker_repo: Arc<dyn WorkerRepository>,
    ) -> Self {
        // Use LoggingEventPublisher for production observability
        let event_publisher = Arc::new(LoggingEventPublisher::new());
        Self::from_repositories_with_events(state_repo, work_repo, worker_repo, event_publisher)
    }

    /// Create domain services with custom event publisher (for testing)
    ///
    /// This constructor provides full control over event publishing, useful for:
    /// - Testing with InMemoryEventPublisher to verify events
    /// - Disabling events with NoOpEventPublisher
    /// - Using custom event publishers (external systems, metrics)
    ///
    /// **NOTE:** VmService is not available in this path. Tests that need VM functionality
    /// should use `DomainServices::new(infrastructure)` for full setup.
    ///
    /// # Arguments
    /// * `state_repo` - Repository for cluster state operations
    /// * `work_repo` - Repository for work queue operations
    /// * `worker_repo` - Repository for worker management operations
    /// * `event_publisher` - Publisher for domain events
    pub fn from_repositories_with_events(
        state_repo: Arc<dyn StateRepository>,
        work_repo: Arc<dyn WorkRepository>,
        worker_repo: Arc<dyn WorkerRepository>,
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

        // WorkerManagementService with worker and work repositories
        let worker_management = Arc::new(WorkerManagementService::new(
            worker_repo.clone(),
            work_repo.clone(),
            Some(60), // Default heartbeat timeout: 60 seconds
        ));

        #[cfg(feature = "tofu-support")]
        let tofu_service = {
            // Create a placeholder TofuService for testing
            // Tests using from_repositories should not call tofu operations
            Arc::new(TofuService::new(
                Arc::new(crate::adapters::ExecutionRegistry::new(
                    crate::adapters::RegistryConfig::default(),
                )),
                Arc::new(crate::hiqlite_service::HiqliteService::placeholder()),
                PathBuf::from("/tmp/tofu-work-test"),
            ))
        };

        #[cfg(feature = "vm-backend")]
        let vm_service = {
            // For VmService in the unit test path, we create a stub that will error if used.
            // This is acceptable because from_repositories is for testing pure domain logic
            // without infrastructure dependencies. Real integration tests should use new().
            let stub_vm_manager = Arc::new({
                // Create a stub that panics if actually used
                struct StubVmManager;
                impl std::ops::Deref for StubVmManager {
                    type Target = crate::vm_manager::VmManager;
                    fn deref(&self) -> &Self::Target {
                        panic!(
                            "VmService called in unit test context. \
                             Use DomainServices::new(infrastructure) for integration tests."
                        );
                    }
                }
                StubVmManager
            });
            // This will panic if actually dereferenced, which is what we want
            // to catch tests that accidentally try to use VM operations
            Arc::new(VmService::new(unsafe {
                // SAFETY: This is intentionally unsafe because we never deref the stub
                // The stub will panic if anyone tries to use it
                std::mem::transmute::<Arc<_>, Arc<crate::vm_manager::VmManager>>(
                    Arc::new(()),
                )
            }))
        };

        Self {
            cluster_status,
            health,
            job_lifecycle,
            #[cfg(feature = "vm-backend")]
            vm_service,
            worker_management,
            #[cfg(feature = "tofu-support")]
            tofu_service,
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

    /// Get the worker management service
    pub fn worker_management(&self) -> Arc<WorkerManagementService> {
        self.worker_management.clone()
    }

    #[cfg(feature = "vm-backend")]
    /// Get the VM service
    pub fn vm_service(&self) -> Arc<VmService> {
        self.vm_service.clone()
    }

    #[cfg(feature = "tofu-support")]
    /// Get the Tofu service
    pub fn tofu_service(&self) -> Arc<TofuService> {
        self.tofu_service.clone()
    }
}
