use super::tests::test_ref;
use super::*;
use crate::fabric_durability::DurableAdapterKind;
use crate::fabric_durability::RedbDurableStateAdapter;
use crate::fabric_durability::tests::descriptor;
use crate::fabric_durability::tests::profile;

const TERM: u64 = 1;
const ENTRY_INDEX: u64 = 1;
const EXPECTED_DURABLE_RECORDS: usize = 3;

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

    let snapshot = ReplicaSnapshot {
        snapshot_ref: test_ref("durable-snapshot"),
        group_binding_ref: test_ref("durable-group"),
        membership_ref: test_ref("durable-membership"),
        config_epoch: TERM,
        fencing_epoch: TERM,
        last_included_index: ENTRY_INDEX,
        last_included_term: TERM,
        application_state_ref: test_ref("durable-application-state"),
    };
    let snapshot_evidence = port.persist_snapshot(&snapshot).expect("durable snapshot");
    assert!(snapshot_evidence.starts_with("blake3:"));
    assert!(port.adapter().state().snapshots.contains_key(&snapshot.snapshot_ref));
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

fn entry(index: u64, label: &str) -> ReplicatedEntry {
    ReplicatedEntry {
        index,
        term: TERM,
        request_ref: test_ref(&format!("{label}-request")),
        command_ref: test_ref(&format!("{label}-command")),
        command_schema_ref: test_ref("durable-command-schema"),
    }
}
