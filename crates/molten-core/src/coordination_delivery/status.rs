use super::*;

pub struct StatusProjectionInput<'a> {
    pub policy: &'a DeliveryPolicy,
    pub requested_limit: u32,
    pub resource_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

// r[impl molten.coordination_delivery.content_refs]
#[must_use]
pub fn plan_delivery_worker_dispatch(
    active: &ActiveDelivery,
    admission: &DeliveryWorkerAdmission,
) -> DeliveryWorkerPlan {
    let is_reference_set_valid = !admission.evidence_refs.is_empty()
        && admission.evidence_refs.iter().all(|reference| valid_delivery_ref(reference));
    let is_admitted = admission.content_verified
        && admission.provenance_current
        && admission.authority_current
        && admission.policy_current
        && admission.resource_admitted
        && admission.execution_admitted
        && is_reference_set_valid;
    DeliveryWorkerPlan {
        schema: DELIVERY_WORKER_PLAN_SCHEMA.to_string(),
        admitted: is_admitted,
        delivery_id: active.token.delivery_id.clone(),
        item_ref: active.item.item_ref.clone(),
        content_ref: active.item.content_ref.clone(),
        issue: (!is_admitted).then_some(DeliveryIssue::WorkerAdmissionIncomplete),
        external_effect_authorized: false,
        exact_once_claimed: false,
    }
}

// r[impl molten.coordination_delivery.retry_dlq_policy]
pub fn project_delivery_status(
    state: &DeliveryState,
    input: &StatusProjectionInput<'_>,
) -> Result<DeliveryStatus, DeliveryIssue> {
    if input.requested_limit == 0
        || input.requested_limit > input.policy.status_item_limit
        || input.resource_refs.is_empty()
        || input.evidence_refs.is_empty()
        || input
            .resource_refs
            .iter()
            .chain(input.evidence_refs)
            .any(|reference| !valid_delivery_ref(reference))
    {
        return Err(DeliveryIssue::InvalidPolicy);
    }
    let mut active_claims = state
        .in_flight
        .values()
        .map(|active| ActiveDeliveryStatus {
            item_ref: active.item.item_ref.clone(),
            delivery_id: active.token.delivery_id.clone(),
            consumer_id: active.token.consumer_id.clone(),
            attempt: active.token.attempt,
            visibility_deadline_tick: active.token.visibility_deadline_tick,
        })
        .collect::<Vec<_>>();
    active_claims.sort_by(|left, right| {
        (left.visibility_deadline_tick, &left.item_ref, &left.delivery_id).cmp(&(
            right.visibility_deadline_tick,
            &right.item_ref,
            &right.delivery_id,
        ))
    });
    let requested_limit = usize::try_from(input.requested_limit).map_err(|_| DeliveryIssue::ArithmeticOverflow)?;
    let is_truncated = active_claims.len() > requested_limit;
    active_claims.truncate(requested_limit);
    let retry_count = state.ready.values().filter(|ready| ready.attempts_in_cycle > 0).count();
    let failed_attempt_count =
        state.attempts.values().flatten().filter(|attempt| attempt.outcome != "acknowledged").count();
    let mut resource_refs = input.resource_refs.to_vec();
    resource_refs.sort();
    resource_refs.dedup();
    let mut evidence_refs = input.evidence_refs.to_vec();
    evidence_refs.sort();
    evidence_refs.dedup();
    Ok(DeliveryStatus {
        schema: DELIVERY_STATUS_SCHEMA.to_string(),
        queue_id: state.queue_id.clone(),
        state_ref: identify_delivery_state(state),
        revision: state.revision,
        policy_ref: state.policy_ref.clone(),
        maximum_attempts: input.policy.maximum_attempts,
        ready_count: bounded_count(state.ready.len())?,
        retry_count: bounded_count(retry_count)?,
        in_flight_count: bounded_count(state.in_flight.len())?,
        dead_letter_count: bounded_count(state.dead_letter.len())?,
        completed_count: bounded_count(state.completed.len())?,
        failed_attempt_count: bounded_count(failed_attempt_count)?,
        active_claims,
        resource_refs,
        evidence_refs,
        truncated: is_truncated,
        payloads_rendered: false,
    })
}

fn bounded_count(length: usize) -> Result<u32, DeliveryIssue> {
    u32::try_from(length).map_err(|_| DeliveryIssue::ArithmeticOverflow)
}
