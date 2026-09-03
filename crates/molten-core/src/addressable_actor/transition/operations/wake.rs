use crate::addressable_actor::*;

pub(super) struct SleepInput<'a> {
    pub checkpoint_ref: &'a str,
    pub pending_mailbox_items: u32,
    pub unresolved_effects: u32,
}

pub(super) fn wake(
    state: &ActorState,
    request: &ActorRequest,
    reason: &WakeReason,
) -> Result<super::super::PlannedChange, ActorIssue> {
    let wake_ref = identify_actor_wake(reason);
    let mut next = state.clone();
    next.last_activity_tick = request.logical_tick;
    let mut effects = Vec::with_capacity(MAX_ACTOR_EFFECTS_PER_TRANSITION);
    let kind = match state.phase {
        ActorPhase::Dormant => {
            next.phase = ActorPhase::Starting;
            next.active_wake_ref = Some(wake_ref.clone());
            if let Some(checkpoint_ref) = &state.checkpoint_ref {
                effects.push(super::common::draft(
                    ActorEffectIntentKind::RestoreCheckpoint,
                    Some(wake_ref.clone()),
                    Some(checkpoint_ref.clone()),
                ));
            }
            effects.push(super::common::draft(ActorEffectIntentKind::StartRuntime, Some(wake_ref.clone()), None));
            effects.push(super::common::wake_effect(reason, &wake_ref));
            ActorTransitionKind::WakeStart
        }
        ActorPhase::Running => {
            next.active_wake_ref = None;
            effects.push(super::common::wake_effect(reason, &wake_ref));
            ActorTransitionKind::WakeDispatch
        }
        ActorPhase::Starting if state.active_wake_ref.as_deref() == Some(wake_ref.as_str()) => {
            return Err(ActorIssue::ActiveWake);
        }
        phase => return Err(super::common::illegal(phase, "wake")),
    };
    Ok(super::common::change(kind, next, effects))
}

pub(super) fn start_succeeded(
    state: &ActorState,
    request: &ActorRequest,
    wake_ref: &str,
) -> Result<super::super::PlannedChange, ActorIssue> {
    if state.phase != ActorPhase::Starting {
        return Err(super::common::illegal(state.phase, "start-succeeded"));
    }
    if state.active_wake_ref.as_deref() != Some(wake_ref) {
        return Err(ActorIssue::WakeMismatch);
    }
    let mut next = state.clone();
    next.phase = ActorPhase::Running;
    next.active_wake_ref = None;
    next.last_activity_tick = request.logical_tick;
    Ok(super::common::change(ActorTransitionKind::StartComplete, next, Vec::new()))
}

pub(super) fn idle_sleep(
    profile: &AddressableActorProfile,
    state: &ActorState,
    request: &ActorRequest,
    input: SleepInput<'_>,
) -> Result<super::super::PlannedChange, ActorIssue> {
    if state.phase != ActorPhase::Running {
        return Err(super::common::illegal(state.phase, "idle-sleep"));
    }
    if input.pending_mailbox_items != 0 {
        return Err(ActorIssue::PendingMailboxItems);
    }
    if input.unresolved_effects != 0 {
        return Err(ActorIssue::UnresolvedEffects);
    }
    let idle_deadline =
        state.last_activity_tick.checked_add(profile.idle_after_ticks).ok_or(ActorIssue::CounterOverflow)?;
    if request.logical_tick < idle_deadline {
        return Err(ActorIssue::IdleThresholdNotReached);
    }
    let mut next = state.clone();
    next.phase = ActorPhase::Dormant;
    next.checkpoint_ref = Some(input.checkpoint_ref.to_string());
    next.active_wake_ref = None;
    let effects = vec![
        super::common::draft(ActorEffectIntentKind::PersistCheckpoint, None, Some(input.checkpoint_ref.to_string())),
        super::common::draft(ActorEffectIntentKind::StopRuntime, None, None),
    ];
    Ok(super::common::change(ActorTransitionKind::Sleep, next, effects))
}
