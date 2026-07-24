use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric_consistency::ConsistencyReadMode;

pub(super) fn handle_proposal(
    state: &ReplicaState,
    request_ref: String,
    command_ref: String,
    command_schema_ref: String,
) -> Result<ReplicaTransition> {
    validation::ensure_running(state)?;
    validation::validate_content_ref(&request_ref, "Raft proposal request ref")?;
    validation::validate_content_ref(&command_ref, "Raft proposal command ref")?;
    validation::validate_content_ref(&command_schema_ref, "Raft proposal command schema ref")?;
    if let Some(existing) = state.log.iter().find(|entry| entry.request_ref == request_ref) {
        return duplicate_proposal_transition(state, request_ref, existing);
    }
    if state.role != ReplicaRole::Leader {
        return proposal_outcome(state, request_ref, ProposalDisposition::Retryable, None);
    }
    if state.log.len() >= state.profile.max_log_entries {
        return proposal_outcome(state, request_ref, ProposalDisposition::Denied, None);
    }
    append_proposal(state, request_ref, command_ref, command_schema_ref)
}

pub(super) fn handle_read(
    state: &ReplicaState,
    request_ref: String,
    mode: ConsistencyReadMode,
) -> Result<ReplicaTransition> {
    validation::ensure_running(state)?;
    validation::validate_content_ref(&request_ref, "Raft read request ref")?;
    let disposition = match mode {
        ConsistencyReadMode::LocalStale => ReadDisposition::Local,
        ConsistencyReadMode::Linearizable | ConsistencyReadMode::Lease => ReadDisposition::Retryable,
    };
    finish_transition(state.clone(), vec![ReplicaEffect::ReadOutcome {
        request_ref,
        mode,
        disposition,
        observed_index: state.last_applied,
    }])
}

pub(super) fn handle_begin_drain(state: &ReplicaState) -> Result<ReplicaTransition> {
    validation::ensure_running(state)?;
    let mut next = state.clone();
    next.lifecycle = ReplicaLifecycle::Draining;
    finish_transition(next, vec![ReplicaEffect::LifecycleChanged {
        lifecycle: ReplicaLifecycle::Draining,
    }])
}

pub(super) fn handle_stop(state: &ReplicaState) -> Result<ReplicaTransition> {
    if state.lifecycle == ReplicaLifecycle::Stopped {
        return finish_transition(state.clone(), Vec::new());
    }
    let mut next = state.clone();
    next.lifecycle = ReplicaLifecycle::Stopped;
    next.role = ReplicaRole::Follower;
    next.leader_id = None;
    next.quorum_confirmed_term = None;
    finish_transition(next, vec![ReplicaEffect::LifecycleChanged {
        lifecycle: ReplicaLifecycle::Stopped,
    }])
}

fn duplicate_proposal_transition(
    state: &ReplicaState,
    request_ref: String,
    existing: &ReplicatedEntry,
) -> Result<ReplicaTransition> {
    let disposition = if existing.index <= state.commit_index {
        ProposalDisposition::Committed
    } else {
        ProposalDisposition::Retryable
    };
    let committed_index = (disposition == ProposalDisposition::Committed).then_some(existing.index);
    proposal_outcome(state, request_ref, disposition, committed_index)
}

fn proposal_outcome(
    state: &ReplicaState,
    request_ref: String,
    disposition: ProposalDisposition,
    committed_index: Option<u64>,
) -> Result<ReplicaTransition> {
    finish_transition(state.clone(), vec![ReplicaEffect::ProposalOutcome {
        request_ref,
        disposition,
        committed_index,
    }])
}

fn append_proposal(
    state: &ReplicaState,
    request_ref: String,
    command_ref: String,
    command_schema_ref: String,
) -> Result<ReplicaTransition> {
    let mut next = state.clone();
    let index = support::last_log_index(&next)
        .checked_add(NEXT_LOG_INDEX_STEP)
        .ok_or_else(|| MoltenError::invalid_harness("Raft proposal log index overflow"))?;
    let entry = ReplicatedEntry {
        index,
        term: next.current_term,
        request_ref,
        command_ref,
        command_schema_ref,
    };
    next.log.push(entry.clone());
    next.match_index.insert(next.node_id.clone(), index);
    let mut effects = vec![
        ReplicaEffect::PersistEntries {
            truncate_from: None,
            entries: vec![entry],
        },
        ReplicaEffect::FlushLog { through_index: index },
    ];
    effects.extend(support::append_effects_for_all(&next)?);
    finish_transition(next, effects)
}
