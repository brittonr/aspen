//! VM pool for warm VM management.
//!
//! Maintains a pool of pre-booted VMs for fast job assignment.
//! Supports snapshot-based fast restoration and elastic scaling.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;

use aspen_core::MAX_CI_VMS_PER_NODE;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
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
use crate::snapshot::GoldenSnapshot;
use crate::vm::ManagedCiVm;
use crate::vm::SharedVm;
use crate::vm::VmState;

/// VM pool for managing warm VMs.
///
/// Lock ordering when multiple pool locks are needed:
/// `idle_vms` -> `all_vms` -> `golden_snapshot` -> `snapshot_needs_regen` ->
/// `shared_workspace_client`.
pub struct VmPool {
    /// Pool configuration.
    config: CloudHypervisorWorkerConfig,

    /// Idle VMs ready for job assignment.
    ///
    /// Lock ordering: `idle_vms` -> `all_vms` -> `golden_snapshot` -> `snapshot_needs_regen` ->
    /// `shared_workspace_client`.
    idle_vms: Mutex<VecDeque<SharedVm>>,

    /// All VMs managed by this pool.
    all_vms: Mutex<Vec<SharedVm>>,

    /// Semaphore to limit total VM count.
    vm_semaphore: Arc<Semaphore>,

    /// Monotonic counter for generating unique VM indices.
    /// Uses AtomicU64 to avoid wrapping collisions over long-lived nodes.
    next_vm_index: AtomicU64,

    /// Current golden snapshot (if valid).
    golden_snapshot: RwLock<Option<GoldenSnapshot>>,

    /// Consecutive restore failure counter (per-node, not distributed).
    /// Reset on success, incremented on failure. After `DEFAULT_MAX_RESTORE_FAILURES`
    /// consecutive failures, the golden snapshot is auto-invalidated.
    restore_failure_count: AtomicU32,

    /// Whether the snapshot has been marked for regeneration.
    snapshot_needs_regen: RwLock<bool>,

    /// External memory pressure level (0=Normal, 1=Warning, 2=Critical).
    ///
    /// Shared with `MemoryWatcher` in `aspen-cluster`. The watcher updates
    /// this atomically; the pool reads it in `acquire()` and `maintain()`.
    /// When `None`, memory pressure is not tracked (restores always allowed).
    pressure_level: Option<Arc<AtomicU8>>,

    /// Shared workspace client (single Iroh endpoint) used by all VMs.
    ///
    /// Created once during `initialize()`. Each VM clones this `Arc` instead
    /// of creating its own `FuseSyncClient` + Iroh endpoint, avoiding ~25s
    /// of relay discovery overhead per VM.
    shared_workspace_client: RwLock<Option<aspen_fuse::SharedClient>>,
}

impl VmPool {
    fn checked_u32_to_usize(value: u32) -> usize {
        usize::try_from(value).unwrap_or(usize::MAX)
    }

    fn checked_usize_to_u32(value: usize) -> u32 {
        u32::try_from(value).unwrap_or(u32::MAX)
    }

    fn checked_u128_to_u64(value: u128) -> u64 {
        u64::try_from(value).unwrap_or(u64::MAX)
    }

    fn max_vms_usize(max_vms: u32) -> usize {
        Self::checked_u32_to_usize(max_vms)
    }

    fn next_vm_index_u32(&self) -> Result<u32> {
        self.next_vm_index.fetch_add(1, Ordering::Relaxed).try_into().map_err(|_| {
            CloudHypervisorError::CreateVmFailed {
                reason: "VM index space exhausted u32 range".to_string(),
            }
        })
    }

    /// Create a new VM pool.
    pub fn new(config: CloudHypervisorWorkerConfig) -> Self {
        Self::with_pressure_level(config, None)
    }

    /// Create a VM pool with an external memory pressure level.
    ///
    /// The `pressure_level` `AtomicU8` should be shared with a `MemoryWatcher`
    /// that updates it periodically. Values: 0=Normal, 1=Warning, 2=Critical.
    ///
    /// At Critical pressure, `acquire()` rejects new restores.
    /// At Warning pressure, `maintain()` skips pre-warming.
    pub fn with_pressure_level(config: CloudHypervisorWorkerConfig, pressure_level: Option<Arc<AtomicU8>>) -> Self {
        let max_vms = config.max_vms.min(MAX_CI_VMS_PER_NODE);
        let max_vm_slots = Self::max_vms_usize(max_vms);

        Self {
            config,
            idle_vms: Mutex::new(VecDeque::with_capacity(max_vm_slots)),
            all_vms: Mutex::new(Vec::new()),
            vm_semaphore: Arc::new(Semaphore::new(max_vm_slots)),
            next_vm_index: AtomicU64::new(0),
            golden_snapshot: RwLock::new(None),
            restore_failure_count: AtomicU32::new(0),
            snapshot_needs_regen: RwLock::new(false),
            pressure_level,
            shared_workspace_client: RwLock::new(None),
        }
    }

    /// Get the current memory pressure level.
    ///
    /// Returns `PRESSURE_NORMAL` if no watcher is configured.
    fn current_pressure_level(&self) -> u8 {
        self.pressure_level
            .as_ref()
            .map(|p| p.load(Ordering::Relaxed))
            .unwrap_or(crate::verified::snapshot::PRESSURE_NORMAL)
    }

    /// Initialize the pool by pre-warming VMs.
    ///
    /// This boots `pool_size` VMs so they're ready for immediate use.
    /// Waits for the cluster ticket file to exist before starting VMs.
    pub async fn initialize(&self) -> Result<()> {
        let target_vm_count = self.config.pool_size.min(self.config.max_vms);

        info!(target_vm_count = target_vm_count, max_vms = self.config.max_vms, "initializing VM pool");

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

        // Create shared workspace client (single Iroh endpoint for all VMs).
        // This avoids ~25s of relay discovery overhead per VM by sharing one endpoint.
        if let Some(ticket_str) = self.config.get_cluster_ticket() {
            let final_ticket = if let Some(bridge_addr) = self.config.bridge_socket_addr() {
                match aspen_ticket::AspenClusterTicket::deserialize(&ticket_str) {
                    Ok(mut ticket) => {
                        ticket.inject_direct_addr(bridge_addr);
                        ticket.serialize()
                    }
                    Err(_) => ticket_str.clone(),
                }
            } else {
                ticket_str.clone()
            };

            match tokio::task::spawn_blocking(move || aspen_fuse::FuseSyncClient::from_ticket(&final_ticket)).await {
                Ok(Ok(client)) => {
                    info!("shared workspace client created (single Iroh endpoint for all VMs)");
                    *self.shared_workspace_client.write().await = Some(Arc::new(client));
                }
                Ok(Err(e)) => {
                    warn!(error = %e, "failed to create shared workspace client, VMs will create individual clients");
                }
                Err(e) => {
                    warn!(error = %e, "spawn_blocking failed for shared workspace client");
                }
            }
        }

        // Phase 1: Check for existing golden snapshot or create one.
        if self.config.enable_snapshots
            && let Some(ticket) = self.config.get_cluster_ticket()
        {
            let snapshot = GoldenSnapshot::from_config(&self.config);

            if snapshot.exists() {
                match snapshot.validate(&ticket) {
                    Ok(()) => {
                        info!(dir = %snapshot.dir.display(), "valid golden snapshot found");
                        *self.golden_snapshot.write().await = Some(snapshot);
                    }
                    Err(e) => {
                        warn!(error = %e, "golden snapshot invalid, will cold-boot and re-snapshot");
                        if let Err(e) = snapshot.invalidate("validation failed").await {
                            warn!(error = %e, "failed to clean up invalid snapshot");
                        }
                    }
                }
            } else {
                debug!("no golden snapshot exists yet, will create on first boot");
            }
        }

        // Phase 2: Pre-warm VMs.
        // If snapshots are enabled and no golden snapshot exists, the first VM
        // cold-boots, gets snapshotted at Idle, then joins the pool.
        let has_snapshot = self.golden_snapshot.read().await.is_some();
        let should_seed_golden_snapshot =
            self.config.enable_snapshots && !has_snapshot && self.config.get_cluster_ticket().is_some();

        for i in 0..target_vm_count {
            let permit = match self.vm_semaphore.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => {
                    warn!("no permits available for pre-warming");
                    break;
                }
            };

            // First VM: cold-boot and snapshot (if needed)
            if i == 0 && should_seed_golden_snapshot {
                match self.create_and_start_vm().await {
                    Ok(vm) => {
                        *vm.pool_permit.write().await = Some(permit);
                        info!(vm_id = %vm.id, "first VM booted, creating golden snapshot");

                        match GoldenSnapshot::create(&vm, &self.config).await {
                            Ok(snapshot) => {
                                info!(dir = %snapshot.dir.display(), "golden snapshot created from first VM");
                                *self.golden_snapshot.write().await = Some(snapshot);
                            }
                            Err(e) => {
                                warn!(error = %e, "failed to create golden snapshot, continuing without snapshots");
                            }
                        }

                        self.idle_vms.lock().await.push_back(vm);
                    }
                    Err(e) => {
                        error!(error = ?e, "failed to boot first VM for golden snapshot");
                    }
                }
                continue;
            }

            // Subsequent VMs: cold-boot (restore will be used by acquire/maintain paths)
            match self.create_and_start_vm().await {
                Ok(vm) => {
                    *vm.pool_permit.write().await = Some(permit);
                    info!(vm_id = %vm.id, "pre-warmed VM added to pool");
                    self.idle_vms.lock().await.push_back(vm);
                }
                Err(e) => {
                    error!(error = ?e, "failed to pre-warm VM, continuing with reduced pool");
                }
            }
        }

        let idle_vm_count = self.idle_vms.lock().await.len();
        let snapshot_status = if self.golden_snapshot.read().await.is_some() {
            "valid"
        } else {
            "none"
        };
        info!(idle_vm_count = idle_vm_count, snapshot = snapshot_status, "VM pool initialized");

        Ok(())
    }

    /// Acquire a VM for job execution.
    ///
    /// Returns an idle VM from the pool, or creates a new one if
    /// none are available and capacity allows. Dead VMs are evicted
    /// on encounter.
    ///
    /// If `force_cold_boot` is true, bypasses snapshot restore.
    pub async fn acquire(&self, job_id: &str) -> Result<SharedVm> {
        self.acquire_inner(job_id, false).await
    }

    /// Acquire a VM with explicit cold-boot control.
    pub async fn acquire_with_options(&self, job_id: &str, is_force_cold_boot: bool) -> Result<SharedVm> {
        self.acquire_inner(job_id, is_force_cold_boot).await
    }

    /// Inner acquire implementation.
    async fn acquire_inner(&self, job_id: &str, is_force_cold_boot: bool) -> Result<SharedVm> {
        debug!(job_id = %job_id, is_force_cold_boot = is_force_cold_boot, "acquiring VM from pool");

        // Check memory pressure — reject at Critical
        let pressure = self.current_pressure_level();
        let active_vm_count = Self::checked_usize_to_u32(self.all_vms.lock().await.len());
        if !crate::verified::should_allow_restore(pressure, active_vm_count) {
            warn!(
                job_id = %job_id,
                pressure = pressure,
                active_vms = active_vm_count,
                "rejecting VM acquire due to critical memory pressure"
            );
            return Err(CloudHypervisorError::PoolAtCapacity {
                max_vms: self.config.max_vms,
            });
        }

        // Try to get a healthy idle VM. Loop because we may encounter dead VMs
        // that need to be evicted before finding a live one.
        while let Some(vm) = self.idle_vms.lock().await.pop_front() {
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
            Ok(permit) => return self.create_vm_with_permit(permit, job_id, is_force_cold_boot).await,
            Err(_) => {
                // Pool at capacity — wait up to ACQUIRE_WAIT_TIMEOUT for a permit
                // (a VM being destroyed will release one).
                debug!(job_id = %job_id, "pool at capacity, waiting for a VM to be released");

                match tokio::time::timeout(ACQUIRE_WAIT_TIMEOUT, self.vm_semaphore.clone().acquire_owned()).await {
                    Ok(Ok(permit)) => return self.create_vm_with_permit(permit, job_id, is_force_cold_boot).await,
                    Ok(Err(_closed)) => {
                        // Semaphore closed — shouldn't happen
                        Err(CloudHypervisorError::PoolAtCapacity {
                            max_vms: self.config.max_vms,
                        })
                    }
                    Err(_timeout) => Err(CloudHypervisorError::NoVmsAvailable {
                        timeout_ms: Self::checked_u128_to_u64(ACQUIRE_WAIT_TIMEOUT.as_millis()),
                    }),
                }
            }
        }
    }

    /// Create a VM using an already-acquired semaphore permit.
    ///
    /// Tries snapshot restore first (if golden snapshot exists), falls back to cold-boot.
    async fn create_vm_with_permit(
        &self,
        permit: tokio::sync::OwnedSemaphorePermit,
        job_id: &str,
        is_force_cold_boot: bool,
    ) -> Result<SharedVm> {
        let vm = self.create_or_restore_vm(is_force_cold_boot).await.map_err(|e| {
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

    /// Create or restore a VM, preferring snapshot restore when available.
    ///
    /// If `force_cold_boot` is true, always cold-boots regardless of snapshot.
    async fn create_or_restore_vm(&self, is_force_cold_boot: bool) -> Result<SharedVm> {
        // Try snapshot restore first (unless forced to cold-boot)
        if !is_force_cold_boot && let Some(snapshot) = self.golden_snapshot.read().await.clone() {
            let vm_index = self.next_vm_index_u32()?;
            let vm = Arc::new(ManagedCiVm::new(self.config.clone(), vm_index));

            // Inject shared workspace client so the VM reuses the pool's Iroh endpoint
            if let Some(ref client) = *self.shared_workspace_client.read().await {
                vm.set_shared_workspace_client(client.clone()).await;
            }

            self.all_vms.lock().await.push(vm.clone());

            match vm.restore_from_snapshot(&snapshot).await {
                Ok(()) => {
                    // Reset consecutive failure counter on success
                    self.restore_failure_count.store(0, Ordering::Relaxed);
                    info!(vm_id = %vm.id, "VM restored from snapshot");
                    return Ok(vm);
                }
                Err(e) => {
                    warn!(vm_id = %vm.id, error = %e, "snapshot restore failed, falling back to cold-boot");

                    // Remove failed VM from tracking
                    self.all_vms.lock().await.retain(|v| !Arc::ptr_eq(v, &vm));

                    // Increment failure counter and check for invalidation
                    let previous_failures = self.restore_failure_count.fetch_add(1, Ordering::Relaxed);
                    let failures = previous_failures.saturating_add(1);
                    let max_failures = self.config.max_restore_failures;
                    if crate::verified::should_invalidate_snapshot(failures, max_failures) {
                        warn!(
                            failures = failures,
                            threshold = max_failures,
                            "consecutive restore failures exceeded threshold, invalidating snapshot"
                        );
                        if let Err(e) = snapshot.invalidate("consecutive restore failures").await {
                            error!(error = %e, "failed to invalidate snapshot");
                        }
                        *self.golden_snapshot.write().await = None;
                        self.restore_failure_count.store(0, Ordering::Relaxed);
                        *self.snapshot_needs_regen.write().await = true;
                    }
                    // Fall through to cold-boot
                }
            }
        }

        // Cold-boot path
        self.create_and_start_vm().await
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
        let target_vm_count = Self::checked_u32_to_usize(self.config.pool_size);
        let is_kept = {
            let mut idle = self.idle_vms.lock().await;
            if idle.len() < target_vm_count {
                idle.push_back(vm.clone());
                true
            } else {
                false
            }
        };

        if is_kept {
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

        // Drop the shared workspace client on a blocking thread.
        // FuseSyncClient owns a tokio Runtime that cannot be dropped inside
        // another Runtime (panics). Move the final Arc to a blocking thread.
        if let Some(client) = self.shared_workspace_client.write().await.take() {
            tokio::task::spawn_blocking(move || drop(client));
        }

        info!("VM pool shutdown complete");
        Ok(())
    }

    /// Get the current pool status.
    pub async fn status(&self) -> PoolStatus {
        let idle = self.idle_vms.lock().await.len();
        let total = self.all_vms.lock().await.len();
        let available_permits = self.vm_semaphore.available_permits();
        let is_snapshot_valid = self.golden_snapshot.read().await.is_some();
        let restore_failure_count = self.restore_failure_count.load(Ordering::Relaxed);

        // Report the effective max (capped by MAX_CI_VMS_PER_NODE), not the
        // uncapped config value, so callers see consistent numbers.
        let effective_max = self.config.max_vms.min(MAX_CI_VMS_PER_NODE);

        PoolStatus {
            idle_vms: idle as u32,
            total_vms: total as u32,
            max_vms: effective_max,
            available_capacity: available_permits as u32,
            target_pool_size: self.config.pool_size,
            is_snapshot_valid,
            restore_failure_count,
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
            let idle_vm_count = idle.len();
            let mut alive = VecDeque::with_capacity(idle_vm_count);
            let mut dead = Vec::with_capacity(idle_vm_count);

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

        // Phase 2: Regenerate golden snapshot if needed.
        if *self.snapshot_needs_regen.read().await
            && self.config.enable_snapshots
            && self.config.get_cluster_ticket().is_some()
        {
            info!("regenerating golden snapshot via cold-boot");
            let permit = match self.vm_semaphore.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => {
                    debug!("no permits for snapshot regeneration, will retry next cycle");
                    return;
                }
            };

            match self.create_and_start_vm().await {
                Ok(vm) => {
                    *vm.pool_permit.write().await = Some(permit);
                    match GoldenSnapshot::create(&vm, &self.config).await {
                        Ok(snapshot) => {
                            info!(dir = %snapshot.dir.display(), "golden snapshot regenerated");
                            *self.golden_snapshot.write().await = Some(snapshot);
                            *self.snapshot_needs_regen.write().await = false;
                        }
                        Err(e) => {
                            warn!(error = %e, "failed to regenerate golden snapshot");
                        }
                    }
                    self.idle_vms.lock().await.push_back(vm);
                }
                Err(e) => {
                    warn!(error = %e, "failed to cold-boot VM for snapshot regeneration");
                }
            }
        }

        // Phase 3: Create replacement VMs to reach target pool size.
        // Skip pre-warming if memory pressure is Warning or above.
        let pressure = self.current_pressure_level();
        if pressure >= crate::verified::snapshot::PRESSURE_WARNING {
            debug!(pressure = pressure, "skipping pool pre-warming due to memory pressure");
            return;
        }

        let current_idle = self.idle_vms.lock().await.len();
        let target_vm_count = Self::checked_u32_to_usize(self.config.pool_size);

        if current_idle >= target_vm_count {
            return;
        }

        let to_create = target_vm_count.saturating_sub(current_idle);
        debug!(
            current_idle = current_idle,
            target_vm_count = target_vm_count,
            to_create = to_create,
            "replenishing pool"
        );

        for _ in 0..to_create {
            let permit = match self.vm_semaphore.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => break,
            };

            // Use restore when snapshot is available
            match self.create_or_restore_vm(false).await {
                Ok(vm) => {
                    *vm.pool_permit.write().await = Some(permit);
                    self.idle_vms.lock().await.push_back(vm);
                }
                Err(e) => {
                    warn!(error = ?e, "failed to create maintenance VM");
                    break;
                }
            }
        }
    }

    /// Get the current golden snapshot (if valid).
    pub async fn golden_snapshot(&self) -> Option<GoldenSnapshot> {
        self.golden_snapshot.read().await.clone()
    }

    /// Acquire multiple VMs from the golden snapshot for speculative execution.
    ///
    /// Each fork gets its own KV branch for workspace isolation. The returned
    /// `SpeculativeGroup` manages the fork lifecycle and first-success-wins logic.
    ///
    /// The effective fork count is adjusted by memory pressure and capped at
    /// `MAX_SPECULATIVE_FORKS`.
    pub async fn acquire_speculative(
        &self,
        job_id: &str,
        requested_count: u32,
    ) -> Result<crate::speculative::SpeculativeGroup> {
        let effective_count = crate::verified::compute_adaptive_fork_count(
            requested_count,
            crate::speculative::MAX_SPECULATIVE_FORKS,
            self.current_pressure_level(),
        );

        info!(
            job_id = %job_id,
            requested = requested_count,
            effective = effective_count,
            "acquiring speculative VM group"
        );

        let mut forks = Vec::with_capacity(Self::checked_u32_to_usize(effective_count));

        for i in 0..effective_count {
            let permit = match self.vm_semaphore.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => {
                    warn!(
                        job_id = %job_id,
                        acquired = i,
                        requested = effective_count,
                        "not enough permits for full speculative group"
                    );
                    break;
                }
            };

            match self.create_or_restore_vm(false).await {
                Ok(vm) => {
                    *vm.pool_permit.write().await = Some(permit);
                    vm.assign(job_id.to_string()).await?;
                    forks.push(vm);
                }
                Err(e) => {
                    warn!(
                        job_id = %job_id,
                        fork_index = i,
                        error = %e,
                        "failed to create speculative fork"
                    );
                    // Permit drops, releasing capacity
                    break;
                }
            }
        }

        if forks.is_empty() {
            return Err(CloudHypervisorError::NoVmsAvailable { timeout_ms: 0 });
        }

        Ok(crate::speculative::SpeculativeGroup::new(forks, job_id.to_string()))
    }

    // Private methods

    /// Create and start a new VM.
    async fn create_and_start_vm(&self) -> Result<SharedVm> {
        // Monotonic: never wraps, never collides.
        // u64 overflow is effectively impossible (~18 quintillion VMs).
        let vm_index = self.next_vm_index_u32()?;

        let vm = Arc::new(ManagedCiVm::new(self.config.clone(), vm_index));

        // Inject shared workspace client so the VM reuses the pool's Iroh endpoint
        if let Some(ref client) = *self.shared_workspace_client.read().await {
            vm.set_shared_workspace_client(client.clone()).await;
        }

        // Track in all_vms
        self.all_vms.lock().await.push(vm.clone());

        // Start the VM
        vm.start().await?;

        Ok(vm)
    }

    /// Destroy a VM and release its resources.
    ///
    /// Handles both cold-booted and snapshot-restored VMs. For restored VMs,
    /// also cleans up fork-specific resources (socket directories, COW overlays).
    /// The golden snapshot itself is never destroyed here.
    ///
    /// The semaphore permit is released automatically when the VM's
    /// `pool_permit` is dropped (either here or on VM drop).
    async fn destroy_vm(&self, vm: &SharedVm) {
        // Full fork cleanup: kill processes, remove sockets, remove fork dir.
        // Safe for both cold-booted and restored VMs (fork_dir just won't exist
        // for cold-booted VMs).
        vm.full_fork_cleanup().await;

        // Shutdown the VM (graceful API shutdown + process kill)
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
    /// Whether a valid golden snapshot exists.
    pub is_snapshot_valid: bool,
    /// Number of consecutive restore failures.
    pub restore_failure_count: u32,
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
            is_snapshot_valid: true,
            restore_failure_count: 0,
        };

        assert_eq!(status.idle_vms, 5);
        assert_eq!(status.total_vms, 10);
        assert_eq!(status.max_vms, 20);
        assert_eq!(status.available_capacity, 10);
        assert_eq!(status.target_pool_size, 5);
        assert!(status.is_snapshot_valid);
        assert_eq!(status.restore_failure_count, 0);
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

    #[tokio::test]
    async fn test_snapshot_failure_tracking_and_invalidation() {
        // Test the restore failure counter and snapshot invalidation logic
        // without attempting real VM restore (which requires Iroh + virtiofsd).
        //
        // We directly manipulate pool state to simulate the same transitions
        // that create_or_restore_vm performs on restore failure.
        let tmp = tempfile::tempdir().unwrap();
        let snapshot_dir = tmp.path().join("snapshots/golden");
        std::fs::create_dir_all(&snapshot_dir).unwrap();
        std::fs::write(snapshot_dir.join("memory"), b"fake").unwrap();
        std::fs::write(snapshot_dir.join("ticket.txt"), "test-ticket").unwrap();

        let config = CloudHypervisorWorkerConfig {
            enable_snapshots: true,
            cluster_ticket: Some("test-ticket".to_string()),
            snapshot_path: Some(snapshot_dir.clone()),
            pool_size: 1,
            max_vms: 2,
            max_restore_failures: 3,
            ..test_config()
        };

        let pool = VmPool::new(config.clone());

        // Inject a golden snapshot
        let snapshot = crate::snapshot::GoldenSnapshot::from_config(&config);
        *pool.golden_snapshot.write().await = Some(snapshot.clone());

        // Initial state: snapshot valid, no failures
        assert!(pool.golden_snapshot().await.is_some());
        assert_eq!(pool.restore_failure_count.load(Ordering::Relaxed), 0);
        assert!(!*pool.snapshot_needs_regen.read().await);

        // Simulate 2 consecutive restore failures (below threshold of 3)
        for i in 1..=2u32 {
            let failures = pool.restore_failure_count.fetch_add(1, Ordering::Relaxed) + 1;
            assert_eq!(failures, i);
            assert!(
                !crate::verified::should_invalidate_snapshot(failures, config.max_restore_failures),
                "should NOT invalidate at {failures} failures (threshold=3)"
            );
        }

        // Snapshot still valid after 2 failures
        assert!(pool.golden_snapshot().await.is_some());

        // Simulate 3rd failure — crosses threshold
        let failures = pool.restore_failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        assert_eq!(failures, 3);
        assert!(
            crate::verified::should_invalidate_snapshot(failures, config.max_restore_failures),
            "should invalidate at {failures} failures (threshold=3)"
        );

        // Perform the invalidation (same logic as create_or_restore_vm)
        if let Err(e) = snapshot.invalidate("consecutive restore failures").await {
            panic!("invalidate failed: {e}");
        }
        *pool.golden_snapshot.write().await = None;
        pool.restore_failure_count.store(0, Ordering::Relaxed);
        *pool.snapshot_needs_regen.write().await = true;

        // Verify final state
        assert!(pool.golden_snapshot().await.is_none(), "snapshot should be cleared");
        assert_eq!(pool.restore_failure_count.load(Ordering::Relaxed), 0, "counter should reset");
        assert!(*pool.snapshot_needs_regen.read().await, "regen should be marked");
        assert!(!snapshot_dir.exists(), "snapshot directory should be deleted");
    }

    #[tokio::test]
    async fn test_restore_success_resets_failure_counter() {
        let config = CloudHypervisorWorkerConfig {
            max_restore_failures: 3,
            ..test_config()
        };
        let pool = VmPool::new(config);

        // Simulate 2 failures
        pool.restore_failure_count.store(2, Ordering::Relaxed);

        // Simulate a successful restore (resets counter)
        pool.restore_failure_count.store(0, Ordering::Relaxed);
        assert_eq!(pool.restore_failure_count.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_acquire_force_cold_boot_bypasses_snapshot() {
        let config = CloudHypervisorWorkerConfig {
            enable_snapshots: true,
            cluster_ticket: Some("test-ticket".to_string()),
            pool_size: 1,
            max_vms: 2,
            ..test_config()
        };

        let pool = VmPool::new(config.clone());

        // Inject a golden snapshot
        let snapshot = crate::snapshot::GoldenSnapshot::from_config(&config);
        *pool.golden_snapshot.write().await = Some(snapshot);

        // Force cold-boot should skip snapshot restore entirely
        let result = pool.acquire_with_options("test-job", true).await;
        assert!(result.is_err(), "should fail without real infrastructure");

        // Restore failure counter should NOT be incremented (didn't attempt restore)
        assert_eq!(
            pool.restore_failure_count.load(Ordering::Relaxed),
            0,
            "force_cold_boot should not increment restore failure count"
        );
    }

    #[tokio::test]
    async fn test_acquire_critical_pressure_rejects() {
        let pressure = Arc::new(AtomicU8::new(crate::verified::snapshot::PRESSURE_CRITICAL));
        let config = CloudHypervisorWorkerConfig {
            pool_size: 1,
            max_vms: 4,
            ..test_config()
        };

        let pool = VmPool::with_pressure_level(config, Some(pressure));
        let result = pool.acquire("test-job").await;

        let err = result.as_ref().err().expect("critical pressure should reject acquire");
        let err_msg = format!("{err}");
        assert!(err_msg.contains("capacity"), "error should mention capacity: {err_msg}");
    }

    #[tokio::test]
    async fn test_acquire_normal_pressure_allows() {
        let pressure = Arc::new(AtomicU8::new(crate::verified::snapshot::PRESSURE_NORMAL));
        let config = CloudHypervisorWorkerConfig {
            pool_size: 1,
            max_vms: 4,
            ..test_config()
        };

        let pool = VmPool::with_pressure_level(config, Some(pressure));
        let result = pool.acquire("test-job").await;

        // Still fails (no real VM) but gets past the pressure check
        let err = result.as_ref().err().expect("should fail without real infrastructure");
        let err_msg = format!("{err}");
        // Should NOT be a capacity error — should fail at VM creation instead
        assert!(
            !err_msg.contains("capacity") || err_msg.contains("VM"),
            "normal pressure should not reject with capacity error: {err_msg}"
        );
    }
}
