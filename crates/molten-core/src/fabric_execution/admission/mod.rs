mod profile;
mod request;
mod support;

use self::profile::*;
use self::request::*;
use super::*;

const STDOUT_ROLE: &str = "stdout-retained-prefix";
const STDERR_ROLE: &str = "stderr-retained-prefix";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionAdmissionIssue {
    SchemaMismatch {
        field: &'static str,
        actual: String,
        expected: &'static str,
    },
    EmptyField(&'static str),
    MalformedToken {
        field: &'static str,
        value: String,
    },
    MalformedRef {
        field: &'static str,
        value: String,
    },
    DuplicateValue(&'static str),
    MissingNonClaim(ExecutionNonClaim),
    MissingFabricNonClaim(crate::fabric::FabricNonClaim),
    ComponentSourceMismatch,
    UnsupportedTerminationScope(ExecutionTerminationScope),
    PlatformTerminationMismatch,
    ZeroBound(&'static str),
    BoundExceeded {
        field: &'static str,
        actual: u64,
        maximum: u64,
    },
    CollectionBoundExceeded {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    TextBoundExceeded {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    EmbeddedNul(&'static str),
    InheritedEnvironmentDenied,
    ShellExpansionDenied,
    PathSearchDenied,
    ImplicitCurrentDirectoryDenied,
    SecretEnvironmentDenied(String),
    ProfileMismatch,
    StaleGeneration {
        actual: u64,
        active: u64,
    },
    AuthorityMismatch(&'static str),
    MissingAuthorityEvidence(&'static str),
    DuplicateEnvironmentName(String),
    AcceptedExitCodesEmpty,
    DuplicateAcceptedExitCode(i32),
    PollIntervalExceedsTimeout,
    CaptureMemoryOverflow,
    CaptureMemoryGrantExceeded {
        required: u64,
        granted: u64,
    },
    DiagnosticGrantExceeded {
        required: u64,
        granted: u64,
    },
    StorageGrantMissing,
}

// r[impl molten.fabric_execution.component_pin]
// r[impl molten.fabric_execution.nonclaims]
pub fn admit_execution_profile(
    descriptor: &ExecutionProfileDescriptor,
) -> Result<AdmittedExecutionProfile, Vec<ExecutionAdmissionIssue>> {
    let mut issues = Vec::new();
    validate_profile_shape(descriptor, &mut issues);
    validate_component_pin(descriptor, &mut issues);
    validate_profile_bounds(descriptor, &mut issues);
    validate_profile_non_claims(descriptor, &mut issues);
    if issues.is_empty() {
        Ok(AdmittedExecutionProfile {
            descriptor: descriptor.clone(),
        })
    } else {
        Err(issues)
    }
}

// r[impl molten.fabric_execution.authority]
// r[impl molten.fabric_execution.request]
// r[impl molten.fabric_execution.environment]
// r[impl molten.fabric_execution.generation]
pub fn admit_execution_request(
    profile: &AdmittedExecutionProfile,
    request: &ExecutionRequest,
    authority: &ExecutionAuthorityFacts,
    resources: ExecutionResourceGrant,
    active_generation: u64,
) -> Result<AdmittedExecutionPlan, Vec<ExecutionAdmissionIssue>> {
    let mut issues = Vec::new();
    validate_request_shape(profile, request, &mut issues);
    validate_environment(profile, request, &mut issues);
    validate_limits(profile, request, resources, &mut issues);
    validate_authority(profile, request, authority, active_generation, &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(AdmittedExecutionPlan {
        profile: profile.clone(),
        request: request.clone(),
        authority: authority.clone(),
        resources,
        resolution: CapabilityResolutionPlan {
            executable_artifact_ref: request.executable_artifact_ref.clone(),
            executable_identity_ref: request.executable_identity_ref.clone(),
            workspace_ref: request.workspace_ref.clone(),
            stdin_ref: request.stdin_ref.clone(),
            stdout_role: STDOUT_ROLE.to_string(),
            stderr_role: STDERR_ROLE.to_string(),
        },
    })
}
