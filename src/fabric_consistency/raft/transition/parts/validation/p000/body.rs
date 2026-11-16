use super::*;

pub(super) fn validate_incoming_entries(
    state: &ReplicaState,
    prev_log_index: u64,
    leader_term: u64,
    entries: &[ReplicatedEntry],
) -> crate::error::Result<()> {
    if entries.len() > state.profile.max_message_entries {
        return Err(crate::error::Failure::invalid_harness(
            "Raft append entries exceed the admitted message bound",
        ));
    }
    let mut expected_index = prev_log_index
        .checked_add(NEXT_LOG_INDEX_STEP)
        .ok_or_else(|| crate::error::Failure::invalid_harness("Raft append index overflow"))?;
    let mut request_refs = std::collections::BTreeMap::new();
    for entry in entries {
        validate_incoming_entry(state, entry, expected_index, leader_term, &mut request_refs)?;
        expected_index = expected_index
            .checked_add(NEXT_LOG_INDEX_STEP)
            .ok_or_else(|| crate::error::Failure::invalid_harness("Raft append index overflow"))?;
    }
    Ok(())
}

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric_consistency/raft/transition/parts/validation/p000/message.rs"));

pub(super) fn validate_replica_state(state: &ReplicaState) -> crate::error::Result<()> {
    if !matches!(state.rejoin, ReplicaRejoin::Voter) && state.role != ReplicaRole::Follower {
        return Err(crate::error::Failure::invalid_harness("recovering Raft participant cannot campaign or lead"));
    }
    if state.membership.voters.len() != STATIC_VOTER_COUNT || !state.membership.voters.contains(&state.node_id) {
        return Err(crate::error::Failure::invalid_harness("Raft state has invalid static membership"));
    }
    if state.log.len() > state.profile.max_log_entries {
        return Err(crate::error::Failure::invalid_harness("Raft state exceeds its admitted log bound"));
    }
    validate_snapshot(state)?;
    validate_log(state)?;
    let last = support::last_log_index(state);
    if state.last_applied > state.commit_index || state.commit_index > last {
        return Err(crate::error::Failure::invalid_harness(
            "Raft applied or committed index exceeds the local log",
        ));
    }
    validate_role(state)?;
    validate_completed_requests(state)?;
    validate_pending_reads(state)?;
    validate_election_timer(state)?;
    if state.voted_for.as_ref().is_some_and(|voter| !state.membership.voters.contains(voter)) {
        return Err(crate::error::Failure::invalid_harness("Raft vote target is outside admitted membership"));
    }
    Ok(())
}

/// The idempotency map must equal the deterministic window of the applied prefix: the snapshot's
/// retained refs plus the applied log entries, restricted to (last_applied - W, last_applied].
// r[impl molten.consensus.bounded_request_dedup]
fn validate_completed_requests(state: &ReplicaState) -> crate::error::Result<()> {
    let floor = request_window_floor(state.last_applied, state.profile.request_retention_entries);
    let mut expected = std::collections::BTreeMap::new();
    if let Some(snapshot) = &state.snapshot {
        expected.extend(
            snapshot
                .completed_requests
                .iter()
                .filter(|(_request_ref, index)| **index > floor)
                .map(|(request_ref, index)| (request_ref.as_str(), *index)),
        );
    }
    expected.extend(
        state
            .log
            .iter()
            .filter(|entry| entry.index > floor && entry.index <= state.last_applied)
            .map(|entry| (entry.request_ref.as_str(), entry.index)),
    );
    let is_equal = expected.len() == state.completed_requests.len()
        && expected.iter().all(|(request_ref, index)| state.completed_requests.get(*request_ref) == Some(index));
    if !is_equal {
        return Err(crate::error::Failure::invalid_harness(
            "Raft idempotency state differs from the applied request window",
        ));
    }
    Ok(())
}

fn validate_pending_reads(state: &ReplicaState) -> crate::error::Result<()> {
    if state.pending_reads.len() > MAX_PENDING_REPLICA_READS {
        return Err(crate::error::Failure::invalid_harness("Raft pending reads exceed their static bound"));
    }
    if !state.pending_reads.is_empty() && state.role != ReplicaRole::Leader {
        return Err(crate::error::Failure::invalid_harness("only a Raft leader may retain pending reads"));
    }
    for (request_ref, pending) in &state.pending_reads {
        validate_content_ref(request_ref, "Raft pending read ref")?;
        if request_ref != &pending.request_ref
            || pending.term != state.current_term
            || pending.required_index > state.commit_index
            || !pending.acknowledgements.contains(&state.node_id)
            || pending.acknowledgements.iter().any(|node_id| !state.membership.voters.contains(node_id))
        {
            return Err(crate::error::Failure::invalid_harness("Raft pending read state is inconsistent"));
        }
    }
    Ok(())
}

fn validate_snapshot(state: &ReplicaState) -> crate::error::Result<()> {
    let Some(snapshot) = &state.snapshot else {
        return Ok(());
    };
    for (reference, label) in [
        (&snapshot.snapshot_ref, "Raft snapshot ref"),
        (&snapshot.group_binding_ref, "Raft snapshot group ref"),
        (&snapshot.membership_ref, "Raft snapshot membership ref"),
        (&snapshot.application_state_ref, "Raft snapshot application state ref"),
    ] {
        validate_content_ref(reference, label)?;
    }
    if snapshot.group_binding_ref != state.profile.group_binding_ref
        || snapshot.membership_ref != state.membership.membership_ref
        || snapshot.config_epoch != state.membership.config_epoch
        || snapshot.fencing_epoch != state.profile.fencing_epoch
        || snapshot.snapshot_ref != snapshot_ref(snapshot)?
    {
        return Err(crate::error::Failure::invalid_harness("Raft snapshot binding or identity mismatch"));
    }
    validate_snapshot_requests(snapshot, state.profile.request_retention_entries)?;
    if snapshot.last_included_index == INITIAL_COMMIT_INDEX || snapshot.last_included_index > state.commit_index {
        return Err(crate::error::Failure::invalid_harness(
            "Raft snapshot boundary must be positive and no newer than commit",
        ));
    }
    Ok(())
}

/// A snapshot carries exactly one request ref per retained index, each committed in
/// (last_included_index - W, last_included_index].
// r[impl molten.consensus.bounded_request_dedup]
fn validate_snapshot_requests(snapshot: &ReplicaSnapshot, window: u64) -> crate::error::Result<()> {
    let floor = request_window_floor(snapshot.last_included_index, window);
    let mut indices = std::collections::BTreeSet::new();
    for (request_ref, index) in &snapshot.completed_requests {
        validate_content_ref(request_ref, "Raft snapshot completed request ref")?;
        if *index <= floor || *index > snapshot.last_included_index || !indices.insert(*index) {
            return Err(crate::error::Failure::invalid_harness(
                "Raft snapshot completed request is outside its duplicate window",
            ));
        }
    }
    Ok(())
}

fn validate_election_timer(state: &ReplicaState) -> crate::error::Result<()> {
    if state.election_timer_sequence == 0 {
        return Err(crate::error::Failure::invalid_harness("Raft election timer sequence must be positive"));
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
        return Err(crate::error::Failure::invalid_harness("Raft active election timer ref does not match state"));
    }
    Ok(())
}

pub(super) fn validate(transition: &ReplicaTransition) -> crate::error::Result<()> {
    validate_replica_state(&transition.next)?;
    if transition.effects.len() > transition.next.profile.max_effects_per_step {
        return Err(crate::error::Failure::invalid_harness("Raft transition exceeds its admitted effect bound"));
    }
    Ok(())
}

pub(super) fn ensure_running(state: &ReplicaState) -> crate::error::Result<()> {
    if state.lifecycle != ReplicaLifecycle::Running {
        return Err(crate::error::Failure::invalid_harness("Raft event requires a running replica"));
    }
    Ok(())
}

pub(super) fn validate_content_ref(value: &str, label: &str) -> crate::error::Result<()> {
    crate::preserves_rail::validate_content_ref(value)
        .map_err(|error| crate::error::Failure::invalid_harness(format!("invalid {label}: {error}")))
}

fn validate_incoming_entry<'a>(
    state: &ReplicaState,
    entry: &'a ReplicatedEntry,
    expected_index: u64,
    leader_term: u64,
    request_refs: &mut std::collections::BTreeMap<&'a str, u64>,
) -> crate::error::Result<()> {
    if entry.index != expected_index || entry.term == 0 || entry.term > leader_term {
        return Err(crate::error::Failure::invalid_harness(
            "Raft append entries are not contiguous or use an invalid term",
        ));
    }
    validate_content_ref(&entry.request_ref, "Raft log request ref")?;
    validate_content_ref(&entry.command_ref, "Raft log command ref")?;
    validate_content_ref(&entry.command_schema_ref, "Raft log command schema ref")?;
    let window = state.profile.request_retention_entries;
    if !admit_request_ref(request_refs, entry, window) {
        return Err(crate::error::Failure::invalid_harness(
            "Raft append entries repeat a request ref within its duplicate window",
        ));
    }
    if state.log.iter().any(|existing| {
        existing.request_ref == entry.request_ref
            && existing.index != entry.index
            && existing.index.abs_diff(entry.index) <= window
    }) {
        return Err(crate::error::Failure::invalid_harness(
            "Raft append reuses a request ref within its duplicate window",
        ));
    }
    Ok(())
}

fn validate_log(state: &ReplicaState) -> crate::error::Result<()> {
    let mut expected = match &state.snapshot {
        Some(snapshot) => snapshot
            .last_included_index
            .checked_add(NEXT_LOG_INDEX_STEP)
            .ok_or_else(|| crate::error::Failure::invalid_harness("Raft snapshot log index overflow"))?,
        None => INITIAL_LOG_INDEX,
    };
    let mut request_refs = std::collections::BTreeMap::new();
    let mut previous_term = state.snapshot.as_ref().map_or(INITIAL_TERM, |snapshot| snapshot.last_included_term);
    for entry in &state.log {
        validate_state_entry(entry, expected, previous_term, &mut request_refs, state.profile.request_retention_entries)?;
        previous_term = entry.term;
        expected = expected
            .checked_add(NEXT_LOG_INDEX_STEP)
            .ok_or_else(|| crate::error::Failure::invalid_harness("Raft log index overflow"))?;
    }
    Ok(())
}
