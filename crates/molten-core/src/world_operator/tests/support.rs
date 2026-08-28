use super::super::*;
use crate::world_commit::WorldCommitRef;
use crate::world_head::WorldBranchId;
use crate::world_head::WorldHeadPolicyRef;

const EXPECTED_GENERATION: u64 = 7;
pub(super) const STALE_GENERATION: u64 = EXPECTED_GENERATION + 1;

pub(super) fn assert_receipt_issue(plan: &WorldWorkflowPlan, link: WorldReceiptLink, expected: WorldWorkflowIssue) {
    let issues = build_world_workflow_receipt(plan, vec![link], None).expect_err("receipt overclaim denied");
    assert!(issues.contains(&expected), "issues={issues:?}");
}

pub(super) fn logical_request() -> WorldWorkflowRequest {
    let world_ref = reference("logical-world");
    let profile_ref = reference("logical-profile");
    let mut request = request_base(&world_ref, &profile_ref, WorldProfileKind::Logical);
    let mut prior = None;
    for kind in WorldOperationKind::all_for_tests() {
        let operation_id = reference(&format!("operation:{}", kind.as_str()));
        let dependencies = prior.into_iter().collect();
        request.operations.push(WorldOperationRequest {
            operation_id: operation_id.clone(),
            kind,
            subject_ref: world_ref.clone(),
            profile_ref: profile_ref.clone(),
            dependencies,
        });
        prior = Some(operation_id);
    }
    add_standard_observations(&mut request, &world_ref, &profile_ref);
    request
}

pub(super) fn single_operation_request(
    kind: WorldOperationKind,
    profile_kind: WorldProfileKind,
) -> WorldWorkflowRequest {
    let world_ref = reference("single-world");
    let profile_ref = reference(&format!("single-profile:{}", profile_kind.as_str()));
    let mut request = request_base(&world_ref, &profile_ref, profile_kind);
    request.operations.push(WorldOperationRequest {
        operation_id: reference(&format!("single-operation:{}", kind.as_str())),
        kind,
        subject_ref: world_ref.clone(),
        profile_ref: profile_ref.clone(),
        dependencies: Vec::new(),
    });
    add_standard_observations(&mut request, &world_ref, &profile_ref);
    add_kind_observations(&mut request, kind, &world_ref);
    request
}

fn add_kind_observations(request: &mut WorldWorkflowRequest, kind: WorldOperationKind, world_ref: &str) {
    match kind {
        WorldOperationKind::Promote => {
            add_observation(request, WorldExpectedObservationKind::Conflict, world_ref, true);
            add_observation(request, WorldExpectedObservationKind::Effect, world_ref, true);
        }
        WorldOperationKind::Replay | WorldOperationKind::Export | WorldOperationKind::Import => {
            add_observation(request, WorldExpectedObservationKind::CapsuleClosure, world_ref, true);
        }
        WorldOperationKind::GarbageCollectionPlan => {
            add_observation(request, WorldExpectedObservationKind::Retention, world_ref, true);
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

fn request_base(world_ref: &str, profile_ref: &str, profile_kind: WorldProfileKind) -> WorldWorkflowRequest {
    WorldWorkflowRequest {
        schema: WORLD_WORKFLOW_REQUEST_SCHEMA.to_string(),
        request_ref: reference("workflow-request"),
        world_ref: WorldCommitRef::new(world_ref.to_string()).expect("world ref"),
        branch_id: WorldBranchId::new("world/operator-candidate".to_string()).expect("branch id"),
        expected_head: WorldCommitRef::new(world_ref.to_string()).expect("expected head"),
        expected_generation: EXPECTED_GENERATION,
        policy_ref: WorldHeadPolicyRef::new(reference("world-policy")).expect("policy ref"),
        authority_observation_ref: reference("authority-observation"),
        limits: WorldWorkflowLimits::standard(reference("workflow-limits")),
        profiles: vec![WorldProfileCapability {
            profile_ref: profile_ref.to_string(),
            kind: profile_kind,
            status: WorldProfileStatus::Admitted,
            status_ref: reference("profile-status"),
        }],
        observations: Vec::new(),
        operations: Vec::new(),
    }
}

fn add_standard_observations(request: &mut WorldWorkflowRequest, world_ref: &str, profile_ref: &str) {
    add_observation(request, WorldExpectedObservationKind::Profile, profile_ref, true);
    add_observation(request, WorldExpectedObservationKind::Head, world_ref, true);
    let policy_ref = request.policy_ref.as_str().to_string();
    add_observation(request, WorldExpectedObservationKind::Policy, &policy_ref, true);
    add_observation(request, WorldExpectedObservationKind::Authority, world_ref, true);
    add_observation(request, WorldExpectedObservationKind::Conflict, world_ref, true);
    add_observation(request, WorldExpectedObservationKind::Effect, world_ref, true);
    add_observation(request, WorldExpectedObservationKind::CapsuleClosure, world_ref, true);
    add_observation(request, WorldExpectedObservationKind::Retention, world_ref, true);
}

pub(super) fn add_observation(
    request: &mut WorldWorkflowRequest,
    kind: WorldExpectedObservationKind,
    subject_ref: &str,
    admitted: bool,
) {
    if request
        .observations
        .iter()
        .any(|observation| observation.kind == kind && observation.subject_ref == subject_ref)
    {
        return;
    }
    let observation_ref = if kind == WorldExpectedObservationKind::Authority {
        request.authority_observation_ref.clone()
    } else {
        reference(&format!("observation:{}:{subject_ref}", kind.as_str()))
    };
    request.observations.push(WorldExpectedObservation {
        kind,
        observation_ref,
        subject_ref: subject_ref.to_string(),
        admitted,
    });
}

pub(super) fn current_facts(
    plan: &WorldWorkflowPlan,
    operation: &WorldOperationPlanNode,
) -> WorldOperationCurrentFacts {
    WorldOperationCurrentFacts {
        plan_ref: plan.plan_ref.clone(),
        operation_id: operation.operation_id.clone(),
        observed_head: plan.expected_head.clone(),
        observed_generation: plan.expected_generation,
        policy_ref: plan.policy_ref.clone(),
        authority_observation_ref: plan.authority_observation_ref.clone(),
        profile_ref: operation.profile_ref.clone(),
        profile_status: WorldProfileStatus::Admitted,
    }
}

pub(super) fn completed_link(operation: &WorldOperationPlanNode, role: WorldReceiptRole) -> WorldReceiptLink {
    WorldReceiptLink {
        operation_id: operation.operation_id.clone(),
        kind: operation.kind,
        owner: operation.kind.owner(),
        role,
        component_ref: reference(&format!("component:{}:{}", operation.kind.as_str(), role.as_str())),
        state: WorldComponentCompletionState::Complete,
        sensitive_material_present: false,
        claims_authority: false,
        claims_deletion_authority: false,
    }
}

pub(super) fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

impl WorldOperationKind {
    pub(super) fn all_for_tests() -> Vec<Self> {
        vec![
            Self::Inspect,
            Self::Checkpoint,
            Self::Branch,
            Self::Simulate,
            Self::Run,
            Self::Diff,
            Self::Conflicts,
            Self::Replay,
            Self::Verify,
            Self::Promote,
            Self::Export,
            Self::Import,
            Self::GarbageCollectionPlan,
        ]
    }
}
