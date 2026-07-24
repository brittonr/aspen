mod client;
mod election;
mod message;
mod read;
mod replication;
mod snapshot;
mod support;
mod validation;

use super::*;
use crate::error::Result;

pub(super) const STATIC_QUORUM_COUNT: usize = (STATIC_VOTER_COUNT / 2) + 1;

pub(super) struct MessageTransition {
    pub next: ReplicaState,
    pub effects: Vec<ReplicaEffect>,
    pub persist_hard_state: bool,
}

// r[impl molten.fabric_consistency.live_raft]
// r[impl molten.fabric_consistency.group_isolation]
pub fn apply_replica_event(state: &ReplicaState, event: ReplicaEvent) -> Result<ReplicaTransition> {
    validation::validate_replica_state(state)?;
    let transition = match event {
        ReplicaEvent::ElectionTimeout { timer_ref } => election::handle_election_timeout(state, timer_ref)?,
        ReplicaEvent::HeartbeatTimeout => election::handle_heartbeat_timeout(state)?,
        ReplicaEvent::Message { envelope } => handle_message(state, envelope)?,
        ReplicaEvent::Propose {
            request_ref,
            command_ref,
            command_schema_ref,
        } => client::handle_proposal(state, request_ref, command_ref, command_schema_ref)?,
        ReplicaEvent::Read { request_ref, mode } => client::handle_read(state, request_ref, mode)?,
        ReplicaEvent::CreateSnapshot { application_state_ref } => {
            client::handle_create_snapshot(state, application_state_ref)?
        }
        ReplicaEvent::BeginDrain => client::handle_begin_drain(state)?,
        ReplicaEvent::Stop => client::handle_stop(state)?,
    };
    validation::validate_transition(&transition)?;
    Ok(transition)
}

fn handle_message(state: &ReplicaState, envelope: ReplicaMessageEnvelope) -> Result<ReplicaTransition> {
    validation::ensure_running(state)?;
    validation::validate_message_envelope(state, &envelope)?;
    let message = envelope.message;
    let mut transition = MessageTransition {
        next: state.clone(),
        effects: Vec::new(),
        persist_hard_state: false,
    };
    observe_higher_term(&mut transition, message.term());
    message::dispatch(&mut transition, envelope.from, message)?;
    finish_message_transition(transition)
}

fn observe_higher_term(transition: &mut MessageTransition, term: u64) {
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
    transition.persist_hard_state = true;
}

fn finish_message_transition(mut transition: MessageTransition) -> Result<ReplicaTransition> {
    let current_timer_ref = election_timer_ref(
        &transition.next.profile.group_binding_ref,
        &transition.next.node_id,
        transition.next.profile.service_generation,
        transition.next.current_term,
        transition.next.election_timer_sequence,
    )?;
    if transition.next.active_election_timer_ref != current_timer_ref {
        let timer_effect = support::arm_election_timer(&mut transition.next)?;
        transition.effects.push(timer_effect);
    }
    if transition.persist_hard_state {
        transition.effects.insert(0, ReplicaEffect::PersistHardState {
            term: transition.next.current_term,
            voted_for: transition.next.voted_for.clone(),
        });
    }
    finish_transition(transition.next, transition.effects)
}

pub(super) fn finish_transition(next: ReplicaState, effects: Vec<ReplicaEffect>) -> Result<ReplicaTransition> {
    Ok(ReplicaTransition { next, effects })
}

pub(super) fn validate_recovered_replica_state(state: &ReplicaState) -> Result<()> {
    validation::validate_replica_state(state)
}
