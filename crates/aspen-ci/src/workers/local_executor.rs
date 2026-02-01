//! LocalExecutorWorker - Worker implementation for direct command execution.
//!
//! This module provides a Worker that executes CI jobs directly on the host
//! without nested VMs. It reuses the Executor from aspen-ci-agent for process
//! management, streaming output, and timeout handling.
//!
//! Key differences from CloudHypervisorWorker:
//! - No VM pool management or vsock communication
//! - Jobs run in per-job temporary directories under a configurable workspace
//! - Uses process groups for isolation (no hardware VM isolation)
//! - Much faster job startup (no VM boot/acquire)
//!
//! This is suitable for running CI within an already-isolated environment
//! (e.g., a dogfood VM), where nested VM isolation is unnecessary.

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use aspen_blob::BlobStore;
use aspen_ci_agent::executor::Executor;
use aspen_ci_agent::protocol::ExecutionRequest;
use aspen_ci_agent::protocol::ExecutionResult;
use aspen_ci_agent::protocol::LogMessage;
use aspen_constants::CI_VM_DEFAULT_EXECUTION_TIMEOUT_MS;
use aspen_constants::CI_VM_MAX_EXECUTION_TIMEOUT_MS;
use aspen_jobs::Job;
use aspen_jobs::JobError;
use aspen_jobs::JobOutput;
use aspen_jobs::JobResult;
use aspen_jobs::Worker;
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::cloud_hypervisor::artifacts::ArtifactCollectionResult;
use super::cloud_hypervisor::artifacts::ArtifactUploadResult;
use super::cloud_hypervisor::artifacts::collect_artifacts;
use super::cloud_hypervisor::artifacts::upload_artifacts_to_blob_store;
use super::cloud_hypervisor::workspace::seed_workspace_from_blob;

/// Maximum command length.
const MAX_COMMAND_LENGTH: usize = 4096;

/// Maximum argument length.
const MAX_ARG_LENGTH: usize = 4096;
/// Maximum total arguments count.
const MAX_ARGS_COUNT: usize = 256;
/// Maximum environment variable count.
const MAX_ENV_COUNT: usize = 256;
/// Maximum artifact glob patterns.
const MAX_ARTIFACTS: usize = 64;
/// Inline log threshold (256 KB).
/// This is the maximum size of stdout/stderr we keep for inline display.
/// We keep the TAIL of output to preserve error messages that typically appear at the end.
const INLINE_LOG_THRESHOLD: usize = 256 * 1024;

/// Marker prepended when output is truncated.
const TRUNCATION_MARKER: &str = "...[truncated - showing last 256 KB of output]...\n";

/// Job payload for local executor.
///
/// Compatible with CloudHypervisorPayload for easy migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalExecutorPayload {
    /// CI job name for status tracking.
    #[serde(default)]
    pub job_name: Option<String>,

    /// Command to execute.
    pub command: String,

    /// Command arguments.
    #[serde(default)]
    pub args: Vec<String>,

    /// Working directory relative to workspace.
    #[serde(default = "default_working_dir")]
    pub working_dir: String,

    /// Environment variables to set.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Execution timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Glob patterns for artifacts to collect.
    #[serde(default)]
    pub artifacts: Vec<String>,

    /// Source hash for workspace setup (blob store key).
    #[serde(default)]
    pub source_hash: Option<String>,

    /// Checkout directory on the host to copy into workspace.
    #[serde(default)]
    pub checkout_dir: Option<String>,

    /// Flake attribute to prefetch for nix commands.
    #[serde(default)]
    pub flake_attr: Option<String>,
}

fn default_working_dir() -> String {
    ".".to_string()
}

fn default_timeout() -> u64 {
    CI_VM_DEFAULT_EXECUTION_TIMEOUT_MS / 1000
}

impl LocalExecutorPayload {
    /// Validate the payload.
    pub fn validate(&self) -> Result<(), String> {
        if self.command.is_empty() {
            return Err("command cannot be empty".to_string());
        }

        if self.command.len() > MAX_COMMAND_LENGTH {
            return Err(format!("command too long: {} bytes (max: {})", self.command.len(), MAX_COMMAND_LENGTH));
        }

        if self.args.len() > MAX_ARGS_COUNT {
            return Err(format!("too many arguments: {} (max: {})", self.args.len(), MAX_ARGS_COUNT));
        }

        for (i, arg) in self.args.iter().enumerate() {
            if arg.len() > MAX_ARG_LENGTH {
                return Err(format!("argument {} too long: {} bytes (max: {})", i, arg.len(), MAX_ARG_LENGTH));
            }
        }

        if self.env.len() > MAX_ENV_COUNT {
            return Err(format!("too many environment variables: {} (max: {})", self.env.len(), MAX_ENV_COUNT));
        }

        let max_timeout = CI_VM_MAX_EXECUTION_TIMEOUT_MS / 1000;
        if self.timeout_secs > max_timeout {
            return Err(format!("timeout too long: {} seconds (max: {})", self.timeout_secs, max_timeout));
        }

        if self.artifacts.len() > MAX_ARTIFACTS {
            return Err(format!("too many artifact patterns: {} (max: {})", self.artifacts.len(), MAX_ARTIFACTS));
        }

        Ok(())
    }
}

/// Configuration for LocalExecutorWorker.
#[derive(Debug, Clone)]
pub struct LocalExecutorWorkerConfig {
    /// Base workspace directory where jobs run.
    /// Each job gets a subdirectory under this path.
    pub workspace_dir: PathBuf,

    /// Whether to clean up job workspaces after completion.
    pub cleanup_workspaces: bool,
}

impl Default for LocalExecutorWorkerConfig {
    fn default() -> Self {
        Self {
            workspace_dir: PathBuf::from("/workspace"),
            cleanup_workspaces: true,
        }
    }
}

/// Local executor worker.
///
/// Executes CI jobs directly using process execution, without nested VMs.
/// Suitable for environments that are already isolated (e.g., running inside
/// a dogfood VM).
pub struct LocalExecutorWorker {
    /// Configuration.
    config: LocalExecutorWorkerConfig,

    /// Command executor (reused from aspen-ci-agent).
    executor: Executor,

    /// Optional blob store for workspace seeding and artifact storage.
    blob_store: Option<Arc<dyn BlobStore>>,
}

impl LocalExecutorWorker {
    /// Create a new local executor worker.
    pub fn new(config: LocalExecutorWorkerConfig) -> Self {
        Self {
            config,
            executor: Executor::new(),
            blob_store: None,
        }
    }

    /// Create a new local executor worker with a blob store.
    pub fn with_blob_store(config: LocalExecutorWorkerConfig, blob_store: Arc<dyn BlobStore>) -> Self {
        Self {
            config,
            executor: Executor::new(),
            blob_store: Some(blob_store),
        }
    }

    /// Execute a job and collect artifacts.
    ///
    /// This is the main orchestration function that coordinates:
    /// 1. Workspace setup (directory creation, checkout copying, blob seeding)
    /// 2. Command execution with log streaming
    /// 3. Artifact collection and upload
    /// 4. Workspace cleanup
    async fn execute_job(
        &self,
        job: &Job,
        payload: &LocalExecutorPayload,
    ) -> Result<(ExecutionResult, ArtifactCollectionResult, Option<ArtifactUploadResult>), String> {
        let job_id = job.id.to_string();

        // Phase 1: Set up workspace
        let job_workspace = self
            .setup_job_workspace(&job_id, payload)
            .await
            .map_err(|e| format!("workspace setup failed: {}", e))?;

        // Phase 2: Build and execute request
        let request = self.build_execution_request(&job_id, payload, &job_workspace);
        let result = self.execute_with_streaming(&job_id, request, payload).await?;

        info!(
            job_id = %job_id,
            exit_code = result.exit_code,
            duration_ms = result.duration_ms,
            "job completed"
        );

        // Phase 3: Collect artifacts (only on success)
        let (artifacts, upload_result) =
            self.collect_and_upload_artifacts(&job_id, &result, payload, &job_workspace).await;

        // Phase 4: Clean up workspace
        self.cleanup_workspace(&job_id, &job_workspace).await;

        Ok((result, artifacts, upload_result))
    }

    /// Set up the job workspace directory.
    ///
    /// Creates a per-job directory, copies checkout contents if provided,
    /// pre-fetches flake inputs for nix commands, and seeds from blob store.
    async fn setup_job_workspace(&self, job_id: &str, payload: &LocalExecutorPayload) -> Result<PathBuf, String> {
        let job_workspace = self.config.workspace_dir.join(job_id);

        // Create workspace directory
        tokio::fs::create_dir_all(&job_workspace)
            .await
            .map_err(|e| format!("failed to create job workspace: {}", e))?;

        info!(job_id = %job_id, workspace = %job_workspace.display(), "created job workspace");

        // Copy checkout directory if provided
        if let Some(ref checkout_dir) = payload.checkout_dir {
            self.copy_checkout_to_workspace(job_id, checkout_dir, &job_workspace, payload).await;
        }

        // Seed from blob store if source_hash provided
        if let Some(ref source_hash) = payload.source_hash {
            self.seed_workspace_from_source(job_id, source_hash, &job_workspace).await;
        }

        Ok(job_workspace)
    }

    /// Copy checkout directory contents to workspace and pre-fetch flake inputs.
    async fn copy_checkout_to_workspace(
        &self,
        job_id: &str,
        checkout_dir: &str,
        job_workspace: &std::path::Path,
        payload: &LocalExecutorPayload,
    ) {
        let checkout_path = PathBuf::from(checkout_dir);
        if !checkout_path.exists() {
            return;
        }

        match copy_directory_contents(&checkout_path, job_workspace).await {
            Ok(count) => {
                info!(
                    job_id = %job_id,
                    checkout_dir = %checkout_dir,
                    files_copied = count,
                    workspace = %job_workspace.display(),
                    "checkout copied to workspace"
                );
            }
            Err(e) => {
                warn!(job_id = %job_id, checkout_dir = %checkout_dir, error = ?e, "failed to copy checkout");
                return;
            }
        }

        // Pre-fetch flake inputs for nix commands
        if payload.command == "nix" && job_workspace.join("flake.nix").exists() {
            match prefetch_and_rewrite_flake_lock(job_workspace).await {
                Ok(()) => info!(job_id = %job_id, "pre-fetched flake inputs"),
                Err(e) => warn!(job_id = %job_id, error = ?e, "failed to pre-fetch flake"),
            }
        }
    }

    /// Seed workspace from blob store if available.
    async fn seed_workspace_from_source(&self, job_id: &str, source_hash: &str, job_workspace: &std::path::Path) {
        let Some(ref blob_store) = self.blob_store else {
            return;
        };

        match seed_workspace_from_blob(blob_store, source_hash, job_workspace).await {
            Ok(bytes) => {
                info!(job_id = %job_id, source_hash = %source_hash, bytes = bytes, "workspace seeded");
            }
            Err(e) => {
                warn!(job_id = %job_id, source_hash = %source_hash, error = ?e, "workspace seeding failed");
            }
        }
    }

    /// Build an execution request from the payload.
    fn build_execution_request(
        &self,
        job_id: &str,
        payload: &LocalExecutorPayload,
        job_workspace: &std::path::Path,
    ) -> ExecutionRequest {
        let working_dir = if payload.working_dir.starts_with('/') {
            PathBuf::from(&payload.working_dir)
        } else {
            job_workspace.join(&payload.working_dir)
        };

        let (command, args) = if payload.command == "nix" {
            inject_nix_flags(&payload.args)
        } else {
            (payload.command.clone(), payload.args.clone())
        };

        let mut env = payload.env.clone();
        env.entry("HOME".to_string()).or_insert_with(|| "/tmp".to_string());

        ExecutionRequest {
            id: job_id.to_string(),
            command,
            args,
            working_dir,
            env,
            timeout_secs: payload.timeout_secs,
        }
    }

    /// Execute a request with log streaming and return the result.
    async fn execute_with_streaming(
        &self,
        job_id: &str,
        request: ExecutionRequest,
        payload: &LocalExecutorPayload,
    ) -> Result<ExecutionResult, String> {
        let (log_tx, log_rx) = mpsc::channel::<LogMessage>(1024);

        info!(
            job_id = %job_id,
            command = %payload.command,
            args = ?payload.args,
            timeout_secs = payload.timeout_secs,
            "executing job"
        );

        let log_consumer = spawn_log_consumer(job_id.to_string(), log_rx);
        let exec_result = self.executor.execute(request, log_tx).await;

        // Log consumer task is non-critical; if it panics, empty logs are acceptable
        let (collected_stdout, collected_stderr) = log_consumer.await.unwrap_or_default();

        match exec_result {
            Ok(mut result) => {
                if result.stdout.is_empty() {
                    result.stdout = collected_stdout;
                }
                if result.stderr.is_empty() {
                    result.stderr = collected_stderr;
                }
                Ok(result)
            }
            Err(e) => Err(format!("execution failed: {}", e)),
        }
    }

    /// Collect and upload artifacts if the job succeeded.
    async fn collect_and_upload_artifacts(
        &self,
        job_id: &str,
        result: &ExecutionResult,
        payload: &LocalExecutorPayload,
        job_workspace: &std::path::Path,
    ) -> (ArtifactCollectionResult, Option<ArtifactUploadResult>) {
        if result.exit_code != 0 || result.error.is_some() || payload.artifacts.is_empty() {
            return (ArtifactCollectionResult::default(), None);
        }

        match collect_artifacts(job_workspace, &payload.artifacts).await {
            Ok(collected) => {
                let upload = if let Some(ref blob_store) = self.blob_store {
                    if !collected.artifacts.is_empty() {
                        Some(upload_artifacts_to_blob_store(&collected, blob_store, job_id).await)
                    } else {
                        None
                    }
                } else {
                    None
                };
                (collected, upload)
            }
            Err(e) => {
                warn!(job_id = %job_id, error = ?e, "artifact collection failed");
                (ArtifactCollectionResult::default(), None)
            }
        }
    }

    /// Clean up the job workspace if configured.
    async fn cleanup_workspace(&self, job_id: &str, job_workspace: &std::path::Path) {
        if !self.config.cleanup_workspaces {
            return;
        }

        if let Err(e) = tokio::fs::remove_dir_all(job_workspace).await {
            warn!(
                job_id = %job_id,
                workspace = %job_workspace.display(),
                error = ?e,
                "failed to clean up job workspace"
            );
        }
    }
}

/// Spawn a task to consume log messages and accumulate stdout/stderr.
///
/// This keeps the TAIL of output when it exceeds INLINE_LOG_THRESHOLD,
/// because error messages typically appear at the end of build output.
fn spawn_log_consumer(
    job_id: String,
    mut log_rx: mpsc::Receiver<LogMessage>,
) -> tokio::task::JoinHandle<(String, String)> {
    tokio::spawn(async move {
        let mut stdout = String::new();
        let mut stderr = String::new();
        let mut stdout_truncated = false;
        let mut stderr_truncated = false;

        while let Some(msg) = log_rx.recv().await {
            match msg {
                LogMessage::Stdout(data) => {
                    stdout.push_str(&data);
                    // Keep the tail when exceeding threshold
                    if stdout.len() > INLINE_LOG_THRESHOLD {
                        let start = stdout.len() - INLINE_LOG_THRESHOLD;
                        stdout = stdout[start..].to_string();
                        stdout_truncated = true;
                    }
                    debug!(job_id = %job_id, len = data.len(), "stdout chunk");
                }
                LogMessage::Stderr(data) => {
                    stderr.push_str(&data);
                    // Keep the tail when exceeding threshold
                    if stderr.len() > INLINE_LOG_THRESHOLD {
                        let start = stderr.len() - INLINE_LOG_THRESHOLD;
                        stderr = stderr[start..].to_string();
                        stderr_truncated = true;
                    }
                    debug!(job_id = %job_id, len = data.len(), "stderr chunk");
                }
                LogMessage::Heartbeat { elapsed_secs } => {
                    debug!(job_id = %job_id, elapsed_secs, "heartbeat");
                }
                LogMessage::Complete(_) => {
                    // Final result handled separately
                }
            }
        }

        // Prepend truncation marker if output was truncated
        if stdout_truncated {
            stdout = format!("{}{}", TRUNCATION_MARKER, stdout);
        }
        if stderr_truncated {
            stderr = format!("{}{}", TRUNCATION_MARKER, stderr);
        }

        (stdout, stderr)
    })
}

#[async_trait]
impl Worker for LocalExecutorWorker {
    async fn execute(&self, job: Job) -> JobResult {
        let job_id = job.id.to_string();
        info!(job_id = %job_id, job_type = %job.spec.job_type, "executing local job");

        // Parse payload
        let payload: LocalExecutorPayload = match serde_json::from_value(job.spec.payload.clone()) {
            Ok(p) => p,
            Err(e) => {
                error!(job_id = %job_id, error = ?e, "failed to parse job payload");
                return JobResult::failure(format!("invalid job payload: {}", e));
            }
        };

        // Validate payload
        if let Err(e) = payload.validate() {
            error!(job_id = %job_id, error = %e, "invalid job payload");
            return JobResult::failure(format!("invalid job payload: {}", e));
        }

        // Execute job
        match self.execute_job(&job, &payload).await {
            Ok((result, artifacts, upload_result)) => {
                if result.exit_code == 0 && result.error.is_none() {
                    // Build artifact list for output
                    let artifact_list: Vec<_> = if let Some(ref upload) = upload_result {
                        upload
                            .uploaded
                            .iter()
                            .map(|a| {
                                serde_json::json!({
                                    "path": a.relative_path.display().to_string(),
                                    "size": a.blob_ref.size,
                                    "blob_hash": a.blob_ref.hash.to_string(),
                                })
                            })
                            .collect()
                    } else {
                        artifacts
                            .artifacts
                            .iter()
                            .map(|a| {
                                serde_json::json!({
                                    "path": a.relative_path.display().to_string(),
                                    "size": a.size,
                                })
                            })
                            .collect()
                    };

                    let upload_stats = upload_result.as_ref().map(|u| {
                        serde_json::json!({
                            "uploaded_count": u.uploaded.len(),
                            "failed_count": u.failed.len(),
                            "total_bytes": u.total_bytes,
                        })
                    });

                    let output = JobOutput {
                        data: serde_json::json!({
                            "exit_code": result.exit_code,
                            "stdout": result.stdout,
                            "stderr": result.stderr,
                            "duration_ms": result.duration_ms,
                            "artifacts": artifact_list,
                            "artifacts_total_size": artifacts.total_size,
                            "artifacts_skipped": artifacts.skipped_files.len(),
                            "artifacts_unmatched_patterns": artifacts.unmatched_patterns,
                            "artifacts_upload": upload_stats,
                        }),
                        metadata: HashMap::from([
                            ("local_execution".to_string(), "true".to_string()),
                            ("duration_ms".to_string(), result.duration_ms.to_string()),
                            ("artifacts_count".to_string(), artifacts.artifacts.len().to_string()),
                            ("artifacts_total_size".to_string(), artifacts.total_size.to_string()),
                        ]),
                    };
                    JobResult::Success(output)
                } else {
                    // Show last 16 KB of stderr and 4 KB of stdout for error diagnosis
                    // We take from the end since error messages typically appear at the end
                    let stderr_len = result.stderr.len();
                    let stdout_len = result.stdout.len();
                    let stderr_preview: String = if stderr_len > 16384 {
                        format!(
                            "...[{} bytes truncated]...\n{}",
                            stderr_len - 16384,
                            result.stderr.chars().skip(stderr_len.saturating_sub(16384)).collect::<String>()
                        )
                    } else {
                        result.stderr.clone()
                    };
                    let stdout_preview: String = if stdout_len > 4096 {
                        format!(
                            "...[{} bytes truncated]...\n{}",
                            stdout_len - 4096,
                            result.stdout.chars().skip(stdout_len.saturating_sub(4096)).collect::<String>()
                        )
                    } else {
                        result.stdout.clone()
                    };

                    let reason = if let Some(err) = result.error {
                        format!("{}\n\nstderr:\n{}\n\nstdout:\n{}", err, stderr_preview, stdout_preview)
                    } else {
                        format!(
                            "command exited with code {}\n\nstderr:\n{}\n\nstdout:\n{}",
                            result.exit_code, stderr_preview, stdout_preview
                        )
                    };
                    JobResult::failure(reason)
                }
            }
            Err(e) => {
                error!(job_id = %job_id, error = %e, "job execution failed");
                JobResult::failure(format!("execution failed: {}", e))
            }
        }
    }

    async fn on_start(&self) -> Result<(), JobError> {
        // Ensure workspace directory exists
        if let Err(e) = tokio::fs::create_dir_all(&self.config.workspace_dir).await {
            return Err(JobError::WorkerRegistrationFailed {
                reason: format!("failed to create workspace directory: {}", e),
            });
        }

        info!(
            workspace = %self.config.workspace_dir.display(),
            "local executor worker initialized"
        );
        Ok(())
    }

    async fn on_shutdown(&self) -> Result<(), JobError> {
        info!("local executor worker shutdown");
        Ok(())
    }

    fn job_types(&self) -> Vec<String> {
        // Handle all CI job types for local execution
        // - shell_command: Shell jobs from ci.ncl
        // - ci_nix_build: Nix build jobs from ci.ncl
        // - ci_vm: VM isolation jobs (CloudHypervisorWorker compatibility)
        // - cloud_hypervisor: Direct CloudHypervisorWorker jobs
        // - local_executor: Jobs explicitly targeting local execution
        vec![
            "shell_command".to_string(),
            "ci_nix_build".to_string(),
            "ci_vm".to_string(),
            "cloud_hypervisor".to_string(),
            "local_executor".to_string(),
        ]
    }
}

/// Inject nix flags for offline execution.
fn inject_nix_flags(args: &[String]) -> (String, Vec<String>) {
    let mut nix_args = args.to_vec();

    if !nix_args.is_empty() {
        let mut insert_pos = 1;

        if !nix_args.iter().any(|a| a == "--offline") {
            nix_args.insert(insert_pos, "--offline".to_string());
            insert_pos += 1;
        }

        if !nix_args.iter().any(|a| a.contains("experimental-features")) {
            nix_args.insert(insert_pos, "--extra-experimental-features".to_string());
            insert_pos += 1;
            nix_args.insert(insert_pos, "nix-command flakes".to_string());
            insert_pos += 1;
        }

        if !nix_args.iter().any(|a| a == "--accept-flake-config") {
            nix_args.insert(insert_pos, "--accept-flake-config".to_string());
            insert_pos += 1;
        }

        if !nix_args.iter().any(|a| a == "--no-write-lock-file") {
            nix_args.insert(insert_pos, "--no-write-lock-file".to_string());
        }
    }

    ("nix".to_string(), nix_args)
}

/// Copy contents of a directory to another directory.
async fn copy_directory_contents(src: &std::path::Path, dst: &std::path::Path) -> io::Result<usize> {
    use tokio::fs;

    fs::create_dir_all(dst).await?;

    let mut count = 0;
    let mut entries = fs::read_dir(src).await?;

    while let Some(entry) = entries.next_entry().await? {
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        let file_type = entry.file_type().await?;

        if file_type.is_dir() {
            count += Box::pin(copy_directory_contents(&src_path, &dst_path)).await?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path).await?;
            count += 1;
        } else if file_type.is_symlink() {
            let target = fs::read_link(&src_path).await?;
            let _ = fs::remove_file(&dst_path).await;
            #[cfg(unix)]
            {
                tokio::fs::symlink(&target, &dst_path).await?;
            }
            count += 1;
        }
    }

    Ok(count)
}

/// Pre-fetch flake inputs and rewrite flake.lock for offline evaluation.
async fn prefetch_and_rewrite_flake_lock(workspace: &std::path::Path) -> io::Result<()> {
    use std::process::Stdio;

    use tokio::process::Command;

    let archive_output = Command::new("nix")
        .args([
            "flake",
            "archive",
            "--json",
            "--no-write-lock-file",
            "--accept-flake-config",
        ])
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !archive_output.status.success() {
        let stderr = String::from_utf8_lossy(&archive_output.stderr);
        return Err(io::Error::other(format!(
            "nix flake archive failed: {}",
            stderr.chars().take(500).collect::<String>()
        )));
    }

    let stdout = String::from_utf8(archive_output.stdout)
        .map_err(|e| io::Error::other(format!("invalid UTF-8 in archive output: {e}")))?;

    let archive_json: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| io::Error::other(format!("failed to parse archive JSON: {e}")))?;

    let mut input_paths = HashMap::new();
    extract_archive_paths(&archive_json, &mut input_paths);

    rewrite_flake_lock_for_offline(workspace, &input_paths)?;

    // Sync to ensure virtiofsd sees the changes
    let _ = Command::new("sync").output().await;

    Ok(())
}

/// Extract input name -> store path mappings from archive JSON output.
fn extract_archive_paths(json: &serde_json::Value, paths: &mut HashMap<String, PathBuf>) {
    if let Some(inputs) = json.get("inputs").and_then(|v| v.as_object()) {
        for (name, value) in inputs {
            if let Some(path) = value.get("path").and_then(|v| v.as_str()) {
                paths.insert(name.clone(), PathBuf::from(path));
            }
            extract_archive_paths(value, paths);
        }
    }
}

/// Rewrite flake.lock to use path: URLs for offline evaluation.
fn rewrite_flake_lock_for_offline(
    workspace: &std::path::Path,
    input_paths: &HashMap<String, PathBuf>,
) -> io::Result<()> {
    let lock_path = workspace.join("flake.lock");
    let lock_content = std::fs::read_to_string(&lock_path)?;
    let mut lock: serde_json::Value = serde_json::from_str(&lock_content)
        .map_err(|e| io::Error::other(format!("failed to parse flake.lock: {e}")))?;

    if let Some(nodes) = lock.get_mut("nodes").and_then(|v| v.as_object_mut()) {
        for (node_name, node_value) in nodes.iter_mut() {
            if node_name == "root" {
                continue;
            }

            if let Some(store_path) = input_paths.get(node_name) {
                rewrite_locked_node_to_path(node_value, store_path);
            }
        }
    }

    let modified_lock = serde_json::to_string_pretty(&lock)
        .map_err(|e| io::Error::other(format!("failed to serialize flake.lock: {e}")))?;
    let temp_path = lock_path.with_extension("lock.tmp");
    std::fs::write(&temp_path, &modified_lock)?;
    std::fs::rename(&temp_path, &lock_path)?;

    Ok(())
}

/// Rewrite a single locked node to use a path: URL.
fn rewrite_locked_node_to_path(node: &mut serde_json::Value, store_path: &std::path::Path) {
    if let Some(locked) = node.get_mut("locked").and_then(|v| v.as_object_mut()) {
        locked.insert("type".to_string(), serde_json::json!("path"));
        locked.insert("path".to_string(), serde_json::json!(store_path.display().to_string()));
        locked.remove("owner");
        locked.remove("repo");
        locked.remove("url");
        locked.remove("ref");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_validation() {
        let payload = LocalExecutorPayload {
            job_name: Some("test".to_string()),
            command: "nix".to_string(),
            args: vec!["build".to_string()],
            working_dir: ".".to_string(),
            env: HashMap::new(),
            timeout_secs: 3600,
            artifacts: vec![],
            source_hash: None,
            checkout_dir: None,
            flake_attr: None,
        };
        assert!(payload.validate().is_ok());

        // Empty command
        let invalid = LocalExecutorPayload {
            command: "".to_string(),
            ..payload.clone()
        };
        assert!(invalid.validate().is_err());

        // Command too long
        let invalid = LocalExecutorPayload {
            command: "x".repeat(MAX_COMMAND_LENGTH + 1),
            ..payload.clone()
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_inject_nix_flags() {
        let (cmd, args) = inject_nix_flags(&["build".to_string(), "-L".to_string(), ".#default".to_string()]);

        assert_eq!(cmd, "nix");
        assert!(args.contains(&"--offline".to_string()));
        assert!(args.contains(&"--accept-flake-config".to_string()));
        assert!(args.contains(&"--no-write-lock-file".to_string()));
    }

    #[test]
    fn test_worker_job_types() {
        let config = LocalExecutorWorkerConfig::default();
        let worker = LocalExecutorWorker::new(config);

        let types = worker.job_types();
        assert!(types.contains(&"shell_command".to_string()));
        assert!(types.contains(&"ci_nix_build".to_string()));
        assert!(types.contains(&"ci_vm".to_string()));
        assert!(types.contains(&"local_executor".to_string()));
    }
}
