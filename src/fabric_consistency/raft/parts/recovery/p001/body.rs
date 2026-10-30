
fn replay_snapshot(
    state: &mut ReplicaState,
    durable_commit_index: &mut u64,
    snapshot: Option<ReplicaSnapshot>,
) -> crate::error::Result<()> {
    if let Some(snapshot) = &snapshot {
        *durable_commit_index = (*durable_commit_index).max(snapshot.last_included_index);
    }
    install_snapshot(state, *durable_commit_index, snapshot)
}

fn install_snapshot(
    state: &mut ReplicaState,
    durable_commit_index: u64,
    snapshot: Option<ReplicaSnapshot>,
) -> crate::error::Result<()> {
    let Some(snapshot) = snapshot else {
        return Ok(());
    };
    if snapshot.group_binding_ref != state.profile.group_binding_ref
        || snapshot.membership_ref != state.membership.membership_ref
        || snapshot.config_epoch != state.membership.config_epoch
        || snapshot.fencing_epoch != state.profile.fencing_epoch
        || snapshot.last_included_index > durable_commit_index
    {
        return Err(crate::error::Failure::invalid_harness(
            "live Raft recovered snapshot binding or boundary mismatch",
        ));
    }
    for index in snapshot.completed_requests.values() {
        if *index == INITIAL_COMMIT_INDEX || *index > snapshot.last_included_index {
            return Err(crate::error::Failure::invalid_harness(
                "recovered snapshot completed request index is out of range",
            ));
        }
    }
    state.completed_requests.clone_from(&snapshot.completed_requests);
    let has_matching_boundary = state.log.iter().any(|entry|
        entry.index == snapshot.last_included_index && entry.term == snapshot.last_included_term);
    state.log.retain(|entry| has_matching_boundary && entry.index > snapshot.last_included_index);
    state.snapshot = Some(snapshot);
    Ok(())
}

fn support_last_log_index(state: &ReplicaState) -> u64 {
    state.log.last().map_or_else(
        || state.snapshot.as_ref().map_or(INITIAL_COMMIT_INDEX, |snapshot| snapshot.last_included_index),
        |entry| entry.index,
    )
}

fn r#ref(
    state: &ReplicaState,
    durable_record_refs: &[String],
    snapshot_bytes: Option<&[u8]>,
) -> crate::error::Result<String> {
    let snapshot_content_ref = snapshot_bytes.map(crate::preserves_rail::content_ref_from_bytes);
    let snapshot_value = snapshot_content_ref.as_deref().map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |reference| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(reference)]),
    );
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-recovery-plan-v2", vec![
        crate::preserves_rail::string(&state.profile.group_binding_ref),
        crate::preserves_rail::string(&state.node_id),
        crate::preserves_rail::u64_value(state.profile.service_generation),
        crate::preserves_rail::u64_value(state.current_term),
        crate::preserves_rail::u64_value(state.commit_index),
        crate::preserves_rail::bool_value(matches!(state.rejoin, ReplicaRejoin::Recovering(_))),
        crate::preserves_rail::sequence(durable_record_refs.iter().map(crate::preserves_rail::string).collect()),
        snapshot_value,
    ]))
}

fn finish_recovered_state(state: &mut ReplicaState, durable_commit_index: u64) -> crate::error::Result<()> {
    state.last_applied = durable_commit_index;
    let log_term = state.log.iter().map(|entry| entry.term)
        .chain(state.snapshot.iter().map(|snapshot| snapshot.last_included_term))
        .max().unwrap_or(INITIAL_TERM);
    let is_hard_state_lost = state.current_term < log_term;
    let is_recovery_required = is_hard_state_lost || matches!(state.rejoin, ReplicaRejoin::Recovering(_));
    if is_hard_state_lost {
        state.current_term = log_term;
        state.voted_for = Some(state.node_id.clone());
    }
    state.election_timer_sequence = INITIAL_ELECTION_TIMER_SEQUENCE;
    state.active_election_timer_ref = election_timer_ref(
        &state.profile.group_binding_ref, &state.node_id, state.profile.service_generation,
        state.current_term, state.election_timer_sequence,
    )?;
    if is_recovery_required {
        state.rejoin = ReplicaRejoin::Recovering(RecoveryRound {
            round_ref: state.active_election_timer_ref.clone(), fence_term: None,
            peers: std::collections::BTreeMap::new(),
        });
    }
    validate_recovered_replica_state(state)
}

fn initial_effects(state: &ReplicaState, replay_entries: &[ReplicatedEntry]) -> crate::error::Result<Vec<ReplicaEffect>> {
    let mut effects = Vec::with_capacity(state.profile.max_effects_per_step);
    if matches!(state.rejoin, ReplicaRejoin::Recovering(_)) {
        effects.push(ReplicaEffect::PersistHardState {
            term: state.current_term, voted_for: state.voted_for.clone(), is_recovery_required: true,
        });
    }
    if let Some(snapshot) = &state.snapshot {
        effects.push(ReplicaEffect::RestoreApplicationSnapshot { snapshot: snapshot.clone() });
    }
    if !replay_entries.is_empty() {
        effects.push(ReplicaEffect::ApplyCommitted { entries: replay_entries.to_vec() });
    }
    effects.push(ReplicaEffect::ArmElectionTimer { timer_ref: state.active_election_timer_ref.clone() });
    if effects.len() > state.profile.max_effects_per_step {
        return Err(crate::error::Failure::invalid_harness("live Raft recovery exceeds the admitted effect bound"));
    }
    Ok(effects)
}
