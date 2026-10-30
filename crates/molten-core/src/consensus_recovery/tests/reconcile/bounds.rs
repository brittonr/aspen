use super::*;

fn index_bound() -> u64 {
    u64::try_from(MAX_RECONCILED_INDICES).expect("reconciliation bound fits u64")
}

#[test]
fn reconciliation_accepts_the_exact_index_bound_and_rejects_the_next_index() {
    let mut input = base(&[], index_bound(), vec![peer("node-b", 3, index_bound(), &[])]);
    for index in 1..=index_bound() {
        input.local_entries.insert(index, LocalEntry::Present(entry(3, "a")));
        input.peers[0].entries.insert(index, entry(3, "a"));
    }
    input.peers = both(input.peers.remove(0));
    assert_eq!(reconcile_participant(&input).admission.role, Some(RejoinRole::Voter));
    input.local_commit_index += 1;
    input.peers[0].commit_index += 1;
    input.local_entries.insert(input.local_commit_index, LocalEntry::Present(entry(3, "a")));
    input.peers[0].entries.insert(input.local_commit_index, entry(3, "a"));
    let outcome = reconcile_participant(&input);
    assert!(outcome.effects.is_empty());
    assert_eq!(outcome.admission.role, None);
}

#[test]
fn index_bounds_apply_to_the_union_of_held_entries_and_committed_coverage() {
    let mut input = base(&[], index_bound(), vec![peer("node-b", 3, index_bound(), &[])]);
    for index in 1..=index_bound() {
        input.peers[0].entries.insert(index, entry(3, "a"));
    }
    input.local_entries.insert(index_bound() + 1, LocalEntry::Present(entry(3, "suffix")));
    let outcome = reconcile_participant(&input);
    assert!(outcome.effects.is_empty());
    assert_eq!(outcome.admission.role, None);
}

#[test]
fn huge_uncovered_indices_are_rejected_but_a_snapshot_can_cover_them() {
    let mut input = base(&[], u64::MAX, vec![peer("node-b", 3, u64::MAX, &[])]);
    assert_eq!(reconcile_participant(&input).admission.role, None);
    input.local_snapshot = Some(snapshot(u64::MAX, "snapshot"));
    input.peers[0].snapshot = Some(snapshot(u64::MAX, "snapshot"));
    input.peers = both(input.peers.remove(0));
    assert_eq!(reconcile_participant(&input).admission.role, Some(RejoinRole::Voter));
    input.local_commit_index = 0;
    input.local_snapshot = None;
    assert_eq!(reconcile_participant(&input).effects, vec![RecoveryEffect::InstallSnapshot {
        from_peer: "node-b".to_string(),
        snapshot: snapshot(u64::MAX, "snapshot"),
    }]);
}

#[test]
fn index_zero_and_local_snapshots_beyond_commit_are_not_admitted() {
    let mut zero = base(&[(0, "invalid")], 0, vec![peer("node-b", 3, 0, &[])]);
    assert_eq!(reconcile_participant(&zero).admission.role, None);
    zero.local_entries.clear();
    zero.local_snapshot = Some(snapshot(1, "snapshot"));
    assert_eq!(reconcile_participant(&zero).admission.role, None);
}

#[test]
fn the_peer_report_bound_accepts_the_limit_and_rejects_one_more_report() {
    let mut input = base(&[], 0, Vec::new());
    for index in 0..=MAX_RECOVERY_PEER_REPORTS {
        let id = format!("peer-{index}");
        input.voters.insert(id.clone());
        input.peers.push(peer(&id, 3, 0, &[]));
        if input.peers.len() == MAX_RECOVERY_PEER_REPORTS {
            assert_eq!(reconcile_participant(&input).admission.role, Some(RejoinRole::Voter));
        }
    }
    let outcome = reconcile_participant(&input);
    assert!(outcome.effects.is_empty());
    assert_eq!(outcome.admission.role, None);
}

#[test]
fn an_empty_committed_history_can_reconcile_but_a_nonmember_participant_cannot() {
    let mut input = base(&[], 0, both(peer("node-b", 3, 0, &[])));
    assert_eq!(reconcile_participant(&input).admission.role, Some(RejoinRole::Voter));
    input.voters.remove(SELF);
    assert_eq!(reconcile_participant(&input).admission.role, None);
}

#[test]
fn an_oversized_uncommitted_peer_log_cannot_bypass_committed_coverage_bounds() {
    let mut input = base(&[], 0, vec![peer("node-b", 3, 0, &[])]);
    for index in 1..=index_bound() + 1 {
        input.peers[0].entries.insert(index, entry(3, "held"));
    }
    let outcome = reconcile_participant(&input);
    assert!(outcome.effects.is_empty());
    assert_eq!(outcome.admission.role, None);
}

#[test]
fn uncommitted_tail_holes_cannot_attest_absence_and_remove_another_local_suffix() {
    for boundary in [0, 5] {
        let mut gapped = peer("node-b", 3, boundary, &[(boundary + 2, "after-hole")]);
        let mut lacking = peer("node-c", 3, boundary, &[]);
        let mut input = base(&[(boundary + 1, "held")], boundary, Vec::new());
        if boundary != 0 {
            gapped.snapshot = Some(snapshot(boundary, "snapshot"));
            lacking.snapshot = Some(snapshot(boundary, "snapshot"));
            input.local_snapshot = Some(snapshot(boundary, "snapshot"));
        }
        input.peers = vec![gapped, lacking];
        let outcome = reconcile_participant(&input);
        assert!(outcome.effects.is_empty());
        assert_eq!(outcome.admission.role, None);
    }
}

#[test]
fn contiguous_uncommitted_tails_can_reconcile_after_an_empty_or_compacted_prefix() {
    for boundary in [0, 5] {
        let held = [(boundary + 1, "first"), (boundary + 2, "second")];
        let mut report = peer("node-b", 3, boundary, &held);
        let mut input = base(&held, boundary, Vec::new());
        if boundary != 0 {
            report.snapshot = Some(snapshot(boundary, "snapshot"));
            input.local_snapshot = Some(snapshot(boundary, "snapshot"));
        }
        input.peers = both(report);
        let outcome = reconcile_participant(&input);
        assert_eq!(outcome.decisions, vec![
            (boundary + 1, EntryRepairDecision::KeepLocal),
            (boundary + 2, EntryRepairDecision::KeepLocal),
        ]);
        assert!(outcome.effects.is_empty());
        assert_eq!(outcome.admission.role, Some(RejoinRole::Voter));
    }
}
