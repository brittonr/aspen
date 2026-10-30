fn recovery_entry(index: u64) -> ReplicatedEntry {
    ReplicatedEntry {
        index,
        term: 1,
        request_ref: super::tests::test_ref(&format!("recovery-request-{index}")),
        command_ref: super::tests::test_ref(&format!("recovery-command-{index}")),
        command_schema_ref: super::tests::test_ref("recovery-schema"),
    }
}

fn recovery_report(term: u64, last: u64) -> ReplicaMessageEnvelope {
    let group = super::tests::active_group();
    let state = super::tests::started_state(&group, super::tests::NODE_A);
    ReplicaMessageEnvelope {
        group_binding_ref: state.profile.group_binding_ref,
        service_generation: state.profile.service_generation,
        from: super::tests::NODE_A.to_string(),
        to: NODE_B.to_string(),
        message: Message::RecoveryReport {
            term,
            peer_id: super::tests::NODE_A.to_string(),
            round_ref: super::tests::test_ref("recovery-round"),
            observation_ref: super::tests::test_ref("recovery-observation"),
            from_index: 1,
            last_log_index: last,
            commit_index: last,
            entries: (1..=last.min(MAX_REPLICA_MESSAGE_ENTRIES as u64)).map(recovery_entry).collect(),
            snapshot: None,
            config_epoch: state.membership.config_epoch,
            fencing_epoch: state.profile.fencing_epoch,
        },
    }
}
/// Deliberately bypasses the encoder validator to exercise untrusted wire input.
fn unchecked_recovery_report_bytes(envelope: &ReplicaMessageEnvelope) -> Vec<u8> {
    let Message::RecoveryReport {
        term, peer_id, round_ref, observation_ref, from_index, last_log_index,
        commit_index, entries, snapshot, config_epoch, fencing_epoch,
    } = &envelope.message else { panic!("report fixture") };
    let message = crate::preserves_rail::record("recovery-report", vec![
        crate::preserves_rail::u64_value(*term), crate::preserves_rail::string(peer_id), crate::preserves_rail::string(round_ref), crate::preserves_rail::string(observation_ref),
        crate::preserves_rail::u64_value(*from_index), crate::preserves_rail::u64_value(*last_log_index), crate::preserves_rail::u64_value(*commit_index),
        crate::preserves_rail::sequence(entries.iter().map(|entry| crate::preserves_rail::record("raft-replicated-entry-v1", vec![
            crate::preserves_rail::string(RAFT_REPLICATED_ENTRY_SCHEMA), crate::preserves_rail::u64_value(entry.index), crate::preserves_rail::u64_value(entry.term),
            crate::preserves_rail::string(&entry.request_ref), crate::preserves_rail::string(&entry.command_ref), crate::preserves_rail::string(&entry.command_schema_ref),
        ])).collect()),
        snapshot.as_deref().map_or_else(
            || crate::preserves_rail::record("none", Vec::new()),
            |snapshot| crate::preserves_rail::record("some", vec![durability::snapshot_value(snapshot)]),
        ),
        crate::preserves_rail::u64_value(*config_epoch), crate::preserves_rail::u64_value(*fencing_epoch),
    ]);
    crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record("raft-message-envelope-v1", vec![
        crate::preserves_rail::string(RAFT_MESSAGE_ENVELOPE_SCHEMA), crate::preserves_rail::string(&envelope.group_binding_ref),
        crate::preserves_rail::u64_value(envelope.service_generation), crate::preserves_rail::string(&envelope.from), crate::preserves_rail::string(&envelope.to), message,
    ])).expect("unchecked canonical bytes")
}

#[test]
fn recovery_query_decoder_requires_boolean_barrier_flag_and_exact_arity() {
    let envelope = recovery_report(0, 0);
    for malformed_flag in [Some(crate::preserves_rail::u64_value(1)), Some(crate::preserves_rail::string("true")), None] {
        let mut fields = vec![
            crate::preserves_rail::u64_value(0), crate::preserves_rail::string(&envelope.from),
            crate::preserves_rail::string(super::tests::test_ref("recovery-round")), crate::preserves_rail::u64_value(1),
        ];
        if let Some(flag) = malformed_flag {
            fields.push(flag);
        }
        fields.extend([crate::preserves_rail::u64_value(1), crate::preserves_rail::u64_value(1)]);
        let bytes = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record("raft-message-envelope-v1", vec![
            crate::preserves_rail::string(RAFT_MESSAGE_ENVELOPE_SCHEMA), crate::preserves_rail::string(&envelope.group_binding_ref),
            crate::preserves_rail::u64_value(envelope.service_generation), crate::preserves_rail::string(&envelope.from), crate::preserves_rail::string(&envelope.to),
            crate::preserves_rail::record("recovery-query", fields),
        ])).expect("malformed query canonical bytes");
        assert!(parse_replica_message(&bytes).is_err(), "barrier flag must be present and boolean");
    }
}

#[test]
fn recovery_barrier_rejects_zero_term_on_encode_and_decode() {
    let mut envelope = recovery_report(0, 0);
    let round_ref = super::tests::test_ref("zero-term-barrier-round");
    envelope.message = Message::RecoveryQuery {
        term: 0,
        requester_id: envelope.from.clone(),
        round_ref: round_ref.clone(),
        from_index: 1,
        is_barrier: true,
        config_epoch: 1,
        fencing_epoch: 1,
    };
    assert!(replica_message(&envelope).is_err());
    let bytes = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record("raft-message-envelope-v1", vec![
        crate::preserves_rail::string(RAFT_MESSAGE_ENVELOPE_SCHEMA), crate::preserves_rail::string(&envelope.group_binding_ref),
        crate::preserves_rail::u64_value(envelope.service_generation), crate::preserves_rail::string(&envelope.from), crate::preserves_rail::string(&envelope.to),
        crate::preserves_rail::record("recovery-query", vec![
            crate::preserves_rail::u64_value(0), crate::preserves_rail::string(&envelope.from), crate::preserves_rail::string(&round_ref), crate::preserves_rail::u64_value(1),
            crate::preserves_rail::bool_value(true), crate::preserves_rail::u64_value(1), crate::preserves_rail::u64_value(1),
        ]),
    ])).expect("unchecked zero-term barrier bytes");
    assert!(parse_replica_message(&bytes).is_err());
}