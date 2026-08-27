use std::collections::BTreeSet;

use super::support::*;
use super::*;

pub(super) fn validate_request_shape(
    profile: &AdmittedExecutionProfile,
    request: &ExecutionRequest,
    issues: &mut Vec<ExecutionAdmissionIssue>,
) {
    if request.schema != EXECUTION_REQUEST_SCHEMA {
        issues.push(ExecutionAdmissionIssue::SchemaMismatch {
            field: "request-schema",
            actual: request.schema.clone(),
            expected: EXECUTION_REQUEST_SCHEMA,
        });
    }
    for (field, value) in [
        ("operation-ref", request.operation_ref.as_str()),
        ("idempotency-ref", request.idempotency_ref.as_str()),
        ("callback-ref", request.callback_ref.as_str()),
        ("effect-ref", request.effect_ref.as_str()),
        ("profile-ref", request.profile_ref.as_str()),
        ("executable-artifact-ref", request.executable_artifact_ref.as_str()),
        ("executable-identity-ref", request.executable_identity_ref.as_str()),
        ("workspace-ref", request.workspace_ref.as_str()),
        ("authority-ref", request.authority_ref.as_str()),
        ("resource-grant-ref", request.resource_grant_ref.as_str()),
    ] {
        validate_ref(field, value, issues);
    }
    if let Some(stdin_ref) = &request.stdin_ref {
        validate_ref("stdin-ref", stdin_ref, issues);
    }
    validate_token("extension-id", &request.extension_id, issues);
    validate_token("service-id", &request.service_id, issues);
    if request.profile_ref != profile.descriptor.profile_ref {
        issues.push(ExecutionAdmissionIssue::ProfileMismatch);
    }
    validate_arguments(profile, request, issues);
    validate_exit_policy(request, issues);
    if !profile.descriptor.supported_termination_scopes.contains(&request.termination_scope) {
        issues.push(ExecutionAdmissionIssue::UnsupportedTerminationScope(request.termination_scope));
    }
    if request.invocation_mode != ExecutionInvocationMode::Direct {
        issues.push(ExecutionAdmissionIssue::ShellExpansionDenied);
    }
    if request.executable_resolution != ExecutableResolutionMode::ExactArtifact {
        issues.push(ExecutionAdmissionIssue::PathSearchDenied);
    }
    if request.workspace_mode != WorkspaceMode::CapabilityRoot {
        issues.push(ExecutionAdmissionIssue::ImplicitCurrentDirectoryDenied);
    }
}

fn validate_arguments(
    profile: &AdmittedExecutionProfile,
    request: &ExecutionRequest,
    issues: &mut Vec<ExecutionAdmissionIssue>,
) {
    if request.arguments.len() > profile.descriptor.max_arguments {
        issues.push(ExecutionAdmissionIssue::CollectionBoundExceeded {
            field: "arguments",
            actual: request.arguments.len(),
            maximum: profile.descriptor.max_arguments,
        });
    }
    for argument in &request.arguments {
        if argument.as_bytes().contains(&0) {
            issues.push(ExecutionAdmissionIssue::EmbeddedNul("argument"));
        }
        if argument.len() > profile.descriptor.max_argument_bytes {
            issues.push(ExecutionAdmissionIssue::TextBoundExceeded {
                field: "argument",
                actual: argument.len(),
                maximum: profile.descriptor.max_argument_bytes,
            });
        }
    }
}

fn validate_exit_policy(request: &ExecutionRequest, issues: &mut Vec<ExecutionAdmissionIssue>) {
    if request.accepted_exit_codes.is_empty() {
        issues.push(ExecutionAdmissionIssue::AcceptedExitCodesEmpty);
    }
    let mut codes = BTreeSet::new();
    for code in &request.accepted_exit_codes {
        if !codes.insert(*code) {
            issues.push(ExecutionAdmissionIssue::DuplicateAcceptedExitCode(*code));
        }
    }
}

pub(super) fn validate_environment(
    profile: &AdmittedExecutionProfile,
    request: &ExecutionRequest,
    issues: &mut Vec<ExecutionAdmissionIssue>,
) {
    if request.environment_mode != ExecutionEnvironmentMode::Clear {
        issues.push(ExecutionAdmissionIssue::InheritedEnvironmentDenied);
    }
    if request.environment.len() > profile.descriptor.max_environment_entries {
        issues.push(ExecutionAdmissionIssue::CollectionBoundExceeded {
            field: "environment",
            actual: request.environment.len(),
            maximum: profile.descriptor.max_environment_entries,
        });
    }
    let mut names = BTreeSet::new();
    for entry in &request.environment {
        validate_environment_entry(profile, entry, issues);
        if !names.insert(entry.name.clone()) {
            issues.push(ExecutionAdmissionIssue::DuplicateEnvironmentName(entry.name.clone()));
        }
    }
}

fn validate_environment_entry(
    profile: &AdmittedExecutionProfile,
    entry: &EnvironmentEntry,
    issues: &mut Vec<ExecutionAdmissionIssue>,
) {
    validate_token("environment-name", &entry.name, issues);
    if entry.name.len() > profile.descriptor.max_environment_name_bytes {
        issues.push(ExecutionAdmissionIssue::TextBoundExceeded {
            field: "environment-name",
            actual: entry.name.len(),
            maximum: profile.descriptor.max_environment_name_bytes,
        });
    }
    if entry.value.len() > profile.descriptor.max_environment_value_bytes {
        issues.push(ExecutionAdmissionIssue::TextBoundExceeded {
            field: "environment-value",
            actual: entry.value.len(),
            maximum: profile.descriptor.max_environment_value_bytes,
        });
    }
    if entry.value.as_bytes().contains(&0) {
        issues.push(ExecutionAdmissionIssue::EmbeddedNul("environment-value"));
    }
    if entry.value_class == EnvironmentValueClass::Secret {
        issues.push(ExecutionAdmissionIssue::SecretEnvironmentDenied(entry.name.clone()));
    }
}

pub(super) fn validate_limits(
    profile: &AdmittedExecutionProfile,
    request: &ExecutionRequest,
    resources: ExecutionResourceGrant,
    issues: &mut Vec<ExecutionAdmissionIssue>,
) {
    let limits = request.limits;
    for (field, actual, maximum) in [
        ("timeout-ms", limits.timeout_ms, profile.descriptor.max_timeout_ms),
        ("stdin-max-bytes", limits.stdin_max_bytes, profile.descriptor.max_stdin_bytes),
        ("stdout-max-bytes", limits.stdout_max_bytes, profile.descriptor.max_stdout_bytes),
        ("stderr-max-bytes", limits.stderr_max_bytes, profile.descriptor.max_stderr_bytes),
        ("poll-interval-ms", limits.poll_interval_ms, profile.descriptor.max_poll_interval_ms),
        ("teardown-timeout-ms", limits.teardown_timeout_ms, profile.descriptor.max_teardown_timeout_ms),
        ("concurrency-units", limits.concurrency_units, profile.descriptor.max_concurrency_units),
        ("queue-units", limits.queue_units, profile.descriptor.max_queue_units),
    ] {
        validate_bound(field, actual, maximum, issues);
    }
    if limits.poll_interval_ms > limits.timeout_ms {
        issues.push(ExecutionAdmissionIssue::PollIntervalExceedsTimeout);
    }
    if limits.concurrency_units > resources.concurrency_units {
        issues.push(ExecutionAdmissionIssue::BoundExceeded {
            field: "resource-concurrency-units",
            actual: limits.concurrency_units,
            maximum: resources.concurrency_units,
        });
    }
    if limits.queue_units > resources.queue_units {
        issues.push(ExecutionAdmissionIssue::BoundExceeded {
            field: "resource-queue-units",
            actual: limits.queue_units,
            maximum: resources.queue_units,
        });
    }
    validate_capture_resources(limits, resources, issues);
}

fn validate_capture_resources(
    limits: ExecutionRequestLimits,
    resources: ExecutionResourceGrant,
    issues: &mut Vec<ExecutionAdmissionIssue>,
) {
    let capture_bytes = limits.stdout_max_bytes.checked_add(limits.stderr_max_bytes);
    let Some(capture_bytes) = capture_bytes else {
        issues.push(ExecutionAdmissionIssue::CaptureMemoryOverflow);
        return;
    };
    if capture_bytes > resources.memory_bytes {
        issues.push(ExecutionAdmissionIssue::CaptureMemoryGrantExceeded {
            required: capture_bytes,
            granted: resources.memory_bytes,
        });
    }
    if capture_bytes > resources.diagnostic_bytes {
        issues.push(ExecutionAdmissionIssue::DiagnosticGrantExceeded {
            required: capture_bytes,
            granted: resources.diagnostic_bytes,
        });
    }
    if resources.storage_bytes == 0 {
        issues.push(ExecutionAdmissionIssue::StorageGrantMissing);
    }
    if resources.logical_deadline_ticks == 0 {
        issues.push(ExecutionAdmissionIssue::ZeroBound("logical-deadline-ticks"));
    }
}

pub(super) fn validate_authority(
    profile: &AdmittedExecutionProfile,
    request: &ExecutionRequest,
    authority: &ExecutionAuthorityFacts,
    active_generation: u64,
    issues: &mut Vec<ExecutionAdmissionIssue>,
) {
    validate_authority_refs(authority, issues);
    for (field, is_matching) in [
        ("authority-ref", authority.authority_ref == request.authority_ref),
        ("resource-grant-ref", authority.resource_grant_ref == request.resource_grant_ref),
        ("executable-artifact-ref", authority.executable_artifact_ref == request.executable_artifact_ref),
        ("executable-identity-ref", authority.executable_identity_ref == request.executable_identity_ref),
        ("workspace-ref", authority.workspace_ref == request.workspace_ref),
        ("operation-ref", authority.operation_ref == request.operation_ref),
        ("extension-id", authority.extension_id == request.extension_id),
        ("service-id", authority.service_id == request.service_id),
        ("profile-ref", authority.profile_ref == profile.descriptor.profile_ref),
    ] {
        if !is_matching {
            issues.push(ExecutionAdmissionIssue::AuthorityMismatch(field));
        }
    }
    if request.generation != active_generation {
        issues.push(ExecutionAdmissionIssue::StaleGeneration {
            actual: request.generation,
            active: active_generation,
        });
    }
    if authority.generation != request.generation {
        issues.push(ExecutionAdmissionIssue::AuthorityMismatch("generation"));
    }
}

fn validate_authority_refs(authority: &ExecutionAuthorityFacts, issues: &mut Vec<ExecutionAdmissionIssue>) {
    for (field, value) in [
        ("authority-ref", authority.authority_ref.as_str()),
        ("executable-authority-ref", authority.executable_authority_ref.as_str()),
        ("provenance-ref", authority.provenance_ref.as_str()),
        ("effect-admission-ref", authority.effect_admission_ref.as_str()),
        ("workspace-authority-ref", authority.workspace_authority_ref.as_str()),
        ("process-authority-ref", authority.process_authority_ref.as_str()),
        ("resource-grant-ref", authority.resource_grant_ref.as_str()),
        ("policy-ref", authority.policy_ref.as_str()),
    ] {
        if value.is_empty() {
            issues.push(ExecutionAdmissionIssue::MissingAuthorityEvidence(field));
        } else {
            validate_ref(field, value, issues);
        }
    }
}
