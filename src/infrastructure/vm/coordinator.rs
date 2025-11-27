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
use tokio::task::JoinHandle;

use super::health_checker::{HealthChecker, HealthCheckConfig};
use super::job_router::JobRouter;
use super::resource_monitor::ResourceMonitor;
use super::vm_controller::VmController;
use super::vm_registry::VmRegistry;
use super::VmManagerConfig;
use crate::hiqlite::HiqliteService;

/// VmCoordinator orchestrates all VM management components
pub struct VmCoordinator {
    // Component instances (owned by coordinator)
    registry: Arc<VmRegistry>,
    controller: Arc<VmController>,
    router: Arc<JobRouter>,
    monitor: Arc<ResourceMonitor>,
    health_checker: Arc<HealthChecker>,

    // Background task handles for graceful shutdown
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
            task_handles: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            config,
        })
    }

    /// Start all background tasks
    pub async fn start_background_tasks(&self) -> Result<()> {
        tracing::info!("Starting VmCoordinator background tasks");

        let mut handles = self.task_handles.lock().await;

        // Start health checking loop
        let health_checker = Arc::clone(&self.health_checker);
        let handle = tokio::spawn(async move {
            health_checker.health_check_loop().await;
        });
        handles.push(handle);

        // Start resource monitoring loop
        let monitor = Arc::clone(&self.monitor);
        let handle = tokio::spawn(async move {
            monitor.monitoring_loop().await;
        });
        handles.push(handle);

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

        // Stop monitoring and health checking
        self.health_checker.stop().await;

        // Gracefully shutdown all VMs
        let all_vms = self.registry.list_all_vms().await?;
        for vm in all_vms {
            if !matches!(vm.state, super::vm_types::VmState::Terminated { .. }) {
                self.controller.shutdown_vm(vm.config.id, true).await?;
            }
        }

        // Cancel all background tasks
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
