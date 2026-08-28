use std::collections::BTreeSet;

use super::*;

mod blocker;
mod order;

use blocker::plan_operations;
use order::order_operations;

const BLAKE3_REFERENCE_PREFIX: &str = "blake3:";
const BLAKE3_HEX_DIGITS: usize = 64;

// r[impl molten.world_operator.plan]
pub fn plan_world_workflow(request: &WorldWorkflowRequest) -> Result<WorldWorkflowPlan, Vec<WorldWorkflowIssue>> {
    let mut issues = validate_world_workflow_request(request);
    if !issues.is_empty() {
        issues.sort();
        issues.dedup();
        return Err(issues);
    }
    let ordered = order_operations(request)?;
    let operations = plan_operations(request, ordered);
    let first_blocker = operations.iter().find_map(|operation| operation.blocker.clone());
    let plan_ref =
        identify_world_workflow_plan(request, &operations, first_blocker.as_ref()).map_err(|issue| vec![issue])?;
    Ok(WorldWorkflowPlan {
        schema: WORLD_WORKFLOW_PLAN_SCHEMA,
        plan_ref,
        request_ref: request.request_ref.clone(),
        world_ref: request.world_ref.clone(),
        branch_id: request.branch_id.clone(),
        expected_head: request.expected_head.clone(),
        expected_generation: request.expected_generation,
        policy_ref: request.policy_ref.clone(),
        authority_observation_ref: request.authority_observation_ref.clone(),
        limits_ref: request.limits.limits_ref.clone(),
        operations,
        first_blocker,
        non_claims: world_operator_non_claims(),
    })
}

pub fn validate_world_workflow_request(request: &WorldWorkflowRequest) -> Vec<WorldWorkflowIssue> {
    let mut issues = Vec::with_capacity(MAX_WORLD_OPERATOR_DIAGNOSTICS);
    validate_request_shape(request, &mut issues);
    validate_limits(request, &mut issues);
    validate_profiles(request, &mut issues);
    validate_observations(request, &mut issues);
    validate_operations(request, &mut issues);
    issues.sort();
    issues.dedup();
    issues
}

// r[impl molten.world_operator.preview_apply]
pub fn admit_world_operation_apply(
    plan: &WorldWorkflowPlan,
    facts: &WorldOperationCurrentFacts,
) -> Result<WorldOperationApplyAdmission, Vec<WorldWorkflowIssue>> {
    let mut issues = Vec::with_capacity(MAX_WORLD_OPERATOR_DIAGNOSTICS);
    let operation = plan.operations.iter().find(|operation| operation.operation_id == facts.operation_id);
    let Some(operation) = operation else {
        return Err(vec![WorldWorkflowIssue::ApplyOperationMissing]);
    };
    if !operation.kind.is_mutating() {
        issues.push(WorldWorkflowIssue::ApplyReadOnlyOperation);
    }
    if operation.state != WorldOperationPlanState::Ready {
        issues.push(WorldWorkflowIssue::ApplyProfileDenied);
    }
    compare_apply_facts(plan, operation, facts, &mut issues);
    if issues.is_empty() {
        Ok(WorldOperationApplyAdmission {
            plan_ref: plan.plan_ref.clone(),
            operation_id: operation.operation_id.clone(),
            admitted: true,
        })
    } else {
        issues.sort();
        issues.dedup();
        Err(issues)
    }
}

#[allow(
    tigerstyle::borrowed_argument_types,
    reason = "the bounded validator appends typed issues to one preallocated local sink"
)]
fn compare_apply_facts(
    plan: &WorldWorkflowPlan,
    operation: &WorldOperationPlanNode,
    facts: &WorldOperationCurrentFacts,
    issues: &mut Vec<WorldWorkflowIssue>,
) {
    if facts.plan_ref != plan.plan_ref {
        issues.push(WorldWorkflowIssue::ApplyPlanMismatch);
    }
    if facts.observed_head != plan.expected_head {
        issues.push(WorldWorkflowIssue::ApplyHeadMismatch);
    }
    if facts.observed_generation != plan.expected_generation {
        issues.push(WorldWorkflowIssue::ApplyGenerationMismatch);
    }
    if facts.policy_ref != plan.policy_ref {
        issues.push(WorldWorkflowIssue::ApplyPolicyMismatch);
    }
    if facts.authority_observation_ref != plan.authority_observation_ref {
        issues.push(WorldWorkflowIssue::ApplyAuthorityMismatch);
    }
    if facts.profile_ref != operation.profile_ref {
        issues.push(WorldWorkflowIssue::ApplyProfileMismatch);
    }
    if facts.profile_status != WorldProfileStatus::Admitted {
        issues.push(WorldWorkflowIssue::ApplyProfileDenied);
    }
}

#[allow(
    tigerstyle::borrowed_argument_types,
    reason = "the bounded validator appends typed issues to one preallocated local sink"
)]
fn validate_request_shape(request: &WorldWorkflowRequest, issues: &mut Vec<WorldWorkflowIssue>) {
    if request.schema != WORLD_WORKFLOW_REQUEST_SCHEMA {
        issues.push(WorldWorkflowIssue::InvalidSchema);
    }
    validate_ref(&request.request_ref, "request", issues);
    validate_ref(&request.authority_observation_ref, "authority-observation", issues);
    validate_ref(&request.limits.limits_ref, "limits", issues);
}

#[allow(
    tigerstyle::borrowed_argument_types,
    reason = "the bounded validator appends typed issues to one preallocated local sink"
)]
fn validate_limits(request: &WorldWorkflowRequest, issues: &mut Vec<WorldWorkflowIssue>) {
    let limits = &request.limits;
    let is_limits_valid = limits.max_operations > 0
        && limits.max_operations <= MAX_WORLD_OPERATOR_OPERATIONS
        && limits.max_dependencies_per_operation > 0
        && limits.max_dependencies_per_operation <= MAX_WORLD_OPERATOR_DEPENDENCIES
        && limits.max_receipt_links >= limits.max_operations
        && limits.max_receipt_links <= MAX_WORLD_OPERATOR_RECEIPT_LINKS
        && limits.max_canonical_bytes > 0
        && limits.max_canonical_bytes <= MAX_WORLD_OPERATOR_CANONICAL_BYTES;
    if !is_limits_valid {
        issues.push(WorldWorkflowIssue::InvalidLimits);
    }
    if request.operations.len() > limits.max_operations {
        issues.push(WorldWorkflowIssue::OperationLimitExceeded);
    }
    if request.profiles.len() > MAX_WORLD_OPERATOR_PROFILES {
        issues.push(WorldWorkflowIssue::ProfileLimitExceeded);
    }
    if request.observations.len() > MAX_WORLD_OPERATOR_OBSERVATIONS {
        issues.push(WorldWorkflowIssue::ObservationLimitExceeded);
    }
}

#[allow(
    tigerstyle::borrowed_argument_types,
    reason = "the bounded validator appends typed issues to one preallocated local sink"
)]
fn validate_profiles(request: &WorldWorkflowRequest, issues: &mut Vec<WorldWorkflowIssue>) {
    let mut profile_refs = BTreeSet::new();
    for profile in &request.profiles {
        validate_ref(&profile.profile_ref, "profile", issues);
        validate_ref(&profile.status_ref, "profile-status", issues);
        if !profile_refs.insert(profile.profile_ref.clone()) {
            issues.push(WorldWorkflowIssue::DuplicateProfile(profile.profile_ref.clone()));
        }
    }
}

#[allow(
    tigerstyle::borrowed_argument_types,
    reason = "the bounded validator appends typed issues to one preallocated local sink"
)]
fn validate_observations(request: &WorldWorkflowRequest, issues: &mut Vec<WorldWorkflowIssue>) {
    let mut observation_keys = BTreeSet::new();
    for observation in &request.observations {
        validate_ref(&observation.observation_ref, "observation", issues);
        validate_ref(&observation.subject_ref, "observation-subject", issues);
        let key = (observation.kind, observation.subject_ref.clone());
        if !observation_keys.insert(key) {
            issues.push(WorldWorkflowIssue::DuplicateObservation(observation.observation_ref.clone()));
        }
    }
}

#[allow(
    tigerstyle::borrowed_argument_types,
    reason = "the bounded validator appends typed issues to one preallocated local sink"
)]
fn validate_operations(request: &WorldWorkflowRequest, issues: &mut Vec<WorldWorkflowIssue>) {
    if request.operations.is_empty() {
        issues.push(WorldWorkflowIssue::EmptyOperations);
        return;
    }
    let operation_ids =
        request.operations.iter().map(|operation| operation.operation_id.clone()).collect::<BTreeSet<_>>();
    validate_duplicate_operations(request, &operation_ids, issues);
    for operation in &request.operations {
        validate_operation(request, operation, &operation_ids, issues);
    }
}

#[allow(
    tigerstyle::borrowed_argument_types,
    reason = "the bounded validator appends typed issues to one preallocated local sink"
)]
fn validate_duplicate_operations(
    request: &WorldWorkflowRequest,
    operation_ids: &BTreeSet<String>,
    issues: &mut Vec<WorldWorkflowIssue>,
) {
    if operation_ids.len() == request.operations.len() {
        return;
    }
    let mut seen = BTreeSet::new();
    for operation in &request.operations {
        if !seen.insert(operation.operation_id.clone()) {
            issues.push(WorldWorkflowIssue::DuplicateOperation(operation.operation_id.clone()));
        }
    }
}

#[allow(
    tigerstyle::borrowed_argument_types,
    reason = "the bounded validator appends typed issues to one preallocated local sink"
)]
fn validate_operation(
    request: &WorldWorkflowRequest,
    operation: &WorldOperationRequest,
    operation_ids: &BTreeSet<String>,
    issues: &mut Vec<WorldWorkflowIssue>,
) {
    validate_ref(&operation.operation_id, "operation", issues);
    validate_ref(&operation.subject_ref, "operation-subject", issues);
    validate_ref(&operation.profile_ref, "operation-profile", issues);
    if operation.dependencies.len() > request.limits.max_dependencies_per_operation {
        issues.push(WorldWorkflowIssue::OperationLimitExceeded);
    }
    let mut dependencies = BTreeSet::new();
    for dependency in &operation.dependencies {
        validate_ref(dependency, "operation-dependency", issues);
        if !dependencies.insert(dependency.clone()) {
            issues.push(WorldWorkflowIssue::DuplicateDependency(dependency.clone()));
        }
        if !operation_ids.contains(dependency) {
            issues.push(WorldWorkflowIssue::MissingDependency(dependency.clone()));
        }
    }
}

#[allow(
    tigerstyle::borrowed_argument_types,
    reason = "the bounded validator appends typed issues to one preallocated local sink"
)]
fn validate_ref(value: &str, field: &'static str, issues: &mut Vec<WorldWorkflowIssue>) {
    if !is_content_ref(value) {
        issues.push(WorldWorkflowIssue::InvalidReference(field));
    }
}

fn is_content_ref(value: &str) -> bool {
    let Some(digest) = value.strip_prefix(BLAKE3_REFERENCE_PREFIX) else {
        return false;
    };
    digest.len() == BLAKE3_HEX_DIGITS && digest.bytes().all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
