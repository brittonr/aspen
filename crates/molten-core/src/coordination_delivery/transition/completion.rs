use super::support::*;
use super::*;

pub(super) fn acknowledge(
    input: &DeliveryTransitionInput<'_>,
    next: &mut DeliveryState,
    token: &DeliveryToken,
) -> Result<AppliedTransition, DeliveryIssue> {
    let active = current_active(input, next, token, true, true)?;
    let item_ref = active.item.item_ref.clone();
    next.in_flight.remove(&item_ref);
    append_attempt(next, token, input.request, "acknowledged")?;
    let total_attempts = attempt_count(next, &item_ref)?;
    next.completed.insert(item_ref.clone(), CompletedDelivery {
        item: active.item,
        delivery_id: token.delivery_id.clone(),
        acknowledged_at_tick: input.request.logical_tick,
        total_attempts,
    });
    Ok(AppliedTransition {
        kind: DeliveryTransitionKind::Acknowledged,
        item_ref: Some(item_ref.clone()),
        token: Some(token.clone()),
        timer_intents: vec![timer_intent(
            DeliveryTimerIntentKind::CancelLeaseExpiry,
            &item_ref,
            Some(&token.delivery_id),
            token.visibility_deadline_tick,
            input.request,
        )],
    })
}

pub(super) fn negative_acknowledge(
    input: &DeliveryTransitionInput<'_>,
    next: &mut DeliveryState,
    token: &DeliveryToken,
    failure_class: &str,
) -> Result<AppliedTransition, DeliveryIssue> {
    let active = current_active(input, next, token, true, true)?;
    if input.policy.poison_failure_classes.contains(failure_class)
        && input.policy.poison_item_handling == PoisonItemHandling::RetainInFlight
    {
        return Err(DeliveryIssue::PoisonItemRetained);
    }
    if !input.policy.retryable_failure_classes.contains(failure_class)
        && !input.policy.poison_failure_classes.contains(failure_class)
    {
        return Err(DeliveryIssue::FailureClassUnsupported);
    }
    next.in_flight.remove(&token.item_ref);
    append_attempt(next, token, input.request, failure_class)?;
    let mut timer_intents = vec![timer_intent(
        DeliveryTimerIntentKind::CancelLeaseExpiry,
        &token.item_ref,
        Some(&token.delivery_id),
        token.visibility_deadline_tick,
        input.request,
    )];
    let should_dead_letter =
        input.policy.poison_failure_classes.contains(failure_class) || token.attempt >= input.policy.maximum_attempts;
    if should_dead_letter {
        let retention_deadline = dead_letter(input, next, active.item, token, failure_class)?;
        timer_intents.push(timer_intent(
            DeliveryTimerIntentKind::ScheduleDeadLetterRetention,
            &token.item_ref,
            Some(&token.delivery_id),
            retention_deadline,
            input.request,
        ));
        return Ok(AppliedTransition {
            kind: DeliveryTransitionKind::DeadLettered,
            item_ref: Some(token.item_ref.clone()),
            token: Some(token.clone()),
            timer_intents,
        });
    }
    let retry_deadline = retry_deadline(input, token)?;
    let retry_count = next.ready.values().filter(|ready| ready.attempts_in_cycle > 0).count();
    if collection_at_capacity(next.ready.len(), input.policy.ready_capacity)
        || collection_at_capacity(retry_count, input.policy.retry_capacity)
    {
        return Err(DeliveryIssue::ReadyCapacityExceeded);
    }
    next.ready.insert(token.item_ref.clone(), ReadyDelivery {
        item: active.item,
        eligible_at_tick: retry_deadline,
        cycle: token.cycle,
        attempts_in_cycle: token.attempt,
    });
    timer_intents.push(timer_intent(
        DeliveryTimerIntentKind::ScheduleRetryEligibility,
        &token.item_ref,
        Some(&token.delivery_id),
        retry_deadline,
        input.request,
    ));
    Ok(AppliedTransition {
        kind: DeliveryTransitionKind::RetryScheduled,
        item_ref: Some(token.item_ref.clone()),
        token: Some(token.clone()),
        timer_intents,
    })
}

pub(super) fn extend_lease(
    input: &DeliveryTransitionInput<'_>,
    next: &mut DeliveryState,
    token: &DeliveryToken,
) -> Result<AppliedTransition, DeliveryIssue> {
    let active = current_active(input, next, token, true, true)?;
    let new_deadline = visibility_deadline(input)?;
    if new_deadline <= token.visibility_deadline_tick {
        return Err(DeliveryIssue::LeaseExpired);
    }
    let mut next_token = token.clone();
    next_token.visibility_deadline_tick = new_deadline;
    next_token.token_ref = identify_delivery_token(&next_token);
    next.in_flight.insert(token.item_ref.clone(), ActiveDelivery {
        item: active.item,
        token: next_token.clone(),
    });
    Ok(AppliedTransition {
        kind: DeliveryTransitionKind::LeaseExtended,
        item_ref: Some(token.item_ref.clone()),
        token: Some(next_token.clone()),
        timer_intents: vec![
            timer_intent(
                DeliveryTimerIntentKind::CancelLeaseExpiry,
                &token.item_ref,
                Some(&token.delivery_id),
                token.visibility_deadline_tick,
                input.request,
            ),
            timer_intent(
                DeliveryTimerIntentKind::ScheduleLeaseExpiry,
                &token.item_ref,
                Some(&token.delivery_id),
                new_deadline,
                input.request,
            ),
        ],
    })
}

pub(super) fn expire_lease(
    input: &DeliveryTransitionInput<'_>,
    next: &mut DeliveryState,
    token: &DeliveryToken,
) -> Result<AppliedTransition, DeliveryIssue> {
    if !input.request.authority_refs.contains(&input.policy.expiry_authority_ref) {
        return Err(DeliveryIssue::ExpiryAuthorityRequired);
    }
    let active = current_active(input, next, token, false, false)?;
    if input.request.logical_tick < token.visibility_deadline_tick {
        return Err(DeliveryIssue::LeaseStillActive);
    }
    next.in_flight.remove(&token.item_ref);
    append_attempt(next, token, input.request, "lease-expired")?;
    if token.attempt >= input.policy.maximum_attempts {
        let retention_deadline = dead_letter(input, next, active.item, token, "attempts-exhausted")?;
        return Ok(AppliedTransition {
            kind: DeliveryTransitionKind::DeadLettered,
            item_ref: Some(token.item_ref.clone()),
            token: Some(token.clone()),
            timer_intents: vec![timer_intent(
                DeliveryTimerIntentKind::ScheduleDeadLetterRetention,
                &token.item_ref,
                Some(&token.delivery_id),
                retention_deadline,
                input.request,
            )],
        });
    }
    let retry_deadline = retry_deadline(input, token)?;
    let retry_count = next.ready.values().filter(|ready| ready.attempts_in_cycle > 0).count();
    if collection_at_capacity(next.ready.len(), input.policy.ready_capacity)
        || collection_at_capacity(retry_count, input.policy.retry_capacity)
    {
        return Err(DeliveryIssue::ReadyCapacityExceeded);
    }
    next.ready.insert(token.item_ref.clone(), ReadyDelivery {
        item: active.item,
        eligible_at_tick: retry_deadline,
        cycle: token.cycle,
        attempts_in_cycle: token.attempt,
    });
    Ok(AppliedTransition {
        kind: DeliveryTransitionKind::RetryScheduled,
        item_ref: Some(token.item_ref.clone()),
        token: Some(token.clone()),
        timer_intents: vec![timer_intent(
            DeliveryTimerIntentKind::ScheduleRetryEligibility,
            &token.item_ref,
            Some(&token.delivery_id),
            retry_deadline,
            input.request,
        )],
    })
}
