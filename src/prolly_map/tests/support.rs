use std::collections::BTreeMap;

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
}

pub(super) struct MemoryPort {
    pub(super) blocks: BTreeMap<String, Vec<u8>>,
    pub(super) head: Option<PublishedProllyRoot>,
    pub(super) unknown_mode: UnknownMode,
    pub(super) compare_calls: u32,
}

impl MemoryPort {
    pub(super) fn new(unknown_mode: UnknownMode) -> Self {
        Self {
            blocks: BTreeMap::new(),
            head: None,
            unknown_mode,
            compare_calls: 0,
        }
    }
}

impl ProllyBlockStorePort for MemoryPort {
    fn read_block(&self, node_ref: &NodeRef) -> ProllyPortResult<Option<Vec<u8>>> {
        Ok(self.blocks.get(node_ref.as_str()).cloned())
    }

    fn stage_blocks(&mut self, blocks: &[EncodedBlock]) -> ProllyPortResult<()> {
        for block in blocks {
            self.blocks.insert(block.node_ref.as_str().to_string(), block.bytes.clone());
        }
        Ok(())
    }

    fn read_root(&self, _map_id: &str) -> ProllyPortResult<Option<PublishedProllyRoot>> {
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
        }
    }

    fn delete_blocks(&mut self, node_refs: &[NodeRef]) -> ProllyPortResult<()> {
        for node_ref in node_refs {
            self.blocks.remove(node_ref.as_str());
        }
        Ok(())
    }
}
