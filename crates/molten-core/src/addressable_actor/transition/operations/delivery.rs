use crate::addressable_actor::*;

pub(super) struct CompletionInput<'a> {
    pub delivery_item_ref: &'a str,
    pub delivery_token_ref: &'a str,
    pub semantic_event_ref: &'a str,
    pub semantic_commit_ref: &'a str,
}

pub(super) fn complete_delivery(
    state: &ActorState,
    request: &ActorRequest,
    input: CompletionInput<'_>,
) -> Result<super::super::PlannedChange, ActorIssue> {
    if state.phase != ActorPhase::Running {
        return Err(super::common::illegal(state.phase, "complete-delivery"));
    }
    if input.semantic_commit_ref.is_empty() {
        return Err(ActorIssue::DeliveryCommitRequired);
    }
    if state.completed_event_refs.len() >= MAX_ACTOR_COMPLETED_EVENTS {
        return Err(ActorIssue::CompletedEventCapacityExceeded);
    }
    let mut next = state.clone();
    next.completed_event_refs.push(input.semantic_event_ref.to_string());
    next.completed_event_refs.sort();
    next.completed_event_refs.dedup();
    next.mailbox_revision = next
        .mailbox_revision
        .checked_add(ADDRESSABLE_ACTOR_REVISION_INCREMENT)
        .ok_or(ActorIssue::CounterOverflow)?;
    next.last_activity_tick = request.logical_tick;
    let effects = vec![super::common::draft(
        ActorEffectIntentKind::AcknowledgeDelivery,
        None,
        Some(input.delivery_token_ref.to_string()),
    )];
    let _ = input.delivery_item_ref;
    Ok(super::common::change(ActorTransitionKind::DeliveryComplete, next, effects))
}

pub(super) fn record_unknown_effect(
    state: &ActorState,
    effect_ref: &str,
) -> Result<super::super::PlannedChange, ActorIssue> {
    if state.phase == ActorPhase::Stopped {
        return Err(super::common::illegal(state.phase, "record-unknown-effect"));
    }
    let mut next = state.clone();
    next.phase = ActorPhase::Degraded;
    next.active_wake_ref = None;
    next.unknown_effect_ref = Some(effect_ref.to_string());
    Ok(super::common::change(ActorTransitionKind::UnknownEffect, next, Vec::new()))
}

pub(super) fn resolve_unknown_effect(
    state: &ActorState,
    effect_ref: &str,
    resolution_ref: &str,
    checkpoint_ref: &str,
) -> Result<super::super::PlannedChange, ActorIssue> {
    if state.phase != ActorPhase::Degraded || state.unknown_effect_ref.as_deref() != Some(effect_ref) {
        return Err(ActorIssue::UnknownEffectMismatch);
    }
    let mut next = state.clone();
    next.phase = ActorPhase::Recovering;
    next.unknown_effect_ref = None;
    next.checkpoint_ref = Some(checkpoint_ref.to_string());
    let effects = vec![
        super::common::draft(ActorEffectIntentKind::NotifyOperator, None, Some(resolution_ref.to_string())),
        super::common::draft(ActorEffectIntentKind::RestoreCheckpoint, None, Some(checkpoint_ref.to_string())),
    ];
    Ok(super::common::change(ActorTransitionKind::UnknownEffectResolved, next, effects))
}
