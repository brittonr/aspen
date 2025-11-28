// VmCoordinator - Orchestrator for VM management components
//
// The coordinator is responsible for:
// - Creating and owning VM management components
// - Managing component lifecycle (startup/shutdown)
// - Coordinating background tasks (health checks, monitoring)
// - Providing controlled access to components via the VmManager facade
//
// Architecture: Components are accessed directly (not via message passing).
// The coordinator owns components and provides Arc references to them.

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

use super::health_checker::{HealthChecker, HealthCheckConfig};
use super::job_router::JobRouter;
use super::resource_monitor::ResourceMonitor;
use super::supervisor::{
    supervised_consistency_checker, supervised_health_checker, supervised_resource_monitor,
    SupervisionStrategy, TaskSupervisor,
};
use super::traits::{VmCommandExecutor, VmQueryService, VmState, VmType, VmAssignment, VmInfo, ResourceUsage};
use super::vm_controller::VmController;
use super::vm_registry::VmRegistry;
use super::VmManagerConfig;
use crate::domain::types::Job;
use crate::hiqlite::HiqliteService;
use async_trait::async_trait;
use uuid::Uuid;

/// VmCoordinator orchestrates all VM management components
pub struct VmCoordinator {
    // Component instances (owned by coordinator)
    registry: Arc<VmRegistry>,
    controller: Arc<VmController>,
    router: Arc<JobRouter>,
    monitor: Arc<ResourceMonitor>,
    health_checker: Arc<HealthChecker>,

    // Task supervisor for critical background tasks
    supervisor: Arc<tokio::sync::Mutex<TaskSupervisor>>,

    // Background task handles for graceful shutdown (non-critical tasks)
    task_handles: Arc<tokio::sync::Mutex<Vec<JoinHandle<()>>>>,

    config: VmManagerConfig,
}

impl VmCoordinator {
    /// Create a new VmCoordinator with all components
    pub async fn new(config: VmManagerConfig, hiqlite: Arc<HiqliteService>) -> Result<Self> {
        // Initialize registry with Hiqlite persistence
        let registry = Arc::new(VmRegistry::new(hiqlite, &config.state_dir).await?);

        // Create controller for VM lifecycle operations
        let controller = Arc::new(VmController::new(config.clone(), Arc::clone(&registry))?);

        // Initialize job router
        let router = Arc::new(JobRouter::new(
            Arc::clone(&registry),
            Arc::clone(&controller),
        ));

        // Set up resource monitor
        let monitor = Arc::new(ResourceMonitor::new(
            Arc::clone(&registry),
            Arc::clone(&controller),
        ));

        // Initialize health checker
        let health_config = HealthCheckConfig::default();
        let health_checker = Arc::new(HealthChecker::new(Arc::clone(&registry), health_config));

        Ok(Self {
            registry,
            controller,
            router,
            monitor,
            health_checker,
            supervisor: Arc::new(tokio::sync::Mutex::new(TaskSupervisor::new())),
            task_handles: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            config,
        })
    }

    /// Start all background tasks
    pub async fn start_background_tasks(&self) -> Result<()> {
        tracing::info!("Starting VmCoordinator background tasks");

        // Set up supervised critical tasks
        let mut supervisor = self.supervisor.lock().await;

        // Supervise health checker with restart on failure
        supervisor.supervise(
            "vm_health_checker".to_string(),
            SupervisionStrategy::RestartWithBackoff {
                initial_delay: Duration::from_secs(1),
                max_delay: Duration::from_secs(60),
                factor: 2.0,
            },
            supervised_health_checker(Arc::clone(&self.health_checker)),
        );

        // Supervise resource monitor with restart on failure
        supervisor.supervise(
            "vm_resource_monitor".to_string(),
            SupervisionStrategy::RestartWithBackoff {
                initial_delay: Duration::from_secs(1),
                max_delay: Duration::from_secs(60),
                factor: 2.0,
            },
            supervised_resource_monitor(Arc::clone(&self.monitor)),
        );

        // Supervise consistency checker (runs every 5 minutes)
        supervisor.supervise(
            "vm_consistency_checker".to_string(),
            SupervisionStrategy::RestartAlways,
            supervised_consistency_checker(
                Arc::clone(&self.registry),
                Duration::from_secs(300), // Check every 5 minutes
            ),
        );

        tracing::info!("Started 3 supervised critical tasks");

        // Drop supervisor lock before continuing
        drop(supervisor);

        let mut handles = self.task_handles.lock().await;

        // Pre-warm VMs if configured
        if self.config.auto_scaling && self.config.pre_warm_count > 0 {
            self.pre_warm_vms().await?;
        }

        // Recover any VMs from previous run
        self.recover_existing_vms().await?;

        tracing::info!("VmCoordinator background tasks started");
        Ok(())
    }

    /// Pre-warm idle VMs for faster job execution
    async fn pre_warm_vms(&self) -> Result<()> {
        tracing::info!(count = self.config.pre_warm_count, "Pre-warming service VMs");

        for i in 0..self.config.pre_warm_count {
            let config = super::vm_types::VmConfig::default_service();

            match self.controller.start_vm(config).await {
                Ok(vm) => {
                    tracing::info!(
                        vm_id = %vm.config.id,
                        index = i + 1,
                        "Pre-warmed service VM started"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        index = i + 1,
                        "Failed to pre-warm service VM"
                    );
                }
            }
        }

        Ok(())
    }

    /// Recover VMs from previous run (on restart)
    async fn recover_existing_vms(&self) -> Result<()> {
        let recovered = self.registry.recover_from_persistence().await?;

        if recovered > 0 {
            tracing::info!(count = recovered, "Recovered VMs from previous run");

            // Mark stale VMs as terminated
            self.registry.cleanup_stale_vms().await?;
        }

        Ok(())
    }

    /// Gracefully shutdown the coordinator and all components
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down VmCoordinator");

        // Stop accepting new jobs
        self.router.stop_routing().await;

        // Stop monitoring and health checking via supervisor
        let mut supervisor = self.supervisor.lock().await;
        supervisor.shutdown().await?;
        drop(supervisor);

        // Gracefully shutdown all VMs
        let all_vms = self.registry.list_all_vms().await?;
        for vm in all_vms {
            if !matches!(vm.state, super::vm_types::VmState::Terminated { .. }) {
                self.controller.shutdown_vm(vm.config.id, true).await?;
            }
        }

        // Cancel any remaining non-supervised background tasks
        let mut handles = self.task_handles.lock().await;
        for handle in handles.drain(..) {
            handle.abort();
        }

        tracing::info!("VmCoordinator shutdown complete");
        Ok(())
    }

    /// Get reference to VM registry
    pub fn registry(&self) -> Arc<VmRegistry> {
        Arc::clone(&self.registry)
    }

    /// Get reference to VM controller
    pub fn controller(&self) -> Arc<VmController> {
        Arc::clone(&self.controller)
    }

    /// Get reference to job router
    pub fn router(&self) -> Arc<JobRouter> {
        Arc::clone(&self.router)
    }

    /// Get reference to resource monitor
    pub fn monitor(&self) -> Arc<ResourceMonitor> {
        Arc::clone(&self.monitor)
    }

    /// Get reference to health checker
    pub fn health_checker(&self) -> Arc<HealthChecker> {
        Arc::clone(&self.health_checker)
    }

    /// Get config reference
    pub fn config(&self) -> &VmManagerConfig {
        &self.config
    }
}

/// Implementation of VmCommandExecutor trait - single point of authority for VM state changes
#[async_trait]
impl VmCommandExecutor for VmCoordinator {
    async fn start_vm(&self, vm_type: VmType) -> Result<Uuid> {
        // Start VM through controller and register atomically
        let vm_id = Uuid::new_v4();

        // Convert our trait type to the existing vm_types
        let vm_config = match vm_type {
            VmType::Ephemeral => {
                super::vm_types::VmConfig::ephemeral(vm_id)
            }
            VmType::Service => {
                super::vm_types::VmConfig::service(vm_id)
            }
        };

        // Start VM and register in one atomic operation
        self.controller.start_vm(vm_config).await?;
        Ok(vm_id)
    }

    async fn stop_vm(&self, vm_id: Uuid) -> Result<()> {
        // Stop VM and update registry atomically
        self.controller.shutdown_vm(vm_id, false).await
    }

    async fn execute_job_on_vm(&self, job: Job, vm_id: Option<Uuid>) -> Result<VmAssignment> {
        // Route job through the router which handles all state transitions
        let assignment = self.router.route_job(job).await?;

        // Convert to trait type
        match assignment {
            super::VmAssignment::Ephemeral(id) => Ok(VmAssignment::Ephemeral(id)),
            super::VmAssignment::Service(id) => Ok(VmAssignment::Service(id)),
        }
    }

    async fn update_vm_state(&self, vm_id: Uuid, new_state: VmState) -> Result<()> {
        // Convert trait state to existing type
        let state = match new_state {
            VmState::Starting => super::vm_types::VmState::Starting,
            VmState::Running => super::vm_types::VmState::Running,
            VmState::Idle => super::vm_types::VmState::Idle,
            VmState::Busy => super::vm_types::VmState::Busy,
            VmState::Stopping => super::vm_types::VmState::Stopping,
            VmState::Stopped => super::vm_types::VmState::Stopped,
            VmState::Failed => super::vm_types::VmState::Failed,
        };

        self.registry.update_state(vm_id, state).await
    }

    async fn get_or_create_service_vm(&self) -> Result<Uuid> {
        // Check for available service VM first
        if let Some(vm_id) = self.registry.get_available_service_vm().await? {
            return Ok(vm_id);
        }

        // No available VM, create new one
        self.start_vm(VmType::Service).await
    }
}

/// Implementation of VmQueryService trait - read-only operations
#[async_trait]
impl VmQueryService for VmCoordinator {
    async fn get_vm(&self, vm_id: Uuid) -> Result<Option<VmInfo>> {
        let vm = self.registry.get_vm(vm_id).await?;
        Ok(vm.map(|v| VmInfo {
            id: v.config.id,
            state: match v.state {
                super::vm_types::VmState::Starting => VmState::Starting,
                super::vm_types::VmState::Running => VmState::Running,
                super::vm_types::VmState::Idle => VmState::Idle,
                super::vm_types::VmState::Busy => VmState::Busy,
                super::vm_types::VmState::Stopping => VmState::Stopping,
                super::vm_types::VmState::Stopped => VmState::Stopped,
                super::vm_types::VmState::Failed => VmState::Failed,
            },
            vm_type: match v.config.mode {
                super::vm_types::VmMode::Ephemeral => VmType::Ephemeral,
                super::vm_types::VmMode::Service => VmType::Service,
            },
            created_at: v.created_at,
            job_count: v.job_count,
        }))
    }

    async fn list_vms(&self) -> Result<Vec<VmInfo>> {
        let vms = self.registry.list_vms().await?;
        Ok(vms.into_iter().map(|v| VmInfo {
            id: v.config.id,
            state: match v.state {
                super::vm_types::VmState::Starting => VmState::Starting,
                super::vm_types::VmState::Running => VmState::Running,
                super::vm_types::VmState::Idle => VmState::Idle,
                super::vm_types::VmState::Busy => VmState::Busy,
                super::vm_types::VmState::Stopping => VmState::Stopping,
                super::vm_types::VmState::Stopped => VmState::Stopped,
                super::vm_types::VmState::Failed => VmState::Failed,
            },
            vm_type: match v.config.mode {
                super::vm_types::VmMode::Ephemeral => VmType::Ephemeral,
                super::vm_types::VmMode::Service => VmType::Service,
            },
            created_at: v.created_at,
            job_count: v.job_count,
        }).collect())
    }

    async fn get_vms_by_state(&self, state: VmState) -> Result<Vec<Uuid>> {
        let vm_state = match state {
            VmState::Starting => super::vm_types::VmState::Starting,
            VmState::Running => super::vm_types::VmState::Running,
            VmState::Idle => super::vm_types::VmState::Idle,
            VmState::Busy => super::vm_types::VmState::Busy,
            VmState::Stopping => super::vm_types::VmState::Stopping,
            VmState::Stopped => super::vm_types::VmState::Stopped,
            VmState::Failed => super::vm_types::VmState::Failed,
        };

        self.registry.get_vms_by_state(vm_state).await
    }

    async fn is_vm_healthy(&self, vm_id: Uuid) -> Result<bool> {
        let status = self.health_checker.check_health(vm_id).await?;
        Ok(status.is_healthy)
    }

    async fn get_vm_resources(&self, vm_id: Uuid) -> Result<ResourceUsage> {
        // For now, return default resources
        // In a real implementation, this would query actual resource usage
        Ok(ResourceUsage::default())
    }
}
