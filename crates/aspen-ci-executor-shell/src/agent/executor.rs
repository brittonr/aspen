//! Command execution engine for the CI agent.
//!
//! Handles spawning processes, streaming output, enforcing timeouts,
//! and process lifecycle management.

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
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

/// Maximum line length for stdout/stderr (64 KB).
/// Lines longer than this are truncated.
const MAX_LINE_LENGTH: usize = 64 * 1024;

/// Heartbeat interval during execution.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

/// Grace period for SIGTERM before SIGKILL.
const GRACE_PERIOD: Duration = Duration::from_secs(5);

/// Handle to a running job, used for cancellation.
pub struct JobHandle {
    /// Cancellation sender.
    cancel_tx: oneshot::Sender<()>,
}

impl JobHandle {
    /// Cancel the running job.
    pub fn cancel(self) {
        let _ = self.cancel_tx.send(());
    }
}

/// Executor that runs commands and streams output.
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
        let start = Instant::now();

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

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok((exit_code, stdout, stderr)) => Ok(ExecutionResult {
                id: job_id,
                exit_code,
                stdout,
                stderr,
                duration_ms,
                error: None,
            }),
            Err(e) => Ok(ExecutionResult {
                id: job_id,
                exit_code: -1,
                stdout: String::new(),
                stderr: String::new(),
                duration_ms,
                error: Some(e.to_string()),
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
        // Must be under the configured workspace root
        if !path.starts_with(&self.workspace_root) {
            return error::WorkingDirNotUnderWorkspaceSnafu {
                path: path.display().to_string(),
            }
            .fail();
        }

        // Check it exists
        if !path.exists() {
            return error::InvalidWorkingDirSnafu {
                path: path.display().to_string(),
            }
            .fail();
        }

        Ok(())
    }

    /// Inner execution logic.
    async fn execute_inner(
        &self,
        request: ExecutionRequest,
        log_tx: mpsc::Sender<LogMessage>,
        mut cancel_rx: oneshot::Receiver<()>,
    ) -> Result<(i32, String, String)> {
        info!(
            job_id = %request.id,
            command = %request.command,
            working_dir = %request.working_dir.display(),
            timeout_secs = request.timeout_secs,
            "executing command"
        );

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

        let stdout = child.inner().stdout.take().expect("stdout piped");
        let stderr = child.inner().stderr.take().expect("stderr piped");

        // Stream stdout
        let stdout_tx = log_tx.clone();
        let stdout_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            let mut collected = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        // Truncate very long lines
                        if line.len() > MAX_LINE_LENGTH {
                            line.truncate(MAX_LINE_LENGTH);
                            line.push_str("... [truncated]\n");
                        }
                        collected.push_str(&line);
                        let _ = stdout_tx.send(LogMessage::Stdout(line.clone())).await;
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

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if line.len() > MAX_LINE_LENGTH {
                            line.truncate(MAX_LINE_LENGTH);
                            line.push_str("... [truncated]\n");
                        }
                        collected.push_str(&line);
                        let _ = stderr_tx.send(LogMessage::Stderr(line.clone())).await;
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
            let start = Instant::now();
            let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
            interval.tick().await; // Skip first immediate tick

            loop {
                interval.tick().await;
                let elapsed_secs = start.elapsed().as_secs();
                debug!(job_id = %job_id, elapsed_secs, "sending heartbeat");
                if heartbeat_tx.send(LogMessage::Heartbeat { elapsed_secs }).await.is_err() {
                    break;
                }
            }
        });

        // Wait for completion with timeout and cancellation
        let timeout = Duration::from_secs(request.timeout_secs);

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
            _ = tokio::time::sleep(timeout) => {
                ExitReason::Timeout
            }
            _ = &mut cancel_rx => {
                ExitReason::Cancelled
            }
        };

        // Handle termination if needed
        let result: Result<i32> = match exit_reason {
            ExitReason::Completed(status) => Ok(status.code().unwrap_or(-1)),
            ExitReason::WaitError(e) => {
                error!("process wait failed: {}", e);
                Ok(-1)
            }
            ExitReason::Timeout => {
                warn!(job_id = %request.id, timeout_secs = request.timeout_secs, "execution timed out");
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

        // Stop heartbeat
        heartbeat_handle.abort();

        // Collect output
        let stdout_result = stdout_handle.await.unwrap_or_default();
        let stderr_result = stderr_handle.await.unwrap_or_default();

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

/// Terminate a process group gracefully.
///
/// On Unix:
/// 1. Send SIGTERM to process group
/// 2. Wait for grace period
/// 3. Send SIGKILL if still running
/// 4. Reap the process
#[cfg(unix)]
async fn terminate_process_group(child: &mut AsyncGroupChild, grace: Duration) {
    use nix::sys::signal::Signal;
    use nix::sys::signal::{self};
    use nix::unistd::Pid;

    let Some(pid) = child.inner().id() else {
        return; // Already exited
    };
    let pgid = Pid::from_raw(-(pid as i32));

    // Send SIGTERM to process group
    if let Err(e) = signal::kill(pgid, Signal::SIGTERM)
        && e != nix::errno::Errno::ESRCH
    {
        warn!(pid, error = ?e, "SIGTERM to process group failed");
    }

    // Wait for graceful exit
    let deadline = tokio::time::Instant::now() + grace;
    while tokio::time::Instant::now() < deadline {
        if child.inner().try_wait().ok().flatten().is_some() {
            return; // Exited gracefully
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Force kill
    if let Err(e) = signal::kill(pgid, Signal::SIGKILL)
        && e != nix::errno::Errno::ESRCH
    {
        warn!(pid, error = ?e, "SIGKILL to process group failed");
    }

    // Reap
    let _ = child.wait().await;
}

#[cfg(not(unix))]
async fn terminate_process_group(child: &mut AsyncGroupChild, _grace: Duration) {
    // On non-Unix, just kill directly via the async method
    let _ = child.kill().await;
    let _ = child.wait().await;
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

/// Load nix database dump from the workspace if present.
///
/// The host generates a database dump after prefetching the build closure.
/// This dump contains metadata for store paths shared via virtiofs - the
/// paths exist in /nix/store but the VM's nix-daemon doesn't know about them.
/// Loading this dump makes nix recognize these paths as valid.
///
/// This function also reads the metadata file for verification and logging.
async fn load_nix_db_dump(workspace_root: &std::path::Path) {
    use std::process::Stdio;
    use std::time::Instant;

    use tokio::fs::File;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;

    let dump_path = workspace_root.join(".nix-db-dump");
    let meta_path = workspace_root.join(".nix-db-dump.meta");

    if !dump_path.exists() {
        info!(dump_path = %dump_path.display(), "no nix database dump found - skipping DB load");
        return;
    }

    let start = Instant::now();

    // Read metadata if available for logging
    let meta: Option<DbDumpMeta> = if meta_path.exists() {
        match tokio::fs::read_to_string(&meta_path).await {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(m) => Some(m),
                Err(e) => {
                    debug!("failed to parse dump metadata: {}", e);
                    None
                }
            },
            Err(e) => {
                debug!("failed to read dump metadata: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Read the dump file size for logging
    let dump_size = match tokio::fs::metadata(&dump_path).await {
        Ok(meta) => meta.len(),
        Err(e) => {
            error!("failed to stat nix database dump: {}", e);
            return;
        }
    };

    // Verify dump size matches metadata if available
    if let Some(ref m) = meta
        && dump_size != m.dump_size_bytes
    {
        warn!(
            expected = m.dump_size_bytes,
            actual = dump_size,
            "dump file size mismatch - file may be corrupted or incomplete"
        );
    }

    info!(
        dump_path = %dump_path.display(),
        dump_size_bytes = dump_size,
        path_count = meta.as_ref().map(|m| m.path_count),
        drv_path = meta.as_ref().map(|m| m.drv_path.as_str()),
        "loading nix database dump"
    );

    // Open the dump file for reading
    let dump_file = match File::open(&dump_path).await {
        Ok(f) => f,
        Err(e) => {
            error!("failed to open nix database dump: {}", e);
            return;
        }
    };

    // Read the entire dump into memory (should be reasonable size)
    let mut dump_contents = Vec::new();
    let mut dump_reader = tokio::io::BufReader::new(dump_file);
    if let Err(e) = dump_reader.read_to_end(&mut dump_contents).await {
        error!("failed to read nix database dump: {}", e);
        return;
    }

    // Load the database using nix-store --load-db
    // This requires piping the dump to stdin
    let mut child = match Command::new("nix-store")
        .arg("--load-db")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            error!("failed to spawn nix-store --load-db: {}", e);
            return;
        }
    };

    // Write dump to stdin
    if let Some(mut stdin) = child.stdin.take()
        && let Err(e) = stdin.write_all(&dump_contents).await
    {
        error!("failed to write to nix-store stdin: {}", e);
        return;
    }
    // stdin is dropped here to close it and signal EOF

    // Wait for completion
    match child.wait().await {
        Ok(status) => {
            let elapsed = start.elapsed();
            if status.success() {
                info!(
                    dump_size_bytes = dump_size,
                    path_count = meta.as_ref().map(|m| m.path_count),
                    elapsed_ms = elapsed.as_millis(),
                    "nix database dump loaded successfully - store paths should now be recognized"
                );
            } else {
                error!(
                    exit_code = status.code(),
                    elapsed_ms = elapsed.as_millis(),
                    "nix-store --load-db failed - build will likely rebuild from scratch"
                );
            }
        }
        Err(e) => {
            error!("failed to wait for nix-store --load-db: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validate_working_dir_rejects_outside_workspace() {
        let executor = Executor::new();

        let result = executor.validate_working_dir(Path::new("/tmp/evil"));
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
    fn test_constants() {
        // Verify constants are reasonable
        assert_eq!(MAX_LINE_LENGTH, 64 * 1024);
        assert_eq!(HEARTBEAT_INTERVAL, Duration::from_secs(30));
        assert_eq!(GRACE_PERIOD, Duration::from_secs(5));
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
