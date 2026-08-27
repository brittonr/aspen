use std::collections::BTreeMap;
use std::collections::BTreeSet;

use choregraph_history::Digest;
use choregraph_history::EventClass;
use choregraph_history::EventDraft;
use choregraph_history::EventId;
use choregraph_history::HistoryGraph;
use choregraph_history::HistoryLimits;
use choregraph_history::ParentLink;

use super::MAX_WORLD_HEAD_LABEL_BYTES;
use super::MAX_WORLD_HEAD_METADATA_ENTRIES;
use super::WorldCommitHistoryNode;
use super::WorldHeadBounds;
use super::WorldHeadIssue;
use super::WorldHeadPolicyRef;
use crate::world_commit::WorldCommitRef;

const WORLD_HISTORY_SCHEMA_DOMAIN: &[u8] = b"molten.world-head.choregraph-schema.v1";
const WORLD_HISTORY_SCHEMA_LABEL: &[u8] = b"molten-world-commit-v1";
const WORLD_PARENT_ROLE: &str = "world-parent";
const BLAKE3_PREFIX: &str = "blake3:";
const BLAKE3_DIGEST_BYTES: usize = 32;
const HEX_PAIR_WIDTH: usize = 2;
const ISSUES_PER_HISTORY_NODE: usize = 3;

pub(crate) struct ChoregraphWorldHistory {
    pub graph: HistoryGraph,
    pub events: BTreeMap<WorldCommitRef, EventId>,
}

pub(crate) fn build_choregraph_history(
    nodes: &[WorldCommitHistoryNode],
    policy_ref: &WorldHeadPolicyRef,
    bounds: &WorldHeadBounds,
) -> Result<ChoregraphWorldHistory, Vec<WorldHeadIssue>> {
    let mut issues = validate_history_shape(nodes, bounds);
    if !issues.is_empty() {
        issues.sort();
        issues.dedup();
        return Err(issues);
    }

    let limits = history_limits(bounds).map_err(|issue| vec![issue])?;
    let mut graph = HistoryGraph::new(limits).map_err(|_| vec![WorldHeadIssue::InvalidBounds("choregraph")])?;
    let mut events = BTreeMap::new();
    debug_assert!(events.len() <= bounds.max_history_nodes);
    let mut pending = nodes.to_vec();
    pending.sort_by(|left, right| left.commit.cmp(&right.commit));
    let schema_identity = Digest::derive(WORLD_HISTORY_SCHEMA_DOMAIN, WORLD_HISTORY_SCHEMA_LABEL);
    let policy_identity = digest_from_reference(policy_ref.as_str()).map_err(|issue| vec![issue])?;

    while !pending.is_empty() {
        let mut next_pending = Vec::with_capacity(pending.len());
        let mut admitted_count = 0_usize;
        for node in pending {
            if !node.parents.iter().all(|parent| events.contains_key(parent)) {
                next_pending.push(node);
                continue;
            }
            let payload_identity = digest_from_reference(node.commit.as_str()).map_err(|issue| vec![issue])?;
            let mut parents = node
                .parents
                .iter()
                .map(|parent| ParentLink {
                    role: WORLD_PARENT_ROLE.to_string(),
                    event: events[parent],
                })
                .collect::<Vec<_>>();
            parents.sort();
            let is_merge = parents.len() > 1;
            let draft = EventDraft {
                version: choregraph_history::HISTORY_EVENT_VERSION_V1,
                class: if is_merge {
                    EventClass::Merge
                } else {
                    EventClass::Application
                },
                payload_identity,
                parents,
                schema_identity,
                projection_identity: is_merge.then_some(policy_identity),
                metadata: Vec::new(),
            };
            let admission = graph.admit(draft).map_err(|_| vec![WorldHeadIssue::ChoregraphDenied])?;
            if events.len() >= bounds.max_history_nodes {
                return Err(vec![WorldHeadIssue::HistoryLimitExceeded]);
            }
            events.insert(node.commit, admission.event);
            graph = admission.graph;
            admitted_count = admitted_count.saturating_add(1);
        }
        if admitted_count == 0 {
            return Err(vec![WorldHeadIssue::HistoryCycle]);
        }
        pending = next_pending;
    }

    Ok(ChoregraphWorldHistory { graph, events })
}

fn validate_history_shape(nodes: &[WorldCommitHistoryNode], bounds: &WorldHeadBounds) -> Vec<WorldHeadIssue> {
    let issue_capacity = nodes.len().saturating_mul(ISSUES_PER_HISTORY_NODE);
    let mut issues = Vec::with_capacity(issue_capacity);
    if nodes.is_empty() {
        issues.push(WorldHeadIssue::HistoryEmpty);
        return issues;
    }
    if nodes.len() > bounds.max_history_nodes {
        issues.push(WorldHeadIssue::HistoryLimitExceeded);
    }
    let known = nodes.iter().map(|node| node.commit.clone()).collect::<BTreeSet<_>>();
    if known.len() != nodes.len() {
        issues.push(WorldHeadIssue::DuplicateHistoryNode);
    }
    for node in nodes {
        if node.parents.len() > bounds.max_parents_per_commit {
            issues.push(WorldHeadIssue::ParentLimitExceeded);
        }
        let unique = node.parents.iter().collect::<BTreeSet<_>>();
        if unique.len() != node.parents.len() {
            issues.push(WorldHeadIssue::DuplicateHistoryNode);
        }
        if node.parents.iter().any(|parent| !known.contains(parent)) {
            issues.push(WorldHeadIssue::MissingHistoryParent);
        }
        if node.parents.contains(&node.commit) {
            issues.push(WorldHeadIssue::HistoryCycle);
        }
    }
    issues
}

fn history_limits(bounds: &WorldHeadBounds) -> Result<HistoryLimits, WorldHeadIssue> {
    let max_events =
        u32::try_from(bounds.max_history_nodes).map_err(|_| WorldHeadIssue::InvalidBounds("max_history_nodes"))?;
    let max_parents = u32::try_from(bounds.max_parents_per_commit)
        .map_err(|_| WorldHeadIssue::InvalidBounds("max_parents_per_commit"))?;
    let max_metadata = u32::try_from(MAX_WORLD_HEAD_METADATA_ENTRIES)
        .map_err(|_| WorldHeadIssue::InvalidBounds("max_metadata_entries"))?;
    let max_label_bytes =
        u32::try_from(MAX_WORLD_HEAD_LABEL_BYTES).map_err(|_| WorldHeadIssue::InvalidBounds("max_label_bytes"))?;
    HistoryLimits::new(max_events, max_parents, max_metadata, max_label_bytes, max_events)
        .map_err(|_| WorldHeadIssue::InvalidBounds("choregraph"))
}

pub(crate) fn digest_from_reference(reference: &str) -> Result<Digest, WorldHeadIssue> {
    let digest = reference.strip_prefix(BLAKE3_PREFIX).ok_or(WorldHeadIssue::ChoregraphDenied)?;
    if digest.len() != BLAKE3_DIGEST_BYTES.saturating_mul(HEX_PAIR_WIDTH) {
        return Err(WorldHeadIssue::ChoregraphDenied);
    }
    let mut bytes = [0_u8; BLAKE3_DIGEST_BYTES];
    for (index, output) in bytes.iter_mut().enumerate() {
        let start = index.saturating_mul(HEX_PAIR_WIDTH);
        let end = start.saturating_add(HEX_PAIR_WIDTH);
        let pair = digest.get(start..end).ok_or(WorldHeadIssue::ChoregraphDenied)?;
        *output = u8::from_str_radix(pair, 16).map_err(|_| WorldHeadIssue::ChoregraphDenied)?;
    }
    Ok(Digest::from_bytes(bytes))
}
