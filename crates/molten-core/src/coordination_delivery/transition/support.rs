use super::*;

pub(super) fn current_active(
    input: &DeliveryTransitionInput<'_>,
    next: &DeliveryState,
    token: &DeliveryToken,
    require_owner: bool,
    require_unexpired: bool,
) -> Result<ActiveDelivery, DeliveryIssue> {
    let active = next.in_flight.get(&token.item_ref).ok_or(DeliveryIssue::ItemNotFound)?;
    if active.token != *token {
        return Err(DeliveryIssue::TokenMismatch);
    }
    if require_owner
        && input.request.actor_id != token.consumer_id
        && !input.request.authority_refs.contains(&input.policy.completion_authority_ref)
    {
        return Err(DeliveryIssue::WrongOwner);
    }
    if require_unexpired && input.request.logical_tick >= token.visibility_deadline_tick {
        return Err(DeliveryIssue::LeaseExpired);
    }
    Ok(active.clone())
}

pub(super) fn dead_letter(
    input: &DeliveryTransitionInput<'_>,
    next: &mut DeliveryState,
    item: DeliveryItem,
    token: &DeliveryToken,
    reason: &str,
) -> Result<u64, DeliveryIssue> {
    if collection_at_capacity(next.dead_letter.len(), input.policy.dead_letter_capacity) {
        return Err(DeliveryIssue::DeadLetterCapacityExceeded);
    }
    let total_attempts = attempt_count(next, &token.item_ref)?;
    next.dead_letter.insert(token.item_ref.clone(), DeadLetterDelivery {
        item,
        entered_at_tick: input.request.logical_tick,
        cycle: token.cycle,
        attempts_in_cycle: token.attempt,
        total_attempts,
        reason: reason.to_string(),
    });
    input
        .request
        .logical_tick
        .checked_add(input.policy.dead_letter_retention_ticks)
        .ok_or(DeliveryIssue::ArithmeticOverflow)
}

pub(super) fn append_attempt(
    next: &mut DeliveryState,
    token: &DeliveryToken,
    request: &DeliveryRequest,
    outcome: &str,
) -> Result<(), DeliveryIssue> {
    let attempts = next.attempts.entry(token.item_ref.clone()).or_default();
    if collection_at_capacity(attempts.len(), MAX_DELIVERY_COLLECTION_ITEMS) {
        return Err(DeliveryIssue::AttemptLimitExceeded);
    }
    attempts.push(DeliveryAttempt {
        delivery_id: token.delivery_id.clone(),
        item_ref: token.item_ref.clone(),
        consumer_id: token.consumer_id.clone(),
        attempt: token.attempt,
        cycle: token.cycle,
        outcome: outcome.to_string(),
        operation_id: request.operation_id.clone(),
        observed_at_tick: request.logical_tick,
    });
    Ok(())
}

pub(super) fn attempt_count(state: &DeliveryState, item_ref: &str) -> Result<u64, DeliveryIssue> {
    let count = state.attempts.get(item_ref).map_or(0, Vec::len);
    u64::try_from(count).map_err(|_| DeliveryIssue::ArithmeticOverflow)
}

pub(super) fn select_ready_item(
    state: &DeliveryState,
    ordering: DeliveryOrdering,
    logical_tick: u64,
) -> Result<String, DeliveryIssue> {
    if state.ready.is_empty() {
        return Err(DeliveryIssue::QueueEmpty);
    }
    let mut ready = state.ready.values().collect::<Vec<_>>();
    ready.sort_by(|left, right| {
        (left.item.enqueue_sequence, &left.item.item_ref).cmp(&(right.item.enqueue_sequence, &right.item.item_ref))
    });
    let selected = match ordering {
        DeliveryOrdering::StrictFifo => ready.first().copied().filter(|entry| entry.eligible_at_tick <= logical_tick),
        DeliveryOrdering::RetryInterleaving => ready.into_iter().find(|entry| entry.eligible_at_tick <= logical_tick),
    };
    selected.map(|entry| entry.item.item_ref.clone()).ok_or(DeliveryIssue::NoEligibleItem)
}

pub(super) fn visibility_deadline(input: &DeliveryTransitionInput<'_>) -> Result<u64, DeliveryIssue> {
    add_logical_duration(input, input.request.logical_tick, input.policy.visibility_timeout_ticks)
}

pub(super) fn retry_deadline(input: &DeliveryTransitionInput<'_>, token: &DeliveryToken) -> Result<u64, DeliveryIssue> {
    let now = logical_time(input);
    let retry_index = token.attempt.saturating_sub(1);
    let plan = plan_retry(
        input.time_profile,
        input.request.service_generation,
        &token.item_ref,
        input.request.service_generation,
        &now,
        retry_index,
        RetryPolicy {
            maximum_attempts: input.policy.maximum_attempts,
            base_delay_ticks: input.policy.retry_base_delay_ticks,
            maximum_delay_ticks: input.policy.retry_maximum_delay_ticks,
            backoff: match input.policy.retry_backoff {
                DeliveryBackoff::Fixed => RetryBackoff::Fixed,
                DeliveryBackoff::Exponential => RetryBackoff::Exponential,
            },
            jitter: RetryJitter::None,
        },
        None,
    )
    .map_err(|_| DeliveryIssue::ArithmeticOverflow)?;
    Ok(plan.deadline.target.ticks())
}

fn add_logical_duration(input: &DeliveryTransitionInput<'_>, start: u64, ticks: u64) -> Result<u64, DeliveryIssue> {
    let start = TimeValue::Logical(LogicalEventTime {
        profile_ref: input.request.time_profile_ref.clone(),
        position: start,
    });
    let duration = CheckedDuration {
        profile_ref: input.request.time_profile_ref.clone(),
        domain: TimeDomain::Logical,
        ticks,
    };
    checked_add_duration(input.time_profile, &start, &duration)
        .map(|value| value.ticks())
        .map_err(|_| DeliveryIssue::ArithmeticOverflow)
}

fn logical_time(input: &DeliveryTransitionInput<'_>) -> TimeValue {
    TimeValue::Logical(LogicalEventTime {
        profile_ref: input.request.time_profile_ref.clone(),
        position: input.request.logical_tick,
    })
}

pub(super) fn timer_intent(
    kind: DeliveryTimerIntentKind,
    item_ref: &str,
    delivery_id: Option<&str>,
    deadline_tick: u64,
    request: &DeliveryRequest,
) -> DeliveryTimerIntent {
    DeliveryTimerIntent {
        kind,
        timer_id: identify_delivery_timer(TimerIdentityInput {
            kind,
            item_ref,
            delivery_id,
            deadline_tick,
            service_generation: request.service_generation,
            consistency_epoch: request.consistency_epoch,
        }),
        item_ref: item_ref.to_string(),
        delivery_id: delivery_id.map(str::to_string),
        deadline_tick,
        service_generation: request.service_generation,
        consistency_epoch: request.consistency_epoch,
    }
}

pub(super) fn contains_item(state: &DeliveryState, item_ref: &str) -> bool {
    state.ready.contains_key(item_ref)
        || state.in_flight.contains_key(item_ref)
        || state.dead_letter.contains_key(item_ref)
        || state.completed.contains_key(item_ref)
}

pub(super) fn collection_at_capacity(length: usize, capacity: u32) -> bool {
    u32::try_from(length).map_or(true, |length| length >= capacity)
}

pub(super) fn duplicate_transition(
    state: &DeliveryState,
    request_ref: &str,
    state_ref: &str,
    existing: &AppliedDeliveryOperation,
) -> DeliveryTransition {
    DeliveryTransition {
        schema: DELIVERY_TRANSITION_SCHEMA.to_string(),
        decision: DeliveryDecisionKind::DuplicateReplay,
        kind: DeliveryTransitionKind::DuplicateReplay,
        request_ref: request_ref.to_string(),
        operation_ref: existing.operation_ref.clone(),
        before_state_ref: state_ref.to_string(),
        after_state_ref: state_ref.to_string(),
        next_state: state.clone(),
        token: existing
            .item_ref
            .as_ref()
            .and_then(|item_ref| state.in_flight.get(item_ref))
            .map(|active| active.token.clone()),
        timer_intents: Vec::new(),
        issue: None,
        prior_operation_ref: Some(existing.operation_ref.clone()),
        worker_dispatch_authorized: false,
        external_effect_exactly_once: false,
    }
}

pub(super) fn denied_transition(
    state: &DeliveryState,
    request_ref: &str,
    state_ref: &str,
    issue: DeliveryIssue,
) -> DeliveryTransition {
    DeliveryTransition {
        schema: DELIVERY_TRANSITION_SCHEMA.to_string(),
        decision: DeliveryDecisionKind::Denied,
        kind: DeliveryTransitionKind::DeniedPreserve,
        request_ref: request_ref.to_string(),
        operation_ref: identify_applied_operation(request_ref, "denied", None, None),
        before_state_ref: state_ref.to_string(),
        after_state_ref: state_ref.to_string(),
        next_state: state.clone(),
        token: None,
        timer_intents: Vec::new(),
        issue: Some(issue),
        prior_operation_ref: None,
        worker_dispatch_authorized: false,
        external_effect_exactly_once: false,
    }
}
