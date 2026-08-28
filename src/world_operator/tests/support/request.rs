use molten_core::world_commit::WorldCommitRef;
use molten_core::world_head::WorldBranchId;
use molten_core::world_head::WorldHeadPolicyRef;
use molten_core::world_operator::*;

const EXPECTED_GENERATION: u64 = 11;
pub(crate) const STALE_GENERATION: u64 = EXPECTED_GENERATION + 1;

pub(crate) fn logical_request() -> WorldWorkflowRequest {
    let world_ref = reference("logical-world");
    let profile_ref = reference("logical-profile");
    let mut request = request_base(&world_ref, &profile_ref, WorldProfileKind::Logical);
    let mut prior = None;
    for kind in operation_kinds() {
        let operation_id = reference(&format!("logical-operation:{}", kind.as_str()));
        request.operations.push(WorldOperationRequest {
            operation_id: operation_id.clone(),
            kind,
            subject_ref: world_ref.clone(),
            profile_ref: profile_ref.clone(),
            dependencies: prior.into_iter().collect(),
        });
        prior = Some(operation_id);
    }
    add_observations(&mut request, &world_ref, &profile_ref);
    request
}

pub(crate) fn single_request(kind: WorldOperationKind, profile_kind: WorldProfileKind) -> WorldWorkflowRequest {
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
    add_observations(&mut request, &world_ref, &profile_ref);
    add_profile_observation(&mut request, profile_kind, &world_ref);
    request
}

fn add_profile_observation(request: &mut WorldWorkflowRequest, profile_kind: WorldProfileKind, world_ref: &str) {
    match profile_kind {
        WorldProfileKind::WitnessedHead => add_observation(request, WorldExpectedObservationKind::Witness, world_ref),
        WorldProfileKind::ExecutableExtent => {
            add_observation(request, WorldExpectedObservationKind::ExecutableExtent, world_ref)
        }
        WorldProfileKind::Logical | WorldProfileKind::Opaque => {}
    }
}

fn request_base(world_ref: &str, profile_ref: &str, profile_kind: WorldProfileKind) -> WorldWorkflowRequest {
    WorldWorkflowRequest {
        schema: WORLD_WORKFLOW_REQUEST_SCHEMA.to_string(),
        request_ref: reference("shell-request"),
        world_ref: WorldCommitRef::new(world_ref.to_string()).expect("world ref"),
        branch_id: WorldBranchId::new("world/operator-dogfood".to_string()).expect("branch id"),
        expected_head: WorldCommitRef::new(world_ref.to_string()).expect("head ref"),
        expected_generation: EXPECTED_GENERATION,
        policy_ref: WorldHeadPolicyRef::new(reference("shell-policy")).expect("policy ref"),
        authority_observation_ref: reference("shell-authority-observation"),
        limits: WorldWorkflowLimits::standard(reference("shell-limits")),
        profiles: vec![WorldProfileCapability {
            profile_ref: profile_ref.to_string(),
            kind: profile_kind,
            status: WorldProfileStatus::Admitted,
            status_ref: reference("shell-profile-status"),
        }],
        observations: Vec::new(),
        operations: Vec::new(),
    }
}

fn add_observations(request: &mut WorldWorkflowRequest, world_ref: &str, profile_ref: &str) {
    add_observation(request, WorldExpectedObservationKind::Profile, profile_ref);
    add_observation(request, WorldExpectedObservationKind::Head, world_ref);
    let policy_ref = request.policy_ref.as_str().to_string();
    add_observation(request, WorldExpectedObservationKind::Policy, &policy_ref);
    add_observation(request, WorldExpectedObservationKind::Authority, world_ref);
    add_observation(request, WorldExpectedObservationKind::Conflict, world_ref);
    add_observation(request, WorldExpectedObservationKind::Effect, world_ref);
    add_observation(request, WorldExpectedObservationKind::CapsuleClosure, world_ref);
    add_observation(request, WorldExpectedObservationKind::Retention, world_ref);
}

fn add_observation(request: &mut WorldWorkflowRequest, kind: WorldExpectedObservationKind, subject_ref: &str) {
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
        reference(&format!("shell-observation:{}:{subject_ref}", kind.as_str()))
    };
    request.observations.push(WorldExpectedObservation {
        kind,
        observation_ref,
        subject_ref: subject_ref.to_string(),
        admitted: true,
    });
}

pub(crate) fn mutating_operation_count(plan: &WorldWorkflowPlan) -> usize {
    plan.operations.iter().filter(|operation| operation.kind.is_mutating()).count()
}

pub(crate) fn operation_kinds() -> Vec<WorldOperationKind> {
    vec![
        WorldOperationKind::Inspect,
        WorldOperationKind::Checkpoint,
        WorldOperationKind::Branch,
        WorldOperationKind::Simulate,
        WorldOperationKind::Run,
        WorldOperationKind::Diff,
        WorldOperationKind::Conflicts,
        WorldOperationKind::Replay,
        WorldOperationKind::Verify,
        WorldOperationKind::Promote,
        WorldOperationKind::Export,
        WorldOperationKind::Import,
        WorldOperationKind::GarbageCollectionPlan,
    ]
}

pub(crate) fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}
