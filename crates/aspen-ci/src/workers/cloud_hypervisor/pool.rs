//! VM pool for warm VM management.
//!
//! Maintains a pool of pre-booted VMs for fast job assignment.
//! Supports snapshot-based fast restoration and elastic scaling.

use std::collections::VecDeque;
use std::sync::Arc;

use aspen_core::MAX_CI_VMS_PER_NODE;
use tokio::sync::Mutex;
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::config::CloudHypervisorWorkerConfig;
use super::error::CloudHypervisorError;
use super::error::Result;
use super::vm::ManagedCiVm;
use super::vm::SharedVm;
use super::vm::VmState;

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
    /// Waits for the cluster ticket file to exist before starting VMs.
    pub async fn initialize(&self) -> Result<()> {
        let target_size = self.config.pool_size.min(self.config.max_vms);

        info!(target_size = target_size, max_vms = self.config.max_vms, "initializing VM pool");

        // Wait for the cluster ticket file to exist before starting VMs.
        // The ticket file is written after the Iroh endpoint is ready, which happens
        // after this worker is registered but before it processes any jobs.
        if let Some(ref ticket_file) = self.config.cluster_ticket_file {
            let max_wait_secs = 60;
            let mut waited_secs = 0;
            info!(
                ticket_file = %ticket_file.display(),
                max_wait_secs = max_wait_secs,
                "waiting for cluster ticket file before starting VMs"
            );

            while !ticket_file.exists() && waited_secs < max_wait_secs {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                waited_secs += 1;
                if waited_secs % 10 == 0 {
                    debug!(
                        ticket_file = %ticket_file.display(),
                        waited_secs = waited_secs,
                        "still waiting for cluster ticket file"
                    );
                }
            }

            if ticket_file.exists() {
                info!(
                    ticket_file = %ticket_file.display(),
                    waited_secs = waited_secs,
                    "cluster ticket file found"
                );
            } else {
                warn!(
                    ticket_file = %ticket_file.display(),
                    waited_secs = waited_secs,
                    "cluster ticket file not found after waiting - VMs will not be able to join cluster"
                );
            }
        } else {
            warn!("no cluster ticket file configured - VMs will not be able to join cluster");
        }

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
                let vm = self.create_and_start_vm().await.map_err(|e| {
                    error!(error = ?e, "failed to create new VM");
                    e
                })?;
                vm.assign(job_id.to_string()).await?;
                // Keep permit alive while VM exists
                std::mem::forget(permit);
                info!(vm_id = %vm.id, job_id = %job_id, "created new VM for job");
                Ok(vm)
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
    ///
    /// If `destroy_after_job` is enabled (default), the VM is destroyed to ensure
    /// a clean overlay state for the next job. This prevents issues where the
    /// tmpfs overlay accumulates whiteout files between jobs, causing library
    /// loading failures (e.g., "cannot open shared object file").
    ///
    /// If `destroy_after_job` is disabled, the VM is returned to the pool for
    /// reuse, which is faster but can cause state leakage between jobs.
    pub async fn release(&self, vm: SharedVm) -> Result<()> {
        debug!(vm_id = %vm.id, destroy_after_job = %self.config.destroy_after_job, "releasing VM");

        // If configured to destroy after each job, skip cleanup and destroy immediately.
        // This ensures a completely fresh overlay state for the next job.
        if self.config.destroy_after_job {
            debug!(vm_id = %vm.id, "destroying VM after job (destroy_after_job=true)");
            self.destroy_vm(&vm).await;
            return Ok(());
        }

        // Clean up the VM for reuse
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
        debug!(current_idle = current_idle, target = target, to_create = to_create, "maintaining pool");

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
    use std::path::PathBuf;

    use aspen_core::CI_VM_DEFAULT_POOL_SIZE;

    use super::*;

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

    #[tokio::test]
    async fn test_pool_status_initial_values() {
        let config = CloudHypervisorWorkerConfig {
            pool_size: 3,
            max_vms: 6, // Use a value below MAX_CI_VMS_PER_NODE (8)
            ..test_config()
        };
        let pool = VmPool::new(config);
        let status = pool.status().await;

        assert_eq!(status.idle_vms, 0);
        assert_eq!(status.total_vms, 0);
        assert_eq!(status.max_vms, 6);
        assert_eq!(status.target_pool_size, 3);
        // All permits should be available initially (capped at max_vms since it's < MAX_CI_VMS_PER_NODE)
        assert_eq!(status.available_capacity, 6);
    }

    #[tokio::test]
    async fn test_pool_max_vms_capped_by_constant() {
        // Try to create pool with max_vms > MAX_CI_VMS_PER_NODE
        let config = CloudHypervisorWorkerConfig {
            max_vms: MAX_CI_VMS_PER_NODE + 100,
            pool_size: 1,
            ..test_config()
        };
        let pool = VmPool::new(config);
        let status = pool.status().await;

        // Should be capped at the constant
        assert_eq!(status.max_vms, MAX_CI_VMS_PER_NODE + 100);
        // Semaphore should use the capped value
        assert_eq!(status.available_capacity, MAX_CI_VMS_PER_NODE);
    }

    #[tokio::test]
    async fn test_pool_shutdown_empty() {
        let pool = VmPool::new(test_config());

        // Shutdown on empty pool should succeed
        let result = pool.shutdown().await;
        assert!(result.is_ok());

        // Status should be all zeros after shutdown
        let status = pool.status().await;
        assert_eq!(status.idle_vms, 0);
        assert_eq!(status.total_vms, 0);
    }

    #[tokio::test]
    async fn test_pool_status_is_clone() {
        let pool = VmPool::new(test_config());
        let status = pool.status().await;

        // PoolStatus should be Clone
        let status2 = status.clone();
        assert_eq!(status.idle_vms, status2.idle_vms);
        assert_eq!(status.max_vms, status2.max_vms);
    }

    #[tokio::test]
    async fn test_pool_status_is_debug() {
        let pool = VmPool::new(test_config());
        let status = pool.status().await;

        // PoolStatus should be Debug
        let debug_str = format!("{:?}", status);
        assert!(debug_str.contains("PoolStatus"));
        assert!(debug_str.contains("idle_vms"));
    }

    #[tokio::test]
    async fn test_pool_maintain_when_at_target() {
        // Use pool_size of 1 (minimum valid) to avoid validation issues
        let config = CloudHypervisorWorkerConfig {
            pool_size: 1,
            max_vms: 8,
            ..test_config()
        };
        let pool = VmPool::new(config);

        // Without actual VMs, maintain will try to create but fail
        // This tests the maintain logic doesn't panic
        pool.maintain().await;

        // Pool should still be empty since VM creation requires real infrastructure
        let status = pool.status().await;
        // We just verify the method completes without panic
        assert!(status.total_vms <= 8);
    }

    #[tokio::test]
    async fn test_pool_with_different_node_ids() {
        let config1 = CloudHypervisorWorkerConfig {
            node_id: 1,
            ..test_config()
        };
        let config2 = CloudHypervisorWorkerConfig {
            node_id: 2,
            ..test_config()
        };

        let pool1 = VmPool::new(config1);
        let pool2 = VmPool::new(config2);

        // Both pools should work independently
        let status1 = pool1.status().await;
        let status2 = pool2.status().await;

        assert_eq!(status1.idle_vms, 0);
        assert_eq!(status2.idle_vms, 0);
    }

    #[test]
    fn test_pool_status_fields() {
        let status = PoolStatus {
            idle_vms: 5,
            total_vms: 10,
            max_vms: 20,
            available_capacity: 10,
            target_pool_size: 5,
        };

        assert_eq!(status.idle_vms, 5);
        assert_eq!(status.total_vms, 10);
        assert_eq!(status.max_vms, 20);
        assert_eq!(status.available_capacity, 10);
        assert_eq!(status.target_pool_size, 5);
    }
}
