pub(crate) fn entry_identity_ref(entry: &ReplicatedEntry) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&entry_value(entry))
}

/// Identifies the stable recovery corpus, independently of retry and rejoin state.
pub(crate) fn recovery_observation_ref(state: &ReplicaState) -> crate::error::Result<String> {
    recovery_corpus_ref(state.current_term, state.commit_index, &state.log, state.snapshot.as_ref())
}

pub(crate) fn recovery_corpus_ref(
    term: u64,
    commit_index: u64,
    log: &[ReplicatedEntry],
    snapshot: Option<&ReplicaSnapshot>,
) -> crate::error::Result<String> {
    if log.len() > MAX_REPLICA_LOG_ENTRIES {
        return Err(crate::error::Failure::invalid_harness("recovery observation log exceeds the bound"));
    }
    let snapshot = snapshot.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |snapshot| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(&snapshot.snapshot_ref)]),
    );
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-recovery-observation-v1", vec![
        crate::preserves_rail::u64_value(term),
        crate::preserves_rail::u64_value(commit_index),
        snapshot,
        crate::preserves_rail::sequence(log.iter().map(entry_value).collect()),
    ]))
}

fn recovery_message_value(message: &Message) -> crate::error::Result<preserves::IOValue> {
    Ok(match message {
        Message::RecoveryQuery { term, requester_id, round_ref, from_index, is_barrier, config_epoch, fencing_epoch } => {
            crate::preserves_rail::record("recovery-query", vec![
                crate::preserves_rail::u64_value(*term),
                crate::preserves_rail::string(requester_id),
                crate::preserves_rail::string(round_ref),
                crate::preserves_rail::u64_value(*from_index),
                crate::preserves_rail::bool_value(*is_barrier),
                crate::preserves_rail::u64_value(*config_epoch),
                crate::preserves_rail::u64_value(*fencing_epoch),
            ])
        }
        Message::RecoveryReport {
            term, peer_id, round_ref, observation_ref, from_index, last_log_index,
            commit_index, entries, snapshot, config_epoch, fencing_epoch,
        } => crate::preserves_rail::record("recovery-report", vec![
            crate::preserves_rail::u64_value(*term),
            crate::preserves_rail::string(peer_id),
            crate::preserves_rail::string(round_ref),
            crate::preserves_rail::string(observation_ref),
            crate::preserves_rail::u64_value(*from_index),
            crate::preserves_rail::u64_value(*last_log_index),
            crate::preserves_rail::u64_value(*commit_index),
            crate::preserves_rail::sequence(entries.iter().map(entry_value).collect()),
            snapshot.as_deref().map_or_else(
                || crate::preserves_rail::record("none", Vec::new()),
                |snapshot| crate::preserves_rail::record("some", vec![super::durability::snapshot_value(snapshot)]),
            ),
            crate::preserves_rail::u64_value(*config_epoch),
            crate::preserves_rail::u64_value(*fencing_epoch),
        ]),
        _ => return Err(crate::error::Failure::invalid_harness("recovery encoding admitted a non-recovery message")),
    })
}

fn parse_recovery_message(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<Option<Message>> {
    if let Some(fields) = value.collect_simple_record("recovery-query", Some(7)) {
        return Ok(Some(Message::RecoveryQuery {
            term: required_u64(&fields[0], "Raft recovery query term")?,
            requester_id: required_string(&fields[1], "Raft recovery requester")?,
            round_ref: required_string(&fields[2], "Raft recovery round")?,
            from_index: required_u64(&fields[3], "Raft recovery start index")?,
            is_barrier: required_bool(&fields[4], "Raft recovery barrier flag")?,
            config_epoch: required_u64(&fields[5], "Raft recovery config epoch")?,
            fencing_epoch: required_u64(&fields[6], "Raft recovery fencing epoch")?,
        }));
    }
    if let Some(fields) = value.collect_simple_record("recovery-report", Some(11)) {
        let sequence = fields[7].collect_sequence().ok_or_else(|| {
            crate::error::Failure::invalid_harness("Raft recovery entries must be a sequence")
        })?;
        if sequence.len() > MAX_REPLICA_MESSAGE_ENTRIES {
            return Err(crate::error::Failure::invalid_harness("Raft recovery entries exceed the message bound"));
        }
        let entries = sequence.as_ref().as_slice().iter().map(parse_entry).collect::<crate::error::Result<Vec<_>>>()?;
        let snapshot = if fields[8].collect_simple_record("none", Some(0)).is_some() {
            None
        } else {
            let snapshot_fields = required_record(&fields[8], "some", 1)?;
            let value: &preserves::IOValue = (&snapshot_fields[0]).into();
            Some(Box::new(super::recovery::parse_snapshot(&crate::preserves_rail::canonical_bytes(value)?)?))
        };
        return Ok(Some(Message::RecoveryReport {
            term: required_u64(&fields[0], "Raft recovery report term")?,
            peer_id: required_string(&fields[1], "Raft recovery peer")?,
            round_ref: required_string(&fields[2], "Raft recovery round")?,
            observation_ref: required_string(&fields[3], "Raft recovery observation")?,
            from_index: required_u64(&fields[4], "Raft recovery start index")?,
            last_log_index: required_u64(&fields[5], "Raft recovery last index")?,
            commit_index: required_u64(&fields[6], "Raft recovery commit index")?,
            entries,
            snapshot,
            config_epoch: required_u64(&fields[9], "Raft recovery config epoch")?,
            fencing_epoch: required_u64(&fields[10], "Raft recovery fencing epoch")?,
        }));
    }
    Ok(None)
}

fn validate_recovery_envelope(envelope: &ReplicaMessageEnvelope) -> crate::error::Result<()> {
    let message = &envelope.message;
    let (round_ref, from_index) = match message {
        Message::RecoveryQuery { round_ref, from_index, .. }
        | Message::RecoveryReport { round_ref, from_index, .. } => (round_ref, *from_index),
        _ => return Ok(()),
    };
    crate::preserves_rail::validate_content_ref(round_ref)?;
    if from_index == 0 {
        return Err(crate::error::Failure::invalid_harness("Raft recovery start index must be positive"));
    }
    if matches!(message, Message::RecoveryQuery { term: 0, is_barrier: true, .. }) {
        return Err(crate::error::Failure::invalid_harness("Raft recovery barrier term must be positive"));
    }
    let Message::RecoveryReport {
        term, observation_ref, last_log_index, commit_index, entries, snapshot, config_epoch, fencing_epoch, ..
    } = message else {
        return Ok(());
    };
    crate::preserves_rail::validate_content_ref(observation_ref)?;
    if commit_index > last_log_index || entries.len() > MAX_REPLICA_MESSAGE_ENTRIES {
        return Err(crate::error::Failure::invalid_harness("Raft recovery commit or page bound is invalid"));
    }
    let boundary = if let Some(snapshot) = snapshot {
        validate_wire_snapshot(snapshot)?;
        if snapshot.group_binding_ref != envelope.group_binding_ref
            || snapshot.config_epoch != *config_epoch
            || snapshot.fencing_epoch != *fencing_epoch
            || snapshot.last_included_term > *term
            || snapshot.last_included_index > *commit_index
        {
            return Err(crate::error::Failure::invalid_harness("Raft recovery snapshot binding, epochs, or boundary is invalid"));
        }
        snapshot.last_included_index
    } else {
        0
    };
    if last_log_index.saturating_sub(boundary) > MAX_REPLICA_LOG_ENTRIES as u64 {
        return Err(crate::error::Failure::invalid_harness("Raft recovery log exceeds the bound"));
    }
    let start = from_index.max(boundary.checked_add(1).ok_or_else(|| {
        crate::error::Failure::invalid_harness("Raft recovery snapshot boundary overflow")
    })?);
    let remaining = if start > *last_log_index { 0 } else { last_log_index - start + 1 };
    let expected_len = usize::try_from(remaining)
        .map_err(|_| crate::error::Failure::invalid_harness("Raft recovery page span exceeds usize"))?
        .min(MAX_REPLICA_MESSAGE_ENTRIES);
    if entries.len() != expected_len {
        return Err(crate::error::Failure::invalid_harness("Raft recovery page is incomplete or exceeds the reported tail"));
    }
    for (offset, entry) in entries.iter().enumerate() {
        if entry.index != start + offset as u64 || entry.term == 0 || entry.term > *term {
            return Err(crate::error::Failure::invalid_harness("Raft recovery page is non-contiguous or uses an invalid term"));
        }
        for reference in [&entry.request_ref, &entry.command_ref, &entry.command_schema_ref] {
            crate::preserves_rail::validate_content_ref(reference)?;
        }
    }
    Ok(())
}