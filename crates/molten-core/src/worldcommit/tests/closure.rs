use super::*;

#[test]
fn closure_reports_first_missing_root_and_parent_cycles() {
    // r[verify molten.world_commit.restore]
    // r[verify molten.world_commit.verification]
    let plan = plan_capture(&logical_request()).expect("capture plan");
    let commit_ref = identify_world_commit(b"subject").expect("subject ref");
    let parent_ref = identify_world_commit(b"parent").expect("parent ref");
    let mut request = complete_closure(commit_ref.clone(), plan.core);
    request.core.parents.push(parent_ref.clone());
    request.roots.remove(0);
    request.parent_graph.push(ParentClosureObservation {
        commit_ref: parent_ref,
        parents: vec![commit_ref],
        object_present: true,
    });

    let report = validate_closure(&request);

    assert!(!report.complete);
    assert_eq!(report.first_missing_root, Some(RootKind::Artifact));
    assert!(report.issues.contains(&ClosureIssue::MissingRootObject(RootKind::Artifact)));
    assert!(report.issues.iter().any(|issue| matches!(issue, ClosureIssue::ParentCycle(_))));
}

fn parent_observation(commit_ref: &WorldCommitRef, parents: &[&WorldCommitRef]) -> ParentClosureObservation {
    ParentClosureObservation {
        commit_ref: commit_ref.clone(),
        parents: parents.iter().map(|parent| (*parent).clone()).collect(),
        object_present: true,
    }
}

/// Subject `S` with a diamond ancestry `S -> {L, R}`, `L -> B`, `R -> B`.
fn diamond_closure() -> ClosureRequest {
    let plan = plan_capture(&logical_request()).expect("capture plan");
    let subject = identify_world_commit(b"diamond-subject").expect("subject ref");
    let left = identify_world_commit(b"diamond-left").expect("left ref");
    let right = identify_world_commit(b"diamond-right").expect("right ref");
    let base = identify_world_commit(b"diamond-base").expect("base ref");
    let mut request = complete_closure(subject, plan.core);
    request.core.parents = vec![left.clone(), right.clone()];
    request.parent_graph = vec![
        parent_observation(&left, &[&base]),
        parent_observation(&right, &[&base]),
        parent_observation(&base, &[]),
    ];
    request
}

#[test]
fn closure_accepts_valid_diamond_ancestry() {
    // r[verify molten.world_commit.merge_conservative_precedence_and_full_graph_closure]
    let request = diamond_closure();
    let report = validate_closure(&request);
    assert!(report.complete, "diamond ancestry is a complete closure: {:?}", report.issues);
    assert!(report.issues.is_empty());
}

#[test]
fn closure_rejects_detached_cycle_and_unreachable_observations() {
    // r[verify molten.world_commit.merge_conservative_precedence_and_full_graph_closure]
    let mut detached = diamond_closure();
    let cycle_x = identify_world_commit(b"detached-x").expect("x ref");
    let cycle_y = identify_world_commit(b"detached-y").expect("y ref");
    detached.parent_graph.push(parent_observation(&cycle_x, &[&cycle_y]));
    detached.parent_graph.push(parent_observation(&cycle_y, &[&cycle_x]));
    let report = validate_closure(&detached);
    assert!(!report.complete);
    assert!(report.issues.iter().any(|issue| matches!(issue, ClosureIssue::ParentCycle(_))));
    for detached_ref in [&cycle_x, &cycle_y] {
        assert!(
            report
                .issues
                .contains(&ClosureIssue::UnreachableParentObservation(detached_ref.as_str().to_string()))
        );
    }
    assert!(!report.issues.iter().any(|issue| matches!(issue, ClosureIssue::MissingParentObservation(_))));

    let mut orphaned = diamond_closure();
    let orphan = identify_world_commit(b"orphan").expect("orphan ref");
    orphaned.parent_graph.push(parent_observation(&orphan, &[]));
    let report = validate_closure(&orphaned);
    assert!(!report.complete);
    assert_eq!(report.issues, vec![ClosureIssue::UnreachableParentObservation(orphan.as_str().to_string())]);
}
