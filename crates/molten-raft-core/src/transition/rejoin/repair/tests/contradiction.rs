//! Committed evidence that contradicts a peer snapshot fails closed as a malformed observation.

use super::*;

/// A's snapshot records request R1 committed at index 1 (term 2); C reports a different request
/// R2 committed at index 1 with the same term.
fn contradictory_boundary_plan(
    log: Vec<ReplicatedEntry>,
    is_c_reported: bool,
) -> (MessagePlan<'static>, ReplicatedEntry) {
    let template = crate::tests::support::started_state(crate::tests::support::NODE_B);
    let snapshot = peer_snapshot(&template, RECOVERY_TERM);
    let contradictory = entry(SNAPSHOT_INDEX, RECOVERY_TERM, "contradictory-committed");
    let recorded = crate::tests::support::test_ref("recovery-committed-request");
    assert_eq!(snapshot.completed_requests.get(&recorded), Some(&1));
    assert_ne!(contradictory.request_ref, recorded);
    let mut peers = std::collections::BTreeMap::from([(
        crate::tests::support::NODE_A.to_string(),
        snapshot_observation(&snapshot, SNAPSHOT_INDEX, Vec::new()),
    )]);
    if is_c_reported {
        peers.insert(crate::tests::support::NODE_C.to_string(), RecoveryObservation {
            observation_ref: crate::tests::support::test_ref("contradictory-observation"),
            current_term: RECOVERY_TERM,
            commit_index: SUFFIX_INDEX,
            last_log_index: SUFFIX_INDEX,
            entries: vec![
                contradictory.clone(),
                entry(SUFFIX_INDEX, RECOVERY_TERM, "contradictory-second"),
            ],
            snapshot: None,
            is_complete: true,
        });
    }
    (recovering_plan(log, peers), contradictory)
}

fn assert_divergence_denied(mut plan: MessagePlan<'_>) {
    let before = plan.next.clone();
    let error = reconcile(&mut plan).expect_err("contradictory committed evidence must fail closed");
    assert_eq!(error.class(), crate::denial::ReplicaErrorClass::MalformedObservation, "{error}");
    assert!(error.to_string().contains("diverges from snapshot"), "{error}");
    assert_eq!(plan.next, before, "nothing is rewritten or installed");
    assert!(plan.effects.is_empty());
}

// r[verify molten.consensus.storage_fault_recovery]
#[test]
fn a_same_term_committed_copy_with_another_request_fails_closed_before_any_rewrite() {
    let faulty = vec![
        entry(SNAPSHOT_INDEX, OLDER_TERM, "contradiction-faulty-first"),
        entry(SUFFIX_INDEX, OLDER_TERM, "contradiction-faulty-second"),
    ];
    let (plan, _contradictory) = contradictory_boundary_plan(faulty, true);
    assert!(matches!(
        suffix_decision(&plan),
        molten_core::consensus_recovery::EntryRepairDecision::ReplaceWithCommitted { .. }
    ));
    assert_divergence_denied(plan);
}

// r[verify molten.consensus.storage_fault_recovery]
#[test]
fn a_restart_holding_the_contradictory_boundary_fails_closed_while_committed_evidence_contradicts() {
    // After a restart B's uncommitted log holds R2 at index 1, whose term matches A's snapshot.
    let (_, contradictory) = contradictory_boundary_plan(Vec::new(), false);
    let restarted = vec![
        contradictory,
        entry(SUFFIX_INDEX, RECOVERY_TERM, "contradictory-second"),
    ];
    // While C reports R2 as committed, committed evidence contradicts A's snapshot: fail closed.
    let (plan, _) = contradictory_boundary_plan(restarted.clone(), true);
    assert_divergence_denied(plan);
    // With C lost, B's suspect uncommitted copy is no evidence; nothing verifies index 2, so
    // recovery waits and keeps the local log instead of failing stop or installing.
    let (mut plan, _) = contradictory_boundary_plan(restarted, false);
    let before = plan.next.clone();
    reconcile(&mut plan).expect("a suspect uncommitted copy is not a divergence");
    assert_eq!(plan.next, before);
    assert!(plan.effects.is_empty());
}
