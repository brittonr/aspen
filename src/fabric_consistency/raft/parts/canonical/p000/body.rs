use preserves::ValueImpl;

use super::*;

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
    pub value: preserves::IOValue,
    pub bytes: Vec<u8>,
}

// r[impl molten.fabric_consistency.live_raft]
pub fn canonical_replica_message(envelope: &ReplicaMessageEnvelope) -> crate::error::Result<CanonicalReplicaMessage> {
    validate_envelope_shape(envelope)?;
    let value = crate::preserves_rail::record("raft-message-envelope-v1", vec![
        crate::preserves_rail::string(RAFT_MESSAGE_ENVELOPE_SCHEMA),
        crate::preserves_rail::string(&envelope.group_binding_ref),
        crate::preserves_rail::u64_value(envelope.service_generation),
        crate::preserves_rail::string(&envelope.from),
        crate::preserves_rail::string(&envelope.to),
        message_value(&envelope.message)?,
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
pub fn parse_canonical_replica_message(bytes: &[u8]) -> crate::error::Result<ReplicaMessageEnvelope> {
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let fields = required_record(&decoded.value, "raft-message-envelope-v1", ENVELOPE_ARITY)?;
    let schema = required_string(&fields[0], "Raft envelope schema")?;
    if schema != RAFT_MESSAGE_ENVELOPE_SCHEMA {
        return Err(crate::error::MoltenError::invalid_harness("unsupported canonical Raft envelope schema"));
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

fn message_value(message: &RaftMessage) -> crate::error::Result<preserves::IOValue> {
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

fn vote_message_value(message: &RaftMessage) -> crate::error::Result<preserves::IOValue> {
    Ok(match message {
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
        _ => return Err(crate::error::MoltenError::invalid_harness("vote encoding admitted a non-vote message")),
    })
}

fn append_message_value(message: &RaftMessage) -> crate::error::Result<preserves::IOValue> {
    Ok(match message {
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
        _ => return Err(crate::error::MoltenError::invalid_harness("append encoding admitted a non-append message")),
    })
}

fn read_message_value(message: &RaftMessage) -> crate::error::Result<preserves::IOValue> {
    Ok(match message {
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
        _ => return Err(crate::error::MoltenError::invalid_harness("read encoding admitted a non-read message")),
    })
}

fn snapshot_message_value(message: &RaftMessage) -> crate::error::Result<preserves::IOValue> {
    Ok(match message {
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
        _ => {
            return Err(crate::error::MoltenError::invalid_harness(
                "snapshot encoding admitted a non-snapshot message",
            ));
        }
    })
}

fn entry_value(entry: &ReplicatedEntry) -> preserves::IOValue {
    crate::preserves_rail::record("raft-replicated-entry-v1", vec![
        crate::preserves_rail::string(RAFT_REPLICATED_ENTRY_SCHEMA),
        crate::preserves_rail::u64_value(entry.index),
        crate::preserves_rail::u64_value(entry.term),
        crate::preserves_rail::string(&entry.request_ref),
        crate::preserves_rail::string(&entry.command_ref),
        crate::preserves_rail::string(&entry.command_schema_ref),
    ])
}

fn parse_message(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<RaftMessage> {
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
    parse_response_or_snapshot_message(value)
}
