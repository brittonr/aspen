//! Recovery reconciliation against peer snapshots: boundary repair, suffix retention, divergence.

mod contradiction;
mod copies;

use super::*;

const RECOVERY_TERM: u64 = 2;
const OLDER_TERM: u64 = 1;
const SNAPSHOT_INDEX: u64 = 1;
const SUFFIX_INDEX: u64 = 2;
const UNCONFIRMED_INDEX: u64 = 3;

fn entry(index: u64, term: u64, label: &str) -> ReplicatedEntry {
    ReplicatedEntry {
        index,
        term,
        request_ref: crate::tests::support::test_ref(&format!("{label}-request")),
        command_ref: crate::tests::support::test_ref(&format!("{label}-command")),
        command_schema_ref: crate::tests::support::test_ref("recovery-schema"),
    }
}

fn peer_snapshot(state: &ReplicaState, last_included_term: u64) -> ReplicaSnapshot {
    let mut snapshot = ReplicaSnapshot {
        snapshot_ref: String::new(),
        group_binding_ref: state.profile.group_binding_ref.clone(),
        membership_ref: state.membership.membership_ref.clone(),
        config_epoch: state.membership.config_epoch,
        fencing_epoch: state.profile.fencing_epoch,
        last_included_index: SNAPSHOT_INDEX,
        last_included_term,
        application_state_ref: crate::tests::support::test_ref("recovery-peer-application-state"),
        completed_requests: std::collections::BTreeMap::from([(
            crate::tests::support::test_ref("recovery-committed-request"),
            SNAPSHOT_INDEX,
        )]),
    };
    snapshot.snapshot_ref = crate::tests::support::snapshot_ref(&snapshot).expect("peer snapshot identity");
    snapshot
}

fn lagging_observation() -> RecoveryObservation {
    RecoveryObservation {
        observation_ref: crate::tests::support::test_ref("recovery-lagging-observation"),
        current_term: RECOVERY_TERM,
        commit_index: INITIAL_COMMIT_INDEX,
        last_log_index: INITIAL_COMMIT_INDEX,
        entries: Vec::new(),
        snapshot: None,
        is_complete: true,
    }
}

/// Node B recovering at `RECOVERY_TERM` with `log`, once the given peers have reported.
fn recovering_plan(
    log: Vec<ReplicatedEntry>,
    peers: std::collections::BTreeMap<String, RecoveryObservation>,
) -> MessagePlan<'static> {
    let mut state = crate::tests::support::started_state(crate::tests::support::NODE_B);
    state.current_term = RECOVERY_TERM;
    state.active_election_timer_ref = crate::tests::support::election_timer_ref(&state);
    state.log = log;
    state.rejoin = ReplicaRejoin::Recovering(RecoveryRound {
        round_ref: state.active_election_timer_ref.clone(),
        fence_term: Some(RECOVERY_TERM),
        peers,
    });
    assert_valid(&state, "recovering state");
    MessagePlan {
        next: state,
        effects: Vec::new(),
        persist_hard_state: false,
        identity: &crate::tests::support::TestIdentity,
    }
}

fn assert_valid(state: &ReplicaState, label: &str) {
    validation::validate_replica_state(state, &crate::tests::support::TestIdentity).expect(label);
}

fn snapshot_observation(
    snapshot: &ReplicaSnapshot,
    commit_index: u64,
    entries: Vec<ReplicatedEntry>,
) -> RecoveryObservation {
    RecoveryObservation {
        observation_ref: crate::tests::support::test_ref("recovery-peer-observation"),
        current_term: RECOVERY_TERM,
        commit_index,
        last_log_index: commit_index,
        entries,
        snapshot: Some(Box::new(snapshot.clone())),
        is_complete: true,
    }
}

/// Node B recovers holding uncommitted term-1 entries at indices 1 and 2; node A reports a
/// snapshot through index 1 taken at `snapshot_term`. With `is_lagging_peer_reported`, node C
/// also reports an empty log, so both other voters lack the local suffix entry.
fn recovering_with_suffix(
    snapshot_term: u64,
    is_lagging_peer_reported: bool,
) -> (MessagePlan<'static>, ReplicaSnapshot) {
    let template = crate::tests::support::started_state(crate::tests::support::NODE_B);
    let snapshot = peer_snapshot(&template, snapshot_term);
    let mut peers = std::collections::BTreeMap::from([(
        crate::tests::support::NODE_A.to_string(),
        snapshot_observation(&snapshot, SNAPSHOT_INDEX, Vec::new()),
    )]);
    if is_lagging_peer_reported {
        peers.insert(crate::tests::support::NODE_C.to_string(), lagging_observation());
    }
    let log = vec![
        // The request the peer snapshot records at index 1, so a matching term is consistent.
        entry(SNAPSHOT_INDEX, OLDER_TERM, "recovery-committed"),
        entry(SUFFIX_INDEX, OLDER_TERM, "local-suffix"),
    ];
    (recovering_plan(log, peers), snapshot)
}

/// B's boundary entry is faulty (term 1), but its index-2 entry E (term 2) is correct; `extra`
/// follows it locally. A's snapshot and C's committed log establish (1, term 2), and both
/// confirm E as committed.
fn confirmed_suffix_plan(extra: Option<ReplicatedEntry>) -> (MessagePlan<'static>, ReplicaSnapshot, ReplicatedEntry) {
    let template = crate::tests::support::started_state(crate::tests::support::NODE_B);
    let snapshot = peer_snapshot(&template, RECOVERY_TERM);
    let confirmed = entry(SUFFIX_INDEX, RECOVERY_TERM, "confirmed-suffix");
    let committed_boundary = entry(SNAPSHOT_INDEX, RECOVERY_TERM, "recovery-committed");
    let peers = std::collections::BTreeMap::from([
        (
            crate::tests::support::NODE_A.to_string(),
            snapshot_observation(&snapshot, SUFFIX_INDEX, vec![confirmed.clone()]),
        ),
        (crate::tests::support::NODE_C.to_string(), RecoveryObservation {
            observation_ref: crate::tests::support::test_ref("recovery-committed-log-observation"),
            current_term: RECOVERY_TERM,
            commit_index: SUFFIX_INDEX,
            last_log_index: SUFFIX_INDEX,
            entries: vec![committed_boundary, confirmed.clone()],
            snapshot: None,
            is_complete: true,
        }),
    ]);
    let mut log = vec![entry(SNAPSHOT_INDEX, OLDER_TERM, "local-boundary"), confirmed.clone()];
    log.extend(extra);
    let plan = recovering_plan(log, peers);
    assert_eq!(suffix_decision(&plan), molten_core::consensus_recovery::EntryRepairDecision::KeepLocal);
    (plan, snapshot, confirmed)
}

fn suffix_decision(plan: &MessagePlan<'_>) -> molten_core::consensus_recovery::EntryRepairDecision {
    let outcome = molten_core::consensus_recovery::reconcile_participant(
        &input(&plan.next, &crate::tests::support::TestIdentity).expect("input"),
    );
    assert!(
        outcome
            .effects
            .iter()
            .any(|effect| matches!(effect, molten_core::consensus_recovery::RecoveryEffect::InstallSnapshot { .. })),
        "recovery plans the peer snapshot installation: {outcome:?}"
    );
    outcome
        .decisions
        .into_iter()
        .find_map(|(index, decision)| (index == SUFFIX_INDEX).then_some(decision))
        .expect("suffix decision")
}

fn suffix_retention(plan: &MessagePlan<'_>) -> Option<bool> {
    plan.effects.iter().find_map(|effect| match effect {
        ReplicaEffect::PersistSnapshot { is_suffix_retained, .. } => Some(*is_suffix_retained),
        _ => None,
    })
}

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[test]
fn a_faulty_boundary_with_a_peer_confirmed_suffix_retains_it_and_drops_only_the_uncommitted_tail() {
    let unconfirmed = entry(UNCONFIRMED_INDEX, RECOVERY_TERM, "unconfirmed-tail");
    let (mut plan, snapshot, confirmed) = confirmed_suffix_plan(Some(unconfirmed));
    reconcile(&mut plan).expect("snapshot installation");
    assert_valid(&plan.next, "installed state");
    assert_eq!(plan.next.snapshot.as_ref(), Some(&snapshot));
    assert_eq!(plan.next.log, vec![confirmed.clone()], "both peers lack index 3, so only it is discarded");
    // One durable mutation, before the snapshot is written, rewrites the faulty boundary from C's
    // verified committed copy, keeps E, and drops only the established-uncommitted index 3.
    let ReplicaEffect::PersistEntries { truncate_from, entries } = &plan.effects[0] else {
        panic!("the log repair precedes the snapshot: {:?}", plan.effects);
    };
    assert_eq!(*truncate_from, Some(SNAPSHOT_INDEX));
    assert_eq!(entries, &vec![entry(SNAPSHOT_INDEX, RECOVERY_TERM, "recovery-committed"), confirmed]);
    assert!(matches!(&plan.effects[1..], [
        ReplicaEffect::FlushLog { .. },
        ReplicaEffect::PersistSnapshot {
            is_suffix_retained: true,
            ..
        },
        ReplicaEffect::RestoreApplicationSnapshot { .. },
    ]));
}

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[test]
fn a_faulty_boundary_with_a_peer_confirmed_suffix_installs_retains_it_and_rejoins() {
    let (mut plan, snapshot, confirmed) = confirmed_suffix_plan(None);

    reconcile(&mut plan).expect("snapshot installation");
    assert_valid(&plan.next, "installed state");
    assert_eq!(plan.next.snapshot.as_ref(), Some(&snapshot));
    assert_eq!(plan.next.log, vec![confirmed.clone()], "the confirmed entry survives the faulty boundary");
    assert_eq!(suffix_retention(&plan), Some(true));

    reconcile(&mut plan).expect("admission");
    assert_valid(&plan.next, "admitted state");
    assert!(matches!(plan.next.rejoin, ReplicaRejoin::VoteFenced { .. }), "B rejoins: {:?}", plan.next.rejoin);
    assert_eq!(plan.next.commit_index, SUFFIX_INDEX);
    assert_eq!(plan.next.last_applied, SUFFIX_INDEX);
    assert_eq!(plan.next.log, vec![confirmed]);
}

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[test]
fn a_single_snapshot_report_defers_installation_over_an_unresolved_conflicting_suffix() {
    let (mut plan, _snapshot) = recovering_with_suffix(RECOVERY_TERM, false);
    let before = plan.next.clone();
    assert_eq!(suffix_decision(&plan), molten_core::consensus_recovery::EntryRepairDecision::Wait);
    reconcile(&mut plan).expect("available repair");
    assert_eq!(plan.next, before, "unresolved local evidence is kept and the round keeps waiting");
    assert!(plan.effects.is_empty());
}

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[test]
fn a_snapshot_with_a_newer_boundary_term_discards_a_suffix_established_uncommitted() {
    let (mut plan, snapshot) = recovering_with_suffix(RECOVERY_TERM, true);
    assert_eq!(suffix_decision(&plan), molten_core::consensus_recovery::EntryRepairDecision::RemoveUncommitted);
    reconcile(&mut plan).expect("barrier repair");
    assert_valid(&plan.next, "installed state keeps log-term monotonicity");
    assert!(plan.next.log.is_empty(), "the term-1 suffix cannot follow a term-2 snapshot boundary");
    assert_eq!(plan.next.snapshot.as_ref(), Some(&snapshot));
    assert_eq!(plan.next.commit_index, SNAPSHOT_INDEX);
    assert_eq!(suffix_retention(&plan), Some(false));
}

// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[test]
fn a_snapshot_with_a_matching_boundary_term_retains_the_suffix() {
    let (mut plan, snapshot) = recovering_with_suffix(OLDER_TERM, false);
    reconcile(&mut plan).expect("available repair");
    assert_valid(&plan.next, "installed state");
    assert_eq!(plan.next.snapshot.as_ref(), Some(&snapshot));
    assert_eq!(plan.next.log, vec![entry(SUFFIX_INDEX, OLDER_TERM, "local-suffix")]);
    assert_eq!(suffix_retention(&plan), Some(true));
}
