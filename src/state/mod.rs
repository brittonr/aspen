//! Application state management
//!
//! Flattened state architecture with minimal indirection.

use std::sync::Arc;
use flawless_utils::DeployedModule;

use crate::adapters::ExecutionRegistry;
use crate::domain::{
    ClusterStatusService, HealthService, JobLifecycleService, WorkerManagementService,
};
#[cfg(feature = "vm-backend")]
use crate::domain::VmService;
#[cfg(feature = "tofu-support")]
use crate::domain::TofuService;
use crate::hiqlite::HiqliteService;
use crate::iroh_service::IrohService;
use crate::work_queue::WorkQueue;

pub mod factory;

// Re-export factory types for convenience
pub use factory::{InfrastructureFactory, ProductionInfrastructureFactory};

/// Application state with flattened structure
///
/// All services are stored directly without nested state containers.
/// Services are wrapped in Arc for cheap cloning across request handlers.
#[derive(Clone)]
pub struct AppState {
    // === Infrastructure services ===
    module: Option<Arc<DeployedModule>>,
    iroh: IrohService,
    hiqlite: HiqliteService,
    work_queue: WorkQueue,
    execution_registry: Arc<ExecutionRegistry>,

    // === Domain services ===
    cluster_status: Arc<ClusterStatusService>,
    health: Arc<HealthService>,
    job_lifecycle: Arc<JobLifecycleService>,
    worker_management: Arc<WorkerManagementService>,

    #[cfg(feature = "vm-backend")]
    vm_service: Arc<VmService>,

    #[cfg(feature = "tofu-support")]
    tofu_service: Arc<TofuService>,
}

impl AppState {
    /// Create new application state from infrastructure and domain services
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        module: Option<DeployedModule>,
        iroh: IrohService,
        hiqlite: HiqliteService,
        work_queue: WorkQueue,
        execution_registry: Arc<ExecutionRegistry>,
        cluster_status: Arc<ClusterStatusService>,
        health: Arc<HealthService>,
        job_lifecycle: Arc<JobLifecycleService>,
        worker_management: Arc<WorkerManagementService>,
        #[cfg(feature = "vm-backend")]
        vm_service: Arc<VmService>,
        #[cfg(feature = "tofu-support")]
        tofu_service: Arc<TofuService>,
    ) -> Self {
        Self {
            module: module.map(Arc::new),
            iroh,
            hiqlite,
            work_queue,
            execution_registry,
            cluster_status,
            health,
            job_lifecycle,
            worker_management,
            #[cfg(feature = "vm-backend")]
            vm_service,
            #[cfg(feature = "tofu-support")]
            tofu_service,
        }
    }

    // === Infrastructure accessors ===

    /// Get the Flawless module (if available)
    pub fn module(&self) -> Option<&DeployedModule> {
        self.module.as_ref().map(|m| m.as_ref())
    }

    /// Get the Iroh service
    pub fn iroh(&self) -> &IrohService {
        &self.iroh
    }

    /// Get the Hiqlite service
    pub fn hiqlite(&self) -> &HiqliteService {
        &self.hiqlite
    }

    /// Get the work queue
    pub fn work_queue(&self) -> &WorkQueue {
        &self.work_queue
    }

    /// Get the execution registry
    pub fn execution_registry(&self) -> &Arc<ExecutionRegistry> {
        &self.execution_registry
    }

    // === Domain service accessors ===

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
