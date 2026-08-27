use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::WorldApplicationHandlerProfile;
use super::WorldMergeBounds;
use super::WorldMergeConflict;
use super::WorldMergeIssue;
use crate::world_commit::RootKind;

pub struct WorldApplicationMergeInput<'a> {
    pub kind: RootKind,
    pub base: &'a [u8],
    pub left: &'a [u8],
    pub right: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldApplicationMergeOutput {
    pub canonical_bytes: Vec<u8>,
    pub requested_effect: bool,
}

pub trait WorldMergeHandler {
    fn profile(&self) -> &WorldApplicationHandlerProfile;

    fn merge(&self, input: &WorldApplicationMergeInput<'_>) -> Result<WorldApplicationMergeOutput, &'static str>;
}

pub(crate) struct KeyedMergeResult {
    pub values: BTreeMap<String, Vec<u8>>,
    pub conflicts: Vec<WorldMergeConflict>,
}

pub(crate) fn merge_keyed_values(
    kind: RootKind,
    base: &BTreeMap<String, Vec<u8>>,
    left: &BTreeMap<String, Vec<u8>>,
    right: &BTreeMap<String, Vec<u8>>,
    bounds: &WorldMergeBounds,
) -> Result<KeyedMergeResult, WorldMergeIssue> {
    let maximum_keys = usize::try_from(bounds.max_keys).map_err(|_| WorldMergeIssue::InvalidBounds("max_keys"))?;
    let maximum_conflicts =
        usize::try_from(bounds.max_conflicts).map_err(|_| WorldMergeIssue::InvalidBounds("max_conflicts"))?;
    let keys = base.keys().chain(left.keys()).chain(right.keys()).cloned().collect::<BTreeSet<_>>();
    if keys.len() > maximum_keys {
        return Err(WorldMergeIssue::KeyLimitExceeded);
    }
    let mut merged = BTreeMap::new();
    let mut conflicts = Vec::with_capacity(maximum_conflicts.min(keys.len()));
    for key in keys {
        let base_value = base.get(&key);
        let left_value = left.get(&key);
        let right_value = right.get(&key);
        let selected = match select_key(base_value, left_value, right_value) {
            KeySelection::Selected(value) => value,
            KeySelection::Conflict => {
                if conflicts.len() >= maximum_conflicts {
                    return Err(WorldMergeIssue::ConflictLimitExceeded);
                }
                conflicts.push(WorldMergeConflict {
                    kind,
                    key: Some(key),
                    code: "concurrent-key-change",
                });
                continue;
            }
        };
        if let Some(value) = selected {
            let value_bytes = u64::try_from(value.len()).map_err(|_| WorldMergeIssue::ValueLimitExceeded)?;
            if value_bytes > bounds.max_value_bytes {
                return Err(WorldMergeIssue::ValueLimitExceeded);
            }
            if merged.len() >= maximum_keys {
                return Err(WorldMergeIssue::KeyLimitExceeded);
            }
            merged.insert(key, value.clone());
        }
    }
    Ok(KeyedMergeResult {
        values: merged,
        conflicts,
    })
}

enum KeySelection<'a> {
    Selected(Option<&'a Vec<u8>>),
    Conflict,
}

fn select_key<'a>(
    base: Option<&'a Vec<u8>>,
    left: Option<&'a Vec<u8>>,
    right: Option<&'a Vec<u8>>,
) -> KeySelection<'a> {
    if left == right {
        KeySelection::Selected(left)
    } else if left == base {
        KeySelection::Selected(right)
    } else if right == base {
        KeySelection::Selected(left)
    } else {
        KeySelection::Conflict
    }
}
