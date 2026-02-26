//! Process spawning, output streaming, and execution lifecycle.

use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

use command_group::AsyncCommandGroup;
use command_group::AsyncGroupChild;
use snafu::ResultExt;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::termination::terminate_process_group;
use crate::agent::error;
use crate::agent::error::AgentError;
use crate::agent::error::Result;
use crate::agent::protocol::ExecutionRequest;
use crate::agent::protocol::LogMessage;

/// Maximum line length for stdout/stderr (64 KB).
/// Lines longer than this are truncated.
const MAX_LINE_LENGTH: usize = 64 * 1024;

/// Heartbeat interval during execution.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

/// Grace period for SIGTERM before SIGKILL.
const GRACE_PERIOD: Duration = Duration::from_secs(5);

/// Inner execution logic.
pub(crate) async fn execute_inner(
    request: &ExecutionRequest,
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

    // SAFETY: stdout/stderr are always Some because we set Stdio::piped() above.
    // Use ok_or instead of expect to satisfy Tiger Style (no panics in production).
    let stdout = child.inner().stdout.take().ok_or(AgentError::SpawnProcess {
        command: request.command.clone(),
        source: std::io::Error::other("stdout pipe not available"),
    })?;
    let stderr = child.inner().stderr.take().ok_or(AgentError::SpawnProcess {
        command: request.command.clone(),
        source: std::io::Error::other("stderr pipe not available"),
    })?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        // Verify constants are reasonable
        assert_eq!(MAX_LINE_LENGTH, 64 * 1024);
        assert_eq!(HEARTBEAT_INTERVAL, Duration::from_secs(30));
        assert_eq!(GRACE_PERIOD, Duration::from_secs(5));
    }
}
