//! Incremental leaf planning over a prior validated build.

use std::collections::BTreeMap;

use super::super::encoding::*;
use super::*;

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
    validate_incremental(profile, next_entries)?;
    let Some(plan) = RechunkPlan::new(prior, next_entries) else {
        return Ok(IncrementalBuildOutcome::Fallback);
    };
    let Some(rechunk) = rechunk_leaves(profile, prior, next_entries, bounds, &plan)? else {
        return Ok(IncrementalBuildOutcome::Fallback);
    };
    let build = finish_build(profile, next_entries, rechunk.level, rechunk.blocks)?;
    Ok(IncrementalBuildOutcome::Built {
        build,
        leaves_encoded: rechunk.leaves_encoded,
        rechunked_entries: rechunk.rechunked_entries,
    })
}

fn validate_incremental(profile: &ProllyProfile, next_entries: &[SemanticEntry]) -> Result<(), Vec<ProllyIssue>> {
    let mut issues = validate_profile(profile);
    issues.extend(validate_entries(profile, next_entries));
    if length_exceeds(next_entries.len(), profile.limits.max_entries) {
        issues.push(ProllyIssue::EntryLimitExceeded);
    }
    issues.sort();
    issues.dedup();
    if issues.is_empty() {
        return Ok(());
    }
    Err(issues)
}

/// The unchanged prefix, suffix and prior leaf index the rechunk loop reuses.
struct RechunkPlan<'a> {
    start: usize,
    aligned_from: usize,
    first_leaf: usize,
    prior_starts: BTreeMap<usize, usize>,
    block_index: BTreeMap<&'a str, &'a EncodedBlock>,
}

impl<'a> RechunkPlan<'a> {
    fn new(prior: IncrementalPrior<'a>, next_entries: &[SemanticEntry]) -> Option<Self> {
        if prior.entries.is_empty() || next_entries.is_empty() || prior.leaf_spans.is_empty() {
            return None;
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
        let prior_starts = prior.leaf_spans.iter().enumerate().map(|(index, span)| (span.start, index)).collect();
        let block_index = prior.blocks.iter().map(|block| (block.node_ref.as_str(), block)).collect();
        Some(Self {
            start,
            aligned_from,
            first_leaf,
            prior_starts,
            block_index,
        })
    }
}

struct Rechunk {
    level: Vec<LevelNode>,
    blocks: BTreeMap<String, EncodedBlock>,
    leaves_encoded: u32,
    rechunked_entries: u32,
}

fn rechunk_leaves(
    profile: &ProllyProfile,
    prior: IncrementalPrior<'_>,
    next_entries: &[SemanticEntry],
    bounds: IncrementalEditBounds,
    plan: &RechunkPlan<'_>,
) -> Result<Option<Rechunk>, Vec<ProllyIssue>> {
    let prior_len = prior.entries.len();
    let next_len = next_entries.len();
    let mut level = Vec::with_capacity(prior.leaf_spans.len().saturating_add(1));
    let mut blocks = BTreeMap::<String, EncodedBlock>::new();
    for span in &prior.leaf_spans[..plan.first_leaf] {
        level.push(reused_leaf_node(prior, &plan.block_index, span, &mut blocks)?);
    }
    let mut current = Vec::new();
    let mut leaves_encoded = 0_u32;
    let mut rechunked_entries = 0_u32;
    let mut resume_from = None;
    for (position, entry) in next_entries.iter().enumerate().skip(plan.start) {
        if current.is_empty() && position > plan.start && position >= plan.aligned_from {
            let prior_position = position + prior_len - next_len;
            if let Some(&leaf_index) = plan.prior_starts.get(&prior_position) {
                resume_from = Some(leaf_index);
                break;
            }
        }
        if rechunked_entries >= bounds.max_rechunked_entries {
            return Ok(None);
        }
        rechunked_entries = rechunked_entries.saturating_add(1);
        let mut candidate = current.clone();
        candidate.push(entry.clone());
        let candidate_size = leaf_encoded_len(profile, &candidate).map_err(|issue| vec![issue])?;
        if !current.is_empty() && candidate_size > profile.max_node_bytes {
            level.push(new_leaf_node(profile, core::mem::take(&mut current), &mut blocks)?);
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
            level.push(new_leaf_node(profile, core::mem::take(&mut current), &mut blocks)?);
            leaves_encoded = leaves_encoded.saturating_add(1);
        }
    }
    if !current.is_empty() {
        level.push(new_leaf_node(profile, current, &mut blocks)?);
        leaves_encoded = leaves_encoded.saturating_add(1);
    }
    if let Some(leaf_index) = resume_from {
        for span in &prior.leaf_spans[leaf_index..] {
            level.push(reused_leaf_node(prior, &plan.block_index, span, &mut blocks)?);
        }
    }
    Ok(Some(Rechunk {
        level,
        blocks,
        leaves_encoded,
        rechunked_entries,
    }))
}

fn new_leaf_node(
    profile: &ProllyProfile,
    chunk: Vec<SemanticEntry>,
    blocks: &mut BTreeMap<String, EncodedBlock>,
) -> Result<LevelNode, Vec<ProllyIssue>> {
    let block = encode_leaf(profile, &chunk)?;
    let range = range_for_entries(&chunk, &block)?;
    blocks.insert(block.node_ref.as_str().to_string(), block.clone());
    Ok(LevelNode { range, block })
}

fn reused_leaf_node(
    prior: IncrementalPrior<'_>,
    block_index: &BTreeMap<&str, &EncodedBlock>,
    span: &LeafSpan,
    blocks: &mut BTreeMap<String, EncodedBlock>,
) -> Result<LevelNode, Vec<ProllyIssue>> {
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
    Ok(LevelNode {
        range,
        block: block.clone(),
    })
}
