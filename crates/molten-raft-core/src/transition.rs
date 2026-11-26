//! The deterministic replica transition function.
//!
//! [`apply_replica_event`] validates the state, plans one step, and validates the planned state and
//! its effect bound. Identical inputs and identity port always yield identical next states and
//! effect plans; a refused step plans nothing.

mod client;
mod election;
mod message;
mod rejoin;
mod support;
mod validation;
pub mod wire;

pub use election::StaleTimerEvent;
pub use election::stale_timer_event;

use crate::model::*;

/// A message step under construction: the state it mutates, the effects it plans, and whether the
/// term or vote changed so the hard state must be persisted first.
pub(crate) struct MessagePlan<'a> {
    pub next: ReplicaState,
    pub effects: Vec<ReplicaEffect>,
    pub persist_hard_state: bool,
    pub identity: &'a dyn crate::identity::ReplicaIdentity,
}

// r[impl molten.fabric_consistency.live_raft]
// r[impl molten.fabric_consistency.group_isolation]
pub fn apply_replica_event(
    state: &ReplicaState,
    event: ReplicaEvent,
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<ReplicaTransition> {
    validation::validate_replica_state(state, identity)?;
    let transition = match event {
        ReplicaEvent::ElectionTimeout { timer_ref } => election::handle_timeout(state, timer_ref, identity)?,
        ReplicaEvent::HeartbeatTimeout => election::handle_heartbeat_timeout(state)?,
        ReplicaEvent::Message { envelope } => handle_message(state, envelope, identity)?,
        ReplicaEvent::Propose {
            request_ref,
            command_ref,
            command_schema_ref,
        } => client::handle_proposal(state, request_ref, command_ref, command_schema_ref)?,
        ReplicaEvent::Read { request_ref, mode } => client::handle_read(state, request_ref, mode)?,
        ReplicaEvent::CreateSnapshot { application_state_ref } => {
            client::handle_create_snapshot(state, application_state_ref, identity)?
        }
        ReplicaEvent::BeginDrain => client::handle_begin_drain(state)?,
        ReplicaEvent::Stop => client::handle_stop(state)?,
    };
    validation::validate(&transition, identity)?;
    Ok(transition)
}

fn handle_message(
    state: &ReplicaState,
    envelope: ReplicaMessageEnvelope,
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<ReplicaTransition> {
    validation::ensure_running(state)?;
    validation::ingress::validate_message_envelope(state, &envelope)?;
    let message = envelope.message;
    let mut transition = MessagePlan {
        next: state.clone(),
        effects: Vec::new(),
        persist_hard_state: false,
        identity,
    };
    observe_higher_term(&mut transition, message.term());
    if let ReplicaRejoin::VoteFenced { through_term } = transition.next.rejoin
        && transition.next.current_term > through_term
    {
        transition.next.rejoin = ReplicaRejoin::Voter;
    }
    if !matches!(transition.next.rejoin, ReplicaRejoin::Voter)
        && !matches!(
            message,
            Message::RecoveryQuery { .. } | Message::RecoveryReport { .. } | Message::RequestVote { .. }
        )
    {
        return finish_message(transition);
    }
    message::dispatch(&mut transition, envelope.from, message)?;
    finish_message(transition)
}

fn observe_higher_term(transition: &mut MessagePlan<'_>, term: u64) {
    if term <= transition.next.current_term {
        return;
    }
    transition.next.current_term = term;
    transition.next.voted_for = None;
    transition.next.role = ReplicaRole::Follower;
    transition.next.leader_id = None;
    transition.next.votes_received.clear();
    transition.next.next_index.clear();
    transition.next.match_index.clear();
    transition.next.pending_reads.clear();
    transition.next.quorum_confirmed_term = None;
    if let ReplicaRejoin::Recovering(round) = &mut transition.next.rejoin
        && round.fence_term.is_some()
    {
        round.fence_term = None;
        round.peers.clear();
    }
    transition.persist_hard_state = true;
}

fn finish_message(mut transition: MessagePlan<'_>) -> crate::denial::Result<ReplicaTransition> {
    let current_timer_ref = support::election_timer_ref(&transition.next, transition.identity)?;
    if transition.next.active_election_timer_ref != current_timer_ref {
        let timer_effect = support::arm_election_timer(&mut transition.next, transition.identity)?;
        transition.effects.push(timer_effect);
    }
    if transition.persist_hard_state {
        transition.effects.insert(0, ReplicaEffect::PersistHardState {
            term: transition.next.current_term,
            voted_for: transition.next.voted_for.clone(),
            is_recovery_required: matches!(transition.next.rejoin, ReplicaRejoin::Recovering(_)),
        });
    }
    finish(transition.next, transition.effects)
}

pub(crate) fn finish(next: ReplicaState, effects: Vec<ReplicaEffect>) -> crate::denial::Result<ReplicaTransition> {
    Ok(ReplicaTransition { next, effects })
}

/// Validate a state recovered from durable storage before the replica runs on it.
pub fn validate_recovered_replica_state(
    state: &ReplicaState,
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<()> {
    validation::validate_replica_state(state, identity)
}

/// The highest applied index outside the duplicate-request window ending at `applied`.
pub(crate) const fn request_window_floor(applied: u64, window: u64) -> u64 {
    applied.saturating_sub(window)
}

/// Record newly applied requests, then keep exactly the refs committed in (last_applied - W,
/// last_applied]. The result depends only on the applied log prefix, never on local snapshot
/// timing.
// r[impl molten.consensus.bounded_request_dedup]
pub fn record_completed_requests<'a>(state: &mut ReplicaState, entries: impl IntoIterator<Item = &'a ReplicatedEntry>) {
    for entry in entries {
        state.completed_requests.insert(entry.request_ref.clone(), entry.index);
    }
    let floor = request_window_floor(state.last_applied, state.profile.request_retention_entries);
    state.completed_requests.retain(|_request_ref, index| *index > floor);
}

/// Whether a running voter retains at least its profile's snapshot threshold of applied entries, so
/// its hosting service should issue `ReplicaEvent::CreateSnapshot`.
// r[impl molten.consensus.snapshot_threshold]
pub fn is_snapshot_due(state: &ReplicaState) -> bool {
    let boundary = state.snapshot.as_ref().map_or(INITIAL_COMMIT_INDEX, |snapshot| snapshot.last_included_index);
    let retained_applied = state.last_applied.saturating_sub(boundary);
    state.lifecycle == ReplicaLifecycle::Running
        && matches!(state.rejoin, ReplicaRejoin::Voter)
        && u64::try_from(state.profile.snapshot_threshold_entries).is_ok_and(|threshold| retained_applied >= threshold)
}
