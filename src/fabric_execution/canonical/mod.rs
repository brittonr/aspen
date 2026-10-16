mod records;

use self::records::*;
use super::*;

const EXECUTION_SOURCE_RECORD: &str = "fabric-execution-source-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub profile: AdmittedExecutionProfile,
    pub profile_ref: String,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub plan: AdmittedExecutionPlan,
    pub request_ref: String,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub receipt_ref: String,
    pub request_ref: String,
    pub profile_ref: String,
    pub operation_ref: String,
    pub generation: u64,
    pub process: ProcessObservation,
    pub stdout_publication: StreamPublication,
    pub stderr_publication: StreamPublication,
    pub non_claims: Vec<ExecutionNonClaim>,
    pub value: preserves::IOValue,
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
    pub value: preserves::IOValue,
}

// r[impl molten.fabric_execution.component_pin]
pub fn bounded_exec_source_cohort(platform: ExecutionPlatform) -> crate::error::Result<BoundedExecSourceCohort> {
    let value = crate::preserves_rail::record(EXECUTION_SOURCE_RECORD, vec![
        field("repository", crate::preserves_rail::string(BOUNDED_EXEC_REPOSITORY)),
        field("revision", crate::preserves_rail::string(BOUNDED_EXEC_REVISION)),
        field("license", crate::preserves_rail::string(BOUNDED_EXEC_LICENSE)),
        field("package", crate::preserves_rail::string(BOUNDED_EXEC_PACKAGE)),
        field("platform", crate::preserves_rail::string(platform.as_str())),
        field(
            "non-claims",
            crate::preserves_rail::sequence(
                REQUIRED_EXECUTION_NON_CLAIMS
                    .iter()
                    .map(|claim| crate::preserves_rail::string(claim.as_str()))
                    .collect(),
            ),
        ),
    ]);
    let source_ref = crate::preserves_rail::canonical_hash(&value)?;
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
pub fn admit_profile(descriptor: &ExecutionProfileDescriptor) -> crate::error::Result<Profile> {
    let profile =
        admit_execution_profile(descriptor).map_err(|issues| validation_error("execution profile", &issues))?;
    let value = profile_value(&profile);
    let profile_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(Profile {
        profile,
        profile_ref,
        value,
    })
}

// r[impl molten.fabric_execution.request]
// r[impl molten.fabric_execution.authority]
pub fn admit_request(
    profile: &Profile,
    request: &ExecutionRequest,
    authority: &ExecutionAuthorityFacts,
    resources: ExecutionResourceGrant,
    active_generation: u64,
) -> crate::error::Result<Request> {
    let plan = admit_execution_request(&profile.profile, request, authority, resources, active_generation)
        .map_err(|issues| validation_error("execution request", &issues))?;
    let value = request_value(&plan, &profile.profile_ref);
    let request_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(Request {
        plan,
        request_ref,
        value,
    })
}

// r[impl molten.fabric_execution.output]
// r[impl molten.fabric_execution.lifecycle]
pub fn receipt(
    request: &Request,
    profile: &Profile,
    process: ProcessObservation,
    stdout_publication: StreamPublication,
    stderr_publication: StreamPublication,
) -> crate::error::Result<Receipt> {
    admit_execution_completion(
        &request.plan.request.identity(),
        &request.plan.request.identity(),
        process.lifecycle,
        request.plan.request.generation,
    )
    .map_err(|issues| validation_error("execution completion", &issues))?;
    let value = receipt_value(request, profile, &process, &stdout_publication, &stderr_publication);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(Receipt {
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
pub fn fabric_execution_port_descriptor(profile: &Profile) -> crate::fabric::FabricPortDescriptor {
    let (determinism, replay) = match profile.profile.descriptor.kind {
        ExecutionProfileKind::LiveBoundedProcess => {
            (crate::fabric::DeterminismClass::ExternalEffect, crate::fabric::ReplayClass::RecordedEffectRequired)
        }
        ExecutionProfileKind::DeterministicSimulation => (
            crate::fabric::DeterminismClass::DeterministicWithRecordedInputs,
            crate::fabric::ReplayClass::Recompute,
        ),
    };
    crate::fabric::FabricPortDescriptor {
        schema: crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
        port_id: EXECUTION_PORT_ID.to_string(),
        version: EXECUTION_PORT_VERSION.to_string(),
        class: crate::fabric::FabricPortClass::Execution,
        operation_classes: vec![
            "cancel".to_string(),
            "execute".to_string(),
            "poll".to_string(),
            "reconcile".to_string(),
        ],
        input_schema_refs: vec![EXECUTION_INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![EXECUTION_OUTPUT_SCHEMA.to_string()],
        authority_requirements: vec![
            crate::fabric::FabricAuthority::Execution,
            crate::fabric::FabricAuthority::Resources,
            crate::fabric::FabricAuthority::Evidence,
        ],
        resource_requirements: vec![
            crate::fabric::FabricResource::Memory,
            crate::fabric::FabricResource::StorageBytes,
            crate::fabric::FabricResource::ExecutionMillis,
            crate::fabric::FabricResource::InputBytes,
            crate::fabric::FabricResource::OutputBytes,
            crate::fabric::FabricResource::Concurrency,
            crate::fabric::FabricResource::QueueDepth,
            crate::fabric::FabricResource::LogicalTime,
            crate::fabric::FabricResource::Diagnostics,
        ],
        determinism,
        replay,
        implementation_profile: profile.profile.descriptor.profile_id.clone(),
        conformance_refs: profile.profile.descriptor.conformance_refs.clone(),
        non_claims: crate::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        enabled: true,
    }
}

fn validation_error(label: &str, issues: &impl std::fmt::Debug) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(format!("{label} denied: {issues:?}"))
}
