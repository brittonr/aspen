//! VM pool for warm VM management.
//!
//! Maintains a pool of pre-booted VMs for fast job assignment.
//! Supports snapshot-based fast restoration and elastic scaling.

use std::collections::VecDeque;
use std::sync::Arc;

use aspen_constants::MAX_CI_VMS_PER_NODE;
use tokio::sync::{Mutex, Semaphore};
use tracing::{debug, error, info, warn};

use super::config::CloudHypervisorWorkerConfig;
use super::error::{CloudHypervisorError, Result};
use super::vm::{ManagedCiVm, SharedVm, VmState};

/// VM pool for managing warm VMs.
pub struct VmPool {
    /// Pool configuration.
    config: CloudHypervisorWorkerConfig,

    /// Idle VMs ready for job assignment.
    idle_vms: Mutex<VecDeque<SharedVm>>,

    /// All VMs managed by this pool.
    all_vms: Mutex<Vec<SharedVm>>,

    /// Semaphore to limit total VM count.
    vm_semaphore: Arc<Semaphore>,

    /// Counter for generating unique VM indices.
    next_vm_index: Mutex<u32>,
}

impl VmPool {
    /// Create a new VM pool.
    pub fn new(config: CloudHypervisorWorkerConfig) -> Self {
        let max_vms = config.max_vms.min(MAX_CI_VMS_PER_NODE);

        Self {
            config,
            idle_vms: Mutex::new(VecDeque::new()),
            all_vms: Mutex::new(Vec::new()),
            vm_semaphore: Arc::new(Semaphore::new(max_vms as usize)),
            next_vm_index: Mutex::new(0),
        }
    }

    /// Initialize the pool by pre-warming VMs.
    ///
    /// This boots `pool_size` VMs so they're ready for immediate use.
    pub async fn initialize(&self) -> Result<()> {
        let target_size = self.config.pool_size.min(self.config.max_vms);

        info!(
            target_size = target_size,
            max_vms = self.config.max_vms,
            "initializing VM pool"
        );

        for _ in 0..target_size {
            match self.create_and_start_vm().await {
                Ok(vm) => {
                    info!(vm_id = %vm.id, "pre-warmed VM added to pool");
                    self.idle_vms.lock().await.push_back(vm);
                }
                Err(e) => {
                    error!(error = ?e, "failed to pre-warm VM, continuing with reduced pool");
                }
            }
        }

        let pool_size = self.idle_vms.lock().await.len();
        info!(pool_size = pool_size, "VM pool initialized");

        Ok(())
    }

    /// Acquire a VM for job execution.
    ///
    /// Returns an idle VM from the pool, or creates a new one if
    /// none are available and capacity allows.
    pub async fn acquire(&self, job_id: &str) -> Result<SharedVm> {
        debug!(job_id = %job_id, "acquiring VM from pool");

        // Try to get an idle VM
        if let Some(vm) = self.idle_vms.lock().await.pop_front() {
            let state = vm.state().await;
            if state == VmState::Idle {
                vm.assign(job_id.to_string()).await?;
                debug!(vm_id = %vm.id, job_id = %job_id, "acquired idle VM");
                return Ok(vm);
            } else {
                // VM is in unexpected state, put it back and try another
                warn!(vm_id = %vm.id, state = ?state, "unexpected VM state in idle queue");
                self.idle_vms.lock().await.push_back(vm);
            }
        }

        // No idle VMs, try to create a new one
        debug!(job_id = %job_id, "no idle VMs, attempting to create new one");

        // Try to acquire permit (non-blocking check)
        match self.vm_semaphore.clone().try_acquire_owned() {
            Ok(permit) => {
                match self.create_and_start_vm().await {
                    Ok(vm) => {
                        vm.assign(job_id.to_string()).await?;
                        // Keep permit alive while VM exists
                        std::mem::forget(permit);
                        info!(vm_id = %vm.id, job_id = %job_id, "created new VM for job");
                        return Ok(vm);
                    }
                    Err(e) => {
                        error!(error = ?e, "failed to create new VM");
                        return Err(e);
                    }
                }
            }
            Err(_) => {
                // Pool is at capacity
                Err(CloudHypervisorError::PoolAtCapacity {
                    max_vms: self.config.max_vms,
                })
            }
        }
    }

    /// Release a VM back to the pool after job completion.
    pub async fn release(&self, vm: SharedVm) -> Result<()> {
        debug!(vm_id = %vm.id, "releasing VM to pool");

        // Clean up the VM
        if let Err(e) = vm.release().await {
            warn!(vm_id = %vm.id, error = ?e, "error cleaning VM, destroying");
            self.destroy_vm(&vm).await;
            return Ok(());
        }

        // Check if we should keep it in the pool
        let pool_size = self.idle_vms.lock().await.len();
        let target_size = self.config.pool_size as usize;

        if pool_size < target_size {
            // Return to pool
            self.idle_vms.lock().await.push_back(vm.clone());
            debug!(
                vm_id = %vm.id,
                pool_size = pool_size + 1,
                "VM returned to pool"
            );
        } else {
            // Pool is full, destroy the VM
            debug!(vm_id = %vm.id, pool_size = pool_size, "pool full, destroying VM");
            self.destroy_vm(&vm).await;
        }

        Ok(())
    }

    /// Shutdown all VMs in the pool.
    pub async fn shutdown(&self) -> Result<()> {
        info!("shutting down VM pool");

        // Drain idle VMs
        let idle_vms: Vec<SharedVm> = {
            let mut idle = self.idle_vms.lock().await;
            idle.drain(..).collect()
        };

        for vm in idle_vms {
            if let Err(e) = vm.shutdown().await {
                warn!(vm_id = %vm.id, error = ?e, "error shutting down VM");
            }
        }

        // Shutdown all remaining VMs
        let all_vms: Vec<SharedVm> = {
            let mut all = self.all_vms.lock().await;
            all.drain(..).collect()
        };

        for vm in all_vms {
            if let Err(e) = vm.shutdown().await {
                warn!(vm_id = %vm.id, error = ?e, "error shutting down VM");
            }
        }

        info!("VM pool shutdown complete");
        Ok(())
    }

    /// Get the current pool status.
    pub async fn status(&self) -> PoolStatus {
        let idle = self.idle_vms.lock().await.len();
        let total = self.all_vms.lock().await.len();
        let available_permits = self.vm_semaphore.available_permits();

        PoolStatus {
            idle_vms: idle as u32,
            total_vms: total as u32,
            max_vms: self.config.max_vms,
            available_capacity: available_permits as u32,
            target_pool_size: self.config.pool_size,
        }
    }

    /// Ensure the pool has at least the target number of idle VMs.
    ///
    /// Call this periodically to maintain warm VM availability.
    pub async fn maintain(&self) {
        let current_idle = self.idle_vms.lock().await.len();
        let target = self.config.pool_size as usize;

        if current_idle >= target {
            return;
        }

        let to_create = target - current_idle;
        debug!(
            current_idle = current_idle,
            target = target,
            to_create = to_create,
            "maintaining pool"
        );

        for _ in 0..to_create {
            // Check if we have capacity
            let available = self.vm_semaphore.available_permits();
            if available == 0 {
                break;
            }

            match self.create_and_start_vm().await {
                Ok(vm) => {
                    self.idle_vms.lock().await.push_back(vm);
                }
                Err(e) => {
                    warn!(error = ?e, "failed to create maintenance VM");
                    break;
                }
            }
        }
    }

    // Private methods

    /// Create and start a new VM.
    async fn create_and_start_vm(&self) -> Result<SharedVm> {
        let vm_index = {
            let mut idx = self.next_vm_index.lock().await;
            let current = *idx;
            *idx = (*idx + 1) % 1000; // Wrap around
            current
        };

        let vm = Arc::new(ManagedCiVm::new(self.config.clone(), vm_index));

        // Track in all_vms
        self.all_vms.lock().await.push(vm.clone());

        // Start the VM
        vm.start().await?;

        Ok(vm)
    }

    /// Destroy a VM and release its resources.
    async fn destroy_vm(&self, vm: &SharedVm) {
        // Shutdown the VM
        if let Err(e) = vm.shutdown().await {
            warn!(vm_id = %vm.id, error = ?e, "error in VM shutdown during destroy");
        }

        // Remove from all_vms
        let mut all = self.all_vms.lock().await;
        all.retain(|v| !Arc::ptr_eq(v, vm));

        // Release semaphore permit
        self.vm_semaphore.add_permits(1);

        debug!(vm_id = %vm.id, "VM destroyed");
    }
}

/// Status of the VM pool.
#[derive(Debug, Clone)]
pub struct PoolStatus {
    /// Number of idle VMs ready for jobs.
    pub idle_vms: u32,
    /// Total number of VMs managed.
    pub total_vms: u32,
    /// Maximum allowed VMs.
    pub max_vms: u32,
    /// Available capacity for new VMs.
    pub available_capacity: u32,
    /// Target pool size for warm VMs.
    pub target_pool_size: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_config() -> CloudHypervisorWorkerConfig {
        CloudHypervisorWorkerConfig {
            node_id: 1,
            state_dir: PathBuf::from("/tmp/aspen-ci-test"),
            pool_size: CI_VM_DEFAULT_POOL_SIZE,
            max_vms: MAX_CI_VMS_PER_NODE,
            kernel_path: PathBuf::new(),
            initrd_path: PathBuf::new(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_pool_creation() {
        let config = test_config();
        let pool = VmPool::new(config.clone());
        let status = pool.status().await;

        assert_eq!(status.idle_vms, 0);
        assert_eq!(status.total_vms, 0);
        assert_eq!(status.max_vms, config.max_vms);
    }
}
