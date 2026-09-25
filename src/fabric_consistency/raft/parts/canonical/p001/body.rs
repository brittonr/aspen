
/// Parses the append-response, read, and snapshot Raft message variants.
fn parse_response_or_snapshot_message(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<RaftMessage> {
    if let Some(fields) = value.collect_simple_record("append-response", Some(APPEND_RESPONSE_ARITY)) {
        return Ok(RaftMessage::AppendResponse {
            term: required_u64(&fields[0], "Raft append response term")?,
            follower_id: required_string(&fields[1], "Raft append response follower")?,
            success: required_bool(&fields[2], "Raft append response decision")?,
            request_prev_log_index: required_u64(&fields[3], "Raft append response request prefix")?,
            match_index: required_u64(&fields[4], "Raft append response match index")?,
            conflict_index: required_u64(&fields[5], "Raft append response conflict index")?,
            config_epoch: required_u64(&fields[6], "Raft append response config epoch")?,
            fencing_epoch: required_u64(&fields[7], "Raft append response fencing epoch")?,
        });
    }
    if let Some(fields) = value.collect_simple_record("read-probe", Some(READ_PROBE_ARITY)) {
        return Ok(RaftMessage::ReadProbe {
            term: required_u64(&fields[0], "Raft read probe term")?,
            leader_id: required_string(&fields[1], "Raft read probe leader")?,
            request_ref: required_string(&fields[2], "Raft read probe request ref")?,
            required_index: required_u64(&fields[3], "Raft read probe required index")?,
            config_epoch: required_u64(&fields[4], "Raft read probe config epoch")?,
            fencing_epoch: required_u64(&fields[5], "Raft read probe fencing epoch")?,
        });
    }
    if let Some(fields) = value.collect_simple_record("read-acknowledgement", Some(READ_ACKNOWLEDGEMENT_ARITY)) {
        return Ok(RaftMessage::ReadAcknowledgement {
            term: required_u64(&fields[0], "Raft read acknowledgement term")?,
            follower_id: required_string(&fields[1], "Raft read acknowledgement follower")?,
            request_ref: required_string(&fields[2], "Raft read acknowledgement request ref")?,
            config_epoch: required_u64(&fields[3], "Raft read acknowledgement config epoch")?,
            fencing_epoch: required_u64(&fields[4], "Raft read acknowledgement fencing epoch")?,
        });
    }
    if let Some(fields) = value.collect_simple_record("install-snapshot", Some(INSTALL_SNAPSHOT_ARITY)) {
        let snapshot_value: &preserves::IOValue = (&fields[2]).into();
        let snapshot_bytes = crate::preserves_rail::canonical_bytes(snapshot_value)?;
        return Ok(RaftMessage::InstallSnapshot {
            term: required_u64(&fields[0], "Raft install snapshot term")?,
            leader_id: required_string(&fields[1], "Raft install snapshot leader")?,
            snapshot: Box::new(super::recovery::parse_snapshot(&snapshot_bytes)?),
            config_epoch: required_u64(&fields[3], "Raft install snapshot config epoch")?,
            fencing_epoch: required_u64(&fields[4], "Raft install snapshot fencing epoch")?,
        });
    }
    if let Some(fields) = value.collect_simple_record("snapshot-response", Some(SNAPSHOT_RESPONSE_ARITY)) {
        return Ok(RaftMessage::SnapshotResponse {
            term: required_u64(&fields[0], "Raft snapshot response term")?,
            follower_id: required_string(&fields[1], "Raft snapshot response follower")?,
            snapshot_index: required_u64(&fields[2], "Raft snapshot response index")?,
            accepted: required_bool(&fields[3], "Raft snapshot response decision")?,
            config_epoch: required_u64(&fields[4], "Raft snapshot response config epoch")?,
            fencing_epoch: required_u64(&fields[5], "Raft snapshot response fencing epoch")?,
        });
    }
    Err(crate::error::MoltenError::invalid_harness("unsupported canonical Raft message variant"))
}

fn parse_append_entries(fields: &[preserves::Value<preserves::IOValue>]) -> crate::error::Result<RaftMessage> {
    let sequence = fields[4]
        .collect_sequence()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("Raft append entries must be a sequence"))?;
    if sequence.len() > MAX_REPLICA_MESSAGE_ENTRIES {
        return Err(crate::error::MoltenError::invalid_harness(
            "canonical Raft append entries exceed the message bound",
        ));
    }
    let entries = sequence.as_ref().as_slice().iter().map(parse_entry).collect::<crate::error::Result<Vec<_>>>()?;
    Ok(RaftMessage::AppendEntries {
        term: required_u64(&fields[0], "Raft append term")?,
        leader_id: required_string(&fields[1], "Raft append leader")?,
        prev_log_index: required_u64(&fields[2], "Raft append previous index")?,
        prev_log_term: required_u64(&fields[3], "Raft append previous term")?,
        entries,
        leader_commit: required_u64(&fields[5], "Raft append leader commit")?,
        config_epoch: required_u64(&fields[6], "Raft append config epoch")?,
        fencing_epoch: required_u64(&fields[7], "Raft append fencing epoch")?,
    })
}

pub(super) fn parse_entry(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<ReplicatedEntry> {
    let fields = required_record(value, "raft-replicated-entry-v1", ENTRY_ARITY)?;
    if required_string(&fields[0], "Raft entry schema")? != RAFT_REPLICATED_ENTRY_SCHEMA {
        return Err(crate::error::MoltenError::invalid_harness("unsupported canonical Raft entry schema"));
    }
    Ok(ReplicatedEntry {
        index: required_u64(&fields[1], "Raft entry index")?,
        term: required_u64(&fields[2], "Raft entry term")?,
        request_ref: required_string(&fields[3], "Raft entry request ref")?,
        command_ref: required_string(&fields[4], "Raft entry command ref")?,
        command_schema_ref: required_string(&fields[5], "Raft entry command schema ref")?,
    })
}

fn validate_envelope_shape(envelope: &ReplicaMessageEnvelope) -> crate::error::Result<()> {
    crate::preserves_rail::validate_content_ref(&envelope.group_binding_ref)?;
    validate_identifier(&envelope.from, "Raft wire sender")?;
    validate_identifier(&envelope.to, "Raft wire recipient")?;
    if envelope.from == envelope.to {
        return Err(crate::error::MoltenError::invalid_harness("Raft wire sender and recipient must differ"));
    }
    if envelope.service_generation == 0
        || envelope.message.term() == 0
        || envelope.message.config_epoch() == 0
        || envelope.message.fencing_epoch() == 0
    {
        return Err(crate::error::MoltenError::invalid_harness(
            "Raft wire generation, term, and epochs must be positive",
        ));
    }
    validate_embedded_sender(envelope)?;
    validate_message_entries(&envelope.message)
}

fn validate_embedded_sender(envelope: &ReplicaMessageEnvelope) -> crate::error::Result<()> {
    let embedded = match &envelope.message {
        RaftMessage::RequestVote { candidate_id, .. } => candidate_id,
        RaftMessage::VoteResponse { voter_id, .. } => voter_id,
        RaftMessage::AppendEntries { leader_id, .. }
        | RaftMessage::ReadProbe { leader_id, .. }
        | RaftMessage::InstallSnapshot { leader_id, .. } => leader_id,
        RaftMessage::AppendResponse { follower_id, .. }
        | RaftMessage::ReadAcknowledgement { follower_id, .. }
        | RaftMessage::SnapshotResponse { follower_id, .. } => follower_id,
    };
    if embedded != &envelope.from {
        return Err(crate::error::MoltenError::invalid_harness("Raft wire sender does not match the embedded sender"));
    }
    Ok(())
}

fn validate_message_entries(message: &RaftMessage) -> crate::error::Result<()> {
    match message {
        RaftMessage::ReadProbe { request_ref, .. } | RaftMessage::ReadAcknowledgement { request_ref, .. } => {
            return crate::preserves_rail::validate_content_ref(request_ref);
        }
        RaftMessage::InstallSnapshot { snapshot, .. } => return validate_wire_snapshot(snapshot),
        _ => {}
    }
    let RaftMessage::AppendEntries {
        term,
        prev_log_index,
        entries,
        ..
    } = message
    else {
        return Ok(());
    };
    if entries.len() > MAX_REPLICA_MESSAGE_ENTRIES {
        return Err(crate::error::MoltenError::invalid_harness("Raft wire entries exceed the message bound"));
    }
    let mut expected = prev_log_index
        .checked_add(NEXT_LOG_INDEX_STEP)
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("Raft wire entry index overflow"))?;
    for entry in entries {
        if entry.index != expected || entry.term == 0 || entry.term > *term {
            return Err(crate::error::MoltenError::invalid_harness(
                "Raft wire entries are non-contiguous or use an invalid term",
            ));
        }
        for reference in [&entry.request_ref, &entry.command_ref, &entry.command_schema_ref] {
            crate::preserves_rail::validate_content_ref(reference)?;
        }
        expected = expected
            .checked_add(NEXT_LOG_INDEX_STEP)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("Raft wire entry index overflow"))?;
    }
    Ok(())
}

fn validate_wire_snapshot(snapshot: &ReplicaSnapshot) -> crate::error::Result<()> {
    for reference in [
        &snapshot.snapshot_ref,
        &snapshot.group_binding_ref,
        &snapshot.membership_ref,
        &snapshot.application_state_ref,
    ] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    if snapshot.snapshot_ref != snapshot_ref(snapshot)?
        || snapshot.last_included_index == INITIAL_COMMIT_INDEX
        || snapshot.last_included_term == INITIAL_TERM
        || snapshot.completed_requests.len() > MAX_REPLICA_LOG_ENTRIES
    {
        return Err(crate::error::MoltenError::invalid_harness("Raft wire snapshot identity or boundary is invalid"));
    }
    for (request_ref, index) in &snapshot.completed_requests {
        crate::preserves_rail::validate_content_ref(request_ref)?;
        if *index == INITIAL_COMMIT_INDEX || *index > snapshot.last_included_index {
            return Err(crate::error::MoltenError::invalid_harness("Raft wire snapshot request index is invalid"));
        }
    }
    Ok(())
}

pub(super) fn required_record(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
    arity: usize,
) -> crate::error::Result<Vec<preserves::Value<preserves::IOValue>>> {
    let fields = value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected canonical {label} record")))?;
    Ok(fields.iter().collect())
}

pub(super) fn required_string(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
) -> crate::error::Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected string for {label}")))
}

pub(super) fn required_u64(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| crate::error::MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

pub(super) fn required_bool(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<bool> {
    value
        .as_boolean()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected bool for {label}")))
}

fn validate_identifier(value: &str, label: &str) -> crate::error::Result<()> {
    if value.is_empty() || value.len() > MAX_WIRE_IDENTIFIER_BYTES {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "{label} must be non-empty and at most {MAX_WIRE_IDENTIFIER_BYTES} bytes"
        )));
    }
    if !value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')) {
        return Err(crate::error::MoltenError::invalid_harness(format!("{label} contains unsupported characters")));
    }
    Ok(())
}
