use crate::addressable_actor::*;

pub(super) struct RecoverySuccessInput<'a> {
    pub checkpoint_ref: &'a str,
    pub restored_classes: &'a [SurvivalClass],
    pub durable_state_ref: &'a str,
}

pub(super) fn begin_recovery(
    state: &ActorState,
    checkpoint_ref: &str,
) -> Result<super::super::PlannedChange, ActorIssue> {
    if !matches!(state.phase, ActorPhase::Dormant | ActorPhase::Degraded) {
        return Err(super::common::illegal(state.phase, "begin-recovery"));
    }
    if state.checkpoint_ref.as_ref().is_some_and(|stored| stored != checkpoint_ref) {
        return Err(ActorIssue::MissingCheckpoint);
    }
    let mut next = state.clone();
    next.phase = ActorPhase::Recovering;
    next.checkpoint_ref = Some(checkpoint_ref.to_string());
    let effects = vec![super::common::draft(
        ActorEffectIntentKind::RestoreCheckpoint,
        None,
        Some(checkpoint_ref.to_string()),
    )];
    Ok(super::common::change(ActorTransitionKind::RecoveryBegin, next, effects))
}

pub(super) fn recovery_succeeded(
    profile: &AddressableActorProfile,
    state: &ActorState,
    request: &ActorRequest,
    input: RecoverySuccessInput<'_>,
) -> Result<super::super::PlannedChange, ActorIssue> {
    if state.phase != ActorPhase::Recovering {
        return Err(super::common::illegal(state.phase, "recovery-succeeded"));
    }
    if state.checkpoint_ref.as_deref() != Some(input.checkpoint_ref) {
        return Err(ActorIssue::MissingCheckpoint);
    }
    if let Some(issue) = validate_restore_classes(&profile.survival, input.restored_classes).into_iter().next() {
        return Err(issue);
    }
    let mut next = state.clone();
    next.phase = ActorPhase::Running;
    next.durable_state_ref = Some(input.durable_state_ref.to_string());
    next.last_activity_tick = request.logical_tick;
    let effects = vec![super::common::draft(ActorEffectIntentKind::StartRuntime, None, None)];
    Ok(super::super::PlannedChange {
        kind: ActorTransitionKind::RecoveryComplete,
        next,
        effects,
        restored_classes: input.restored_classes.to_vec(),
    })
}

pub(super) fn recovery_failed(
    state: &ActorState,
    failure_ref: &str,
) -> Result<super::super::PlannedChange, ActorIssue> {
    if state.phase != ActorPhase::Recovering {
        return Err(super::common::illegal(state.phase, "recovery-failed"));
    }
    let mut next = state.clone();
    next.phase = ActorPhase::Degraded;
    let effects = vec![super::common::draft(
        ActorEffectIntentKind::NotifyOperator,
        None,
        Some(failure_ref.to_string()),
    )];
    Ok(super::common::change(ActorTransitionKind::RecoveryFailed, next, effects))
}
