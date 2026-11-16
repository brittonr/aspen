// Framed compaction retains a committed suffix larger than one durable operation.
const LARGE_SUFFIX_ENTRIES: usize = 512;
const SERVICE_OPERATION_BYTES: u64 = 65_536;

fn open_large_suffix_port(root: &std::path::Path) -> RedbReplicaPort {
    let mut declaration =
        crate::fabric_durability::tests::profile(crate::fabric_durability::DurableAdapterKind::LiveRedb).profile;
    // Match the reliability service's actual durable operation, record and namespace bounds.
    // The tiny shared adapter-test profile otherwise permits only 32 records of 1 KiB each.
    declaration.max_operation_bytes = SERVICE_OPERATION_BYTES;
    declaration.max_log_records = 65_536;
    declaration.max_namespace_bytes = 67_108_864;
    let profile = crate::fabric_durability::durable_profile(&declaration).expect("service-sized profile");
    let mut namespace = crate::fabric_durability::tests::descriptor();
    namespace.quota_bytes = declaration.max_namespace_bytes;
    namespace.atomicity_domain.max_bytes = declaration.max_operation_bytes;
    let adapter = crate::fabric_durability::RedbDurableStateAdapter::open(root, profile, namespace)
        .expect("bounded suffix adapter");
    let authority = adapter.state().descriptor.retention_authority_ref.clone().expect("retention authority");
    RedbReplicaPort::new(
        adapter,
        super::tests::test_ref("large-suffix-log"),
        super::tests::test_ref("large-suffix-store"),
    )
    .expect("large suffix port")
    .with_retention_authority(authority)
    .expect("authorized compaction")
}

fn large_suffix_fixture(
    root: &std::path::Path,
) -> (RedbReplicaPort, ReplicaState, ReplicaSnapshot, Vec<ReplicatedEntry>) {
    let group = super::tests::active_group();
    let mut initial = super::tests::started_state(&group, super::tests::NODE_A);
    initial.profile.max_log_entries = 4_096;
    initial.profile.snapshot_threshold_entries = 2_048;
    let mut port = open_large_suffix_port(root);
    port.persist_hard_state(3, Some(super::tests::NODE_B), false).expect("term and vote");
    let boundary = entry(1, "large-suffix-boundary");
    port.persist_entries(None, std::slice::from_ref(&boundary)).expect("boundary entry");
    let suffix = (0..LARGE_SUFFIX_ENTRIES)
        .map(|offset| {
            let mut entry = entry(offset as u64 + 2, &format!("large-suffix-{offset}"));
            entry.term = 2;
            entry
        })
        .collect::<Vec<_>>();
    for entry in &suffix {
        port.persist_entries(None, std::slice::from_ref(entry)).expect("individually bounded append");
    }
    let last_index = suffix.last().expect("nonempty suffix").index;
    port.flush_log(last_index).expect("durable suffix");
    port.persist_commit(last_index).expect("committed suffix above installed snapshot");
    let mut snapshot = ReplicaSnapshot {
        snapshot_ref: String::new(),
        group_binding_ref: initial.profile.group_binding_ref.clone(),
        membership_ref: initial.membership.membership_ref.clone(),
        config_epoch: initial.membership.config_epoch,
        fencing_epoch: initial.profile.fencing_epoch,
        last_included_index: boundary.index,
        last_included_term: boundary.term,
        application_state_ref: super::tests::test_ref("large-suffix-application"),
        completed_requests: std::collections::BTreeMap::from([(boundary.request_ref, boundary.index)]),
    };
    snapshot.snapshot_ref = snapshot_ref(&snapshot).expect("snapshot identity");
    (port, initial, snapshot, suffix)
}

fn assert_large_suffix_recovery(recovered: &ReplicaState, snapshot: &ReplicaSnapshot, suffix: &[ReplicatedEntry]) {
    assert_eq!(recovered.snapshot.as_ref(), Some(snapshot));
    assert_eq!(recovered.current_term, 3);
    assert_eq!(recovered.voted_for.as_deref(), Some(super::tests::NODE_B));
    assert_eq!(recovered.log, suffix);
    assert_eq!(recovered.commit_index, suffix.last().expect("suffix").index);
    assert_eq!(recovered.last_applied, recovered.commit_index);
    let completed = snapshot
        .completed_requests
        .iter()
        .map(|(request, index)| (request, *index))
        .chain(suffix.iter().map(|entry| (&entry.request_ref, entry.index)))
        .filter(|(_, index)| {
            *index > recovered.last_applied.saturating_sub(recovered.profile.request_retention_entries)
                && *index <= recovered.last_applied
        })
        .map(|(request, index)| (request.clone(), index))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(recovered.completed_requests, completed);
}

// r[verify molten.consensus.durable_log_compaction]
#[test]
fn compaction_reopens_512_distinct_committed_suffix_entries_with_the_service_operation_bound() {
    let root = crate::test_support::process_workspace("raft-compaction-large-suffix").expect("workspace");
    let (mut port, initial, snapshot, suffix) = large_suffix_fixture(&root);
    let retained = ReplicaRetainedState {
        term: 3,
        voted_for: Some(super::tests::NODE_B),
        entries: &suffix,
    };
    port.persist_snapshot(&snapshot, true, &retained).expect("bounded complete compaction");
    port.persist_hard_state(3, Some(super::tests::NODE_B), false).expect("completed effect batch");
    let records = &port.adapter().state().durable_log;
    assert_eq!(records.len(), LARGE_SUFFIX_ENTRIES + 3, "header, individual entries, footer and cleared marker");
    assert!(records.first().expect("base").sequence > 0);
    assert!(records.iter().all(|record| record.value.len() as u64 <= SERVICE_OPERATION_BYTES));
    assert!(
        records.iter().all(|record| record.value.len() < 1_024),
        "no temporary base value needs the full suffix"
    );
    let suffix_bytes = records[1..=LARGE_SUFFIX_ENTRIES].iter().map(|record| record.value.len()).sum::<usize>();
    assert!(
        u64::try_from(suffix_bytes).unwrap() > SERVICE_OPERATION_BYTES,
        "a one-record base would exceed the real operation bound"
    );
    assert_eq!(
        suffix.iter().map(|entry| &entry.request_ref).collect::<std::collections::BTreeSet<_>>().len(),
        LARGE_SUFFIX_ENTRIES
    );
    assert_eq!(
        suffix.iter().map(|entry| &entry.command_ref).collect::<std::collections::BTreeSet<_>>().len(),
        LARGE_SUFFIX_ENTRIES
    );
    drop(port);
    let reopened = open_large_suffix_port(&root);
    let group = super::tests::active_group();
    let recovered = reopened.plan_recovery(recovery_start_plan(&group, initial)).expect("reopened complete base");
    assert_large_suffix_recovery(&recovered.start_plan.state, &snapshot, &suffix);
    assert_eq!(recovered.start_plan.state.rejoin, ReplicaRejoin::Voter);
    assert_eq!(recovered.replay_entry_count, LARGE_SUFFIX_ENTRIES);
}

// r[verify molten.consensus.durable_log_compaction]
#[test]
fn every_interrupted_base_prefix_preserves_committed_suffix_or_denies_truncated_recovery() {
    let root = crate::test_support::process_workspace("raft-compaction-base-prefixes").expect("workspace");
    let (mut port, initial, snapshot, suffix) = large_suffix_fixture(&root);
    let retained = ReplicaRetainedState {
        term: 3,
        voted_for: Some(super::tests::NODE_B),
        entries: &suffix,
    };
    port.persist_snapshot_through(&snapshot, true, &retained, Some(CompactionStep::BaseWritten))
        .expect("base before retirement or truncation");
    let records = &port.adapter().state().durable_log;
    let coverage = port.adapter().state().snapshots[&snapshot.snapshot_ref].covered_log_sequence;
    let base_start = records
        .iter()
        .position(|record| Some(record.sequence) == coverage.map(|value| value + 1))
        .expect("header immediately follows snapshot coverage");
    let bytes = crate::preserves_rail::canonical_bytes(&snapshot_value(&snapshot)).expect("snapshot bytes");
    let group = super::tests::active_group();
    let start = || recovery_start_plan(&group, initial.clone());
    for end in base_start..=records.len() {
        let plan = plan_replica(start(), &records[..end], Some(&bytes), coverage)
            .expect("old records retain every committed entry while the new base is incomplete");
        assert_large_suffix_recovery(&plan.start_plan.state, &snapshot, &suffix);
        if end > base_start {
            assert!(matches!(plan.start_plan.state.rejoin, ReplicaRejoin::Recovering(_)));
            let truncated = plan_replica(start(), &records[base_start..end], Some(&bytes), coverage);
            if end == records.len() {
                assert_large_suffix_recovery(
                    &truncated.expect("footer completes truncated base").start_plan.state,
                    &snapshot,
                    &suffix,
                );
            } else {
                assert!(truncated.is_err(), "truncated prefix {end} must not expose a partial suffix");
            }
        }
    }
    // A footer cannot claim completion with missing entries, even with a gap-free sequence.
    let mut early_footer = records.last().expect("footer").clone();
    early_footer.sequence = records[base_start].sequence + 1;
    let incomplete = vec![records[base_start].clone(), early_footer];
    assert!(plan_replica(start(), &incomplete, Some(&bytes), coverage).is_err());
    // Wrong footer identity must not complete an otherwise full base.
    let mut wrong_footer = records[base_start..].to_vec();
    let last = wrong_footer.last_mut().expect("footer");
    last.value =
        crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record("raft-compaction-end-v1", vec![
            crate::preserves_rail::string(super::tests::test_ref("different-base-header")),
        ]))
        .expect("wrong footer bytes");
    last.value_ref = crate::preserves_rail::content_ref_from_bytes(&last.value);
    assert!(plan_replica(start(), &wrong_footer, Some(&bytes), coverage).is_err());
    // A complete base cannot rewrite a committed request from the pre-truncation log.
    let mut changed_base = records.to_vec();
    let mut changed_entry = suffix[0].clone();
    changed_entry.request_ref = super::tests::test_ref("changed-committed-request");
    let changed = &mut changed_base[base_start + 1];
    changed.value =
        crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record("raft-log-mutation-v1", vec![
            crate::preserves_rail::record("none", Vec::new()),
            crate::preserves_rail::sequence(vec![canonical::entry_value(&changed_entry)]),
        ]))
        .expect("changed entry bytes");
    changed.value_ref = crate::preserves_rail::content_ref_from_bytes(&changed.value);
    let error = plan_replica(start(), &changed_base, Some(&bytes), coverage).expect_err("committed replacement denied");
    assert!(error.to_string().contains("compaction base changes committed entries"));
}

// r[verify molten.consensus.durable_log_compaction]
#[test]
fn interrupted_base_writes_reopen_with_the_original_full_suffix_and_recovery_marker() {
    for (label, step) in [
        ("header", CompactionStep::BaseStarted),
        ("first-entry", CompactionStep::BaseEntriesWritten(1)),
        ("last-entry", CompactionStep::BaseEntriesWritten(LARGE_SUFFIX_ENTRIES)),
    ] {
        let root =
            crate::test_support::process_workspace(&format!("raft-compaction-interrupted-{label}")).expect("workspace");
        let (mut port, initial, snapshot, suffix) = large_suffix_fixture(&root);
        let retained = ReplicaRetainedState {
            term: 3,
            voted_for: Some(super::tests::NODE_B),
            entries: &suffix,
        };
        port.persist_snapshot_through(&snapshot, true, &retained, Some(step))
            .expect("interrupted base write");
        let expected_records = match step {
            CompactionStep::BaseStarted => 1,
            CompactionStep::BaseEntriesWritten(count) => count + 1,
            _ => unreachable!("only incomplete bases"),
        };
        let coverage = port.adapter().state().snapshots[&snapshot.snapshot_ref].covered_log_sequence.expect("coverage");
        assert_eq!(
            port.adapter().state().durable_log.iter().filter(|record| record.sequence > coverage).count(),
            expected_records
        );
        drop(port);
        let mut reopened = open_large_suffix_port(&root);
        let group = super::tests::active_group();
        let recovery = reopened
            .plan_recovery(recovery_start_plan(&group, initial.clone()))
            .expect("reopened incomplete base");
        assert_large_suffix_recovery(&recovery.start_plan.state, &snapshot, &suffix);
        assert!(matches!(recovery.start_plan.state.rejoin, ReplicaRejoin::Recovering(_)));
        // Startup's marker terminates the abandoned frame. Another restart must still recover
        // the original suffix, never interpret later ordinary writes as completing that base.
        reopened.persist_hard_state(3, Some(super::tests::NODE_B), true).expect("startup recovery marker");
        drop(reopened);
        let reopened_again = open_large_suffix_port(&root);
        let recovered_again =
            reopened_again.plan_recovery(recovery_start_plan(&group, initial)).expect("second restart");
        assert_large_suffix_recovery(&recovered_again.start_plan.state, &snapshot, &suffix);
        assert!(matches!(recovered_again.start_plan.state.rejoin, ReplicaRejoin::Recovering(_)));
    }
}
