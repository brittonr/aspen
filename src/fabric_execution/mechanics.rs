use std::ffi::OsString;

use bounded_exec::CommandSpec;
use bounded_exec::Completion;
use bounded_exec::Disposition;
use bounded_exec::EnvironmentMode;
use bounded_exec::ExecutionLimits;
use bounded_exec::Input;
use bounded_exec::OutcomePolicy;
use bounded_exec::RunRequest;
use bounded_exec::TerminationScope;

use super::*;

const STDOUT_ROLE: &str = "stdout-retained-prefix";
const STDERR_ROLE: &str = "stderr-retained-prefix";
pub(super) const PUBLICATION_FAILURE_CODE: &str = "execution-output-publication-failed";

pub(super) fn validate_resolved_context(
    request: &CanonicalExecutionRequest,
    resolved: &ResolvedExecutionContext,
) -> Result<(), String> {
    let plan = &request.plan.resolution;
    if resolved.executable_artifact_ref != plan.executable_artifact_ref {
        return Err("resolved executable artifact identity differs from the admitted plan".to_string());
    }
    if resolved.executable_identity_ref != plan.executable_identity_ref {
        return Err("resolved executable measurement differs from the admitted plan".to_string());
    }
    if resolved.workspace_ref != plan.workspace_ref {
        return Err("resolved workspace capability differs from the admitted plan".to_string());
    }
    if resolved.stdin_ref != plan.stdin_ref {
        return Err("resolved stdin content differs from the admitted plan".to_string());
    }
    if !resolved.executable_path.is_absolute() {
        return Err("resolved executable path is not absolute".to_string());
    }
    if !resolved.workspace_path.is_absolute() {
        return Err("resolved workspace path is not absolute".to_string());
    }
    match (&resolved.stdin_ref, &resolved.stdin_bytes) {
        (None, None) | (Some(_), Some(_)) => {}
        (None, Some(_)) | (Some(_), None) => {
            return Err("resolved stdin reference and bytes are inconsistent".to_string());
        }
    }
    if resolved
        .stdin_bytes
        .as_ref()
        .is_some_and(|bytes| u64::try_from(bytes.len()).ok() > Some(request.plan.request.limits.stdin_max_bytes))
    {
        return Err("resolved stdin exceeds the admitted byte bound".to_string());
    }
    Ok(())
}

pub(super) fn bounded_request(
    request: &CanonicalExecutionRequest,
    resolved: &ResolvedExecutionContext,
) -> Result<RunRequest, String> {
    let limits = request.plan.request.limits;
    let stdin_max_bytes = usize::try_from(limits.stdin_max_bytes)
        .map_err(|_| "stdin byte bound does not fit the host platform".to_string())?;
    let stdout_max_bytes = usize::try_from(limits.stdout_max_bytes)
        .map_err(|_| "stdout byte bound does not fit the host platform".to_string())?;
    let stderr_max_bytes = usize::try_from(limits.stderr_max_bytes)
        .map_err(|_| "stderr byte bound does not fit the host platform".to_string())?;
    let outcome_policy = OutcomePolicy::new(
        request.plan.request.accepted_exit_codes.clone(),
        request.plan.request.reject_stdout_truncation,
        request.plan.request.reject_stderr_truncation,
    )
    .map_err(|error| format!("execution outcome policy is invalid: {error:?}"))?;
    let input = resolved.stdin_bytes.clone().map_or(Input::Null, Input::Bytes);
    Ok(RunRequest {
        command: CommandSpec {
            program: resolved.executable_path.clone(),
            args: request.plan.request.arguments.iter().map(OsString::from).collect(),
            current_dir: resolved.workspace_path.clone(),
            environment_mode: EnvironmentMode::Clear,
            environment: request
                .plan
                .request
                .environment
                .iter()
                .map(|entry| (OsString::from(&entry.name), OsString::from(&entry.value)))
                .collect(),
            input,
        },
        limits: ExecutionLimits {
            timeout_ms: limits.timeout_ms,
            stdin_max_bytes,
            stdout_max_bytes,
            stderr_max_bytes,
            poll_interval_ms: limits.poll_interval_ms,
            teardown_timeout_ms: limits.teardown_timeout_ms,
        },
        termination_scope: match request.plan.request.termination_scope {
            ExecutionTerminationScope::DirectChild => TerminationScope::Child,
            ExecutionTerminationScope::ProcessGroup => TerminationScope::ProcessGroup,
        },
        outcome_policy,
    })
}

pub(super) fn process_observation(
    output: bounded_exec::ExecutionOutput,
) -> Result<ExecutionProcessObservation, String> {
    let lifecycle = match output.completion {
        Completion::Exited => ExecutionLifecycleState::Exited,
        Completion::TimedOut => ExecutionLifecycleState::TimedOut,
        Completion::Cancelled => ExecutionLifecycleState::Cancelled,
    };
    let disposition = match output.disposition {
        Disposition::Succeeded => ExecutionObservedDisposition::ExitPolicyAccepted,
        Disposition::ExitFailed => ExecutionObservedDisposition::ExitPolicyRejected,
        Disposition::TimedOut => ExecutionObservedDisposition::TimedOut,
        Disposition::Cancelled => ExecutionObservedDisposition::Cancelled,
        Disposition::OutputLimitExceeded(_) => ExecutionObservedDisposition::OutputPolicyRejected,
    };
    Ok(ExecutionProcessObservation {
        lifecycle,
        start_observed: true,
        terminal_observed: true,
        teardown_observed: true,
        exit_code: output.exit_code,
        signal: output.signal,
        disposition,
        stdout: retained_stream(STDOUT_ROLE, output.stdout)?,
        stderr: retained_stream(STDERR_ROLE, output.stderr)?,
    })
}

fn retained_stream(role: &str, output: bounded_exec::CapturedOutput) -> Result<RetainedExecutionStream, String> {
    let observed_bytes = u64::try_from(output.observed_bytes)
        .map_err(|_| "observed execution output byte count does not fit u64".to_string())?;
    let retained_byte_count = u64::try_from(output.bytes.len())
        .map_err(|_| "retained execution output byte count does not fit u64".to_string())?;
    Ok(RetainedExecutionStream {
        role: role.to_string(),
        retained_bytes: output.bytes,
        observed_bytes,
        retained_byte_count,
        truncated: output.truncated,
    })
}

pub(super) fn publish_stream<P: ExecutionOutputPublisher>(
    publisher: &mut P,
    operation_ref: &str,
    stream: &RetainedExecutionStream,
) -> ExecutionStreamPublication {
    match publisher.publish(operation_ref, stream) {
        Ok(published) => ExecutionStreamPublication::Published(published),
        Err(_) => ExecutionStreamPublication::Failed {
            diagnostic_code: PUBLICATION_FAILURE_CODE,
        },
    }
}
