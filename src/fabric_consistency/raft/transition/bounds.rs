//! Long-running bounds: threshold snapshots keep proposals flowing past the log bound, and the
//! duplicate-request window depends only on the applied log prefix.

use super::*;

const SMALL_LOG_ENTRIES: usize = 4;
const SMALL_SNAPSHOT_THRESHOLD: usize = 2;
const SMALL_WINDOW: u64 = 3;
const WRITES_BEYOND_CAPACITY: u64 = 10;

fn bounded(mut state: ReplicaState, max_log_entries: usize) -> ReplicaState {
    state.profile.max_log_entries = max_log_entries;
    state.profile.snapshot_threshold_entries = SMALL_SNAPSHOT_THRESHOLD;
    state.profile.request_retention_entries = SMALL_WINDOW;
    state
}

fn apply(state: &ReplicaState, event: ReplicaEvent) -> ReplicaTransition {
    apply_replica_event(state, event).expect("admitted transition")
}

fn propose(state: &ReplicaState, label: &str) -> ReplicaTransition {
    apply(state, ReplicaEvent::Propose {
        request_ref: super::tests::test_ref(label),
        command_ref: super::tests::test_ref(&format!("{label}-command")),
        command_schema_ref: super::tests::test_ref("bounds-schema"),
    })
}

fn snapshot_if_due(state: ReplicaState) -> ReplicaState {
    if !is_snapshot_due(&state) {
        return state;
    }
    apply(&state, ReplicaEvent::CreateSnapshot {
        application_state_ref: super::tests::test_ref(&format!("application-at-{}", state.last_applied)),
    })
    .next
}

fn proposal_outcome(transition: &ReplicaTransition) -> Option<(ProposalDisposition, Option<u64>)> {
    transition.effects.iter().find_map(|effect| match effect {
        ReplicaEffect::ProposalOutcome {
            disposition,
            committed_index,
            ..
        } => Some((*disposition, *committed_index)),
        _ => None,
    })
}

struct Pair {
    leader: ReplicaState,
    follower: ReplicaState,
}

impl Pair {
    fn elected(follower_log_entries: usize) -> Self {
        let (leader, follower) = super::tests::elect_node_a();
        Self {
            leader: bounded(leader, SMALL_LOG_ENTRIES),
            follower: bounded(follower, follower_log_entries),
        }
    }

    /// Commit one request through the follower; the leader snapshots whenever its threshold is
    /// reached, the follower only when `follower_snapshots` is set. Returns the commit outcome.
    fn commit(&mut self, label: &str, follower_snapshots: bool) -> (ProposalDisposition, Option<u64>) {
        let proposal = propose(&self.leader, label);
        let replicated = apply(&self.follower, ReplicaEvent::Message {
            envelope: super::tests::sent_envelope_to(&proposal, super::tests::NODE_B),
        });
        let committed = apply(&proposal.next, ReplicaEvent::Message {
            envelope: super::tests::sent_envelope_to(&replicated, super::tests::NODE_A),
        });
        let outcome = committed
            .effects
            .iter()
            .find_map(|effect| match effect {
                ReplicaEffect::ProposalOutcome {
                    disposition,
                    committed_index,
                    ..
                } => Some((*disposition, *committed_index)),
                _ => None,
            })
            .expect("commit outcome");
        self.follower = replicated.next;
        self.leader = snapshot_if_due(committed.next);
        self.sync_follower(follower_snapshots);
        outcome
    }

    /// Carry the leader's commit index to the follower with a heartbeat.
    fn sync_follower(&mut self, follower_snapshots: bool) {
        let heartbeat = apply(&self.leader, ReplicaEvent::HeartbeatTimeout);
        let applied = apply(&self.follower, ReplicaEvent::Message {
            envelope: super::tests::sent_envelope_to(&heartbeat, super::tests::NODE_B),
        });
        self.leader = heartbeat.next;
        self.follower = if follower_snapshots {
            snapshot_if_due(applied.next)
        } else {
            applied.next
        };
    }
}

// r[verify molten.consensus.snapshot_threshold]
#[test]
fn threshold_snapshots_let_more_writes_than_the_log_bound_commit() {
    let mut pair = Pair::elected(SMALL_LOG_ENTRIES);
    for write in 1..=WRITES_BEYOND_CAPACITY {
        let outcome = pair.commit(&format!("bounded-write-{write}"), true);
        assert_eq!(outcome, (ProposalDisposition::Committed, Some(write)));
        assert!(pair.leader.log.len() < SMALL_LOG_ENTRIES);
        assert!(pair.follower.log.len() < SMALL_LOG_ENTRIES);
    }
    assert_eq!(pair.leader.commit_index, WRITES_BEYOND_CAPACITY);
    assert_eq!(pair.follower.last_applied, WRITES_BEYOND_CAPACITY);
}

// r[verify molten.consensus.snapshot_threshold]
#[test]
fn without_snapshots_a_full_log_denies_new_proposals() {
    let (leader, follower) = super::tests::elect_node_a();
    let mut leader = bounded(leader, SMALL_LOG_ENTRIES);
    let mut follower = bounded(follower, SMALL_LOG_ENTRIES);
    for write in 0..SMALL_LOG_ENTRIES {
        let proposal = propose(&leader, &format!("unsnapshotted-{write}"));
        let replicated = apply(&follower, ReplicaEvent::Message {
            envelope: super::tests::sent_envelope_to(&proposal, super::tests::NODE_B),
        });
        leader = apply(&proposal.next, ReplicaEvent::Message {
            envelope: super::tests::sent_envelope_to(&replicated, super::tests::NODE_A),
        })
        .next;
        follower = replicated.next;
    }
    assert!(is_snapshot_due(&leader));
    let denied = propose(&leader, "beyond-the-log-bound");
    assert_eq!(proposal_outcome(&denied), Some((ProposalDisposition::Denied, None)));
}

// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[test]
fn a_follower_at_its_log_bound_acknowledges_what_fits_and_compacts_before_accepting_more() {
    // The follower has not snapshotted yet, so committed entries fill its log to the bound.
    let mut pair = Pair::elected(SMALL_LOG_ENTRIES);
    for write in 1..=SMALL_LOG_ENTRIES {
        pair.commit(&format!("follower-bound-write-{write}"), false);
    }
    assert_eq!(pair.follower.log.len(), SMALL_LOG_ENTRIES);
    let prev_log_index = pair.follower.log.last().expect("full follower log").index;

    let proposal = propose(&pair.leader, "beyond-the-follower-bound");
    let full = apply(&pair.follower, ReplicaEvent::Message {
        envelope: super::tests::sent_envelope_to(&proposal, super::tests::NODE_B),
    });
    assert_eq!(full.next.log.len(), SMALL_LOG_ENTRIES, "nothing beyond the bound is stored");
    let response = super::tests::sent_envelope_to(&full, super::tests::NODE_A);
    assert!(matches!(response.message, Message::AppendResponse {
        success: true,
        match_index,
        ..
    } if match_index == prev_log_index));

    // The hosting service compacts after the step; the leader resends the entry from the
    // acknowledged index on its next heartbeat and the follower now stores it.
    let follower = snapshot_if_due(full.next);
    assert!(follower.snapshot.is_some());
    let leader = apply(&proposal.next, ReplicaEvent::Message { envelope: response }).next;
    let heartbeat = apply(&leader, ReplicaEvent::HeartbeatTimeout);
    let accepted = apply(&follower, ReplicaEvent::Message {
        envelope: super::tests::sent_envelope_to(&heartbeat, super::tests::NODE_B),
    });
    let stored = accepted.next.log.last().expect("resent entry").index;
    assert_eq!(stored, prev_log_index + 1);
    assert!(
        matches!(super::tests::sent_envelope_to(&accepted, super::tests::NODE_A).message, Message::AppendResponse {
        success: true,
        match_index,
        ..
    } if match_index == stored)
    );
}

// r[verify molten.consensus.bounded_request_dedup]
#[test]
fn a_retry_within_the_window_returns_the_prior_commit_without_appending() {
    let mut pair = Pair::elected(SMALL_LOG_ENTRIES);
    let (_, original) = pair.commit("windowed-request", true);
    pair.commit("later-write", true);
    let retry = propose(&pair.leader, "windowed-request");
    assert_eq!(proposal_outcome(&retry), Some((ProposalDisposition::Committed, original)));
    assert_eq!(retry.next.log, pair.leader.log);
    assert!(!retry.effects.iter().any(|effect| matches!(effect, ReplicaEffect::PersistEntries { .. })));
}

// r[verify molten.consensus.bounded_request_dedup]
#[test]
fn a_retry_older_than_the_window_is_a_new_request() {
    let mut pair = Pair::elected(SMALL_LOG_ENTRIES);
    let (_, original) = pair.commit("expired-request", true);
    for write in 0..SMALL_WINDOW {
        pair.commit(&format!("window-filler-{write}"), true);
    }
    assert!(!pair.leader.completed_requests.contains_key(&super::tests::test_ref("expired-request")));
    let (disposition, retried) = pair.commit("expired-request", true);
    assert_eq!(disposition, ProposalDisposition::Committed);
    assert!(retried > original, "the retry is appended and committed again at a new index");
}

// r[verify molten.consensus.bounded_request_dedup]
#[test]
fn replicas_at_one_applied_index_hold_identical_windows_whatever_their_snapshot_timing() {
    // The leader snapshots at every threshold; the follower, with room for every entry, never does.
    let mut pair = Pair::elected(MAX_REPLICA_LOG_ENTRIES);
    for write in 1..=WRITES_BEYOND_CAPACITY {
        pair.commit(&format!("timing-write-{write}"), false);
        assert_eq!(pair.leader.last_applied, pair.follower.last_applied);
        assert_eq!(pair.leader.completed_requests, pair.follower.completed_requests);
        assert!(pair.leader.completed_requests.len() as u64 <= SMALL_WINDOW);
    }
    assert!(pair.leader.snapshot.is_some());
    assert!(pair.follower.snapshot.is_none());
    assert!(pair.follower.log.len() as u64 > SMALL_WINDOW, "the follower still holds expired entries in its log");
}

/// The leader's InstallSnapshot to a fresh third replica after several compacted writes.
fn install_for_fresh_replica() -> (ReplicaState, ReplicaMessageEnvelope) {
    let mut pair = Pair::elected(SMALL_LOG_ENTRIES);
    for write in 1..=WRITES_BEYOND_CAPACITY {
        pair.commit(&format!("install-write-{write}"), true);
    }
    let heartbeat = apply(&pair.leader, ReplicaEvent::HeartbeatTimeout);
    let envelope = super::tests::sent_envelope_to(&heartbeat, super::tests::NODE_C);
    assert!(matches!(envelope.message, Message::InstallSnapshot { .. }));
    let fresh =
        bounded(super::tests::started_state(&super::tests::active_group(), super::tests::NODE_C), SMALL_LOG_ENTRIES);
    (fresh, envelope)
}

// r[verify molten.consensus.bounded_request_dedup]
#[test]
fn snapshot_install_restores_the_bounded_window() {
    let (fresh, envelope) = install_for_fresh_replica();
    let Message::InstallSnapshot { snapshot, .. } = &envelope.message else {
        unreachable!("checked above");
    };
    let installed = apply(&fresh, ReplicaEvent::Message {
        envelope: envelope.clone(),
    });
    assert_eq!(installed.next.last_applied, snapshot.last_included_index);
    assert_eq!(installed.next.completed_requests, snapshot.completed_requests);
    assert!(!snapshot.completed_requests.is_empty());
    assert!(snapshot.completed_requests.len() as u64 <= SMALL_WINDOW);
}

// r[verify molten.consensus.bounded_request_dedup]
#[test]
fn snapshots_with_refs_outside_their_window_are_rejected() {
    let (fresh, envelope) = install_for_fresh_replica();
    let Message::InstallSnapshot { snapshot, .. } = &envelope.message else {
        unreachable!("checked above");
    };
    let boundary = snapshot.last_included_index;
    for (label, index) in [
        ("above-boundary", boundary + 1),
        ("below-window", boundary - SMALL_WINDOW),
    ] {
        let mut tampered = envelope.clone();
        let Message::InstallSnapshot { snapshot, .. } = &mut tampered.message else {
            unreachable!("checked above");
        };
        snapshot.completed_requests.insert(super::tests::test_ref(label), index);
        snapshot.snapshot_ref = snapshot_ref(snapshot).expect("re-bound tampered snapshot");
        assert!(
            apply_replica_event(&fresh, ReplicaEvent::Message { envelope: tampered }).is_err(),
            "{label}: a snapshot with a ref outside its duplicate window must be rejected"
        );
    }
}
