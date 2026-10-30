// r[verify molten.consensus.storage_fault_recovery]
#[test]
fn redb_restart_retains_explicit_suffix_across_mismatching_snapshot_boundary() {
    assert_snapshot_suffix_restart("raft-retained-snapshot-suffix", true, false);
}

// r[verify molten.consensus.storage_fault_recovery]
#[test]
fn redb_restart_discards_explicit_suffix_across_mismatching_snapshot_boundary() {
    assert_snapshot_suffix_restart("raft-discarded-snapshot-suffix", false, false);
}

// r[verify molten.consensus.storage_fault_recovery]
#[test]
fn redb_restart_retains_committed_suffix_across_mismatching_snapshot_boundary() {
    assert_snapshot_suffix_restart("raft-committed-snapshot-suffix", true, true);
}

fn assert_snapshot_suffix_restart(label: &str, is_suffix_retained: bool, is_suffix_committed: bool) {
    let root = crate::test_support::process_workspace(label).expect("isolated suffix workspace");
    let group = super::tests::active_group();
    let initial = super::tests::started_state(&group, super::tests::NODE_A);
    let log_ref = super::tests::test_ref(&format!("{label}-log"));
    let store_ref = super::tests::test_ref(&format!("{label}-store"));
    let mut port = open_snapshot_suffix_port(&root, &log_ref, &store_ref);
    port.persist_hard_state(2, None, false).expect("hard state without recovery marker");
    let boundary = entry(ENTRY_INDEX, &format!("{label}-boundary"));
    let mut suffix = entry(SECOND_ENTRY_INDEX, &format!("{label}-suffix"));
    suffix.term = 2;
    port.persist_entries(None, &[boundary, suffix.clone()]).expect("local boundary and valid suffix");
    port.flush_log(SECOND_ENTRY_INDEX).expect("durable local history");
    if is_suffix_committed {
        port.persist_commit(SECOND_ENTRY_INDEX).expect("durable suffix commit");
    }
    let before = port.plan_recovery(recovery_start_plan(&group, initial.clone())).expect("local recovery");
    assert_eq!(before.durable_commit_index, if is_suffix_committed { SECOND_ENTRY_INDEX } else { 0 });
    assert_eq!(before.start_plan.state.log.last(), Some(&suffix));
    let mut snapshot = ReplicaSnapshot {
        snapshot_ref: String::new(),
        group_binding_ref: initial.profile.group_binding_ref.clone(),
        membership_ref: initial.membership.membership_ref.clone(),
        config_epoch: initial.membership.config_epoch,
        fencing_epoch: initial.profile.fencing_epoch,
        last_included_index: ENTRY_INDEX,
        last_included_term: 2,
        application_state_ref: super::tests::test_ref(&format!("{label}-application")),
        completed_requests: Default::default(),
    };
    snapshot.snapshot_ref = snapshot_ref(&snapshot).expect("peer snapshot identity");
    port.persist_snapshot(&snapshot, is_suffix_retained).expect("explicit snapshot suffix disposition");
    drop(port);

    let reopened = open_snapshot_suffix_port(&root, &log_ref, &store_ref);
    let recovered = reopened.plan_recovery(recovery_start_plan(&group, initial)).expect("clean snapshot restart");
    assert_eq!(recovered.start_plan.state.snapshot, Some(snapshot));
    assert_eq!(recovered.start_plan.state.rejoin, ReplicaRejoin::Voter);
    let commit_index = if is_suffix_committed { SECOND_ENTRY_INDEX } else { ENTRY_INDEX };
    assert_eq!(recovered.durable_commit_index, commit_index);
    assert_eq!(recovered.start_plan.state.commit_index, commit_index);
    assert_eq!(recovered.start_plan.state.last_applied, commit_index);
    assert_eq!(recovered.replay_entry_count, usize::from(is_suffix_committed));
    if is_suffix_retained {
        assert_eq!(recovered.start_plan.state.log, vec![suffix.clone()]);
    } else {
        assert!(recovered.start_plan.state.log.is_empty());
    }
    if is_suffix_committed {
        assert_eq!(recovered.start_plan.state.completed_requests.get(&suffix.request_ref), Some(&SECOND_ENTRY_INDEX));
        assert!(recovered.start_plan.initial_effects.iter().any(|effect| matches!(effect,
            ReplicaEffect::ApplyCommitted { entries } if entries.as_slice() == std::slice::from_ref(&suffix))));
    } else {
        assert!(!recovered.start_plan.state.completed_requests.contains_key(&suffix.request_ref));
    }
}

fn open_snapshot_suffix_port(root: &std::path::Path, log_ref: &str, store_ref: &str) -> RedbReplicaPort {
    let adapter = crate::fabric_durability::RedbDurableStateAdapter::open(
        root,
        crate::fabric_durability::tests::profile(crate::fabric_durability::DurableAdapterKind::LiveRedb),
        crate::fabric_durability::tests::descriptor(),
    ).expect("real Redb suffix adapter");
    RedbReplicaPort::new(adapter, log_ref.to_owned(), store_ref.to_owned()).expect("suffix durability port")
}
