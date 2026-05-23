//! Command execution engine for the CI agent.
//!
//! Handles spawning processes, streaming output, enforcing timeouts,
//! and process lifecycle management.

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use command_group::AsyncCommandGroup;
use command_group::AsyncGroupChild;
use snafu::ResultExt;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::agent::error::AgentError;
use crate::agent::error::Result;
use crate::agent::error::{self};
use crate::agent::protocol::ExecutionRequest;
use crate::agent::protocol::ExecutionResult;
use crate::agent::protocol::LogMessage;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const GRACE_PERIOD: Duration = Duration::from_secs(5);
const OUTPUT_DRAIN_GRACE: Duration = Duration::from_secs(2);
const MAX_LINE_LENGTH: usize = 64 * 1024;
const CI_PROGRESS_MARKER: &str = "ASPEN_CI_COMMAND_PROGRESS";

fn command_start_marker(request: &ExecutionRequest) -> String {
    format!(
        "{CI_PROGRESS_MARKER} phase=command_started job_id={} command={} args_count={} working_dir={} timeout_secs={}\n",
        request.id,
        request.command,
        request.args.len(),
        request.working_dir.display(),
        request.timeout_secs
    )
}

fn command_heartbeat_marker(job_id: &str, elapsed_secs: u64) -> String {
    format!("{CI_PROGRESS_MARKER} phase=command_running job_id={job_id} elapsed_secs={elapsed_secs}\n")
}

fn command_timeout_marker(job_id: &str, timeout_secs: u64, elapsed_secs: u64, origin: &'static str) -> String {
    format!(
        "{CI_PROGRESS_MARKER} phase=command_timeout job_id={job_id} timeout_secs={timeout_secs} elapsed_secs={elapsed_secs} origin={origin}\n"
    )
}

fn try_send_progress_marker(log_tx: &mpsc::Sender<LogMessage>, job_id: &str, marker: String, phase: &'static str) {
    match log_tx.try_send(LogMessage::Stderr(marker)) {
        Ok(()) => {}
        Err(mpsc::error::TrySendError::Full(_)) => {
            warn!(job_id = %job_id, phase, "dropping progress marker because log channel is full");
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            debug!(job_id = %job_id, phase, "dropping progress marker because log receiver is closed");
        }
    }
}

#[allow(unknown_lints)]
#[allow(ambient_clock, reason = "CI executor measures real monotonic process durations")]
fn monotonic_now() -> Instant {
    Instant::now()
}

fn elapsed_ms_u64(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn canonical_working_dir(path: &Path) -> Result<std::path::PathBuf> {
    path.canonicalize().map_err(|_| AgentError::InvalidWorkingDir {
        path: path.display().to_string(),
    })
}

/// Handle to a running job, used for cancellation.
pub struct JobHandle {
    /// Cancellation sender.
    cancel_tx: oneshot::Sender<()>,
}

impl JobHandle {
    /// Cancel the running job.
    pub fn cancel(self) {
        if self.cancel_tx.send(()).is_err() {
            debug!("job cancel receiver already dropped");
        }
    }
}

/// Executor that runs commands and streams output.
#[derive(Clone)]
pub struct Executor {
    /// Currently running jobs, keyed by job ID.
    running_jobs: Arc<Mutex<HashMap<String, JobHandle>>>,

    /// Workspace root path for directory validation.
    /// Working directories must be under this path.
    /// Defaults to `/workspace` for VM environments.
    workspace_root: std::path::PathBuf,
}

impl Executor {
    /// Create a new executor with default `/workspace` root.
    pub fn new() -> Self {
        Self {
            running_jobs: Arc::new(Mutex::new(HashMap::new())),
            workspace_root: std::path::PathBuf::from("/workspace"),
        }
    }

    /// Create a new executor with a custom workspace root.
    ///
    /// This is useful for local execution where the workspace
    /// is not mounted at `/workspace`.
    pub fn with_workspace_root(workspace_root: std::path::PathBuf) -> Self {
        Self {
            running_jobs: Arc::new(Mutex::new(HashMap::new())),
            workspace_root,
        }
    }

    /// Execute a command and stream output via the provided channel.
    ///
    /// Returns when the command completes or is cancelled.
    pub async fn execute(
        &self,
        request: ExecutionRequest,
        log_tx: mpsc::Sender<LogMessage>,
    ) -> Result<ExecutionResult> {
        let job_id = request.id.clone();
        let start = monotonic_now();

        // Validate working directory
        self.validate_working_dir(&request.working_dir)?;

        // Load nix database dump if present and command is nix-related.
        // The host generates this file with `nix-store --dump-db` after prefetching
        // the build closure. We load it here (not at startup) because the dump is
        // written AFTER the VM boots and the job is assigned.
        if is_nix_command(&request.command) {
            load_nix_db_dump(&self.workspace_root).await;
        }

        // Create cancellation channel
        let (cancel_tx, cancel_rx) = oneshot::channel();

        // Register job handle
        {
            let mut jobs = self.running_jobs.lock().await;
            jobs.insert(job_id.clone(), JobHandle { cancel_tx });
        }

        // Execute with cleanup on drop
        let result = self.execute_inner(request.clone(), log_tx.clone(), cancel_rx).await;

        // Unregister job
        {
            let mut jobs = self.running_jobs.lock().await;
            jobs.remove(&job_id);
        }

        let duration_ms = elapsed_ms_u64(start);

        match result {
            Ok((exit_code, stdout, stderr)) => Ok(ExecutionResult {
                id: job_id,
                exit_code,
                stdout,
                stderr,
                duration_ms,
                error: None,
                cache_hits: 0,
                cache_misses: 0,
                cache_time_saved_ms: 0,
            }),
            Err(e) => Ok(ExecutionResult {
                id: job_id,
                exit_code: -1,
                stdout: String::new(),
                stderr: String::new(),
                duration_ms,
                error: Some(e.to_string()),
                cache_hits: 0,
                cache_misses: 0,
                cache_time_saved_ms: 0,
            }),
        }
    }

    /// Cancel a running job by ID.
    pub async fn cancel(&self, job_id: &str) -> Result<()> {
        let handle = {
            let mut jobs = self.running_jobs.lock().await;
            jobs.remove(job_id)
        };

        match handle {
            Some(handle) => {
                handle.cancel();
                info!(job_id = %job_id, "job cancelled");
                Ok(())
            }
            None => error::JobNotFoundSnafu { id: job_id }.fail(),
        }
    }

    /// Check if a job is running.
    pub async fn is_running(&self, job_id: &str) -> bool {
        let jobs = self.running_jobs.lock().await;
        jobs.contains_key(job_id)
    }

    /// Validate that working directory is safe.
    fn validate_working_dir(&self, path: &Path) -> Result<()> {
        if !path.is_absolute() {
            return error::WorkingDirNotUnderWorkspaceSnafu {
                path: path.display().to_string(),
            }
            .fail();
        }

        let canonical_path = canonical_working_dir(path)?;
        if self.is_allowed_canonical_working_dir(path, &canonical_path)? {
            return Ok(());
        }

        error::WorkingDirNotUnderWorkspaceSnafu {
            path: path.display().to_string(),
        }
        .fail()
    }

    fn is_allowed_canonical_working_dir(&self, path: &Path, canonical_path: &Path) -> Result<bool> {
        let path_str = path.to_string_lossy();
        let canonical_str = canonical_path.to_string_lossy();
        if path_str.starts_with("/tmp/ci-workspace-") && canonical_str.starts_with("/tmp/ci-workspace-") {
            return Ok(true);
        }

        let Ok(workspace_root) = self.workspace_root.canonicalize() else {
            return Ok(false);
        };
        Ok(canonical_path.starts_with(&workspace_root))
    }

    /// Inner execution logic.
    async fn execute_inner(
        &self,
        request: ExecutionRequest,
        log_tx: mpsc::Sender<LogMessage>,
        mut cancel_rx: oneshot::Receiver<()>,
    ) -> Result<(i32, String, String)> {
        let start = monotonic_now();
        info!(
            job_id = %request.id,
            command = %request.command,
            working_dir = %request.working_dir.display(),
            timeout_secs = request.timeout_secs,
            "executing command"
        );

        // Emit a durable progress marker through the same stream that CI diagnostics preserve.
        try_send_progress_marker(&log_tx, &request.id, command_start_marker(&request), "command_started");

        // Build command
        let mut cmd = Command::new(&request.command);
        cmd.args(&request.args)
            .current_dir(&request.working_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // Set environment
        cmd.env_clear();
        for (key, value) in &request.env {
            cmd.env(key, value);
        }

        // Add essential PATH if not provided
        if !request.env.contains_key("PATH") {
            cmd.env("PATH", "/run/current-system/sw/bin:/nix/var/nix/profiles/default/bin:/usr/bin:/bin");
        }

        // Spawn as process group for clean termination
        let mut child: AsyncGroupChild = cmd.group_spawn().context(error::SpawnProcessSnafu {
            command: request.command.clone(),
        })?;

        let stdout = child.inner().stdout.take().ok_or_else(|| error::AgentError::SpawnProcess {
            command: request.command.clone(),
            source: std::io::Error::other("stdout pipe not available"),
        })?;
        let stderr = child.inner().stderr.take().ok_or_else(|| error::AgentError::SpawnProcess {
            command: request.command.clone(),
            source: std::io::Error::other("stderr pipe not available"),
        })?;

        let timeout_guard_fired = Arc::new(AtomicBool::new(false));
        let timeout_guard_handle = spawn_process_timeout_guard(
            child.inner().id(),
            request.id.clone(),
            request.timeout_secs,
            log_tx.clone(),
            start,
            timeout_guard_fired.clone(),
        );

        // Stream stdout
        let stdout_tx = log_tx.clone();
        let stdout_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            let mut collected = String::new();

            for _line_idx in 0..u32::MAX {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if line.len() > MAX_LINE_LENGTH {
                            line.truncate(MAX_LINE_LENGTH);
                            line.push_str("... [truncated]\n");
                        }
                        collected.push_str(&line);
                        if stdout_tx.send(LogMessage::Stdout(line.clone())).await.is_err() {
                            debug!("stdout log receiver dropped");
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("error reading stdout: {}", e);
                        break;
                    }
                }
            }
            collected
        });

        // Stream stderr
        let stderr_tx = log_tx.clone();
        let stderr_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            let mut collected = String::new();

            for _line_idx in 0..u32::MAX {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if line.len() > MAX_LINE_LENGTH {
                            line.truncate(MAX_LINE_LENGTH);
                            line.push_str("... [truncated]\n");
                        }
                        collected.push_str(&line);
                        if stderr_tx.send(LogMessage::Stderr(line.clone())).await.is_err() {
                            debug!("stderr log receiver dropped");
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("error reading stderr: {}", e);
                        break;
                    }
                }
            }
            collected
        });

        // Heartbeat task
        let heartbeat_tx = log_tx.clone();
        let job_id = request.id.clone();
        let heartbeat_handle = tokio::spawn(async move {
            let start = monotonic_now();
            let mut heartbeat_timer = tokio::time::interval(HEARTBEAT_INTERVAL);
            heartbeat_timer.tick().await; // Skip first immediate tick

            for _heartbeat_idx in 0..u32::MAX {
                heartbeat_timer.tick().await;
                let elapsed_secs = start.elapsed().as_secs();
                debug!(job_id = %job_id, elapsed_secs, "sending heartbeat");
                if heartbeat_tx
                    .send(LogMessage::Stderr(command_heartbeat_marker(&job_id, elapsed_secs)))
                    .await
                    .is_err()
                {
                    break;
                }
                if heartbeat_tx.send(LogMessage::Heartbeat { elapsed_secs }).await.is_err() {
                    break;
                }
            }
        });

        // Wait for completion with timeout and cancellation

        enum ExitReason {
            Completed(std::process::ExitStatus),
            WaitError(std::io::Error),
            Timeout,
            Cancelled,
        }

        let exit_reason = tokio::select! {
            wait_result = child.wait() => {
                match wait_result {
                    Ok(status) => ExitReason::Completed(status),
                    Err(e) => ExitReason::WaitError(e),
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(request.timeout_secs)) => {
                ExitReason::Timeout
            }
            _ = &mut cancel_rx => {
                ExitReason::Cancelled
            }
        };

        // Handle termination if needed
        let result: Result<i32> = match exit_reason {
            ExitReason::Completed(_) if timeout_guard_fired.load(Ordering::SeqCst) => {
                Err(AgentError::ExecutionTimeout {
                    timeout_secs: request.timeout_secs,
                })
            }
            ExitReason::Completed(status) => Ok(status.code().unwrap_or(-1)),
            ExitReason::WaitError(e) => {
                error!("process wait failed: {}", e);
                Ok(-1)
            }
            ExitReason::Timeout => {
                timeout_guard_fired.store(true, Ordering::SeqCst);
                warn!(job_id = %request.id, timeout_secs = request.timeout_secs, "execution timed out");
                try_send_progress_marker(
                    &log_tx,
                    &request.id,
                    command_timeout_marker(&request.id, request.timeout_secs, start.elapsed().as_secs(), "select"),
                    "command_timeout",
                );
                terminate_process_group(&mut child, GRACE_PERIOD).await;
                Err(AgentError::ExecutionTimeout {
                    timeout_secs: request.timeout_secs,
                })
            }
            ExitReason::Cancelled => {
                info!(job_id = %request.id, "execution cancelled");
                terminate_process_group(&mut child, GRACE_PERIOD).await;
                Ok(-15) // SIGTERM
            }
        };

        // Stop heartbeat and the independent process timeout guard.
        heartbeat_handle.abort();
        if let Some(handle) = timeout_guard_handle {
            handle.abort();
        }

        // Collect output. A timed-out process can leave stdout/stderr pipes open via
        // descendants, so bound the drain; otherwise the timeout marker is emitted
        // but the worker never returns a failed job result to the cluster.
        let stdout_result = collect_stream_output(stdout_handle, OUTPUT_DRAIN_GRACE, &request.id, "stdout").await;
        let stderr_result = collect_stream_output(stderr_handle, OUTPUT_DRAIN_GRACE, &request.id, "stderr").await;

        match result {
            Ok(exit_code) => Ok((exit_code, stdout_result, stderr_result)),
            Err(e) => Err(e),
        }
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(unix)]
fn spawn_process_timeout_guard(
    child_pid: Option<u32>,
    job_id: String,
    timeout_secs: u64,
    log_tx: mpsc::Sender<LogMessage>,
    start: Instant,
    fired: Arc<AtomicBool>,
) -> Option<tokio::task::JoinHandle<()>> {
    let pid = child_pid?;
    Some(tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(timeout_secs)).await;
        fired.store(true, Ordering::SeqCst);
        warn!(job_id = %job_id, timeout_secs, "process timeout guard firing");
        try_send_progress_marker(
            &log_tx,
            &job_id,
            command_timeout_marker(&job_id, timeout_secs, start.elapsed().as_secs(), "guard"),
            "command_timeout",
        );
        terminate_process_group_by_pid(pid, GRACE_PERIOD).await;
    }))
}

#[cfg(not(unix))]
fn spawn_process_timeout_guard(
    _child_pid: Option<u32>,
    _job_id: String,
    _timeout_secs: u64,
    _log_tx: mpsc::Sender<LogMessage>,
    _start: Instant,
    _fired: Arc<AtomicBool>,
) -> Option<tokio::task::JoinHandle<()>> {
    None
}

async fn collect_stream_output(
    mut handle: tokio::task::JoinHandle<String>,
    grace: Duration,
    job_id: &str,
    stream: &'static str,
) -> String {
    tokio::select! {
        joined = &mut handle => {
            match joined {
                Ok(output) => output,
                Err(error) => {
                    warn!(job_id = %job_id, stream, error = %error, "output stream task failed");
                    String::new()
                }
            }
        }
        _ = tokio::time::sleep(grace) => {
            warn!(job_id = %job_id, stream, grace_ms = grace.as_millis(), "output stream drain timed out");
            handle.abort();
            String::new()
        }
    }
}

/// Terminate a process group gracefully.
///
/// On Unix:
/// 1. Send SIGTERM to process group
/// 2. Wait for grace period
/// 3. Send SIGKILL if still running
/// 4. Reap the process
#[cfg(unix)]
async fn terminate_process_group_by_pid(pid: u32, grace: Duration) {
    use nix::sys::signal::Signal;
    use nix::sys::signal::{self};
    use nix::unistd::Pid;

    debug_assert!(grace >= Duration::from_millis(100));
    debug_assert!(grace <= Duration::from_secs(60));
    let pgid = Pid::from_raw(-(pid as i32));

    if let Err(e) = signal::kill(pgid, Signal::SIGTERM)
        && e != nix::errno::Errno::ESRCH
    {
        warn!(pid, error = ?e, "SIGTERM to process group failed");
    }

    tokio::time::sleep(grace).await;

    if let Err(e) = signal::kill(pgid, Signal::SIGKILL)
        && e != nix::errno::Errno::ESRCH
    {
        warn!(pid, error = ?e, "SIGKILL to process group failed");
    }
}

#[cfg(unix)]
async fn terminate_process_group(child: &mut AsyncGroupChild, grace: Duration) {
    debug_assert!(grace >= Duration::from_millis(100));
    debug_assert!(grace <= Duration::from_secs(60));
    let Some(pid) = child.inner().id() else {
        return; // Already exited
    };

    terminate_process_group_by_pid(pid, grace).await;

    // Reap
    if let Err(error) = child.wait().await {
        warn!(pid, "failed to reap process group child: {error}");
    }
}

#[cfg(not(unix))]
async fn terminate_process_group(child: &mut AsyncGroupChild, _grace: Duration) {
    // On non-Unix, just kill directly via the async method
    if let Err(error) = child.kill().await {
        warn!("failed to kill child process: {error}");
    }
    if let Err(error) = child.wait().await {
        warn!("failed to reap child process: {error}");
    }
}

/// Check if a command is nix-related (needs database dump loaded).
///
/// This handles:
/// - Direct nix commands: nix, nix-build, nix-shell, etc.
/// - Full paths: /nix/store/.../bin/nix, /run/current-system/sw/bin/nix
/// - Shell wrappers: Commands that might invoke nix internally
fn is_nix_command(cmd: &str) -> bool {
    // Direct command match
    if matches!(cmd, "nix" | "nix-build" | "nix-shell" | "nix-store" | "nix-env" | "nix-instantiate") {
        return true;
    }

    // Check if command is a path containing nix binary
    if cmd.contains("/nix") && cmd.contains("/bin/nix") {
        return true;
    }

    // Shell commands that might run nix internally should also trigger DB load
    // since they commonly wrap nix builds in CI pipelines
    if matches!(cmd, "sh" | "bash" | "zsh") {
        return true;
    }

    false
}

/// Metadata for the nix database dump, written by the host.
#[derive(Debug, serde::Deserialize)]
struct DbDumpMeta {
    /// Schema version (currently 1)
    #[allow(dead_code)]
    version: u32,
    /// Derivation path that was dumped
    drv_path: String,
    /// Number of store paths in the dump
    path_count: u64,
    /// Size of the dump file in bytes
    dump_size_bytes: u64,
    /// Timestamp when the dump was generated
    #[allow(dead_code)]
    generated_at: String,
}

async fn read_db_dump_meta(meta_path: &Path) -> Option<DbDumpMeta> {
    if !meta_path.exists() {
        return None;
    }

    match tokio::fs::read_to_string(meta_path).await {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(meta) => Some(meta),
            Err(error) => {
                debug!("failed to parse dump metadata: {error}");
                None
            }
        },
        Err(error) => {
            debug!("failed to read dump metadata: {error}");
            None
        }
    }
}

async fn read_db_dump_contents(dump_path: &Path) -> Option<(Vec<u8>, u64)> {
    use tokio::fs::File;
    use tokio::io::AsyncReadExt;

    debug_assert!(dump_path.is_absolute(), "dump path must be absolute");
    let dump_size_bytes = match tokio::fs::metadata(dump_path).await {
        Ok(metadata) => metadata.len(),
        Err(error) => {
            error!("failed to stat nix database dump: {error}");
            return None;
        }
    };

    let dump_file = match File::open(dump_path).await {
        Ok(file) => file,
        Err(error) => {
            error!("failed to open nix database dump: {error}");
            return None;
        }
    };

    let dump_capacity_bytes = usize::try_from(dump_size_bytes).unwrap_or(0);
    debug_assert!(dump_capacity_bytes == 0 || dump_size_bytes <= usize::MAX as u64);
    let mut dump_contents = Vec::with_capacity(dump_capacity_bytes);
    let mut dump_reader = tokio::io::BufReader::new(dump_file);
    if let Err(error) = dump_reader.read_to_end(&mut dump_contents).await {
        error!("failed to read nix database dump: {error}");
        return None;
    }

    Some((dump_contents, dump_size_bytes))
}

fn spawn_nix_store_loader() -> std::io::Result<tokio::process::Child> {
    use std::process::Stdio;

    Command::new("nix-store")
        .arg("--load-db")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

async fn write_db_dump_to_loader(child: &mut tokio::process::Child, dump_contents: &[u8]) -> bool {
    use tokio::io::AsyncWriteExt;

    if let Some(mut stdin) = child.stdin.take()
        && let Err(error) = stdin.write_all(dump_contents).await
    {
        error!("failed to write to nix-store stdin: {error}");
        return false;
    }

    true
}

fn log_db_dump_load_result(
    status: std::process::ExitStatus,
    start: Instant,
    dump_size_bytes: u64,
    meta: Option<&DbDumpMeta>,
) {
    let elapsed_ms = elapsed_ms_u64(start);
    if status.success() {
        info!(
            dump_size_bytes,
            path_count = meta.map(|dump_meta| dump_meta.path_count),
            elapsed_ms,
            "nix database dump loaded successfully - store paths should now be recognized"
        );
        return;
    }

    error!(
        exit_code = status.code(),
        elapsed_ms, "nix-store --load-db failed - build will likely rebuild from scratch"
    );
}

/// Load nix database dump from the workspace if present.
///
/// The host generates a database dump after prefetching the build closure.
/// This dump contains metadata for store paths shared via virtiofs - the
/// paths exist in /nix/store but the VM's nix-daemon doesn't know about them.
/// Loading this dump makes nix recognize these paths as valid.
///
/// This function also reads the metadata file for verification and logging.
async fn load_nix_db_dump(workspace_root: &Path) {
    let dump_path = workspace_root.join(".nix-db-dump");
    let meta_path = workspace_root.join(".nix-db-dump.meta");

    debug_assert!(workspace_root.is_absolute(), "workspace root must be absolute");
    debug_assert!(dump_path.starts_with(workspace_root), "dump path must stay under workspace root");
    if !dump_path.exists() {
        info!(dump_path = %dump_path.display(), "no nix database dump found - skipping DB load");
        return;
    }

    let start = monotonic_now();
    let meta = read_db_dump_meta(&meta_path).await;
    let Some((dump_contents, dump_size_bytes)) = read_db_dump_contents(&dump_path).await else {
        return;
    };

    if let Some(ref dump_meta) = meta
        && dump_size_bytes != dump_meta.dump_size_bytes
    {
        warn!(
            expected = dump_meta.dump_size_bytes,
            actual = dump_size_bytes,
            "dump file size mismatch - file may be corrupted or incomplete"
        );
    }

    info!(
        dump_path = %dump_path.display(),
        dump_size_bytes,
        path_count = meta.as_ref().map(|dump_meta| dump_meta.path_count),
        drv_path = meta.as_ref().map(|dump_meta| dump_meta.drv_path.as_str()),
        "loading nix database dump"
    );

    let mut child = match spawn_nix_store_loader() {
        Ok(child) => child,
        Err(error) => {
            error!("failed to spawn nix-store --load-db: {error}");
            return;
        }
    };

    if !write_db_dump_to_loader(&mut child, &dump_contents).await {
        return;
    }

    match child.wait().await {
        Ok(status) => log_db_dump_load_result(status, start, dump_size_bytes, meta.as_ref()),
        Err(error) => error!("failed to wait for nix-store --load-db: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validate_working_dir_rejects_outside_workspace() {
        let executor = Executor::new();

        let result = executor.validate_working_dir(Path::new("/tmp"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("/workspace"));
    }

    #[tokio::test]
    async fn test_validate_working_dir_rejects_root() {
        let executor = Executor::new();

        let result = executor.validate_working_dir(Path::new("/"));
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_working_dir_rejects_relative_path() {
        let executor = Executor::new();

        let result = executor.validate_working_dir(Path::new("workspace/project"));
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_working_dir_accepts_canonical_workspace_child() {
        let workspace = tempfile::tempdir().unwrap();
        let checkout = workspace.path().join("checkout");
        std::fs::create_dir(&checkout).unwrap();
        let executor = Executor::with_workspace_root(workspace.path().to_path_buf());

        let result = executor.validate_working_dir(&checkout);
        assert!(result.is_ok());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_validate_working_dir_rejects_symlink_escape() {
        let workspace = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let link = workspace.path().join("escape");
        std::os::unix::fs::symlink(outside.path(), &link).unwrap();
        let executor = Executor::with_workspace_root(workspace.path().to_path_buf());

        let result = executor.validate_working_dir(&link);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("/workspace"));
    }

    #[tokio::test]
    async fn test_executor_is_running_empty() {
        let executor = Executor::new();
        assert!(!executor.is_running("nonexistent-job").await);
    }

    #[tokio::test]
    async fn test_cancel_nonexistent_job() {
        let executor = Executor::new();

        let result = executor.cancel("nonexistent-job").await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_job_handle_cancel() {
        let (tx, rx) = oneshot::channel();
        let handle = JobHandle { cancel_tx: tx };

        // Cancel should send signal
        handle.cancel();

        // Receiver should get the signal
        assert!(rx.await.is_ok());
    }

    #[test]
    fn test_command_progress_markers_are_bounded_and_redacted() {
        let request = ExecutionRequest {
            id: "job-123".to_string(),
            command: "nix".to_string(),
            args: vec!["build".to_string(), ".#clippy".to_string()],
            working_dir: Path::new("/workspace/job-123").to_path_buf(),
            env: HashMap::new(),
            timeout_secs: 7200,
        };

        let started = command_start_marker(&request);
        assert!(started.contains("ASPEN_CI_COMMAND_PROGRESS phase=command_started"));
        assert!(started.contains("job_id=job-123"));
        assert!(started.contains("command=nix"));
        assert!(started.contains("args_count=2"));
        assert!(started.contains("timeout_secs=7200"));
        assert!(!started.contains(".#clippy"));

        let running = command_heartbeat_marker("job-123", 60);
        assert_eq!(running, "ASPEN_CI_COMMAND_PROGRESS phase=command_running job_id=job-123 elapsed_secs=60\n");

        let timeout = command_timeout_marker("job-123", 7200, 7201, "select");
        assert_eq!(
            timeout,
            "ASPEN_CI_COMMAND_PROGRESS phase=command_timeout job_id=job-123 timeout_secs=7200 elapsed_secs=7201 origin=select\n"
        );
    }

    #[tokio::test]
    async fn test_timeout_returns_even_when_log_channel_is_full() {
        let workspace = tempfile::tempdir().unwrap();
        let executor = Executor::with_workspace_root(workspace.path().to_path_buf());
        let (log_tx, mut log_rx) = mpsc::channel(1);
        log_tx.send(LogMessage::Heartbeat { elapsed_secs: 0 }).await.unwrap();

        let request = ExecutionRequest {
            id: "full-log-timeout".to_string(),
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "sleep 30".to_string()],
            working_dir: workspace.path().to_path_buf(),
            env: HashMap::new(),
            timeout_secs: 1,
        };

        let started = monotonic_now();
        let result = tokio::time::timeout(Duration::from_secs(10), executor.execute(request, log_tx))
            .await
            .expect("executor should not hang on full log channel")
            .expect("execution result should be returned");

        assert_eq!(result.exit_code, -1);
        assert!(result.error.unwrap().contains("timed out"));
        assert!(started.elapsed() < Duration::from_secs(10));
        assert!(matches!(log_rx.try_recv(), Ok(LogMessage::Heartbeat { elapsed_secs: 0 })));
    }

    #[tokio::test]
    async fn test_collect_stream_output_returns_completed_output() {
        let handle = tokio::spawn(async { "done".to_string() });

        let output = collect_stream_output(handle, Duration::from_secs(1), "job-123", "stdout").await;

        assert_eq!(output, "done");
    }

    #[tokio::test]
    async fn test_collect_stream_output_bounds_stuck_reader() {
        let handle = tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
            "late".to_string()
        });
        let started = monotonic_now();

        let output = collect_stream_output(handle, Duration::from_millis(10), "job-123", "stderr").await;

        assert!(output.is_empty());
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn test_constants() {
        // Verify constants are reasonable
        assert_eq!(MAX_LINE_LENGTH, 64 * 1024);
        assert_eq!(HEARTBEAT_INTERVAL, Duration::from_secs(30));
        assert_eq!(GRACE_PERIOD, Duration::from_secs(5));
        assert_eq!(OUTPUT_DRAIN_GRACE, Duration::from_secs(2));
    }

    #[test]
    fn test_executor_default() {
        let executor = Executor::default();
        // Just verify it can be created via Default
        assert!(std::ptr::eq(&executor as *const _, &executor as *const _));
    }

    #[test]
    fn test_is_nix_command_direct_commands() {
        // Direct nix commands
        assert!(is_nix_command("nix"));
        assert!(is_nix_command("nix-build"));
        assert!(is_nix_command("nix-shell"));
        assert!(is_nix_command("nix-store"));
        assert!(is_nix_command("nix-env"));
        assert!(is_nix_command("nix-instantiate"));
    }

    #[test]
    fn test_is_nix_command_paths() {
        // Full paths to nix binaries
        assert!(is_nix_command("/nix/store/abc123/bin/nix"));
        assert!(is_nix_command("/run/current-system/sw/bin/nix-build"));
        assert!(is_nix_command("/nix/var/nix/profiles/default/bin/nix"));
    }

    #[test]
    fn test_is_nix_command_shell_wrappers() {
        // Shell commands that might invoke nix
        assert!(is_nix_command("sh"));
        assert!(is_nix_command("bash"));
        assert!(is_nix_command("zsh"));
    }

    #[test]
    fn test_is_nix_command_non_nix() {
        // Commands that should not trigger DB load
        assert!(!is_nix_command("cargo"));
        assert!(!is_nix_command("rustc"));
        assert!(!is_nix_command("make"));
        assert!(!is_nix_command("gcc"));
        assert!(!is_nix_command("ls"));
        assert!(!is_nix_command("/usr/bin/python"));
    }
}
