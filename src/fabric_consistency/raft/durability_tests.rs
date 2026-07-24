use super::tests::NODE_A;
use super::tests::active_group;
use super::tests::started_state;
use super::tests::test_ref;
use super::*;
use crate::fabric_durability::DurableAdapterKind;
use crate::fabric_durability::RedbDurableStateAdapter;
use crate::fabric_durability::tests::descriptor;
use crate::fabric_durability::tests::profile;

const TERM: u64 = 1;
const ENTRY_INDEX: u64 = 1;
const SECOND_ENTRY_INDEX: u64 = 2;
const EXPECTED_DURABLE_RECORDS: usize = 3;
const EXPECTED_RECOVERY_EFFECTS: usize = 3;
const EXPECTED_SNAPSHOT_ONLY_RECOVERY_EFFECTS: usize = 2;

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn redb_replica_port_makes_hard_state_and_flushed_entries_durable() {
    let root = crate::test_support::process_workspace("live-raft-redb-port").expect("workspace");
    let adapter = RedbDurableStateAdapter::open(&root, profile(DurableAdapterKind::LiveRedb), descriptor())
        .expect("Redb adapter");
    let mut port =
        RedbReplicaDurabilityPort::new(adapter, test_ref("Redb-durable-log"), test_ref("Redb-snapshot-store"))
            .expect("Redb replica durability port");

    let hard_state = port.persist_hard_state(TERM, Some("node-a")).expect("hard state");
    assert!(hard_state.starts_with("blake3:"));
    assert!(port.adapter().state().buffered_log.is_empty());
    assert_eq!(port.adapter().state().durable_log.len(), 1);

    let entry = entry(ENTRY_INDEX, "durable-entry");
    port.persist_entries(None, std::slice::from_ref(&entry)).expect("buffered entry");
    assert_eq!(port.adapter().state().buffered_log.len(), 1);
    let flush = port.flush_log(ENTRY_INDEX).expect("machine-loss flush");
    assert!(flush.starts_with("blake3:"));
    assert!(port.adapter().state().buffered_log.is_empty());
    assert_eq!(port.adapter().state().durable_log.len(), EXPECTED_DURABLE_RECORDS);

    let mut snapshot = ReplicaSnapshot {
        snapshot_ref: String::new(),
        group_binding_ref: test_ref("durable-group"),
        membership_ref: test_ref("durable-membership"),
        config_epoch: TERM,
        fencing_epoch: TERM,
        last_included_index: ENTRY_INDEX,
        last_included_term: TERM,
        application_state_ref: test_ref("durable-application-state"),
        completed_requests: Default::default(),
    };
    snapshot.snapshot_ref = snapshot_ref(&snapshot).expect("snapshot identity");
    let snapshot_evidence = port.persist_snapshot(&snapshot).expect("durable snapshot");
    assert!(snapshot_evidence.starts_with("blake3:"));
    assert!(port.adapter().state().snapshots.contains_key(&snapshot.snapshot_ref));
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn durable_installed_snapshot_establishes_its_recovery_commit_boundary() {
    let root = crate::test_support::process_workspace("live-raft-installed-snapshot-recovery").expect("workspace");
    let group = active_group();
    let initial = started_state(&group, NODE_A);
    let durable_log_ref = test_ref("installed-snapshot-durable-log");
    let snapshot_store_ref = test_ref("installed-snapshot-store");
    let adapter = RedbDurableStateAdapter::open(&root, profile(DurableAdapterKind::LiveRedb), descriptor())
        .expect("Redb adapter");
    let mut port = RedbReplicaDurabilityPort::new(adapter, durable_log_ref.clone(), snapshot_store_ref.clone())
        .expect("Redb replica durability port");
    port.persist_hard_state(TERM, None).expect("snapshot hard state");
    let mut snapshot = ReplicaSnapshot {
        snapshot_ref: String::new(),
        group_binding_ref: initial.profile.group_binding_ref.clone(),
        membership_ref: initial.membership.membership_ref.clone(),
        config_epoch: initial.membership.config_epoch,
        fencing_epoch: initial.profile.fencing_epoch,
        last_included_index: ENTRY_INDEX,
        last_included_term: TERM,
        application_state_ref: test_ref("installed-snapshot-application-state"),
        completed_requests: Default::default(),
    };
    snapshot.snapshot_ref = snapshot_ref(&snapshot).expect("installed snapshot identity");
    port.persist_snapshot(&snapshot).expect("machine-loss snapshot");
    drop(port);

    let reopened = RedbDurableStateAdapter::open(&root, profile(DurableAdapterKind::LiveRedb), descriptor())
        .expect("reopened Redb adapter");
    let reopened = RedbReplicaDurabilityPort::new(reopened, durable_log_ref, snapshot_store_ref)
        .expect("reopened replica durability port");
    let recovery = reopened.plan_recovery(recovery_start_plan(&group, initial)).expect("snapshot-only recovery");
    assert_eq!(recovery.durable_commit_index, ENTRY_INDEX);
    assert_eq!(recovery.start_plan.state.commit_index, ENTRY_INDEX);
    assert_eq!(recovery.start_plan.state.last_applied, ENTRY_INDEX);
    assert_eq!(recovery.start_plan.state.snapshot.as_ref(), Some(&snapshot));
    assert_eq!(recovery.start_plan.initial_effects.len(), EXPECTED_SNAPSHOT_ONLY_RECOVERY_EFFECTS);
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn redb_recovery_restores_snapshot_and_replays_only_committed_suffix() {
    let root = crate::test_support::process_workspace("live-raft-redb-recovery").expect("workspace");
    let group = active_group();
    let initial = started_state(&group, NODE_A);
    let durable_log_ref = test_ref("recovery-durable-log");
    let snapshot_store_ref = test_ref("recovery-snapshot-store");
    let adapter = RedbDurableStateAdapter::open(&root, profile(DurableAdapterKind::LiveRedb), descriptor())
        .expect("Redb adapter");
    let mut port = RedbReplicaDurabilityPort::new(adapter, durable_log_ref.clone(), snapshot_store_ref.clone())
        .expect("Redb replica durability port");
    port.persist_hard_state(TERM, Some(NODE_A)).expect("hard state");
    let entries = vec![
        entry(ENTRY_INDEX, "recovery-first"),
        entry(SECOND_ENTRY_INDEX, "recovery-second"),
    ];
    port.persist_entries(None, &entries).expect("recovery entries");
    port.flush_log(SECOND_ENTRY_INDEX).expect("recovery flush");
    port.persist_commit(SECOND_ENTRY_INDEX).expect("recovery commit");
    let mut snapshot = ReplicaSnapshot {
        snapshot_ref: String::new(),
        group_binding_ref: initial.profile.group_binding_ref.clone(),
        membership_ref: initial.membership.membership_ref.clone(),
        config_epoch: initial.membership.config_epoch,
        fencing_epoch: initial.profile.fencing_epoch,
        last_included_index: ENTRY_INDEX,
        last_included_term: TERM,
        application_state_ref: test_ref("recovery-application-state"),
        completed_requests: std::collections::BTreeMap::from([(entries[0].request_ref.clone(), ENTRY_INDEX)]),
    };
    snapshot.snapshot_ref = snapshot_ref(&snapshot).expect("recovery snapshot identity");
    port.persist_snapshot(&snapshot).expect("recovery snapshot");
    drop(port);

    let reopened = RedbDurableStateAdapter::open(&root, profile(DurableAdapterKind::LiveRedb), descriptor())
        .expect("reopened Redb adapter");
    let reopened = RedbReplicaDurabilityPort::new(reopened, durable_log_ref, snapshot_store_ref)
        .expect("reopened replica durability port");
    let recovery = reopened.plan_recovery(recovery_start_plan(&group, initial)).expect("recovery plan");

    assert!(recovery.recovery_ref.starts_with("blake3:"));
    assert_eq!(recovery.durable_commit_index, SECOND_ENTRY_INDEX);
    assert_eq!(recovery.replay_entry_count, 1);
    assert_eq!(recovery.start_plan.state.current_term, TERM);
    assert_eq!(recovery.start_plan.state.voted_for.as_deref(), Some(NODE_A));
    assert_eq!(recovery.start_plan.state.commit_index, SECOND_ENTRY_INDEX);
    assert_eq!(recovery.start_plan.state.last_applied, SECOND_ENTRY_INDEX);
    assert_eq!(recovery.start_plan.state.snapshot.as_ref(), Some(&snapshot));
    assert_eq!(recovery.start_plan.state.log, vec![entries[1].clone()]);
    assert_eq!(recovery.start_plan.initial_effects.len(), EXPECTED_RECOVERY_EFFECTS);
    assert!(matches!(recovery.start_plan.initial_effects.as_slice(), [
        ReplicaEffect::RestoreApplicationSnapshot { .. },
        ReplicaEffect::ApplyCommitted { .. },
        ReplicaEffect::ArmElectionTimer { .. }
    ]));
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn redb_recovery_denies_commit_boundary_beyond_durable_log() {
    let root = crate::test_support::process_workspace("live-raft-redb-invalid-recovery").expect("workspace");
    let group = active_group();
    let initial = started_state(&group, NODE_A);
    let adapter = RedbDurableStateAdapter::open(&root, profile(DurableAdapterKind::LiveRedb), descriptor())
        .expect("Redb adapter");
    let mut port = RedbReplicaDurabilityPort::new(
        adapter,
        test_ref("invalid-recovery-durable-log"),
        test_ref("invalid-recovery-snapshot-store"),
    )
    .expect("Redb replica durability port");
    port.persist_hard_state(TERM, Some(NODE_A)).expect("hard state");
    port.persist_entries(None, &[entry(ENTRY_INDEX, "invalid-recovery")])
        .expect("single recovery entry");
    port.flush_log(ENTRY_INDEX).expect("single recovery flush");
    port.persist_commit(SECOND_ENTRY_INDEX).expect("invalid commit marker persisted for recovery test");

    let error = port
        .plan_recovery(recovery_start_plan(&group, initial))
        .expect_err("out-of-range commit must deny recovery");
    assert!(error.to_string().contains("commit boundary is stale or beyond the log"));
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn redb_replica_port_rejects_over_bound_entry_batch_without_mutation() {
    let root = crate::test_support::process_workspace("live-raft-redb-over-bound").expect("workspace");
    let adapter = RedbDurableStateAdapter::open(&root, profile(DurableAdapterKind::LiveRedb), descriptor())
        .expect("Redb adapter");
    let mut port = RedbReplicaDurabilityPort::new(
        adapter,
        test_ref("Redb-negative-durable-log"),
        test_ref("Redb-negative-snapshot-store"),
    )
    .expect("Redb replica durability port");
    let entries = (0..MAX_REPLICA_MESSAGE_ENTRIES)
        .map(|offset| {
            let index = u64::try_from(offset + 1).expect("bounded entry index");
            entry(index, &format!("over-bound-entry-{offset}"))
        })
        .collect::<Vec<_>>();

    let error = port.persist_entries(None, &entries).expect_err("over-bound durability operation must deny");
    assert!(error.to_string().contains("ByteLimitExceeded"), "unexpected error: {error}");
    assert!(port.adapter().state().buffered_log.is_empty());
    assert!(port.adapter().state().durable_log.is_empty());
}

fn recovery_start_plan(
    group: &crate::fabric_consistency::ConsistencyGroupBinding,
    state: ReplicaState,
) -> ReplicaStartPlan {
    ReplicaStartPlan {
        state,
        service_id: group.service_id.clone(),
        application_manifest_ref: group.application_manifest_ref.clone(),
        initial_effects: Vec::new(),
        port_binding_refs: Vec::new(),
        production_admitted: false,
    }
}

fn entry(index: u64, label: &str) -> ReplicatedEntry {
    ReplicatedEntry {
        index,
        term: TERM,
        request_ref: test_ref(&format!("{label}-request")),
        command_ref: test_ref(&format!("{label}-command")),
        command_schema_ref: test_ref("durable-command-schema"),
    }
}
