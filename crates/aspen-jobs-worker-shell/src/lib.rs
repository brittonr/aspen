//! Shell command worker for executing system commands as distributed jobs.
//!
//! This worker executes shell commands with:
//! - Capability-based authorization via `aspen-auth` tokens
//! - Process group management for clean termination
//! - Graceful shutdown (SIGTERM -> SIGKILL)
//! - Output capture with size limits
//! - Large output streaming to blob store
//!
//! # Security
//!
//! Every job requires a valid `CapabilityToken` with `ShellExecute` capability.
//! Tokens can scope permissions to specific commands and working directories.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use aspen_auth::CapabilityToken;
use aspen_auth::TokenVerifier;
use aspen_blob::prelude::*;
use aspen_jobs::Job;
use aspen_jobs::JobError;
use aspen_jobs::JobFailure;
use aspen_jobs::JobResult;
use aspen_jobs::Worker;
use async_trait::async_trait;
use command_group::AsyncCommandGroup;
use command_group::AsyncGroupChild;
use serde::Deserialize;
use serde::Serialize;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;
use tokio::process::Command;
use tracing::info;
use tracing::warn;

/// Result type alias for this crate.
type Result<T, E = JobError> = std::result::Result<T, E>;

// Tiger Style: All limits explicit and bounded
/// Maximum command name/path length in bytes.
const MAX_COMMAND_LENGTH: usize = 4096;
/// Maximum number of command arguments.
const MAX_ARGS_COUNT: usize = 100;
/// Maximum length of a single argument in bytes.
const MAX_ARG_LENGTH: usize = 4096;
/// Maximum number of environment variables.
const MAX_ENV_VARS: usize = 100;
/// Output size threshold for inline storage (64 KB).
pub const INLINE_OUTPUT_THRESHOLD: u64 = 64 * 1024;
/// Default maximum output capture size (10 MB).
const DEFAULT_MAX_OUTPUT_BYTES: u64 = 10 * 1024 * 1024;
/// Absolute maximum output capture size (100 MB).
const ABSOLUTE_MAX_OUTPUT_BYTES: u64 = 100 * 1024 * 1024;
/// Maximum grace period allowed.
const MAX_GRACE_PERIOD: Duration = Duration::from_secs(60);
/// Interval for polling process exit status.
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Job payload for shell command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellCommandPayload {
    /// Command to execute (name or path).
    pub command: String,
    /// Command arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables to set.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Working directory for command execution.
    pub working_dir: Option<PathBuf>,
    /// Whether to capture stderr separately (default: true).
    #[serde(default = "default_true")]
    pub should_capture_stderr: bool,
    /// Maximum output size in bytes before truncation.
    #[serde(default = "default_max_output")]
    pub max_output_bytes: u64,
    /// Grace period in seconds before SIGKILL.
    #[serde(default = "default_grace_period")]
    pub graceful_shutdown_secs: u64,
    /// Base64-encoded capability token (required).
    pub auth_token: Option<String>,
}

fn default_true() -> bool {
    true
}
fn default_max_output() -> u64 {
    DEFAULT_MAX_OUTPUT_BYTES
}
fn default_grace_period() -> u64 {
    5
}

/// Configuration for ShellCommandWorker.
pub struct ShellCommandWorkerConfig {
    /// Node ID for logging/metrics.
    pub node_id: u64,
    /// Token verifier for authorization (optional - if None, auth is skipped).
    pub token_verifier: Option<Arc<TokenVerifier>>,
    /// Optional blob store for large outputs.
    pub blob_store: Option<Arc<dyn BlobStore>>,
    /// Default working directory if not specified in payload.
    pub default_working_dir: PathBuf,
    /// Default environment variables to set for all commands.
    ///
    /// These are applied first, then job-specific env vars override them.
    /// Use this to inject PATH or other environment from the parent process
    /// (e.g., Nix paths). If not specified, a minimal safe environment is used.
    pub default_env: HashMap<String, String>,
}

impl std::fmt::Debug for ShellCommandWorkerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShellCommandWorkerConfig")
            .field("node_id", &self.node_id)
            .field("token_verifier", &self.token_verifier)
            .field("blob_store", &self.blob_store.is_some())
            .field("default_working_dir", &self.default_working_dir)
            .field("default_env_keys", &self.default_env.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Worker for executing shell commands as distributed jobs.
///
/// # Authorization
///
/// Every job must include a valid `auth_token` in the payload.
/// The token must contain a `ShellExecute` capability matching
/// the command and working directory.
///
/// # Process Management
///
/// Commands are spawned in new process groups for clean termination.
/// On timeout:
/// 1. SIGTERM is sent to the process group
/// 2. After grace period, SIGKILL is sent if still running
/// 3. Process is reaped to prevent zombies
pub struct ShellCommandWorker {
    config: ShellCommandWorkerConfig,
}

impl ShellCommandWorker {
    /// Create a new shell command worker with the given configuration.
    pub fn new(config: ShellCommandWorkerConfig) -> Self {
        Self { config }
    }

    /// Validate payload and check authorization.
    fn validate_and_authorize(&self, job: &Job) -> Result<ShellCommandPayload> {
        // Parse payload
        let payload: ShellCommandPayload =
            serde_json::from_value(job.spec.payload.clone()).map_err(|e| JobError::InvalidJobSpec {
                reason: format!("Invalid payload: {}", e),
            })?;

        // Tiger Style: validate all input bounds
        if payload.command.is_empty() || payload.command.len() > MAX_COMMAND_LENGTH {
            return Err(JobError::InvalidJobSpec {
                reason: format!("Command length must be 1-{} bytes, got {}", MAX_COMMAND_LENGTH, payload.command.len()),
            });
        }

        if payload.args.len() > MAX_ARGS_COUNT {
            return Err(JobError::InvalidJobSpec {
                reason: format!("Too many arguments: {} > {}", payload.args.len(), MAX_ARGS_COUNT),
            });
        }

        for (i, arg) in payload.args.iter().enumerate() {
            if arg.len() > MAX_ARG_LENGTH {
                return Err(JobError::InvalidJobSpec {
                    reason: format!("Argument {} too long: {} > {}", i, arg.len(), MAX_ARG_LENGTH),
                });
            }
        }

        if payload.env.len() > MAX_ENV_VARS {
            return Err(JobError::InvalidJobSpec {
                reason: format!("Too many environment variables: {} > {}", payload.env.len(), MAX_ENV_VARS),
            });
        }

        // Capability-based authorization (skip if no verifier configured)
        if let Some(ref verifier) = self.config.token_verifier {
            let token_str = payload.auth_token.as_ref().ok_or_else(|| JobError::InvalidJobSpec {
                reason: "Missing auth_token - shell execution requires capability token".into(),
            })?;

            let token = CapabilityToken::from_base64(token_str).map_err(|e| JobError::InvalidJobSpec {
                reason: format!("Invalid auth token: {}", e),
            })?;

            // Verify token signature and expiration
            // Use None for presenter since we're using bearer tokens
            verifier.verify(&token, None).map_err(|e| JobError::InvalidJobSpec {
                reason: format!("Token verification failed: {}", e),
            })?;

            // Check ShellExecute capability
            let working_dir = payload.working_dir.as_ref().map(|p| p.to_string_lossy().to_string());
            let authorized = token
                .capabilities
                .iter()
                .any(|cap| cap.authorizes_shell_command(&payload.command, working_dir.as_deref()));

            if !authorized {
                return Err(JobError::InvalidJobSpec {
                    reason: format!(
                        "Token lacks ShellExecute capability for command '{}' in '{}'",
                        payload.command,
                        working_dir.as_deref().unwrap_or("<default>")
                    ),
                });
            }
        }
        // If no verifier configured, skip auth (for local/dev use)

        Ok(payload)
    }

    /// Execute command and capture output.
    async fn execute_command(&self, payload: &ShellCommandPayload, timeout: Duration) -> Result<CommandOutput> {
        let working_dir = payload.working_dir.clone().unwrap_or_else(|| self.config.default_working_dir.clone());

        // Verify working directory exists
        if !working_dir.exists() {
            return Err(JobError::InvalidJobSpec {
                reason: format!("Working directory does not exist: {}", working_dir.display()),
            });
        }

        // Build command
        let mut cmd = Command::new(&payload.command);
        cmd.args(&payload.args)
            .current_dir(&working_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(if payload.should_capture_stderr {
                Stdio::piped()
            } else {
                Stdio::null()
            });

        // Set environment (don't inherit for security)
        cmd.env_clear();

        // Apply config defaults first (e.g., PATH from parent nix shell)
        for (key, value) in &self.config.default_env {
            cmd.env(key, value);
        }

        // Apply job-specific env vars (overrides config defaults)
        for (key, value) in &payload.env {
            cmd.env(key, value);
        }

        // Set safe fallback defaults only if not provided anywhere
        let has_path = self.config.default_env.contains_key("PATH") || payload.env.contains_key("PATH");
        let has_locale = self.config.default_env.contains_key("LC_ALL") || payload.env.contains_key("LC_ALL");
        if !has_path {
            cmd.env("PATH", "/usr/bin:/bin");
        }
        if !has_locale {
            cmd.env("LC_ALL", "C.UTF-8");
        }

        // Spawn in process group for clean termination
        let mut child = cmd.group_spawn().map_err(|e| JobError::ExecutionFailed {
            reason: format!("Failed to spawn command '{}': {}", payload.command, e),
        })?;

        let pid = child.inner().id().unwrap_or(0) as i32;
        info!(
            command = %payload.command,
            pid,
            working_dir = %working_dir.display(),
            "Spawned shell command"
        );

        // Take stdout/stderr handles
        let stdout = child.inner().stdout.take();
        let stderr = child.inner().stderr.take();

        let max_output = payload.max_output_bytes.min(ABSOLUTE_MAX_OUTPUT_BYTES);

        // Spawn output capture tasks
        let stdout_task = tokio::spawn(capture_output(stdout, max_output));
        let stderr_task = tokio::spawn(capture_output(stderr, max_output));

        // Wait with timeout
        let wait_result = tokio::time::timeout(timeout, child.wait()).await;

        let (exit_status, timed_out) = match wait_result {
            Ok(Ok(status)) => (Some(status), false),
            Ok(Err(e)) => {
                return Err(JobError::ExecutionFailed {
                    reason: format!("Wait failed: {}", e),
                });
            }
            Err(_) => {
                // Timeout - terminate process group
                warn!(pid, "Command timed out, terminating process group");
                let grace = Duration::from_secs(payload.graceful_shutdown_secs.min(MAX_GRACE_PERIOD.as_secs()));
                terminate_process_group(&mut child, grace).await?;
                (None, true)
            }
        };

        // Collect output
        let (stdout_data, stdout_total, stdout_truncated) =
            stdout_task.await.map_err(|e| JobError::ExecutionFailed {
                reason: format!("stdout capture task failed: {}", e),
            })??;
        let (stderr_data, stderr_total, stderr_truncated) =
            stderr_task.await.map_err(|e| JobError::ExecutionFailed {
                reason: format!("stderr capture task failed: {}", e),
            })??;

        // Handle timeout
        if timed_out {
            return Err(JobError::JobTimeout { timeout });
        }

        let status: std::process::ExitStatus = exit_status.unwrap();
        let exit_code = status.code().unwrap_or(-1);

        Ok(CommandOutput {
            exit_code,
            stdout: stdout_data,
            stderr: stderr_data,
            stdout_total_bytes: stdout_total,
            stderr_total_bytes: stderr_total,
            stdout_truncated,
            stderr_truncated,
        })
    }

    /// Store output, returning inline data or blob reference.
    async fn store_output(&self, data: Vec<u8>, job_id: &str, stream: &str) -> OutputRef {
        if data.len() as u64 <= INLINE_OUTPUT_THRESHOLD {
            return OutputRef::Inline(String::from_utf8_lossy(&data).to_string());
        }

        // Try to store in blob store
        if let Some(ref blob_store) = self.config.blob_store {
            match blob_store.add_bytes(&data).await {
                Ok(result) => {
                    info!(
                        job_id,
                        stream,
                        hash = %result.blob_ref.hash.to_hex(),
                        size = data.len(),
                        "Stored large output in blob store"
                    );
                    return OutputRef::Blob {
                        hash: result.blob_ref.hash.to_hex().to_string(),
                        size: data.len() as u64,
                    };
                }
                Err(e) => {
                    warn!(job_id, stream, error = ?e, "Failed to store output in blob store");
                }
            }
        }

        // Fallback: inline with truncation note
        let truncated_data = &data[..INLINE_OUTPUT_THRESHOLD as usize];
        let truncated_str = String::from_utf8_lossy(truncated_data).to_string();
        OutputRef::Inline(format!("{}...[truncated, full size: {} bytes]", truncated_str, data.len()))
    }
}

#[async_trait]
impl Worker for ShellCommandWorker {
    async fn execute(&self, job: Job) -> JobResult {
        let start = std::time::Instant::now();
        let job_id = job.id.to_string();

        // Validate and authorize
        let payload = match self.validate_and_authorize(&job) {
            Ok(p) => p,
            Err(e) => {
                return JobResult::Failure(JobFailure {
                    reason: format!("{}", e),
                    is_retryable: false,
                    error_code: Some("AUTH_FAILED".into()),
                });
            }
        };

        info!(
            job_id = %job_id,
            command = %payload.command,
            args_count = payload.args.len(),
            "Executing shell command"
        );

        let timeout = job.spec.config.timeout.unwrap_or(Duration::from_secs(300));

        // Execute
        let output = match self.execute_command(&payload, timeout).await {
            Ok(o) => o,
            Err(e) => {
                return JobResult::Failure(JobFailure {
                    reason: format!("{}", e),
                    is_retryable: matches!(e, JobError::JobTimeout { .. }),
                    error_code: Some(error_code_from_error(&e)),
                });
            }
        };

        // Check exit code
        if output.exit_code != 0 {
            // Take from the END of stderr since errors are usually at the end
            // (e.g., nix outputs config messages at the start, actual error at the end)
            const STDERR_PREVIEW_BYTES: usize = 4096;
            let stderr_len = output.stderr.len();
            let start_offset = stderr_len.saturating_sub(STDERR_PREVIEW_BYTES);
            let stderr_preview = String::from_utf8_lossy(&output.stderr[start_offset..]).to_string();

            return JobResult::Failure(JobFailure {
                reason: format!("Command exited with code {}: {}", output.exit_code, stderr_preview),
                // Exit codes 126-128 are typically permanent failures
                is_retryable: !matches!(output.exit_code, 126..=128),
                error_code: Some(format!("EXIT_{}", output.exit_code)),
            });
        }

        // Store outputs
        let stdout_ref = self.store_output(output.stdout, &job_id, "stdout").await;
        let stderr_ref = self.store_output(output.stderr, &job_id, "stderr").await;

        let duration_ms = start.elapsed().as_millis() as u64;

        info!(
            job_id = %job_id,
            exit_code = 0,
            duration_ms,
            stdout_bytes = output.stdout_total_bytes,
            stderr_bytes = output.stderr_total_bytes,
            "Shell command completed successfully"
        );

        JobResult::success(serde_json::json!({
            "exit_code": 0,
            "stdout": stdout_ref,
            "stderr": stderr_ref,
            "stdout_total_bytes": output.stdout_total_bytes,
            "stderr_total_bytes": output.stderr_total_bytes,
            "stdout_truncated": output.stdout_truncated,
            "stderr_truncated": output.stderr_truncated,
            "duration_ms": duration_ms,
        }))
    }

    fn job_types(&self) -> Vec<String> {
        vec!["shell_command".to_string()]
    }
}

// Helper types

#[derive(Debug)]
struct CommandOutput {
    exit_code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    stdout_total_bytes: u64,
    stderr_total_bytes: u64,
    stdout_truncated: bool,
    stderr_truncated: bool,
}

/// Reference to command output, either inline or in blob store.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OutputRef {
    /// Output stored inline as string.
    Inline(String),
    /// Output stored in blob store.
    Blob {
        /// BLAKE3 hash of the blob.
        hash: String,
        /// Size in bytes.
        size: u64,
    },
}

/// Capture output from a reader with size limits.
async fn capture_output<R: AsyncRead + Unpin + Send + 'static>(
    reader: Option<R>,
    max_bytes: u64,
) -> Result<(Vec<u8>, u64, bool)> {
    let Some(reader) = reader else {
        return Ok((vec![], 0, false));
    };

    let mut buf_reader = BufReader::new(reader);
    let mut captured = Vec::with_capacity(max_bytes.min(64 * 1024) as usize);
    let mut total: u64 = 0;
    let mut truncated = false;
    let mut chunk = vec![0u8; 8192];

    loop {
        let n = buf_reader.read(&mut chunk).await.map_err(|e| JobError::ExecutionFailed {
            reason: format!("Output read error: {}", e),
        })?;
        if n == 0 {
            break;
        }

        total += n as u64;

        if (captured.len() as u64) < max_bytes {
            let remaining = (max_bytes - captured.len() as u64) as usize;
            captured.extend_from_slice(&chunk[..n.min(remaining)]);
            if n > remaining {
                truncated = true;
            }
        } else {
            truncated = true;
        }
    }

    Ok((captured, total, truncated))
}

/// Gracefully terminate a process group.
///
/// 1. Send SIGTERM to process group
/// 2. Wait for grace period
/// 3. Send SIGKILL if still running
/// 4. Reap the process
#[cfg(unix)]
async fn terminate_process_group(child: &mut AsyncGroupChild, grace: Duration) -> Result<()> {
    use nix::sys::signal::Signal;
    use nix::sys::signal::{self};
    use nix::unistd::Pid;

    let Some(pid) = child.inner().id() else {
        return Ok(()); // Already exited
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
            return Ok(());
        }
        tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    }

    // Force kill
    warn!(pid, "Process did not exit gracefully, sending SIGKILL");
    let _ = signal::kill(pgid, Signal::SIGKILL);
    let _ = child.wait().await;

    Ok(())
}

#[cfg(not(unix))]
async fn terminate_process_group(child: &mut AsyncGroupChild, _grace: Duration) -> Result<()> {
    // On non-Unix, just kill directly
    let _ = child.kill();
    let _ = child.wait().await;
    Ok(())
}

fn error_code_from_error(e: &JobError) -> String {
    match e {
        JobError::JobTimeout { .. } => "TIMEOUT".into(),
        JobError::InvalidJobSpec { .. } => "INVALID_SPEC".into(),
        JobError::ExecutionFailed { .. } => "EXEC_FAILED".into(),
        _ => "UNKNOWN".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_defaults() {
        let json = r#"{"command": "echo", "args": ["hello"]}"#;
        let payload: ShellCommandPayload = serde_json::from_str(json).unwrap();

        assert_eq!(payload.command, "echo");
        assert_eq!(payload.args, vec!["hello"]);
        assert!(payload.should_capture_stderr);
        assert_eq!(payload.max_output_bytes, DEFAULT_MAX_OUTPUT_BYTES);
        assert_eq!(payload.graceful_shutdown_secs, 5);
        assert!(payload.auth_token.is_none());
    }

    #[test]
    fn test_output_ref_serialization() {
        let inline = OutputRef::Inline("hello".into());
        let json = serde_json::to_string(&inline).unwrap();
        assert_eq!(json, r#""hello""#);

        let blob = OutputRef::Blob {
            hash: "abc123".into(),
            size: 1024,
        };
        let json = serde_json::to_string(&blob).unwrap();
        assert!(json.contains("abc123"));
        assert!(json.contains("1024"));
    }
}
