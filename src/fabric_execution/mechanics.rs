use super::*;

const STDOUT_ROLE: &str = "stdout-retained-prefix";
const STDERR_ROLE: &str = "stderr-retained-prefix";
pub(super) const PUBLICATION_FAILURE_CODE: &str = "execution-output-publication-failed";

pub(super) fn validate_resolved_context(request: &Request, resolved: &ResolvedContext) -> Result<(), String> {
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
        (None, None) => {}
        (Some(stdin_ref), Some(bytes)) => {
            if crate::preserves_rail::content_ref_from_bytes(bytes) != *stdin_ref {
                return Err("resolved stdin bytes do not hash to the admitted stdin reference".to_string());
            }
        }
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

/// Why the resolved executable could not be bound to its admitted measurement. Either way nothing
/// was spawned.
pub(super) enum ExecutableMeasurementFailure {
    /// The executable could not be opened, read, or bounded.
    Unavailable(String),
    /// The opened executable bytes differ from the admitted executable identity.
    Mismatch,
}

/// The resolved executable, opened once and measured from that handle. The process is spawned
/// through the same handle, so the bytes that run are the bytes that were measured.
pub(super) struct MeasuredExecutable {
    file: std::fs::File,
}

impl MeasuredExecutable {
    /// The handle-relative path the spawned program resolves to; it stays valid while `self` is
    /// alive.
    pub(super) fn process_path(&self) -> Result<std::path::PathBuf, String> {
        #[cfg(target_os = "linux")]
        {
            let descriptor = std::os::fd::AsRawFd::as_raw_fd(&self.file);
            Ok(std::path::PathBuf::from(format!("/proc/{}/fd/{descriptor}", std::process::id())))
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = &self.file;
            Err("measured executable handles require the admitted Linux process profile".to_string())
        }
    }
}

/// The `fstat` identity of an opened file. A rename, replacement, size change, or metadata change
/// alters at least one field; a same-size write within one timestamp tick of a coarse-grained
/// filesystem may not, so excluding writers remains the resolver capability's responsibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct FileStamp {
    device: u64,
    inode: u64,
    size: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

impl FileStamp {
    fn of(metadata: &std::fs::Metadata) -> Self {
        use std::os::unix::fs::MetadataExt as _;
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            size: metadata.size(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        }
    }
}

/// Executable identities this adapter has already measured, each with the stamp of the handle it
/// hashed. A handle whose stamp differs from the recorded one is hashed again.
#[derive(Debug, Default)]
pub(super) struct MeasurementCache {
    measured: std::collections::BTreeMap<String, FileStamp>,
}

const MEASUREMENT_BUFFER_BYTES: usize = 1 << 20;

// r[impl molten.bounded_execution.live_adapter_binding_and_no_replay]
pub(super) fn measure_executable(
    request: &Request,
    resolved: &ResolvedContext,
    cache: &mut MeasurementCache,
) -> Result<MeasuredExecutable, ExecutableMeasurementFailure> {
    // Non-blocking open, so naming a FIFO cannot stall the adapter before the regular-file check.
    let file = std::os::unix::fs::OpenOptionsExt::custom_flags(
        std::fs::OpenOptions::new().read(true),
        rustix::fs::OFlags::NONBLOCK.bits().cast_signed(),
    )
    .open(&resolved.executable_path)
    .map_err(|error| {
        ExecutableMeasurementFailure::Unavailable(format!("resolved executable could not be opened: {error}"))
    })?;
    // The length comes from the same handle; only a regular file has one, so a device or pipe cannot
    // make measurement unbounded.
    let metadata = file.metadata().map_err(|error| {
        ExecutableMeasurementFailure::Unavailable(format!("resolved executable could not be inspected: {error}"))
    })?;
    if !metadata.is_file() {
        return Err(ExecutableMeasurementFailure::Unavailable("resolved executable is not a regular file".to_string()));
    }
    let identity = &request.plan.resolution.executable_identity_ref;
    let stamp = FileStamp::of(&metadata);
    if cache.measured.get(identity) == Some(&stamp) {
        return Ok(MeasuredExecutable { file });
    }
    let mut hasher = blake3::Hasher::new();
    // Large reads let BLAKE3 hash many chunks per update instead of one small buffer at a time.
    let mut bounded = std::io::BufReader::with_capacity(
        MEASUREMENT_BUFFER_BYTES,
        std::io::Read::take(&file, metadata.len().saturating_add(1)),
    );
    let measured_bytes = std::io::copy(&mut bounded, &mut hasher).map_err(|error| {
        ExecutableMeasurementFailure::Unavailable(format!("resolved executable could not be measured: {error}"))
    })?;
    if measured_bytes != metadata.len() {
        return Err(ExecutableMeasurementFailure::Unavailable(
            "resolved executable changed length while it was measured".to_string(),
        ));
    }
    if crate::preserves_rail::content_ref_from_blake3_hash(hasher.finalize()) != *identity {
        cache.measured.remove(identity);
        return Err(ExecutableMeasurementFailure::Mismatch);
    }
    // A write during hashing changes the stamp; such a measurement is refused rather than cached.
    let after = file.metadata().map_err(|error| {
        ExecutableMeasurementFailure::Unavailable(format!("resolved executable could not be inspected: {error}"))
    })?;
    if FileStamp::of(&after) != stamp {
        return Err(ExecutableMeasurementFailure::Unavailable(
            "resolved executable changed while it was measured".to_string(),
        ));
    }
    cache.measured.insert(identity.clone(), stamp);
    Ok(MeasuredExecutable { file })
}

pub(super) fn bounded_request(
    request: &Request,
    resolved: &ResolvedContext,
    program: std::path::PathBuf,
) -> Result<bounded_exec::RunRequest, String> {
    let limits = request.plan.request.limits;
    let stdin_max_bytes = usize::try_from(limits.stdin_max_bytes)
        .map_err(|_| "stdin byte bound does not fit the host platform".to_string())?;
    let stdout_max_bytes = usize::try_from(limits.stdout_max_bytes)
        .map_err(|_| "stdout byte bound does not fit the host platform".to_string())?;
    let stderr_max_bytes = usize::try_from(limits.stderr_max_bytes)
        .map_err(|_| "stderr byte bound does not fit the host platform".to_string())?;
    let outcome_policy = bounded_exec::OutcomePolicy::new(
        request.plan.request.accepted_exit_codes.clone(),
        request.plan.request.reject_stdout_truncation,
        request.plan.request.reject_stderr_truncation,
    )
    .map_err(|error| format!("execution outcome policy is invalid: {error:?}"))?;
    let input = resolved.stdin_bytes.clone().map_or(bounded_exec::Input::Null, bounded_exec::Input::Bytes);
    Ok(bounded_exec::RunRequest {
        command: bounded_exec::CommandSpec {
            program,
            args: request.plan.request.arguments.iter().map(std::ffi::OsString::from).collect(),
            current_dir: resolved.workspace_path.clone(),
            environment_mode: bounded_exec::EnvironmentMode::Clear,
            environment: request
                .plan
                .request
                .environment
                .iter()
                .map(|entry| (std::ffi::OsString::from(&entry.name), std::ffi::OsString::from(&entry.value)))
                .collect(),
            input,
        },
        limits: bounded_exec::ExecutionLimits {
            timeout_ms: limits.timeout_ms,
            stdin_max_bytes,
            stdout_max_bytes,
            stderr_max_bytes,
            poll_interval_ms: limits.poll_interval_ms,
            teardown_timeout_ms: limits.teardown_timeout_ms,
        },
        termination_scope: match request.plan.request.termination_scope {
            ExecutionTerminationScope::DirectChild => bounded_exec::TerminationScope::Child,
            ExecutionTerminationScope::ProcessGroup => bounded_exec::TerminationScope::ProcessGroup,
        },
        outcome_policy,
    })
}

pub(super) fn process_observation(output: bounded_exec::ExecutionOutput) -> Result<ProcessObservation, String> {
    let lifecycle = match output.completion {
        bounded_exec::Completion::Exited => ExecutionLifecycleState::Exited,
        bounded_exec::Completion::TimedOut => ExecutionLifecycleState::TimedOut,
        bounded_exec::Completion::Cancelled => ExecutionLifecycleState::Cancelled,
    };
    let disposition = match output.disposition {
        bounded_exec::Disposition::Succeeded => ObservedDisposition::ExitPolicyAccepted,
        bounded_exec::Disposition::ExitFailed => ObservedDisposition::ExitPolicyRejected,
        bounded_exec::Disposition::TimedOut => ObservedDisposition::TimedOut,
        bounded_exec::Disposition::Cancelled => ObservedDisposition::Cancelled,
        bounded_exec::Disposition::OutputLimitExceeded(_) => ObservedDisposition::OutputPolicyRejected,
    };
    Ok(ProcessObservation {
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

fn retained_stream(role: &str, output: bounded_exec::CapturedOutput) -> Result<RetainedStream, String> {
    let observed_bytes = u64::try_from(output.observed_bytes)
        .map_err(|_| "observed execution output byte count does not fit u64".to_string())?;
    let retained_byte_count = u64::try_from(output.bytes.len())
        .map_err(|_| "retained execution output byte count does not fit u64".to_string())?;
    Ok(RetainedStream {
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
    stream: &RetainedStream,
) -> StreamPublication {
    match publisher.publish(operation_ref, stream) {
        Ok(published) => StreamPublication::Published(published),
        Err(_) => StreamPublication::Failed {
            diagnostic_code: PUBLICATION_FAILURE_CODE,
        },
    }
}

/// Process and receipt evidence attached to an execution port failure, when the run got that far.
pub(super) struct FailureEvidence {
    pub(super) process_observation: Option<ProcessObservation>,
    pub(super) receipt: Option<Receipt>,
}

impl FailureEvidence {
    pub(super) const NONE: Self = Self {
        process_observation: None,
        receipt: None,
    };
}
