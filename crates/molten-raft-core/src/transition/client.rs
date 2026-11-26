use super::*;

pub(super) fn handle_proposal(
    state: &ReplicaState,
    request_ref: String,
    command_ref: String,
    command_schema_ref: String,
) -> crate::denial::Result<ReplicaTransition> {
    validation::ensure_running(state)?;
    validation::validate_content_ref(&request_ref, "Raft proposal request ref")?;
    validation::validate_content_ref(&command_ref, "Raft proposal command ref")?;
    validation::validate_content_ref(&command_schema_ref, "Raft proposal command schema ref")?;
    if !matches!(state.rejoin, ReplicaRejoin::Voter) {
        return proposal_outcome(state, request_ref, ProposalDisposition::Denied, None);
    }
    // r[impl molten.consensus.bounded_request_dedup]
    // Committed duplicates are recognized only through the deterministic request window; an older
    // committed entry still present in the local log does not count, so the decision never depends on
    // when this replica last snapshotted.
    if let Some(committed_index) = state.completed_requests.get(&request_ref).copied() {
        return proposal_outcome(state, request_ref, ProposalDisposition::Committed, Some(committed_index));
    }
    if state.log.iter().any(|entry| entry.index > state.commit_index && entry.request_ref == request_ref) {
        return proposal_outcome(state, request_ref, ProposalDisposition::Retryable, None);
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
    mode: ReadMode,
) -> crate::denial::Result<ReplicaTransition> {
    validation::ensure_running(state)?;
    validation::validate_content_ref(&request_ref, "Raft read request ref")?;
    if !matches!(state.rejoin, ReplicaRejoin::Voter) {
        return read_outcome(state, request_ref, mode, ReadDisposition::Denied);
    }
    match mode {
        ReadMode::LocalStale => read_outcome(state, request_ref, mode, ReadDisposition::Local),
        ReadMode::Lease => read_outcome(state, request_ref, mode, ReadDisposition::Retryable),
        ReadMode::Linearizable if state.role != ReplicaRole::Leader => {
            read_outcome(state, request_ref, mode, ReadDisposition::Retryable)
        }
        ReadMode::Linearizable if state.pending_reads.contains_key(&request_ref) => {
            read_outcome(state, request_ref, mode, ReadDisposition::Retryable)
        }
        ReadMode::Linearizable if state.pending_reads.len() >= MAX_PENDING_REPLICA_READS => {
            read_outcome(state, request_ref, mode, ReadDisposition::Retryable)
        }
        // r[impl molten.consensus.linearizable_reads]
        // An earlier leader may have committed and acknowledged an index above this leader's commit
        // index, so a read index taken now could miss that write.
        ReadMode::Linearizable if !support::is_commit_index_current(state) => {
            read_outcome(state, request_ref, mode, ReadDisposition::Retryable)
        }
        ReadMode::Linearizable => begin_linearizable_read(state, request_ref),
    }
}

fn read_outcome(
    state: &ReplicaState,
    request_ref: String,
    mode: ReadMode,
    disposition: ReadDisposition,
) -> crate::denial::Result<ReplicaTransition> {
    finish(state.clone(), vec![ReplicaEffect::ReadOutcome {
        request_ref,
        mode,
        disposition,
        observed_index: state.last_applied,
    }])
}

fn begin_linearizable_read(state: &ReplicaState, request_ref: String) -> crate::denial::Result<ReplicaTransition> {
    let mut next = state.clone();
    next.pending_reads.insert(request_ref.clone(), PendingReplicaRead {
        request_ref: request_ref.clone(),
        term: state.current_term,
        required_index: state.commit_index,
        acknowledgements: alloc::collections::BTreeSet::from([state.node_id.clone()]),
    });
    let effects = support::peers(&next)
        .into_iter()
        .map(|peer| {
            support::send_effect(&next, peer, Message::ReadProbe {
                term: next.current_term,
                leader_id: next.node_id.clone(),
                request_ref: request_ref.clone(),
                required_index: next.commit_index,
                config_epoch: next.membership.config_epoch,
                fencing_epoch: next.profile.fencing_epoch,
            })
        })
        .collect();
    finish(next, effects)
}

pub(super) fn handle_create_snapshot(
    state: &ReplicaState,
    application_state_ref: String,
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<ReplicaTransition> {
    validation::ensure_running(state)?;
    validation::validate_content_ref(&application_state_ref, "Raft snapshot application state ref")?;
    if !matches!(state.rejoin, ReplicaRejoin::Voter) {
        return Err(crate::denial::ReplicaError::Denied("unreconciled Raft participant cannot publish snapshots"));
    }
    if state.last_applied == INITIAL_COMMIT_INDEX {
        return Err(crate::denial::ReplicaError::Denied("Raft snapshot requires a committed application boundary"));
    }
    if state.snapshot.as_ref().is_some_and(|snapshot| snapshot.last_included_index >= state.last_applied) {
        return Err(crate::denial::ReplicaError::Denied("Raft snapshot boundary has not advanced"));
    }
    let last_included_term = support::term_at(state, state.last_applied)
        .ok_or(crate::denial::ReplicaError::Denied("Raft snapshot boundary term is absent"))?;
    let mut snapshot = ReplicaSnapshot {
        snapshot_ref: String::new(),
        group_binding_ref: state.profile.group_binding_ref.clone(),
        membership_ref: state.membership.membership_ref.clone(),
        config_epoch: state.membership.config_epoch,
        fencing_epoch: state.profile.fencing_epoch,
        last_included_index: state.last_applied,
        last_included_term,
        application_state_ref,
        completed_requests: state.completed_requests.clone(),
    };
    snapshot.snapshot_ref = support::snapshot_ref(&snapshot, identity)?;
    let mut next = state.clone();
    next.log.retain(|entry| entry.index > snapshot.last_included_index);
    next.snapshot = Some(snapshot.clone());
    finish(next, vec![ReplicaEffect::PersistSnapshot {
        snapshot,
        is_suffix_retained: true,
    }])
}

pub(super) fn handle_begin_drain(state: &ReplicaState) -> crate::denial::Result<ReplicaTransition> {
    validation::ensure_running(state)?;
    let mut next = state.clone();
    next.lifecycle = ReplicaLifecycle::Draining;
    next.pending_reads.clear();
    finish(next, vec![ReplicaEffect::LifecycleChanged {
        lifecycle: ReplicaLifecycle::Draining,
    }])
}

pub(super) fn handle_stop(state: &ReplicaState) -> crate::denial::Result<ReplicaTransition> {
    if state.lifecycle == ReplicaLifecycle::Stopped {
        return finish(state.clone(), Vec::new());
    }
    let mut next = state.clone();
    next.lifecycle = ReplicaLifecycle::Stopped;
    next.role = ReplicaRole::Follower;
    next.leader_id = None;
    next.pending_reads.clear();
    next.quorum_confirmed_term = None;
    finish(next, vec![ReplicaEffect::LifecycleChanged {
        lifecycle: ReplicaLifecycle::Stopped,
    }])
}

fn proposal_outcome(
    state: &ReplicaState,
    request_ref: String,
    disposition: ProposalDisposition,
    committed_index: Option<u64>,
) -> crate::denial::Result<ReplicaTransition> {
    finish(state.clone(), vec![ReplicaEffect::ProposalOutcome {
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
) -> crate::denial::Result<ReplicaTransition> {
    let mut next = state.clone();
    let index = support::last_log_index(&next)
        .checked_add(NEXT_LOG_INDEX_STEP)
        .ok_or(crate::denial::ReplicaError::Overflow("Raft proposal log index overflow"))?;
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
    finish(next, effects)
}
