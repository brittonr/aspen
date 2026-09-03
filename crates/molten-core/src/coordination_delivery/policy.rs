#![allow(
    tigerstyle::non_trait_imports,
    reason = "the admission unit uses one explicit ordered-set type throughout validation"
)]
#![allow(
    tigerstyle::borrowed_argument_types,
    reason = "private validation helpers append to one caller-owned bounded issue buffer"
)]

mod request;
mod state;

use std::collections::BTreeSet;

use super::*;

const BLAKE3_REF_PREFIX: &str = "blake3:";
const BLAKE3_HEX_LENGTH: usize = 64;
const MAX_VALIDATION_ISSUES: usize = 64;

pub fn validate_delivery_policy(policy: &DeliveryPolicy) -> Result<(), Vec<DeliveryIssue>> {
    let mut issues = Vec::with_capacity(MAX_VALIDATION_ISSUES);
    if policy.schema != DELIVERY_POLICY_SCHEMA || !valid_delivery_id(&policy.policy_id) {
        issues.push(DeliveryIssue::InvalidPolicy);
    }
    if policy.visibility_timeout_ticks == 0
        || policy.visibility_timeout_ticks > MAX_DELIVERY_TICKS
        || policy.maximum_attempts == 0
        || policy.maximum_attempts > MAX_DELIVERY_ATTEMPTS
        || policy.retry_base_delay_ticks == 0
        || policy.retry_base_delay_ticks > policy.retry_maximum_delay_ticks
        || policy.retry_maximum_delay_ticks > MAX_DELIVERY_TICKS
        || policy.dead_letter_retention_ticks == 0
        || policy.dead_letter_retention_ticks > MAX_DELIVERY_TICKS
    {
        issues.push(DeliveryIssue::InvalidPolicy);
    }
    for capacity in [
        policy.ready_capacity,
        policy.in_flight_capacity,
        policy.retry_capacity,
        policy.dead_letter_capacity,
    ] {
        if capacity == 0 || capacity > MAX_DELIVERY_COLLECTION_ITEMS {
            issues.push(DeliveryIssue::InvalidPolicy);
        }
    }
    if policy.metadata_byte_limit == 0
        || policy.metadata_byte_limit > MAX_DELIVERY_METADATA_BYTES
        || policy.status_item_limit == 0
        || policy.status_item_limit > MAX_DELIVERY_STATUS_ITEMS
    {
        issues.push(DeliveryIssue::InvalidPolicy);
    }
    if !valid_delivery_id(&policy.dead_letter_queue_id)
        || !valid_delivery_ref(&policy.completion_authority_ref)
        || !valid_delivery_ref(&policy.expiry_authority_ref)
        || !valid_delivery_ref(&policy.redrive_authority_ref)
        || !valid_delivery_ref(&policy.retention_authority_ref)
    {
        issues.push(DeliveryIssue::InvalidReference);
    }
    if policy.retryable_failure_classes.is_empty()
        || policy.retryable_failure_classes.iter().any(|value| !valid_failure_class(value))
        || policy.poison_failure_classes.iter().any(|value| !valid_failure_class(value))
        || !policy.retryable_failure_classes.is_disjoint(&policy.poison_failure_classes)
    {
        issues.push(DeliveryIssue::InvalidPolicy);
    }
    if policy.non_claims != required_delivery_non_claims() {
        issues.push(DeliveryIssue::InvalidPolicy);
    }
    finish_validation(issues)
}

pub fn validate_delivery_manifest(
    manifest: &DeliveryManifest,
    policy: &DeliveryPolicy,
) -> Result<(), Vec<DeliveryIssue>> {
    let mut issues = Vec::with_capacity(MAX_VALIDATION_ISSUES);
    if manifest.schema != DELIVERY_MANIFEST_SCHEMA
        || !valid_delivery_id(&manifest.extension_id)
        || !valid_delivery_id(&manifest.service_id)
        || manifest.service_generation == 0
        || !valid_delivery_ref(&manifest.implementation_ref)
        || !valid_delivery_ref(&manifest.time_profile_ref)
    {
        issues.push(DeliveryIssue::InvalidManifest);
    }
    if manifest.policy_ref != identify_delivery_policy(policy) {
        issues.push(DeliveryIssue::PolicyMismatch);
    }
    let actual_ports = manifest.port_bindings.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let required_ports = REQUIRED_DELIVERY_PORTS.into_iter().collect::<BTreeSet<_>>();
    if actual_ports != required_ports || manifest.port_bindings.values().any(|reference| !valid_delivery_ref(reference))
    {
        issues.push(DeliveryIssue::InvalidManifest);
    }
    if manifest.non_claims != required_delivery_non_claims() {
        issues.push(DeliveryIssue::InvalidManifest);
    }
    if validate_delivery_policy(policy).is_err() {
        issues.push(DeliveryIssue::InvalidPolicy);
    }
    finish_validation(issues)
}

pub fn validate_delivery_state(
    state: &DeliveryState,
    manifest: &DeliveryManifest,
    policy: &DeliveryPolicy,
) -> Result<(), Vec<DeliveryIssue>> {
    let mut issues = Vec::with_capacity(MAX_VALIDATION_ISSUES);
    if state.schema != DELIVERY_STATE_SCHEMA
        || !valid_delivery_id(&state.queue_id)
        || state.service_generation != manifest.service_generation
        || state.consistency_epoch == 0
        || state.next_sequence == 0
        || state.next_fencing_token == 0
    {
        issues.push(DeliveryIssue::InvalidState);
    }
    if state.policy_ref != manifest.policy_ref || state.policy_ref != identify_delivery_policy(policy) {
        issues.push(DeliveryIssue::PolicyMismatch);
    }
    state::validate_state_collections(state, policy, &mut issues);
    finish_validation(issues)
}

pub fn validate_delivery_request(
    request: &DeliveryRequest,
    state: &DeliveryState,
    manifest: &DeliveryManifest,
    policy: &DeliveryPolicy,
) -> Result<(), Vec<DeliveryIssue>> {
    let mut issues = Vec::with_capacity(MAX_VALIDATION_ISSUES);
    if request.schema != DELIVERY_REQUEST_SCHEMA {
        issues.push(DeliveryIssue::SchemaMismatch);
    }
    if request.queue_id != state.queue_id {
        issues.push(DeliveryIssue::QueueMismatch);
    }
    if request.service_generation != state.service_generation
        || request.service_generation != manifest.service_generation
    {
        issues.push(DeliveryIssue::GenerationMismatch);
    }
    if request.consistency_epoch != state.consistency_epoch {
        issues.push(DeliveryIssue::ConsistencyEpochMismatch);
    }
    if !request.currentness.is_current() {
        issues.push(DeliveryIssue::CurrentnessRequired);
    }
    if request.engine_epoch == 0 {
        issues.push(DeliveryIssue::EngineEpochRequired);
    }
    if request.time_profile_ref != manifest.time_profile_ref {
        issues.push(DeliveryIssue::LogicalTimeProfileMismatch);
    }
    if request.logical_tick > MAX_DELIVERY_TICKS {
        issues.push(DeliveryIssue::LogicalTimeRequired);
    }
    if !valid_delivery_ref(&request.operation_id) || !valid_delivery_id(&request.actor_id) {
        issues.push(DeliveryIssue::InvalidIdentifier);
    }
    request::validate_required_refs(request, manifest, policy, &mut issues);
    request::validate_operation(&request.operation, state, policy, &mut issues);
    finish_validation(issues)
}

#[must_use]
pub fn required_delivery_non_claims() -> Vec<String> {
    REQUIRED_DELIVERY_NON_CLAIMS.iter().map(|value| (*value).to_string()).collect()
}

pub(crate) fn valid_delivery_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_DELIVERY_ID_BYTES
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

pub(crate) fn valid_delivery_ref(value: &str) -> bool {
    if value.len() > MAX_DELIVERY_REF_BYTES {
        return false;
    }
    let Some(hex) = value.strip_prefix(BLAKE3_REF_PREFIX) else {
        return false;
    };
    hex.len() == BLAKE3_HEX_LENGTH && hex.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(crate) fn valid_failure_class(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_DELIVERY_CLASS_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_'))
}

fn exceeds_bound(length: usize, maximum: u32) -> bool {
    u32::try_from(length).map_or(true, |length| length > maximum)
}

fn finish_validation(mut issues: Vec<DeliveryIssue>) -> Result<(), Vec<DeliveryIssue>> {
    assert!(issues.len() <= MAX_VALIDATION_ISSUES);
    issues.sort_by_key(DeliveryIssue::code);
    issues.dedup();
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}
