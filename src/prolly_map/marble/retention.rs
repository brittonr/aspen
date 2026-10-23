// The retention operations of the marble store: pins, the retention
// inventory, and compare-and-delete, plus maintenance and statistics.
//
// Pins and the retention generation live in the catalog object, so a pin
// change or a publication is one crash-atomic batch. A deletion compares the
// current inventory with the admission's inventory first, then removes the
// candidates' objects in one batch.

use molten_core::marble_store::*;
use molten_core::prolly_map::*;

use super::super::store::decode_head;
use super::super::*;
use super::backend::*;

/// Observed sizes of one marble store, for measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarbleStoreStatistics {
    pub heap_bytes: u64,
    pub live_objects: u64,
    pub stored_objects: u64,
    pub indexed_blocks: u64,
    pub is_poisoned: bool,
}

impl MarbleBackend {
    pub(super) fn pin_blocks(&mut self, node_refs: &[NodeRef]) -> ProllyPortResult<()> {
        self.ensure_serving()?;
        let mut catalog = self.catalog.clone();
        for node_ref in node_refs {
            catalog
                .pins
                .insert(digest_of_node_ref(node_ref).map_err(|issue| issue_error("prolly-marble-node-ref", &issue))?);
        }
        if catalog == self.catalog {
            return Ok(());
        }
        self.write_catalog(catalog, ProllyPortPhase::Staging)
    }

    pub(super) fn release_pins(&mut self, node_refs: &[NodeRef]) -> ProllyPortResult<()> {
        self.ensure_serving()?;
        let mut catalog = self.catalog.clone();
        for digest in node_refs.iter().filter_map(|node_ref| digest_of_node_ref(node_ref).ok()) {
            catalog.pins.remove(&digest);
        }
        if catalog == self.catalog {
            return Ok(());
        }
        self.write_catalog(catalog, ProllyPortPhase::Publication)
    }

    pub(super) fn retention_inventory(&self) -> ProllyPortResult<ProllyRetentionInventory> {
        self.ensure_serving()?;
        let mut roots = Vec::with_capacity(self.catalog.heads.len());
        for head in self.catalog.heads.values() {
            roots.push(decode_head(head)?.root.top_node_ref);
        }
        roots.sort();
        roots.dedup();
        Ok(ProllyRetentionInventory {
            roots,
            pins: self.catalog.pins.iter().map(node_ref_of_digest).collect(),
            retention_generation: self.catalog.retention_generation,
        })
    }

    /// Compare the current inventory with the admission's, then delete.
    pub(super) fn compare_retention_inventory_and_delete(
        &mut self,
        expected: &ProllyRetentionInventory,
        candidates: Vec<NodeRef>,
    ) -> ProllyPortResult<GcDeletionOutcome> {
        let current = self.retention_inventory()?;
        let mismatch = if current.roots != expected.roots {
            Some("published-roots-changed")
        } else if current.retention_generation != expected.retention_generation {
            Some("retention-generation-changed")
        } else if !expected.pins.iter().all(|pin| current.pins.contains(pin)) {
            Some("expected-pin-missing")
        } else if candidates.iter().any(|candidate| current.pins.contains(candidate)) {
            Some("candidate-is-pinned")
        } else {
            None
        };
        if let Some(reason) = mismatch {
            return Ok(GcDeletionOutcome::InventoryMismatch { reason });
        }
        let mut deletions = Vec::with_capacity(candidates.len());
        for digest in candidates.iter().filter_map(|candidate| digest_of_node_ref(candidate).ok()) {
            if let Some(object_id) = self.index.get(&digest).copied() {
                deletions.push((digest, object_id));
            }
        }
        if !deletions.is_empty() {
            self.write_objects(
                deletions.iter().map(|(_, object_id)| (*object_id, None::<&[u8]>)),
                ProllyPortPhase::Deletion,
            )?;
            for (digest, _) in &deletions {
                self.index.remove(digest);
                self.cache.evict(digest);
            }
            self.count_committed_batch();
        }
        Ok(GcDeletionOutcome::Deleted { deleted: candidates })
    }

    /// Defragment the heap now; returns the rewritten object count.
    pub(super) fn maintain(&mut self) -> ProllyPortResult<u64> {
        self.ensure_serving()?;
        self.batches_since_maintenance = 0;
        let rewritten = self.heap.maintenance().map_err(|error| {
            self.is_poisoned = true;
            heap_error(ProllyPortPhase::Staging, ProllyPortFailureKind::Commit, error)
        })?;
        u64::try_from(rewritten)
            .map_err(|_| ProllyPortError::new("prolly-marble-maintenance", "rewritten count overflows", false))
    }

    pub(super) fn statistics(&self) -> ProllyPortResult<MarbleStoreStatistics> {
        let stats = self.heap.stats();
        let indexed_blocks = u64::try_from(self.index.len())
            .map_err(|_| ProllyPortError::new("prolly-marble-statistics", "index size overflows", false))?;
        Ok(MarbleStoreStatistics {
            heap_bytes: stats.total_file_size,
            live_objects: stats.live_objects,
            stored_objects: stats.stored_objects,
            indexed_blocks,
            is_poisoned: self.is_poisoned,
        })
    }
}
