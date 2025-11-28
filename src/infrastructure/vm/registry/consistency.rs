// VM Consistency Checker - Periodic verification
//
// Compares database state with cache and indices, auto-repairs inconsistencies.
// Database is always the source of truth in conflicts.

use anyhow::Result;
use std::sync::Arc;

use super::cache::VmCache;
use super::index::VmStateIndex;
use super::persistence::VmPersistence;

pub struct VmConsistency<P, C, I>
where
    P: VmPersistence,
    C: VmCache,
    I: VmStateIndex,
{
    persistence: Arc<P>,
    cache: Arc<C>,
    index: Arc<I>,
}

#[derive(Debug, Clone)]
pub struct ConsistencyReport {
    pub checked: usize,
    pub found: usize,
    pub fixed: usize,
}

impl<P, C, I> VmConsistency<P, C, I>
where
    P: VmPersistence,
    C: VmCache,
    I: VmStateIndex,
{
    pub fn new(persistence: Arc<P>, cache: Arc<C>, index: Arc<I>) -> Self {
        Self {
            persistence,
            cache,
            index,
        }
    }

    /// Verify consistency between database and cache/indices
    pub async fn verify(&self) -> Result<ConsistencyReport> {
        let db_vms = self.persistence.load_all().await?;
        let mut checked = 0;
        let mut found = 0;
        let mut fixed = 0;

        for db_vm in db_vms {
            checked += 1;
            let vm_id = db_vm.config.id;

            if let Some(cached_vm_arc) = self.cache.get(&vm_id) {
                found += 1;

                // Check if states match
                let cached_vm = cached_vm_arc.read().await;
                if cached_vm.state != db_vm.state {
                    tracing::warn!(
                        vm_id = %vm_id,
                        db_state = ?db_vm.state,
                        cache_state = ?cached_vm.state,
                        "State mismatch detected - fixing from database"
                    );

                    drop(cached_vm); // Release read lock
                    let mut cached_vm = cached_vm_arc.write().await;

                    // Update cache to match database (source of truth)
                    let old_state = cached_vm.state.clone();
                    cached_vm.state = db_vm.state.clone();

                    // Update index
                    self.index.update(vm_id, &old_state, &db_vm.state);

                    fixed += 1;
                }
            } else {
                // VM in database but not cache - add it
                tracing::warn!(
                    vm_id = %vm_id,
                    "VM in database but not cache - adding"
                );

                self.cache.insert(db_vm.clone());
                self.index.add(vm_id, &db_vm.state);
                fixed += 1;
            }
        }

        tracing::info!(
            checked,
            found,
            fixed,
            "Consistency verification complete"
        );

        Ok(ConsistencyReport {
            checked,
            found,
            fixed,
        })
    }
}
