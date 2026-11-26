//! Live-cluster liveness regressions: queued timer races, ReadProbe against a lagging follower,
//! partition heal with a retried request ref, and effect-bounded commit of a full message batch.

use super::*;

/// The reliability service's admitted per-step effect bound (`reliability/service/recipe/node.rs`).
const RELIABILITY_EFFECTS_PER_STEP: usize = 128;
/// A full `AppendEntries` batch commits in two steps: the effect budget leaves room for this many
/// proposal outcomes beside the commit record, the application batch and both intent fences.
const FIRST_COMMIT_CHUNK: u64 = 124;

fn apply(state: &ReplicaState, event: ReplicaEvent) -> ReplicaTransition {
    apply_replica_event(state, event).expect("admitted transition")
}

fn deliver(state: &ReplicaState, from: &ReplicaTransition) -> ReplicaTransition {
    apply(state, ReplicaEvent::Message {
        envelope: super::tests::sent_envelope_to(from, &state.node_id),
    })
}

fn propose(state: &ReplicaState, label: &str) -> ReplicaTransition {
    apply(state, ReplicaEvent::Propose {
        request_ref: super::tests::test_ref(label),
        command_ref: super::tests::test_ref(&format!("{label}-command")),
        command_schema_ref: super::tests::test_ref("liveness-schema"),
    })
}

fn envelope(state: &ReplicaState, from: &str, message: Message) -> ReplicaEvent {
    ReplicaEvent::Message {
        envelope: ReplicaMessageEnvelope {
            group_binding_ref: state.profile.group_binding_ref.clone(),
            service_generation: state.profile.service_generation,
            from: from.to_string(),
            to: state.node_id.clone(),
            message,
        },
    }
}

fn assert_ignored(state: &ReplicaState, event: ReplicaEvent, expected: StaleTimerEvent) {
    assert_eq!(stale_timer_event(state, &event), Some(expected));
    let ignored = apply(state, event);
    assert_eq!(&ignored.next, state, "a stale timer event keeps the replica state");
    assert!(ignored.effects.is_empty(), "a stale timer event plans no protocol effects");
}

// r[verify molten.consensus.raft_timer_lifecycle_safety]
#[test]
fn a_new_leader_ignores_the_election_timeout_it_armed_as_candidate() {
    let (leader, _follower) = super::tests::elect_node_a();
    // The candidate's election timer keeps its token after the win; once its delay elapses, the
    // delivery (or an already-queued copy) reaches the leader.
    let pending = ReplicaEvent::ElectionTimeout {
        timer_ref: leader.active_election_timer_ref.clone(),
    };
    assert_ignored(&leader, pending, StaleTimerEvent::ElectionTimeoutAtLeader);

    let heartbeat = apply(&leader, ReplicaEvent::HeartbeatTimeout);
    assert_eq!(heartbeat.next.role, ReplicaRole::Leader);
    assert!(matches!(
        super::tests::sent_envelope_to(&heartbeat, super::tests::NODE_B).message,
        Message::AppendEntries { .. }
    ));
    assert!(matches!(heartbeat.effects.last(), Some(ReplicaEffect::ArmHeartbeatTimer)));
    let proposal = propose(&heartbeat.next, "leader-after-election-timeout");
    assert_eq!(proposal.next.log.len(), 1, "the leader keeps serving client operations");
}

// r[verify molten.consensus.raft_timer_lifecycle_safety]
#[test]
fn a_leader_that_steps_down_ignores_its_pending_heartbeat_tick() {
    let (leader, _follower) = super::tests::elect_node_a();
    let higher_term = leader.current_term + 1;
    let stepped_down = apply(
        &leader,
        envelope(&leader, super::tests::NODE_C, Message::RequestVote {
            term: higher_term,
            candidate_id: super::tests::NODE_C.to_string(),
            last_log_index: 0,
            last_log_term: 0,
            config_epoch: leader.membership.config_epoch,
            fencing_epoch: leader.profile.fencing_epoch,
        }),
    );
    assert_eq!(stepped_down.next.role, ReplicaRole::Follower);
    assert_eq!(stepped_down.next.current_term, higher_term);
    assert!(stepped_down.effects.iter().any(|effect| matches!(effect, ReplicaEffect::ArmElectionTimer { .. })));

    assert_ignored(&stepped_down.next, ReplicaEvent::HeartbeatTimeout, StaleTimerEvent::HeartbeatAtNonLeader);
    assert_eq!(stale_timer_event(&leader, &ReplicaEvent::HeartbeatTimeout), None, "a leader's tick is current");
}

// r[verify molten.consensus.raft_timer_lifecycle_safety]
#[test]
fn a_timeout_queued_before_an_append_entries_reset_is_ignored() {
    let (leader, follower) = super::tests::elect_node_a();
    let queued = ReplicaEvent::ElectionTimeout {
        timer_ref: follower.active_election_timer_ref.clone(),
    };
    let heartbeat = apply(&leader, ReplicaEvent::HeartbeatTimeout);
    let reset = deliver(&follower, &heartbeat);
    assert_ne!(reset.next.active_election_timer_ref, follower.active_election_timer_ref);

    assert_ignored(&reset.next, queued, StaleTimerEvent::SupersededElectionTimer);
    let current = apply(&reset.next, ReplicaEvent::ElectionTimeout {
        timer_ref: reset.next.active_election_timer_ref.clone(),
    });
    assert_eq!(current.next.role, ReplicaRole::Candidate, "the reset timer still drives elections");
}

// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[test]
fn a_lagging_follower_acknowledges_a_read_probe_and_adopts_its_term() {
    // Node A committed index 1 with node B; node C has not received it.
    let leader = super::tests::committed_leader();
    let lagging = super::tests::started_state(&super::tests::active_group(), super::tests::NODE_C);
    let read_ref = super::tests::test_ref("lagging-follower-read");
    let read = apply(&leader, ReplicaEvent::Read {
        request_ref: read_ref.clone(),
        mode: crate::fabric_consistency::ReadMode::Linearizable,
    });
    let probe = super::tests::sent_envelope_to(&read, super::tests::NODE_C);
    assert!(matches!(probe.message, Message::ReadProbe { required_index: 1, .. }));
    assert_eq!(lagging.log.len(), 0);

    let acknowledged = apply(&lagging, ReplicaEvent::Message { envelope: probe });
    assert_eq!(acknowledged.next.current_term, leader.current_term);
    assert_eq!(acknowledged.next.leader_id.as_deref(), Some(super::tests::NODE_A));
    assert!(matches!(
        acknowledged.effects.first(),
        Some(ReplicaEffect::PersistHardState { term, .. }) if *term == leader.current_term
    ));
    let acknowledgement = super::tests::sent_envelope_to(&acknowledged, super::tests::NODE_A);
    assert!(
        matches!(&acknowledgement.message, Message::ReadAcknowledgement { request_ref, .. } if *request_ref == read_ref)
    );

    let served = apply(&read.next, ReplicaEvent::Message {
        envelope: acknowledgement,
    });
    assert!(served.effects.iter().any(|effect| matches!(effect, ReplicaEffect::ReadOutcome {
        disposition: ReadDisposition::Current,
        observed_index: 1,
        ..
    })));
}

// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[test]
fn a_read_probe_from_a_superseded_term_is_not_acknowledged() {
    let leader = super::tests::committed_leader();
    let read = apply(&leader, ReplicaEvent::Read {
        request_ref: super::tests::test_ref("superseded-term-read"),
        mode: crate::fabric_consistency::ReadMode::Linearizable,
    });
    let mut newer = super::tests::started_state(&super::tests::active_group(), super::tests::NODE_C);
    newer = apply(&newer, ReplicaEvent::ElectionTimeout {
        timer_ref: newer.active_election_timer_ref.clone(),
    })
    .next;
    newer = apply(&newer, ReplicaEvent::ElectionTimeout {
        timer_ref: newer.active_election_timer_ref.clone(),
    })
    .next;
    assert!(newer.current_term > leader.current_term);
    let ignored = apply(&newer, ReplicaEvent::Message {
        envelope: super::tests::sent_envelope_to(&read, super::tests::NODE_C),
    });
    assert!(!ignored.effects.iter().any(|effect| matches!(effect, ReplicaEffect::Send { .. })));
}

/// Node A led term 1 and appended two entries a partition kept from the others; node B then led
/// term 2 with node C and committed the client's retry of the second request at index 1.
struct HealedPartition {
    old_leader: ReplicaState,
    new_leader: ReplicaState,
    retried_ref: String,
}

fn healed_partition() -> HealedPartition {
    let retried_ref = super::tests::test_ref("partition-retried-request");
    let (old_leader, follower_b) = super::tests::elect_node_a();
    let old_leader = propose(&old_leader, "partition-isolated-request").next;
    let old_leader = propose(&old_leader, "partition-retried-request").next;
    assert_eq!(old_leader.log.iter().map(|entry| (entry.index, entry.term)).collect::<Vec<_>>(), vec![
        (1, 1),
        (2, 1)
    ]);
    assert_eq!(old_leader.log[1].request_ref, retried_ref);
    assert_eq!(old_leader.commit_index, 0);

    let node_c = super::tests::started_state(&super::tests::active_group(), super::tests::NODE_C);
    let campaign = apply(&follower_b, ReplicaEvent::ElectionTimeout {
        timer_ref: follower_b.active_election_timer_ref.clone(),
    });
    let vote = deliver(&node_c, &campaign);
    let new_leader = deliver(&campaign.next, &vote).next;
    assert_eq!(new_leader.role, ReplicaRole::Leader);
    assert_eq!(new_leader.current_term, 2);
    let retry = propose(&new_leader, "partition-retried-request");
    let replicated = deliver(&vote.next, &retry);
    let committed = deliver(&retry.next, &replicated).next;
    assert_eq!(committed.commit_index, 1);
    HealedPartition {
        old_leader,
        new_leader: committed,
        retried_ref,
    }
}

// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[test]
fn a_healed_old_leader_truncates_its_suffix_and_accepts_the_committed_retry() {
    let healed = healed_partition();
    let heartbeat = apply(&healed.new_leader, ReplicaEvent::HeartbeatTimeout);
    let append = super::tests::sent_envelope_to(&heartbeat, super::tests::NODE_A);
    assert!(matches!(&append.message, Message::AppendEntries { prev_log_index: 0, entries, .. } if entries.len() == 1));

    let reconciled = apply(&healed.old_leader, ReplicaEvent::Message { envelope: append });
    assert_eq!(reconciled.next.role, ReplicaRole::Follower);
    assert_eq!(reconciled.next.log.len(), 1);
    assert_eq!(reconciled.next.log[0].term, 2);
    assert_eq!(reconciled.next.log[0].request_ref, healed.retried_ref);
    assert_eq!(reconciled.next.commit_index, 1);
    assert!(reconciled.effects.iter().any(|effect| matches!(effect, ReplicaEffect::PersistEntries {
        truncate_from: Some(1),
        ..
    })));
    let response = super::tests::sent_envelope_to(&reconciled, super::tests::NODE_B);
    assert!(matches!(response.message, Message::AppendResponse {
        success: true,
        match_index: 1,
        ..
    }));
}

// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[test]
fn a_request_ref_repeated_against_the_retained_prefix_is_still_rejected() {
    let healed = healed_partition();
    let heartbeat = apply(&healed.new_leader, ReplicaEvent::HeartbeatTimeout);
    let reconciled = deliver(&healed.old_leader, &heartbeat).next;
    let retained = reconciled.log[0].clone();
    let duplicate = ReplicatedEntry {
        index: 2,
        term: retained.term,
        request_ref: retained.request_ref.clone(),
        command_ref: super::tests::test_ref("duplicate-command"),
        command_schema_ref: retained.command_schema_ref.clone(),
    };
    let error = apply_replica_event(
        &reconciled,
        envelope(&reconciled, super::tests::NODE_B, Message::AppendEntries {
            term: healed.new_leader.current_term,
            leader_id: super::tests::NODE_B.to_string(),
            prev_log_index: retained.index,
            prev_log_term: retained.term,
            entries: vec![duplicate],
            leader_commit: 1,
            config_epoch: reconciled.membership.config_epoch,
            fencing_epoch: reconciled.profile.fencing_epoch,
        }),
    )
    .expect_err("a retained request ref cannot be appended again within its window");
    assert!(error.to_string().contains("reuses a request ref"));
}

// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[tokio::test]
async fn a_full_message_batch_commits_in_effect_bounded_chunks() {
    let (mut leader, mut follower) = super::tests::elect_node_a();
    leader.profile.max_effects_per_step = RELIABILITY_EFFECTS_PER_STEP;
    follower.profile.max_effects_per_step = RELIABILITY_EFFECTS_PER_STEP;
    let mut last_proposal = None;
    for write in 0..MAX_REPLICA_MESSAGE_ENTRIES {
        let proposal = propose(&leader, &format!("batch-write-{write}"));
        leader = proposal.next.clone();
        last_proposal = Some(proposal);
    }
    let last_proposal = last_proposal.expect("batch proposals");
    let append = super::tests::sent_envelope_to(&last_proposal, super::tests::NODE_B);
    assert!(
        matches!(&append.message, Message::AppendEntries { entries, .. } if entries.len() == MAX_REPLICA_MESSAGE_ENTRIES)
    );
    let replicated = apply(&follower, ReplicaEvent::Message { envelope: append });
    let response = super::tests::sent_envelope_to(&replicated, super::tests::NODE_A);

    let mut ports = super::executor_tests::RecordingPorts::default();
    let first = execute_replica_event(
        &leader,
        ReplicaEvent::Message {
            envelope: response.clone(),
        },
        &mut ports,
    )
    .await;
    let ReplicaExecutionOutcome::Applied(first) = first else {
        panic!("the first chunk must fit the fenced effect bound: {first:?}");
    };
    assert_eq!(first.next.commit_index, FIRST_COMMIT_CHUNK);
    assert!(first.observations.len() <= RELIABILITY_EFFECTS_PER_STEP);

    // The next successful response (here a heartbeat acknowledgement) commits the remainder.
    let second = execute_replica_event(&first.next, ReplicaEvent::Message { envelope: response }, &mut ports).await;
    let ReplicaExecutionOutcome::Applied(second) = second else {
        panic!("the remainder must commit: {second:?}");
    };
    let full_batch = u64::try_from(MAX_REPLICA_MESSAGE_ENTRIES).expect("bounded message batch");
    assert_eq!(second.next.commit_index, full_batch);
    assert_eq!(second.next.last_applied, second.next.commit_index);
}
