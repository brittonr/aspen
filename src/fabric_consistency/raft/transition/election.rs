use super::*;
use crate::error::MoltenError;
use crate::error::Result;

pub(super) struct VoteRequestInput {
    pub from: String,
    pub term: u64,
    pub candidate_id: String,
    pub last_log_index: u64,
    pub last_log_term: u64,
}

pub(super) struct VoteResponseInput {
    pub from: String,
    pub term: u64,
    pub voter_id: String,
    pub is_granted: bool,
}

pub(super) fn handle_election_timeout(state: &ReplicaState, entropy_ref: String) -> Result<ReplicaTransition> {
    validation::ensure_running(state)?;
    validation::validate_content_ref(&entropy_ref, "Raft election entropy ref")?;
    if state.role == ReplicaRole::Leader {
        return Err(MoltenError::invalid_harness("leader cannot process a follower election timeout"));
    }
    let next_term = state
        .current_term
        .checked_add(NEXT_TERM_STEP)
        .ok_or_else(|| MoltenError::invalid_harness("Raft term overflow"))?;
    let mut next = state.clone();
    next.current_term = next_term;
    next.role = ReplicaRole::Candidate;
    next.voted_for = Some(next.node_id.clone());
    next.leader_id = None;
    next.votes_received.clear();
    next.votes_received.insert(next.node_id.clone());
    next.next_index.clear();
    next.match_index.clear();
    next.quorum_confirmed_term = None;

    let mut effects = Vec::with_capacity(state.profile.max_effects_per_step);
    effects.push(ReplicaEffect::PersistHardState {
        term: next.current_term,
        voted_for: next.voted_for.clone(),
    });
    for voter in support::peers(&next) {
        effects.push(support::send_effect(&next, voter, RaftMessage::RequestVote {
            term: next.current_term,
            candidate_id: next.node_id.clone(),
            last_log_index: support::last_log_index(&next),
            last_log_term: support::last_log_term(&next),
            config_epoch: next.membership.config_epoch,
            fencing_epoch: next.profile.fencing_epoch,
        }));
    }
    effects.push(ReplicaEffect::ArmElectionTimer { entropy_ref });
    finish_transition(next, effects)
}

pub(super) fn handle_heartbeat_timeout(state: &ReplicaState) -> Result<ReplicaTransition> {
    validation::ensure_running(state)?;
    if state.role != ReplicaRole::Leader {
        return Err(MoltenError::invalid_harness("only a live Raft leader can emit heartbeat traffic"));
    }
    let next = state.clone();
    let mut effects = support::append_effects_for_all(&next)?;
    effects.push(ReplicaEffect::ArmHeartbeatTimer);
    finish_transition(next, effects)
}

pub(super) fn handle_request_vote(transition: &mut MessageTransition, input: VoteRequestInput) -> Result<()> {
    if input.term < transition.next.current_term {
        transition.effects.push(support::send_effect(
            &transition.next,
            input.from,
            support::vote_response(&transition.next, false),
        ));
        return Ok(());
    }
    let can_vote = transition.next.voted_for.as_deref().is_none_or(|voted_for| voted_for == input.candidate_id);
    let is_up_to_date =
        support::candidate_log_is_up_to_date(&transition.next, input.last_log_index, input.last_log_term);
    let is_granted = can_vote && is_up_to_date;
    if is_granted {
        if transition.next.voted_for.as_deref() != Some(input.candidate_id.as_str()) {
            transition.next.voted_for = Some(input.candidate_id);
            transition.persist_hard_state = true;
        }
        transition.next.role = ReplicaRole::Follower;
        transition.next.leader_id = None;
        transition.next.votes_received.clear();
        transition.next.quorum_confirmed_term = None;
    }
    transition.effects.push(support::send_effect(
        &transition.next,
        input.from,
        support::vote_response(&transition.next, is_granted),
    ));
    if is_granted {
        transition.effects.push(ReplicaEffect::ArmElectionTimer {
            entropy_ref: transition.next.profile.entropy_profile_ref.clone(),
        });
    }
    Ok(())
}

pub(super) fn handle_vote_response(transition: &mut MessageTransition, input: VoteResponseInput) -> Result<()> {
    if input.term < transition.next.current_term || transition.next.role != ReplicaRole::Candidate {
        return Ok(());
    }
    if input.voter_id != input.from {
        return Err(MoltenError::invalid_harness("Raft vote response voter does not match its sender"));
    }
    if input.is_granted {
        transition.next.votes_received.insert(input.voter_id);
    }
    if transition.next.votes_received.len() >= STATIC_QUORUM_COUNT {
        become_leader(transition)?;
    }
    Ok(())
}

fn become_leader(transition: &mut MessageTransition) -> Result<()> {
    let next_log_index = support::last_log_index(&transition.next)
        .checked_add(NEXT_LOG_INDEX_STEP)
        .ok_or_else(|| MoltenError::invalid_harness("Raft next log index overflow"))?;
    transition.next.role = ReplicaRole::Leader;
    transition.next.leader_id = Some(transition.next.node_id.clone());
    transition.next.quorum_confirmed_term = Some(transition.next.current_term);
    transition.next.next_index.clear();
    transition.next.match_index.clear();
    transition
        .next
        .match_index
        .insert(transition.next.node_id.clone(), support::last_log_index(&transition.next));
    for voter in support::peers(&transition.next) {
        transition.next.next_index.insert(voter, next_log_index);
    }
    transition.effects.extend(support::append_effects_for_all(&transition.next)?);
    transition.effects.push(ReplicaEffect::ArmHeartbeatTimer);
    Ok(())
}
