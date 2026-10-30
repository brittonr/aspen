use super::*;

const TRAILING_SENTINEL_BYTE: u8 = 0xff;
const NODE_B: &str = "node-b";

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn vote_and_append_envelopes_roundtrip_exactly() {
    let group = super::tests::active_group();
    let node_a = super::tests::started_state(&group, super::tests::NODE_A);
    let election = apply_replica_event(&node_a, ReplicaEvent::ElectionTimeout {
        timer_ref: node_a.active_election_timer_ref.clone(),
    })
    .expect("election transition");
    let vote = super::tests::sent_envelope_to(&election, NODE_B);
    assert_roundtrip(&vote);

    let (leader, follower) = super::tests::elect_node_a();
    let proposal = apply_replica_event(&leader, ReplicaEvent::Propose {
        request_ref: super::tests::test_ref("canonical-proposal-request"),
        command_ref: super::tests::test_ref("canonical-proposal-command"),
        command_schema_ref: super::tests::test_ref("canonical-proposal-schema"),
    })
    .expect("proposal transition");
    let append = super::tests::sent_envelope_to(&proposal, NODE_B);
    assert_roundtrip(&append);

    let read = apply_replica_event(&leader, ReplicaEvent::Read {
        request_ref: super::tests::test_ref("canonical-read-request"),
        mode: crate::fabric_consistency::ReadMode::Linearizable,
    })
    .expect("read transition");
    let probe = super::tests::sent_envelope_to(&read, NODE_B);
    assert_roundtrip(&probe);
    let acknowledgement =
        apply_replica_event(&follower, ReplicaEvent::Message { envelope: probe }).expect("read acknowledgement");
    assert_roundtrip(&super::tests::sent_envelope_to(&acknowledgement, super::tests::NODE_A));

    let committed = super::tests::committed_leader();
    let snapshot = apply_replica_event(&committed, ReplicaEvent::CreateSnapshot {
        application_state_ref: super::tests::test_ref("canonical-snapshot-state"),
    })
    .expect("canonical snapshot");
    let heartbeat = apply_replica_event(&snapshot.next, ReplicaEvent::HeartbeatTimeout).expect("snapshot heartbeat");
    let install = super::tests::sent_envelope_to(&heartbeat, super::tests::NODE_C);
    assert_roundtrip(&install);
    let installed =
        apply_replica_event(&super::tests::started_state(&group, super::tests::NODE_C), ReplicaEvent::Message {
            envelope: install,
        })
        .expect("canonical snapshot install");
    assert_roundtrip(&super::tests::sent_envelope_to(&installed, super::tests::NODE_A));
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn decoder_rejects_trailing_bytes_and_sender_substitution() {
    let group = super::tests::active_group();
    let node_a = super::tests::started_state(&group, super::tests::NODE_A);
    let election = apply_replica_event(&node_a, ReplicaEvent::ElectionTimeout {
        timer_ref: node_a.active_election_timer_ref.clone(),
    })
    .expect("negative election transition");
    let envelope = super::tests::sent_envelope_to(&election, NODE_B);
    let mut bytes = replica_message(&envelope).expect("canonical envelope").bytes;
    bytes.push(TRAILING_SENTINEL_BYTE);
    assert!(parse_replica_message(&bytes).is_err());

    let mut substituted = envelope;
    substituted.from = "node-c".to_string();
    let error = replica_message(&substituted).expect_err("substituted sender must deny");
    assert!(error.to_string().contains("embedded sender"));
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn encoder_rejects_over_bound_append_batches() {
    let group = super::tests::active_group();
    let state = super::tests::started_state(&group, super::tests::NODE_A);
    let entry_count = MAX_REPLICA_MESSAGE_ENTRIES + 1;
    let entries = (0..entry_count)
        .map(|offset| {
            let index = u64::try_from(offset + 1).expect("bounded test index");
            ReplicatedEntry {
                index,
                term: INITIAL_LOG_INDEX,
                request_ref: super::tests::test_ref(&format!("over-bound-request-{offset}")),
                command_ref: super::tests::test_ref(&format!("over-bound-command-{offset}")),
                command_schema_ref: super::tests::test_ref("over-bound-schema"),
            }
        })
        .collect();
    let envelope = ReplicaMessageEnvelope {
        group_binding_ref: state.profile.group_binding_ref,
        service_generation: state.profile.service_generation,
        from: super::tests::NODE_A.to_string(),
        to: NODE_B.to_string(),
        message: Message::AppendEntries {
            term: INITIAL_LOG_INDEX,
            leader_id: super::tests::NODE_A.to_string(),
            prev_log_index: INITIAL_COMMIT_INDEX,
            prev_log_term: INITIAL_COMMIT_INDEX,
            entries,
            leader_commit: INITIAL_COMMIT_INDEX,
            config_epoch: state.membership.config_epoch,
            fencing_epoch: state.profile.fencing_epoch,
        },
    };
    let error = replica_message(&envelope).expect_err("over-bound append must deny");
    assert!(error.to_string().contains("message bound"));
}

fn assert_roundtrip(envelope: &ReplicaMessageEnvelope) {
    let first = replica_message(envelope).expect("first canonical envelope");
    let second = replica_message(envelope).expect("second canonical envelope");
    let parsed = parse_replica_message(&first.bytes).expect("strict canonical parse");
    assert_eq!(first, second);
    assert_eq!(&parsed, envelope);
    assert_eq!(first.envelope_ref, crate::preserves_rail::content_ref_from_bytes(&first.bytes));
}

#[test]
fn recovery_queries_and_paged_reports_roundtrip_including_term_zero() {
    let mut envelope = recovery_report(0, 0);
    assert_roundtrip(&envelope);
    envelope.message = Message::RecoveryQuery {
        term: 0,
        requester_id: envelope.from.clone(),
        round_ref: super::tests::test_ref("recovery-round"),
        from_index: 1,
        is_barrier: false,
        config_epoch: 1,
        fencing_epoch: 1,
    };
    assert_roundtrip(&envelope);
    if let Message::RecoveryQuery { term, is_barrier, .. } = &mut envelope.message {
        *term = 1;
        *is_barrier = true;
    }
    assert_roundtrip(&envelope);
    assert_roundtrip(&recovery_report(1, 128));
    let mut tail = recovery_report(1, 129);
    if let Message::RecoveryReport {
        from_index, entries, ..
    } = &mut tail.message
    {
        *from_index = 129;
        entries.clear();
        entries.push(recovery_entry(129));
    }
    assert_roundtrip(&tail);
}

#[test]
fn recovery_snapshot_reports_roundtrip_and_reject_inconsistent_boundaries() {
    let committed = super::tests::committed_leader();
    let transition = apply_replica_event(&committed, ReplicaEvent::CreateSnapshot {
        application_state_ref: super::tests::test_ref("recovery-wire-snapshot"),
    })
    .expect("snapshot transition");
    let state = transition.next;
    let snapshot = state.snapshot.expect("created snapshot");
    let mut compacted = recovery_report(state.current_term, 0);
    compacted.group_binding_ref = snapshot.group_binding_ref.clone();
    if let Message::RecoveryReport {
        last_log_index,
        commit_index,
        snapshot: report_snapshot,
        config_epoch,
        fencing_epoch,
        ..
    } = &mut compacted.message
    {
        *last_log_index = snapshot.last_included_index;
        *commit_index = snapshot.last_included_index;
        *config_epoch = snapshot.config_epoch;
        *fencing_epoch = snapshot.fencing_epoch;
        *report_snapshot = Some(Box::new(snapshot));
    }
    assert_roundtrip(&compacted);

    for kind in 0..5 {
        let mut malformed = compacted.clone();
        if let Message::RecoveryReport {
            term,
            snapshot: Some(snapshot),
            ..
        } = &mut malformed.message
        {
            match kind {
                0 => snapshot.config_epoch += 1,
                1 => snapshot.fencing_epoch += 1,
                2 => snapshot.group_binding_ref = super::tests::test_ref("other-recovery-group"),
                3 => snapshot.membership_ref = "invalid-membership-ref".to_string(),
                _ => snapshot.last_included_term = *term + 1,
            }
            snapshot.snapshot_ref = snapshot_ref(snapshot).expect("updated snapshot identity");
        }
        assert!(replica_message(&malformed).is_err(), "invalid recovery snapshot must deny");
        assert!(parse_replica_message(&unchecked_recovery_report_bytes(&malformed)).is_err());
    }
}

#[test]
fn recovery_rejects_malformed_pages_on_encode_and_decode() {
    let valid = recovery_report(1, 2);
    const MUTATED_PAGE_CASES: usize = 10;
    let mut cases = Vec::with_capacity(MUTATED_PAGE_CASES + 1);
    for kind in 0..MUTATED_PAGE_CASES {
        let mut malformed = valid.clone();
        if let Message::RecoveryReport {
            term,
            peer_id,
            round_ref,
            observation_ref,
            from_index,
            last_log_index,
            commit_index,
            entries,
            ..
        } = &mut malformed.message
        {
            match kind {
                0 => entries[1].index = 3,
                1 => *commit_index = 3,
                2 => *observation_ref = "invalid-ref".to_string(),
                3 => *round_ref = "invalid-ref".to_string(),
                4 => *peer_id = "node-c".to_string(),
                5 => *from_index = 0,
                6 => *term = 0,
                7 => {
                    entries.pop();
                }
                8 => *last_log_index = MAX_REPLICA_LOG_ENTRIES as u64 + 1,
                _ => entries[0].command_ref = "invalid-ref".to_string(),
            }
        }
        cases.push(malformed);
    }
    let mut oversized = recovery_report(1, 129);
    if let Message::RecoveryReport { entries, .. } = &mut oversized.message {
        entries.push(recovery_entry(129));
    }
    cases.push(oversized);
    for malformed in cases {
        assert!(replica_message(&malformed).is_err(), "must reject {:?}", malformed.message);
        let bytes = unchecked_recovery_report_bytes(&malformed);
        assert!(parse_replica_message(&bytes).is_err(), "decoder must reject {:?}", malformed.message);
    }
    let mut normal = valid;
    normal.message = Message::VoteResponse {
        term: 0,
        voter_id: normal.from.clone(),
        granted: false,
        config_epoch: 1,
        fencing_epoch: 1,
    };
    assert!(replica_message(&normal).is_err(), "term zero remains forbidden for normal Raft");
}

#[test]
fn recovery_observation_binds_durable_corpus_not_transient_state() {
    let mut state = super::tests::committed_leader();
    let original = canonical::recovery_observation_ref(&state).expect("observation");
    state.active_election_timer_ref = super::tests::test_ref("different-recovery-timer");
    state.leader_id = None;
    state.pending_reads.clear();
    assert_eq!(original, canonical::recovery_observation_ref(&state).expect("same durable corpus"));
    let mut changed = state.clone();
    changed.current_term += 1;
    assert_ne!(original, canonical::recovery_observation_ref(&changed).expect("term observation"));
    changed = state.clone();
    changed.commit_index = 0;
    assert_ne!(original, canonical::recovery_observation_ref(&changed).expect("commit observation"));
    changed = state.clone();
    let before_entry = canonical::entry_identity_ref(&changed.log[0]).expect("entry identity");
    changed.log[0].command_ref = super::tests::test_ref("different-recovery-command");
    assert_ne!(before_entry, canonical::entry_identity_ref(&changed.log[0]).expect("changed entry identity"));
    assert_ne!(original, canonical::recovery_observation_ref(&changed).expect("log observation"));
    let compacted = apply_replica_event(&super::tests::committed_leader(), ReplicaEvent::CreateSnapshot {
        application_state_ref: super::tests::test_ref("observation-snapshot"),
    })
    .expect("snapshot transition")
    .next;
    let compacted_ref = canonical::recovery_observation_ref(&compacted).expect("snapshot observation");
    let mut without_snapshot = compacted;
    without_snapshot.snapshot = None;
    assert_ne!(compacted_ref, canonical::recovery_observation_ref(&without_snapshot).expect("without snapshot"));
}

include!("helpers.rs");
