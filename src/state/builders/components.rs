// Component structs returned by each builder
//
// These are data containers that group related components together,
// making the dependency flow explicit and reducing parameter passing complexity.

use std::sync::Arc;

use crate::adapters::ExecutionRegistry;
use crate::domain::{
    ClusterStatusService, HealthService, JobCommandService, JobQueryService,
    WorkerManagementService,
};
use crate::hiqlite::HiqliteService;
use crate::infrastructure::vm::{VmManagement, VmManager};
use crate::iroh_service::IrohService;
use crate::repositories::{StateRepository, WorkRepository, WorkerRepository};

#[cfg(feature = "vm-backend")]
use crate::domain::VmService;
#[cfg(feature = "tofu-support")]
use crate::domain::TofuService;

/// Infrastructure components (core platform services)
pub struct InfrastructureComponents {
    pub hiqlite: HiqliteService,
    pub iroh: IrohService,
    pub node_id: String,
    pub vm_manager: Option<Arc<VmManager>>,
    pub execution_registry: Arc<ExecutionRegistry>,
}

/// Repository components (data access layer)
pub struct RepositoryComponents {
    pub state: Arc<dyn StateRepository>,
    pub work: Arc<dyn WorkRepository>,
    pub worker: Arc<dyn WorkerRepository>,
}

/// Domain service components (business logic layer)
pub struct DomainServiceComponents {
    pub cluster_status: Arc<ClusterStatusService>,
    pub health: Arc<HealthService>,
    pub job_commands: Arc<JobCommandService>,
    pub job_queries: Arc<JobQueryService>,
    pub worker_management: Arc<WorkerManagementService>,
}

/// Optional feature components
pub struct FeatureComponents {
    #[cfg(feature = "vm-backend")]
    pub vm_service: Option<Arc<VmService>>,
    #[cfg(feature = "tofu-support")]
    pub tofu_service: Option<Arc<TofuService>>,
}
