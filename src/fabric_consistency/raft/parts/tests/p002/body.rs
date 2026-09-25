
// r[verify molten.fabric_consistency.live_raft]
#[test]
fn live_raft_completes_linearizable_reads_only_after_a_current_term_majority() {
    let (leader, follower) = elect_node_a();
    let request_ref = test_ref("linearizable-read");
    let pending = apply_replica_event(&leader, ReplicaEvent::Read {
        request_ref: request_ref.clone(),
        mode: crate::fabric_consistency::ConsistencyReadMode::Linearizable,
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
        RaftMessage::ReadAcknowledgement { request_ref, .. } => *request_ref = test_ref("unrelated-read"),
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
        mode: crate::fabric_consistency::ConsistencyReadMode::Linearizable,
    })
    .expect("follower read retry");
    assert!(follower_read.effects.iter().any(|effect| matches!(effect, ReplicaEffect::ReadOutcome {
        disposition: ReadDisposition::Retryable,
        ..
    })));
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn live_raft_ignores_out_of_order_append_failure_after_success() {
    let leader = committed_leader();
    assert_eq!(leader.next_index.get(NODE_B), Some(&NEXT_INDEX_AFTER_FIRST_ENTRY));
    let stale_failure = ReplicaMessageEnvelope {
        group_binding_ref: leader.profile.group_binding_ref.clone(),
        service_generation: leader.profile.service_generation,
        from: NODE_B.to_string(),
        to: NODE_A.to_string(),
        message: RaftMessage::AppendResponse {
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
