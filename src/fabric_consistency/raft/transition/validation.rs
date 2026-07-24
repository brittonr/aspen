use std::collections::BTreeSet;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;

pub(super) fn validate_incoming_entries(
    state: &ReplicaState,
    prev_log_index: u64,
    leader_term: u64,
    entries: &[ReplicatedEntry],
) -> Result<()> {
    if entries.len() > state.profile.max_message_entries {
        return Err(MoltenError::invalid_harness("Raft append entries exceed the admitted message bound"));
    }
    let mut expected_index = prev_log_index
        .checked_add(NEXT_LOG_INDEX_STEP)
        .ok_or_else(|| MoltenError::invalid_harness("Raft append index overflow"))?;
    let mut request_refs = BTreeSet::new();
    for entry in entries {
        validate_incoming_entry(state, entry, expected_index, leader_term, &mut request_refs)?;
        expected_index = expected_index
            .checked_add(NEXT_LOG_INDEX_STEP)
            .ok_or_else(|| MoltenError::invalid_harness("Raft append index overflow"))?;
    }
    Ok(())
}

pub(super) fn validate_message_envelope(state: &ReplicaState, envelope: &ReplicaMessageEnvelope) -> Result<()> {
    validate_content_ref(&envelope.group_binding_ref, "Raft message group binding ref")?;
    if envelope.group_binding_ref != state.profile.group_binding_ref {
        return Err(MoltenError::invalid_harness("Raft message uses a substituted group binding"));
    }
    if envelope.service_generation != state.profile.service_generation {
        return Err(MoltenError::invalid_harness("Raft message uses a stale service generation"));
    }
    if envelope.to != state.node_id || envelope.from == state.node_id {
        return Err(MoltenError::invalid_harness("Raft message endpoint identity is invalid"));
    }
    if !state.membership.voters.contains(&envelope.from) || !state.membership.voters.contains(&envelope.to) {
        return Err(MoltenError::invalid_harness("Raft message endpoint is outside admitted membership"));
    }
    if envelope.message.term() == 0 {
        return Err(MoltenError::invalid_harness("Raft protocol message cannot use term zero"));
    }
    if envelope.message.config_epoch() != state.membership.config_epoch
        || envelope.message.fencing_epoch() != state.profile.fencing_epoch
    {
        return Err(MoltenError::invalid_harness("Raft message uses a stale configuration or fencing epoch"));
    }
    validate_message_sender(envelope)
}

pub(super) fn validate_replica_state(state: &ReplicaState) -> Result<()> {
    if state.membership.voters.len() != STATIC_VOTER_COUNT || !state.membership.voters.contains(&state.node_id) {
        return Err(MoltenError::invalid_harness("Raft state has invalid static membership"));
    }
    if state.log.len() > state.profile.max_log_entries {
        return Err(MoltenError::invalid_harness("Raft state exceeds its admitted log bound"));
    }
    validate_log(state)?;
    let last = support::last_log_index(state);
    if state.last_applied > state.commit_index || state.commit_index > last {
        return Err(MoltenError::invalid_harness("Raft applied or committed index exceeds the local log"));
    }
    validate_role(state)?;
    validate_election_timer(state)?;
    if state.voted_for.as_ref().is_some_and(|voter| !state.membership.voters.contains(voter)) {
        return Err(MoltenError::invalid_harness("Raft vote target is outside admitted membership"));
    }
    Ok(())
}

fn validate_election_timer(state: &ReplicaState) -> Result<()> {
    if state.election_timer_sequence == 0 {
        return Err(MoltenError::invalid_harness("Raft election timer sequence must be positive"));
    }
    validate_content_ref(&state.active_election_timer_ref, "Raft active election timer ref")?;
    let expected = election_timer_ref(
        &state.profile.group_binding_ref,
        &state.node_id,
        state.profile.service_generation,
        state.current_term,
        state.election_timer_sequence,
    )?;
    if state.active_election_timer_ref != expected {
        return Err(MoltenError::invalid_harness("Raft active election timer ref does not match state"));
    }
    Ok(())
}

pub(super) fn validate_transition(transition: &ReplicaTransition) -> Result<()> {
    validate_replica_state(&transition.next)?;
    if transition.effects.len() > transition.next.profile.max_effects_per_step {
        return Err(MoltenError::invalid_harness("Raft transition exceeds its admitted effect bound"));
    }
    Ok(())
}

pub(super) fn ensure_running(state: &ReplicaState) -> Result<()> {
    if state.lifecycle != ReplicaLifecycle::Running {
        return Err(MoltenError::invalid_harness("Raft event requires a running replica"));
    }
    Ok(())
}

pub(super) fn validate_content_ref(value: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid {label}: {error}")))
}

fn validate_incoming_entry<'a>(
    state: &ReplicaState,
    entry: &'a ReplicatedEntry,
    expected_index: u64,
    leader_term: u64,
    request_refs: &mut BTreeSet<&'a str>,
) -> Result<()> {
    if entry.index != expected_index || entry.term == 0 || entry.term > leader_term {
        return Err(MoltenError::invalid_harness("Raft append entries are not contiguous or use an invalid term"));
    }
    validate_content_ref(&entry.request_ref, "Raft log request ref")?;
    validate_content_ref(&entry.command_ref, "Raft log command ref")?;
    validate_content_ref(&entry.command_schema_ref, "Raft log command schema ref")?;
    if !request_refs.insert(entry.request_ref.as_str()) {
        return Err(MoltenError::invalid_harness("Raft append entries contain a duplicate request ref"));
    }
    if state
        .log
        .iter()
        .any(|existing| existing.request_ref == entry.request_ref && existing.index != entry.index)
    {
        return Err(MoltenError::invalid_harness("Raft append reuses a request ref at a different index"));
    }
    Ok(())
}

fn validate_message_sender(envelope: &ReplicaMessageEnvelope) -> Result<()> {
    let embedded_sender = match &envelope.message {
        RaftMessage::RequestVote { candidate_id, .. } => candidate_id,
        RaftMessage::VoteResponse { voter_id, .. } => voter_id,
        RaftMessage::AppendEntries { leader_id, .. } => leader_id,
        RaftMessage::AppendResponse { follower_id, .. } => follower_id,
    };
    if embedded_sender != &envelope.from {
        return Err(MoltenError::invalid_harness("Raft message sender does not match its canonical envelope"));
    }
    Ok(())
}

fn validate_log(state: &ReplicaState) -> Result<()> {
    let mut expected = match &state.snapshot {
        Some(snapshot) => snapshot
            .last_included_index
            .checked_add(NEXT_LOG_INDEX_STEP)
            .ok_or_else(|| MoltenError::invalid_harness("Raft snapshot log index overflow"))?,
        None => INITIAL_LOG_INDEX,
    };
    let mut request_refs = BTreeSet::new();
    for entry in &state.log {
        validate_state_entry(entry, expected, &mut request_refs)?;
        expected = expected
            .checked_add(NEXT_LOG_INDEX_STEP)
            .ok_or_else(|| MoltenError::invalid_harness("Raft log index overflow"))?;
    }
    Ok(())
}

fn validate_state_entry<'a>(
    entry: &'a ReplicatedEntry,
    expected_index: u64,
    request_refs: &mut BTreeSet<&'a str>,
) -> Result<()> {
    if entry.index != expected_index || entry.term == 0 {
        return Err(MoltenError::invalid_harness("Raft log indices or terms are invalid"));
    }
    validate_content_ref(&entry.request_ref, "Raft state request ref")?;
    validate_content_ref(&entry.command_ref, "Raft state command ref")?;
    validate_content_ref(&entry.command_schema_ref, "Raft state command schema ref")?;
    if !request_refs.insert(entry.request_ref.as_str()) {
        return Err(MoltenError::invalid_harness("Raft log contains a duplicate request ref"));
    }
    Ok(())
}

fn validate_role(state: &ReplicaState) -> Result<()> {
    if state.role == ReplicaRole::Leader && state.leader_id.as_deref() != Some(state.node_id.as_str()) {
        return Err(MoltenError::invalid_harness("Raft leader state lacks self leader identity"));
    }
    if state.role != ReplicaRole::Leader && state.leader_id.as_deref() == Some(state.node_id.as_str()) {
        return Err(MoltenError::invalid_harness("non-leader Raft state claims self leadership"));
    }
    if state.role != ReplicaRole::Follower && state.current_term == 0 {
        return Err(MoltenError::invalid_harness("candidate or leader cannot occupy term zero"));
    }
    Ok(())
}
