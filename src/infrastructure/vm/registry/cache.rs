// VM Cache - In-memory storage for fast access
//
// Provides concurrent access to VM instances using DashMap.
// Each VM is wrapped in Arc<RwLock> for safe concurrent modification.

use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::infrastructure::vm::vm_types::VmInstance;

/// Trait for in-memory VM cache
pub trait VmCache: Send + Sync {
    /// Insert a VM into the cache, returns Arc for sharing
    fn insert(&self, vm: VmInstance) -> Arc<RwLock<VmInstance>>;

    /// Get a VM by ID
    fn get(&self, vm_id: &Uuid) -> Option<Arc<RwLock<VmInstance>>>;

    /// Remove a VM from cache
    fn remove(&self, vm_id: &Uuid) -> Option<Arc<RwLock<VmInstance>>>;

    /// Get all cached VMs
    fn get_all(&self) -> Vec<Arc<RwLock<VmInstance>>>;

    /// Count cached VMs
    fn count(&self) -> usize;

    /// Clear all VMs from cache
    fn clear(&self);
}

/// DashMap-based concurrent VM cache
pub struct DashMapVmCache {
    vms: DashMap<Uuid, Arc<RwLock<VmInstance>>>,
}

impl DashMapVmCache {
    pub fn new() -> Self {
        Self {
            vms: DashMap::new(),
        }
    }
}

impl VmCache for DashMapVmCache {
    fn insert(&self, vm: VmInstance) -> Arc<RwLock<VmInstance>> {
        let vm_id = vm.config.id;
        let vm_arc = Arc::new(RwLock::new(vm));
        self.vms.insert(vm_id, vm_arc.clone());
        vm_arc
    }

    fn get(&self, vm_id: &Uuid) -> Option<Arc<RwLock<VmInstance>>> {
        self.vms.get(vm_id).map(|entry| entry.value().clone())
    }

    fn remove(&self, vm_id: &Uuid) -> Option<Arc<RwLock<VmInstance>>> {
        self.vms.remove(vm_id).map(|(_, vm)| vm)
    }

    fn get_all(&self) -> Vec<Arc<RwLock<VmInstance>>> {
        self.vms.iter().map(|entry| entry.value().clone()).collect()
    }

    fn count(&self) -> usize {
        self.vms.len()
    }

    fn clear(&self) {
        self.vms.clear();
    }
}

impl Default for DashMapVmCache {
    fn default() -> Self {
        Self::new()
    }
}
