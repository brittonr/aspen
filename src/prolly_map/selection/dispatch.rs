// The selected store serves both Prolly ports by delegating to its backend.

use molten_core::prolly_map::*;

use super::super::*;
use super::SelectedProllyStore;

macro_rules! dispatch {
    ($selected:expr, $store:ident => $call:expr) => {
        match $selected {
            SelectedProllyStore::Redb($store) => $call,
            #[cfg(feature = "marble-object-store")]
            SelectedProllyStore::MarbleArt($store) => $call,
        }
    };
}

impl ProllyBlockStorePort for SelectedProllyStore {
    fn read_block(&self, node_ref: &NodeRef) -> ProllyPortResult<Option<Vec<u8>>> {
        dispatch!(self, store => store.read_block(node_ref))
    }

    fn open_read_session(&self, bounds: ReadSessionBounds) -> ProllyPortResult<Box<dyn ProllyReadSession + '_>> {
        dispatch!(self, store => store.open_read_session(bounds))
    }

    fn stage_blocks(&mut self, blocks: &[EncodedBlock]) -> ProllyPortResult<()> {
        dispatch!(self, store => store.stage_blocks(blocks))
    }

    fn read_root(&self, map_id: &str) -> ProllyPortResult<Option<PublishedProllyRoot>> {
        dispatch!(self, store => store.read_root(map_id))
    }

    fn compare_and_advance(
        &mut self,
        map_id: &str,
        expected: &ExpectedProllyRoot,
        next: &PublishedProllyRoot,
    ) -> ProllyPortResult<ProllyPublicationObservation> {
        dispatch!(self, store => store.compare_and_advance(map_id, expected, next))
    }
}

impl ProllyRetentionPort for SelectedProllyStore {
    fn pin_blocks(&mut self, node_refs: &[NodeRef]) -> ProllyPortResult<()> {
        dispatch!(self, store => store.pin_blocks(node_refs))
    }

    fn release_pins(&mut self, node_refs: &[NodeRef]) -> ProllyPortResult<()> {
        dispatch!(self, store => store.release_pins(node_refs))
    }

    fn retention_inventory(&self) -> ProllyPortResult<ProllyRetentionInventory> {
        dispatch!(self, store => store.retention_inventory())
    }

    fn compare_retention_inventory_and_delete(
        &mut self,
        expected: &ProllyRetentionInventory,
        candidates: &[NodeRef],
    ) -> ProllyPortResult<GcDeletionOutcome> {
        dispatch!(self, store => store.compare_retention_inventory_and_delete(expected, candidates))
    }
}
