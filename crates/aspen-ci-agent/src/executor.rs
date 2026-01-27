//! Command execution engine for the CI agent.
//!
//! Handles spawning processes, streaming output, enforcing timeouts,
//! and process lifecycle management.

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use command_group::{AsyncCommandGroup, AsyncGroupChild};
use snafu::ResultExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{Mutex, mpsc, oneshot};
use tracing::{debug, error, info, warn};

use crate::error::{self, AgentError, Result};
use crate::protocol::{ExecutionRequest, ExecutionResult, LogMessage};

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
}

impl Executor {
    /// Create a new executor.
    pub fn new() -> Self {
        Self {
            running_jobs: Arc::new(Mutex::new(HashMap::new())),
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
        // Must be under /workspace (virtiofs mount point)
        if !path.starts_with("/workspace") {
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
    use nix::sys::signal::{self, Signal};
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
}
