use preserves::IOValue;
use preserves::Value;
use preserves::ValueImpl;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;

pub const RAFT_MESSAGE_ENVELOPE_SCHEMA: &str = "molten.fabric-consistency.raft-message-envelope.v1";
pub const RAFT_REPLICATED_ENTRY_SCHEMA: &str = "molten.fabric-consistency.raft-replicated-entry.v1";

const ENVELOPE_ARITY: usize = 6;
const ENTRY_ARITY: usize = 6;
const REQUEST_VOTE_ARITY: usize = 6;
const VOTE_RESPONSE_ARITY: usize = 5;
const APPEND_ENTRIES_ARITY: usize = 8;
const APPEND_RESPONSE_ARITY: usize = 8;
const READ_PROBE_ARITY: usize = 6;
const READ_ACKNOWLEDGEMENT_ARITY: usize = 5;
const INSTALL_SNAPSHOT_ARITY: usize = 5;
const SNAPSHOT_RESPONSE_ARITY: usize = 6;
const MAX_WIRE_IDENTIFIER_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalReplicaMessage {
    pub envelope_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

// r[impl molten.fabric_consistency.live_raft]
pub fn canonical_replica_message(envelope: &ReplicaMessageEnvelope) -> Result<CanonicalReplicaMessage> {
    validate_envelope_shape(envelope)?;
    let value = crate::preserves_rail::record("raft-message-envelope-v1", vec![
        crate::preserves_rail::string(RAFT_MESSAGE_ENVELOPE_SCHEMA),
        crate::preserves_rail::string(&envelope.group_binding_ref),
        crate::preserves_rail::u64_value(envelope.service_generation),
        crate::preserves_rail::string(&envelope.from),
        crate::preserves_rail::string(&envelope.to),
        message_value(&envelope.message),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    let envelope_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
    Ok(CanonicalReplicaMessage {
        envelope_ref,
        value,
        bytes,
    })
}

// r[impl molten.fabric_consistency.live_raft]
pub fn parse_canonical_replica_message(bytes: &[u8]) -> Result<ReplicaMessageEnvelope> {
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let fields = required_record(&decoded.value, "raft-message-envelope-v1", ENVELOPE_ARITY)?;
    let schema = required_string(&fields[0], "Raft envelope schema")?;
    if schema != RAFT_MESSAGE_ENVELOPE_SCHEMA {
        return Err(MoltenError::invalid_harness("unsupported canonical Raft envelope schema"));
    }
    let envelope = ReplicaMessageEnvelope {
        group_binding_ref: required_string(&fields[1], "Raft group binding ref")?,
        service_generation: required_u64(&fields[2], "Raft service generation")?,
        from: required_string(&fields[3], "Raft message sender")?,
        to: required_string(&fields[4], "Raft message recipient")?,
        message: parse_message(&fields[5])?,
    };
    validate_envelope_shape(&envelope)?;
    Ok(envelope)
}

fn message_value(message: &RaftMessage) -> IOValue {
    match message {
        message @ (RaftMessage::RequestVote { .. } | RaftMessage::VoteResponse { .. }) => vote_message_value(message),
        message @ (RaftMessage::AppendEntries { .. } | RaftMessage::AppendResponse { .. }) => {
            append_message_value(message)
        }
        message @ (RaftMessage::ReadProbe { .. } | RaftMessage::ReadAcknowledgement { .. }) => {
            read_message_value(message)
        }
        message @ (RaftMessage::InstallSnapshot { .. } | RaftMessage::SnapshotResponse { .. }) => {
            snapshot_message_value(message)
        }
    }
}

fn vote_message_value(message: &RaftMessage) -> IOValue {
    match message {
        RaftMessage::RequestVote {
            term,
            candidate_id,
            last_log_index,
            last_log_term,
            config_epoch,
            fencing_epoch,
        } => crate::preserves_rail::record("request-vote", vec![
            crate::preserves_rail::u64_value(*term),
            crate::preserves_rail::string(candidate_id),
            crate::preserves_rail::u64_value(*last_log_index),
            crate::preserves_rail::u64_value(*last_log_term),
            crate::preserves_rail::u64_value(*config_epoch),
            crate::preserves_rail::u64_value(*fencing_epoch),
        ]),
        RaftMessage::VoteResponse {
            term,
            voter_id,
            granted,
            config_epoch,
            fencing_epoch,
        } => crate::preserves_rail::record("vote-response", vec![
            crate::preserves_rail::u64_value(*term),
            crate::preserves_rail::string(voter_id),
            crate::preserves_rail::bool_value(*granted),
            crate::preserves_rail::u64_value(*config_epoch),
            crate::preserves_rail::u64_value(*fencing_epoch),
        ]),
        _ => unreachable!("vote encoding admitted a non-vote message"),
    }
}

fn append_message_value(message: &RaftMessage) -> IOValue {
    match message {
        RaftMessage::AppendEntries {
            term,
            leader_id,
            prev_log_index,
            prev_log_term,
            entries,
            leader_commit,
            config_epoch,
            fencing_epoch,
        } => crate::preserves_rail::record("append-entries", vec![
            crate::preserves_rail::u64_value(*term),
            crate::preserves_rail::string(leader_id),
            crate::preserves_rail::u64_value(*prev_log_index),
            crate::preserves_rail::u64_value(*prev_log_term),
            crate::preserves_rail::sequence(entries.iter().map(entry_value).collect()),
            crate::preserves_rail::u64_value(*leader_commit),
            crate::preserves_rail::u64_value(*config_epoch),
            crate::preserves_rail::u64_value(*fencing_epoch),
        ]),
        RaftMessage::AppendResponse {
            term,
            follower_id,
            success,
            request_prev_log_index,
            match_index,
            conflict_index,
            config_epoch,
            fencing_epoch,
        } => crate::preserves_rail::record("append-response", vec![
            crate::preserves_rail::u64_value(*term),
            crate::preserves_rail::string(follower_id),
            crate::preserves_rail::bool_value(*success),
            crate::preserves_rail::u64_value(*request_prev_log_index),
            crate::preserves_rail::u64_value(*match_index),
            crate::preserves_rail::u64_value(*conflict_index),
            crate::preserves_rail::u64_value(*config_epoch),
            crate::preserves_rail::u64_value(*fencing_epoch),
        ]),
        _ => unreachable!("append encoding admitted a non-append message"),
    }
}

fn read_message_value(message: &RaftMessage) -> IOValue {
    match message {
        RaftMessage::ReadProbe {
            term,
            leader_id,
            request_ref,
            required_index,
            config_epoch,
            fencing_epoch,
        } => crate::preserves_rail::record("read-probe", vec![
            crate::preserves_rail::u64_value(*term),
            crate::preserves_rail::string(leader_id),
            crate::preserves_rail::string(request_ref),
            crate::preserves_rail::u64_value(*required_index),
            crate::preserves_rail::u64_value(*config_epoch),
            crate::preserves_rail::u64_value(*fencing_epoch),
        ]),
        RaftMessage::ReadAcknowledgement {
            term,
            follower_id,
            request_ref,
            config_epoch,
            fencing_epoch,
        } => crate::preserves_rail::record("read-acknowledgement", vec![
            crate::preserves_rail::u64_value(*term),
            crate::preserves_rail::string(follower_id),
            crate::preserves_rail::string(request_ref),
            crate::preserves_rail::u64_value(*config_epoch),
            crate::preserves_rail::u64_value(*fencing_epoch),
        ]),
        _ => unreachable!("read encoding admitted a non-read message"),
    }
}

fn snapshot_message_value(message: &RaftMessage) -> IOValue {
    match message {
        RaftMessage::InstallSnapshot {
            term,
            leader_id,
            snapshot,
            config_epoch,
            fencing_epoch,
        } => crate::preserves_rail::record("install-snapshot", vec![
            crate::preserves_rail::u64_value(*term),
            crate::preserves_rail::string(leader_id),
            super::durability::snapshot_value(snapshot),
            crate::preserves_rail::u64_value(*config_epoch),
            crate::preserves_rail::u64_value(*fencing_epoch),
        ]),
        RaftMessage::SnapshotResponse {
            term,
            follower_id,
            snapshot_index,
            accepted,
            config_epoch,
            fencing_epoch,
        } => crate::preserves_rail::record("snapshot-response", vec![
            crate::preserves_rail::u64_value(*term),
            crate::preserves_rail::string(follower_id),
            crate::preserves_rail::u64_value(*snapshot_index),
            crate::preserves_rail::bool_value(*accepted),
            crate::preserves_rail::u64_value(*config_epoch),
            crate::preserves_rail::u64_value(*fencing_epoch),
        ]),
        _ => unreachable!("snapshot encoding admitted a non-snapshot message"),
    }
}

fn entry_value(entry: &ReplicatedEntry) -> IOValue {
    crate::preserves_rail::record("raft-replicated-entry-v1", vec![
        crate::preserves_rail::string(RAFT_REPLICATED_ENTRY_SCHEMA),
        crate::preserves_rail::u64_value(entry.index),
        crate::preserves_rail::u64_value(entry.term),
        crate::preserves_rail::string(&entry.request_ref),
        crate::preserves_rail::string(&entry.command_ref),
        crate::preserves_rail::string(&entry.command_schema_ref),
    ])
}

fn parse_message(value: &Value<IOValue>) -> Result<RaftMessage> {
    if let Some(fields) = value.collect_simple_record("request-vote", Some(REQUEST_VOTE_ARITY)) {
        return Ok(RaftMessage::RequestVote {
            term: required_u64(&fields[0], "Raft vote term")?,
            candidate_id: required_string(&fields[1], "Raft candidate id")?,
            last_log_index: required_u64(&fields[2], "Raft candidate last log index")?,
            last_log_term: required_u64(&fields[3], "Raft candidate last log term")?,
            config_epoch: required_u64(&fields[4], "Raft vote config epoch")?,
            fencing_epoch: required_u64(&fields[5], "Raft vote fencing epoch")?,
        });
    }
    if let Some(fields) = value.collect_simple_record("vote-response", Some(VOTE_RESPONSE_ARITY)) {
        return Ok(RaftMessage::VoteResponse {
            term: required_u64(&fields[0], "Raft vote response term")?,
            voter_id: required_string(&fields[1], "Raft voter id")?,
            granted: required_bool(&fields[2], "Raft vote decision")?,
            config_epoch: required_u64(&fields[3], "Raft vote response config epoch")?,
            fencing_epoch: required_u64(&fields[4], "Raft vote response fencing epoch")?,
        });
    }
    if let Some(fields) = value.collect_simple_record("append-entries", Some(APPEND_ENTRIES_ARITY)) {
        let fields = fields.iter().collect::<Vec<_>>();
        return parse_append_entries(&fields);
    }
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
        let snapshot_value: &IOValue = (&fields[2]).into();
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
    Err(MoltenError::invalid_harness("unsupported canonical Raft message variant"))
}

fn parse_append_entries(fields: &[Value<IOValue>]) -> Result<RaftMessage> {
    let sequence = fields[4]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("Raft append entries must be a sequence"))?;
    if sequence.len() > MAX_REPLICA_MESSAGE_ENTRIES {
        return Err(MoltenError::invalid_harness("canonical Raft append entries exceed the message bound"));
    }
    let entries = sequence.as_ref().as_slice().iter().map(parse_entry).collect::<Result<Vec<_>>>()?;
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

pub(super) fn parse_entry(value: &Value<IOValue>) -> Result<ReplicatedEntry> {
    let fields = required_record(value, "raft-replicated-entry-v1", ENTRY_ARITY)?;
    if required_string(&fields[0], "Raft entry schema")? != RAFT_REPLICATED_ENTRY_SCHEMA {
        return Err(MoltenError::invalid_harness("unsupported canonical Raft entry schema"));
    }
    Ok(ReplicatedEntry {
        index: required_u64(&fields[1], "Raft entry index")?,
        term: required_u64(&fields[2], "Raft entry term")?,
        request_ref: required_string(&fields[3], "Raft entry request ref")?,
        command_ref: required_string(&fields[4], "Raft entry command ref")?,
        command_schema_ref: required_string(&fields[5], "Raft entry command schema ref")?,
    })
}

fn validate_envelope_shape(envelope: &ReplicaMessageEnvelope) -> Result<()> {
    crate::preserves_rail::validate_content_ref(&envelope.group_binding_ref)?;
    validate_identifier(&envelope.from, "Raft wire sender")?;
    validate_identifier(&envelope.to, "Raft wire recipient")?;
    if envelope.from == envelope.to {
        return Err(MoltenError::invalid_harness("Raft wire sender and recipient must differ"));
    }
    if envelope.service_generation == 0
        || envelope.message.term() == 0
        || envelope.message.config_epoch() == 0
        || envelope.message.fencing_epoch() == 0
    {
        return Err(MoltenError::invalid_harness("Raft wire generation, term, and epochs must be positive"));
    }
    validate_embedded_sender(envelope)?;
    validate_message_entries(&envelope.message)
}

fn validate_embedded_sender(envelope: &ReplicaMessageEnvelope) -> Result<()> {
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
        return Err(MoltenError::invalid_harness("Raft wire sender does not match the embedded sender"));
    }
    Ok(())
}

fn validate_message_entries(message: &RaftMessage) -> Result<()> {
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
        return Err(MoltenError::invalid_harness("Raft wire entries exceed the message bound"));
    }
    let mut expected = prev_log_index
        .checked_add(NEXT_LOG_INDEX_STEP)
        .ok_or_else(|| MoltenError::invalid_harness("Raft wire entry index overflow"))?;
    for entry in entries {
        if entry.index != expected || entry.term == 0 || entry.term > *term {
            return Err(MoltenError::invalid_harness("Raft wire entries are non-contiguous or use an invalid term"));
        }
        for reference in [&entry.request_ref, &entry.command_ref, &entry.command_schema_ref] {
            crate::preserves_rail::validate_content_ref(reference)?;
        }
        expected = expected
            .checked_add(NEXT_LOG_INDEX_STEP)
            .ok_or_else(|| MoltenError::invalid_harness("Raft wire entry index overflow"))?;
    }
    Ok(())
}

fn validate_wire_snapshot(snapshot: &ReplicaSnapshot) -> Result<()> {
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
        return Err(MoltenError::invalid_harness("Raft wire snapshot identity or boundary is invalid"));
    }
    for (request_ref, index) in &snapshot.completed_requests {
        crate::preserves_rail::validate_content_ref(request_ref)?;
        if *index == INITIAL_COMMIT_INDEX || *index > snapshot.last_included_index {
            return Err(MoltenError::invalid_harness("Raft wire snapshot request index is invalid"));
        }
    }
    Ok(())
}

pub(super) fn required_record(value: &Value<IOValue>, label: &str, arity: usize) -> Result<Vec<Value<IOValue>>> {
    let fields = value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected canonical {label} record")))?;
    Ok(fields.iter().collect())
}

pub(super) fn required_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {label}")))
}

pub(super) fn required_u64(value: &Value<IOValue>, label: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn required_bool(value: &Value<IOValue>, label: &str) -> Result<bool> {
    value.as_boolean().ok_or_else(|| MoltenError::invalid_harness(format!("expected bool for {label}")))
}

fn validate_identifier(value: &str, label: &str) -> Result<()> {
    if value.is_empty() || value.len() > MAX_WIRE_IDENTIFIER_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "{label} must be non-empty and at most {MAX_WIRE_IDENTIFIER_BYTES} bytes"
        )));
    }
    if !value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')) {
        return Err(MoltenError::invalid_harness(format!("{label} contains unsupported characters")));
    }
    Ok(())
}
