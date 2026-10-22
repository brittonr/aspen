use std::collections::BTreeMap;

use super::super::*;
use super::encoding::*;

const BOUNDARY_SCALE: u64 = 1_024;
const TARGET_BOUNDARY_THRESHOLD: u64 = 128;
const BOUNDARY_SCORE_BYTES: usize = core::mem::size_of::<u64>();

struct LevelNode {
    range: ChildRange,
    block: EncodedBlock,
}

// r[impl molten.prolly_map.history_independence]
// r[impl molten.prolly_map.boundaries]
pub fn build_map(profile: &ProllyProfile, entries: &[SemanticEntry]) -> Result<MapBuild, Vec<ProllyIssue>> {
    let mut issues = validate_profile(profile);
    issues.extend(validate_entries(profile, entries));
    if length_exceeds(entries.len(), profile.limits.max_entries) {
        issues.push(ProllyIssue::EntryLimitExceeded);
    }
    issues.sort();
    issues.dedup();
    if !issues.is_empty() {
        return Err(issues);
    }

    let leaf_chunks = chunk_entries(profile, entries).map_err(|issue| vec![issue])?;
    let mut blocks = BTreeMap::<String, EncodedBlock>::new();
    let mut level = Vec::with_capacity(leaf_chunks.len());
    for chunk in leaf_chunks {
        let block = encode_leaf(profile, &chunk)?;
        let range = range_for_entries(&chunk, &block)?;
        blocks.insert(block.node_ref.as_str().to_string(), block.clone());
        level.push(LevelNode { range, block });
    }
    finish_build(profile, entries, level, blocks)
}

/// Build the internal levels and the root over a complete leaf level.
///
/// The full rebuild and the incremental planner both finish here, so the
/// internal levels and the root follow one construction.
fn finish_build(
    profile: &ProllyProfile,
    entries: &[SemanticEntry],
    leaves: Vec<LevelNode>,
    mut blocks: BTreeMap<String, EncodedBlock>,
) -> Result<MapBuild, Vec<ProllyIssue>> {
    let mut level = leaves;
    let mut height = 0_u16;
    while level.len() > 1 {
        height = height.checked_add(1).ok_or_else(|| vec![ProllyIssue::TreeHeightExceeded])?;
        if height > profile.limits.max_tree_height {
            return Err(vec![ProllyIssue::TreeHeightExceeded]);
        }
        level = build_internal_level(profile, level, &mut blocks)?;
    }
    let top = level.first().ok_or_else(|| vec![ProllyIssue::MissingBlock("empty-build".to_string())])?;
    let entry_count = u32::try_from(entries.len()).map_err(|_| vec![ProllyIssue::EntryLimitExceeded])?;
    let root = ProllyRoot {
        schema: PROLLY_ROOT_SCHEMA.to_string(),
        profile_ref: profile.profile_ref.clone(),
        top_node_ref: top.block.node_ref.clone(),
        height,
        entry_count,
        root_ref: derive_root_ref(profile, &top.block.node_ref, height, entry_count).map_err(|issue| vec![issue])?,
    };
    let blocks = blocks.into_values().collect::<Vec<_>>();
    let logical_bytes = logical_bytes(entries).map_err(|issue| vec![issue])?;
    let block_bytes = blocks
        .iter()
        .try_fold(0_u64, |total, block| {
            let length = u64::try_from(block.bytes.len()).map_err(|_| ProllyIssue::NodeSizeLimitExceeded)?;
            total.checked_add(length).ok_or(ProllyIssue::NodeSizeLimitExceeded)
        })
        .map_err(|issue| vec![issue])?;
    Ok(MapBuild {
        snapshot: MapSnapshot { root, blocks },
        logical_bytes,
        block_bytes,
    })
}

pub fn derive_root_ref(
    profile: &ProllyProfile,
    top_node_ref: &NodeRef,
    height: u16,
    entry_count: u32,
) -> Result<RootRef, ProllyIssue> {
    let mut hasher = hasher(&profile.root_domain)?;
    hasher.update_tagged_str(b"profile-ref", profile.profile_ref.as_str()).map_err(identity_issue)?;
    hasher.update_tagged_str(b"top-node-ref", top_node_ref.as_str()).map_err(identity_issue)?;
    hasher.update_tagged_u64_le(b"height", u64::from(height));
    hasher.update_tagged_u64_le(b"entry-count", u64::from(entry_count));
    Ok(RootRef::new(format!("blake3:{}", hasher.finish().to_hex())))
}

pub fn boundary_decision(profile: &ProllyProfile, key: &[u8], encoded_size: u32) -> Result<bool, ProllyIssue> {
    if encoded_size >= profile.max_node_bytes {
        return Ok(true);
    }
    if encoded_size < profile.min_node_bytes {
        return Ok(false);
    }
    let threshold = boundary_threshold(profile, encoded_size)?;
    let mut hasher = hasher(&profile.boundary_domain)?;
    hasher.update_tagged_str(b"seed-ref", &profile.boundary_seed_ref).map_err(identity_issue)?;
    hasher.update_tagged_bytes(b"key", key).map_err(identity_issue)?;
    hasher.update_tagged_u64_le(b"encoded-size", u64::from(encoded_size));
    let digest = hasher.finish();
    let score_bytes = digest
        .as_bytes()
        .get(..BOUNDARY_SCORE_BYTES)
        .ok_or(ProllyIssue::IdentityFailure("boundary digest is too short".to_string()))?;
    let score_array = <[u8; BOUNDARY_SCORE_BYTES]>::try_from(score_bytes)
        .map_err(|_| ProllyIssue::IdentityFailure("boundary score width mismatch".to_string()))?;
    let score = u64::from_le_bytes(score_array) % BOUNDARY_SCALE;
    Ok(score < threshold)
}

fn chunk_entries(profile: &ProllyProfile, entries: &[SemanticEntry]) -> Result<Vec<Vec<SemanticEntry>>, ProllyIssue> {
    if entries.is_empty() {
        return Ok(vec![Vec::new()]);
    }
    let mut chunks = Vec::new();
    let mut current = Vec::new();
    for entry in entries {
        let mut candidate = current.clone();
        candidate.push(entry.clone());
        let candidate_size = leaf_encoded_len(profile, &candidate)?;
        if !current.is_empty() && candidate_size > profile.max_node_bytes {
            chunks.push(core::mem::take(&mut current));
            candidate = vec![entry.clone()];
            let single_size = leaf_encoded_len(profile, &candidate)?;
            if single_size > profile.max_node_bytes {
                return Err(ProllyIssue::NodeSizeLimitExceeded);
            }
        }
        current = candidate;
        let current_size = leaf_encoded_len(profile, &current)?;
        if boundary_decision(profile, &entry.key, current_size)? {
            chunks.push(core::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    Ok(chunks)
}

fn build_internal_level(
    profile: &ProllyProfile,
    prior: Vec<LevelNode>,
    blocks: &mut BTreeMap<String, EncodedBlock>,
) -> Result<Vec<LevelNode>, Vec<ProllyIssue>> {
    let ranges = prior.into_iter().map(|node| node.range).collect::<Vec<_>>();
    let groups = chunk_children(profile, ranges).map_err(|issue| vec![issue])?;
    let mut next = Vec::with_capacity(groups.len());
    for group in groups {
        let block = encode_internal(profile, &group)?;
        let range = range_for_children(&group, &block)?;
        blocks.insert(block.node_ref.as_str().to_string(), block.clone());
        next.push(LevelNode { range, block });
    }
    Ok(next)
}

fn chunk_children(profile: &ProllyProfile, children: Vec<ChildRange>) -> Result<Vec<Vec<ChildRange>>, ProllyIssue> {
    let mut groups = Vec::new();
    let mut current = Vec::new();
    for child in children {
        let mut candidate = current.clone();
        candidate.push(child.clone());
        let candidate_size = internal_encoded_len(profile, &candidate)?;
        let is_over_fanout = candidate.len() > usize::from(profile.max_fanout);
        if !current.is_empty() && (candidate_size > profile.max_node_bytes || is_over_fanout) {
            groups.push(core::mem::take(&mut current));
            candidate = vec![child.clone()];
        }
        current = candidate;
        let current_size = internal_encoded_len(profile, &current)?;
        let has_minimum = current.len() >= usize::from(profile.min_fanout);
        if has_minimum && boundary_decision(profile, &child.max_key, current_size)? {
            groups.push(core::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        groups.push(current);
    }
    rebalance_last_group(profile, &mut groups)?;
    Ok(groups)
}

#[allow(
    tigerstyle::borrowed_argument_types,
    reason = "deterministic rebalancing must pop and merge the bounded group vector"
)]
fn rebalance_last_group(profile: &ProllyProfile, groups: &mut Vec<Vec<ChildRange>>) -> Result<(), ProllyIssue> {
    if groups.len() < 2 {
        return Ok(());
    }
    let minimum = usize::from(profile.min_fanout);
    let last_index = groups.len() - 1;
    while groups[last_index].len() < minimum && groups[last_index - 1].len() > minimum {
        let moved = groups[last_index - 1].pop().ok_or(ProllyIssue::InternalFanoutExceeded)?;
        groups[last_index].insert(0, moved);
    }
    if groups[last_index].len() < minimum {
        let combined = groups[last_index - 1]
            .len()
            .checked_add(groups[last_index].len())
            .ok_or(ProllyIssue::InternalFanoutExceeded)?;
        if combined > usize::from(profile.max_fanout) {
            return Err(ProllyIssue::InternalFanoutExceeded);
        }
        let last = groups.pop().ok_or(ProllyIssue::InternalFanoutExceeded)?;
        groups.last_mut().ok_or(ProllyIssue::InternalFanoutExceeded)?.extend(last);
    }
    Ok(())
}

fn range_for_entries(entries: &[SemanticEntry], block: &EncodedBlock) -> Result<ChildRange, Vec<ProllyIssue>> {
    if entries.is_empty() {
        return Ok(ChildRange {
            min_key: Vec::new(),
            max_key: Vec::new(),
            node_ref: block.node_ref.clone(),
            encoded_len: block_len(block)?,
        });
    }
    Ok(ChildRange {
        min_key: entries.first().ok_or_else(|| vec![ProllyIssue::EmptyKey])?.key.clone(),
        max_key: entries.last().ok_or_else(|| vec![ProllyIssue::EmptyKey])?.key.clone(),
        node_ref: block.node_ref.clone(),
        encoded_len: block_len(block)?,
    })
}

fn range_for_children(children: &[ChildRange], block: &EncodedBlock) -> Result<ChildRange, Vec<ProllyIssue>> {
    Ok(ChildRange {
        min_key: children.first().ok_or_else(|| vec![ProllyIssue::EmptyInternalNode])?.min_key.clone(),
        max_key: children.last().ok_or_else(|| vec![ProllyIssue::EmptyInternalNode])?.max_key.clone(),
        node_ref: block.node_ref.clone(),
        encoded_len: block_len(block)?,
    })
}

fn block_len(block: &EncodedBlock) -> Result<u32, Vec<ProllyIssue>> {
    u32::try_from(block.bytes.len()).map_err(|_| vec![ProllyIssue::NodeSizeLimitExceeded])
}

fn logical_bytes(entries: &[SemanticEntry]) -> Result<u64, ProllyIssue> {
    entries.iter().try_fold(0_u64, |total, entry| {
        let key = u64::try_from(entry.key.len()).map_err(|_| ProllyIssue::EntryLimitExceeded)?;
        let value = u64::try_from(entry.value.len()).map_err(|_| ProllyIssue::EntryLimitExceeded)?;
        total
            .checked_add(key)
            .and_then(|total| total.checked_add(value))
            .ok_or(ProllyIssue::EntryLimitExceeded)
    })
}

fn boundary_threshold(profile: &ProllyProfile, encoded_size: u32) -> Result<u64, ProllyIssue> {
    if encoded_size < profile.target_node_bytes {
        let progress = u64::from(encoded_size - profile.min_node_bytes);
        let span = u64::from(profile.target_node_bytes - profile.min_node_bytes);
        return progress
            .checked_mul(TARGET_BOUNDARY_THRESHOLD)
            .and_then(|value| value.checked_div(span))
            .ok_or(ProllyIssue::ProfileBoundInvalid("boundary-span"));
    }
    let progress = u64::from(encoded_size - profile.target_node_bytes);
    let span = u64::from(profile.max_node_bytes - profile.target_node_bytes);
    let remaining = BOUNDARY_SCALE - TARGET_BOUNDARY_THRESHOLD;
    let growth = progress
        .checked_mul(remaining)
        .and_then(|value| value.checked_div(span))
        .ok_or(ProllyIssue::ProfileBoundInvalid("boundary-span"))?;
    TARGET_BOUNDARY_THRESHOLD
        .checked_add(growth)
        .map(|threshold| threshold.min(BOUNDARY_SCALE))
        .ok_or(ProllyIssue::ProfileBoundInvalid("boundary-threshold"))
}

fn length_exceeds(length: usize, maximum: u32) -> bool {
    match u32::try_from(length) {
        Ok(length) => length > maximum,
        Err(_) => true,
    }
}

/// The bounds of the incremental leaf planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IncrementalEditBounds {
    /// The most entries the planner may rechunk before it falls back to the
    /// full rebuild.
    pub max_rechunked_entries: u32,
}

/// The prior leaf level the incremental planner reuses.
#[derive(Debug, Clone, Copy)]
pub struct IncrementalPrior<'a> {
    /// The flattened entries of the validated prior snapshot.
    pub entries: &'a [SemanticEntry],
    /// The prior leaves in key order, as spans of `entries`.
    pub leaf_spans: &'a [LeafSpan],
    /// The blocks of the prior snapshot.
    pub blocks: &'a [EncodedBlock],
}

/// The outcome of the incremental leaf planner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncrementalBuildOutcome {
    /// The planner built the next map.
    Built {
        build: MapBuild,
        leaves_encoded: u32,
        rechunked_entries: u32,
    },
    /// The planner reached its work bound, or the input has no reusable leaf
    /// level. The caller completes through the full rebuild.
    Fallback,
}

/// Build the next map by rechunking only the affected leaf region.
///
/// Leaf chunking restarts its size accumulator at every chunk start, so a
/// chunk that starts at a prior leaf start, with the same remaining entries,
/// produces the same chunks as the prior build from that point on. The planner
/// reuses the prior leaves before the first changed entry, rechunks from the
/// start of the leaf that holds it, and resumes reusing prior leaves as soon
/// as a new chunk ends at a prior leaf start inside the unchanged suffix. The
/// internal levels and the root are then built over the complete leaf level
/// exactly as the full rebuild builds them, so the canonical root is the same.
// r[impl molten.prolly.incremental]
pub fn build_map_incremental(
    profile: &ProllyProfile,
    prior: IncrementalPrior<'_>,
    next_entries: &[SemanticEntry],
    bounds: IncrementalEditBounds,
) -> Result<IncrementalBuildOutcome, Vec<ProllyIssue>> {
    let mut issues = validate_profile(profile);
    issues.extend(validate_entries(profile, next_entries));
    if length_exceeds(next_entries.len(), profile.limits.max_entries) {
        issues.push(ProllyIssue::EntryLimitExceeded);
    }
    issues.sort();
    issues.dedup();
    if !issues.is_empty() {
        return Err(issues);
    }
    if prior.entries.is_empty() || next_entries.is_empty() || prior.leaf_spans.is_empty() {
        return Ok(IncrementalBuildOutcome::Fallback);
    }
    let prior_len = prior.entries.len();
    let next_len = next_entries.len();
    let prefix = prior.entries.iter().zip(next_entries).take_while(|(left, right)| left == right).count();
    let max_suffix = prior_len.min(next_len) - prefix;
    let suffix = prior
        .entries
        .iter()
        .rev()
        .zip(next_entries.iter().rev())
        .take(max_suffix)
        .take_while(|(left, right)| left == right)
        .count();
    let anchor = prefix.min(prior_len - 1);
    let first_leaf = prior.leaf_spans.iter().rposition(|span| span.start <= anchor).unwrap_or(0);
    let start = prior.leaf_spans[first_leaf].start;
    let aligned_from = next_len - suffix;
    let prior_starts = prior
        .leaf_spans
        .iter()
        .enumerate()
        .map(|(index, span)| (span.start, index))
        .collect::<BTreeMap<_, _>>();
    let block_index = prior.blocks.iter().map(|block| (block.node_ref.as_str(), block)).collect::<BTreeMap<_, _>>();
    let mut level = Vec::with_capacity(prior.leaf_spans.len().saturating_add(1));
    let mut blocks = BTreeMap::<String, EncodedBlock>::new();
    for span in &prior.leaf_spans[..first_leaf] {
        push_reused_leaf(prior, &block_index, span, &mut level, &mut blocks)?;
    }
    let mut current = Vec::new();
    let mut leaves_encoded = 0_u32;
    let mut rechunked_entries = 0_u32;
    let mut resume_from = None;
    for (position, entry) in next_entries.iter().enumerate().skip(start) {
        if current.is_empty() && position > start && position >= aligned_from {
            let prior_position = position + prior_len - next_len;
            if let Some(&leaf_index) = prior_starts.get(&prior_position) {
                resume_from = Some(leaf_index);
                break;
            }
        }
        if rechunked_entries >= bounds.max_rechunked_entries {
            return Ok(IncrementalBuildOutcome::Fallback);
        }
        rechunked_entries = rechunked_entries.saturating_add(1);
        let mut candidate = current.clone();
        candidate.push(entry.clone());
        let candidate_size = leaf_encoded_len(profile, &candidate).map_err(|issue| vec![issue])?;
        if !current.is_empty() && candidate_size > profile.max_node_bytes {
            push_new_leaf(profile, core::mem::take(&mut current), &mut level, &mut blocks)?;
            leaves_encoded = leaves_encoded.saturating_add(1);
            candidate = vec![entry.clone()];
            let single_size = leaf_encoded_len(profile, &candidate).map_err(|issue| vec![issue])?;
            if single_size > profile.max_node_bytes {
                return Err(vec![ProllyIssue::NodeSizeLimitExceeded]);
            }
        }
        current = candidate;
        let current_size = leaf_encoded_len(profile, &current).map_err(|issue| vec![issue])?;
        if boundary_decision(profile, &entry.key, current_size).map_err(|issue| vec![issue])? {
            push_new_leaf(profile, core::mem::take(&mut current), &mut level, &mut blocks)?;
            leaves_encoded = leaves_encoded.saturating_add(1);
        }
    }
    if !current.is_empty() {
        push_new_leaf(profile, current, &mut level, &mut blocks)?;
        leaves_encoded = leaves_encoded.saturating_add(1);
    }
    if let Some(leaf_index) = resume_from {
        for span in &prior.leaf_spans[leaf_index..] {
            push_reused_leaf(prior, &block_index, span, &mut level, &mut blocks)?;
        }
    }
    let build = finish_build(profile, next_entries, level, blocks)?;
    Ok(IncrementalBuildOutcome::Built {
        build,
        leaves_encoded,
        rechunked_entries,
    })
}

fn push_new_leaf(
    profile: &ProllyProfile,
    chunk: Vec<SemanticEntry>,
    level: &mut Vec<LevelNode>,
    blocks: &mut BTreeMap<String, EncodedBlock>,
) -> Result<(), Vec<ProllyIssue>> {
    let block = encode_leaf(profile, &chunk)?;
    let range = range_for_entries(&chunk, &block)?;
    blocks.insert(block.node_ref.as_str().to_string(), block.clone());
    level.push(LevelNode { range, block });
    Ok(())
}

fn push_reused_leaf(
    prior: IncrementalPrior<'_>,
    block_index: &BTreeMap<&str, &EncodedBlock>,
    span: &LeafSpan,
    level: &mut Vec<LevelNode>,
    blocks: &mut BTreeMap<String, EncodedBlock>,
) -> Result<(), Vec<ProllyIssue>> {
    let block = *block_index
        .get(span.node_ref.as_str())
        .ok_or_else(|| vec![ProllyIssue::MissingBlock(span.node_ref.as_str().to_string())])?;
    let end = span.start.checked_add(span.len).ok_or_else(|| vec![ProllyIssue::EntryLimitExceeded])?;
    let chunk = prior
        .entries
        .get(span.start..end)
        .ok_or_else(|| vec![ProllyIssue::MissingBlock(span.node_ref.as_str().to_string())])?;
    let range = range_for_entries(chunk, block)?;
    blocks.insert(block.node_ref.as_str().to_string(), block.clone());
    level.push(LevelNode {
        range,
        block: block.clone(),
    });
    Ok(())
}
