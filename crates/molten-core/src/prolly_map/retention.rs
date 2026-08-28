use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;

// r[impl molten.prolly_map.retention]
pub fn plan_gc(
    profile: &ProllyProfile,
    all_nodes: &[NodeRef],
    roots: &[NodeRef],
    pins: &[NodeRef],
    facts: &[GraphFact],
) -> Result<GcPlan, Vec<ProllyIssue>> {
    if length_exceeds(facts.len(), profile.limits.max_graph_facts)
        || length_exceeds(all_nodes.len(), profile.limits.max_graph_facts)
    {
        return Err(vec![ProllyIssue::GraphLimitExceeded]);
    }
    let mut issues = validate_refs(all_nodes, roots, pins);
    let mut index = BTreeMap::<String, &GraphFact>::new();
    for fact in facts {
        if index.insert(fact.node_ref.as_str().to_string(), fact).is_some() {
            issues.push(ProllyIssue::DuplicateGraphFact(fact.node_ref.as_str().to_string()));
        }
    }
    if !issues.is_empty() {
        issues.sort();
        issues.dedup();
        return Err(issues);
    }
    let mut reachable = BTreeSet::new();
    let mut pending = roots.iter().chain(pins).cloned().collect::<Vec<_>>();
    let mut diagnostics = Vec::new();
    let mut is_complete = true;
    while let Some(node_ref) = pending.pop() {
        if !reachable.insert(node_ref.as_str().to_string()) {
            continue;
        }
        let Some(fact) = index.get(node_ref.as_str()) else {
            is_complete = false;
            diagnostics.push(format!("missing-graph-fact:{}", node_ref.as_str()));
            continue;
        };
        if !fact.complete {
            is_complete = false;
            diagnostics.push(format!("incomplete-graph-fact:{}", node_ref.as_str()));
        }
        pending.extend(fact.children.iter().cloned());
    }
    diagnostics.sort();
    diagnostics.dedup();
    let all = all_nodes.iter().map(NodeRef::as_str).collect::<BTreeSet<_>>();
    let candidate_unreachable = all
        .difference(&reachable.iter().map(String::as_str).collect::<BTreeSet<_>>())
        .map(|value| NodeRef::new((*value).to_string()))
        .collect::<Vec<_>>();
    Ok(GcPlan {
        profile_ref: profile.profile_ref.clone(),
        roots: sorted_refs(roots),
        pins: sorted_refs(pins),
        reachable: reachable.into_iter().map(NodeRef::new).collect(),
        candidate_unreachable,
        complete: is_complete,
        deletion_authorized: false,
        diagnostics,
    })
}

pub fn facts_from_snapshot(
    profile: &ProllyProfile,
    snapshot: &MapSnapshot,
) -> Result<Vec<GraphFact>, Vec<ProllyIssue>> {
    validate_snapshot(profile, snapshot).map(|read| read.graph_facts)
}

fn validate_refs(all_nodes: &[NodeRef], roots: &[NodeRef], pins: &[NodeRef]) -> Vec<ProllyIssue> {
    let mut issues = Vec::new();
    for node_ref in all_nodes.iter().chain(roots).chain(pins) {
        if !is_content_ref(node_ref.as_str()) {
            issues.push(ProllyIssue::MalformedReference(node_ref.as_str().to_string()));
        }
    }
    issues
}

fn sorted_refs(values: &[NodeRef]) -> Vec<NodeRef> {
    let mut values = values.to_vec();
    values.sort();
    values.dedup();
    values
}

fn length_exceeds(length: usize, maximum: u32) -> bool {
    match u32::try_from(length) {
        Ok(length) => length > maximum,
        Err(_) => true,
    }
}
