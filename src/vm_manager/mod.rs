// VM Lifecycle Management System
// Supports both ephemeral (one-job) and service (long-running) VMs
//
// Architecture:
// - VmCoordinator: Message-based orchestrator (internal)
// - VmManager: Backward-compatible facade (public API)
// - Components communicate via channels for loose coupling

pub mod vm_types;
pub mod vm_registry;
pub mod vm_controller;
pub mod job_router;
pub mod resource_monitor;
pub mod health_checker;
pub mod control_protocol;
pub mod messages;
pub mod coordinator;

use anyhow::Result;
use std::sync::Arc;

use crate::hiqlite_service::HiqliteService;

pub use vm_types::{VmConfig, VmInstance, VmMode, VmState, IsolationLevel};
pub use vm_registry::VmRegistry;
pub use vm_controller::VmController;
pub use job_router::JobRouter;
pub use resource_monitor::ResourceMonitor;
pub use health_checker::HealthChecker;

// Re-export VmAssignment from job_router for backward compatibility
pub use job_router::VmAssignment;

use coordinator::VmCoordinator;

/// Main VM Manager - Backward-compatible facade over VmCoordinator
///
/// This struct provides the same public API as before, but now uses
/// the VmCoordinator internally for message-based component orchestration.
/// All public fields are maintained for backward compatibility.
pub struct VmManager {
    // Coordinator handles all message passing (private)
    coordinator: Arc<VmCoordinator>,

    // Public fields for backward compatibility
    pub registry: Arc<VmRegistry>,
    pub controller: Arc<VmController>,
    pub router: Arc<JobRouter>,
    pub monitor: Arc<ResourceMonitor>,
    pub health_checker: Arc<HealthChecker>,
    pub config: VmManagerConfig,
}

#[derive(Debug, Clone)]
pub struct VmManagerConfig {
    /// Maximum number of concurrent VMs
    pub max_vms: usize,
    /// Enable auto-scaling
    pub auto_scaling: bool,
    /// Pre-warm this many idle VMs
    pub pre_warm_count: usize,
    /// Path to microvm flake directory
    pub flake_dir: std::path::PathBuf,
    /// Path for VM state and job data
    pub state_dir: std::path::PathBuf,
    /// Default memory for VMs (MB)
    pub default_memory_mb: u32,
    /// Default vCPUs for VMs
    pub default_vcpus: u32,
}

impl Default for VmManagerConfig {
    fn default() -> Self {
        Self {
            max_vms: 20,
            auto_scaling: true,
            pre_warm_count: 2,
            flake_dir: std::path::PathBuf::from("./microvms"),
            state_dir: std::path::PathBuf::from("./mvm-ci-state"),
            default_memory_mb: 512,
            default_vcpus: 1,
        }
    }
}

impl VmManager {
    /// Create a new VM manager
    ///
    /// This now creates a VmCoordinator internally and exposes component
    /// references for backward compatibility.
    pub async fn new(config: VmManagerConfig, hiqlite: Arc<HiqliteService>) -> Result<Self> {
        // Create the coordinator with all components
        let coordinator = Arc::new(VmCoordinator::new(config.clone(), hiqlite).await?);

        // Get references to components for backward compatibility
        let registry = coordinator.registry();
        let controller = coordinator.controller();
        let router = coordinator.router();
        let monitor = coordinator.monitor();
        let health_checker = coordinator.health_checker();

        Ok(Self {
            coordinator,
            registry,
            controller,
            router,
            monitor,
            health_checker,
            config,
        })
    }

    /// Start the VM manager background tasks
    ///
    /// This now delegates to the coordinator's background task management.
    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting VM Manager");

        // Start coordinator background tasks (handles all component coordination)
        self.coordinator.start_background_tasks().await?;

        Ok(())
    }

    /// Route a job to an appropriate VM
    pub async fn execute_job(&self, job: crate::Job) -> Result<VmAssignment> {
        tracing::info!(job_id = %job.id, "Routing job to VM");

        // Route job based on requirements
        let assignment = self.router.route_job(&job).await?;

        match assignment {
            VmAssignment::Ephemeral(vm_id) => {
                tracing::info!(job_id = %job.id, vm_id = %vm_id, "Job assigned to ephemeral VM");
                // VM will automatically terminate after job
            }
            VmAssignment::Service(vm_id) => {
                tracing::info!(job_id = %job.id, vm_id = %vm_id, "Job assigned to service VM");
                // Send job to service VM
                self.controller.send_job_to_vm(vm_id, &job).await?;
            }
        }

        Ok(assignment)
    }


    /// Get current VM statistics
    pub async fn get_stats(&self) -> Result<VmStats> {
        Ok(VmStats {
            total_vms: self.registry.count_all().await,
            running_vms: self.registry.count_by_state(VmState::Ready).await
                + self.registry.count_by_state(VmState::Busy {
                    job_id: String::new(),
                    started_at: 0
                }).await,
            idle_vms: self.registry.count_by_state(VmState::Idle {
                jobs_completed: 0,
                last_job_at: 0
            }).await,
            failed_vms: self.registry.count_by_state(VmState::Failed {
                error: String::new()
            }).await,
        })
    }

    /// Gracefully shutdown all VMs
    ///
    /// This now delegates to the coordinator for proper shutdown sequence.
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down VM Manager");

        // Delegate to coordinator for proper shutdown
        self.coordinator.shutdown().await?;

        Ok(())
    }

    /// Submit a job to the VM manager (routes to best VM)
    pub async fn submit_job(&self, job: crate::Job) -> Result<JobResult> {
        let assignment = self.execute_job(job.clone()).await?;

        // Convert VmAssignment to JobResult
        Ok(JobResult {
            vm_id: assignment.vm_id(),
            success: true,  // Would be set based on actual execution
            output: format!("Job {} assigned to VM {}", job.id, assignment.vm_id()),
            error: None,
        })
    }

    /// Submit a job to a specific VM
    pub async fn submit_job_to_vm(&self, vm_id: uuid::Uuid, job: crate::Job) -> Result<JobResult> {
        // Send job to specific VM
        self.controller.send_job_to_vm(vm_id, &job).await?;

        Ok(JobResult {
            vm_id,
            success: true,
            output: format!("Job {} sent to VM {}", job.id, vm_id),
            error: None,
        })
    }

    /// Start a new VM with the given configuration
    pub async fn start_vm(&self, config: VmConfig) -> Result<VmInstance> {
        self.controller.start_vm(config).await
    }

    /// Stop a running VM
    pub async fn stop_vm(&self, vm_id: uuid::Uuid) -> Result<()> {
        self.controller.stop_vm(vm_id).await
    }

    /// Start monitoring tasks
    pub async fn start_monitoring(&self) -> Result<()> {
        // Start health monitoring
        let health_checker = self.health_checker.clone();
        tokio::spawn(async move {
            health_checker.start_monitoring().await;
        });

        // Start resource monitoring
        let monitor = self.monitor.clone();
        tokio::spawn(async move {
            monitor.monitoring_loop().await;
        });

        Ok(())
    }

    /// Get health status of a specific VM
    pub async fn get_health_status(&self, vm_id: uuid::Uuid) -> Result<health_checker::HealthStatus> {
        Ok(self.health_checker.get_health_status(vm_id).await)
    }

    /// Get VM state
    pub async fn get_vm_status(&self, vm_id: uuid::Uuid) -> Result<Option<vm_types::VmState>> {
        self.controller.get_vm_status(vm_id).await
    }
}

/// Result from job execution
pub struct JobResult {
    pub vm_id: uuid::Uuid,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VmStats {
    pub total_vms: usize,
    pub running_vms: usize,
    pub idle_vms: usize,
    pub failed_vms: usize,
}