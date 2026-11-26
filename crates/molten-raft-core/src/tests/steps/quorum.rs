//! Leader read index: current-term majorities, stale leaders, and out-of-order responses.

use super::*;

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn live_completes_linearizable_reads_only_after_a_current_term_majority() {
    let (leader, follower) = elect_node_a();
    let request_ref = test_ref("linearizable-read");
    let pending = apply_replica_event(&leader, ReplicaEvent::Read {
        request_ref: request_ref.clone(),
        mode: ReadMode::Linearizable,
    })
    .expect("linearizable read probe");
    assert!(pending.next.pending_reads.contains_key(&request_ref));
    assert!(!pending.effects.iter().any(|effect| matches!(effect, ReplicaEffect::ReadOutcome { .. })));

    let probe = sent_envelope_to(&pending, NODE_B);
    let acknowledged = apply_replica_event(&follower, ReplicaEvent::Message { envelope: probe })
        .expect("follower read acknowledgement");
    let acknowledgement = sent_envelope_to(&acknowledged, NODE_A);
    let mut unrelated = acknowledgement.clone();
    match &mut unrelated.message {
        Message::ReadAcknowledgement { request_ref, .. } => *request_ref = test_ref("unrelated-read"),
        other => panic!("expected read acknowledgement, got {other:?}"),
    }
    let ignored = apply_replica_event(&pending.next, ReplicaEvent::Message { envelope: unrelated })
        .expect("unrelated acknowledgement ignored");
    assert!(ignored.next.pending_reads.contains_key(&request_ref));
    assert!(ignored.effects.is_empty());

    let completed = apply_replica_event(&pending.next, ReplicaEvent::Message {
        envelope: acknowledgement,
    })
    .expect("majority read outcome");
    assert!(completed.next.pending_reads.is_empty());
    assert!(completed.effects.iter().any(|effect| matches!(effect, ReplicaEffect::ReadOutcome {
        request_ref: completed_ref,
        disposition: ReadDisposition::Current,
        ..
    } if completed_ref == &request_ref)));

    let follower_read = apply_replica_event(&acknowledged.next, ReplicaEvent::Read {
        request_ref: test_ref("follower-linearizable-read"),
        mode: ReadMode::Linearizable,
    })
    .expect("follower read retry");
    assert!(follower_read.effects.iter().any(|effect| matches!(effect, ReplicaEffect::ReadOutcome {
        disposition: ReadDisposition::Retryable,
        ..
    })));
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn live_ignores_out_of_order_append_failure_after_success() {
    let leader = committed_leader();
    assert_eq!(leader.next_index.get(NODE_B), Some(&NEXT_INDEX_AFTER_FIRST_ENTRY));
    let stale_failure = ReplicaMessageEnvelope {
        group_binding_ref: leader.profile.group_binding_ref.clone(),
        service_generation: leader.profile.service_generation,
        from: NODE_B.to_string(),
        to: NODE_A.to_string(),
        message: Message::AppendResponse {
            term: leader.current_term,
            follower_id: NODE_B.to_string(),
            success: false,
            request_prev_log_index: INITIAL_COMMIT_INDEX,
            match_index: INITIAL_COMMIT_INDEX,
            conflict_index: INITIAL_LOG_INDEX,
            config_epoch: leader.membership.config_epoch,
            fencing_epoch: leader.profile.fencing_epoch,
        },
    };
    let after = apply_replica_event(&leader, ReplicaEvent::Message {
        envelope: stale_failure,
    })
    .expect("stale append failure ignored");
    assert_eq!(after.next.next_index.get(NODE_B), Some(&NEXT_INDEX_AFTER_FIRST_ENTRY));
    assert!(after.effects.is_empty());
}

// r[verify molten.consensus.linearizable_reads]
#[test]
fn live_does_not_complete_stale_read_after_leader_change() {
    let (initial_leader, follower_b) = elect_node_a();
    let proposal = apply_replica_event(&initial_leader, ReplicaEvent::Propose {
        request_ref: test_ref("stale-read-request"),
        command_ref: test_ref("stale-read-command"),
        command_schema_ref: test_ref("stale-read-schema"),
    })
    .expect("initial proposal");
    let append = sent_envelope_to(&proposal, NODE_B);
    let replicated =
        apply_replica_event(&follower_b, ReplicaEvent::Message { envelope: append }).expect("follower replication");
    let response = sent_envelope_to(&replicated, NODE_A);

    let b_timeout = apply_replica_event(&replicated.next, ReplicaEvent::ElectionTimeout {
        timer_ref: replicated.next.active_election_timer_ref.clone(),
    })
    .expect("challenger timeout");

    let committed = apply_replica_event(&proposal.next, ReplicaEvent::Message { envelope: response })
        .expect("initial leader commit");
    assert_eq!(committed.next.commit_index, INITIAL_LOG_INDEX);

    let vote_request = sent_envelope_to(&b_timeout, NODE_A);
    let voted = apply_replica_event(&committed.next, ReplicaEvent::Message { envelope: vote_request })
        .expect("initial leader votes for challenger");

    let vote_response = sent_envelope_to(&voted, NODE_B);
    let new_leader = apply_replica_event(&b_timeout.next, ReplicaEvent::Message {
        envelope: vote_response,
    })
    .expect("challenger leadership");
    assert_eq!(new_leader.next.role, ReplicaRole::Leader);
    assert_eq!(new_leader.next.commit_index, INITIAL_COMMIT_INDEX);

    let stale_read_ref = test_ref("stale-read");
    let read = apply_replica_event(&new_leader.next, ReplicaEvent::Read {
        request_ref: stale_read_ref.clone(),
        mode: ReadMode::Linearizable,
    })
    .expect("stale read handling");
    assert!(read.effects.iter().any(|effect| matches!(effect, ReplicaEffect::ReadOutcome {
        request_ref: read_ref,
        disposition: ReadDisposition::Retryable,
        ..
    } if read_ref == &stale_read_ref)));
    assert!(!read.next.pending_reads.contains_key(&stale_read_ref));

    // Once a current-term entry commits, the read index covers the write the old leader acknowledged.
    let current_term_write = apply_replica_event(&read.next, ReplicaEvent::Propose {
        request_ref: test_ref("current-term-request"),
        command_ref: test_ref("current-term-command"),
        command_schema_ref: test_ref("current-term-schema"),
    })
    .expect("new leader proposal");
    let append = sent_envelope_to(&current_term_write, NODE_A);
    let replicated_a =
        apply_replica_event(&voted.next, ReplicaEvent::Message { envelope: append }).expect("old leader replication");
    let response = sent_envelope_to(&replicated_a, NODE_B);
    let caught_up = apply_replica_event(&current_term_write.next, ReplicaEvent::Message { envelope: response })
        .expect("current-term commit");
    let current_read_ref = test_ref("current-read");
    let current_read = apply_replica_event(&caught_up.next, ReplicaEvent::Read {
        request_ref: current_read_ref.clone(),
        mode: ReadMode::Linearizable,
    })
    .expect("current read handling");
    let pending = current_read.next.pending_reads.get(&current_read_ref).expect("read-index read begins");
    assert!(pending.required_index >= committed.next.commit_index);
}

// r[verify molten.consensus.linearizable_reads]
#[test]
fn live_new_leader_whose_commit_index_reaches_its_log_serves_read_index_reads() {
    let (initial_leader, follower_b) = elect_node_a();
    let proposal = apply_replica_event(&initial_leader, ReplicaEvent::Propose {
        request_ref: test_ref("caught-up-request"),
        command_ref: test_ref("caught-up-command"),
        command_schema_ref: test_ref("caught-up-schema"),
    })
    .expect("initial proposal");
    let append = sent_envelope_to(&proposal, NODE_B);
    let replicated =
        apply_replica_event(&follower_b, ReplicaEvent::Message { envelope: append }).expect("follower replication");
    let response = sent_envelope_to(&replicated, NODE_A);
    let committed = apply_replica_event(&proposal.next, ReplicaEvent::Message { envelope: response })
        .expect("initial leader commit");
    let heartbeat = apply_replica_event(&committed.next, ReplicaEvent::HeartbeatTimeout).expect("commit heartbeat");
    let informed = apply_replica_event(&replicated.next, ReplicaEvent::Message {
        envelope: sent_envelope_to(&heartbeat, NODE_B),
    })
    .expect("follower learns the commit index");
    assert_eq!(informed.next.commit_index, INITIAL_LOG_INDEX);

    let b_timeout = apply_replica_event(&informed.next, ReplicaEvent::ElectionTimeout {
        timer_ref: informed.next.active_election_timer_ref.clone(),
    })
    .expect("challenger timeout");
    let voted = apply_replica_event(&heartbeat.next, ReplicaEvent::Message {
        envelope: sent_envelope_to(&b_timeout, NODE_A),
    })
    .expect("initial leader votes for challenger");
    let new_leader = apply_replica_event(&b_timeout.next, ReplicaEvent::Message {
        envelope: sent_envelope_to(&voted, NODE_B),
    })
    .expect("challenger leadership");
    assert_eq!(new_leader.next.role, ReplicaRole::Leader);
    // No current-term entry has committed, but every entry the new leader holds is known committed.
    assert_eq!(new_leader.next.commit_index, INITIAL_LOG_INDEX);

    let read_ref = test_ref("caught-up-read");
    let read = apply_replica_event(&new_leader.next, ReplicaEvent::Read {
        request_ref: read_ref.clone(),
        mode: ReadMode::Linearizable,
    })
    .expect("caught-up read handling");
    assert_eq!(
        read.next.pending_reads.get(&read_ref).map(|pending| pending.required_index),
        Some(INITIAL_LOG_INDEX)
    );
    assert!(!read.effects.iter().any(|effect| matches!(effect, ReplicaEffect::ReadOutcome { .. })));
}
