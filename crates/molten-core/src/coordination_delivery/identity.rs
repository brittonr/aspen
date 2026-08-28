#![allow(
    tigerstyle::non_trait_imports,
    reason = "the identity projection uses one private framing owner throughout"
)]

mod framing;

use framing::FramedHasher;

use super::*;

const POLICY_IDENTITY_DOMAIN: &str = "onixresearch.molten.coordination-delivery-policy.v1";
const MANIFEST_IDENTITY_DOMAIN: &str = "onixresearch.molten.coordination-delivery-manifest.v1";
const REQUEST_IDENTITY_DOMAIN: &str = "onixresearch.molten.coordination-delivery-request.v1";
const TOKEN_IDENTITY_DOMAIN: &str = "onixresearch.molten.coordination-delivery-token.v1";
const STATE_IDENTITY_DOMAIN: &str = "onixresearch.molten.coordination-delivery-state.v1";
const OPERATION_IDENTITY_DOMAIN: &str = "onixresearch.molten.coordination-delivery-operation.v1";
const TIMER_IDENTITY_DOMAIN: &str = "onixresearch.molten.coordination-delivery-timer.v1";

#[must_use]
pub fn identify_delivery_policy(policy: &DeliveryPolicy) -> String {
    let mut framed = FramedHasher::new(POLICY_IDENTITY_DOMAIN);
    framed.text("schema", &policy.schema);
    framed.text("policy-id", &policy.policy_id);
    framed.number("visibility-timeout", policy.visibility_timeout_ticks);
    framed.number("maximum-attempts", policy.maximum_attempts);
    framed.number("retry-base-delay", policy.retry_base_delay_ticks);
    framed.number("retry-maximum-delay", policy.retry_maximum_delay_ticks);
    framed.text("retry-backoff", policy.retry_backoff.as_str());
    framed.text("ordering", policy.ordering.as_str());
    framed.text("dead-letter-queue", &policy.dead_letter_queue_id);
    framed.number("dead-letter-retention", policy.dead_letter_retention_ticks);
    framed.number("ready-capacity", u64::from(policy.ready_capacity));
    framed.number("in-flight-capacity", u64::from(policy.in_flight_capacity));
    framed.number("retry-capacity", u64::from(policy.retry_capacity));
    framed.number("dead-letter-capacity", u64::from(policy.dead_letter_capacity));
    framed.number("metadata-byte-limit", u64::from(policy.metadata_byte_limit));
    framed.number("status-item-limit", u64::from(policy.status_item_limit));
    framed.text("completion-authority", &policy.completion_authority_ref);
    framed.text("expiry-authority", &policy.expiry_authority_ref);
    framed.text("redrive-authority", &policy.redrive_authority_ref);
    framed.text("retention-authority", &policy.retention_authority_ref);
    for failure in &policy.retryable_failure_classes {
        framed.text("retryable-failure", failure);
    }
    for failure in &policy.poison_failure_classes {
        framed.text("poison-failure", failure);
    }
    framed.text("poison-handling", policy.poison_item_handling.as_str());
    for non_claim in &policy.non_claims {
        framed.text("non-claim", non_claim);
    }
    framed.finish()
}

#[must_use]
pub fn identify_delivery_manifest(manifest: &DeliveryManifest) -> String {
    let mut framed = FramedHasher::new(MANIFEST_IDENTITY_DOMAIN);
    framed.text("schema", &manifest.schema);
    framed.text("extension-id", &manifest.extension_id);
    framed.text("service-id", &manifest.service_id);
    framed.number("service-generation", manifest.service_generation);
    framed.text("implementation-ref", &manifest.implementation_ref);
    framed.text("time-profile-ref", &manifest.time_profile_ref);
    framed.text("policy-ref", &manifest.policy_ref);
    for (port, binding) in &manifest.port_bindings {
        framed.text("port", port);
        framed.text("binding", binding);
    }
    for non_claim in &manifest.non_claims {
        framed.text("non-claim", non_claim);
    }
    framed.finish()
}

#[must_use]
pub fn identify_delivery_request(request: &DeliveryRequest) -> String {
    let mut framed = FramedHasher::new(REQUEST_IDENTITY_DOMAIN);
    framed.text("schema", &request.schema);
    framed.text("queue-id", &request.queue_id);
    framed.text("operation-id", &request.operation_id);
    framed.text("actor-id", &request.actor_id);
    framed.number("service-generation", request.service_generation);
    framed.number("consistency-epoch", request.consistency_epoch);
    framed.number("engine-epoch", request.engine_epoch);
    framed.text("time-profile-ref", &request.time_profile_ref);
    framed.number("logical-tick", request.logical_tick);
    framed.text("currentness", request.currentness.as_str());
    for reference in &request.authority_refs {
        framed.text("authority-ref", reference);
    }
    for reference in &request.policy_refs {
        framed.text("policy-ref", reference);
    }
    for reference in &request.resource_refs {
        framed.text("resource-ref", reference);
    }
    for reference in &request.evidence_refs {
        framed.text("evidence-ref", reference);
    }
    hash_operation(&mut framed, &request.operation);
    framed.finish()
}

#[must_use]
pub fn identify_delivery_token(token: &DeliveryToken) -> String {
    let mut framed = FramedHasher::new(TOKEN_IDENTITY_DOMAIN);
    hash_token_fields(&mut framed, token, false);
    framed.finish()
}

#[must_use]
pub fn identify_delivery_state(state: &DeliveryState) -> String {
    let mut framed = FramedHasher::new(STATE_IDENTITY_DOMAIN);
    framed.text("schema", &state.schema);
    framed.text("queue-id", &state.queue_id);
    framed.text("policy-ref", &state.policy_ref);
    framed.number("service-generation", state.service_generation);
    framed.number("consistency-epoch", state.consistency_epoch);
    framed.number("revision", state.revision);
    framed.number("next-sequence", state.next_sequence);
    framed.number("next-fencing-token", state.next_fencing_token);
    for (item_ref, ready) in &state.ready {
        framed.text("ready-key", item_ref);
        hash_ready(&mut framed, ready);
    }
    for (item_ref, active) in &state.in_flight {
        framed.text("in-flight-key", item_ref);
        hash_item(&mut framed, &active.item);
        hash_token_fields(&mut framed, &active.token, true);
    }
    for (item_ref, dead_letter) in &state.dead_letter {
        framed.text("dead-letter-key", item_ref);
        hash_item(&mut framed, &dead_letter.item);
        framed.number("dead-letter-entered-at", dead_letter.entered_at_tick);
        framed.number("dead-letter-cycle", u64::from(dead_letter.cycle));
        framed.number("dead-letter-cycle-attempts", dead_letter.attempts_in_cycle);
        framed.number("dead-letter-total-attempts", dead_letter.total_attempts);
        framed.text("dead-letter-reason", &dead_letter.reason);
    }
    for (item_ref, completed) in &state.completed {
        framed.text("completed-key", item_ref);
        hash_item(&mut framed, &completed.item);
        framed.text("completed-delivery-id", &completed.delivery_id);
        framed.number("completed-at", completed.acknowledged_at_tick);
        framed.number("completed-total-attempts", completed.total_attempts);
    }
    for (item_ref, attempts) in &state.attempts {
        framed.text("attempt-key", item_ref);
        for attempt in attempts {
            hash_attempt(&mut framed, attempt);
        }
    }
    for (operation_id, operation) in &state.operations {
        framed.text("operation-id", operation_id);
        framed.text("operation-request-ref", &operation.request_ref);
        framed.text("operation-ref", &operation.operation_ref);
        framed.text("operation-kind", &operation.operation_kind);
        framed.optional_text("operation-item-ref", operation.item_ref.as_deref());
        framed.optional_text("operation-token-ref", operation.token_ref.as_deref());
    }
    framed.finish()
}

#[must_use]
pub fn identify_applied_operation(
    request_ref: &str,
    operation_kind: &str,
    item_ref: Option<&str>,
    token_ref: Option<&str>,
) -> String {
    let mut framed = FramedHasher::new(OPERATION_IDENTITY_DOMAIN);
    framed.text("request-ref", request_ref);
    framed.text("operation-kind", operation_kind);
    framed.optional_text("item-ref", item_ref);
    framed.optional_text("token-ref", token_ref);
    framed.finish()
}

#[derive(Clone, Copy, Debug)]
pub struct TimerIdentityInput<'a> {
    pub kind: DeliveryTimerIntentKind,
    pub item_ref: &'a str,
    pub delivery_id: Option<&'a str>,
    pub deadline_tick: u64,
    pub service_generation: u64,
    pub consistency_epoch: u64,
}

#[must_use]
pub fn identify_delivery_timer(input: TimerIdentityInput<'_>) -> String {
    let mut framed = FramedHasher::new(TIMER_IDENTITY_DOMAIN);
    framed.text("kind", input.kind.as_str());
    framed.text("item-ref", input.item_ref);
    framed.optional_text("delivery-id", input.delivery_id);
    framed.number("deadline-tick", input.deadline_tick);
    framed.number("service-generation", input.service_generation);
    framed.number("consistency-epoch", input.consistency_epoch);
    framed.finish()
}

fn hash_operation(framed: &mut FramedHasher, operation: &DeliveryOperation) {
    framed.text("operation-kind", operation.kind());
    match operation {
        DeliveryOperation::Enqueue {
            item_ref,
            content_ref,
            metadata_ref,
            metadata_bytes,
        } => {
            framed.text("item-ref", item_ref);
            framed.text("content-ref", content_ref);
            framed.text("metadata-ref", metadata_ref);
            framed.number("metadata-bytes", u64::from(*metadata_bytes));
        }
        DeliveryOperation::Claim => {}
        DeliveryOperation::Acknowledge { token }
        | DeliveryOperation::ExtendLease { token }
        | DeliveryOperation::ExpireLease { token } => hash_token_fields(framed, token, true),
        DeliveryOperation::NegativeAcknowledge { token, failure_class } => {
            hash_token_fields(framed, token, true);
            framed.text("failure-class", failure_class);
        }
        DeliveryOperation::Redrive { item_ref } => framed.text("item-ref", item_ref),
        DeliveryOperation::CleanupDeadLetter { through_tick } => {
            framed.number("through-tick", *through_tick);
        }
    }
}

fn hash_ready(framed: &mut FramedHasher, ready: &ReadyDelivery) {
    hash_item(framed, &ready.item);
    framed.number("ready-eligible-at", ready.eligible_at_tick);
    framed.number("ready-cycle", u64::from(ready.cycle));
    framed.number("ready-attempts", ready.attempts_in_cycle);
}

fn hash_item(framed: &mut FramedHasher, item: &DeliveryItem) {
    framed.text("item-ref", &item.item_ref);
    framed.text("content-ref", &item.content_ref);
    framed.text("metadata-ref", &item.metadata_ref);
    framed.number("metadata-bytes", u64::from(item.metadata_bytes));
    framed.number("enqueue-sequence", item.enqueue_sequence);
    framed.text("item-policy-ref", &item.policy_ref);
}

fn hash_token_fields(framed: &mut FramedHasher, token: &DeliveryToken, include_token_ref: bool) {
    if include_token_ref {
        framed.text("token-ref", &token.token_ref);
    }
    framed.text("delivery-id", &token.delivery_id);
    framed.text("token-queue-id", &token.queue_id);
    framed.text("token-item-ref", &token.item_ref);
    framed.text("consumer-id", &token.consumer_id);
    framed.number("attempt", token.attempt);
    framed.number("cycle", u64::from(token.cycle));
    framed.number("fencing-token", token.fencing_token);
    framed.number("claimed-at", token.claimed_at_tick);
    framed.number("visibility-deadline", token.visibility_deadline_tick);
    framed.number("token-consistency-epoch", token.consistency_epoch);
    framed.number("token-service-generation", token.service_generation);
    framed.text("token-policy-ref", &token.policy_ref);
}

fn hash_attempt(framed: &mut FramedHasher, attempt: &DeliveryAttempt) {
    framed.text("attempt-delivery-id", &attempt.delivery_id);
    framed.text("attempt-item-ref", &attempt.item_ref);
    framed.text("attempt-consumer-id", &attempt.consumer_id);
    framed.number("attempt-number", attempt.attempt);
    framed.number("attempt-cycle", u64::from(attempt.cycle));
    framed.text("attempt-outcome", &attempt.outcome);
    framed.text("attempt-operation-id", &attempt.operation_id);
    framed.number("attempt-observed-at", attempt.observed_at_tick);
}
