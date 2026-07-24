use super::*;
use crate::error::MoltenError;
use crate::error::Result;

pub(super) struct AppendEntriesInput {
    pub from: String,
    pub term: u64,
    pub leader_id: String,
    pub prev_log_index: u64,
    pub prev_log_term: u64,
    pub entries: Vec<ReplicatedEntry>,
    pub leader_commit: u64,
}

pub(super) struct AppendResponseInput {
    pub from: String,
    pub term: u64,
    pub follower_id: String,
    pub is_success: bool,
    pub request_prev_log_index: u64,
    pub match_index: u64,
    pub conflict_index: u64,
}

pub(super) fn handle_append_entries(transition: &mut MessageTransition, input: AppendEntriesInput) -> Result<()> {
    if input.term < transition.next.current_term {
        push_append_response(
            transition,
            &input.from,
            false,
            input.prev_log_index,
            support::last_log_index(&transition.next),
        )?;
        return Ok(());
    }
    if transition.next.role == ReplicaRole::Leader && input.leader_id != transition.next.node_id {
        return Err(MoltenError::invalid_harness("Raft observed two leaders in one term"));
    }
    become_follower(transition, input.leader_id);
    if !support::previous_log_matches(&transition.next, input.prev_log_index, input.prev_log_term) {
        push_append_response(
            transition,
            &input.from,
            false,
            input.prev_log_index,
            support::last_log_index(&transition.next),
        )?;
        arm_election_timer(transition)?;
        return Ok(());
    }

    validation::validate_incoming_entries(&transition.next, input.prev_log_index, input.term, &input.entries)?;
    let acknowledged_index = input.entries.last().map_or(input.prev_log_index, |entry| entry.index);
    let (truncate_from, appended) = merge_entries(&mut transition.next, input.entries)?;
    push_log_effects(transition, truncate_from, appended);
    apply_leader_commit(transition, input.leader_commit.min(acknowledged_index));
    push_append_response(transition, &input.from, true, input.prev_log_index, acknowledged_index)?;
    arm_election_timer(transition)?;
    Ok(())
}

pub(super) fn handle_append_response(transition: &mut MessageTransition, input: AppendResponseInput) -> Result<()> {
    if input.term < transition.next.current_term || transition.next.role != ReplicaRole::Leader {
        return Ok(());
    }
    if input.follower_id != input.from {
        return Err(MoltenError::invalid_harness("Raft append response follower does not match its sender"));
    }
    if input.is_success {
        apply_successful_response(transition, input)?;
        return Ok(());
    }
    apply_failed_response(transition, input)
}

fn become_follower(transition: &mut MessageTransition, leader_id: String) {
    transition.next.role = ReplicaRole::Follower;
    transition.next.leader_id = Some(leader_id);
    transition.next.votes_received.clear();
    transition.next.next_index.clear();
    transition.next.match_index.clear();
    transition.next.quorum_confirmed_term = None;
}

fn push_log_effects(transition: &mut MessageTransition, truncate_from: Option<u64>, appended: Vec<ReplicatedEntry>) {
    if truncate_from.is_none() && appended.is_empty() {
        return;
    }
    transition.effects.push(ReplicaEffect::PersistEntries {
        truncate_from,
        entries: appended,
    });
    transition.effects.push(ReplicaEffect::FlushLog {
        through_index: support::last_log_index(&transition.next),
    });
}

fn push_append_response(
    transition: &mut MessageTransition,
    recipient: &str,
    is_success: bool,
    request_prev_log_index: u64,
    match_index: u64,
) -> Result<()> {
    let conflict_index = support::next_conflict_index(&transition.next)?;
    transition.effects.push(support::send_effect(
        &transition.next,
        recipient.to_string(),
        support::append_response(&transition.next, is_success, request_prev_log_index, match_index, conflict_index),
    ));
    Ok(())
}

fn arm_election_timer(transition: &mut MessageTransition) -> Result<()> {
    let timer_effect = support::arm_election_timer(&mut transition.next)?;
    transition.effects.push(timer_effect);
    Ok(())
}

fn apply_successful_response(transition: &mut MessageTransition, input: AppendResponseInput) -> Result<()> {
    if input.match_index < input.request_prev_log_index || input.match_index > support::last_log_index(&transition.next)
    {
        return Err(MoltenError::invalid_harness("Raft follower acknowledged an invalid log range"));
    }
    let prior_match = transition.next.match_index.get(&input.follower_id).copied().unwrap_or(0);
    if input.match_index < prior_match {
        return Ok(());
    }
    let next_index = input
        .match_index
        .checked_add(NEXT_LOG_INDEX_STEP)
        .ok_or_else(|| MoltenError::invalid_harness("Raft follower next index overflow"))?;
    transition.next.match_index.insert(input.follower_id.clone(), input.match_index);
    transition.next.next_index.insert(input.follower_id, next_index);
    transition.next.quorum_confirmed_term = Some(transition.next.current_term);
    advance_leader_commit(transition);
    Ok(())
}

fn apply_failed_response(transition: &mut MessageTransition, input: AppendResponseInput) -> Result<()> {
    let local_next = match transition.next.next_index.get(&input.from).copied() {
        Some(index) => index,
        None => support::next_conflict_index(&transition.next)?,
    };
    let response_next = input
        .request_prev_log_index
        .checked_add(NEXT_LOG_INDEX_STEP)
        .ok_or_else(|| MoltenError::invalid_harness("Raft append response prefix overflow"))?;
    if response_next != local_next {
        return Ok(());
    }
    let decremented = local_next.checked_sub(NEXT_LOG_INDEX_STEP).unwrap_or(INITIAL_LOG_INDEX).max(INITIAL_LOG_INDEX);
    let retry_index = input.conflict_index.clamp(INITIAL_LOG_INDEX, decremented);
    transition.next.next_index.insert(input.from.clone(), retry_index);
    transition.effects.push(support::append_effect_for(&transition.next, input.from)?);
    Ok(())
}

fn apply_leader_commit(transition: &mut MessageTransition, leader_commit: u64) {
    let next_commit = leader_commit.min(support::last_log_index(&transition.next));
    if next_commit <= transition.next.commit_index {
        return;
    }
    let entries = support::entries_in_range(&transition.next, transition.next.commit_index, next_commit);
    transition.next.commit_index = next_commit;
    transition.next.last_applied = next_commit;
    record_completed_requests(&mut transition.next, &entries);
    transition.effects.push(ReplicaEffect::PersistCommit {
        through_index: next_commit,
    });
    if !entries.is_empty() {
        transition.effects.push(ReplicaEffect::ApplyCommitted { entries });
    }
}

fn advance_leader_commit(transition: &mut MessageTransition) {
    let next_commit = highest_quorum_index(&transition.next);
    if next_commit <= transition.next.commit_index {
        return;
    }
    let entries = support::entries_in_range(&transition.next, transition.next.commit_index, next_commit);
    transition.next.commit_index = next_commit;
    transition.next.last_applied = next_commit;
    record_completed_requests(&mut transition.next, &entries);
    transition.effects.push(ReplicaEffect::PersistCommit {
        through_index: next_commit,
    });
    transition.effects.push(ReplicaEffect::ApplyCommitted {
        entries: entries.clone(),
    });
    transition.effects.extend(entries.into_iter().map(|entry| ReplicaEffect::ProposalOutcome {
        request_ref: entry.request_ref,
        disposition: ProposalDisposition::Committed,
        committed_index: Some(entry.index),
    }));
}

fn record_completed_requests(state: &mut ReplicaState, entries: &[ReplicatedEntry]) {
    for entry in entries {
        state.completed_requests.insert(entry.request_ref.clone(), entry.index);
    }
}

fn highest_quorum_index(state: &ReplicaState) -> u64 {
    let last = support::last_log_index(state);
    for candidate in ((state.commit_index + NEXT_LOG_INDEX_STEP)..=last).rev() {
        if support::term_at(state, candidate) != Some(state.current_term) {
            continue;
        }
        let replicated = state
            .membership
            .voters
            .iter()
            .filter(|voter| support::match_index_for(state, voter) >= candidate)
            .count();
        if replicated >= STATIC_QUORUM_COUNT {
            return candidate;
        }
    }
    state.commit_index
}

fn merge_entries(
    state: &mut ReplicaState,
    incoming: Vec<ReplicatedEntry>,
) -> Result<(Option<u64>, Vec<ReplicatedEntry>)> {
    let first_change = incoming.iter().position(|entry| {
        state
            .log
            .iter()
            .find(|existing| existing.index == entry.index)
            .is_none_or(|existing| existing != entry)
    });
    let Some(first_change) = first_change else {
        return Ok((None, Vec::new()));
    };
    let change_index = incoming[first_change].index;
    if change_index <= state.commit_index {
        return Err(MoltenError::invalid_harness("Raft append would overwrite a committed entry"));
    }
    let truncate_from = state.log.iter().any(|entry| entry.index >= change_index).then_some(change_index);
    state.log.retain(|entry| entry.index < change_index);
    let appended = incoming.into_iter().skip(first_change).collect::<Vec<_>>();
    state.log.extend(appended.iter().cloned());
    Ok((truncate_from, appended))
}
