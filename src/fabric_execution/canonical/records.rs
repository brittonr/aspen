use super::*;

const EXECUTION_PROFILE_RECORD: &str = "fabric-execution-profile-v1";
const EXECUTION_REQUEST_RECORD: &str = "fabric-execution-request-v1";
const EXECUTION_RECEIPT_RECORD: &str = "fabric-execution-receipt-v1";
const EXECUTION_STREAM_RECORD: &str = "fabric-execution-stream-v1";
const NO_VALUE: &str = "none";

pub(super) fn execution_profile_value(profile: &AdmittedExecutionProfile) -> preserves::IOValue {
    let descriptor = &profile.descriptor;
    crate::preserves_rail::record(EXECUTION_PROFILE_RECORD, vec![
        field("schema", crate::preserves_rail::string(&descriptor.schema)),
        field("profile-id", crate::preserves_rail::string(&descriptor.profile_id)),
        field("profile-contract-ref", crate::preserves_rail::string(&descriptor.profile_ref)),
        field("kind", crate::preserves_rail::string(descriptor.kind.as_str())),
        field("platform", crate::preserves_rail::string(descriptor.platform.as_str())),
        field(
            "termination-scopes",
            crate::preserves_rail::sequence(
                descriptor
                    .supported_termination_scopes
                    .iter()
                    .map(|scope| crate::preserves_rail::string(scope.as_str()))
                    .collect(),
            ),
        ),
        field("max-timeout-ms", crate::preserves_rail::u64_value(descriptor.max_timeout_ms)),
        field("max-stdin-bytes", crate::preserves_rail::u64_value(descriptor.max_stdin_bytes)),
        field("max-stdout-bytes", crate::preserves_rail::u64_value(descriptor.max_stdout_bytes)),
        field("max-stderr-bytes", crate::preserves_rail::u64_value(descriptor.max_stderr_bytes)),
        field("max-poll-interval-ms", crate::preserves_rail::u64_value(descriptor.max_poll_interval_ms)),
        field("max-teardown-timeout-ms", crate::preserves_rail::u64_value(descriptor.max_teardown_timeout_ms)),
        field("component-repository", crate::preserves_rail::string(&descriptor.component_repository)),
        field("component-revision", crate::preserves_rail::string(&descriptor.component_revision)),
        field("component-license", crate::preserves_rail::string(&descriptor.component_license)),
        field("component-package", crate::preserves_rail::string(&descriptor.component_package)),
        field(
            "conformance-refs",
            crate::preserves_rail::sequence(
                descriptor.conformance_refs.iter().map(crate::preserves_rail::string).collect(),
            ),
        ),
        field(
            "non-claims",
            crate::preserves_rail::sequence(
                descriptor.non_claims.iter().map(|claim| crate::preserves_rail::string(claim.as_str())).collect(),
            ),
        ),
    ])
}

pub(super) fn execution_request_value(plan: &AdmittedExecutionPlan, profile_admission_ref: &str) -> preserves::IOValue {
    let request = &plan.request;
    crate::preserves_rail::record(EXECUTION_REQUEST_RECORD, vec![
        field("schema", crate::preserves_rail::string(&request.schema)),
        field("profile-admission-ref", crate::preserves_rail::string(profile_admission_ref)),
        field("profile-contract-ref", crate::preserves_rail::string(&request.profile_ref)),
        field("operation-ref", crate::preserves_rail::string(&request.operation_ref)),
        field("idempotency-ref", crate::preserves_rail::string(&request.idempotency_ref)),
        field("extension-id", crate::preserves_rail::string(&request.extension_id)),
        field("service-id", crate::preserves_rail::string(&request.service_id)),
        field("callback-ref", crate::preserves_rail::string(&request.callback_ref)),
        field("effect-ref", crate::preserves_rail::string(&request.effect_ref)),
        field("generation", crate::preserves_rail::u64_value(request.generation)),
        field("executable-artifact-ref", crate::preserves_rail::string(&request.executable_artifact_ref)),
        field("executable-identity-ref", crate::preserves_rail::string(&request.executable_identity_ref)),
        field(
            "arguments",
            crate::preserves_rail::sequence(request.arguments.iter().map(crate::preserves_rail::string).collect()),
        ),
        field(
            "environment",
            crate::preserves_rail::sequence(
                request
                    .environment
                    .iter()
                    .map(|entry| {
                        crate::preserves_rail::record("environment-entry-v1", vec![
                            field("name", crate::preserves_rail::string(&entry.name)),
                            field("value", crate::preserves_rail::string(&entry.value)),
                            field("class", crate::preserves_rail::string(entry.value_class.as_str())),
                        ])
                    })
                    .collect(),
            ),
        ),
        field("environment-mode", crate::preserves_rail::string(request.environment_mode.as_str())),
        field("invocation-mode", crate::preserves_rail::string(request.invocation_mode.as_str())),
        field("executable-resolution", crate::preserves_rail::string(request.executable_resolution.as_str())),
        field("workspace-ref", crate::preserves_rail::string(&request.workspace_ref)),
        field("workspace-mode", crate::preserves_rail::string(request.workspace_mode.as_str())),
        field("stdin-ref", crate::preserves_rail::string(request.stdin_ref.as_deref().unwrap_or(NO_VALUE))),
        field("timeout-ms", crate::preserves_rail::u64_value(request.limits.timeout_ms)),
        field("stdin-max-bytes", crate::preserves_rail::u64_value(request.limits.stdin_max_bytes)),
        field("stdout-max-bytes", crate::preserves_rail::u64_value(request.limits.stdout_max_bytes)),
        field("stderr-max-bytes", crate::preserves_rail::u64_value(request.limits.stderr_max_bytes)),
        field("poll-interval-ms", crate::preserves_rail::u64_value(request.limits.poll_interval_ms)),
        field("teardown-timeout-ms", crate::preserves_rail::u64_value(request.limits.teardown_timeout_ms)),
        field("termination-scope", crate::preserves_rail::string(request.termination_scope.as_str())),
        field(
            "accepted-exit-codes",
            crate::preserves_rail::sequence(
                request
                    .accepted_exit_codes
                    .iter()
                    .map(|code| crate::preserves_rail::string(code.to_string()))
                    .collect(),
            ),
        ),
        field("reject-stdout-truncation", crate::preserves_rail::bool_value(request.reject_stdout_truncation)),
        field("reject-stderr-truncation", crate::preserves_rail::bool_value(request.reject_stderr_truncation)),
        field("authority-ref", crate::preserves_rail::string(&request.authority_ref)),
        field("resource-grant-ref", crate::preserves_rail::string(&request.resource_grant_ref)),
        field("executable-authority-ref", crate::preserves_rail::string(&plan.authority.executable_authority_ref)),
        field("provenance-ref", crate::preserves_rail::string(&plan.authority.provenance_ref)),
        field("effect-admission-ref", crate::preserves_rail::string(&plan.authority.effect_admission_ref)),
        field("workspace-authority-ref", crate::preserves_rail::string(&plan.authority.workspace_authority_ref)),
        field("process-authority-ref", crate::preserves_rail::string(&plan.authority.process_authority_ref)),
        field("policy-ref", crate::preserves_rail::string(&plan.authority.policy_ref)),
    ])
}

pub(super) fn execution_receipt_value(
    request: &CanonicalExecutionRequest,
    profile: &CanonicalExecutionProfile,
    process: &ExecutionProcessObservation,
    stdout_publication: &ExecutionStreamPublication,
    stderr_publication: &ExecutionStreamPublication,
) -> preserves::IOValue {
    crate::preserves_rail::record(EXECUTION_RECEIPT_RECORD, vec![
        field("schema", crate::preserves_rail::string(EXECUTION_RECEIPT_SCHEMA)),
        field("request-ref", crate::preserves_rail::string(&request.request_ref)),
        field("profile-ref", crate::preserves_rail::string(&profile.profile_ref)),
        field("operation-ref", crate::preserves_rail::string(&request.plan.request.operation_ref)),
        field("generation", crate::preserves_rail::u64_value(request.plan.request.generation)),
        field("lifecycle", crate::preserves_rail::string(process.lifecycle.as_str())),
        field("start-observed", crate::preserves_rail::bool_value(process.start_observed)),
        field("terminal-observed", crate::preserves_rail::bool_value(process.terminal_observed)),
        field("teardown-observed", crate::preserves_rail::bool_value(process.teardown_observed)),
        field(
            "exit-code",
            crate::preserves_rail::string(
                process.exit_code.map_or_else(|| NO_VALUE.to_string(), |code| code.to_string()),
            ),
        ),
        field(
            "signal",
            crate::preserves_rail::string(
                process.signal.map_or_else(|| NO_VALUE.to_string(), |signal| signal.to_string()),
            ),
        ),
        field("disposition", crate::preserves_rail::string(process.disposition.as_str())),
        field("stdout", execution_stream_value(&process.stdout, stdout_publication)),
        field("stderr", execution_stream_value(&process.stderr, stderr_publication)),
        field(
            "non-claims",
            crate::preserves_rail::sequence(
                REQUIRED_EXECUTION_NON_CLAIMS
                    .iter()
                    .map(|claim| crate::preserves_rail::string(claim.as_str()))
                    .collect(),
            ),
        ),
    ])
}

fn execution_stream_value(
    stream: &RetainedExecutionStream,
    publication: &ExecutionStreamPublication,
) -> preserves::IOValue {
    let (publication_state, content_ref, publication_receipt_ref, diagnostic_code) = match publication {
        ExecutionStreamPublication::Published(published) => {
            ("published", published.content_ref.as_str(), published.publication_receipt_ref.as_str(), NO_VALUE)
        }
        ExecutionStreamPublication::Failed { diagnostic_code } => ("failed", NO_VALUE, NO_VALUE, *diagnostic_code),
    };
    crate::preserves_rail::record(EXECUTION_STREAM_RECORD, vec![
        field("role", crate::preserves_rail::string(&stream.role)),
        field("observed-bytes", crate::preserves_rail::u64_value(stream.observed_bytes)),
        field("retained-bytes", crate::preserves_rail::u64_value(stream.retained_byte_count)),
        field("truncated", crate::preserves_rail::bool_value(stream.truncated)),
        field("publication", crate::preserves_rail::string(publication_state)),
        field("content-ref", crate::preserves_rail::string(content_ref)),
        field("publication-receipt-ref", crate::preserves_rail::string(publication_receipt_ref)),
        field("diagnostic-code", crate::preserves_rail::string(diagnostic_code)),
    ])
}

pub(super) fn field(name: &'static str, value: preserves::IOValue) -> preserves::IOValue {
    crate::preserves_rail::record("field", vec![crate::preserves_rail::string(name), value])
}
