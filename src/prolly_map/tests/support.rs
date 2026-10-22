use std::collections::BTreeMap;
use std::collections::BTreeSet;

use molten_core::prolly_map::*;

use super::super::*;

pub(super) const SHELL_ENTRY_COUNT: usize = 64;
pub(super) const SHELL_VALUE_BYTES: usize = 64;
pub(super) const UPDATE_INDEX: usize = SHELL_ENTRY_COUNT / 2;
pub(super) const MAP_ID: &str = "semantic-state";
pub(super) const INITIAL_GENERATION: u64 = 0;
pub(super) const FIRST_GENERATION: u64 = 1;
pub(super) const SECOND_GENERATION: u64 = 2;

pub(super) fn profile() -> ProllyProfile {
    standard_prolly_profile().expect("profile")
}

pub(super) fn entries() -> Vec<SemanticEntry> {
    (0..SHELL_ENTRY_COUNT)
        .map(|index| SemanticEntry {
            key: format!("key-{index:04}").into_bytes(),
            value: vec![b'v'; SHELL_VALUE_BYTES],
        })
        .collect()
}

pub(super) fn initial_plan() -> EditPlan {
    let profile = profile();
    let empty = build_map(&profile, &[]).expect("empty map");
    let edits = entries().into_iter().map(MapEdit::Insert).collect::<Vec<_>>();
    plan_edits(&profile, &empty.snapshot, &edits).expect("initial plan")
}

pub(super) fn update_plan(snapshot: &MapSnapshot) -> EditPlan {
    let item = entries().get(UPDATE_INDEX).cloned().expect("update entry");
    let updated = SemanticEntry {
        key: item.key,
        value: vec![b'z'; SHELL_VALUE_BYTES],
    };
    plan_edits(&profile(), snapshot, &[MapEdit::Update(updated)]).expect("update plan")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UnknownMode {
    None,
    BeforeApply,
    AfterApply,
    /// The first staging commit fails after possible durability.
    StagingCommitUnknown,
    /// The readback fails as if the stored head were corrupt.
    CorruptReadback,
}

pub(super) struct MemoryPort {
    pub(super) blocks: BTreeMap<String, Vec<u8>>,
    pub(super) head: Option<PublishedProllyRoot>,
    pub(super) unknown_mode: UnknownMode,
    pub(super) compare_calls: u32,
    pub(super) pins: BTreeSet<String>,
    pub(super) retention_generation: u64,
    pub(super) staging_attempted: bool,
}

impl MemoryPort {
    pub(super) fn new(unknown_mode: UnknownMode) -> Self {
        Self {
            blocks: BTreeMap::new(),
            head: None,
            unknown_mode,
            compare_calls: 0,
            pins: BTreeSet::new(),
            retention_generation: 0,
            staging_attempted: false,
        }
    }

    fn current_inventory(&self) -> ProllyRetentionInventory {
        let mut roots = Vec::new();
        if let Some(head) = &self.head {
            roots.push(head.root.top_node_ref.clone());
        }
        roots.sort();
        roots.dedup();
        let pins = self.pins.iter().map(|pin| NodeRef::new(pin.clone())).collect();
        ProllyRetentionInventory {
            roots,
            pins,
            retention_generation: self.retention_generation,
        }
    }
}

impl ProllyBlockStorePort for MemoryPort {
    fn read_block(&self, node_ref: &NodeRef) -> ProllyPortResult<Option<Vec<u8>>> {
        Ok(self.blocks.get(node_ref.as_str()).cloned())
    }

    fn open_read_session(&self, bounds: ReadSessionBounds) -> ProllyPortResult<Box<dyn ProllyReadSession + '_>> {
        Ok(Box::new(MemorySession {
            blocks: &self.blocks,
            bounds,
            blocks_read: 0,
            bytes_copied: 0,
            is_closed: false,
        }))
    }

    fn stage_blocks(&mut self, blocks: &[EncodedBlock]) -> ProllyPortResult<()> {
        if self.unknown_mode == UnknownMode::StagingCommitUnknown && !self.staging_attempted {
            self.staging_attempted = true;
            return Err(ProllyPortError::new(
                "scripted-unknown",
                "staging commit failed after possible durability",
                true,
            ));
        }
        for block in blocks {
            self.blocks.insert(block.node_ref.as_str().to_string(), block.bytes.clone());
        }
        Ok(())
    }

    fn read_root(&self, _map_id: &str) -> ProllyPortResult<Option<PublishedProllyRoot>> {
        if self.unknown_mode == UnknownMode::CorruptReadback {
            return Err(ProllyPortError::new("prolly-head-shape", "stored head has the wrong field count", false));
        }
        Ok(self.head.clone())
    }

    fn compare_and_advance(
        &mut self,
        _map_id: &str,
        _expected: &ExpectedProllyRoot,
        next: &PublishedProllyRoot,
    ) -> ProllyPortResult<ProllyPublicationObservation> {
        self.compare_calls += 1;
        match self.unknown_mode {
            UnknownMode::BeforeApply => Err(ProllyPortError::new("scripted-unknown", "before apply", true)),
            UnknownMode::AfterApply => {
                self.head = Some(next.clone());
                Err(ProllyPortError::new("scripted-unknown", "after apply", true))
            }
            UnknownMode::None => {
                self.head = Some(next.clone());
                Ok(ProllyPublicationObservation::Applied)
            }
            UnknownMode::StagingCommitUnknown => {
                self.head = Some(next.clone());
                Ok(ProllyPublicationObservation::Applied)
            }
            UnknownMode::CorruptReadback => Err(ProllyPortError::new("scripted-unknown", "corrupt readback", true)),
        }
    }
}

impl ProllyRetentionPort for MemoryPort {
    fn pin_blocks(&mut self, node_refs: &[NodeRef]) -> ProllyPortResult<()> {
        for node_ref in node_refs {
            self.pins.insert(node_ref.as_str().to_string());
        }
        Ok(())
    }

    fn release_pins(&mut self, node_refs: &[NodeRef]) -> ProllyPortResult<()> {
        for node_ref in node_refs {
            self.pins.remove(node_ref.as_str());
        }
        Ok(())
    }

    fn retention_inventory(&self) -> ProllyPortResult<ProllyRetentionInventory> {
        Ok(self.current_inventory())
    }

    fn compare_retention_inventory_and_delete(
        &mut self,
        expected: &ProllyRetentionInventory,
        candidates: &[NodeRef],
    ) -> ProllyPortResult<GcDeletionOutcome> {
        let current = self.current_inventory();
        if current.roots != expected.roots {
            return Ok(GcDeletionOutcome::InventoryMismatch {
                reason: "published-roots-changed",
            });
        }
        if current.retention_generation != expected.retention_generation {
            return Ok(GcDeletionOutcome::InventoryMismatch {
                reason: "retention-generation-changed",
            });
        }
        if !expected.pins.iter().all(|pin| current.pins.contains(pin)) {
            return Ok(GcDeletionOutcome::InventoryMismatch {
                reason: "expected-pin-missing",
            });
        }
        if candidates.iter().any(|candidate| current.pins.contains(candidate)) {
            return Ok(GcDeletionOutcome::InventoryMismatch {
                reason: "candidate-is-pinned",
            });
        }
        for node_ref in candidates {
            self.blocks.remove(node_ref.as_str());
        }
        Ok(GcDeletionOutcome::Deleted {
            deleted: candidates.to_vec(),
        })
    }
}

pub(super) struct MemorySession<'a> {
    blocks: &'a BTreeMap<String, Vec<u8>>,
    bounds: ReadSessionBounds,
    blocks_read: u32,
    bytes_copied: u64,
    is_closed: bool,
}

impl ProllyReadSession for MemorySession<'_> {
    fn read_block(&mut self, node_ref: &NodeRef) -> ProllyPortResult<Option<Vec<u8>>> {
        if self.is_closed {
            return Err(ProllyPortError::new("prolly-read-session-closed", "the read session is closed", false));
        }
        if self.blocks_read >= self.bounds.max_blocks {
            return Err(ProllyPortError::new(
                "prolly-read-session-block-bound",
                "the read session block bound is reached",
                false,
            ));
        }
        let bytes = self.blocks.get(node_ref.as_str()).cloned();
        self.blocks_read += 1;
        self.bytes_copied += bytes.as_ref().map_or(0, |bytes| bytes.len() as u64);
        Ok(bytes)
    }

    fn close(&mut self) -> ReadSessionObservation {
        self.is_closed = true;
        ReadSessionObservation {
            blocks_read: self.blocks_read,
            bytes_copied: self.bytes_copied,
            transactions_opened: 1,
            end: ReadSessionEnd::Closed,
        }
    }
}
