mod baseline;
mod bounds;
mod safety;
mod snapshots;

use super::*;

fn peer(id: &str, term: u64, commit_index: u64, held: &[(u64, &str)]) -> PeerRecovery {
    PeerRecovery {
        peer_id: id.to_string(),
        current_term: term,
        commit_index,
        entries: held.iter().map(|(index, label)| (*index, entry(3, label))).collect(),
        snapshot: None,
        is_complete: true,
    }
}

fn both(report: PeerRecovery) -> Vec<PeerRecovery> {
    let mut other = report.clone();
    other.peer_id = "node-c".to_string();
    vec![report, other]
}

fn base(local: &[(u64, &str)], local_commit_index: u64, peers: Vec<PeerRecovery>) -> ReconcileInput {
    ReconcileInput {
        participant_id: SELF.to_string(),
        voters: voters(),
        current_term: 3,
        local_commit_index,
        local_entries: local
            .iter()
            .map(|(index, label)| (*index, LocalEntry::Present(entry(3, label))))
            .collect::<std::collections::BTreeMap<_, _>>(),
        local_hard_state: LocalHardState::Present {
            term: 3,
            voted_for: None,
        },
        local_snapshot: None,
        peers,
        is_persistent_state_suspect: true,
        is_storage_serving: true,
        is_membership_current: true,
        is_log_or_snapshot_validated: true,
    }
}

#[test]
fn committed_local_history_is_not_replaced_even_when_peers_agree_on_another_entry() {
    let outcome = reconcile_participant(&base(&[(1, "local")], 1, vec![
        peer("node-b", 3, 1, &[(1, "other")]),
        peer("node-c", 3, 1, &[(1, "other")]),
    ]));
    assert_eq!(outcome.decisions, vec![(1, EntryRepairDecision::ConflictingHistory)]);
    assert!(outcome.effects.is_empty());
    assert_eq!(outcome.admission.role, None);
}

#[test]
fn an_uncommitted_local_entry_can_be_replaced_by_committed_history() {
    let outcome = reconcile_participant(&base(&[(1, "local")], 0, vec![peer("node-b", 3, 1, &[(1, "committed")])]));
    assert_eq!(outcome.effects, vec![RecoveryEffect::FetchEntry {
        from_peer: "node-b".to_string(),
        index: 1,
        entry: entry(3, "committed"),
    }]);
    assert_eq!(outcome.admission.role, None);
}

#[test]
fn an_uncommitted_suffix_held_by_a_quorum_is_retained_without_advancing_commitment() {
    let input = base(&[(1, "held")], 0, both(peer("node-b", 3, 0, &[(1, "held")])));
    let outcome = reconcile_participant(&input);
    assert_eq!(outcome.decisions, vec![(1, EntryRepairDecision::KeepLocal)]);
    assert!(outcome.is_reconciled);
    assert!(outcome.effects.is_empty());
    assert_eq!(outcome.admission.role, Some(RejoinRole::Voter));
}

#[test]
fn an_unknown_conflicting_uncommitted_suffix_stays_unresolved() {
    let outcome = reconcile_participant(&base(&[(1, "held")], 0, vec![peer("node-b", 3, 0, &[(1, "other")])]));
    assert_eq!(outcome.decisions, vec![(1, EntryRepairDecision::Wait)]);
    assert_eq!(outcome.admission.role, None);
}

#[test]
fn missing_and_faulty_locally_committed_entries_are_never_treated_as_uncommitted() {
    for local in [LocalEntry::Missing, LocalEntry::Faulty] {
        let mut input = base(&[], 1, vec![peer("node-b", 3, 0, &[]), peer("node-c", 3, 0, &[])]);
        input.local_entries.insert(1, local);
        let outcome = reconcile_participant(&input);
        assert_eq!(outcome.decisions, vec![(1, EntryRepairDecision::Wait)]);
        assert!(outcome.effects.iter().all(|effect| !matches!(effect, RecoveryEffect::RemoveSuffixFrom { .. })));
        assert_eq!(outcome.admission.role, None);
    }
}

#[test]
fn a_hole_in_local_committed_coverage_is_repaired_even_if_not_explicitly_reported() {
    let outcome = reconcile_participant(&base(&[(2, "b")], 2, vec![peer("node-b", 3, 2, &[(1, "a"), (2, "b")])]));
    assert_eq!(outcome.effects, vec![RecoveryEffect::FetchEntry {
        from_peer: "node-b".to_string(),
        index: 1,
        entry: entry(3, "a"),
    }]);
    assert_eq!(outcome.admission.role, None);
}

#[test]
fn incomplete_or_gapped_peer_coverage_cannot_attest_absence() {
    let mut incomplete = peer("node-b", 3, 0, &[]);
    incomplete.is_complete = false;
    for report in [incomplete, peer("node-b", 3, 2, &[(2, "b")])] {
        let outcome = reconcile_participant(&base(&[(1, "held")], 0, vec![report]));
        assert!(!outcome.is_reconciled);
        assert!(outcome.effects.is_empty());
        assert_eq!(outcome.admission.role, None);
    }
}

#[test]
fn duplicate_self_and_nonvoter_reports_are_rejected_instead_of_forming_a_quorum() {
    let report = peer("node-b", 3, 1, &[(1, "a")]);
    for reports in [vec![report.clone(), report], vec![peer(SELF, 3, 1, &[(1, "a")])], vec![
        peer("outsider", 3, 1, &[(1, "a")]),
    ]] {
        let outcome = reconcile_participant(&base(&[(1, "a")], 1, reports));
        assert!(!outcome.is_reconciled);
        assert!(outcome.effects.is_empty());
        assert_eq!(outcome.admission.role, None);
    }
}

fn snapshot(index: u64, content: &str) -> SnapshotFact {
    SnapshotFact {
        last_included_index: index,
        last_included_term: 3,
        content_ref: content.to_string(),
    }
}
