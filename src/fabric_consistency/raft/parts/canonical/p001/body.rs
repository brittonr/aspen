
/// Parses the append-response, read, and snapshot Raft message variants.
fn parse_response_or_snapshot_message(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<Message> {
    if let Some(fields) = value.collect_simple_record("append-response", Some(APPEND_RESPONSE_ARITY)) {
        return Ok(Message::AppendResponse {
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
        return Ok(Message::ReadProbe {
            term: required_u64(&fields[0], "Raft read probe term")?,
            leader_id: required_string(&fields[1], "Raft read probe leader")?,
            request_ref: required_string(&fields[2], "Raft read probe request ref")?,
            required_index: required_u64(&fields[3], "Raft read probe required index")?,
            config_epoch: required_u64(&fields[4], "Raft read probe config epoch")?,
            fencing_epoch: required_u64(&fields[5], "Raft read probe fencing epoch")?,
        });
    }
    if let Some(fields) = value.collect_simple_record("read-acknowledgement", Some(READ_ACKNOWLEDGEMENT_ARITY)) {
        return Ok(Message::ReadAcknowledgement {
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
        return Ok(Message::InstallSnapshot {
            term: required_u64(&fields[0], "Raft install snapshot term")?,
            leader_id: required_string(&fields[1], "Raft install snapshot leader")?,
            snapshot: Box::new(super::recovery::parse_snapshot(&snapshot_bytes)?),
            config_epoch: required_u64(&fields[3], "Raft install snapshot config epoch")?,
            fencing_epoch: required_u64(&fields[4], "Raft install snapshot fencing epoch")?,
        });
    }
    if let Some(fields) = value.collect_simple_record("snapshot-response", Some(SNAPSHOT_RESPONSE_ARITY)) {
        return Ok(Message::SnapshotResponse {
            term: required_u64(&fields[0], "Raft snapshot response term")?,
            follower_id: required_string(&fields[1], "Raft snapshot response follower")?,
            snapshot_index: required_u64(&fields[2], "Raft snapshot response index")?,
            accepted: required_bool(&fields[3], "Raft snapshot response decision")?,
            config_epoch: required_u64(&fields[4], "Raft snapshot response config epoch")?,
            fencing_epoch: required_u64(&fields[5], "Raft snapshot response fencing epoch")?,
        });
    }
    Err(crate::error::Failure::invalid_harness("unsupported canonical Raft message variant"))
}

fn parse_append_entries(fields: &[preserves::Value<preserves::IOValue>]) -> crate::error::Result<Message> {
    let sequence = fields[4]
        .collect_sequence()
        .ok_or_else(|| crate::error::Failure::invalid_harness("Raft append entries must be a sequence"))?;
    if sequence.len() > MAX_REPLICA_MESSAGE_ENTRIES {
        return Err(crate::error::Failure::invalid_harness(
            "canonical Raft append entries exceed the message bound",
        ));
    }
    let entries = sequence.as_ref().as_slice().iter().map(parse_entry).collect::<crate::error::Result<Vec<_>>>()?;
    Ok(Message::AppendEntries {
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
        return Err(crate::error::Failure::invalid_harness("unsupported canonical Raft entry schema"));
    }
    Ok(ReplicatedEntry {
        index: required_u64(&fields[1], "Raft entry index")?,
        term: required_u64(&fields[2], "Raft entry term")?,
        request_ref: required_string(&fields[3], "Raft entry request ref")?,
        command_ref: required_string(&fields[4], "Raft entry command ref")?,
        command_schema_ref: required_string(&fields[5], "Raft entry command schema ref")?,
    })
}

pub(super) fn required_record(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
    arity: usize,
) -> crate::error::Result<Vec<preserves::Value<preserves::IOValue>>> {
    let fields = value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected canonical {label} record")))?;
    Ok(fields.iter().collect())
}

pub(super) fn required_string(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
) -> crate::error::Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected string for {label}")))
}

pub(super) fn required_u64(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| crate::error::Failure::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

pub(super) fn required_bool(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<bool> {
    value
        .as_boolean()
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected bool for {label}")))
}

