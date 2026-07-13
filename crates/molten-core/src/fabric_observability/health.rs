use std::collections::BTreeMap;

use super::*;
use crate::fabric::valid_blake3_ref;
use crate::fabric::valid_fabric_token;

// r[impl molten.fabric_observability.pure_core]
// r[impl molten.fabric_observability.health_scope]
pub fn evaluate_health_readiness(
    profile: &ObservationProfile,
    policy: &ReadinessPolicy,
    prior_state: HealthState,
    inputs: &[HealthInput],
) -> HealthDecision {
    let mut issues = validate_observation_profile(profile);
    validate_readiness_policy(profile, policy, &mut issues);
    let index = index_health_inputs(profile, inputs, &mut issues);
    let mut supporting_refs = Vec::new();
    let mut state = HealthState::Healthy;
    let mut strongest_scope = ClaimScope::LocalComponent;
    for source_id in &policy.required_source_ids {
        match index.get(source_id) {
            Some(input) => {
                supporting_refs.push(input.health_ref.clone());
                strongest_scope = strongest_scope.max(input.context.scope);
                if policy.as_of_tick > input.context.valid_until_tick {
                    issues.push(ObservabilityIssue::ObservationStale(source_id.clone()));
                    state = worse_state(state, HealthState::Unavailable);
                } else {
                    state = worse_state(state, input.state);
                    if input.state == HealthState::Unavailable {
                        issues.push(ObservabilityIssue::ObservationUnavailable(source_id.clone()));
                    }
                }
            }
            None => {
                issues.push(ObservabilityIssue::RequiredSourceMissing(source_id.clone()));
                state = worse_state(state, HealthState::Unavailable);
            }
        }
    }
    if strongest_scope < policy.target_scope && policy.scope_evidence_refs.is_empty() {
        issues.push(ObservabilityIssue::ClaimScopeOverreach);
    }
    supporting_refs.sort();
    supporting_refs.dedup();
    let readiness = readiness_decision(state, policy.allow_degraded, &issues);
    HealthDecision {
        prior_state,
        state,
        readiness,
        scope: policy.target_scope,
        supporting_health_refs: supporting_refs,
        issues,
    }
}

// r[impl molten.fabric_observability.health_scope]
pub const fn observation_authority_decision() -> AuthorityDecision {
    AuthorityDecision::Deny
}

pub fn validate_readiness_policy(
    profile: &ObservationProfile,
    policy: &ReadinessPolicy,
    issues: &mut Vec<ObservabilityIssue>,
) {
    if policy.schema != READINESS_POLICY_SCHEMA {
        issues.push(ObservabilityIssue::SchemaMismatch("readiness-policy-schema"));
    }
    if !valid_blake3_ref(&policy.policy_ref) {
        issues.push(ObservabilityIssue::MalformedRef("readiness-policy-ref"));
    }
    if policy.required_source_ids.is_empty() {
        issues.push(ObservabilityIssue::EmptyField("readiness-required-sources"));
    }
    if policy.required_source_ids.len() > profile.bounds.max_descriptors {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("readiness-required-sources"));
    }
    if policy.required_source_ids.windows(ADJACENT_PAIR_WIDTH).any(|pair| pair[0] >= pair[1]) {
        issues.push(ObservabilityIssue::DuplicateValue("readiness-required-source"));
    }
    for source_id in &policy.required_source_ids {
        if !valid_fabric_token(source_id) {
            issues.push(ObservabilityIssue::MalformedToken("readiness-required-source"));
        }
    }
    validate_scope_evidence(policy, issues);
}

fn validate_scope_evidence(policy: &ReadinessPolicy, issues: &mut Vec<ObservabilityIssue>) {
    if policy.scope_evidence_refs.len() > MAX_OBSERVATION_REFS {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("readiness-scope-evidence"));
    }
    if policy.scope_evidence_refs.windows(ADJACENT_PAIR_WIDTH).any(|pair| pair[0] >= pair[1]) {
        issues.push(ObservabilityIssue::DuplicateValue("readiness-scope-evidence"));
    }
    for evidence_ref in &policy.scope_evidence_refs {
        if !valid_blake3_ref(evidence_ref) {
            issues.push(ObservabilityIssue::MalformedRef("readiness-scope-evidence"));
        }
    }
}

fn index_health_inputs<'a>(
    profile: &ObservationProfile,
    inputs: &'a [HealthInput],
    issues: &mut Vec<ObservabilityIssue>,
) -> BTreeMap<String, &'a HealthInput> {
    let mut index = BTreeMap::new();
    if inputs.len() > profile.bounds.max_descriptors {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("health-inputs"));
    }
    for input in inputs {
        validate_health_input(profile, input, issues);
        if index.insert(input.context.source_id.clone(), input).is_some() {
            issues.push(ObservabilityIssue::DuplicateValue("health-input-source"));
        }
    }
    index
}

pub fn validate_health_input(profile: &ObservationProfile, input: &HealthInput, issues: &mut Vec<ObservabilityIssue>) {
    if input.schema != HEALTH_INPUT_SCHEMA {
        issues.push(ObservabilityIssue::SchemaMismatch("health-input-schema"));
    }
    if !valid_blake3_ref(&input.health_ref) {
        issues.push(ObservabilityIssue::MalformedRef("health-input-ref"));
    }
    issues.extend(validate_context(profile, &input.context));
    if input.diagnostic_refs.len() > profile.bounds.max_diagnostics {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("health-diagnostics"));
    }
    if input.diagnostic_refs.windows(ADJACENT_PAIR_WIDTH).any(|pair| pair[0] >= pair[1]) {
        issues.push(ObservabilityIssue::DuplicateValue("health-diagnostic-ref"));
    }
    for diagnostic_ref in &input.diagnostic_refs {
        if !valid_blake3_ref(diagnostic_ref) {
            issues.push(ObservabilityIssue::MalformedRef("health-diagnostic-ref"));
        }
    }
}

fn worse_state(left: HealthState, right: HealthState) -> HealthState {
    if right.severity() > left.severity() {
        right
    } else {
        left
    }
}

fn readiness_decision(state: HealthState, allow_degraded: bool, issues: &[ObservabilityIssue]) -> ReadinessDecision {
    if issues.iter().any(|issue| {
        !matches!(
            issue,
            ObservabilityIssue::ObservationStale(_)
                | ObservabilityIssue::ObservationUnavailable(_)
                | ObservabilityIssue::RequiredSourceMissing(_)
        )
    }) {
        return ReadinessDecision::Deny;
    }
    match state {
        HealthState::Healthy => ReadinessDecision::Pass,
        HealthState::Degraded if allow_degraded => ReadinessDecision::Degraded,
        HealthState::Unavailable => ReadinessDecision::Unavailable,
        HealthState::Degraded | HealthState::Failed => ReadinessDecision::Deny,
    }
}
