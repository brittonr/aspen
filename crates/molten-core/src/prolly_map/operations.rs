use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;

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
    edit_plan(profile, snapshot, &prior, edits, next, strategy, leaves_encoded)
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
    edit_plan(profile, snapshot, &prior, edits, next, EditStrategy::FullRebuild, leaves)
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

fn edit_plan(
    profile: &ProllyProfile,
    snapshot: &MapSnapshot,
    prior: &MapReadResult,
    edits: &[MapEdit],
    next: MapBuild,
    strategy: EditStrategy,
    leaves_encoded: u32,
) -> Result<EditPlan, Vec<ProllyIssue>> {
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

/// Diff two snapshots.
///
/// Both snapshots are validated under the existing contracts first: a matching
/// root string never admits an untrusted snapshot. The comparison then walks
/// both trees from their roots and skips every pair of equal subtrees it meets
/// at the same position, so it reads and decodes nodes only in the differing
/// regions. Validation shares one operation-scoped decoded-node cache, so the
/// comparison reuses the verified decodes instead of decoding again. The
/// records are exactly the records of the reference diff.
// r[impl molten.prolly_map.diff]
// r[impl molten.prolly.diff]
// r[impl molten.prolly.metrics]
pub fn diff_maps(
    profile: &ProllyProfile,
    left: &MapSnapshot,
    right: &MapSnapshot,
) -> Result<MapDiff, Vec<ProllyIssue>> {
    let mut cache = DecodedNodeCache::operation_scoped(left.blocks.len().saturating_add(right.blocks.len()));
    let left_read = validate_snapshot_cached(profile, left, &mut cache)?;
    let right_read = validate_snapshot_cached(profile, right, &mut cache)?;
    let shared_node_count = shared_nodes(&left_read, &right_read)?;
    let validation = left_read.work.combined(right_read.work);
    let mut comparison = DiffComparison {
        profile,
        left: block_map(left),
        right: block_map(right),
        cache: &mut cache,
        work: ProllyWorkCounters::default(),
        records: Vec::new(),
    };
    comparison.run(left, right)?;
    let work = comparison.work;
    let records = comparison.records;
    Ok(MapDiff {
        left_root_ref: left.root.root_ref.clone(),
        right_root_ref: right.root.root_ref.clone(),
        records,
        shared_node_count,
        validation,
        comparison: work,
        complete: true,
        selects_merge_winner: false,
    })
}

/// The flattened-merge reference diff, retained as the oracle.
///
/// It validates both snapshots and merges their complete entry sequences.
pub fn diff_maps_reference(
    profile: &ProllyProfile,
    left: &MapSnapshot,
    right: &MapSnapshot,
) -> Result<MapDiff, Vec<ProllyIssue>> {
    let left_read = validate_snapshot(profile, left)?;
    let right_read = validate_snapshot(profile, right)?;
    let shared_node_count = shared_nodes(&left_read, &right_read)?;
    let validation = left_read.work.combined(right_read.work);
    let comparison = ProllyWorkCounters {
        nodes_read: left_read.visited_nodes.saturating_add(right_read.visited_nodes),
        ..ProllyWorkCounters::default()
    };
    if left.root.root_ref == right.root.root_ref {
        return Ok(MapDiff {
            left_root_ref: left.root.root_ref.clone(),
            right_root_ref: right.root.root_ref.clone(),
            records: Vec::new(),
            shared_node_count,
            validation,
            comparison,
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
    Ok(MapDiff {
        left_root_ref: left.root.root_ref.clone(),
        right_root_ref: right.root.root_ref.clone(),
        records,
        shared_node_count,
        validation,
        comparison,
        complete: true,
        selects_merge_winner: false,
    })
}

fn shared_nodes(left: &MapReadResult, right: &MapReadResult) -> Result<u32, Vec<ProllyIssue>> {
    let left_closure = left.closure.iter().map(NodeRef::as_str).collect::<BTreeSet<_>>();
    let right_closure = right.closure.iter().map(NodeRef::as_str).collect::<BTreeSet<_>>();
    u32::try_from(left_closure.intersection(&right_closure).count()).map_err(|_| vec![ProllyIssue::GraphLimitExceeded])
}

fn block_map(snapshot: &MapSnapshot) -> BTreeMap<&str, &EncodedBlock> {
    snapshot.blocks.iter().map(|block| (block.node_ref.as_str(), block)).collect()
}

/// One position of a demand-driven diff cursor.
enum DiffItem {
    Node { node_ref: NodeRef, height: u16 },
    Entry(SemanticEntry),
}

struct DiffComparison<'a, 'c> {
    profile: &'a ProllyProfile,
    left: BTreeMap<&'a str, &'a EncodedBlock>,
    right: BTreeMap<&'a str, &'a EncodedBlock>,
    cache: &'c mut DecodedNodeCache,
    work: ProllyWorkCounters,
    records: Vec<DiffRecord>,
}

impl DiffComparison<'_, '_> {
    fn run(&mut self, left: &MapSnapshot, right: &MapSnapshot) -> Result<(), Vec<ProllyIssue>> {
        let mut left_items = VecDeque::from([DiffItem::Node {
            node_ref: left.root.top_node_ref.clone(),
            height: left.root.height,
        }]);
        let mut right_items = VecDeque::from([DiffItem::Node {
            node_ref: right.root.top_node_ref.clone(),
            height: right.root.height,
        }]);
        loop {
            match (left_items.front(), right_items.front()) {
                (None, None) => break,
                (
                    Some(DiffItem::Node {
                        node_ref: left_ref,
                        height: left_height,
                    }),
                    Some(DiffItem::Node {
                        node_ref: right_ref,
                        height: right_height,
                    }),
                ) => {
                    if left_ref == right_ref {
                        left_items.pop_front();
                        right_items.pop_front();
                        self.work.subtrees_not_traversed = self.work.subtrees_not_traversed.saturating_add(1);
                        continue;
                    }
                    let expand_left = left_height >= right_height;
                    let expand_right = right_height >= left_height;
                    if expand_left {
                        self.expand(Side::Left, &mut left_items)?;
                    }
                    if expand_right {
                        self.expand(Side::Right, &mut right_items)?;
                    }
                }
                (Some(DiffItem::Node { .. }), _) => self.expand(Side::Left, &mut left_items)?,
                (_, Some(DiffItem::Node { .. })) => self.expand(Side::Right, &mut right_items)?,
                (Some(DiffItem::Entry(left_entry)), Some(DiffItem::Entry(right_entry))) => {
                    match left_entry.key.cmp(&right_entry.key) {
                        core::cmp::Ordering::Less => {
                            self.records.push(removed(left_entry));
                            left_items.pop_front();
                        }
                        core::cmp::Ordering::Greater => {
                            self.records.push(added(right_entry));
                            right_items.pop_front();
                        }
                        core::cmp::Ordering::Equal => {
                            if left_entry.value != right_entry.value {
                                self.records.push(modified(left_entry, right_entry));
                            }
                            left_items.pop_front();
                            right_items.pop_front();
                        }
                    }
                }
                (Some(DiffItem::Entry(left_entry)), None) => {
                    self.records.push(removed(left_entry));
                    left_items.pop_front();
                }
                (None, Some(DiffItem::Entry(right_entry))) => {
                    self.records.push(added(right_entry));
                    right_items.pop_front();
                }
            }
            if length_exceeds(self.records.len(), self.profile.limits.max_diff_records) {
                return Err(vec![ProllyIssue::DiffLimitExceeded]);
            }
        }
        Ok(())
    }

    fn expand(&mut self, side: Side, items: &mut VecDeque<DiffItem>) -> Result<(), Vec<ProllyIssue>> {
        let Some(DiffItem::Node { node_ref, height }) = items.pop_front() else {
            return Ok(());
        };
        let blocks = match side {
            Side::Left => &self.left,
            Side::Right => &self.right,
        };
        let block = *blocks
            .get(node_ref.as_str())
            .ok_or_else(|| vec![ProllyIssue::MissingBlock(node_ref.as_str().to_string())])?;
        self.work.nodes_read = self.work.nodes_read.saturating_add(1);
        let (node, is_decoded) = decode_block_cached(self.profile, block, self.cache)?;
        if is_decoded {
            self.work.nodes_decoded = self.work.nodes_decoded.saturating_add(1);
        }
        match node {
            ProllyNode::Leaf(leaf) => {
                for entry in leaf.entries.into_iter().rev() {
                    items.push_front(DiffItem::Entry(entry));
                }
            }
            ProllyNode::Internal(internal) => {
                let child_height = height.checked_sub(1).ok_or_else(|| vec![ProllyIssue::TreeHeightExceeded])?;
                for child in internal.children.into_iter().rev() {
                    items.push_front(DiffItem::Node {
                        node_ref: child.node_ref,
                        height: child_height,
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum Side {
    Left,
    Right,
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
