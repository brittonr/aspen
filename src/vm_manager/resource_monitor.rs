// Resource Monitor - Tracks VM resource usage and handles auto-scaling

use anyhow::Result;
use std::sync::Arc;
use tokio::time::{interval, Duration};

use super::vm_controller::VmController;
use super::vm_registry::VmRegistry;
use super::vm_types::{VmConfig, VmState};

/// Resource monitor for VM management
pub struct ResourceMonitor {
    registry: Arc<VmRegistry>,
    controller: Arc<VmController>,
}

impl ResourceMonitor {
    /// Create new resource monitor
    pub fn new(registry: Arc<VmRegistry>, controller: Arc<VmController>) -> Self {
        Self {
            registry,
            controller,
        }
    }

    /// Main monitoring loop
    pub async fn monitoring_loop(&self) {
        let mut interval = interval(Duration::from_secs(10));

        loop {
            interval.tick().await;

            if let Err(e) = self.check_resources().await {
                tracing::error!(error = %e, "Resource monitoring error");
            }
        }
    }

    /// Check resource usage and take action
    async fn check_resources(&self) -> Result<()> {
        let vms = self.registry.list_all_vms().await?;

        for vm in vms {
            // Check if VM should be recycled
            if vm.should_recycle() {
                tracing::info!(vm_id = %vm.config.id, "Recycling VM due to limits");
                self.controller.shutdown_vm(vm.config.id, true).await?;
            }

            // Check for idle service VMs
            if let VmState::Idle { last_job_at, .. } = vm.state {
                let idle_time = chrono::Utc::now().timestamp() - last_job_at;
                if idle_time > 300 {
                    // 5 minutes idle
                    tracing::info!(vm_id = %vm.config.id, "Shutting down idle VM");
                    self.controller.shutdown_vm(vm.config.id, true).await?;
                }
            }
        }

        // Auto-scaling check
        self.check_scaling().await?;

        Ok(())
    }

    /// Check if scaling is needed
    async fn check_scaling(&self) -> Result<()> {
        // Count available service VMs
        let idle_vms = self
            .registry
            .count_by_state(VmState::Idle {
                jobs_completed: 0,
                last_job_at: 0,
            })
            .await;

        // Scale up if needed (maintain at least 2 idle VMs)
        if idle_vms < 2 {
            let vms_to_start = 2 - idle_vms;
            for _ in 0..vms_to_start {
                let config = VmConfig::default_service();
                if let Err(e) = self.controller.start_vm(config).await {
                    tracing::error!(error = %e, "Failed to start service VM for scaling");
                }
            }
        }

        Ok(())
    }
}