//! Application state management
//!
//! Refactored to use focused state containers instead of a monolithic AppState.
//!
//! # State Containers
//!
//! - `DomainState` - Core business logic services (most handlers use this)
//! - `InfraState` - Low-level infrastructure (Iroh, Hiqlite, WorkQueue)
//! - `ConfigState` - Static configuration (AuthConfig, DeployedModule)
//! - `FeaturesState` - Optional feature-gated services (VM, Tofu)
//!
//! # Migration Status
//!
//! AppState is deprecated but kept temporarily for backward compatibility.
//! New code should use focused state containers.

use std::sync::Arc;
use flawless_utils::DeployedModule;

use crate::adapters::ExecutionRegistry;
use crate::config::AuthConfig;
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

// === New focused state containers ===
pub mod config;
pub mod domain;
pub mod features;
pub mod infra;

// Re-export new state containers for convenience
pub use self::config::ConfigState;
pub use self::domain::DomainState;
pub use self::features::FeaturesState;
pub use self::infra::InfraState;

pub mod factory;

// Re-export factory types for convenience
pub use factory::{InfrastructureFactory, ProductionInfrastructureFactory, StateBuilder};

/// Application state with flattened structure
///
/// All services are stored directly without nested state containers.
/// Services are wrapped in Arc for cheap cloning across request handlers.
#[derive(Clone)]
pub struct AppState {
    // === Configuration ===
    auth_config: Arc<AuthConfig>,

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
        auth_config: Arc<AuthConfig>,
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
            auth_config,
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

    // === Configuration accessors ===

    /// Get the authentication configuration
    pub fn auth_config(&self) -> Arc<AuthConfig> {
        self.auth_config.clone()
    }

    // === Infrastructure accessors ===
    //
    // NOTE: Infrastructure services should NOT be accessed by regular HTTP handlers.
    // Handlers should use domain services instead. Infrastructure accessors are
    // marked pub(crate) to enforce this boundary.
    //
    // Exception: iroh() is pub for infrastructure-level P2P API handlers in iroh_api.rs

    /// Get the Flawless module (if available)
    ///
    /// Internal use only - for startup/display purposes
    pub(crate) fn module(&self) -> Option<&DeployedModule> {
        self.module.as_ref().map(|m| m.as_ref())
    }

    /// Get the Iroh service
    ///
    /// NOTE: This is public for infrastructure-level P2P API handlers (iroh_api.rs).
    /// Regular handlers should use domain services, not infrastructure directly.
    pub fn iroh(&self) -> &IrohService {
        &self.iroh
    }

    /// Get the Hiqlite service
    ///
    /// Internal use only - handlers should use domain services
    pub(crate) fn hiqlite(&self) -> &HiqliteService {
        &self.hiqlite
    }

    /// Get the work queue
    ///
    /// Internal use only - for startup/display purposes
    pub(crate) fn work_queue(&self) -> &WorkQueue {
        &self.work_queue
    }

    /// Get the work queue connection ticket for worker registration
    ///
    /// This is exposed for startup/display purposes without exposing the full WorkQueue
    pub fn get_work_queue_ticket(&self) -> String {
        self.work_queue.get_ticket()
    }

    /// Get the execution registry
    ///
    /// Internal use only - handlers should use domain services
    pub(crate) fn execution_registry(&self) -> &Arc<ExecutionRegistry> {
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
