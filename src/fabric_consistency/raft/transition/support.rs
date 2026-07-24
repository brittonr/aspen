use super::*;
use crate::error::MoltenError;
use crate::error::Result;

pub(super) fn arm_election_timer(state: &mut ReplicaState) -> Result<ReplicaEffect> {
    let sequence = state
        .election_timer_sequence
        .checked_add(NEXT_ELECTION_TIMER_SEQUENCE_STEP)
        .ok_or_else(|| MoltenError::invalid_harness("Raft election timer sequence overflow"))?;
    let timer_ref = election_timer_ref(
        &state.profile.group_binding_ref,
        &state.node_id,
        state.profile.service_generation,
        state.current_term,
        sequence,
    )?;
    state.election_timer_sequence = sequence;
    state.active_election_timer_ref.clone_from(&timer_ref);
    Ok(ReplicaEffect::ArmElectionTimer { timer_ref })
}

pub(super) fn append_effects_for_all(state: &ReplicaState) -> Result<Vec<ReplicaEffect>> {
    peers(state).into_iter().map(|peer| append_effect_for(state, peer)).collect()
}

pub(super) fn append_effect_for(state: &ReplicaState, peer: String) -> Result<ReplicaEffect> {
    let next_index = state.next_index.get(&peer).copied().unwrap_or(INITIAL_LOG_INDEX);
    if let Some(snapshot) = &state.snapshot
        && next_index <= snapshot.last_included_index
    {
        return Ok(send_effect(state, peer, RaftMessage::InstallSnapshot {
            term: state.current_term,
            leader_id: state.node_id.clone(),
            snapshot: Box::new(snapshot.clone()),
            config_epoch: state.membership.config_epoch,
            fencing_epoch: state.profile.fencing_epoch,
        }));
    }
    let prev_log_index = next_index
        .checked_sub(NEXT_LOG_INDEX_STEP)
        .ok_or_else(|| MoltenError::invalid_harness("Raft next index is below the initial log index"))?;
    let prev_log_term = term_at(state, prev_log_index).unwrap_or(0);
    let entries = state
        .log
        .iter()
        .filter(|entry| entry.index >= next_index)
        .take(state.profile.max_message_entries)
        .cloned()
        .collect();
    Ok(send_effect(state, peer, RaftMessage::AppendEntries {
        term: state.current_term,
        leader_id: state.node_id.clone(),
        prev_log_index,
        prev_log_term,
        entries,
        leader_commit: state.commit_index,
        config_epoch: state.membership.config_epoch,
        fencing_epoch: state.profile.fencing_epoch,
    }))
}

pub(super) fn send_effect(state: &ReplicaState, to: String, message: RaftMessage) -> ReplicaEffect {
    ReplicaEffect::Send {
        envelope: ReplicaMessageEnvelope {
            group_binding_ref: state.profile.group_binding_ref.clone(),
            service_generation: state.profile.service_generation,
            from: state.node_id.clone(),
            to,
            message,
        },
    }
}

pub(super) fn vote_response(state: &ReplicaState, is_granted: bool) -> RaftMessage {
    RaftMessage::VoteResponse {
        term: state.current_term,
        voter_id: state.node_id.clone(),
        granted: is_granted,
        config_epoch: state.membership.config_epoch,
        fencing_epoch: state.profile.fencing_epoch,
    }
}

pub(super) fn append_response(
    state: &ReplicaState,
    is_success: bool,
    request_prev_log_index: u64,
    match_index: u64,
    conflict_index: u64,
) -> RaftMessage {
    RaftMessage::AppendResponse {
        term: state.current_term,
        follower_id: state.node_id.clone(),
        success: is_success,
        request_prev_log_index,
        match_index,
        conflict_index,
        config_epoch: state.membership.config_epoch,
        fencing_epoch: state.profile.fencing_epoch,
    }
}

pub(super) fn candidate_log_is_up_to_date(state: &ReplicaState, candidate_index: u64, candidate_term: u64) -> bool {
    candidate_term > last_log_term(state)
        || (candidate_term == last_log_term(state) && candidate_index >= last_log_index(state))
}

pub(super) fn previous_log_matches(state: &ReplicaState, index: u64, term: u64) -> bool {
    term_at(state, index) == Some(term)
}

pub(super) fn term_at(state: &ReplicaState, index: u64) -> Option<u64> {
    if index == 0 {
        return Some(0);
    }
    if let Some(snapshot) = &state.snapshot
        && snapshot.last_included_index == index
    {
        return Some(snapshot.last_included_term);
    }
    state.log.iter().find(|entry| entry.index == index).map(|entry| entry.term)
}

pub(super) fn last_log_index(state: &ReplicaState) -> u64 {
    state
        .log
        .last()
        .map_or_else(|| state.snapshot.as_ref().map_or(0, |snapshot| snapshot.last_included_index), |entry| entry.index)
}

pub(super) fn last_log_term(state: &ReplicaState) -> u64 {
    state
        .log
        .last()
        .map_or_else(|| state.snapshot.as_ref().map_or(0, |snapshot| snapshot.last_included_term), |entry| entry.term)
}

pub(super) fn next_conflict_index(state: &ReplicaState) -> Result<u64> {
    last_log_index(state)
        .checked_add(NEXT_LOG_INDEX_STEP)
        .ok_or_else(|| MoltenError::invalid_harness("Raft conflict index overflow"))
}

pub(super) fn entries_in_range(state: &ReplicaState, exclusive_start: u64, inclusive_end: u64) -> Vec<ReplicatedEntry> {
    state
        .log
        .iter()
        .filter(|entry| entry.index > exclusive_start && entry.index <= inclusive_end)
        .cloned()
        .collect()
}

pub(super) fn match_index_for(state: &ReplicaState, voter: &str) -> u64 {
    if voter == state.node_id {
        return last_log_index(state);
    }
    state.match_index.get(voter).copied().unwrap_or(0)
}

pub(super) fn peers(state: &ReplicaState) -> Vec<String> {
    state.membership.voters.iter().filter(|voter| *voter != &state.node_id).cloned().collect()
}
