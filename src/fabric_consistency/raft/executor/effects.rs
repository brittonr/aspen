use super::*;

pub(super) fn requires_durable_intent(effect: &ReplicaEffect) -> bool {
    matches!(
        effect,
        ReplicaEffect::PersistHardState { .. }
            | ReplicaEffect::PersistEntries { .. }
            | ReplicaEffect::FlushLog { .. }
            | ReplicaEffect::PersistCommit { .. }
            | ReplicaEffect::PersistSnapshot { .. }
            | ReplicaEffect::RestoreApplicationSnapshot { .. }
            | ReplicaEffect::ApplyCommitted { .. }
    )
}

pub(super) async fn execute_effect<P: LiveReplicaEffectPorts>(
    ports: &mut P,
    effect: &ReplicaEffect,
    is_intent_required: bool,
) -> crate::error::Result<String> {
    match effect {
        ReplicaEffect::PersistHardState {
            term,
            voted_for,
            is_recovery_required,
        } => ports.persist_hard_state(*term, voted_for.as_deref(), is_intent_required || *is_recovery_required),
        ReplicaEffect::PersistEntries { truncate_from, entries } => ports.persist_entries(*truncate_from, entries),
        ReplicaEffect::FlushLog { through_index } => ports.flush_log(*through_index),
        ReplicaEffect::PersistCommit { through_index } => ports.persist_commit(*through_index),
        ReplicaEffect::PersistSnapshot {
            snapshot,
            is_suffix_retained,
        } => ports.persist_snapshot(snapshot, *is_suffix_retained),
        ReplicaEffect::Send { envelope } => ports.send(envelope).await,
        ReplicaEffect::ArmElectionTimer { timer_ref } => ports.arm_election_timer(timer_ref),
        ReplicaEffect::ArmHeartbeatTimer => ports.arm_heartbeat_timer(),
        ReplicaEffect::RestoreApplicationSnapshot { snapshot } => ports.restore_snapshot(snapshot),
        ReplicaEffect::ApplyCommitted { entries } => ports.apply_committed(entries),
        ReplicaEffect::ProposalOutcome {
            request_ref,
            disposition,
            committed_index,
        } => ports.proposal_outcome(request_ref, *disposition, *committed_index),
        ReplicaEffect::ReadOutcome {
            request_ref,
            mode,
            disposition,
            observed_index,
        } => ports.read_outcome(request_ref, *mode, *disposition, *observed_index),
        ReplicaEffect::LifecycleChanged { lifecycle } => ports.lifecycle_changed(*lifecycle),
    }
}

pub(super) fn effect_kind(effect: &ReplicaEffect) -> ReplicaEffectKind {
    match effect {
        ReplicaEffect::PersistHardState { .. } => ReplicaEffectKind::PersistHardState,
        ReplicaEffect::PersistEntries { .. } => ReplicaEffectKind::PersistEntries,
        ReplicaEffect::FlushLog { .. } => ReplicaEffectKind::FlushLog,
        ReplicaEffect::PersistCommit { .. } => ReplicaEffectKind::PersistCommit,
        ReplicaEffect::PersistSnapshot { .. } => ReplicaEffectKind::PersistSnapshot,
        ReplicaEffect::Send { .. } => ReplicaEffectKind::Send,
        ReplicaEffect::ArmElectionTimer { .. } => ReplicaEffectKind::ArmElectionTimer,
        ReplicaEffect::ArmHeartbeatTimer => ReplicaEffectKind::ArmHeartbeatTimer,
        ReplicaEffect::RestoreApplicationSnapshot { .. } => ReplicaEffectKind::RestoreApplicationSnapshot,
        ReplicaEffect::ApplyCommitted { .. } => ReplicaEffectKind::ApplyCommitted,
        ReplicaEffect::ProposalOutcome { .. } => ReplicaEffectKind::ProposalOutcome,
        ReplicaEffect::ReadOutcome { .. } => ReplicaEffectKind::ReadOutcome,
        ReplicaEffect::LifecycleChanged { .. } => ReplicaEffectKind::LifecycleChanged,
    }
}

#[cfg(test)]
#[test]
fn transport_failure_diagnostic_is_bounded_at_a_utf8_boundary() {
    let error = crate::error::Failure::invalid_harness("界".repeat(512));
    let diagnostic = bounded_transport_diagnostic(&error);
    assert!(diagnostic.len() <= 512);
    assert!(error.to_string().starts_with(&diagnostic));
    assert!(diagnostic.ends_with('界'));
}
pub(super) fn bounded_transport_diagnostic(error: &crate::error::Failure) -> String {
    const MAX_BYTES: usize = 512;
    let mut diagnostic = error.to_string();
    let mut end = diagnostic.len().min(MAX_BYTES);
    while !diagnostic.is_char_boundary(end) {
        end -= 1;
    }
    diagnostic.truncate(end);
    diagnostic
}

pub(super) fn quarantine(state: &ReplicaState) -> ReplicaState {
    let mut retained = state.clone();
    retained.role = ReplicaRole::Follower;
    retained.lifecycle = ReplicaLifecycle::Stopped;
    retained.leader_id = None;
    retained.votes_received.clear();
    retained.pending_reads.clear();
    retained.next_index.clear();
    retained.match_index.clear();
    retained.quorum_confirmed_term = None;
    retained.rejoin = ReplicaRejoin::Recovering(RecoveryRound {
        round_ref: retained.active_election_timer_ref.clone(),
        fence_term: None,
        peers: std::collections::BTreeMap::new(),
    });
    retained
}
