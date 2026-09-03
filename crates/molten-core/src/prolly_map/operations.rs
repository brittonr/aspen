use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;

// r[impl molten.prolly_map.history_independence]
pub fn plan_edits(
    profile: &ProllyProfile,
    snapshot: &MapSnapshot,
    edits: &[MapEdit],
) -> Result<EditPlan, Vec<ProllyIssue>> {
    if length_exceeds(edits.len(), profile.limits.max_entries) {
        return Err(vec![ProllyIssue::EditCountExceeded]);
    }
    let prior = validate_snapshot(profile, snapshot)?;
    let mut entries = prior
        .entries
        .iter()
        .map(|entry| (entry.key.clone(), entry.value.clone()))
        .collect::<BTreeMap<_, _>>();
    for edit in edits {
        apply_edit(&mut entries, edit)?;
    }
    let canonical = entries.into_iter().map(|(key, value)| SemanticEntry { key, value }).collect::<Vec<_>>();
    let next = build_map(profile, &canonical)?;
    let prior_refs = prior.closure.iter().map(NodeRef::as_str).collect::<BTreeSet<_>>();
    let next_refs = next.snapshot.blocks.iter().map(|block| block.node_ref.as_str()).collect::<BTreeSet<_>>();
    let reused_block_count = u32::try_from(prior_refs.intersection(&next_refs).count())
        .map_err(|_| vec![ProllyIssue::GraphLimitExceeded])?;
    let staged_blocks = next
        .snapshot
        .blocks
        .iter()
        .filter(|block| !prior_refs.contains(block.node_ref.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let edit_count = u32::try_from(edits.len()).map_err(|_| vec![ProllyIssue::EditCountExceeded])?;
    Ok(EditPlan {
        profile_ref: profile.profile_ref.clone(),
        prior_root_ref: snapshot.root.root_ref.clone(),
        next,
        staged_blocks,
        reused_block_count,
        edit_count,
    })
}

// r[impl molten.prolly_map.diff]
pub fn diff_maps(
    profile: &ProllyProfile,
    left: &MapSnapshot,
    right: &MapSnapshot,
) -> Result<MapDiff, Vec<ProllyIssue>> {
    let left_read = validate_snapshot(profile, left)?;
    let right_read = validate_snapshot(profile, right)?;
    if left.root.root_ref == right.root.root_ref {
        return Ok(MapDiff {
            left_root_ref: left.root.root_ref.clone(),
            right_root_ref: right.root.root_ref.clone(),
            records: Vec::new(),
            skipped_equal_nodes: left_read.visited_nodes,
            complete: true,
            selects_merge_winner: false,
        });
    }
    let mut records = Vec::new();
    let mut left_index = 0_usize;
    let mut right_index = 0_usize;
    while left_index < left_read.entries.len() || right_index < right_read.entries.len() {
        match (left_read.entries.get(left_index), right_read.entries.get(right_index)) {
            (Some(left_entry), Some(right_entry)) => match left_entry.key.cmp(&right_entry.key) {
                core::cmp::Ordering::Less => {
                    records.push(removed(left_entry));
                    left_index += 1;
                }
                core::cmp::Ordering::Greater => {
                    records.push(added(right_entry));
                    right_index += 1;
                }
                core::cmp::Ordering::Equal => {
                    if left_entry.value != right_entry.value {
                        records.push(modified(left_entry, right_entry));
                    }
                    left_index += 1;
                    right_index += 1;
                }
            },
            (Some(left_entry), None) => {
                records.push(removed(left_entry));
                left_index += 1;
            }
            (None, Some(right_entry)) => {
                records.push(added(right_entry));
                right_index += 1;
            }
            (None, None) => break,
        }
        if length_exceeds(records.len(), profile.limits.max_diff_records) {
            return Err(vec![ProllyIssue::DiffLimitExceeded]);
        }
    }
    let left_closure = left_read.closure.iter().map(NodeRef::as_str).collect::<BTreeSet<_>>();
    let right_closure = right_read.closure.iter().map(NodeRef::as_str).collect::<BTreeSet<_>>();
    let skipped_equal_nodes = u32::try_from(left_closure.intersection(&right_closure).count())
        .map_err(|_| vec![ProllyIssue::GraphLimitExceeded])?;
    Ok(MapDiff {
        left_root_ref: left.root.root_ref.clone(),
        right_root_ref: right.root.root_ref.clone(),
        records,
        skipped_equal_nodes,
        complete: true,
        selects_merge_winner: false,
    })
}

pub fn benchmark_map(
    build: &MapBuild,
    edit: &EditPlan,
    diff: &MapDiff,
    gc: &GcPlan,
    restart_verified: bool,
) -> Result<ProllyBenchmarkResult, Vec<ProllyIssue>> {
    if !diff.complete || diff.selects_merge_winner || gc.deletion_authorized || !restart_verified {
        return Err(vec![ProllyIssue::BenchmarkOverclaim]);
    }
    Ok(ProllyBenchmarkResult {
        profile_ref: build.snapshot.root.profile_ref.clone(),
        entry_count: build.snapshot.root.entry_count,
        logical_bytes: build.logical_bytes,
        block_count: count_u32(build.snapshot.blocks.len())?,
        block_bytes: build.block_bytes,
        reused_blocks: edit.reused_block_count,
        diff_records: count_u32(diff.records.len())?,
        skipped_equal_nodes: diff.skipped_equal_nodes,
        gc_candidates: count_u32(gc.candidate_unreachable.len())?,
        restart_verified,
        timing_proves_correctness: false,
    })
}

fn apply_edit(entries: &mut BTreeMap<Vec<u8>, Vec<u8>>, edit: &MapEdit) -> Result<(), Vec<ProllyIssue>> {
    match edit {
        MapEdit::Insert(entry) => {
            if entries.contains_key(&entry.key) {
                return Err(vec![ProllyIssue::EditInsertExisting(entry.key.clone())]);
            }
            entries.insert(entry.key.clone(), entry.value.clone());
        }
        MapEdit::Update(entry) => {
            let Some(value) = entries.get_mut(&entry.key) else {
                return Err(vec![ProllyIssue::EditUpdateMissing(entry.key.clone())]);
            };
            *value = entry.value.clone();
        }
        MapEdit::Delete(key) => {
            if entries.remove(key).is_none() {
                return Err(vec![ProllyIssue::EditDeleteMissing(key.clone())]);
            }
        }
    }
    Ok(())
}

fn added(entry: &SemanticEntry) -> DiffRecord {
    DiffRecord {
        kind: DiffKind::Added,
        key: entry.key.clone(),
        before: None,
        after: Some(entry.value.clone()),
    }
}

fn removed(entry: &SemanticEntry) -> DiffRecord {
    DiffRecord {
        kind: DiffKind::Removed,
        key: entry.key.clone(),
        before: Some(entry.value.clone()),
        after: None,
    }
}

fn modified(left: &SemanticEntry, right: &SemanticEntry) -> DiffRecord {
    DiffRecord {
        kind: DiffKind::Modified,
        key: left.key.clone(),
        before: Some(left.value.clone()),
        after: Some(right.value.clone()),
    }
}

fn count_u32(length: usize) -> Result<u32, Vec<ProllyIssue>> {
    u32::try_from(length).map_err(|_| vec![ProllyIssue::GraphLimitExceeded])
}

fn length_exceeds(length: usize, maximum: u32) -> bool {
    match u32::try_from(length) {
        Ok(length) => length > maximum,
        Err(_) => true,
    }
}
