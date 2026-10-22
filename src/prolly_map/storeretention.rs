// The retention boundary of the local Prolly block store.
//
// The pins, the retention generation, and the compare-and-delete operation
// live apart from the block and head operations: the deletion boundary
// compares the current durable inventory and deletes in one write transaction,
// so a stale admission cannot remove content that became reachable or pinned.

use molten_core::prolly_map::*;
use redb::ReadableDatabase;
use redb::ReadableTable;

use super::store::*;
use super::*;

impl ProllyRetentionPort for LocalProllyBlockStore {
    fn pin_blocks(&mut self, node_refs: &[NodeRef]) -> ProllyPortResult<()> {
        let write = self
            .database
            .begin_write()
            .map_err(|error| redb_error(ProllyPortPhase::Staging, ProllyPortFailureKind::Write, error))?;
        {
            let mut pins = write
                .open_table(PROLLY_PINS_TABLE)
                .map_err(|error| redb_error(ProllyPortPhase::Staging, ProllyPortFailureKind::Write, error))?;
            for node_ref in node_refs {
                pins.insert(node_ref.as_str(), b"".as_slice())
                    .map_err(|error| redb_error(ProllyPortPhase::Staging, ProllyPortFailureKind::Write, error))?;
            }
        }
        write
            .commit()
            .map_err(|error| redb_error(ProllyPortPhase::Staging, ProllyPortFailureKind::Commit, error))
    }

    fn release_pins(&mut self, node_refs: &[NodeRef]) -> ProllyPortResult<()> {
        let write = self
            .database
            .begin_write()
            .map_err(|error| redb_error(ProllyPortPhase::Publication, ProllyPortFailureKind::Write, error))?;
        {
            let mut pins = write
                .open_table(PROLLY_PINS_TABLE)
                .map_err(|error| redb_error(ProllyPortPhase::Publication, ProllyPortFailureKind::Write, error))?;
            for node_ref in node_refs {
                pins.remove(node_ref.as_str())
                    .map_err(|error| redb_error(ProllyPortPhase::Publication, ProllyPortFailureKind::Write, error))?;
            }
        }
        write
            .commit()
            .map_err(|error| redb_error(ProllyPortPhase::Publication, ProllyPortFailureKind::Commit, error))
    }

    fn retention_inventory(&self) -> ProllyPortResult<ProllyRetentionInventory> {
        let read = self
            .database
            .begin_read()
            .map_err(|error| redb_error(ProllyPortPhase::Readback, ProllyPortFailureKind::Read, error))?;
        let heads = read
            .open_table(PROLLY_HEADS_TABLE)
            .map_err(|error| redb_error(ProllyPortPhase::Readback, ProllyPortFailureKind::Read, error))?;
        let mut roots = Vec::with_capacity(MAX_HEADS);
        for head in heads
            .iter()
            .map_err(|error| redb_error(ProllyPortPhase::Readback, ProllyPortFailureKind::Read, error))?
        {
            let (_key, value) =
                head.map_err(|error| redb_error(ProllyPortPhase::Readback, ProllyPortFailureKind::Read, error))?;
            let head =
                decode_head(value.value()).map_err(|error| ProllyPortError::new(error.code, error.detail, false))?;
            roots.push(head.root.top_node_ref);
        }
        roots.sort();
        roots.dedup();
        let pins = read
            .open_table(PROLLY_PINS_TABLE)
            .map_err(|error| redb_error(ProllyPortPhase::Readback, ProllyPortFailureKind::Read, error))?;
        let mut pinned = Vec::with_capacity(MAX_PINS);
        for pin in pins
            .iter()
            .map_err(|error| redb_error(ProllyPortPhase::Readback, ProllyPortFailureKind::Read, error))?
        {
            let (key, _value) =
                pin.map_err(|error| redb_error(ProllyPortPhase::Readback, ProllyPortFailureKind::Read, error))?;
            pinned.push(NodeRef::new(key.value().to_string()));
        }
        pinned.sort();
        pinned.dedup();
        let meta = read
            .open_table(PROLLY_META_TABLE)
            .map_err(|error| redb_error(ProllyPortPhase::Readback, ProllyPortFailureKind::Read, error))?;
        let retention_generation = meta
            .get(PROLLY_RETENTION_GENERATION_KEY)
            .map_err(|error| redb_error(ProllyPortPhase::Readback, ProllyPortFailureKind::Read, error))?
            .map(|value| value.value())
            .unwrap_or(0);
        Ok(ProllyRetentionInventory {
            roots,
            pins: pinned,
            retention_generation,
        })
    }

    fn compare_retention_inventory_and_delete(
        &mut self,
        expected: &ProllyRetentionInventory,
        candidates: &[NodeRef],
    ) -> ProllyPortResult<GcDeletionOutcome> {
        let write = self
            .database
            .begin_write()
            .map_err(|error| redb_error(ProllyPortPhase::Deletion, ProllyPortFailureKind::Write, error))?;
        {
            let heads = write
                .open_table(PROLLY_HEADS_TABLE)
                .map_err(|error| redb_error(ProllyPortPhase::Deletion, ProllyPortFailureKind::Read, error))?;
            let mut roots = Vec::with_capacity(MAX_HEADS);
            for head in heads
                .iter()
                .map_err(|error| redb_error(ProllyPortPhase::Deletion, ProllyPortFailureKind::Read, error))?
            {
                let (_key, value) =
                    head.map_err(|error| redb_error(ProllyPortPhase::Deletion, ProllyPortFailureKind::Read, error))?;
                let head = decode_head(value.value())
                    .map_err(|error| ProllyPortError::new(error.code, error.detail, false))?;
                roots.push(head.root.top_node_ref);
            }
            roots.sort();
            roots.dedup();
            if roots != expected.roots {
                return Ok(GcDeletionOutcome::InventoryMismatch {
                    reason: "published-roots-changed",
                });
            }
            let meta = write
                .open_table(PROLLY_META_TABLE)
                .map_err(|error| redb_error(ProllyPortPhase::Deletion, ProllyPortFailureKind::Read, error))?;
            let retention_generation = meta
                .get(PROLLY_RETENTION_GENERATION_KEY)
                .map_err(|error| redb_error(ProllyPortPhase::Deletion, ProllyPortFailureKind::Read, error))?
                .map(|value| value.value())
                .unwrap_or(0);
            if retention_generation != expected.retention_generation {
                return Ok(GcDeletionOutcome::InventoryMismatch {
                    reason: "retention-generation-changed",
                });
            }
            let pins = write
                .open_table(PROLLY_PINS_TABLE)
                .map_err(|error| redb_error(ProllyPortPhase::Deletion, ProllyPortFailureKind::Read, error))?;
            let mut pinned = Vec::with_capacity(MAX_PINS);
            for pin in pins
                .iter()
                .map_err(|error| redb_error(ProllyPortPhase::Deletion, ProllyPortFailureKind::Read, error))?
            {
                let (key, _value) =
                    pin.map_err(|error| redb_error(ProllyPortPhase::Deletion, ProllyPortFailureKind::Read, error))?;
                pinned.push(NodeRef::new(key.value().to_string()));
            }
            pinned.sort();
            pinned.dedup();
            if !expected.pins.iter().all(|pin| pinned.contains(pin)) {
                return Ok(GcDeletionOutcome::InventoryMismatch {
                    reason: "expected-pin-missing",
                });
            }
            if candidates.iter().any(|candidate| pinned.contains(candidate)) {
                return Ok(GcDeletionOutcome::InventoryMismatch {
                    reason: "candidate-is-pinned",
                });
            }
            let mut blocks = write
                .open_table(PROLLY_BLOCKS_TABLE)
                .map_err(|error| redb_error(ProllyPortPhase::Deletion, ProllyPortFailureKind::Write, error))?;
            for node_ref in candidates {
                blocks
                    .remove(node_ref.as_str())
                    .map_err(|error| redb_error(ProllyPortPhase::Deletion, ProllyPortFailureKind::Write, error))?;
            }
        }
        write
            .commit()
            .map_err(|error| redb_error(ProllyPortPhase::Deletion, ProllyPortFailureKind::Commit, error))?;
        Ok(GcDeletionOutcome::Deleted {
            deleted: candidates.to_vec(),
        })
    }
}
