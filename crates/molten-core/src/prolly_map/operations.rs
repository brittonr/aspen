use std::collections::BTreeMap;
use std::collections::BTreeSet;

mod diff;

pub use diff::*;

use super::*;

/// The default bound on the entries one incremental edit may rechunk.
pub const DEFAULT_MAX_RECHUNKED_ENTRIES: u32 = 1_024;

/// Plan one atomic batch of edits.
///
/// The plan rechunks only the affected leaf region and reuses the rest, under
/// the default work bound, and falls back to the full rebuild when the bound is
/// reached. The canonical root is the one the full rebuild produces.
// r[impl molten.prolly_map.history_independence]
pub fn plan_edits(
    profile: &ProllyProfile,
    snapshot: &MapSnapshot,
    edits: &[MapEdit],
) -> Result<EditPlan, Vec<ProllyIssue>> {
    plan_edits_with_bounds(profile, snapshot, edits, IncrementalEditBounds {
        max_rechunked_entries: DEFAULT_MAX_RECHUNKED_ENTRIES,
    })
}

/// Plan one atomic batch of edits under an explicit incremental work bound.
// r[impl molten.prolly.incremental]
pub fn plan_edits_with_bounds(
    profile: &ProllyProfile,
    snapshot: &MapSnapshot,
    edits: &[MapEdit],
    bounds: IncrementalEditBounds,
) -> Result<EditPlan, Vec<ProllyIssue>> {
    if length_exceeds(edits.len(), profile.limits.max_entries) {
        return Err(vec![ProllyIssue::EditCountExceeded]);
    }
    let prior = validate_snapshot(profile, snapshot)?;
    let canonical = edited_entries(&prior.entries, edits)?;
    let incremental = build_map_incremental(
        profile,
        IncrementalPrior {
            entries: &prior.entries,
            leaf_spans: &prior.leaf_spans,
            blocks: &snapshot.blocks,
        },
        &canonical,
        bounds,
    )?;
    let (next, strategy, leaves_encoded) = match incremental {
        IncrementalBuildOutcome::Built {
            build, leaves_encoded, ..
        } => (build, EditStrategy::Incremental, leaves_encoded),
        IncrementalBuildOutcome::Fallback => {
            let build = build_map(profile, &canonical)?;
            let leaves = leaf_count(profile, &build)?;
            (build, EditStrategy::RebuildFallback, leaves)
        }
    };
    edit_plan(EditPlanInput {
        profile,
        snapshot,
        prior: &prior,
        edits,
        next,
        strategy,
        leaves_encoded,
    })
}

/// The rebuild-first reference planner, retained as the oracle.
///
/// It applies the edits to the full entry sequence and rebuilds every block.
pub fn plan_edits_reference(
    profile: &ProllyProfile,
    snapshot: &MapSnapshot,
    edits: &[MapEdit],
) -> Result<EditPlan, Vec<ProllyIssue>> {
    if length_exceeds(edits.len(), profile.limits.max_entries) {
        return Err(vec![ProllyIssue::EditCountExceeded]);
    }
    let prior = validate_snapshot(profile, snapshot)?;
    let canonical = edited_entries(&prior.entries, edits)?;
    let next = build_map(profile, &canonical)?;
    let leaves = leaf_count(profile, &next)?;
    edit_plan(EditPlanInput {
        profile,
        snapshot,
        prior: &prior,
        edits,
        next,
        strategy: EditStrategy::FullRebuild,
        leaves_encoded: leaves,
    })
}

fn edited_entries(prior: &[SemanticEntry], edits: &[MapEdit]) -> Result<Vec<SemanticEntry>, Vec<ProllyIssue>> {
    let mut entries = prior.iter().map(|entry| (entry.key.clone(), entry.value.clone())).collect::<BTreeMap<_, _>>();
    for edit in edits {
        apply_edit(&mut entries, edit)?;
    }
    Ok(entries.into_iter().map(|(key, value)| SemanticEntry { key, value }).collect())
}

fn leaf_count(profile: &ProllyProfile, build: &MapBuild) -> Result<u32, Vec<ProllyIssue>> {
    let read = validate_snapshot(profile, &build.snapshot)?;
    count_u32(read.leaf_spans.len())
}

struct EditPlanInput<'a> {
    profile: &'a ProllyProfile,
    snapshot: &'a MapSnapshot,
    prior: &'a MapReadResult,
    edits: &'a [MapEdit],
    next: MapBuild,
    strategy: EditStrategy,
    leaves_encoded: u32,
}

fn edit_plan(input: EditPlanInput<'_>) -> Result<EditPlan, Vec<ProllyIssue>> {
    let EditPlanInput {
        profile,
        snapshot,
        prior,
        edits,
        next,
        strategy,
        leaves_encoded,
    } = input;
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
        strategy,
        leaves_encoded,
        work: prior.work,
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
        shared_node_count: diff.shared_node_count,
        diff_subtrees_not_traversed: diff.comparison.subtrees_not_traversed,
        diff_comparison_nodes_decoded: diff.comparison.nodes_decoded,
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
