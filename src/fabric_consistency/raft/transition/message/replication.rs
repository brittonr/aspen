use super::*;

/// Effects the executor adds around a guarded transition: the durable intent write and its clear
/// (`executor/batch.rs`).
const DURABLE_INTENT_FENCE_EFFECTS: usize = 2;
/// The commit record and the application batch that accompany every commit advancement.
const COMMIT_RECORD_EFFECTS: usize = 2;

/// The follower log after reconciling one `AppendEntries` batch.
struct MergedEntries {
    truncate_from: Option<u64>,
    appended: Vec<ReplicatedEntry>,
    /// The last index known to match the leader's log after the merge.
    acknowledged_index: u64,
}

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

pub(super) fn handle_append_entries(
    transition: &mut MessagePlan,
    input: AppendEntriesInput,
) -> crate::error::Result<()> {
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
        return Err(crate::error::Failure::invalid_harness("Raft observed two leaders in one term"));
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

    // r[impl molten.consensus.raft_replication_and_recovery_liveness]
    // Resolve the conflicting suffix first: request refs are checked only against entries the
    // merge retains, never against uncommitted entries it is about to truncate.
    let first_change = first_changed_entry(&transition.next, &input.entries);
    let retained_before = first_change.and_then(|position| input.entries.get(position)).map(|entry| entry.index);
    validation::validate_incoming_entries(
        &transition.next,
        input.prev_log_index,
        input.term,
        &input.entries,
        retained_before,
    )?;
    let merged = merge_entries(&mut transition.next, input.prev_log_index, input.entries, first_change)?;
    push_log_effects(transition, merged.truncate_from, merged.appended);
    apply_leader_commit(transition, input.leader_commit.min(merged.acknowledged_index));
    push_append_response(transition, &input.from, true, input.prev_log_index, merged.acknowledged_index)?;
    arm_election_timer(transition)?;
    Ok(())
}

pub(super) fn handle_append_response(
    transition: &mut MessagePlan,
    input: AppendResponseInput,
) -> crate::error::Result<()> {
    if input.term < transition.next.current_term || transition.next.role != ReplicaRole::Leader {
        return Ok(());
    }
    if input.follower_id != input.from {
        return Err(crate::error::Failure::invalid_harness("Raft append response follower does not match its sender"));
    }
    if input.is_success {
        apply_successful_response(transition, input)?;
        return Ok(());
    }
    apply_failed_response(transition, input)
}

fn become_follower(transition: &mut MessagePlan, leader_id: String) {
    transition.next.role = ReplicaRole::Follower;
    transition.next.leader_id = Some(leader_id);
    transition.next.votes_received.clear();
    transition.next.next_index.clear();
    transition.next.match_index.clear();
    transition.next.pending_reads.clear();
    transition.next.quorum_confirmed_term = None;
}

fn push_log_effects(transition: &mut MessagePlan, truncate_from: Option<u64>, appended: Vec<ReplicatedEntry>) {
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
    transition: &mut MessagePlan,
    recipient: &str,
    is_success: bool,
    request_prev_log_index: u64,
    match_index: u64,
) -> crate::error::Result<()> {
    let conflict_index = support::next_conflict_index(&transition.next)?;
    transition.effects.push(support::send_effect(
        &transition.next,
        recipient.to_string(),
        support::append_response(&transition.next, is_success, request_prev_log_index, match_index, conflict_index),
    ));
    Ok(())
}

fn arm_election_timer(transition: &mut MessagePlan) -> crate::error::Result<()> {
    let timer_effect = support::arm_election_timer(&mut transition.next)?;
    transition.effects.push(timer_effect);
    Ok(())
}

fn apply_successful_response(transition: &mut MessagePlan, input: AppendResponseInput) -> crate::error::Result<()> {
    if input.match_index < input.request_prev_log_index || input.match_index > support::last_log_index(&transition.next)
    {
        return Err(crate::error::Failure::invalid_harness("Raft follower acknowledged an invalid log range"));
    }
    let prior_match = transition.next.match_index.get(&input.follower_id).copied().unwrap_or(0);
    if input.match_index < prior_match {
        return Ok(());
    }
    let next_index = input
        .match_index
        .checked_add(NEXT_LOG_INDEX_STEP)
        .ok_or_else(|| crate::error::Failure::invalid_harness("Raft follower next index overflow"))?;
    transition.next.match_index.insert(input.follower_id.clone(), input.match_index);
    transition.next.next_index.insert(input.follower_id, next_index);
    transition.next.quorum_confirmed_term = Some(transition.next.current_term);
    advance_leader_commit(transition);
    Ok(())
}

fn apply_failed_response(transition: &mut MessagePlan, input: AppendResponseInput) -> crate::error::Result<()> {
    let local_next = match transition.next.next_index.get(&input.from).copied() {
        Some(index) => index,
        None => support::next_conflict_index(&transition.next)?,
    };
    let response_next = input
        .request_prev_log_index
        .checked_add(NEXT_LOG_INDEX_STEP)
        .ok_or_else(|| crate::error::Failure::invalid_harness("Raft append response prefix overflow"))?;
    if response_next != local_next {
        return Ok(());
    }
    let decremented = local_next.checked_sub(NEXT_LOG_INDEX_STEP).unwrap_or(INITIAL_LOG_INDEX).max(INITIAL_LOG_INDEX);
    let retry_index = input.conflict_index.clamp(INITIAL_LOG_INDEX, decremented);
    transition.next.next_index.insert(input.from.clone(), retry_index);
    transition.effects.push(support::append_effect_for(&transition.next, input.from)?);
    Ok(())
}

fn apply_leader_commit(transition: &mut MessagePlan, leader_commit: u64) {
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

/// Commit through the highest quorum-replicated current-term index, at most as many entries as the
/// step's effect budget leaves room for: each committed entry adds one proposal outcome, and the
/// executor fences the batch with a durable intent write and clear. Committing a prefix is safe
/// because every entry up to a quorum-replicated current-term index is committed; every later
/// successful append response continues from the new commit index.
// r[impl molten.consensus.raft_replication_and_recovery_liveness]
fn advance_leader_commit(transition: &mut MessagePlan) {
    let quorum_commit = highest_quorum_index(&transition.next);
    if quorum_commit <= transition.next.commit_index {
        return;
    }
    let outcome_budget = transition
        .next
        .profile
        .max_effects_per_step
        .saturating_sub(transition.effects.len())
        .saturating_sub(DURABLE_INTENT_FENCE_EFFECTS)
        .saturating_sub(COMMIT_RECORD_EFFECTS);
    let Ok(outcome_budget) = u64::try_from(outcome_budget) else {
        return;
    };
    let next_commit = quorum_commit.min(transition.next.commit_index.saturating_add(outcome_budget));
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

// r[impl molten.project.release_overflow_checks]
fn highest_quorum_index(state: &ReplicaState) -> u64 {
    let last = support::last_log_index(state);
    // At the largest representable commit index no higher candidate exists.
    let Some(first_candidate) = state.commit_index.checked_add(NEXT_LOG_INDEX_STEP) else {
        return state.commit_index;
    };
    for candidate in (first_candidate..=last).rev() {
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

fn first_changed_entry(state: &ReplicaState, incoming: &[ReplicatedEntry]) -> Option<usize> {
    incoming.iter().position(|entry| {
        state
            .log
            .iter()
            .find(|existing| existing.index == entry.index)
            .is_none_or(|existing| existing != entry)
    })
}

/// Truncate the conflicting suffix and append the incoming entries that fit the admitted log bound.
///
/// A follower at its bound stores only the prefix that fits and acknowledges exactly that prefix:
/// commit advancement through it lets the hosting service compact with a snapshot, and the leader
/// resends the rest from the acknowledged index.
// r[impl molten.consensus.raft_replication_and_recovery_liveness]
fn merge_entries(
    state: &mut ReplicaState,
    prev_log_index: u64,
    incoming: Vec<ReplicatedEntry>,
    first_change: Option<usize>,
) -> crate::error::Result<MergedEntries> {
    let Some(first_change) = first_change else {
        return Ok(MergedEntries {
            truncate_from: None,
            appended: Vec::new(),
            acknowledged_index: incoming.last().map_or(prev_log_index, |entry| entry.index),
        });
    };
    let change_index = incoming
        .get(first_change)
        .map(|entry| entry.index)
        .ok_or_else(|| crate::error::Failure::invalid_harness("Raft append change lies outside its batch"))?;
    if change_index <= state.commit_index {
        return Err(crate::error::Failure::invalid_harness("Raft append would overwrite a committed entry"));
    }
    let matched_index = change_index
        .checked_sub(NEXT_LOG_INDEX_STEP)
        .ok_or_else(|| crate::error::Failure::invalid_harness("Raft append change index underflow"))?;
    let truncate_from = state.log.iter().any(|entry| entry.index >= change_index).then_some(change_index);
    state.log.retain(|entry| entry.index < change_index);
    let capacity = state.profile.max_log_entries.saturating_sub(state.log.len());
    let appended = incoming.into_iter().skip(first_change).take(capacity).collect::<Vec<_>>();
    state.log.extend(appended.iter().cloned());
    Ok(MergedEntries {
        truncate_from,
        acknowledged_index: appended.last().map_or(matched_index, |entry| entry.index),
        appended,
    })
}

#[cfg(test)]
mod tests {
    // r[verify molten.project.release_overflow_checks]
    #[test]
    fn quorum_search_at_the_maximum_commit_index_returns_it_without_overflow() {
        let mut state = crate::fabric_consistency::raft::tests::committed_leader();
        state.commit_index = u64::MAX;
        assert_eq!(super::highest_quorum_index(&state), u64::MAX);
    }
}
