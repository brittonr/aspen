mod records;

use preserves::IOValue;

use self::records::*;
use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric::DeterminismClass;
use crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA;
use crate::fabric::FabricAuthority;
use crate::fabric::FabricPortClass;
use crate::fabric::FabricPortDescriptor;
use crate::fabric::FabricResource;
use crate::fabric::REQUIRED_FABRIC_NON_CLAIMS;
use crate::fabric::ReplayClass;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;

const EXECUTION_SOURCE_RECORD: &str = "fabric-execution-source-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalExecutionProfile {
    pub profile: AdmittedExecutionProfile,
    pub profile_ref: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalExecutionRequest {
    pub plan: AdmittedExecutionPlan,
    pub request_ref: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalExecutionReceipt {
    pub receipt_ref: String,
    pub request_ref: String,
    pub profile_ref: String,
    pub operation_ref: String,
    pub generation: u64,
    pub process: ExecutionProcessObservation,
    pub stdout_publication: ExecutionStreamPublication,
    pub stderr_publication: ExecutionStreamPublication,
    pub non_claims: Vec<ExecutionNonClaim>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedExecSourceCohort {
    pub repository: String,
    pub revision: String,
    pub license: String,
    pub package: String,
    pub platform: ExecutionPlatform,
    pub non_claims: Vec<ExecutionNonClaim>,
    pub source_ref: String,
    pub value: IOValue,
}

// r[impl molten.fabric_execution.component_pin]
pub fn canonical_bounded_exec_source_cohort(platform: ExecutionPlatform) -> Result<BoundedExecSourceCohort> {
    let value = record(EXECUTION_SOURCE_RECORD, vec![
        field("repository", string(BOUNDED_EXEC_REPOSITORY)),
        field("revision", string(BOUNDED_EXEC_REVISION)),
        field("license", string(BOUNDED_EXEC_LICENSE)),
        field("package", string(BOUNDED_EXEC_PACKAGE)),
        field("platform", string(platform.as_str())),
        field(
            "non-claims",
            sequence(REQUIRED_EXECUTION_NON_CLAIMS.iter().map(|claim| string(claim.as_str())).collect()),
        ),
    ]);
    let source_ref = canonical_hash(&value)?;
    Ok(BoundedExecSourceCohort {
        repository: BOUNDED_EXEC_REPOSITORY.to_string(),
        revision: BOUNDED_EXEC_REVISION.to_string(),
        license: BOUNDED_EXEC_LICENSE.to_string(),
        package: BOUNDED_EXEC_PACKAGE.to_string(),
        platform,
        non_claims: REQUIRED_EXECUTION_NON_CLAIMS.to_vec(),
        source_ref,
        value,
    })
}

// r[impl molten.fabric_execution.port_contract]
// r[impl molten.fabric_execution.nonclaims]
pub fn canonical_admit_execution_profile(descriptor: &ExecutionProfileDescriptor) -> Result<CanonicalExecutionProfile> {
    let profile =
        admit_execution_profile(descriptor).map_err(|issues| validation_error("execution profile", &issues))?;
    let value = execution_profile_value(&profile);
    let profile_ref = canonical_hash(&value)?;
    Ok(CanonicalExecutionProfile {
        profile,
        profile_ref,
        value,
    })
}

// r[impl molten.fabric_execution.request]
// r[impl molten.fabric_execution.authority]
pub fn canonical_admit_execution_request(
    profile: &CanonicalExecutionProfile,
    request: &ExecutionRequest,
    authority: &ExecutionAuthorityFacts,
    resources: ExecutionResourceGrant,
    active_generation: u64,
) -> Result<CanonicalExecutionRequest> {
    let plan = admit_execution_request(&profile.profile, request, authority, resources, active_generation)
        .map_err(|issues| validation_error("execution request", &issues))?;
    let value = execution_request_value(&plan, &profile.profile_ref);
    let request_ref = canonical_hash(&value)?;
    Ok(CanonicalExecutionRequest {
        plan,
        request_ref,
        value,
    })
}

// r[impl molten.fabric_execution.output]
// r[impl molten.fabric_execution.lifecycle]
pub fn canonical_execution_receipt(
    request: &CanonicalExecutionRequest,
    profile: &CanonicalExecutionProfile,
    process: ExecutionProcessObservation,
    stdout_publication: ExecutionStreamPublication,
    stderr_publication: ExecutionStreamPublication,
) -> Result<CanonicalExecutionReceipt> {
    admit_execution_completion(
        &request.plan.request.identity(),
        &request.plan.request.identity(),
        process.lifecycle,
        request.plan.request.generation,
    )
    .map_err(|issues| validation_error("execution completion", &issues))?;
    let value = execution_receipt_value(request, profile, &process, &stdout_publication, &stderr_publication);
    let receipt_ref = canonical_hash(&value)?;
    Ok(CanonicalExecutionReceipt {
        receipt_ref,
        request_ref: request.request_ref.clone(),
        profile_ref: profile.profile_ref.clone(),
        operation_ref: request.plan.request.operation_ref.clone(),
        generation: request.plan.request.generation,
        process,
        stdout_publication,
        stderr_publication,
        non_claims: REQUIRED_EXECUTION_NON_CLAIMS.to_vec(),
        value,
    })
}

// r[impl molten.fabric_execution.port_contract]
pub fn fabric_execution_port_descriptor(profile: &CanonicalExecutionProfile) -> FabricPortDescriptor {
    let (determinism, replay) = match profile.profile.descriptor.kind {
        ExecutionProfileKind::LiveBoundedProcess => {
            (DeterminismClass::ExternalEffect, ReplayClass::RecordedEffectRequired)
        }
        ExecutionProfileKind::DeterministicSimulation => {
            (DeterminismClass::DeterministicWithRecordedInputs, ReplayClass::Recompute)
        }
    };
    FabricPortDescriptor {
        schema: FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
        port_id: EXECUTION_PORT_ID.to_string(),
        version: EXECUTION_PORT_VERSION.to_string(),
        class: FabricPortClass::Execution,
        operation_classes: vec![
            "cancel".to_string(),
            "execute".to_string(),
            "poll".to_string(),
            "reconcile".to_string(),
        ],
        input_schema_refs: vec![EXECUTION_INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![EXECUTION_OUTPUT_SCHEMA.to_string()],
        authority_requirements: vec![
            FabricAuthority::Execution,
            FabricAuthority::Resources,
            FabricAuthority::Evidence,
        ],
        resource_requirements: vec![
            FabricResource::Memory,
            FabricResource::StorageBytes,
            FabricResource::ExecutionMillis,
            FabricResource::InputBytes,
            FabricResource::OutputBytes,
            FabricResource::Concurrency,
            FabricResource::QueueDepth,
            FabricResource::LogicalTime,
            FabricResource::Diagnostics,
        ],
        determinism,
        replay,
        implementation_profile: profile.profile.descriptor.profile_id.clone(),
        conformance_refs: profile.profile.descriptor.conformance_refs.clone(),
        non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        enabled: true,
    }
}

fn validation_error(label: &str, issues: &impl std::fmt::Debug) -> MoltenError {
    MoltenError::invalid_harness(format!("{label} denied: {issues:?}"))
}
