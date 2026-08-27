use super::support::*;
use super::*;

pub(super) fn validate_profile_shape(
    descriptor: &ExecutionProfileDescriptor,
    issues: &mut Vec<ExecutionAdmissionIssue>,
) {
    if descriptor.schema != EXECUTION_PROFILE_SCHEMA {
        issues.push(ExecutionAdmissionIssue::SchemaMismatch {
            field: "profile-schema",
            actual: descriptor.schema.clone(),
            expected: EXECUTION_PROFILE_SCHEMA,
        });
    }
    validate_token("profile-id", &descriptor.profile_id, issues);
    validate_ref("profile-ref", &descriptor.profile_ref, issues);
    validate_refs("conformance-ref", &descriptor.conformance_refs, issues);
    if descriptor.conformance_refs.is_empty() {
        issues.push(ExecutionAdmissionIssue::EmptyField("conformance-refs"));
    }
    if has_duplicates(&descriptor.conformance_refs) {
        issues.push(ExecutionAdmissionIssue::DuplicateValue("conformance-refs"));
    }
    if descriptor.supported_termination_scopes.is_empty() {
        issues.push(ExecutionAdmissionIssue::EmptyField("supported-termination-scopes"));
    }
    if has_duplicates(&descriptor.supported_termination_scopes) {
        issues.push(ExecutionAdmissionIssue::DuplicateValue("supported-termination-scopes"));
    }
    if descriptor.platform == ExecutionPlatform::DirectChildOnly
        && descriptor.supported_termination_scopes.contains(&ExecutionTerminationScope::ProcessGroup)
    {
        issues.push(ExecutionAdmissionIssue::PlatformTerminationMismatch);
    }
}

pub(super) fn validate_component_pin(
    descriptor: &ExecutionProfileDescriptor,
    issues: &mut Vec<ExecutionAdmissionIssue>,
) {
    let is_exact_pin = descriptor.component_repository == BOUNDED_EXEC_REPOSITORY
        && descriptor.component_revision == BOUNDED_EXEC_REVISION
        && descriptor.component_license == BOUNDED_EXEC_LICENSE
        && descriptor.component_package == BOUNDED_EXEC_PACKAGE;
    if !is_exact_pin {
        issues.push(ExecutionAdmissionIssue::ComponentSourceMismatch);
    }
}

pub(super) fn validate_profile_bounds(
    descriptor: &ExecutionProfileDescriptor,
    issues: &mut Vec<ExecutionAdmissionIssue>,
) {
    for (field, bound) in [
        ("max-timeout-ms", descriptor.max_timeout_ms),
        ("max-stdin-bytes", descriptor.max_stdin_bytes),
        ("max-stdout-bytes", descriptor.max_stdout_bytes),
        ("max-stderr-bytes", descriptor.max_stderr_bytes),
        ("max-poll-interval-ms", descriptor.max_poll_interval_ms),
        ("max-teardown-timeout-ms", descriptor.max_teardown_timeout_ms),
        ("max-concurrency-units", descriptor.max_concurrency_units),
        ("max-queue-units", descriptor.max_queue_units),
    ] {
        if bound == 0 {
            issues.push(ExecutionAdmissionIssue::ZeroBound(field));
        }
    }
    for (field, bound) in [
        ("max-arguments", descriptor.max_arguments),
        ("max-argument-bytes", descriptor.max_argument_bytes),
        ("max-environment-entries", descriptor.max_environment_entries),
        ("max-environment-name-bytes", descriptor.max_environment_name_bytes),
        ("max-environment-value-bytes", descriptor.max_environment_value_bytes),
    ] {
        if bound == 0 {
            issues.push(ExecutionAdmissionIssue::ZeroBound(field));
        }
    }
    if descriptor.max_poll_interval_ms > descriptor.max_timeout_ms {
        issues.push(ExecutionAdmissionIssue::PollIntervalExceedsTimeout);
    }
}

pub(super) fn validate_profile_non_claims(
    descriptor: &ExecutionProfileDescriptor,
    issues: &mut Vec<ExecutionAdmissionIssue>,
) {
    if has_duplicates(&descriptor.non_claims) {
        issues.push(ExecutionAdmissionIssue::DuplicateValue("execution-non-claims"));
    }
    for required in REQUIRED_EXECUTION_NON_CLAIMS {
        if !descriptor.non_claims.contains(&required) {
            issues.push(ExecutionAdmissionIssue::MissingNonClaim(required));
        }
    }
    if has_duplicates(&descriptor.fabric_non_claims) {
        issues.push(ExecutionAdmissionIssue::DuplicateValue("fabric-non-claims"));
    }
    for required in crate::fabric::REQUIRED_FABRIC_NON_CLAIMS {
        if !descriptor.fabric_non_claims.contains(&required) {
            issues.push(ExecutionAdmissionIssue::MissingFabricNonClaim(required));
        }
    }
}
