use preserves::IOValue;

use super::*;
use crate::preserves_rail::bool_value;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;

const EXECUTION_PROFILE_RECORD: &str = "fabric-execution-profile-v1";
const EXECUTION_REQUEST_RECORD: &str = "fabric-execution-request-v1";
const EXECUTION_RECEIPT_RECORD: &str = "fabric-execution-receipt-v1";
const EXECUTION_STREAM_RECORD: &str = "fabric-execution-stream-v1";
const NO_VALUE: &str = "none";

pub(super) fn execution_profile_value(profile: &AdmittedExecutionProfile) -> IOValue {
    let descriptor = &profile.descriptor;
    record(EXECUTION_PROFILE_RECORD, vec![
        field("schema", string(&descriptor.schema)),
        field("profile-id", string(&descriptor.profile_id)),
        field("profile-contract-ref", string(&descriptor.profile_ref)),
        field("kind", string(descriptor.kind.as_str())),
        field("platform", string(descriptor.platform.as_str())),
        field(
            "termination-scopes",
            sequence(descriptor.supported_termination_scopes.iter().map(|scope| string(scope.as_str())).collect()),
        ),
        field("max-timeout-ms", u64_value(descriptor.max_timeout_ms)),
        field("max-stdin-bytes", u64_value(descriptor.max_stdin_bytes)),
        field("max-stdout-bytes", u64_value(descriptor.max_stdout_bytes)),
        field("max-stderr-bytes", u64_value(descriptor.max_stderr_bytes)),
        field("max-poll-interval-ms", u64_value(descriptor.max_poll_interval_ms)),
        field("max-teardown-timeout-ms", u64_value(descriptor.max_teardown_timeout_ms)),
        field("component-repository", string(&descriptor.component_repository)),
        field("component-revision", string(&descriptor.component_revision)),
        field("component-license", string(&descriptor.component_license)),
        field("component-package", string(&descriptor.component_package)),
        field("conformance-refs", sequence(descriptor.conformance_refs.iter().map(string).collect())),
        field("non-claims", sequence(descriptor.non_claims.iter().map(|claim| string(claim.as_str())).collect())),
    ])
}

pub(super) fn execution_request_value(plan: &AdmittedExecutionPlan, profile_admission_ref: &str) -> IOValue {
    let request = &plan.request;
    record(EXECUTION_REQUEST_RECORD, vec![
        field("schema", string(&request.schema)),
        field("profile-admission-ref", string(profile_admission_ref)),
        field("profile-contract-ref", string(&request.profile_ref)),
        field("operation-ref", string(&request.operation_ref)),
        field("idempotency-ref", string(&request.idempotency_ref)),
        field("extension-id", string(&request.extension_id)),
        field("service-id", string(&request.service_id)),
        field("callback-ref", string(&request.callback_ref)),
        field("effect-ref", string(&request.effect_ref)),
        field("generation", u64_value(request.generation)),
        field("executable-artifact-ref", string(&request.executable_artifact_ref)),
        field("executable-identity-ref", string(&request.executable_identity_ref)),
        field("arguments", sequence(request.arguments.iter().map(string).collect())),
        field(
            "environment",
            sequence(
                request
                    .environment
                    .iter()
                    .map(|entry| {
                        record("environment-entry-v1", vec![
                            field("name", string(&entry.name)),
                            field("value", string(&entry.value)),
                            field("class", string(entry.value_class.as_str())),
                        ])
                    })
                    .collect(),
            ),
        ),
        field("environment-mode", string(request.environment_mode.as_str())),
        field("invocation-mode", string(request.invocation_mode.as_str())),
        field("executable-resolution", string(request.executable_resolution.as_str())),
        field("workspace-ref", string(&request.workspace_ref)),
        field("workspace-mode", string(request.workspace_mode.as_str())),
        field("stdin-ref", string(request.stdin_ref.as_deref().unwrap_or(NO_VALUE))),
        field("timeout-ms", u64_value(request.limits.timeout_ms)),
        field("stdin-max-bytes", u64_value(request.limits.stdin_max_bytes)),
        field("stdout-max-bytes", u64_value(request.limits.stdout_max_bytes)),
        field("stderr-max-bytes", u64_value(request.limits.stderr_max_bytes)),
        field("poll-interval-ms", u64_value(request.limits.poll_interval_ms)),
        field("teardown-timeout-ms", u64_value(request.limits.teardown_timeout_ms)),
        field("termination-scope", string(request.termination_scope.as_str())),
        field(
            "accepted-exit-codes",
            sequence(request.accepted_exit_codes.iter().map(|code| string(code.to_string())).collect()),
        ),
        field("reject-stdout-truncation", bool_value(request.reject_stdout_truncation)),
        field("reject-stderr-truncation", bool_value(request.reject_stderr_truncation)),
        field("authority-ref", string(&request.authority_ref)),
        field("resource-grant-ref", string(&request.resource_grant_ref)),
        field("executable-authority-ref", string(&plan.authority.executable_authority_ref)),
        field("provenance-ref", string(&plan.authority.provenance_ref)),
        field("effect-admission-ref", string(&plan.authority.effect_admission_ref)),
        field("workspace-authority-ref", string(&plan.authority.workspace_authority_ref)),
        field("process-authority-ref", string(&plan.authority.process_authority_ref)),
        field("policy-ref", string(&plan.authority.policy_ref)),
    ])
}

pub(super) fn execution_receipt_value(
    request: &CanonicalExecutionRequest,
    profile: &CanonicalExecutionProfile,
    process: &ExecutionProcessObservation,
    stdout_publication: &ExecutionStreamPublication,
    stderr_publication: &ExecutionStreamPublication,
) -> IOValue {
    record(EXECUTION_RECEIPT_RECORD, vec![
        field("schema", string(EXECUTION_RECEIPT_SCHEMA)),
        field("request-ref", string(&request.request_ref)),
        field("profile-ref", string(&profile.profile_ref)),
        field("operation-ref", string(&request.plan.request.operation_ref)),
        field("generation", u64_value(request.plan.request.generation)),
        field("lifecycle", string(process.lifecycle.as_str())),
        field("start-observed", bool_value(process.start_observed)),
        field("terminal-observed", bool_value(process.terminal_observed)),
        field("teardown-observed", bool_value(process.teardown_observed)),
        field("exit-code", string(process.exit_code.map_or_else(|| NO_VALUE.to_string(), |code| code.to_string()))),
        field("signal", string(process.signal.map_or_else(|| NO_VALUE.to_string(), |signal| signal.to_string()))),
        field("disposition", string(process.disposition.as_str())),
        field("stdout", execution_stream_value(&process.stdout, stdout_publication)),
        field("stderr", execution_stream_value(&process.stderr, stderr_publication)),
        field(
            "non-claims",
            sequence(REQUIRED_EXECUTION_NON_CLAIMS.iter().map(|claim| string(claim.as_str())).collect()),
        ),
    ])
}

fn execution_stream_value(stream: &RetainedExecutionStream, publication: &ExecutionStreamPublication) -> IOValue {
    let (publication_state, content_ref, publication_receipt_ref, diagnostic_code) = match publication {
        ExecutionStreamPublication::Published(published) => {
            ("published", published.content_ref.as_str(), published.publication_receipt_ref.as_str(), NO_VALUE)
        }
        ExecutionStreamPublication::Failed { diagnostic_code } => ("failed", NO_VALUE, NO_VALUE, *diagnostic_code),
    };
    record(EXECUTION_STREAM_RECORD, vec![
        field("role", string(&stream.role)),
        field("observed-bytes", u64_value(stream.observed_bytes)),
        field("retained-bytes", u64_value(stream.retained_byte_count)),
        field("truncated", bool_value(stream.truncated)),
        field("publication", string(publication_state)),
        field("content-ref", string(content_ref)),
        field("publication-receipt-ref", string(publication_receipt_ref)),
        field("diagnostic-code", string(diagnostic_code)),
    ])
}

pub(super) fn field(name: &'static str, value: IOValue) -> IOValue {
    record("field", vec![string(name), value])
}
