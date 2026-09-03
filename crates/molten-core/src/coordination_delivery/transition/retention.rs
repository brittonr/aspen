use super::support::*;
use super::*;

pub(super) fn redrive(
    input: &DeliveryTransitionInput<'_>,
    next: &mut DeliveryState,
    item_ref: &str,
) -> Result<AppliedTransition, DeliveryIssue> {
    if !input.request.authority_refs.contains(&input.policy.redrive_authority_ref) {
        return Err(DeliveryIssue::RedriveAuthorityRequired);
    }
    if collection_at_capacity(next.ready.len(), input.policy.ready_capacity) {
        return Err(DeliveryIssue::ReadyCapacityExceeded);
    }
    let dead_letter = next.dead_letter.remove(item_ref).ok_or(DeliveryIssue::ItemNotFound)?;
    let cycle = dead_letter.cycle.checked_add(1).ok_or(DeliveryIssue::ArithmeticOverflow)?;
    let sequence = next.next_sequence;
    next.next_sequence = sequence.checked_add(1).ok_or(DeliveryIssue::ArithmeticOverflow)?;
    let mut item = dead_letter.item;
    item.enqueue_sequence = sequence;
    next.ready.insert(item_ref.to_string(), ReadyDelivery {
        item,
        eligible_at_tick: input.request.logical_tick,
        cycle,
        attempts_in_cycle: 0,
    });
    let retention_deadline = dead_letter
        .entered_at_tick
        .checked_add(input.policy.dead_letter_retention_ticks)
        .ok_or(DeliveryIssue::ArithmeticOverflow)?;
    Ok(AppliedTransition {
        kind: DeliveryTransitionKind::Redriven,
        item_ref: Some(item_ref.to_string()),
        token: None,
        timer_intents: vec![timer_intent(
            DeliveryTimerIntentKind::CancelDeadLetterRetention,
            item_ref,
            None,
            retention_deadline,
            input.request,
        )],
    })
}

pub(super) fn cleanup_dead_letter(
    input: &DeliveryTransitionInput<'_>,
    next: &mut DeliveryState,
    through_tick: u64,
) -> Result<AppliedTransition, DeliveryIssue> {
    if !input.request.authority_refs.contains(&input.policy.retention_authority_ref) {
        return Err(DeliveryIssue::RetentionAuthorityRequired);
    }
    if through_tick > input.request.logical_tick {
        return Err(DeliveryIssue::RetentionNotExpired);
    }
    let mut removed = Vec::with_capacity(next.dead_letter.len());
    for (item_ref, dead_letter) in &next.dead_letter {
        let expiry = dead_letter
            .entered_at_tick
            .checked_add(input.policy.dead_letter_retention_ticks)
            .ok_or(DeliveryIssue::ArithmeticOverflow)?;
        if expiry <= through_tick {
            removed.push((item_ref.clone(), expiry));
        }
    }
    if removed.is_empty() {
        return Err(DeliveryIssue::RetentionNotExpired);
    }
    let timer_intents = removed
        .iter()
        .map(|(item_ref, deadline)| {
            timer_intent(DeliveryTimerIntentKind::CancelDeadLetterRetention, item_ref, None, *deadline, input.request)
        })
        .collect();
    for (item_ref, _) in &removed {
        next.dead_letter.remove(item_ref);
    }
    Ok(AppliedTransition {
        kind: DeliveryTransitionKind::DeadLetterCleaned,
        item_ref: None,
        token: None,
        timer_intents,
    })
}
