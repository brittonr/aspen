use super::*;

#[test]
fn a_snapshot_is_installed_before_planning_entry_repairs() {
    let mut report = peer("node-b", 3, 6, &[(6, "after")]);
    report.snapshot = Some(snapshot(5, "snapshot"));
    let mut input = base(&[], 0, vec![report]);
    let outcome = reconcile_participant(&input);
    assert_eq!(outcome.effects, vec![RecoveryEffect::InstallSnapshot {
        from_peer: "node-b".to_string(),
        snapshot: snapshot(5, "snapshot"),
    }]);
    assert_eq!(outcome.admission.role, None);
    input.local_snapshot = Some(snapshot(5, "snapshot"));
    input.local_commit_index = 5;
    assert_eq!(reconcile_participant(&input).effects, vec![RecoveryEffect::FetchEntry {
        from_peer: "node-b".to_string(),
        index: 6,
        entry: entry(3, "after"),
    }]);
}

#[test]
fn agreeing_compacted_history_is_not_fetched_as_missing_entries() {
    let mut report = peer("node-b", 3, 6, &[(6, "after")]);
    report.snapshot = Some(snapshot(5, "snapshot"));
    let mut input = base(&[(6, "after")], 6, both(report));
    input.local_snapshot = Some(snapshot(5, "snapshot"));
    let outcome = reconcile_participant(&input);
    assert_eq!(outcome.decisions, vec![(6, EntryRepairDecision::KeepLocal)]);
    assert!(outcome.effects.is_empty());
    assert_eq!(outcome.admission.role, Some(RejoinRole::Voter));
}

#[test]
fn snapshot_conflicts_suppress_otherwise_valid_entry_repairs() {
    let mut report = peer("node-b", 3, 2, &[(2, "b")]);
    report.snapshot = Some(snapshot(1, "remote"));
    let mut input = base(&[], 1, vec![report]);
    input.local_snapshot = Some(snapshot(1, "local"));
    let outcome = reconcile_participant(&input);
    assert_eq!(outcome.snapshot, SnapshotRepairDecision::ConflictingContent);
    assert!(outcome.effects.is_empty());
    assert_eq!(outcome.admission.role, None);
}

#[test]
fn committed_entry_conflicts_suppress_snapshot_installation() {
    let mut compacted = peer("node-b", 3, 2, &[]);
    compacted.snapshot = Some(snapshot(2, "snapshot"));
    let outcome =
        reconcile_participant(&base(&[(1, "local")], 1, vec![compacted, peer("node-c", 3, 1, &[(1, "other")])]));
    assert_eq!(outcome.decisions, vec![(1, EntryRepairDecision::ConflictingHistory)]);
    assert!(matches!(outcome.snapshot, SnapshotRepairDecision::InstallFrom { .. }));
    assert!(outcome.effects.is_empty());
    assert_eq!(outcome.admission.role, None);
}

#[test]
fn snapshots_beyond_commit_or_overlapping_reported_entries_are_rejected() {
    let mut beyond = peer("node-b", 3, 0, &[]);
    beyond.snapshot = Some(snapshot(1, "snapshot"));
    let mut overlap = peer("node-b", 3, 1, &[(1, "a")]);
    overlap.snapshot = Some(snapshot(1, "snapshot"));
    for report in [beyond, overlap] {
        let outcome = reconcile_participant(&base(&[], 0, vec![report]));
        assert!(outcome.effects.is_empty());
        assert_eq!(outcome.admission.role, None);
    }
}

#[test]
fn snapshot_boundary_term_conflicts_with_local_commit_are_preserved() {
    let mut report = peer("node-b", 4, 1, &[]);
    report.snapshot = Some(SnapshotFact {
        last_included_term: 4,
        ..snapshot(1, "snapshot")
    });
    let outcome = reconcile_participant(&base(&[(1, "local")], 1, vec![report]));
    assert_eq!(outcome.snapshot, SnapshotRepairDecision::ConflictingContent);
    assert!(outcome.effects.is_empty());
    assert_eq!(outcome.admission.role, None);
}
#[test]
fn a_peer_snapshot_covered_by_validated_local_commit_does_not_roll_back_state() {
    for boundary in [1, 2] {
        let mut report = peer("node-b", 3, 2, if boundary == 1 { &[(2, "b")] } else { &[] });
        report.snapshot = Some(snapshot(boundary, "snapshot"));
        let outcome = reconcile_participant(&base(&[(1, "a"), (2, "b")], 2, both(report)));
        assert_eq!(outcome.snapshot, SnapshotRepairDecision::KeepLocal);
        assert!(outcome.effects.is_empty());
        assert_eq!(outcome.admission.role, Some(RejoinRole::Voter));
    }
}

#[test]
fn an_older_snapshot_cannot_roll_back_committed_state_without_a_validated_boundary() {
    let mut report = peer("node-b", 3, 2, &[(2, "b")]);
    report.snapshot = Some(snapshot(1, "snapshot"));
    let outcome = reconcile_participant(&base(&[(2, "b")], 2, vec![report]));
    assert_eq!(outcome.snapshot, SnapshotRepairDecision::Wait);
    assert!(outcome.effects.iter().all(|effect| matches!(effect, RecoveryEffect::QueryPeers { .. })));
    assert_eq!(outcome.admission.role, None);
}

#[test]
fn snapshot_conflicts_at_an_older_boundary_are_not_hidden_by_a_newer_snapshot() {
    let mut older = peer("node-b", 3, 1, &[]);
    older.snapshot = Some(snapshot(1, "remote"));
    let mut newer = peer("node-c", 3, 2, &[]);
    newer.snapshot = Some(snapshot(2, "newer"));
    let mut input = base(&[], 1, vec![older, newer]);
    input.local_snapshot = Some(snapshot(1, "local"));
    let outcome = reconcile_participant(&input);
    assert_eq!(outcome.snapshot, SnapshotRepairDecision::ConflictingContent);
    assert!(outcome.effects.is_empty());
    assert_eq!(outcome.admission.role, None);
}
