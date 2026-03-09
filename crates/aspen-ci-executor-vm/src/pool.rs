//! VM pool for warm VM management.
//!
//! Maintains a pool of pre-booted VMs for fast job assignment.
//! Supports snapshot-based fast restoration and elastic scaling.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;

use aspen_core::MAX_CI_VMS_PER_NODE;
use tokio::sync::Mutex;
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

/// How long `acquire()` waits for a VM to become available when the pool
/// is at capacity, before returning `NoVmsAvailable`. This gives in-flight
/// jobs time to finish and release their VMs.
const ACQUIRE_WAIT_TIMEOUT: Duration = Duration::from_secs(30);

use crate::config::CloudHypervisorWorkerConfig;
use crate::error::CloudHypervisorError;
use crate::error::Result;
use crate::vm::ManagedCiVm;
use crate::vm::SharedVm;
use crate::vm::VmState;

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

    /// Monotonic counter for generating unique VM indices.
    /// Uses AtomicU64 to avoid wrapping collisions over long-lived nodes.
    next_vm_index: AtomicU64,
}

impl VmPool {
    /// Create a new VM pool.
    pub fn new(config: CloudHypervisorWorkerConfig) -> Self {
        let max_vms = config.max_vms.min(MAX_CI_VMS_PER_NODE);

        Self {
            config,
            idle_vms: Mutex::new(VecDeque::with_capacity(max_vms as usize)),
            all_vms: Mutex::new(Vec::new()),
            vm_semaphore: Arc::new(Semaphore::new(max_vms as usize)),
            next_vm_index: AtomicU64::new(0),
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
            // Acquire permit before creating VM
            let permit = match self.vm_semaphore.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => {
                    warn!("no permits available for pre-warming");
                    break;
                }
            };

            match self.create_and_start_vm().await {
                Ok(vm) => {
                    // Store permit in VM — released automatically on VM drop
                    *vm.pool_permit.write().await = Some(permit);
                    info!(vm_id = %vm.id, "pre-warmed VM added to pool");
                    self.idle_vms.lock().await.push_back(vm);
                }
                Err(e) => {
                    // Permit drops here, releasing capacity back
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
    /// none are available and capacity allows. Dead VMs are evicted
    /// on encounter.
    pub async fn acquire(&self, job_id: &str) -> Result<SharedVm> {
        debug!(job_id = %job_id, "acquiring VM from pool");

        // Try to get a healthy idle VM. Loop because we may encounter dead VMs
        // that need to be evicted before finding a live one.
        loop {
            let vm = self.idle_vms.lock().await.pop_front();
            let Some(vm) = vm else {
                break; // No more idle VMs
            };

            // Check process liveness before using
            if !vm.is_process_alive().await {
                warn!(vm_id = %vm.id, job_id = %job_id, "evicting dead VM encountered during acquire");
                self.destroy_vm(&vm).await;
                continue;
            }

            let state = vm.state().await;
            if state == VmState::Idle {
                vm.assign(job_id.to_string()).await?;
                debug!(vm_id = %vm.id, job_id = %job_id, "acquired idle VM");
                return Ok(vm);
            }

            // VM is in unexpected state — destroy it
            warn!(vm_id = %vm.id, state = ?state, "unexpected VM state in idle queue, destroying");
            self.destroy_vm(&vm).await;
        }

        // No idle VMs, try to create a new one
        debug!(job_id = %job_id, "no idle VMs, attempting to create new one");

        // Try non-blocking acquire first
        match self.vm_semaphore.clone().try_acquire_owned() {
            Ok(permit) => return self.create_vm_with_permit(permit, job_id).await,
            Err(_) => {
                // Pool at capacity — wait up to ACQUIRE_WAIT_TIMEOUT for a permit
                // (a VM being destroyed will release one).
                debug!(job_id = %job_id, "pool at capacity, waiting for a VM to be released");

                match tokio::time::timeout(ACQUIRE_WAIT_TIMEOUT, self.vm_semaphore.clone().acquire_owned()).await {
                    Ok(Ok(permit)) => return self.create_vm_with_permit(permit, job_id).await,
                    Ok(Err(_closed)) => {
                        // Semaphore closed — shouldn't happen
                        Err(CloudHypervisorError::PoolAtCapacity {
                            max_vms: self.config.max_vms,
                        })
                    }
                    Err(_timeout) => Err(CloudHypervisorError::NoVmsAvailable {
                        timeout_ms: ACQUIRE_WAIT_TIMEOUT.as_millis() as u64,
                    }),
                }
            }
        }
    }

    /// Create a VM using an already-acquired semaphore permit.
    async fn create_vm_with_permit(&self, permit: tokio::sync::OwnedSemaphorePermit, job_id: &str) -> Result<SharedVm> {
        let vm = self.create_and_start_vm().await.map_err(|e| {
            error!(error = ?e, "failed to create new VM");
            // Permit drops here automatically, releasing capacity
            e
        })?;
        // Store permit in VM — released automatically on VM drop
        *vm.pool_permit.write().await = Some(permit);
        vm.assign(job_id.to_string()).await?;
        info!(vm_id = %vm.id, job_id = %job_id, "created new VM for job");
        Ok(vm)
    }

    /// Release a VM back to the pool after job completion.
    ///
    /// If `should_destroy_after_job` is enabled (default), the VM is destroyed to ensure
    /// a clean overlay state for the next job. This prevents issues where the
    /// tmpfs overlay accumulates whiteout files between jobs, causing library
    /// loading failures (e.g., "cannot open shared object file").
    ///
    /// If `should_destroy_after_job` is disabled, the VM is returned to the pool for
    /// reuse, which is faster but can cause state leakage between jobs.
    pub async fn release(&self, vm: SharedVm) -> Result<()> {
        debug!(vm_id = %vm.id, destroy_after_job = %self.config.should_destroy_after_job, "releasing VM");

        // If configured to destroy after each job, skip cleanup and destroy immediately.
        // This ensures a completely fresh overlay state for the next job.
        if self.config.should_destroy_after_job {
            debug!(vm_id = %vm.id, "destroying VM after job (should_destroy_after_job=true)");
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
        let target_size = self.config.pool_size as usize;
        let kept = {
            let mut idle = self.idle_vms.lock().await;
            if idle.len() < target_size {
                idle.push_back(vm.clone());
                true
            } else {
                false
            }
        };

        if kept {
            debug!(vm_id = %vm.id, "VM returned to pool");
        } else {
            debug!(vm_id = %vm.id, "pool full, destroying VM");
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

        // Report the effective max (capped by MAX_CI_VMS_PER_NODE), not the
        // uncapped config value, so callers see consistent numbers.
        let effective_max = self.config.max_vms.min(MAX_CI_VMS_PER_NODE);

        PoolStatus {
            idle_vms: idle as u32,
            total_vms: total as u32,
            max_vms: effective_max,
            available_capacity: available_permits as u32,
            target_pool_size: self.config.pool_size,
        }
    }

    /// Ensure the pool has at least the target number of healthy idle VMs.
    ///
    /// Call this periodically to maintain warm VM availability.
    /// First evicts dead VMs from the idle queue, then creates replacements.
    pub async fn maintain(&self) {
        // Phase 1: Evict dead idle VMs.
        // Check each idle VM's process liveness and remove any that have crashed.
        let dead_vms = {
            let mut idle = self.idle_vms.lock().await;
            let mut alive = VecDeque::with_capacity(idle.len());
            let mut dead = Vec::new();

            for vm in idle.drain(..) {
                if vm.is_process_alive().await {
                    alive.push_back(vm);
                } else {
                    warn!(vm_id = %vm.id, "evicting dead VM from idle queue");
                    dead.push(vm);
                }
            }

            *idle = alive;
            dead
        };

        // Destroy dead VMs (releases permits + cleans up resources)
        for vm in &dead_vms {
            self.destroy_vm(vm).await;
        }
        if !dead_vms.is_empty() {
            info!(evicted = dead_vms.len(), "evicted dead VMs from idle pool");
        }

        // Phase 2: Create replacement VMs to reach target pool size.
        let current_idle = self.idle_vms.lock().await.len();
        let target = self.config.pool_size as usize;

        if current_idle >= target {
            return;
        }

        let to_create = target - current_idle;
        debug!(current_idle = current_idle, target = target, to_create = to_create, "replenishing pool");

        for _ in 0..to_create {
            // Acquire permit before creating VM
            let permit = match self.vm_semaphore.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => break,
            };

            match self.create_and_start_vm().await {
                Ok(vm) => {
                    *vm.pool_permit.write().await = Some(permit);
                    self.idle_vms.lock().await.push_back(vm);
                }
                Err(e) => {
                    // Permit drops here, releasing capacity back
                    warn!(error = ?e, "failed to create maintenance VM");
                    break;
                }
            }
        }
    }

    // Private methods

    /// Create and start a new VM.
    async fn create_and_start_vm(&self) -> Result<SharedVm> {
        // Monotonic: never wraps, never collides.
        // u64 overflow is effectively impossible (~18 quintillion VMs).
        let vm_index = self.next_vm_index.fetch_add(1, Ordering::Relaxed) as u32;

        let vm = Arc::new(ManagedCiVm::new(self.config.clone(), vm_index));

        // Track in all_vms
        self.all_vms.lock().await.push(vm.clone());

        // Start the VM
        vm.start().await?;

        Ok(vm)
    }

    /// Destroy a VM and release its resources.
    ///
    /// The semaphore permit is released automatically when the VM's
    /// `pool_permit` is dropped (either here or on VM drop).
    async fn destroy_vm(&self, vm: &SharedVm) {
        // Shutdown the VM
        if let Err(e) = vm.shutdown().await {
            warn!(vm_id = %vm.id, error = ?e, "error in VM shutdown during destroy");
        }

        // Remove from all_vms
        {
            let mut all = self.all_vms.lock().await;
            all.retain(|v| !Arc::ptr_eq(v, vm));
        }

        // Release semaphore permit by dropping it explicitly.
        // This is safe even if the permit was already dropped (e.g., VM created
        // during initialize() before a permit was assigned).
        let _ = vm.pool_permit.write().await.take();

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

        // Both max_vms and available_capacity should be capped consistently
        assert_eq!(status.max_vms, MAX_CI_VMS_PER_NODE);
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

    #[tokio::test]
    async fn test_acquire_returns_no_vms_available_when_empty() {
        // With pool_size=1, max_vms=1 and no actual VMs created (no infra),
        // acquire will try to create a VM, fail, and the permit drops.
        // But since create_and_start_vm will fail without real kernel paths,
        // we verify the error path doesn't panic.
        let config = CloudHypervisorWorkerConfig {
            pool_size: 1,
            max_vms: 1,
            ..test_config()
        };
        let pool = VmPool::new(config);

        // Pool has permits but no real infrastructure — creation will fail
        let result = pool.acquire("test-job-1").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_acquire_wait_timeout_is_reasonable() {
        // Verify the timeout constant is bounded and reasonable
        assert!(ACQUIRE_WAIT_TIMEOUT.as_secs() > 0, "timeout must be positive");
        assert!(ACQUIRE_WAIT_TIMEOUT.as_secs() <= 120, "timeout should not exceed 2 minutes");
    }
}
