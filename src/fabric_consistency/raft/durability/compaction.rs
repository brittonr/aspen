// Durable compaction under explicit retention authority. Included by `durability/tests.rs`.


const COMPACTION_LOG_ENTRIES: usize = 4;
const COMPACTION_SNAPSHOT_THRESHOLD: usize = 2;
const COMPACTION_WINDOW: u64 = 3;

fn compaction_bounded(mut state: ReplicaState) -> ReplicaState {
    state.profile.max_log_entries = COMPACTION_LOG_ENTRIES;
    state.profile.snapshot_threshold_entries = COMPACTION_SNAPSHOT_THRESHOLD;
    state.profile.request_retention_entries = COMPACTION_WINDOW;
    state
}

/// Persist a leader transition's durable effects the way the executor orders them, ending a guarded
/// batch with the cleared recovery marker.
fn persist_durable_effects(port: &mut RedbReplicaPort, transition: &ReplicaTransition) {
    let mut is_guarded = false;
    for effect in &transition.effects {
        is_guarded |= match effect {
            ReplicaEffect::PersistHardState { term, voted_for, .. } => {
                port.persist_hard_state(*term, voted_for.as_deref(), true).expect("hard state");
                true
            }
            ReplicaEffect::PersistEntries { truncate_from, entries } => {
                port.persist_entries(*truncate_from, entries).expect("entries");
                true
            }
            ReplicaEffect::FlushLog { through_index } => {
                port.flush_log(*through_index).expect("flush");
                true
            }
            ReplicaEffect::PersistCommit { through_index } => {
                port.persist_commit(*through_index).expect("commit");
                true
            }
            ReplicaEffect::PersistSnapshot { snapshot, is_suffix_retained } => {
                let retained = ReplicaRetainedState::after(&transition.next, snapshot);
                port.persist_snapshot(snapshot, *is_suffix_retained, &retained).expect("snapshot");
                true
            }
            _ => false,
        };
    }
    if is_guarded {
        let next = &transition.next;
        port.persist_hard_state(next.current_term, next.voted_for.as_deref(), false).expect("cleared marker");
    }
}

struct DurableLeader {
    leader: ReplicaState,
    follower: ReplicaState,
    port: RedbReplicaPort,
}

impl DurableLeader {
    fn elected(port: RedbReplicaPort) -> Self {
        let group = super::tests::active_group();
        let node_a = compaction_bounded(super::tests::started_state(&group, super::tests::NODE_A));
        let node_b = compaction_bounded(super::tests::started_state(&group, super::tests::NODE_B));
        let mut durable = Self { leader: node_a.clone(), follower: node_b, port };
        let election = durable.step(ReplicaEvent::ElectionTimeout {
            timer_ref: node_a.active_election_timer_ref.clone(),
        });
        let vote = apply_replica_event(&durable.follower, ReplicaEvent::Message {
            envelope: super::tests::sent_envelope_to(&election, super::tests::NODE_B),
        })
        .expect("vote");
        durable.follower = vote.next.clone();
        durable.step(ReplicaEvent::Message {
            envelope: super::tests::sent_envelope_to(&vote, super::tests::NODE_A),
        });
        assert_eq!(durable.leader.role, ReplicaRole::Leader);
        durable
    }

    fn step(&mut self, event: ReplicaEvent) -> ReplicaTransition {
        let transition = apply_replica_event(&self.leader, event).expect("leader transition");
        persist_durable_effects(&mut self.port, &transition);
        self.leader = transition.next.clone();
        transition
    }

    /// Commit one write; returns the snapshot event when the threshold is due afterwards.
    fn commit(&mut self, label: &str) -> Option<ReplicaEvent> {
        let proposal = self.step(ReplicaEvent::Propose {
            request_ref: super::tests::test_ref(label),
            command_ref: super::tests::test_ref(&format!("{label}-command")),
            command_schema_ref: super::tests::test_ref("compaction-schema"),
        });
        let replicated = apply_replica_event(&self.follower, ReplicaEvent::Message {
            envelope: super::tests::sent_envelope_to(&proposal, super::tests::NODE_B),
        })
        .expect("follower append");
        self.follower = replicated.next.clone();
        if is_snapshot_due(&self.follower) {
            self.follower = apply_replica_event(&self.follower, ReplicaEvent::CreateSnapshot {
                application_state_ref: super::tests::test_ref("compaction-follower-application"),
            }).expect("follower threshold snapshot").next;
        }
        self.step(ReplicaEvent::Message {
            envelope: super::tests::sent_envelope_to(&replicated, super::tests::NODE_A),
        });
        is_snapshot_due(&self.leader).then(|| ReplicaEvent::CreateSnapshot {
            application_state_ref: super::tests::test_ref(&format!("compaction-application-{}", self.leader.last_applied)),
        })
    }
}

fn open_compacting_port(root: &std::path::Path) -> RedbReplicaPort {
    let adapter = crate::fabric_durability::RedbDurableStateAdapter::open(
        root,
        crate::fabric_durability::tests::profile(crate::fabric_durability::DurableAdapterKind::LiveRedb),
        crate::fabric_durability::tests::descriptor(),
    )
    .expect("Redb adapter");
    let authority = adapter.state().descriptor.retention_authority_ref.clone().expect("namespace retention authority");
    RedbReplicaPort::new(adapter, super::tests::test_ref("compaction-log"), super::tests::test_ref("compaction-snapshots"))
        .expect("port")
        .with_retention_authority(authority)
        .expect("explicit retention authority")
}

fn recovered_state(port: &RedbReplicaPort) -> ReplicaState {
    let group = super::tests::active_group();
    let initial = compaction_bounded(super::tests::started_state(&group, super::tests::NODE_A));
    port.plan_recovery(recovery_start_plan(&group, initial)).expect("recovery").start_plan.state
}

fn assert_recovers(recovered: &ReplicaState, compacted: &ReplicaState) {
    assert_eq!(recovered.snapshot, compacted.snapshot);
    assert_eq!(recovered.log, compacted.log);
    assert_eq!(recovered.commit_index, compacted.commit_index);
    assert_eq!(recovered.completed_requests, compacted.completed_requests);
    assert_eq!(recovered.current_term, compacted.current_term);
    assert_eq!(recovered.voted_for, compacted.voted_for);
}

// r[verify molten.consensus.durable_log_compaction]
// r[verify molten.fabric_durability.snapshot_compaction]
#[test]
fn compaction_keeps_records_and_snapshots_bounded_beyond_the_snapshot_limit() {
    let root = crate::test_support::process_workspace("live-raft-bounded-compaction").expect("workspace");
    let mut durable = DurableLeader::elected(open_compacting_port(&root));
    let snapshot_limit = durable.port.adapter().profile().profile.max_snapshots;
    let record_limit = durable.port.adapter().profile().profile.max_log_records;
    let writes = snapshot_limit * u64::try_from(COMPACTION_SNAPSHOT_THRESHOLD).expect("threshold") + 6;
    let mut snapshots = 0_u64;
    for write in 0..writes {
        if let Some(event) = durable.commit(&format!("compaction-write-{write}")) {
            durable.step(event);
            snapshots += 1;
            let state = durable.port.adapter().state();
            assert_eq!(state.snapshots.len(), 1, "only the newest snapshot stays");
            assert!(state.durable_log.first().is_some_and(|record| record.sequence > 0));
        }
        assert!((durable.port.adapter().state().durable_log.len() as u64) < record_limit);
    }
    assert!(snapshots > snapshot_limit, "more snapshots than the namespace could ever hold at once");
    let compacted = durable.leader.clone();
    drop(durable);
    let reopened = open_compacting_port(&root);
    let recovered = recovered_state(&reopened);
    assert_recovers(&recovered, &compacted);
    assert!(matches!(recovered.rejoin, ReplicaRejoin::Voter));
}

// r[verify molten.consensus.durable_log_compaction]
#[test]
fn a_crash_after_any_compaction_step_recovers_the_compacted_state() {
    for (label, last_step) in [
        ("snapshot", None),
        ("base", Some(CompactionStep::BaseWritten)),
        ("retired", Some(CompactionStep::SnapshotsRetired)),
        ("truncated", Some(CompactionStep::RecordsTruncated)),
    ] {
        let root = crate::test_support::process_workspace(&format!("live-raft-compaction-crash-{label}"))
            .expect("workspace");
        let mut durable = DurableLeader::elected(open_compacting_port(&root));
        let mut pending = None;
        let mut write = 0;
        // Two compacted snapshots, then a third interrupted after `last_step`.
        let mut completed = 0;
        while completed < 3 {
            if let Some(event) = pending.take() {
                let transition = apply_replica_event(&durable.leader, event).expect("snapshot transition");
                let ReplicaEffect::PersistSnapshot { snapshot, is_suffix_retained } = &transition.effects[0] else {
                    unreachable!("a snapshot transition persists its snapshot first");
                };
                let step = if completed == 2 { last_step } else { Some(CompactionStep::RecordsTruncated) };
                let retained = ReplicaRetainedState::after(&transition.next, snapshot);
                durable.port.persist_snapshot_through(snapshot, *is_suffix_retained, &retained, step).expect("persist");
                durable.leader = transition.next;
                completed += 1;
                continue;
            }
            pending = durable.commit(&format!("crash-{label}-{write}"));
            write += 1;
        }
        let compacted = durable.leader.clone();
        drop(durable);
        let recovered = recovered_state(&open_compacting_port(&root));
        assert_recovers(&recovered, &compacted);
    }
}

// r[verify molten.consensus.durable_log_compaction]
#[test]
fn recovery_rejects_retained_records_that_no_snapshot_covers() {
    let root = crate::test_support::process_workspace("live-raft-compaction-gap").expect("workspace");
    let mut durable = DurableLeader::elected(open_compacting_port(&root));
    let mut write = 0;
    while durable.leader.snapshot.is_none() {
        if let Some(event) = durable.commit(&format!("gap-write-{write}")) {
            durable.step(event);
        }
        write += 1;
    }
    let state = durable.port.adapter().state();
    let snapshot = durable.leader.snapshot.clone().expect("snapshot");
    let coverage = state.snapshots[&snapshot.snapshot_ref].covered_log_sequence;
    let bytes = crate::preserves_rail::canonical_bytes(&snapshot_value(&snapshot)).expect("snapshot bytes");
    let group = super::tests::active_group();
    let start = || recovery_start_plan(&group, compaction_bounded(super::tests::started_state(&group, super::tests::NODE_A)));
    assert!(plan_replica(start(), &state.durable_log, Some(&bytes), coverage).is_ok());
    // Losing the compaction base leaves a gap between the snapshot and the first retained record.
    assert!(plan_replica(start(), &state.durable_log[1..], Some(&bytes), coverage).is_err());
    // Retained records that no snapshot covers cannot start after sequence zero.
    assert!(plan_replica(start(), &state.durable_log, None, None).is_err());
}

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric_consistency/raft/durability/compaction/suffix.rs"));
