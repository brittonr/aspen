pub(super) mod ingress;
pub(super) mod sequence;

use super::*;

pub(super) fn validate_replica_state(
    state: &ReplicaState,
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<()> {
    if !matches!(state.rejoin, ReplicaRejoin::Voter) && state.role != ReplicaRole::Follower {
        return Err(crate::denial::ReplicaError::Denied("recovering Raft participant cannot campaign or lead"));
    }
    if state.membership.voters.len() != STATIC_VOTER_COUNT || !state.membership.voters.contains(&state.node_id) {
        return Err(crate::denial::ReplicaError::Denied("Raft state has invalid static membership"));
    }
    if state.log.len() > state.profile.max_log_entries {
        return Err(crate::denial::ReplicaError::Denied("Raft state exceeds its admitted log bound"));
    }
    validate_snapshot(state, identity)?;
    sequence::validate_log(state)?;
    let last = support::last_log_index(state);
    if state.last_applied > state.commit_index || state.commit_index > last {
        return Err(crate::denial::ReplicaError::Denied("Raft applied or committed index exceeds the local log"));
    }
    sequence::validate_role(state)?;
    validate_completed_requests(state)?;
    validate_pending_reads(state)?;
    validate_election_timer(state, identity)?;
    if state.voted_for.as_ref().is_some_and(|voter| !state.membership.voters.contains(voter)) {
        return Err(crate::denial::ReplicaError::Denied("Raft vote target is outside admitted membership"));
    }
    Ok(())
}

/// The idempotency map must equal the deterministic window of the applied prefix: the snapshot's
/// retained refs plus the applied log entries, restricted to (last_applied - W, last_applied].
// r[impl molten.consensus.bounded_request_dedup]
fn validate_completed_requests(state: &ReplicaState) -> crate::denial::Result<()> {
    let floor = request_window_floor(state.last_applied, state.profile.request_retention_entries);
    let mut expected = alloc::collections::BTreeMap::new();
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
        && expected
            .iter()
            .all(|(request_ref, index)| state.completed_requests.get(*request_ref) == Some(index));
    if !is_equal {
        return Err(crate::denial::ReplicaError::Denied(
            "Raft idempotency state differs from the applied request window",
        ));
    }
    Ok(())
}

fn validate_pending_reads(state: &ReplicaState) -> crate::denial::Result<()> {
    if state.pending_reads.len() > MAX_PENDING_REPLICA_READS {
        return Err(crate::denial::ReplicaError::Denied("Raft pending reads exceed their static bound"));
    }
    if !state.pending_reads.is_empty() && state.role != ReplicaRole::Leader {
        return Err(crate::denial::ReplicaError::Denied("only a Raft leader may retain pending reads"));
    }
    for (request_ref, pending) in &state.pending_reads {
        validate_content_ref(request_ref, "Raft pending read ref")?;
        if request_ref != &pending.request_ref
            || pending.term != state.current_term
            || pending.required_index > state.commit_index
            || !pending.acknowledgements.contains(&state.node_id)
            || pending.acknowledgements.iter().any(|node_id| !state.membership.voters.contains(node_id))
        {
            return Err(crate::denial::ReplicaError::Denied("Raft pending read state is inconsistent"));
        }
    }
    Ok(())
}

fn validate_snapshot(
    state: &ReplicaState,
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<()> {
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
        || snapshot.snapshot_ref != support::snapshot_ref(snapshot, identity)?
    {
        return Err(crate::denial::ReplicaError::Denied("Raft snapshot binding or identity mismatch"));
    }
    validate_snapshot_requests(snapshot, state.profile.request_retention_entries)?;
    if snapshot.last_included_index == INITIAL_COMMIT_INDEX || snapshot.last_included_index > state.commit_index {
        return Err(crate::denial::ReplicaError::Denied(
            "Raft snapshot boundary must be positive and no newer than commit",
        ));
    }
    Ok(())
}

/// A snapshot carries exactly one request ref per retained index, each committed in
/// (last_included_index - W, last_included_index].
// r[impl molten.consensus.bounded_request_dedup]
fn validate_snapshot_requests(snapshot: &ReplicaSnapshot, window: u64) -> crate::denial::Result<()> {
    let floor = request_window_floor(snapshot.last_included_index, window);
    let mut indices = alloc::collections::BTreeSet::new();
    for (request_ref, index) in &snapshot.completed_requests {
        validate_content_ref(request_ref, "Raft snapshot completed request ref")?;
        if *index <= floor || *index > snapshot.last_included_index || !indices.insert(*index) {
            return Err(crate::denial::ReplicaError::Denied(
                "Raft snapshot completed request is outside its duplicate window",
            ));
        }
    }
    Ok(())
}

fn validate_election_timer(
    state: &ReplicaState,
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<()> {
    if state.election_timer_sequence == 0 {
        return Err(crate::denial::ReplicaError::Denied("Raft election timer sequence must be positive"));
    }
    validate_content_ref(&state.active_election_timer_ref, "Raft active election timer ref")?;
    if state.active_election_timer_ref != support::election_timer_ref(state, identity)? {
        return Err(crate::denial::ReplicaError::Denied("Raft active election timer ref does not match state"));
    }
    Ok(())
}

pub(super) fn validate(
    transition: &ReplicaTransition,
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<()> {
    validate_replica_state(&transition.next, identity)?;
    if transition.effects.len() > transition.next.profile.max_effects_per_step {
        return Err(crate::denial::ReplicaError::Denied("Raft transition exceeds its admitted effect bound"));
    }
    Ok(())
}

pub(super) fn ensure_running(state: &ReplicaState) -> crate::denial::Result<()> {
    if state.lifecycle != ReplicaLifecycle::Running {
        return Err(crate::denial::ReplicaError::Denied("Raft event requires a running replica"));
    }
    Ok(())
}

pub(super) fn validate_content_ref(value: &str, label: &'static str) -> crate::denial::Result<()> {
    crate::denial::refs::validate_labelled_content_ref(value, label)
}
