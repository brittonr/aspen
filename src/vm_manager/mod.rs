// VM Lifecycle Management System
// Supports both ephemeral (one-job) and service (long-running) VMs

pub mod vm_types;
pub mod vm_registry;
pub mod vm_controller;
pub mod job_router;
pub mod resource_monitor;
pub mod health_checker;
pub mod control_protocol;

#[cfg(test)]
mod tests;

use anyhow::Result;
use std::sync::Arc;

use crate::hiqlite_service::HiqliteService;

pub use vm_types::{VmConfig, VmInstance, VmMode, VmState, IsolationLevel};
pub use vm_registry::VmRegistry;
pub use vm_controller::VmController;
pub use job_router::{JobRouter, VmAssignment};
pub use resource_monitor::ResourceMonitor;
pub use health_checker::HealthChecker;

/// Main VM Manager coordinating all components
pub struct VmManager {
    registry: Arc<VmRegistry>,
    controller: Arc<VmController>,
    router: Arc<JobRouter>,
    monitor: Arc<ResourceMonitor>,
    health_checker: Arc<HealthChecker>,
    config: VmManagerConfig,
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
            state_dir: std::path::PathBuf::from("/var/lib/mvm-ci"),
            default_memory_mb: 512,
            default_vcpus: 1,
        }
    }
}

impl VmManager {
    /// Create a new VM manager
    pub async fn new(config: VmManagerConfig, hiqlite: Arc<HiqliteService>) -> Result<Self> {
        // Initialize registry with Hiqlite persistence
        let registry = Arc::new(VmRegistry::new(hiqlite, &config.state_dir).await?);

        // Create controller for VM lifecycle operations
        let controller = Arc::new(VmController::new(
            config.clone(),
            Arc::clone(&registry),
        )?);

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

        // Initialize health checker with default config
        let health_config = crate::vm_manager::health_checker::HealthCheckConfig::default();
        let health_checker = Arc::new(HealthChecker::new(
            Arc::clone(&registry),
            health_config,
        ));

        Ok(Self {
            registry,
            controller,
            router,
            monitor,
            health_checker,
            config,
        })
    }

    /// Start the VM manager background tasks
    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting VM Manager");

        // Start health checking loop
        let health_checker = Arc::clone(&self.health_checker);
        tokio::spawn(async move {
            health_checker.health_check_loop().await;
        });

        // Start resource monitoring
        let monitor = Arc::clone(&self.monitor);
        tokio::spawn(async move {
            monitor.monitoring_loop().await;
        });

        // Pre-warm VMs if configured
        if self.config.auto_scaling && self.config.pre_warm_count > 0 {
            self.pre_warm_vms().await?;
        }

        // Recover any VMs from previous run
        self.recover_existing_vms().await?;

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

    /// Pre-warm idle VMs for faster job execution
    async fn pre_warm_vms(&self) -> Result<()> {
        tracing::info!(count = self.config.pre_warm_count, "Pre-warming service VMs");

        for i in 0..self.config.pre_warm_count {
            let config = VmConfig::default_service();

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
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down VM Manager");

        // Stop accepting new jobs
        self.router.stop_routing().await;

        // Gracefully shutdown all VMs
        let all_vms = self.registry.list_all_vms().await?;

        for vm in all_vms {
            if !matches!(vm.state, VmState::Terminated { .. }) {
                self.controller.shutdown_vm(vm.config.id, true).await?;
            }
        }

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