//! A faulty local prefix is rewritten from verified committed peer copies, never dropping a
//! committed tail.

use super::*;

const COMMITTED_TAIL_INDEX: u64 = 3;
const FAULTY_HIGHER_TERM: u64 = 3;

/// The review scenario: B's indices 1–2 are faulty (term 1) but its index-3 entry E (term 2)
/// is correct. C committed Y, X and E with B; A holds only a snapshot through index 1. With
/// C's report, the repair must rewrite indices 1–2 and keep E in the same durable mutation.
struct CommittedTail {
    local: Vec<ReplicatedEntry>,
    committed: Vec<ReplicatedEntry>,
    snapshot: ReplicaSnapshot,
}

impl CommittedTail {
    fn new() -> Self {
        let template = crate::tests::support::started_state(crate::tests::support::NODE_B);
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
            snapshot: peer_snapshot(&template, RECOVERY_TERM),
        }
    }

    fn plan(&self, log: Vec<ReplicatedEntry>, is_c_reported: bool) -> MessagePlan<'static> {
        let mut peers = std::collections::BTreeMap::from([(
            crate::tests::support::NODE_A.to_string(),
            snapshot_observation(&self.snapshot, SNAPSHOT_INDEX, Vec::new()),
        )]);
        if is_c_reported {
            peers.insert(crate::tests::support::NODE_C.to_string(), RecoveryObservation {
                observation_ref: crate::tests::support::test_ref("committed-tail-observation"),
                current_term: RECOVERY_TERM,
                commit_index: COMMITTED_TAIL_INDEX,
                last_log_index: COMMITTED_TAIL_INDEX,
                entries: self.committed.clone(),
                snapshot: None,
                is_complete: true,
            });
        }
        recovering_plan(log, peers)
    }
}

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[test]
fn a_committed_tail_after_replaced_entries_is_rewritten_in_the_same_mutation_and_never_dropped() {
    let scenario = CommittedTail::new();
    let mut plan = scenario.plan(scenario.local.clone(), true);
    let outcome = molten_core::consensus_recovery::reconcile_participant(
        &input(&plan.next, &crate::tests::support::TestIdentity).expect("input"),
    );
    assert!(matches!(
        outcome.decisions.iter().find(|(index, _)| *index == SUFFIX_INDEX),
        Some((_, molten_core::consensus_recovery::EntryRepairDecision::ReplaceWithCommitted { .. }))
    ));
    assert!(matches!(
        outcome.decisions.iter().find(|(index, _)| *index == COMMITTED_TAIL_INDEX),
        Some((_, molten_core::consensus_recovery::EntryRepairDecision::KeepLocal))
    ));

    reconcile(&mut plan).expect("repair and install");
    assert_valid(&plan.next, "installed state");
    assert_eq!(plan.next.snapshot.as_ref(), Some(&scenario.snapshot));
    assert_eq!(plan.next.log, scenario.committed[1..].to_vec());
    // Indices 1–2 take C's verified committed copies and E is rewritten in that same durable
    // mutation, so no effect ever leaves E only on C.
    assert!(matches!(plan.effects.as_slice(), [
        ReplicaEffect::PersistEntries { truncate_from: Some(SNAPSHOT_INDEX), entries },
        ReplicaEffect::FlushLog { through_index: COMMITTED_TAIL_INDEX },
        ReplicaEffect::PersistSnapshot { is_suffix_retained: true, .. },
        ReplicaEffect::RestoreApplicationSnapshot { .. },
    ] if *entries == scenario.committed));
}

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[test]
fn a_lost_peer_before_the_rewrite_defers_and_keeps_the_committed_tail() {
    let scenario = CommittedTail::new();
    let mut plan = scenario.plan(scenario.local.clone(), false);
    let before = plan.next.clone();
    reconcile(&mut plan).expect("available repair");
    assert_eq!(plan.next, before, "without C's committed copies nothing is rewritten or dropped");
    assert!(plan.effects.is_empty());
}

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[test]
fn a_repaired_log_installs_from_one_snapshot_report_and_keeps_the_committed_tail() {
    // The state a restart replays after the repair mutation landed and before the snapshot did:
    // the repaired boundary now matches A's snapshot, so A's report alone installs it.
    let scenario = CommittedTail::new();
    let mut after_crash = scenario.plan(scenario.committed.clone(), false);
    reconcile(&mut after_crash).expect("install from the repaired log");
    assert_valid(&after_crash.next, "installed state");
    assert_eq!(after_crash.next.snapshot.as_ref(), Some(&scenario.snapshot));
    assert_eq!(after_crash.next.log, scenario.committed[1..].to_vec());
    assert_eq!(after_crash.next.log.last(), scenario.committed.last());
    assert_eq!(suffix_retention(&after_crash), Some(true));
}

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[test]
fn a_faulty_higher_term_boundary_is_repaired_from_the_verified_committed_copy_and_rejoins() {
    // B's faulty log is (1, term 3), (2, term 3); A has a snapshot through (1, term 2) and C has
    // committed (1, term 2), (2, term 2). Rewriting only index 2 would leave terms 3, 2.
    let committed = vec![
        entry(SNAPSHOT_INDEX, RECOVERY_TERM, "recovery-committed"),
        entry(SUFFIX_INDEX, RECOVERY_TERM, "reversed-committed-second"),
    ];
    let faulty = vec![
        entry(SNAPSHOT_INDEX, FAULTY_HIGHER_TERM, "reversed-faulty-first"),
        entry(SUFFIX_INDEX, FAULTY_HIGHER_TERM, "reversed-faulty-second"),
    ];
    let (mut plan, snapshot) = committed_log_plan(faulty, &committed, "reversed-term-observation");
    assert!(matches!(
        suffix_decision(&plan),
        molten_core::consensus_recovery::EntryRepairDecision::ReplaceWithCommitted { .. }
    ));
    assert_repaired_and_admitted(&mut plan, &snapshot, &committed);
}

// r[verify molten.consensus.storage_fault_recovery]
// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[test]
fn a_faulty_uncommitted_same_term_boundary_is_repaired_when_committed_evidence_agrees() {
    // A's snapshot and C's committed log agree on R1 at (1, term 2); B's uncommitted boundary
    // entry has term 2 but request R2 (a local storage fault), and its index-2 entry is correct.
    let committed = vec![
        entry(SNAPSHOT_INDEX, RECOVERY_TERM, "recovery-committed"),
        entry(SUFFIX_INDEX, RECOVERY_TERM, "agreed-committed-second"),
    ];
    let faulty_boundary = entry(SNAPSHOT_INDEX, RECOVERY_TERM, "same-term-faulty-boundary");
    let (mut plan, snapshot) =
        committed_log_plan(vec![faulty_boundary, committed[1].clone()], &committed, "agreed-committed-observation");
    assert_repaired_and_admitted(&mut plan, &snapshot, &committed);
}

/// B recovers with `log`; A reports a snapshot through index 1 at `RECOVERY_TERM` and C reports
/// `committed` as its committed log.
fn committed_log_plan(
    log: Vec<ReplicatedEntry>,
    committed: &[ReplicatedEntry],
    observation_label: &str,
) -> (MessagePlan<'static>, ReplicaSnapshot) {
    let template = crate::tests::support::started_state(crate::tests::support::NODE_B);
    let snapshot = peer_snapshot(&template, RECOVERY_TERM);
    let peers = std::collections::BTreeMap::from([
        (
            crate::tests::support::NODE_A.to_string(),
            snapshot_observation(&snapshot, SNAPSHOT_INDEX, Vec::new()),
        ),
        (crate::tests::support::NODE_C.to_string(), RecoveryObservation {
            observation_ref: crate::tests::support::test_ref(observation_label),
            current_term: RECOVERY_TERM,
            commit_index: SUFFIX_INDEX,
            last_log_index: SUFFIX_INDEX,
            entries: committed.to_vec(),
            snapshot: None,
            is_complete: true,
        }),
    ]);
    (recovering_plan(log, peers), snapshot)
}

/// The boundary and index 2 are rewritten from C's verified copies in term order, the snapshot
/// installs, and the next reconciliation admits B as a vote-fenced voter.
fn assert_repaired_and_admitted(plan: &mut MessagePlan<'_>, snapshot: &ReplicaSnapshot, committed: &[ReplicatedEntry]) {
    reconcile(plan).expect("repair and install");
    assert_valid(&plan.next, "installed state");
    assert!(
        matches!(plan.effects.as_slice(), [
            ReplicaEffect::PersistEntries { truncate_from: Some(SNAPSHOT_INDEX), entries },
            ReplicaEffect::FlushLog { through_index: SUFFIX_INDEX },
            ReplicaEffect::PersistSnapshot { is_suffix_retained: true, .. },
            ReplicaEffect::RestoreApplicationSnapshot { .. },
        ] if entries.as_slice() == committed),
        "the faulty prefix is rewritten from C's verified copies: {:?}",
        plan.effects
    );
    assert_eq!(plan.next.snapshot.as_ref(), Some(snapshot));
    assert_eq!(plan.next.log, committed[1..].to_vec());

    reconcile(plan).expect("admission");
    assert_valid(&plan.next, "admitted state");
    assert!(matches!(plan.next.rejoin, ReplicaRejoin::VoteFenced { .. }), "B rejoins: {:?}", plan.next.rejoin);
    assert_eq!(plan.next.commit_index, SUFFIX_INDEX);
    assert_eq!(plan.next.log, committed[1..].to_vec());
}
