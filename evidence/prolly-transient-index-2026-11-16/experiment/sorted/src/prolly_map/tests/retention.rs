use std::collections::BTreeMap;

use super::*;

// r[verify molten.prolly_map.retention]
#[test]
fn complete_graph_plans_candidates_but_never_grants_deletion() {
    let profile = profile();
    let old = build();
    let changed = SemanticEntry {
        key: entry(UPDATE_INDEX).key,
        value: vec![UPDATED_VALUE_BYTE; VALUE_BYTES],
    };
    let edit = plan_edits(&profile, &old.snapshot, &[MapEdit::Update(changed)]).expect("edit");
    let all_nodes = old
        .snapshot
        .blocks
        .iter()
        .chain(&edit.next.snapshot.blocks)
        .map(|block| block.node_ref.clone())
        .collect::<Vec<_>>();
    let facts = merged_facts(&profile, &[&old.snapshot, &edit.next.snapshot]);
    let plan = plan_gc(&profile, &all_nodes, core::slice::from_ref(&edit.next.snapshot.root.top_node_ref), &[], &facts)
        .expect("complete plan");
    assert!(plan.complete);
    assert!(!plan.deletion_authorized);
    assert!(!plan.candidate_unreachable.is_empty());

    let pinned = plan_gc(
        &profile,
        &all_nodes,
        core::slice::from_ref(&edit.next.snapshot.root.top_node_ref),
        core::slice::from_ref(&old.snapshot.root.top_node_ref),
        &facts,
    )
    .expect("pinned plan");
    assert!(pinned.candidate_unreachable.len() < plan.candidate_unreachable.len());
    assert!(!pinned.deletion_authorized);
}

#[test]
fn incomplete_graph_and_duplicate_facts_fail_closed() {
    let profile = profile();
    let build = build();
    let all_nodes = build.snapshot.blocks.iter().map(|block| block.node_ref.clone()).collect::<Vec<_>>();
    let mut facts = facts_from_snapshot(&profile, &build.snapshot).expect("facts");
    let removed = facts.pop().expect("fact");
    let incomplete =
        plan_gc(&profile, &all_nodes, core::slice::from_ref(&build.snapshot.root.top_node_ref), &[], &facts)
            .expect("incomplete plan");
    assert!(!incomplete.complete);
    assert!(!incomplete.deletion_authorized);
    assert!(incomplete.diagnostics.iter().any(|item| item.contains("missing-graph-fact")));

    facts.push(removed.clone());
    facts.push(removed.clone());
    assert!(
        plan_gc(&profile, &all_nodes, core::slice::from_ref(&build.snapshot.root.top_node_ref), &[], &facts,)
            .expect_err("duplicate fact")
            .contains(&ProllyIssue::DuplicateGraphFact(removed.node_ref.as_str().to_string()))
    );
}

fn merged_facts(profile: &ProllyProfile, snapshots: &[&MapSnapshot]) -> Vec<GraphFact> {
    let mut facts = BTreeMap::new();
    for snapshot in snapshots {
        for fact in facts_from_snapshot(profile, snapshot).expect("snapshot facts") {
            facts.insert(fact.node_ref.as_str().to_string(), fact);
        }
    }
    facts.into_values().collect()
}
