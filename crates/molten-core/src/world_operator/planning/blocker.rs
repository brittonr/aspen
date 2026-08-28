use std::collections::BTreeSet;

use super::super::*;

const MAX_REQUIRED_OBSERVATIONS_PER_OPERATION: usize = 7;

type ObservationRequirement = (WorldExpectedObservationKind, String, WorldWorkflowBlockerCode);

#[allow(
    tigerstyle::unbounded_collection_growth,
    reason = "the blocked-id set is bounded by the validated MAX_WORLD_OPERATOR_OPERATIONS request limit"
)]
pub(super) fn plan_operations(
    request: &WorldWorkflowRequest,
    ordered: Vec<WorldOperationRequest>,
) -> Vec<WorldOperationPlanNode> {
    let mut blocked = BTreeSet::new();
    ordered
        .into_iter()
        .map(|mut operation| {
            operation.dependencies.sort();
            let blocker = blocker_for_operation(request, &operation, &blocked);
            if blocker.is_some() {
                blocked.insert(operation.operation_id.clone());
            }
            WorldOperationPlanNode {
                operation_id: operation.operation_id,
                kind: operation.kind,
                subject_ref: operation.subject_ref,
                profile_ref: operation.profile_ref,
                dependencies: operation.dependencies,
                state: if blocker.is_some() {
                    WorldOperationPlanState::Blocked
                } else {
                    WorldOperationPlanState::Ready
                },
                blocker,
            }
        })
        .collect()
}

fn blocker_for_operation(
    request: &WorldWorkflowRequest,
    operation: &WorldOperationRequest,
    blocked: &BTreeSet<String>,
) -> Option<WorldWorkflowBlocker> {
    if let Some(dependency) = operation.dependencies.iter().find(|dependency| blocked.contains(*dependency)) {
        return Some(blocker(operation, WorldWorkflowBlockerCode::DependencyBlocked, Some(dependency.clone())));
    }
    let Some(profile) = request.profiles.iter().find(|profile| profile.profile_ref == operation.profile_ref) else {
        return Some(blocker(operation, WorldWorkflowBlockerCode::ProfileUnavailable, None));
    };
    if profile.status != WorldProfileStatus::Admitted {
        return Some(profile_blocker(operation, profile));
    }
    if profile.kind == WorldProfileKind::Opaque && operation.kind.is_semantic_comparison() {
        return Some(blocker(
            operation,
            WorldWorkflowBlockerCode::OpaqueSemanticOperation,
            Some(profile.status_ref.clone()),
        ));
    }
    observation_blocker(request, operation, profile)
}

fn profile_blocker(operation: &WorldOperationRequest, profile: &WorldProfileCapability) -> WorldWorkflowBlocker {
    let code = match profile.status {
        WorldProfileStatus::Admitted => WorldWorkflowBlockerCode::ProfileUnavailable,
        WorldProfileStatus::Blocked => WorldWorkflowBlockerCode::ProfileBlocked,
        WorldProfileStatus::Unsupported => WorldWorkflowBlockerCode::ProfileUnsupported,
        WorldProfileStatus::Unavailable => WorldWorkflowBlockerCode::ProfileUnavailable,
    };
    blocker(operation, code, Some(profile.status_ref.clone()))
}

fn observation_blocker(
    request: &WorldWorkflowRequest,
    operation: &WorldOperationRequest,
    profile: &WorldProfileCapability,
) -> Option<WorldWorkflowBlocker> {
    for (kind, subject, code) in required_observations(request, operation, profile) {
        let observation = find_observation(request, kind, &subject);
        if observation.is_none_or(|observation| !observation.admitted) {
            return Some(blocker(operation, code, observation.map(|observation| observation.observation_ref.clone())));
        }
    }
    None
}

fn required_observations(
    request: &WorldWorkflowRequest,
    operation: &WorldOperationRequest,
    profile: &WorldProfileCapability,
) -> Vec<ObservationRequirement> {
    let mut required = Vec::with_capacity(MAX_REQUIRED_OBSERVATIONS_PER_OPERATION);
    required.push((
        WorldExpectedObservationKind::Profile,
        operation.profile_ref.clone(),
        WorldWorkflowBlockerCode::ProfileObservationDenied,
    ));
    if operation.kind.is_mutating() {
        required.extend([
            (
                WorldExpectedObservationKind::Head,
                request.expected_head.as_str().to_string(),
                WorldWorkflowBlockerCode::HeadObservationDenied,
            ),
            (
                WorldExpectedObservationKind::Policy,
                request.policy_ref.as_str().to_string(),
                WorldWorkflowBlockerCode::PolicyObservationDenied,
            ),
            (
                WorldExpectedObservationKind::Authority,
                operation.subject_ref.clone(),
                WorldWorkflowBlockerCode::AuthorityObservationDenied,
            ),
        ]);
    }
    append_operation_observations(operation, &mut required);
    append_profile_observation(operation, profile, &mut required);
    required
}

#[allow(
    tigerstyle::borrowed_argument_types,
    reason = "this internal helper must append to the caller-owned bounded observation vector"
)]
fn append_operation_observations(operation: &WorldOperationRequest, required: &mut Vec<ObservationRequirement>) {
    match operation.kind {
        WorldOperationKind::Replay | WorldOperationKind::Export | WorldOperationKind::Import => {
            required.push((
                WorldExpectedObservationKind::CapsuleClosure,
                operation.subject_ref.clone(),
                WorldWorkflowBlockerCode::CapsuleIncomplete,
            ));
        }
        WorldOperationKind::Promote => {
            required.push((
                WorldExpectedObservationKind::Conflict,
                operation.subject_ref.clone(),
                WorldWorkflowBlockerCode::ConflictUnresolved,
            ));
            required.push((
                WorldExpectedObservationKind::Effect,
                operation.subject_ref.clone(),
                WorldWorkflowBlockerCode::EffectObservationDenied,
            ));
        }
        WorldOperationKind::GarbageCollectionPlan => {
            required.push((
                WorldExpectedObservationKind::Retention,
                operation.subject_ref.clone(),
                WorldWorkflowBlockerCode::RetentionObservationDenied,
            ));
        }
        WorldOperationKind::Inspect
        | WorldOperationKind::Checkpoint
        | WorldOperationKind::Branch
        | WorldOperationKind::Run
        | WorldOperationKind::Diff
        | WorldOperationKind::Conflicts
        | WorldOperationKind::Simulate
        | WorldOperationKind::Verify => {}
    }
}

#[allow(
    tigerstyle::borrowed_argument_types,
    reason = "this internal helper must append to the caller-owned bounded observation vector"
)]
fn append_profile_observation(
    operation: &WorldOperationRequest,
    profile: &WorldProfileCapability,
    required: &mut Vec<ObservationRequirement>,
) {
    match profile.kind {
        WorldProfileKind::WitnessedHead => required.push((
            WorldExpectedObservationKind::Witness,
            operation.subject_ref.clone(),
            WorldWorkflowBlockerCode::WitnessUnavailable,
        )),
        WorldProfileKind::ExecutableExtent => required.push((
            WorldExpectedObservationKind::ExecutableExtent,
            operation.subject_ref.clone(),
            WorldWorkflowBlockerCode::ExecutableExtentUnavailable,
        )),
        WorldProfileKind::Logical | WorldProfileKind::Opaque => {}
    }
}

fn find_observation<'a>(
    request: &'a WorldWorkflowRequest,
    kind: WorldExpectedObservationKind,
    subject_ref: &str,
) -> Option<&'a WorldExpectedObservation> {
    request.observations.iter().find(|observation| {
        observation.kind == kind
            && observation.subject_ref == subject_ref
            && (kind != WorldExpectedObservationKind::Authority
                || observation.observation_ref == request.authority_observation_ref)
    })
}

fn blocker(
    operation: &WorldOperationRequest,
    code: WorldWorkflowBlockerCode,
    evidence_ref: Option<String>,
) -> WorldWorkflowBlocker {
    WorldWorkflowBlocker {
        operation_id: operation.operation_id.clone(),
        code,
        evidence_ref,
    }
}
