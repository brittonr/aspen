use crate::addressable_actor::*;

pub(super) fn begin_drain(state: &ActorState) -> Result<super::super::PlannedChange, ActorIssue> {
    if state.phase != ActorPhase::Running {
        return Err(super::common::illegal(state.phase, "begin-drain"));
    }
    let mut next = state.clone();
    next.phase = ActorPhase::Draining;
    Ok(super::common::change(ActorTransitionKind::DrainBegin, next, Vec::new()))
}

pub(super) fn drain_succeeded(
    profile: &AddressableActorProfile,
    state: &ActorState,
    checkpoint_ref: &str,
    remaining_items: u32,
) -> Result<super::super::PlannedChange, ActorIssue> {
    if state.phase != ActorPhase::Draining {
        return Err(super::common::illegal(state.phase, "drain-succeeded"));
    }
    if remaining_items != 0 || remaining_items > profile.maximum_drain_items {
        return Err(ActorIssue::DrainNotComplete);
    }
    let mut next = state.clone();
    next.phase = ActorPhase::Stopped;
    next.checkpoint_ref = Some(checkpoint_ref.to_string());
    let effects = vec![
        super::common::draft(ActorEffectIntentKind::PersistCheckpoint, None, Some(checkpoint_ref.to_string())),
        super::common::draft(ActorEffectIntentKind::StopRuntime, None, None),
    ];
    Ok(super::common::change(ActorTransitionKind::DrainComplete, next, effects))
}

pub(super) fn stop(state: &ActorState) -> Result<super::super::PlannedChange, ActorIssue> {
    if state.phase == ActorPhase::Stopped {
        return Err(super::common::illegal(state.phase, "stop"));
    }
    let mut next = state.clone();
    let is_runtime_live = matches!(state.phase, ActorPhase::Starting | ActorPhase::Running | ActorPhase::Draining);
    next.phase = ActorPhase::Stopped;
    next.active_wake_ref = None;
    let effects = if is_runtime_live {
        vec![super::common::draft(ActorEffectIntentKind::StopRuntime, None, None)]
    } else {
        Vec::new()
    };
    Ok(super::common::change(ActorTransitionKind::Stop, next, effects))
}

pub(super) fn degrade(state: &ActorState, failure_ref: &str) -> Result<super::super::PlannedChange, ActorIssue> {
    if state.phase == ActorPhase::Stopped {
        return Err(super::common::illegal(state.phase, "degrade"));
    }
    let mut next = state.clone();
    next.phase = ActorPhase::Degraded;
    next.active_wake_ref = None;
    let effects = vec![super::common::draft(
        ActorEffectIntentKind::NotifyOperator,
        None,
        Some(failure_ref.to_string()),
    )];
    Ok(super::common::change(ActorTransitionKind::Degrade, next, effects))
}
