use super::*;

#[test]
fn reporters_must_intersect_commit_quorums_without_the_suspect_participant() {
    let held = [(1, "a"), (2, "b"), (3, "c"), (4, "d")];
    let mut input = base(&held, 4, vec![peer("node-b", 3, 4, &held)]);
    assert_eq!(reconcile_participant(&input).admission.role, None);
    input.peers.push(peer("node-c", 3, 4, &held));
    assert_eq!(reconcile_participant(&input).admission.role, Some(RejoinRole::Voter));
}

#[test]
fn a_lost_leader_commit_record_cannot_hide_an_entry_held_only_above_peer_commitment() {
    let prefix = [(1, "a"), (2, "b"), (3, "c"), (4, "d")];
    let tail = [(1, "a"), (2, "b"), (3, "c"), (4, "d"), (5, "acknowledged")];
    let input = base(&prefix, 4, vec![peer("node-b", 3, 4, &prefix), peer("node-c", 3, 4, &tail)]);
    let outcome = reconcile_participant(&input);
    assert_eq!(outcome.decisions.last(), Some(&(5, EntryRepairDecision::Wait)));
    assert_eq!(outcome.admission.role, None);
    assert!(outcome.effects.iter().all(|effect| matches!(effect, RecoveryEffect::QueryPeers { .. })));
}

#[test]
fn unreadable_local_slots_never_count_as_absence_when_any_peer_holds_an_entry() {
    for local in [LocalEntry::Missing, LocalEntry::Faulty] {
        let mut input = base(&[], 0, vec![
            peer("node-b", 3, 0, &[(1, "held")]),
            peer("node-c", 3, 0, &[]),
            peer("node-d", 3, 0, &[]),
            peer("node-e", 3, 0, &[]),
        ]);
        input.voters.extend(["node-d".to_string(), "node-e".to_string()]);
        input.local_entries.insert(1, local);
        let outcome = reconcile_participant(&input);
        assert_eq!(outcome.decisions, vec![(1, EntryRepairDecision::Wait)]);
        assert_eq!(outcome.admission.role, None);
        assert!(outcome.effects.iter().all(|effect| matches!(effect, RecoveryEffect::QueryPeers { .. })));
    }
}

#[test]
fn unreadable_slots_require_a_quorum_of_peer_only_absence_evidence() {
    let mut input = base(&[], 0, vec![peer("node-b", 3, 0, &[])]);
    input.local_entries.insert(1, LocalEntry::Missing);
    assert_eq!(reconcile_participant(&input).decisions, vec![(1, EntryRepairDecision::Wait)]);
    input.peers.push(peer("node-c", 3, 0, &[]));
    assert_eq!(reconcile_participant(&input).admission.role, Some(RejoinRole::Voter));
    input.local_entries.insert(1, LocalEntry::Faulty);
    assert_eq!(reconcile_participant(&input).effects, vec![RecoveryEffect::RemoveSuffixFrom { index: 1 }]);
}

fn bounded_snapshot_input(local_commit: u64, boundary: u64) -> ReconcileInput {
    let mut input = base(&[], local_commit, Vec::new());
    for index in 1..=local_commit {
        input.local_entries.insert(index, LocalEntry::Present(entry(3, "held")));
    }
    let mut report = peer("node-b", 3, 4_200, &[]);
    report.snapshot = Some(snapshot(boundary, "snapshot"));
    for index in boundary + 1..=4_200 {
        report.entries.insert(index, entry(3, "held"));
    }
    input.peers = both(report);
    input
}

#[test]
fn snapshot_replaced_prefixes_do_not_exhaust_the_4096_index_repair_bound() {
    let input = bounded_snapshot_input(3_000, 3_100);
    assert_eq!(reconcile_participant(&input).effects, vec![RecoveryEffect::InstallSnapshot {
        from_peer: "node-b".to_string(),
        snapshot: snapshot(3_100, "snapshot"),
    }]);
    let input = bounded_snapshot_input(4_000, 3_900);
    let outcome = reconcile_participant(&input);
    assert_eq!(outcome.snapshot, SnapshotRepairDecision::KeepLocal);
    assert_eq!(
        outcome.effects,
        (4_001..=4_200)
            .map(|index| RecoveryEffect::FetchEntry {
                from_peer: "node-b".to_string(),
                index,
                entry: entry(3, "held"),
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(outcome.admission.role, None);
}

#[test]
fn a_superseded_committed_prefix_still_vetoes_conflicting_history() {
    let mut input = bounded_snapshot_input(3_000, 3_100);
    let mut older = peer("node-c", 3, 3_000, &[]);
    for index in 1..=3_000 {
        older.entries.insert(index, entry(3, "held"));
    }
    older.entries.insert(1, entry(3, "conflict"));
    input.peers[1] = older;
    let outcome = reconcile_participant(&input);
    assert_eq!(outcome.decisions, vec![(1, EntryRepairDecision::ConflictingHistory)]);
    assert!(outcome.effects.is_empty());
    assert_eq!(outcome.admission.role, None);
}

#[test]
fn a_waiting_older_snapshot_does_not_suppress_the_fetch_that_can_validate_its_boundary() {
    let mut input = base(&[], 12, Vec::new());
    let mut compacted = peer("node-b", 3, 12, &[]);
    compacted.snapshot = Some(snapshot(5, "snapshot"));
    let mut complete = peer("node-c", 3, 12, &[]);
    for index in 1..=12 {
        input.local_entries.insert(index, LocalEntry::Present(entry(3, "held")));
        complete.entries.insert(index, entry(3, "held"));
        if index > 5 {
            compacted.entries.insert(index, entry(3, "held"));
        }
    }
    input.local_entries.insert(5, LocalEntry::Faulty);
    input.peers = vec![compacted, complete];
    let outcome = reconcile_participant(&input);
    assert_eq!(outcome.snapshot, SnapshotRepairDecision::Wait);
    assert_eq!(outcome.effects, vec![RecoveryEffect::FetchEntry {
        from_peer: "node-c".to_string(),
        index: 5,
        entry: entry(3, "held"),
    }]);
    input.local_entries.insert(5, LocalEntry::Present(entry(3, "held")));
    assert_eq!(reconcile_participant(&input).admission.role, Some(RejoinRole::Voter));
}

// r[verify molten.consensus.repair_admission]
#[test]
fn lost_votes_wait_for_every_peer_term_even_when_log_reporters_intersect_commit_quorums() {
    let mut input = base(&[], 0, vec![
        peer("node-b", 3, 0, &[]),
        peer("node-c", 3, 0, &[]),
        peer("node-d", 3, 0, &[]),
    ]);
    input.voters.extend(["node-d".to_string(), "node-e".to_string()]);
    input.local_hard_state = LocalHardState::Lost;
    input.current_term = 100;
    let unknown = reconcile_participant(&input);
    assert_eq!(unknown.hard_state, HardStateDecision::Wait);
    assert_eq!(unknown.admission.role, None);
    input.peers.push(peer("node-e", 101, 0, &[]));
    let fenced = reconcile_participant(&input);
    assert_eq!(fenced.hard_state, HardStateDecision::VoteFenced { through_term: 101 });
    assert_eq!(fenced.admission.role, Some(RejoinRole::NonVoting));
    input.current_term = 102;
    assert_eq!(reconcile_participant(&input).admission.role, Some(RejoinRole::Voter));
}
