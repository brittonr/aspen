use std::path::Path;

use molten::error::MoltenError;
use molten::error::Result;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_head::WorldBranchId;
use molten_core::world_head::WorldHeadPolicyRef;
use molten_core::world_operator::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WorkflowDocument {
    schema: String,
    request_ref: String,
    world_ref: String,
    branch_id: String,
    expected_head: String,
    expected_generation: u64,
    policy_ref: String,
    authority_observation_ref: String,
    limits: LimitsDocument,
    profiles: Vec<ProfileDocument>,
    observations: Vec<ObservationDocument>,
    operations: Vec<OperationDocument>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LimitsDocument {
    limits_ref: String,
    max_operations: usize,
    max_dependencies_per_operation: usize,
    max_receipt_links: usize,
    max_canonical_bytes: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileDocument {
    profile_ref: String,
    kind: String,
    status: String,
    status_ref: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationDocument {
    kind: String,
    observation_ref: String,
    subject_ref: String,
    admitted: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationDocument {
    operation_id: String,
    kind: String,
    subject_ref: String,
    profile_ref: String,
    dependencies: Vec<String>,
}

pub(super) fn read_world_workflow_request(path: &Path) -> Result<WorldWorkflowRequest> {
    let document: WorkflowDocument = serde_json::from_slice(&std::fs::read(path)?)
        .map_err(|error| MoltenError::invalid_harness(format!("parse world workflow request: {error}")))?;
    document.into_request()
}

impl WorkflowDocument {
    pub(super) fn into_request(self) -> Result<WorldWorkflowRequest> {
        Ok(WorldWorkflowRequest {
            schema: self.schema,
            request_ref: self.request_ref,
            world_ref: WorldCommitRef::new(self.world_ref).map_err(commit_reference_error)?,
            branch_id: WorldBranchId::new(self.branch_id).map_err(head_reference_error)?,
            expected_head: WorldCommitRef::new(self.expected_head).map_err(commit_reference_error)?,
            expected_generation: self.expected_generation,
            policy_ref: WorldHeadPolicyRef::new(self.policy_ref).map_err(head_reference_error)?,
            authority_observation_ref: self.authority_observation_ref,
            limits: self.limits.into_limits(),
            profiles: self.profiles.into_iter().map(ProfileDocument::into_profile).collect::<Result<Vec<_>>>()?,
            observations: self
                .observations
                .into_iter()
                .map(ObservationDocument::into_observation)
                .collect::<Result<Vec<_>>>()?,
            operations: self
                .operations
                .into_iter()
                .map(OperationDocument::into_operation)
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

impl LimitsDocument {
    fn into_limits(self) -> WorldWorkflowLimits {
        WorldWorkflowLimits {
            limits_ref: self.limits_ref,
            max_operations: self.max_operations,
            max_dependencies_per_operation: self.max_dependencies_per_operation,
            max_receipt_links: self.max_receipt_links,
            max_canonical_bytes: self.max_canonical_bytes,
        }
    }
}

impl ProfileDocument {
    fn into_profile(self) -> Result<WorldProfileCapability> {
        Ok(WorldProfileCapability {
            profile_ref: self.profile_ref,
            kind: parse_profile_kind(&self.kind)?,
            status: parse_profile_status(&self.status)?,
            status_ref: self.status_ref,
        })
    }
}

impl ObservationDocument {
    fn into_observation(self) -> Result<WorldExpectedObservation> {
        Ok(WorldExpectedObservation {
            kind: parse_observation_kind(&self.kind)?,
            observation_ref: self.observation_ref,
            subject_ref: self.subject_ref,
            admitted: self.admitted,
        })
    }
}

impl OperationDocument {
    fn into_operation(self) -> Result<WorldOperationRequest> {
        Ok(WorldOperationRequest {
            operation_id: self.operation_id,
            kind: parse_operation_kind(&self.kind)?,
            subject_ref: self.subject_ref,
            profile_ref: self.profile_ref,
            dependencies: self.dependencies,
        })
    }
}

fn parse_operation_kind(value: &str) -> Result<WorldOperationKind> {
    match value {
        "inspect" => Ok(WorldOperationKind::Inspect),
        "checkpoint" => Ok(WorldOperationKind::Checkpoint),
        "branch" => Ok(WorldOperationKind::Branch),
        "run" => Ok(WorldOperationKind::Run),
        "diff" => Ok(WorldOperationKind::Diff),
        "conflicts" => Ok(WorldOperationKind::Conflicts),
        "replay" => Ok(WorldOperationKind::Replay),
        "simulate" => Ok(WorldOperationKind::Simulate),
        "verify" => Ok(WorldOperationKind::Verify),
        "promote" => Ok(WorldOperationKind::Promote),
        "export" => Ok(WorldOperationKind::Export),
        "import" => Ok(WorldOperationKind::Import),
        "gc-plan" => Ok(WorldOperationKind::GarbageCollectionPlan),
        _ => Err(MoltenError::invalid_harness("unsupported world workflow operation")),
    }
}

fn parse_profile_kind(value: &str) -> Result<WorldProfileKind> {
    match value {
        "logical" => Ok(WorldProfileKind::Logical),
        "opaque" => Ok(WorldProfileKind::Opaque),
        "witnessed-head" => Ok(WorldProfileKind::WitnessedHead),
        "executable-extent" => Ok(WorldProfileKind::ExecutableExtent),
        _ => Err(MoltenError::invalid_harness("unsupported world workflow profile kind")),
    }
}

fn parse_profile_status(value: &str) -> Result<WorldProfileStatus> {
    match value {
        "admitted" => Ok(WorldProfileStatus::Admitted),
        "blocked" => Ok(WorldProfileStatus::Blocked),
        "unsupported" => Ok(WorldProfileStatus::Unsupported),
        "unavailable" => Ok(WorldProfileStatus::Unavailable),
        _ => Err(MoltenError::invalid_harness("unsupported world workflow profile status")),
    }
}

fn parse_observation_kind(value: &str) -> Result<WorldExpectedObservationKind> {
    match value {
        "head" => Ok(WorldExpectedObservationKind::Head),
        "policy" => Ok(WorldExpectedObservationKind::Policy),
        "authority" => Ok(WorldExpectedObservationKind::Authority),
        "profile" => Ok(WorldExpectedObservationKind::Profile),
        "conflict" => Ok(WorldExpectedObservationKind::Conflict),
        "effect" => Ok(WorldExpectedObservationKind::Effect),
        "capsule-closure" => Ok(WorldExpectedObservationKind::CapsuleClosure),
        "retention" => Ok(WorldExpectedObservationKind::Retention),
        "witness" => Ok(WorldExpectedObservationKind::Witness),
        "executable-extent" => Ok(WorldExpectedObservationKind::ExecutableExtent),
        _ => Err(MoltenError::invalid_harness("unsupported world workflow observation kind")),
    }
}

fn head_reference_error(error: molten_core::world_head::WorldHeadReferenceError) -> MoltenError {
    MoltenError::invalid_harness(format!("invalid world head reference: {error}"))
}

fn commit_reference_error(error: molten_core::world_commit::WorldCommitReferenceError) -> MoltenError {
    MoltenError::invalid_harness(format!("invalid world commit reference: {error:?}"))
}
