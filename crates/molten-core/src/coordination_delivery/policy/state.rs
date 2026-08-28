use std::collections::BTreeSet;

use super::*;

pub(super) fn validate_state_collections(
    state: &DeliveryState,
    policy: &DeliveryPolicy,
    issues: &mut Vec<DeliveryIssue>,
) {
    if exceeds_bound(state.ready.len(), policy.ready_capacity) {
        issues.push(DeliveryIssue::ReadyCapacityExceeded);
    }
    if exceeds_bound(state.in_flight.len(), policy.in_flight_capacity) {
        issues.push(DeliveryIssue::InFlightCapacityExceeded);
    }
    if exceeds_bound(state.dead_letter.len(), policy.dead_letter_capacity) {
        issues.push(DeliveryIssue::DeadLetterCapacityExceeded);
    }
    let mut item_refs = BTreeSet::new();
    for (key, ready) in &state.ready {
        validate_item_key(key, &ready.item, policy, &mut item_refs, issues);
    }
    for (key, active) in &state.in_flight {
        validate_item_key(key, &active.item, policy, &mut item_refs, issues);
        if active.token.item_ref != *key || active.token.token_ref != identify_delivery_token(&active.token) {
            issues.push(DeliveryIssue::InvalidState);
        }
    }
    for (key, dead_letter) in &state.dead_letter {
        validate_item_key(key, &dead_letter.item, policy, &mut item_refs, issues);
    }
    for (key, completed) in &state.completed {
        validate_item_key(key, &completed.item, policy, &mut item_refs, issues);
    }
    if exceeds_bound(state.operations.len(), MAX_DELIVERY_COLLECTION_ITEMS)
        || exceeds_bound(state.attempts.len(), MAX_DELIVERY_COLLECTION_ITEMS)
    {
        issues.push(DeliveryIssue::InvalidState);
    }
    for (item_ref, attempts) in &state.attempts {
        if exceeds_bound(attempts.len(), MAX_DELIVERY_COLLECTION_ITEMS)
            || !valid_delivery_ref(item_ref)
            || attempts.iter().any(|attempt| {
                attempt.item_ref != *item_ref
                    || !valid_delivery_id(&attempt.consumer_id)
                    || !valid_failure_class(&attempt.outcome)
                    || !valid_delivery_ref(&attempt.operation_id)
            })
        {
            issues.push(DeliveryIssue::InvalidState);
        }
    }
    for (operation_id, operation) in &state.operations {
        if !valid_delivery_ref(operation_id)
            || !valid_delivery_ref(&operation.request_ref)
            || !valid_delivery_ref(&operation.operation_ref)
        {
            issues.push(DeliveryIssue::InvalidState);
        }
    }
}

fn validate_item_key(
    key: &str,
    item: &DeliveryItem,
    policy: &DeliveryPolicy,
    item_refs: &mut BTreeSet<String>,
    issues: &mut Vec<DeliveryIssue>,
) {
    if key != item.item_ref
        || !item_refs.insert(key.to_string())
        || !valid_delivery_ref(&item.item_ref)
        || !valid_delivery_ref(&item.content_ref)
        || !valid_delivery_ref(&item.metadata_ref)
        || item.metadata_bytes > policy.metadata_byte_limit
        || item.policy_ref != identify_delivery_policy(policy)
        || item.enqueue_sequence == 0
    {
        issues.push(DeliveryIssue::InvalidState);
    }
}
