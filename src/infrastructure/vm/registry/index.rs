// VM State Index - Efficient state-based queries
//
// Maintains indices for fast querying by VM state.
// Uses JSON serialization for state keys to preserve all variant data.

use dashmap::DashMap;
use std::collections::HashSet;
use uuid::Uuid;

use crate::infrastructure::vm::vm_types::VmState;

use super::cache::VmCache;

/// Trait for VM state indices
pub trait VmStateIndex: Send + Sync {
    /// Add a VM to the state index
    fn add(&self, vm_id: Uuid, state: &VmState);

    /// Remove a VM from the state index
    fn remove(&self, vm_id: &Uuid, state: &VmState);

    /// Update index when VM state changes
    fn update(&self, vm_id: Uuid, old_state: &VmState, new_state: &VmState);

    /// Find all VMs in a specific state
    fn find_by_state(&self, state: &VmState) -> Vec<Uuid>;

    /// Count VMs in a specific state
    fn count_by_state(&self, state: &VmState) -> usize;

    /// Clear all indices
    fn clear(&self);

    /// Rebuild indices from cache
    fn rebuild<C: VmCache>(&self, cache: &C);
}

/// DashMap-based state index implementation
pub struct StateIndexImpl {
    by_state: DashMap<String, HashSet<Uuid>>,
}

impl StateIndexImpl {
    pub fn new() -> Self {
        Self {
            by_state: DashMap::new(),
        }
    }

    /// Serialize state to index key
    fn state_key(state: &VmState) -> String {
        serde_json::to_string(state).unwrap_or_else(|e| {
            tracing::error!("Failed to serialize VM state for index: {}", e);
            String::from("{\"error\":\"serialization_failed\"}")
        })
    }
}

impl VmStateIndex for StateIndexImpl {
    fn add(&self, vm_id: Uuid, state: &VmState) {
        let key = Self::state_key(state);
        self.by_state
            .entry(key)
            .or_insert_with(HashSet::new)
            .insert(vm_id);
    }

    fn remove(&self, vm_id: &Uuid, state: &VmState) {
        let key = Self::state_key(state);
        if let Some(mut set) = self.by_state.get_mut(&key) {
            set.remove(vm_id);
        }
    }

    fn update(&self, vm_id: Uuid, old_state: &VmState, new_state: &VmState) {
        // Remove from old state
        self.remove(&vm_id, old_state);
        // Add to new state
        self.add(vm_id, new_state);
    }

    fn find_by_state(&self, state: &VmState) -> Vec<Uuid> {
        let key = Self::state_key(state);
        self.by_state
            .get(&key)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    fn count_by_state(&self, state: &VmState) -> usize {
        let key = Self::state_key(state);
        self.by_state.get(&key).map(|set| set.len()).unwrap_or(0)
    }

    fn clear(&self) {
        self.by_state.clear();
    }

    fn rebuild<C: VmCache>(&self, cache: &C) {
        self.clear();
        // This is a synchronous rebuild, so we can't await RwLock reads
        // The caller should ensure this is called appropriately
        // For now, we'll keep this simple and just document the limitation
        tracing::warn!("Index rebuild not fully implemented - use recovery service");
    }
}

impl Default for StateIndexImpl {
    fn default() -> Self {
        Self::new()
    }
}
