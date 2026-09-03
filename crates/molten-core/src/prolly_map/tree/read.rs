#![allow(
    tigerstyle::borrowed_argument_types,
    reason = "snapshot validators append bounded diagnostics to the caller-owned vector"
)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::super::*;
use super::encoding::decode_block;

type OptionalKeyRange = Option<(Vec<u8>, Vec<u8>)>;
type WalkResult = Result<OptionalKeyRange, Vec<ProllyIssue>>;

// r[impl molten.prolly_map.canonical_nodes]
// r[impl molten.prolly_map.history_independence]
pub fn validate_snapshot(profile: &ProllyProfile, snapshot: &MapSnapshot) -> Result<MapReadResult, Vec<ProllyIssue>> {
    let mut issues = validate_profile(profile);
    validate_root(profile, &snapshot.root, &mut issues);
    let blocks = block_index(&snapshot.blocks, &mut issues);
    if !issues.is_empty() {
        issues.sort();
        issues.dedup();
        return Err(issues);
    }
    let mut state = WalkState::default();
    walk_node(profile, &blocks, &snapshot.root.top_node_ref, snapshot.root.height, &mut state)?;
    let unexpected = blocks
        .keys()
        .filter(|node_ref| !state.closure.contains(node_ref.as_str()))
        .map(|node_ref| ProllyIssue::UnexpectedBlock(node_ref.clone()))
        .collect::<Vec<_>>();
    if !unexpected.is_empty() {
        return Err(unexpected);
    }
    let entry_count = u32::try_from(state.entries.len()).map_err(|_| vec![ProllyIssue::EntryLimitExceeded])?;
    if entry_count != snapshot.root.entry_count {
        return Err(vec![ProllyIssue::RootEntryCountMismatch]);
    }
    let entry_issues = super::encoding::validate_entries(profile, &state.entries);
    if !entry_issues.is_empty() {
        return Err(entry_issues);
    }
    let visited_nodes = u32::try_from(state.closure.len()).map_err(|_| vec![ProllyIssue::GraphLimitExceeded])?;
    Ok(MapReadResult {
        root_ref: snapshot.root.root_ref.clone(),
        entries: state.entries,
        closure: state.closure.into_iter().map(NodeRef::new).collect(),
        graph_facts: state.graph_facts,
        visited_nodes,
    })
}

pub fn point_read(
    profile: &ProllyProfile,
    snapshot: &MapSnapshot,
    key: &[u8],
) -> Result<Option<SemanticEntry>, Vec<ProllyIssue>> {
    let read = validate_snapshot(profile, snapshot)?;
    match read.entries.binary_search_by(|entry| entry.key.as_slice().cmp(key)) {
        Ok(index) => Ok(read.entries.get(index).cloned()),
        Err(_) => Ok(None),
    }
}

pub fn range_read(
    profile: &ProllyProfile,
    snapshot: &MapSnapshot,
    start: &[u8],
    end: &[u8],
) -> Result<MapReadResult, Vec<ProllyIssue>> {
    if start > end {
        return Err(vec![ProllyIssue::ChildRangeInvalid]);
    }
    let mut read = validate_snapshot(profile, snapshot)?;
    read.entries.retain(|entry| entry.key.as_slice() >= start && entry.key.as_slice() <= end);
    Ok(read)
}

#[derive(Default)]
struct WalkState {
    entries: Vec<SemanticEntry>,
    closure: BTreeSet<String>,
    active: BTreeSet<String>,
    graph_facts: Vec<GraphFact>,
}

fn walk_node(
    profile: &ProllyProfile,
    blocks: &BTreeMap<String, &EncodedBlock>,
    node_ref: &NodeRef,
    height: u16,
    state: &mut WalkState,
) -> WalkResult {
    if state.closure.contains(node_ref.as_str()) {
        return existing_range(profile, blocks, node_ref);
    }
    if !state.active.insert(node_ref.as_str().to_string()) {
        return Err(vec![ProllyIssue::TreeCycle(node_ref.as_str().to_string())]);
    }
    let block = blocks
        .get(node_ref.as_str())
        .ok_or_else(|| vec![ProllyIssue::MissingBlock(node_ref.as_str().to_string())])?;
    let node = decode_block(profile, block)?;
    let result = match node {
        ProllyNode::Leaf(leaf) => walk_leaf(leaf, height, state)?,
        ProllyNode::Internal(internal) => walk_internal(profile, blocks, internal, height, state)?,
    };
    state.active.remove(node_ref.as_str());
    state.closure.insert(node_ref.as_str().to_string());
    Ok(result)
}

fn walk_leaf(leaf: LeafNode, height: u16, state: &mut WalkState) -> WalkResult {
    if height != 0 {
        return Err(vec![ProllyIssue::TreeHeightExceeded]);
    }
    let range = leaf
        .entries
        .first()
        .zip(leaf.entries.last())
        .map(|(first, last)| (first.key.clone(), last.key.clone()));
    state.graph_facts.push(GraphFact {
        node_ref: leaf.node_ref,
        children: Vec::new(),
        complete: true,
    });
    state.entries.extend(leaf.entries);
    Ok(range)
}

fn walk_internal(
    profile: &ProllyProfile,
    blocks: &BTreeMap<String, &EncodedBlock>,
    internal: InternalNode,
    height: u16,
    state: &mut WalkState,
) -> WalkResult {
    let child_height = height.checked_sub(1).ok_or_else(|| vec![ProllyIssue::TreeHeightExceeded])?;
    let node_ref = internal.node_ref.clone();
    let children = internal.children.iter().map(|child| child.node_ref.clone()).collect::<Vec<_>>();
    let mut actual_min = None;
    let mut actual_max = None;
    for child in internal.children {
        let range = walk_node(profile, blocks, &child.node_ref, child_height, state)?
            .ok_or_else(|| vec![ProllyIssue::ChildRangeInvalid])?;
        if range.0 != child.min_key || range.1 != child.max_key {
            return Err(vec![ProllyIssue::ChildRangeInvalid]);
        }
        if actual_min.is_none() {
            actual_min = Some(range.0);
        }
        actual_max = Some(range.1);
    }
    state.graph_facts.push(GraphFact {
        node_ref,
        children,
        complete: true,
    });
    Ok(actual_min.zip(actual_max))
}

fn existing_range(profile: &ProllyProfile, blocks: &BTreeMap<String, &EncodedBlock>, node_ref: &NodeRef) -> WalkResult {
    let block = blocks
        .get(node_ref.as_str())
        .ok_or_else(|| vec![ProllyIssue::MissingBlock(node_ref.as_str().to_string())])?;
    match decode_block(profile, block)? {
        ProllyNode::Leaf(leaf) => Ok(leaf
            .entries
            .first()
            .zip(leaf.entries.last())
            .map(|(first, last)| (first.key.clone(), last.key.clone()))),
        ProllyNode::Internal(internal) => Ok(internal
            .children
            .first()
            .zip(internal.children.last())
            .map(|(first, last)| (first.min_key.clone(), last.max_key.clone()))),
    }
}

fn validate_root(profile: &ProllyProfile, root: &ProllyRoot, issues: &mut Vec<ProllyIssue>) {
    if root.schema != PROLLY_ROOT_SCHEMA {
        issues.push(ProllyIssue::RootSchemaMismatch);
    }
    if root.profile_ref != profile.profile_ref {
        issues.push(ProllyIssue::RootProfileMismatch);
    }
    if root.height > profile.limits.max_tree_height {
        issues.push(ProllyIssue::TreeHeightExceeded);
    }
    if !is_content_ref(root.top_node_ref.as_str()) || !is_content_ref(root.root_ref.as_str()) {
        issues.push(ProllyIssue::MalformedReference(root.root_ref.as_str().to_string()));
    }
    match super::build::derive_root_ref(profile, &root.top_node_ref, root.height, root.entry_count) {
        Ok(expected) if expected != root.root_ref => issues.push(ProllyIssue::RootIdentityMismatch),
        Ok(_) => {}
        Err(issue) => issues.push(issue),
    }
}

fn block_index<'a>(blocks: &'a [EncodedBlock], issues: &mut Vec<ProllyIssue>) -> BTreeMap<String, &'a EncodedBlock> {
    let mut index = BTreeMap::new();
    for block in blocks {
        if !is_content_ref(block.node_ref.as_str()) {
            issues.push(ProllyIssue::MalformedReference(block.node_ref.as_str().to_string()));
        }
        if let Some(prior) = index.insert(block.node_ref.as_str().to_string(), block) {
            issues.push(if prior.bytes == block.bytes {
                ProllyIssue::DuplicateBlock(block.node_ref.as_str().to_string())
            } else {
                ProllyIssue::BlockBytesMismatch(block.node_ref.as_str().to_string())
            });
        }
    }
    index
}
