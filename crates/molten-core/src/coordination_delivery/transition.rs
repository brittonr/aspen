#![allow(
    tigerstyle::non_trait_imports,
    reason = "the transition table names the exact accepted fabric-time primitives at its boundary"
)]

mod completion;
mod queue;
mod retention;
mod support;

use super::*;
use crate::fabric_time::AdmittedTimeProfile;
use crate::fabric_time::CheckedDuration;
use crate::fabric_time::LogicalEventTime;
use crate::fabric_time::RetryBackoff;
use crate::fabric_time::RetryJitter;
use crate::fabric_time::RetryPolicy;
use crate::fabric_time::TimeDomain;
use crate::fabric_time::TimeValue;
use crate::fabric_time::checked_add_duration;
use crate::fabric_time::plan_retry;

pub struct DeliveryTransitionInput<'a> {
    pub manifest: &'a DeliveryManifest,
    pub policy: &'a DeliveryPolicy,
    pub time_profile: &'a AdmittedTimeProfile,
    pub state: &'a DeliveryState,
    pub request: &'a DeliveryRequest,
}

pub(super) struct AppliedTransition {
    pub(super) kind: DeliveryTransitionKind,
    pub(super) item_ref: Option<String>,
    pub(super) token: Option<DeliveryToken>,
    pub(super) timer_intents: Vec<DeliveryTimerIntent>,
}

#[derive(Clone, Copy)]
pub(super) struct EnqueueFields<'a> {
    pub(super) item_ref: &'a str,
    pub(super) content_ref: &'a str,
    pub(super) metadata_ref: &'a str,
    pub(super) metadata_bytes: u32,
}

// r[impl molten.coordination_delivery.claim_lease]
// r[impl molten.coordination_delivery.fenced_completion]
// r[impl molten.coordination_delivery.retry_dlq_policy]
// r[impl molten.coordination_delivery.logical_time]
#[must_use]
pub fn plan_delivery_transition(input: &DeliveryTransitionInput<'_>) -> DeliveryTransition {
    let request_ref = identify_delivery_request(input.request);
    let before_state_ref = identify_delivery_state(input.state);
    if let Some(issue) = validate_transition_input(input) {
        return support::denied_transition(input.state, &request_ref, &before_state_ref, issue);
    }
    if let Some(existing) = input.state.operations.get(&input.request.operation_id) {
        if existing.request_ref == request_ref {
            return support::duplicate_transition(input.state, &request_ref, &before_state_ref, existing);
        }
        return support::denied_transition(
            input.state,
            &request_ref,
            &before_state_ref,
            DeliveryIssue::ConflictingDuplicateOperation,
        );
    }

    let mut next = input.state.clone();
    let applied = match apply_operation(input, &request_ref, &mut next) {
        Ok(applied) => applied,
        Err(issue) => {
            return support::denied_transition(input.state, &request_ref, &before_state_ref, issue);
        }
    };
    next.revision = match next.revision.checked_add(1) {
        Some(revision) => revision,
        None => {
            return support::denied_transition(
                input.state,
                &request_ref,
                &before_state_ref,
                DeliveryIssue::ArithmeticOverflow,
            );
        }
    };
    let operation_ref = identify_applied_operation(
        &request_ref,
        input.request.operation.kind(),
        applied.item_ref.as_deref(),
        applied.token.as_ref().map(|token| token.token_ref.as_str()),
    );
    next.operations.insert(input.request.operation_id.clone(), AppliedDeliveryOperation {
        request_ref: request_ref.clone(),
        operation_ref: operation_ref.clone(),
        operation_kind: input.request.operation.kind().to_string(),
        item_ref: applied.item_ref,
        token_ref: applied.token.as_ref().map(|token| token.token_ref.clone()),
    });
    if validate_delivery_state(&next, input.manifest, input.policy).is_err() {
        return support::denied_transition(input.state, &request_ref, &before_state_ref, DeliveryIssue::InvalidState);
    }
    let after_state_ref = identify_delivery_state(&next);
    DeliveryTransition {
        schema: DELIVERY_TRANSITION_SCHEMA.to_string(),
        decision: DeliveryDecisionKind::Applied,
        kind: applied.kind,
        request_ref,
        operation_ref,
        before_state_ref,
        after_state_ref,
        next_state: next,
        token: applied.token,
        timer_intents: applied.timer_intents,
        issue: None,
        prior_operation_ref: None,
        worker_dispatch_authorized: false,
        external_effect_exactly_once: false,
    }
}

fn validate_transition_input(input: &DeliveryTransitionInput<'_>) -> Option<DeliveryIssue> {
    if validate_delivery_policy(input.policy).is_err() {
        return Some(DeliveryIssue::InvalidPolicy);
    }
    if validate_delivery_manifest(input.manifest, input.policy).is_err() {
        return Some(DeliveryIssue::InvalidManifest);
    }
    if validate_delivery_state(input.state, input.manifest, input.policy).is_err() {
        return Some(DeliveryIssue::InvalidState);
    }
    if let Err(issues) = validate_delivery_request(input.request, input.state, input.manifest, input.policy) {
        return issues.into_iter().next();
    }
    if input.time_profile.profile_ref != input.manifest.time_profile_ref {
        return Some(DeliveryIssue::LogicalTimeProfileMismatch);
    }
    if !input.time_profile.supported_domains.contains(&TimeDomain::Logical) {
        return Some(DeliveryIssue::LogicalTimeRequired);
    }
    None
}

fn apply_operation(
    input: &DeliveryTransitionInput<'_>,
    request_ref: &str,
    next: &mut DeliveryState,
) -> Result<AppliedTransition, DeliveryIssue> {
    match &input.request.operation {
        DeliveryOperation::Enqueue {
            item_ref,
            content_ref,
            metadata_ref,
            metadata_bytes,
        } => queue::enqueue(input, next, EnqueueFields {
            item_ref,
            content_ref,
            metadata_ref,
            metadata_bytes: *metadata_bytes,
        }),
        DeliveryOperation::Claim => queue::claim(input, request_ref, next),
        DeliveryOperation::Acknowledge { token } => completion::acknowledge(input, next, token),
        DeliveryOperation::NegativeAcknowledge { token, failure_class } => {
            completion::negative_acknowledge(input, next, token, failure_class)
        }
        DeliveryOperation::ExtendLease { token } => completion::extend_lease(input, next, token),
        DeliveryOperation::ExpireLease { token } => completion::expire_lease(input, next, token),
        DeliveryOperation::Redrive { item_ref } => retention::redrive(input, next, item_ref),
        DeliveryOperation::CleanupDeadLetter { through_tick } => {
            retention::cleanup_dead_letter(input, next, *through_tick)
        }
    }
}
