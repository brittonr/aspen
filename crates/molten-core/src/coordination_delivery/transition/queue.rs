use super::support::*;
use super::*;

pub(super) fn enqueue(
    input: &DeliveryTransitionInput<'_>,
    next: &mut DeliveryState,
    fields: EnqueueFields<'_>,
) -> Result<AppliedTransition, DeliveryIssue> {
    if contains_item(next, fields.item_ref) {
        return Err(DeliveryIssue::ItemAlreadyExists);
    }
    if collection_at_capacity(next.ready.len(), input.policy.ready_capacity) {
        return Err(DeliveryIssue::ReadyCapacityExceeded);
    }
    if fields.metadata_bytes > input.policy.metadata_byte_limit {
        return Err(DeliveryIssue::MetadataLimitExceeded);
    }
    let sequence = next.next_sequence;
    next.next_sequence = sequence.checked_add(1).ok_or(DeliveryIssue::ArithmeticOverflow)?;
    let item = DeliveryItem {
        item_ref: fields.item_ref.to_string(),
        content_ref: fields.content_ref.to_string(),
        metadata_ref: fields.metadata_ref.to_string(),
        metadata_bytes: fields.metadata_bytes,
        enqueue_sequence: sequence,
        policy_ref: next.policy_ref.clone(),
    };
    next.ready.insert(fields.item_ref.to_string(), ReadyDelivery {
        item,
        eligible_at_tick: input.request.logical_tick,
        cycle: INITIAL_DELIVERY_CYCLE,
        attempts_in_cycle: 0,
    });
    Ok(AppliedTransition {
        kind: DeliveryTransitionKind::Enqueued,
        item_ref: Some(fields.item_ref.to_string()),
        token: None,
        timer_intents: Vec::new(),
    })
}

pub(super) fn claim(
    input: &DeliveryTransitionInput<'_>,
    request_ref: &str,
    next: &mut DeliveryState,
) -> Result<AppliedTransition, DeliveryIssue> {
    if collection_at_capacity(next.in_flight.len(), input.policy.in_flight_capacity) {
        return Err(DeliveryIssue::InFlightCapacityExceeded);
    }
    let item_ref = select_ready_item(next, input.policy.ordering, input.request.logical_tick)?;
    let ready = next.ready.remove(&item_ref).ok_or(DeliveryIssue::ItemNotFound)?;
    let attempt = ready.attempts_in_cycle.checked_add(1).ok_or(DeliveryIssue::ArithmeticOverflow)?;
    if attempt > input.policy.maximum_attempts {
        return Err(DeliveryIssue::AttemptLimitExceeded);
    }
    let deadline = visibility_deadline(input)?;
    let fencing_token = next.next_fencing_token;
    next.next_fencing_token = fencing_token.checked_add(1).ok_or(DeliveryIssue::ArithmeticOverflow)?;
    let delivery_id = identify_applied_operation(request_ref, "delivery", Some(&item_ref), None);
    let mut token = DeliveryToken {
        token_ref: String::new(),
        delivery_id: delivery_id.clone(),
        queue_id: next.queue_id.clone(),
        item_ref: item_ref.clone(),
        consumer_id: input.request.actor_id.clone(),
        attempt,
        cycle: ready.cycle,
        fencing_token,
        claimed_at_tick: input.request.logical_tick,
        visibility_deadline_tick: deadline,
        consistency_epoch: input.request.consistency_epoch,
        service_generation: input.request.service_generation,
        policy_ref: next.policy_ref.clone(),
    };
    token.token_ref = identify_delivery_token(&token);
    next.in_flight.insert(item_ref.clone(), ActiveDelivery {
        item: ready.item,
        token: token.clone(),
    });
    Ok(AppliedTransition {
        kind: DeliveryTransitionKind::Claimed,
        item_ref: Some(item_ref.clone()),
        token: Some(token.clone()),
        timer_intents: vec![timer_intent(
            DeliveryTimerIntentKind::ScheduleLeaseExpiry,
            &item_ref,
            Some(&delivery_id),
            deadline,
            input.request,
        )],
    })
}
