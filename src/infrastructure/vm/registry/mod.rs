// VM Registry Module - Trait-based VM repository
//
// Clean architecture with focused components:
// - VmPersistence: Database operations
// - VmCache: In-memory storage
// - VmStateIndex: Query optimization
// - VmRecovery: Startup recovery
// - VmConsistency: Verification

mod cache;
mod consistency;
mod hiqlite_persistence;
mod index;
mod persistence;
mod recovery;

pub use cache::{DashMapVmCache, VmCache};
pub use consistency::{ConsistencyReport, VmConsistency};
pub use hiqlite_persistence::HiqliteVmPersistence;
pub use index::{StateIndexImpl, VmStateIndex};
pub use persistence::VmPersistence;
pub use recovery::VmRecovery;

use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::hiqlite::HiqliteService;
use crate::infrastructure::vm::vm_types::{VmInstance, VmMetrics, VmState};

/// VM Repository coordinating all components
pub struct VmRepository<P, C, I>
where
    P: VmPersistence,
    C: VmCache,
    I: VmStateIndex,
{
    persistence: Arc<P>,
    cache: Arc<C>,
    index: Arc<I>,
    _recovery: VmRecovery<P, C, I>,
    consistency: VmConsistency<P, C, I>,
}

impl<P, C, I> VmRepository<P, C, I>
where
    P: VmPersistence + 'static,
    C: VmCache + 'static,
    I: VmStateIndex + 'static,
{
    /// Create new repository and recover from persistence
    pub async fn new(persistence: Arc<P>, cache: Arc<C>, index: Arc<I>) -> Result<Self> {
        let recovery = VmRecovery::new(
            persistence.clone(),
            cache.clone(),
            index.clone(),
        );

        // Recover on startup
        recovery.recover().await?;

        let consistency = VmConsistency::new(
            persistence.clone(),
            cache.clone(),
            index.clone(),
        );

        Ok(Self {
            persistence,
            cache,
            index,
            _recovery: recovery,
            consistency,
        })
    }

    /// Register a new VM
    pub async fn register(&self, vm: VmInstance) -> Result<()> {
        let vm_id = vm.config.id;

        // Phase 1: Persist to database
        self.persistence.save(&vm).await?;

        // Phase 2: Add to cache
        self.cache.insert(vm.clone());

        // Phase 3: Update index
        self.index.add(vm_id, &vm.state);

        // Phase 4: Log event (best-effort)
        let _ = self.persistence.log_event(&vm_id, "registered", None).await;

        tracing::info!(vm_id = %vm_id, state = ?vm.state, "VM registered");

        Ok(())
    }

    /// Update VM state (atomic 3-phase commit)
    pub async fn update_state(&self, vm_id: &Uuid, new_state: VmState) -> Result<()> {
        // Get VM and acquire lock
        let vm_arc = self
            .cache
            .get(vm_id)
            .ok_or_else(|| anyhow!("VM not found: {}", vm_id))?;

        let mut vm = vm_arc.write().await;
        let old_state = vm.state.clone();

        // No-op optimization
        if old_state == new_state {
            return Ok(());
        }

        // Phase 1: Persist to database (source of truth)
        self.persistence.update_state(vm_id, &new_state).await?;

        // Phase 2: Update indices (with panic guard)
        let index = self.index.clone();
        let vm_id_copy = *vm_id;
        let old_state_copy = old_state.clone();
        let new_state_copy = new_state.clone();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            index.update(vm_id_copy, &old_state_copy, &new_state_copy);
        }));

        match result {
            Ok(()) => {
                // Success - update VM instance
                vm.state = new_state.clone();
            }
            Err(_) => {
                // Panic - attempt rollback
                tracing::error!(
                    vm_id = %vm_id,
                    "Index update panicked - attempting rollback"
                );
                self.persistence.update_state(vm_id, &old_state).await?;
                return Err(anyhow!("Index update failed, rolled back"));
            }
        }

        // Phase 3: Log event (best-effort)
        let event_data = format!("{:?} -> {:?}", old_state, new_state);
        let _ = self
            .persistence
            .log_event(vm_id, "state_changed", Some(event_data))
            .await;

        tracing::debug!(
            vm_id = %vm_id,
            old_state = ?old_state,
            new_state = ?new_state,
            "VM state updated"
        );

        Ok(())
    }

    /// Remove a VM
    pub async fn remove(&self, vm_id: &Uuid) -> Result<()> {
        // Get VM to know its state
        let vm_arc = self
            .cache
            .get(vm_id)
            .ok_or_else(|| anyhow!("VM not found: {}", vm_id))?;
        let vm = vm_arc.read().await;
        let state = vm.state.clone();
        drop(vm);

        // Phase 1: Delete from database
        self.persistence.delete(vm_id).await?;

        // Phase 2: Remove from cache
        self.cache.remove(vm_id);

        // Phase 3: Remove from index
        self.index.remove(vm_id, &state);

        // Phase 4: Log event (best-effort)
        let _ = self.persistence.log_event(vm_id, "removed", None).await;

        tracing::info!(vm_id = %vm_id, "VM removed");

        Ok(())
    }

    /// Update VM metrics
    pub async fn update_metrics(&self, vm_id: &Uuid, metrics: VmMetrics) -> Result<()> {
        // Update in database
        self.persistence.update_metrics(vm_id, &metrics).await?;

        // Update in cache
        if let Some(vm_arc) = self.cache.get(vm_id) {
            let mut vm = vm_arc.write().await;
            vm.metrics = metrics;
        }

        Ok(())
    }

    // Query methods

    pub fn get(&self, vm_id: &Uuid) -> Option<Arc<RwLock<VmInstance>>> {
        self.cache.get(vm_id)
    }

    pub async fn list_all(&self) -> Vec<VmInstance> {
        let mut vms = Vec::new();
        for vm_arc in self.cache.get_all() {
            let vm = vm_arc.read().await;
            vms.push(vm.clone());
        }
        vms
    }

    pub async fn list_by_state(&self, state: &VmState) -> Vec<VmInstance> {
        let vm_ids = self.index.find_by_state(state);
        let mut vms = Vec::new();

        for vm_id in vm_ids {
            if let Some(vm_arc) = self.cache.get(&vm_id) {
                let vm = vm_arc.read().await;
                vms.push(vm.clone());
            }
        }

        vms
    }

    pub fn count_all(&self) -> usize {
        self.cache.count()
    }

    pub fn count_by_state(&self, state: &VmState) -> usize {
        self.index.count_by_state(state)
    }

    /// List all running VMs (VmState is Running or Busy)
    pub async fn list_running_vms(&self) -> Vec<VmInstance> {
        let mut running_vms = Vec::new();

        for vm_arc in self.cache.get_all() {
            let vm = vm_arc.read().await;
            if vm.state.is_running() {
                running_vms.push(vm.clone());
            }
        }

        running_vms
    }

    /// Alias for list_all() for backwards compatibility
    pub async fn list_all_vms(&self) -> Result<Vec<VmInstance>> {
        Ok(self.list_all().await)
    }

    /// Get VM by ID with Result wrapper for backwards compatibility
    pub async fn get_vm(&self, vm_id: &Uuid) -> Result<Option<Arc<RwLock<VmInstance>>>> {
        Ok(self.get(vm_id))
    }

    /// Log an event for a VM
    pub async fn log_event(&self, vm_id: Uuid, event: &str, data: Option<String>) -> Result<()> {
        self.persistence.log_event(&vm_id, event, data).await
    }

    /// Find an available service VM (Ready state, Service mode)
    pub async fn get_available_service_vm(&self) -> Option<Uuid> {
        use crate::infrastructure::vm::vm_types::VmMode;

        for vm_arc in self.cache.get_all() {
            let vm = vm_arc.read().await;

            // Check if it's a service VM in Ready state
            if matches!(vm.config.mode, VmMode::Service { .. }) && matches!(vm.state, VmState::Ready) {
                return Some(vm.config.id);
            }
        }

        None
    }

    /// List VMs by state (pattern matching variant)
    pub async fn list_vms_by_state(&self, state: VmState) -> Result<Vec<VmInstance>> {
        Ok(self.list_by_state(&state).await)
    }

    /// Recover from persistence (no-op, recovery happens in new())
    pub async fn recover_from_persistence(&self) -> Result<usize> {
        // Recovery already happened in new(), return current VM count
        Ok(self.count_all())
    }

    /// Clean up stale VMs (VMs marked as failed or terminated)
    pub async fn cleanup_stale_vms(&self) -> Result<usize> {
        let mut cleaned = 0;

        for vm_arc in self.cache.get_all() {
            let vm = vm_arc.read().await;
            let should_remove = matches!(
                vm.state,
                VmState::Failed { .. } | VmState::Terminated { .. }
            );

            if should_remove {
                let vm_id = vm.config.id;
                drop(vm); // Release lock before removal
                self.remove(&vm_id).await?;
                cleaned += 1;
            }
        }

        tracing::info!(count = cleaned, "Cleaned up stale VMs");
        Ok(cleaned)
    }

    /// Find an idle VM matching requirements (for backwards compatibility)
    pub async fn find_idle_vm(&self, _requirements: &crate::infrastructure::vm::vm_types::JobRequirements) -> Option<Uuid> {
        // Simple implementation: return first idle service VM
        // Could be enhanced to match requirements in the future
        self.get_available_service_vm().await
    }

    // Maintenance methods

    pub async fn verify_consistency(&self) -> Result<ConsistencyReport> {
        self.consistency.verify().await
    }
}

// Production type alias with concrete implementations
pub type DefaultVmRepository = VmRepository<HiqliteVmPersistence, DashMapVmCache, StateIndexImpl>;

impl DefaultVmRepository {
    /// Create default repository with Hiqlite backend
    pub async fn create(hiqlite: Arc<HiqliteService>, node_id: String) -> Result<Self> {
        let persistence = Arc::new(HiqliteVmPersistence::new(hiqlite, node_id));
        let cache = Arc::new(DashMapVmCache::new());
        let index = Arc::new(StateIndexImpl::new());

        Self::new(persistence, cache, index).await
    }
}
