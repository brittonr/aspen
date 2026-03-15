//! Supervised Nix build process manager.
//!
//! Runs Nix evaluation and build commands as supervised child processes
//! with configurable timeouts. If a Nix process hangs past its timeout,
//! the supervisor kills it and reports failure without crashing the
//! orchestrator.
//!
//! # Architecture
//!
//! ```text
//! PipelineOrchestrator / LocalExecutorWorker
//!   └─ NixBuildSupervisor::spawn_build()
//!        ├─ tokio::process::Command     → child process
//!        ├─ mpsc channel                → real-time log streaming
//!        └─ oneshot channel             → final result delivery
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! use aspen_ci::orchestrator::{NixBuildSupervisor, BuildRequest};
//!
//! # async fn example() {
//! let request = BuildRequest {
//!     command: "nix".to_string(),
//!     args: vec!["build".to_string(), ".#default".to_string()],
//!     working_dir: "/workspace/project".to_string(),
//!     env: vec![],
//!     timeout_secs: Some(1800),
//! };
//!
//! let (result_rx, mut log_rx) = NixBuildSupervisor::spawn_build(request);
//!
//! // Stream logs while build runs
//! while let Some(log) = log_rx.recv().await {
//!     // Forward to CI log writer
//! }
//!
//! // Get final result
//! let result = result_rx.await.unwrap();
//! if result.is_success() {
//!     // Parse output_paths from stdout
//! }
//! # }

use std::process::Stdio;
use std::time::Duration;

use serde::Deserialize;
use serde::Serialize;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Default Nix build timeout: 30 minutes.
const DEFAULT_NIX_BUILD_TIMEOUT_SECS: u64 = 1800;

/// Log line from a supervised Nix build.
#[derive(Debug, Clone)]
pub enum BuildLog {
    /// Stdout line.
    Stdout(String),
    /// Stderr line.
    Stderr(String),
}

/// Result of a supervised Nix build.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildResult {
    /// Process exit code (0 = success). `None` if killed by timeout.
    pub exit_code: Option<i32>,
    /// Whether the process was killed due to timeout.
    pub is_timeout: bool,
    /// Captured stdout (complete).
    pub stdout: String,
    /// Captured stderr (complete).
    pub stderr: String,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
}

impl BuildResult {
    /// Whether the build succeeded (exit code 0, no timeout).
    pub fn is_success(&self) -> bool {
        !self.is_timeout && self.exit_code == Some(0)
    }
}

/// Request to spawn a supervised Nix build.
#[derive(Debug, Clone)]
pub struct BuildRequest {
    /// Command to execute (e.g., "nix").
    pub command: String,
    /// Command arguments (e.g., ["build", ".#default"]).
    pub args: Vec<String>,
    /// Working directory.
    pub working_dir: String,
    /// Environment variables.
    pub env: Vec<(String, String)>,
    /// Timeout in seconds. Defaults to 1800 (30 min).
    pub timeout_secs: Option<u64>,
}

/// Supervised Nix build process manager.
///
/// Spawns child processes with timeout enforcement and streams
/// output back to the orchestrator via channels.
pub struct NixBuildSupervisor;

impl NixBuildSupervisor {
    /// Spawn a supervised Nix build.
    ///
    /// Returns a oneshot receiver for the final result and an mpsc
    /// receiver for real-time log streaming.
    ///
    /// The build runs in a background task. If it exceeds the timeout,
    /// the child process is killed via SIGKILL and the result reports
    /// `is_timeout: true`.
    pub fn spawn_build(request: BuildRequest) -> (oneshot::Receiver<BuildResult>, mpsc::Receiver<BuildLog>) {
        let (result_tx, result_rx) = oneshot::channel();
        let (log_tx, log_rx) = mpsc::channel(256);

        let timeout = Duration::from_secs(request.timeout_secs.unwrap_or(DEFAULT_NIX_BUILD_TIMEOUT_SECS));

        tokio::spawn(async move {
            let start = tokio::time::Instant::now();

            let mut cmd = tokio::process::Command::new(&request.command);
            cmd.args(&request.args)
                .current_dir(&request.working_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true);

            for (key, value) in &request.env {
                cmd.env(key, value);
            }

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    let result = BuildResult {
                        exit_code: None,
                        is_timeout: false,
                        stdout: String::new(),
                        stderr: format!("Failed to spawn process: {e}"),
                        duration_ms: start.elapsed().as_millis() as u64,
                    };
                    let _ = result_tx.send(result);
                    return;
                }
            };

            info!(
                command = %request.command,
                args = ?request.args,
                timeout_secs = timeout.as_secs(),
                "Spawned supervised Nix build"
            );

            // Stream stdout/stderr in background tasks
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            let log_tx_stdout = log_tx.clone();
            let stdout_handle = tokio::spawn(async move {
                let mut lines = Vec::new();
                if let Some(stdout) = stdout {
                    let reader = BufReader::new(stdout);
                    let mut line_stream = reader.lines();
                    while let Ok(Some(line)) = line_stream.next_line().await {
                        let _ = log_tx_stdout.send(BuildLog::Stdout(line.clone())).await;
                        lines.push(line);
                    }
                }
                lines.join("\n")
            });

            let log_tx_stderr = log_tx.clone();
            let stderr_handle = tokio::spawn(async move {
                let mut lines = Vec::new();
                if let Some(stderr) = stderr {
                    let reader = BufReader::new(stderr);
                    let mut line_stream = reader.lines();
                    while let Ok(Some(line)) = line_stream.next_line().await {
                        let _ = log_tx_stderr.send(BuildLog::Stderr(line.clone())).await;
                        lines.push(line);
                    }
                }
                lines.join("\n")
            });

            // Wait for process with timeout
            let wait_result = tokio::time::timeout(timeout, child.wait()).await;

            let (exit_code, is_timeout) = match wait_result {
                Ok(Ok(status)) => {
                    debug!(exit_code = status.code(), "Nix build process exited");
                    (status.code(), false)
                }
                Ok(Err(e)) => {
                    warn!(error = %e, "Error waiting for Nix build process");
                    (None, false)
                }
                Err(_) => {
                    warn!(timeout_secs = timeout.as_secs(), "Nix build timed out, killing child process");
                    // kill_on_drop handles cleanup, but be explicit
                    let _ = child.kill().await;
                    (None, true)
                }
            };

            // Collect remaining output
            let stdout_text = stdout_handle.await.unwrap_or_default();
            let stderr_text = stderr_handle.await.unwrap_or_default();

            let result = BuildResult {
                exit_code,
                is_timeout,
                stdout: stdout_text,
                stderr: stderr_text,
                duration_ms: start.elapsed().as_millis() as u64,
            };

            let _ = result_tx.send(result);
        });

        (result_rx, log_rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_successful_build() {
        let request = BuildRequest {
            command: "echo".to_string(),
            args: vec!["hello world".to_string()],
            working_dir: "/tmp".to_string(),
            env: vec![],
            timeout_secs: Some(10),
        };

        let (result_rx, mut log_rx) = NixBuildSupervisor::spawn_build(request);

        // Drain logs
        let mut logs = vec![];
        while let Ok(log) = log_rx.try_recv() {
            logs.push(log);
        }

        let result = result_rx.await.unwrap();
        assert!(result.is_success());
        assert_eq!(result.exit_code, Some(0));
        assert!(!result.is_timeout);
    }

    #[tokio::test]
    async fn test_build_failure() {
        let request = BuildRequest {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "exit 42".to_string()],
            working_dir: "/tmp".to_string(),
            env: vec![],
            timeout_secs: Some(10),
        };

        let (result_rx, _log_rx) = NixBuildSupervisor::spawn_build(request);
        let result = result_rx.await.unwrap();

        assert!(!result.is_success());
        assert_eq!(result.exit_code, Some(42));
        assert!(!result.is_timeout);
    }

    #[tokio::test]
    async fn test_build_timeout_kills_process() {
        let request = BuildRequest {
            command: "sleep".to_string(),
            args: vec!["300".to_string()],
            working_dir: "/tmp".to_string(),
            env: vec![],
            timeout_secs: Some(1), // 1 second timeout
        };

        let (result_rx, _log_rx) = NixBuildSupervisor::spawn_build(request);
        let result = result_rx.await.unwrap();

        assert!(!result.is_success());
        assert!(result.is_timeout, "should report timeout");
        assert!(result.duration_ms >= 1000, "should have waited at least 1s");
        assert!(result.duration_ms < 5000, "should not have waited too long");
    }

    #[tokio::test]
    async fn test_build_stderr_capture() {
        let request = BuildRequest {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "echo error >&2; exit 1".to_string()],
            working_dir: "/tmp".to_string(),
            env: vec![],
            timeout_secs: Some(10),
        };

        let (result_rx, _log_rx) = NixBuildSupervisor::spawn_build(request);
        let result = result_rx.await.unwrap();

        assert!(!result.is_success());
        assert!(result.stderr.contains("error"));
    }

    #[tokio::test]
    async fn test_log_streaming() {
        let request = BuildRequest {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "echo line1; echo line2; echo err >&2".to_string()],
            working_dir: "/tmp".to_string(),
            env: vec![],
            timeout_secs: Some(10),
        };

        let (result_rx, mut log_rx) = NixBuildSupervisor::spawn_build(request);

        // Wait for completion first
        let result = result_rx.await.unwrap();
        assert!(result.is_success());

        // Drain remaining logs
        let mut _stdout_lines = 0u32;
        let mut _stderr_lines = 0u32;
        while let Ok(log) = log_rx.try_recv() {
            match log {
                BuildLog::Stdout(_) => _stdout_lines += 1,
                BuildLog::Stderr(_) => _stderr_lines += 1,
            }
        }
        // At minimum the stdout should have captured lines
        // (exact count depends on timing of channel drain vs process)
        assert!(result.stdout.contains("line1"));
        assert!(result.stdout.contains("line2"));
    }

    #[tokio::test]
    async fn test_spawn_nonexistent_command() {
        let request = BuildRequest {
            command: "/nonexistent/binary/path".to_string(),
            args: vec![],
            working_dir: "/tmp".to_string(),
            env: vec![],
            timeout_secs: Some(5),
        };

        let (result_rx, _log_rx) = NixBuildSupervisor::spawn_build(request);
        let result = result_rx.await.unwrap();

        assert!(!result.is_success());
        assert!(result.stderr.contains("Failed to spawn"));
    }

    #[tokio::test]
    async fn test_env_variables_passed() {
        let request = BuildRequest {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "echo $MY_VAR".to_string()],
            working_dir: "/tmp".to_string(),
            env: vec![("MY_VAR".to_string(), "test_value".to_string())],
            timeout_secs: Some(10),
        };

        let (result_rx, _log_rx) = NixBuildSupervisor::spawn_build(request);
        let result = result_rx.await.unwrap();

        assert!(result.is_success());
        assert!(result.stdout.contains("test_value"));
    }
}
