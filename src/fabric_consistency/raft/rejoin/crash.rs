//! A crash between the recovery log repair and the snapshot it precedes, replayed through Redb.
//!
//! The kernel plans the repair (`molten-raft-core`'s `rejoin::repair`); this test drives it with
//! peer recovery reports through the live transition, persists the pre-snapshot effects through the
//! durable port, stops, and replays the store, so the committed tail survives the restart.

use super::*;

const RECOVERY_TERM: u64 = 2;
const OLDER_TERM: u64 = 1;
const SNAPSHOT_INDEX: u64 = 1;
const SUFFIX_INDEX: u64 = 2;
const COMMITTED_TAIL_INDEX: u64 = 3;

fn entry(index: u64, term: u64, label: &str) -> ReplicatedEntry {
    ReplicatedEntry {
        index,
        term,
        request_ref: tests::test_ref(&format!("{label}-request")),
        command_ref: tests::test_ref(&format!("{label}-command")),
        command_schema_ref: tests::test_ref("recovery-schema"),
    }
}

/// B's indices 1–2 are faulty (term 1) but its index-3 entry E (term 2) is correct. C committed
/// all three with B; A holds only a snapshot through index 1.
struct CommittedTail {
    local: Vec<ReplicatedEntry>,
    committed: Vec<ReplicatedEntry>,
    snapshot: ReplicaSnapshot,
}

impl CommittedTail {
    fn new() -> Self {
        let template = recovering(Vec::new(), std::collections::BTreeMap::new());
        let mut snapshot = ReplicaSnapshot {
            snapshot_ref: String::new(),
            group_binding_ref: template.profile.group_binding_ref.clone(),
            membership_ref: template.membership.membership_ref.clone(),
            config_epoch: template.membership.config_epoch,
            fencing_epoch: template.profile.fencing_epoch,
            last_included_index: SNAPSHOT_INDEX,
            last_included_term: RECOVERY_TERM,
            application_state_ref: tests::test_ref("recovery-peer-application-state"),
            completed_requests: std::collections::BTreeMap::from([(
                tests::test_ref("recovery-committed-request"),
                SNAPSHOT_INDEX,
            )]),
        };
        snapshot.snapshot_ref = snapshot_ref(&snapshot).expect("peer snapshot identity");
        let tail = entry(COMMITTED_TAIL_INDEX, RECOVERY_TERM, "committed-tail");
        Self {
            local: vec![
                entry(SNAPSHOT_INDEX, OLDER_TERM, "faulty-first"),
                entry(SUFFIX_INDEX, OLDER_TERM, "faulty-second"),
                tail.clone(),
            ],
            committed: vec![
                entry(SNAPSHOT_INDEX, RECOVERY_TERM, "recovery-committed"),
                entry(SUFFIX_INDEX, RECOVERY_TERM, "committed-second"),
                tail,
            ],
            snapshot,
        }
    }

    /// A's complete snapshot observation, as recovery records it.
    fn snapshot_observation(&self) -> RecoveryObservation {
        RecoveryObservation {
            observation_ref: self.snapshot_observation_ref(),
            current_term: RECOVERY_TERM,
            commit_index: SNAPSHOT_INDEX,
            last_log_index: SNAPSHOT_INDEX,
            entries: Vec::new(),
            snapshot: Some(Box::new(self.snapshot.clone())),
            is_complete: true,
        }
    }

    fn snapshot_observation_ref(&self) -> String {
        canonical::recovery_corpus_ref(RECOVERY_TERM, SNAPSHOT_INDEX, &[], Some(&self.snapshot)).expect("A corpus")
    }
}

/// Node B recovering at `RECOVERY_TERM` behind a barrier, holding `log`, with `peers` reported.
fn recovering(
    log: Vec<ReplicatedEntry>,
    peers: std::collections::BTreeMap<String, RecoveryObservation>,
) -> ReplicaState {
    let mut state = tests::started_state(&tests::active_group(), tests::NODE_B);
    state.current_term = RECOVERY_TERM;
    state.active_election_timer_ref = election_timer_ref(
        &state.profile.group_binding_ref,
        &state.node_id,
        state.profile.service_generation,
        state.current_term,
        state.election_timer_sequence,
    )
    .expect("recovery timer ref");
    state.log = log;
    state.rejoin = ReplicaRejoin::Recovering(RecoveryRound {
        round_ref: state.active_election_timer_ref.clone(),
        fence_term: Some(RECOVERY_TERM),
        peers,
    });
    state
}

/// A complete first-page recovery report from `peer` for the state's active round.
fn report(state: &ReplicaState, peer: &str, corpus: &RecoveryObservation) -> ReplicaEvent {
    let ReplicaRejoin::Recovering(round) = &state.rejoin else {
        panic!("recovering state");
    };
    ReplicaEvent::Message {
        envelope: ReplicaMessageEnvelope {
            group_binding_ref: state.profile.group_binding_ref.clone(),
            service_generation: state.profile.service_generation,
            from: peer.to_string(),
            to: state.node_id.clone(),
            message: Message::RecoveryReport {
                term: corpus.current_term,
                peer_id: peer.to_string(),
                round_ref: round.round_ref.clone(),
                observation_ref: corpus.observation_ref.clone(),
                from_index: INITIAL_LOG_INDEX,
                last_log_index: corpus.last_log_index,
                commit_index: corpus.commit_index,
                entries: corpus.entries.clone(),
                snapshot: corpus.snapshot.clone(),
                config_epoch: state.membership.config_epoch,
                fencing_epoch: state.profile.fencing_epoch,
            },
        },
    }
}

fn suffix_retention(transition: &ReplicaTransition) -> Option<bool> {
    transition.effects.iter().find_map(|effect| match effect {
        ReplicaEffect::PersistSnapshot { is_suffix_retained, .. } => Some(*is_suffix_retained),
        _ => None,
    })
}

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[test]
fn a_crash_between_the_repair_and_the_snapshot_then_peer_loss_keeps_the_committed_tail() {
    let scenario = CommittedTail::new();
    let state = recovering(
        scenario.local.clone(),
        std::collections::BTreeMap::from([(tests::NODE_A.to_string(), scenario.snapshot_observation())]),
    );
    let c_corpus = RecoveryObservation {
        observation_ref: canonical::recovery_corpus_ref(RECOVERY_TERM, COMMITTED_TAIL_INDEX, &scenario.committed, None)
            .expect("C corpus"),
        current_term: RECOVERY_TERM,
        commit_index: COMMITTED_TAIL_INDEX,
        last_log_index: COMMITTED_TAIL_INDEX,
        entries: scenario.committed.clone(),
        snapshot: None,
        is_complete: true,
    };
    let repaired = apply_replica_event(&state, report(&state, tests::NODE_C, &c_corpus)).expect("repair and install");
    let replayed = replay_after_crash(&scenario, &repaired);
    assert_eq!(replayed, scenario.committed, "the repaired log replays with E");

    // C is lost before the next round. The repaired boundary now matches A's snapshot, so A's
    // report alone installs it and the committed tail survives.
    let after_crash = recovering(replayed, std::collections::BTreeMap::new());
    let installed =
        apply_replica_event(&after_crash, report(&after_crash, tests::NODE_A, &scenario.snapshot_observation()))
            .expect("install from the repaired log");
    assert_eq!(installed.next.snapshot.as_ref(), Some(&scenario.snapshot));
    assert_eq!(installed.next.log, scenario.committed[1..].to_vec());
    assert_eq!(installed.next.log.last(), scenario.committed.last());
    assert_eq!(suffix_retention(&installed), Some(true));
}

/// Persist the faulty log and the repair's pre-snapshot effects through Redb, stop before the
/// snapshot, reopen the store, and return the log recovery replays.
fn replay_after_crash(scenario: &CommittedTail, repaired: &ReplicaTransition) -> Vec<ReplicatedEntry> {
    let root = crate::test_support::process_workspace("raft-recovery-committed-tail-crash").expect("workspace");
    let log_ref = tests::test_ref("committed-tail-crash-log");
    let store_ref = tests::test_ref("committed-tail-crash-store");
    let open = || {
        let adapter = crate::fabric_durability::RedbDurableStateAdapter::open(
            &root,
            crate::fabric_durability::tests::profile(crate::fabric_durability::DurableAdapterKind::LiveRedb),
            crate::fabric_durability::tests::descriptor(),
        )
        .expect("Redb adapter");
        RedbReplicaPort::new(adapter, log_ref.clone(), store_ref.clone()).expect("durability port")
    };
    let mut port = open();
    port.persist_hard_state(RECOVERY_TERM, None, true).expect("recovery marker");
    port.persist_entries(None, &scenario.local).expect("faulty local log");
    port.flush_log(COMMITTED_TAIL_INDEX).expect("durable local log");
    // The process stops after the log repair and before the snapshot is written.
    let before_snapshot = repaired
        .effects
        .iter()
        .take_while(|effect| !matches!(effect, ReplicaEffect::PersistSnapshot { .. }));
    for effect in before_snapshot {
        match effect {
            ReplicaEffect::PersistEntries { truncate_from, entries } => {
                port.persist_entries(*truncate_from, entries).expect("repair mutation");
            }
            ReplicaEffect::FlushLog { through_index } => {
                port.flush_log(*through_index).expect("repair flush");
            }
            other => panic!("unexpected pre-snapshot effect {other:?}"),
        }
    }
    drop(port);

    let group = tests::active_group();
    let recovered = open()
        .plan_recovery(ReplicaStartPlan {
            state: tests::started_state(&group, tests::NODE_B),
            service_id: group.service_id.clone(),
            application_manifest_ref: group.application_manifest_ref.clone(),
            initial_effects: Vec::new(),
            port_binding_refs: Vec::new(),
            production_admitted: false,
        })
        .expect("restart after the crash");
    assert!(matches!(recovered.start_plan.state.rejoin, ReplicaRejoin::Recovering(_)));
    recovered.start_plan.state.log
}
