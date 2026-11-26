mod pages;
mod repair;

use super::*;

// r[impl molten.consensus.repair_admission]
pub(super) fn retry(
    state: &ReplicaState,
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<ReplicaTransition> {
    let mut next = state.clone();
    let timer = support::arm_election_timer(&mut next, identity)?;
    let ReplicaRejoin::Recovering(round) = &mut next.rejoin else {
        return Err(crate::denial::ReplicaError::Denied("recovery retry requires a round"));
    };
    if round.peers.len() == STATIC_VOTER_COUNT - 1 && round.peers.values().all(|peer| peer.is_complete) {
        round.round_ref = next.active_election_timer_ref.clone();
        round.peers.clear();
    }
    let mut effects = Vec::with_capacity(STATIC_VOTER_COUNT);
    for peer in support::peers(&next) {
        let ReplicaRejoin::Recovering(round) = &next.rejoin else {
            return Err(crate::denial::ReplicaError::Denied("recovery retry lost its round"));
        };
        let from_index = match round.peers.get(&peer) {
            Some(observation) if observation.is_complete => continue,
            Some(observation) => continuation_index(observation)?,
            None => INITIAL_LOG_INDEX,
        };
        effects.push(query(&next, peer, from_index)?);
    }
    effects.push(timer);
    let mut transition = MessagePlan {
        next,
        effects,
        persist_hard_state: false,
        identity,
    };
    if matches!(&transition.next.rejoin, ReplicaRejoin::Recovering(round)
        if round.peers.values().any(|peer| peer.is_complete))
    {
        repair::reconcile(&mut transition)?;
    }
    finish_message(transition)
}

/// The first index a partial observation still lacks.
fn continuation_index(observation: &RecoveryObservation) -> crate::denial::Result<u64> {
    observation
        .entries
        .last()
        .map_or_else(
            || observation.snapshot.as_ref().map_or(0, |snapshot| snapshot.last_included_index),
            |entry| entry.index,
        )
        .checked_add(1)
        .ok_or(crate::denial::ReplicaError::Overflow("recovery continuation overflow"))
}

fn query(state: &ReplicaState, peer: String, from_index: u64) -> crate::denial::Result<ReplicaEffect> {
    let ReplicaRejoin::Recovering(round) = &state.rejoin else {
        return Err(crate::denial::ReplicaError::Denied("recovery query requires an active round"));
    };
    Ok(support::send_effect(state, peer, Message::RecoveryQuery {
        term: state.current_term,
        requester_id: state.node_id.clone(),
        is_barrier: round.fence_term.is_some(),
        round_ref: round.round_ref.clone(),
        from_index,
        config_epoch: state.membership.config_epoch,
        fencing_epoch: state.profile.fencing_epoch,
    }))
}

pub(super) fn dispatch(transition: &mut MessagePlan<'_>, from: String, message: Message) -> crate::denial::Result<()> {
    match message {
        Message::RecoveryQuery {
            term,
            round_ref,
            from_index,
            is_barrier,
            ..
        } => {
            if is_barrier && term == 0 {
                return Err(crate::denial::ReplicaError::Denied("recovery barrier term must be positive"));
            }
            let should_join = matches!(&transition.next.rejoin,
                ReplicaRejoin::Recovering(round) if round.fence_term != Some(term));
            if is_barrier && term == transition.next.current_term && should_join {
                join_barrier(transition, term)?;
            }
            report(transition, from, round_ref, from_index)
        }
        message @ Message::RecoveryReport { .. } => receive(transition, from, message),
        Message::RequestVote { .. }
        | Message::VoteResponse { .. }
        | Message::AppendEntries { .. }
        | Message::AppendResponse { .. }
        | Message::ReadProbe { .. }
        | Message::ReadAcknowledgement { .. }
        | Message::InstallSnapshot { .. }
        | Message::SnapshotResponse { .. } => {
            Err(crate::denial::ReplicaError::Denied("recovery dispatch admitted an ordinary Raft message"))
        }
    }
}

/// The identity of a recovery corpus, refused when its log exceeds the admitted bound.
fn corpus_ref(
    identity: &dyn crate::identity::ReplicaIdentity,
    corpus: &crate::identity::RecoveryCorpus<'_>,
) -> crate::denial::Result<String> {
    if corpus.log.len() > MAX_REPLICA_LOG_ENTRIES {
        return Err(crate::denial::ReplicaError::Denied("recovery observation log exceeds the bound"));
    }
    Ok(identity.recovery_corpus_ref(corpus)?)
}

fn report(
    transition: &mut MessagePlan<'_>,
    to: String,
    round_ref: String,
    from_index: u64,
) -> crate::denial::Result<()> {
    crate::denial::refs::validate_content_ref(&round_ref)?;
    if from_index == 0 {
        return Err(crate::denial::ReplicaError::Denied("recovery query index must be positive"));
    }
    let state = &transition.next;
    let observation_ref = corpus_ref(transition.identity, &crate::identity::RecoveryCorpus {
        term: state.current_term,
        commit_index: state.commit_index,
        log: &state.log,
        snapshot: state.snapshot.as_ref(),
    })?;
    let entries = state
        .log
        .iter()
        .filter(|entry| entry.index >= from_index)
        .take(MAX_REPLICA_MESSAGE_ENTRIES)
        .cloned()
        .collect();
    transition.effects.push(support::send_effect(state, to, Message::RecoveryReport {
        term: state.current_term,
        peer_id: state.node_id.clone(),
        round_ref,
        observation_ref,
        from_index,
        last_log_index: support::last_log_index(state),
        commit_index: state.commit_index,
        entries,
        snapshot: state.snapshot.clone().map(Box::new),
        config_epoch: state.membership.config_epoch,
        fencing_epoch: state.profile.fencing_epoch,
    }));
    Ok(())
}

fn receive(transition: &mut MessagePlan<'_>, from: String, message: Message) -> crate::denial::Result<()> {
    // Typed simulation ingress must pass the same validation as live wire ingress.
    crate::transition::wire::validate_envelope(
        &ReplicaMessageEnvelope {
            group_binding_ref: transition.next.profile.group_binding_ref.clone(),
            service_generation: transition.next.profile.service_generation,
            from: from.clone(),
            to: transition.next.node_id.clone(),
            message: message.clone(),
        },
        transition.identity,
    )?;
    let Message::RecoveryReport {
        term,
        round_ref,
        observation_ref,
        from_index,
        last_log_index,
        commit_index,
        entries,
        snapshot,
        ..
    } = message
    else {
        return Err(crate::denial::ReplicaError::Denied("expected a recovery report"));
    };
    if let Some(snapshot) = &snapshot
        && snapshot.membership_ref != transition.next.membership.membership_ref
    {
        return Err(crate::denial::ReplicaError::Denied("recovery snapshot substituted membership"));
    }
    let identity = transition.identity;
    let ReplicaRejoin::Recovering(round) = &mut transition.next.rejoin else {
        return Ok(());
    };
    if round.round_ref != round_ref {
        return Ok(());
    }
    if from_index == INITIAL_LOG_INDEX {
        round.peers.insert(from.clone(), RecoveryObservation {
            observation_ref: observation_ref.clone(),
            current_term: term,
            commit_index,
            last_log_index,
            entries: Vec::with_capacity(MAX_REPLICA_LOG_ENTRIES),
            snapshot: snapshot.clone(),
            is_complete: false,
        });
    }
    let Some(observation) = round.peers.get_mut(&from) else {
        return Ok(());
    };
    if observation.observation_ref != observation_ref
        || observation.current_term != term
        || observation.commit_index != commit_index
        || observation.last_log_index != last_log_index
        || observation.snapshot != snapshot
    {
        round.peers.remove(&from);
        transition.effects.push(query(&transition.next, from, INITIAL_LOG_INDEX)?);
        return Ok(());
    }
    match pages::append(observation, from_index, entries, identity)? {
        pages::Progress::Ignored => Ok(()),
        pages::Progress::More(next_index) => {
            transition.effects.push(query(&transition.next, from, next_index)?);
            Ok(())
        }
        pages::Progress::Complete => repair::reconcile(transition),
    }
}

fn establish_barrier(transition: &mut MessagePlan<'_>) -> crate::denial::Result<()> {
    let term = transition
        .next
        .current_term
        .checked_add(NEXT_TERM_STEP)
        .ok_or(crate::denial::ReplicaError::Overflow("recovery fence term overflow"))?;
    join_barrier(transition, term)
}

fn join_barrier(transition: &mut MessagePlan<'_>, term: u64) -> crate::denial::Result<()> {
    let round_ref = transition.identity.recovery_barrier_ref(&transition.next.active_election_timer_ref, term)?;
    transition.next.current_term = term;
    transition.next.voted_for = Some(transition.next.node_id.clone());
    transition.next.rejoin = ReplicaRejoin::Recovering(RecoveryRound {
        round_ref,
        fence_term: Some(term),
        peers: alloc::collections::BTreeMap::new(),
    });
    transition.persist_hard_state = true;
    for peer in support::peers(&transition.next) {
        transition.effects.push(query(&transition.next, peer, INITIAL_LOG_INDEX)?);
    }
    let timer = support::arm_election_timer(&mut transition.next, transition.identity)?;
    transition.effects.push(timer);
    Ok(())
}
