use super::*;

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.repair_admission]
#[test]
fn repaired_storage_that_agrees_with_a_quorum_of_peers_is_admitted_as_a_voter() {
    let committed = [(1, "a"), (2, "b")];
    let outcome = reconcile_participant(&base(&committed, 2, vec![
        peer("node-b", 3, 2, &committed),
        peer("node-c", 3, 2, &committed),
    ]));
    assert!(outcome.is_reconciled);
    assert!(outcome.effects.is_empty());
    assert_eq!(outcome.admission.role, Some(RejoinRole::Voter));
}

// r[verify molten.consensus.repair_admission]
#[test]
fn local_agreement_alone_never_admits_a_suspect_participant() {
    let committed = [(1, "a"), (2, "b")];
    let none = reconcile_participant(&base(&committed, 2, Vec::new()));
    assert!(!none.is_reconciled, "no peer reported, so no quorum formed with the participant");
    assert_eq!(none.admission.role, None);
    assert_eq!(none.admission.blockers, vec![AdmissionBlocker::ProtocolNotReconciled]);
    let mut trusted = base(&committed, 2, Vec::new());
    trusted.is_persistent_state_suspect = false;
    assert_eq!(reconcile_participant(&trusted).admission.role, Some(RejoinRole::Voter));
}

// r[verify molten.consensus.storage_fault_recovery]
#[test]
fn a_missing_committed_entry_is_fetched_before_any_admission() {
    let committed = [(1, "a"), (2, "b")];
    let outcome = reconcile_participant(&base(&[(1, "a")], 1, vec![
        peer("node-b", 3, 2, &committed),
        peer("node-c", 3, 2, &committed),
    ]));
    assert!(!outcome.is_reconciled);
    assert_eq!(outcome.effects, vec![RecoveryEffect::FetchEntry {
        from_peer: "node-b".to_string(),
        index: 2,
        entry: entry(3, "b"),
    }]);
    assert_eq!(outcome.admission.role, None);
}

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.storage_fault_validation]
#[test]
fn a_local_suffix_a_quorum_lacks_is_removed_and_a_committed_entry_never_is() {
    let outcome = reconcile_participant(&base(&[(1, "a"), (2, "local-only")], 1, vec![
        peer("node-b", 3, 1, &[(1, "a")]),
        peer("node-c", 3, 1, &[(1, "a")]),
    ]));
    assert_eq!(outcome.effects, vec![RecoveryEffect::RemoveSuffixFrom { index: 2 }]);
    assert!(!outcome.is_reconciled);
    let committed_elsewhere = reconcile_participant(&base(&[(1, "a"), (2, "b")], 1, vec![
        peer("node-b", 3, 2, &[(1, "a"), (2, "b")]),
        peer("node-c", 3, 1, &[(1, "a")]),
    ]));
    assert!(committed_elsewhere.effects.is_empty(), "a committed entry is never removed");
}

// r[verify molten.consensus.storage_fault_validation]
#[test]
fn conflicting_committed_history_plans_nothing_and_admits_nobody() {
    let outcome = reconcile_participant(&base(&[(1, "a")], 1, vec![
        peer("node-b", 3, 1, &[(1, "x")]),
        peer("node-c", 3, 1, &[(1, "y")]),
    ]));
    assert_eq!(outcome.decisions, vec![(1, EntryRepairDecision::ConflictingHistory)]);
    assert!(outcome.effects.is_empty());
    assert!(!outcome.is_reconciled);
    assert_eq!(outcome.admission.role, None);
}

// r[verify molten.consensus.storage_fault_recovery]
#[test]
fn a_compacted_peer_repairs_the_prefix_with_a_snapshot_not_suffix_removal() {
    let mut compacted = peer("node-b", 3, 5, &[]);
    compacted.snapshot = Some(SnapshotFact {
        last_included_index: 5,
        last_included_term: 3,
        content_ref: "blake3:snapshot".to_string(),
    });
    let outcome = reconcile_participant(&base(&[(3, "held")], 0, vec![compacted, peer("node-c", 3, 0, &[])]));
    assert_eq!(outcome.effects, vec![RecoveryEffect::InstallSnapshot {
        from_peer: "node-b".to_string(),
        snapshot: SnapshotFact {
            last_included_index: 5,
            last_included_term: 3,
            content_ref: "blake3:snapshot".to_string(),
        },
    }]);
}

// r[verify molten.consensus.repair_admission]
#[test]
fn a_lost_vote_record_admits_only_a_non_voter_until_the_term_passes_its_fence() {
    let committed = [(1, "a")];
    let mut input = base(&committed, 1, vec![peer("node-b", 5, 1, &committed), peer("node-c", 5, 1, &committed)]);
    input.local_hard_state = LocalHardState::Lost;
    input.current_term = 5;
    let fenced = reconcile_participant(&input);
    assert_eq!(fenced.hard_state, HardStateDecision::VoteFenced { through_term: 5 });
    assert_eq!(fenced.admission.role, Some(RejoinRole::NonVoting));
    assert_eq!(fenced.admission.blockers, vec![AdmissionBlocker::VoteFenced]);
    input.current_term = 6;
    assert_eq!(reconcile_participant(&input).admission.role, Some(RejoinRole::Voter));
}

// r[verify molten.consensus.repair_admission]
#[test]
fn a_lost_vote_record_with_no_peer_term_is_unresolved() {
    let mut input = base(&[(1, "a")], 1, Vec::new());
    input.local_hard_state = LocalHardState::Lost;
    let outcome = reconcile_participant(&input);
    assert_eq!(outcome.hard_state, HardStateDecision::Wait);
    assert_eq!(outcome.admission.role, None);
    assert!(outcome.admission.blockers.contains(&AdmissionBlocker::HardStateUnresolved));
}
