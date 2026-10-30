// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.storage_fault_validation]
#[test]
fn repeated_snapshot_recovery_replays_post_install_entries_without_old_snapshot_metadata() {
    let root = crate::test_support::process_workspace("raft-repeated-snapshot-replay").expect("workspace");
    let group = super::tests::active_group();
    let initial = super::tests::started_state(&group, super::tests::NODE_A);
    let log_ref = super::tests::test_ref("snapshot-replay-log");
    let store_ref = super::tests::test_ref("snapshot-replay-store");
    let adapter = crate::fabric_durability::RedbDurableStateAdapter::open(
        &root,
        crate::fabric_durability::tests::profile(crate::fabric_durability::DurableAdapterKind::LiveRedb),
        crate::fabric_durability::tests::descriptor(),
    ).expect("adapter");
    let mut port = RedbReplicaPort::new(adapter, log_ref.clone(), store_ref.clone()).expect("durability port");
    port.persist_hard_state(TERM, None, true).expect("hard state with recovery marker");
    let first = entry(1, "installed-snapshot-first");
    let mut snapshot = ReplicaSnapshot {
        snapshot_ref: String::new(), group_binding_ref: group.binding_ref.clone(),
        membership_ref: initial.membership.membership_ref.clone(), config_epoch: initial.membership.config_epoch,
        fencing_epoch: initial.profile.fencing_epoch, last_included_index: 1, last_included_term: TERM,
        application_state_ref: super::tests::test_ref("installed-snapshot-application"),
        completed_requests: std::collections::BTreeMap::from([(first.request_ref.clone(), 1)]),
    };
    snapshot.snapshot_ref = snapshot_ref(&snapshot).expect("first snapshot identity");
    port.persist_snapshot(&snapshot, true).expect("installed peer snapshot");
    let second = entry(2, "installed-snapshot-second");
    snapshot.last_included_index = 2;
    snapshot.completed_requests.insert(second.request_ref.clone(), 2);
    snapshot.application_state_ref = super::tests::test_ref("second-checkpoint-application");
    snapshot.snapshot_ref = snapshot_ref(&snapshot).expect("second snapshot identity");
    port.persist_snapshot(&snapshot, true).expect("newer peer checkpoint at identical durable coverage");
    let third = entry(3, "installed-snapshot-third");
    port.persist_entries(None, std::slice::from_ref(&third)).expect("entry after checkpoint");
    port.flush_log(3).expect("third durable");
    port.persist_commit(3).expect("third committed");
    drop(port);
    let adapter = crate::fabric_durability::RedbDurableStateAdapter::open(
        &root,
        crate::fabric_durability::tests::profile(crate::fabric_durability::DurableAdapterKind::LiveRedb),
        crate::fabric_durability::tests::descriptor(),
    ).expect("reopened adapter");
    let port = RedbReplicaPort::new(adapter, log_ref, store_ref).expect("reopened durability port");
    let recovery = port.plan_recovery(recovery_start_plan(&group, initial)).expect("latest snapshot is self-contained");
    assert_eq!(recovery.start_plan.state.snapshot, Some(snapshot));
    assert_eq!(recovery.start_plan.state.log, vec![third.clone()]);
    assert_eq!(recovery.start_plan.state.commit_index, 3);
    assert!(matches!(recovery.start_plan.state.rejoin, ReplicaRejoin::Recovering(_)),
        "snapshot-covered hard state must retain the recovery requirement");
    assert_eq!(recovery.start_plan.state.completed_requests.get(&first.request_ref), Some(&1));
    assert_eq!(recovery.start_plan.state.completed_requests.get(&second.request_ref), Some(&2));
    assert_eq!(recovery.start_plan.state.completed_requests.get(&third.request_ref), Some(&3));
}
