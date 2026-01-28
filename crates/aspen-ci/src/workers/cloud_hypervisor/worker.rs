//! CloudHypervisorWorker - Worker implementation for Cloud Hypervisor VMs.
//!
//! This module implements the Worker trait for executing CI jobs inside
//! Cloud Hypervisor microVMs. Jobs are executed by:
//!
//! 1. Acquiring a warm VM from the pool
//! 2. Setting up the workspace via virtiofs
//! 3. Sending ExecutionRequest to guest agent via vsock
//! 4. Streaming logs back to the CI system
//! 5. Collecting artifacts and releasing the VM

use std::collections::HashMap;
use std::io::{self, ErrorKind};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use aspen_blob::BlobStore;
use aspen_constants::{CI_VM_DEFAULT_EXECUTION_TIMEOUT_MS, CI_VM_MAX_EXECUTION_TIMEOUT_MS, CI_VM_VSOCK_PORT};
use aspen_jobs::{Job, JobError, JobOutput, JobResult, Worker};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::{debug, error, info, warn};

use super::artifacts::{
    ArtifactCollectionResult, ArtifactUploadResult, collect_artifacts, upload_artifacts_to_blob_store,
};
use super::config::CloudHypervisorWorkerConfig;
use super::error::{CloudHypervisorError, Result};
use super::pool::VmPool;
use super::vm::SharedVm;
use super::workspace::seed_workspace_from_blob;

// Re-use protocol types from aspen-ci-agent
use aspen_ci_agent::protocol::{AgentMessage, ExecutionRequest, ExecutionResult, HostMessage, MAX_MESSAGE_SIZE};

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
/// Inline log threshold (64 KB).
const INLINE_LOG_THRESHOLD: usize = 64 * 1024;

/// Job payload for Cloud Hypervisor VM execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudHypervisorPayload {
    /// CI job name for status tracking.
    #[serde(default)]
    pub job_name: Option<String>,

    /// Command to execute in the VM.
    pub command: String,

    /// Command arguments.
    #[serde(default)]
    pub args: Vec<String>,

    /// Working directory relative to /workspace in guest.
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

    /// Checkout directory on the host to copy into /workspace.
    /// This is used when the checkout is on the host filesystem and needs
    /// to be copied into the VM's workspace via virtiofs.
    #[serde(default)]
    pub checkout_dir: Option<String>,

    /// Flake attribute to prefetch for nix commands.
    /// If not set, will attempt to extract from args.
    #[serde(default)]
    pub flake_attr: Option<String>,
}

fn default_working_dir() -> String {
    ".".to_string()
}

fn default_timeout() -> u64 {
    CI_VM_DEFAULT_EXECUTION_TIMEOUT_MS / 1000
}

impl CloudHypervisorPayload {
    /// Validate the payload.
    pub fn validate(&self) -> Result<()> {
        if self.command.is_empty() {
            return Err(CloudHypervisorError::InvalidConfig {
                message: "command cannot be empty".to_string(),
            });
        }

        if self.command.len() > MAX_COMMAND_LENGTH {
            return Err(CloudHypervisorError::InvalidConfig {
                message: format!("command too long: {} bytes (max: {})", self.command.len(), MAX_COMMAND_LENGTH),
            });
        }

        if self.args.len() > MAX_ARGS_COUNT {
            return Err(CloudHypervisorError::InvalidConfig {
                message: format!("too many arguments: {} (max: {})", self.args.len(), MAX_ARGS_COUNT),
            });
        }

        for (i, arg) in self.args.iter().enumerate() {
            if arg.len() > MAX_ARG_LENGTH {
                return Err(CloudHypervisorError::InvalidConfig {
                    message: format!("argument {} too long: {} bytes (max: {})", i, arg.len(), MAX_ARG_LENGTH),
                });
            }
        }

        if self.env.len() > MAX_ENV_COUNT {
            return Err(CloudHypervisorError::InvalidConfig {
                message: format!("too many environment variables: {} (max: {})", self.env.len(), MAX_ENV_COUNT),
            });
        }

        let max_timeout = CI_VM_MAX_EXECUTION_TIMEOUT_MS / 1000;
        if self.timeout_secs > max_timeout {
            return Err(CloudHypervisorError::InvalidConfig {
                message: format!("timeout too long: {} seconds (max: {})", self.timeout_secs, max_timeout),
            });
        }

        if self.artifacts.len() > MAX_ARTIFACTS {
            return Err(CloudHypervisorError::InvalidConfig {
                message: format!("too many artifact patterns: {} (max: {})", self.artifacts.len(), MAX_ARTIFACTS),
            });
        }

        Ok(())
    }
}

/// Cloud Hypervisor-based CI worker.
///
/// Executes CI jobs inside isolated microVMs using Cloud Hypervisor.
/// VMs are managed by a warm pool for fast job startup.
pub struct CloudHypervisorWorker {
    /// Worker configuration.
    config: CloudHypervisorWorkerConfig,

    /// VM pool for warm VM management.
    pool: Arc<VmPool>,

    /// Optional blob store for workspace seeding and artifact storage.
    /// When provided, the worker can:
    /// - Seed workspace from source blobs (via source_hash in payload)
    /// - Upload collected artifacts to distributed storage
    blob_store: Option<Arc<dyn BlobStore>>,

    /// Handle for the pool maintenance background task.
    /// Dropped on worker shutdown.
    maintenance_task: tokio::sync::RwLock<Option<tokio::task::JoinHandle<()>>>,
}

/// Interval between pool maintenance cycles (30 seconds).
const POOL_MAINTENANCE_INTERVAL_SECS: u64 = 30;

impl CloudHypervisorWorker {
    /// Create a new Cloud Hypervisor worker.
    pub fn new(config: CloudHypervisorWorkerConfig) -> Result<Self> {
        Self::with_blob_store(config, None)
    }

    /// Create a new Cloud Hypervisor worker with an optional blob store.
    ///
    /// When a blob store is provided:
    /// - Jobs with `source_hash` will have their workspace seeded from the blob
    /// - Collected artifacts can be uploaded to distributed storage
    pub fn with_blob_store(
        config: CloudHypervisorWorkerConfig,
        blob_store: Option<Arc<dyn BlobStore>>,
    ) -> Result<Self> {
        config.validate().map_err(|e| CloudHypervisorError::InvalidConfig { message: e })?;

        let pool = Arc::new(VmPool::new(config.clone()));

        Ok(Self {
            config,
            pool,
            blob_store,
            maintenance_task: tokio::sync::RwLock::new(None),
        })
    }

    /// Start the pool maintenance background task.
    ///
    /// This task periodically checks the pool and ensures there are enough
    /// warm VMs available for quick job startup. Called automatically by `on_start()`.
    async fn start_maintenance_task(&self) {
        let pool = self.pool.clone();
        let interval = Duration::from_secs(POOL_MAINTENANCE_INTERVAL_SECS);

        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                ticker.tick().await;

                // Run pool maintenance
                pool.maintain().await;

                // Log pool status periodically
                let status = pool.status().await;
                debug!(
                    idle = status.idle_vms,
                    total = status.total_vms,
                    max = status.max_vms,
                    target = status.target_pool_size,
                    "pool maintenance cycle complete"
                );
            }
        });

        *self.maintenance_task.write().await = Some(handle);
        info!(interval_secs = POOL_MAINTENANCE_INTERVAL_SECS, "pool maintenance task started");
    }

    /// Stop the pool maintenance background task.
    async fn stop_maintenance_task(&self) {
        if let Some(handle) = self.maintenance_task.write().await.take() {
            handle.abort();
            info!("pool maintenance task stopped");
        }
    }

    /// Get the VM pool for monitoring.
    pub fn pool(&self) -> &Arc<VmPool> {
        &self.pool
    }

    /// Execute a job in a VM, collect artifacts, and optionally upload to blob store.
    async fn execute_in_vm(
        &self,
        job: &Job,
        payload: &CloudHypervisorPayload,
    ) -> Result<(ExecutionResult, ArtifactCollectionResult, Option<ArtifactUploadResult>)> {
        let job_id = job.id.to_string();

        // Acquire VM from pool
        let vm = self.pool.acquire(&job_id).await?;
        info!(
            job_id = %job_id,
            vm_id = %vm.id,
            "acquired VM for job"
        );

        // Copy checkout directory into workspace if provided
        // This copies the host checkout (e.g., /tmp/ci-checkout-xxx) into the VM's
        // workspace directory which is shared via virtiofs as /workspace
        if let Some(ref checkout_dir) = payload.checkout_dir {
            let workspace = vm.workspace_dir();
            let checkout_path = PathBuf::from(checkout_dir);
            if checkout_path.exists() {
                match copy_directory_contents(&checkout_path, &workspace).await {
                    Ok(count) => {
                        info!(
                            job_id = %job_id,
                            vm_id = %vm.id,
                            checkout_dir = %checkout_dir,
                            files_copied = count,
                            workspace = %workspace.display(),
                            "checkout copied to workspace"
                        );
                    }
                    Err(e) => {
                        warn!(
                            job_id = %job_id,
                            vm_id = %vm.id,
                            checkout_dir = %checkout_dir,
                            error = ?e,
                            "failed to copy checkout to workspace"
                        );
                        // Continue anyway - job may handle missing files
                    }
                }

                // Pre-fetch flake inputs on the host for nix commands.
                // CI VMs have no network, so we must ensure all flake inputs are
                // in the /nix/store before VM execution. The VM mounts the store
                // via virtiofs and can then build offline.
                if payload.command == "nix" && workspace.join("flake.nix").exists() {
                    let flake_ref = extract_flake_ref(payload);
                    match prefetch_flake_inputs(&workspace, &flake_ref).await {
                        Ok(()) => {
                            info!(
                                job_id = %job_id,
                                vm_id = %vm.id,
                                flake_ref = %flake_ref,
                                "pre-fetched flake inputs on host"
                            );
                        }
                        Err(e) => {
                            warn!(
                                job_id = %job_id,
                                vm_id = %vm.id,
                                flake_ref = %flake_ref,
                                error = ?e,
                                "failed to pre-fetch flake inputs (VM build may fail)"
                            );
                            // Continue anyway - the build will fail with a clearer error
                        }
                    }
                }
            } else {
                warn!(
                    job_id = %job_id,
                    checkout_dir = %checkout_dir,
                    "checkout_dir does not exist, skipping copy"
                );
            }
        }

        // Seed workspace from blob store if source_hash is provided
        if let Some(ref source_hash) = payload.source_hash {
            if let Some(ref blob_store) = self.blob_store {
                let workspace = vm.workspace_dir();
                match seed_workspace_from_blob(blob_store, source_hash, &workspace).await {
                    Ok(bytes) => {
                        info!(
                            job_id = %job_id,
                            vm_id = %vm.id,
                            source_hash = %source_hash,
                            bytes = bytes,
                            "workspace seeded from blob"
                        );
                    }
                    Err(e) => {
                        warn!(
                            job_id = %job_id,
                            vm_id = %vm.id,
                            source_hash = %source_hash,
                            error = ?e,
                            "workspace seeding failed, continuing with empty workspace"
                        );
                        // Don't fail the job - continue with empty workspace
                        // The job itself may handle missing sources
                    }
                }
            } else {
                warn!(
                    job_id = %job_id,
                    source_hash = %source_hash,
                    "source_hash provided but no blob_store configured, skipping workspace seeding"
                );
            }
        }

        // Execute job in VM
        let exec_result = self.execute_on_vm(&vm, &job_id, payload).await;

        // Collect artifacts from workspace (before releasing VM)
        // Only collect if execution succeeded
        let (artifacts, upload_result) = if let Ok(ref result) = exec_result {
            if result.exit_code == 0 && result.error.is_none() && !payload.artifacts.is_empty() {
                let workspace = vm.workspace_dir();
                match collect_artifacts(&workspace, &payload.artifacts).await {
                    Ok(collected) => {
                        // Upload artifacts to blob store if available
                        let upload = if let Some(ref blob_store) = self.blob_store {
                            if !collected.artifacts.is_empty() {
                                Some(upload_artifacts_to_blob_store(&collected, blob_store, &job_id).await)
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
            } else {
                (ArtifactCollectionResult::default(), None)
            }
        } else {
            (ArtifactCollectionResult::default(), None)
        };

        // Always release VM back to pool
        if let Err(e) = self.pool.release(vm.clone()).await {
            warn!(vm_id = %vm.id, error = ?e, "failed to release VM to pool");
        }

        exec_result.map(|r| (r, artifacts, upload_result))
    }

    /// Execute a job on a specific VM.
    async fn execute_on_vm(
        &self,
        vm: &SharedVm,
        job_id: &str,
        payload: &CloudHypervisorPayload,
    ) -> Result<ExecutionResult> {
        // Mark VM as running
        vm.mark_running().await?;

        // Build execution request
        let working_dir = if payload.working_dir.starts_with('/') {
            PathBuf::from(&payload.working_dir)
        } else {
            PathBuf::from("/workspace").join(&payload.working_dir)
        };

        // Inject --offline for nix commands since CI VMs have no network access.
        // This allows flake evaluation to succeed using only the local /nix/store
        // and flake.lock without attempting to verify/fetch remote inputs.
        let args = if payload.command == "nix" && !payload.args.iter().any(|a| a == "--offline") {
            let mut args = payload.args.clone();
            // Insert --offline after the subcommand (build, develop, etc.)
            // nix <subcommand> --offline [rest of args]
            if !args.is_empty() {
                args.insert(1, "--offline".to_string());
                debug!(job_id = %job_id, "injected --offline for nix command (VM has no network)");
            }
            args
        } else {
            payload.args.clone()
        };

        // Set up environment with writable HOME and cache directories.
        // CI VMs have read-only root filesystems, so /root/.cache/nix fails.
        // Use /tmp for caches since it's always writable.
        let mut env = payload.env.clone();
        env.entry("HOME".to_string()).or_insert_with(|| "/tmp".to_string());
        env.entry("XDG_CACHE_HOME".to_string()).or_insert_with(|| "/tmp/.cache".to_string());

        let request = ExecutionRequest {
            id: job_id.to_string(),
            command: payload.command.clone(),
            args,
            working_dir,
            env,
            timeout_secs: payload.timeout_secs,
        };

        // Connect to guest agent via vsock
        let vsock_path = vm.vsock_socket_path();
        debug!(vm_id = %vm.id, vsock = ?vsock_path, "connecting to guest agent");

        let stream = UnixStream::connect(&vsock_path).await.map_err(|e| CloudHypervisorError::VsockConnect {
            vm_id: vm.id.clone(),
            source: e,
        })?;

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Send Cloud Hypervisor vsock CONNECT handshake.
        // Cloud Hypervisor requires "CONNECT <port>\n" before any data exchange.
        // After sending CONNECT, we must wait for "OK <port>\n" response.
        let connect_cmd = format!("CONNECT {}\n", CI_VM_VSOCK_PORT);
        writer
            .write_all(connect_cmd.as_bytes())
            .await
            .map_err(|e| CloudHypervisorError::VsockSend { source: e })?;
        debug!(vm_id = %vm.id, port = CI_VM_VSOCK_PORT, "sent vsock CONNECT handshake");

        // Wait for "OK <port>\n" response from Cloud Hypervisor
        let mut ok_line = String::new();
        let ok_timeout = Duration::from_secs(5);
        match tokio::time::timeout(ok_timeout, reader.read_line(&mut ok_line)).await {
            Ok(Ok(0)) => {
                return Err(CloudHypervisorError::GuestAgentError {
                    message: format!("vsock connection closed (no listener on port {})", CI_VM_VSOCK_PORT),
                });
            }
            Ok(Ok(_)) => {
                let trimmed = ok_line.trim();
                if trimmed.starts_with("OK ") {
                    debug!(vm_id = %vm.id, response = %trimmed, "received vsock OK response");
                } else {
                    warn!(
                        vm_id = %vm.id,
                        response = %trimmed,
                        "unexpected vsock response (expected OK <port>)"
                    );
                }
            }
            Ok(Err(e)) => {
                return Err(CloudHypervisorError::VsockRecv { source: e });
            }
            Err(_) => {
                return Err(CloudHypervisorError::GuestAgentError {
                    message: format!("vsock OK response timeout after {}ms", ok_timeout.as_millis()),
                });
            }
        }

        // Send execution request
        let host_msg = HostMessage::Execute(request);
        self.send_message(&mut writer, &host_msg).await?;

        // Stream logs and wait for completion
        let start = Instant::now();
        let timeout = Duration::from_secs(payload.timeout_secs);
        let mut stdout = String::new();
        let mut stderr = String::new();

        loop {
            // Check timeout
            if start.elapsed() > timeout {
                // Send cancel
                let cancel_msg = HostMessage::Cancel { id: job_id.to_string() };
                let _ = self.send_message(&mut writer, &cancel_msg).await;

                return Ok(ExecutionResult {
                    id: job_id.to_string(),
                    exit_code: -1,
                    stdout,
                    stderr,
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: Some("execution timed out".to_string()),
                });
            }

            // Read next message with timeout
            let read_timeout = timeout.saturating_sub(start.elapsed());
            match tokio::time::timeout(read_timeout, self.read_message(&mut reader)).await {
                Ok(Ok(msg)) => match msg {
                    AgentMessage::Stdout { data } => {
                        if stdout.len() + data.len() <= INLINE_LOG_THRESHOLD {
                            stdout.push_str(&data);
                        }
                        debug!(job_id = %job_id, len = data.len(), "stdout chunk");
                    }
                    AgentMessage::Stderr { data } => {
                        if stderr.len() + data.len() <= INLINE_LOG_THRESHOLD {
                            stderr.push_str(&data);
                        }
                        debug!(job_id = %job_id, len = data.len(), "stderr chunk");
                    }
                    AgentMessage::Complete { result } => {
                        info!(
                            job_id = %job_id,
                            exit_code = result.exit_code,
                            duration_ms = result.duration_ms,
                            "job completed"
                        );
                        return Ok(result);
                    }
                    AgentMessage::Heartbeat { elapsed_secs } => {
                        debug!(job_id = %job_id, elapsed_secs = elapsed_secs, "heartbeat");
                    }
                    AgentMessage::Error { message } => {
                        return Ok(ExecutionResult {
                            id: job_id.to_string(),
                            exit_code: -1,
                            stdout,
                            stderr,
                            duration_ms: start.elapsed().as_millis() as u64,
                            error: Some(message),
                        });
                    }
                    AgentMessage::Pong | AgentMessage::Ready => {
                        // Unexpected but not fatal
                        debug!(job_id = %job_id, "unexpected agent message");
                    }
                },
                Ok(Err(e)) => {
                    if matches!(e, CloudHypervisorError::VsockRecv { .. }) {
                        // Connection closed
                        return Ok(ExecutionResult {
                            id: job_id.to_string(),
                            exit_code: -1,
                            stdout,
                            stderr,
                            duration_ms: start.elapsed().as_millis() as u64,
                            error: Some("connection to guest agent lost".to_string()),
                        });
                    }
                    return Err(e);
                }
                Err(_) => {
                    // Timeout
                    return Ok(ExecutionResult {
                        id: job_id.to_string(),
                        exit_code: -1,
                        stdout,
                        stderr,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error: Some("execution timed out".to_string()),
                    });
                }
            }
        }
    }

    /// Send a framed message to the guest agent.
    async fn send_message<W: AsyncWriteExt + Unpin>(&self, writer: &mut W, msg: &HostMessage) -> Result<()> {
        let json = serde_json::to_vec(msg).map_err(|e| CloudHypervisorError::SerializeRequest { source: e })?;

        if json.len() > MAX_MESSAGE_SIZE as usize {
            return Err(CloudHypervisorError::GuestAgentError {
                message: format!("message too large: {} bytes", json.len()),
            });
        }

        // Write length prefix (4 bytes, big endian)
        let len_bytes = (json.len() as u32).to_be_bytes();
        writer.write_all(&len_bytes).await.map_err(|e| CloudHypervisorError::VsockSend { source: e })?;

        // Write JSON payload
        writer.write_all(&json).await.map_err(|e| CloudHypervisorError::VsockSend { source: e })?;

        writer.flush().await.map_err(|e| CloudHypervisorError::VsockSend { source: e })?;

        Ok(())
    }

    /// Read a framed message from the guest agent.
    async fn read_message<R: AsyncReadExt + Unpin>(&self, reader: &mut R) -> Result<AgentMessage> {
        // Read length prefix (4 bytes, big endian)
        let mut len_bytes = [0u8; 4];
        reader.read_exact(&mut len_bytes).await.map_err(|e| {
            if e.kind() == ErrorKind::UnexpectedEof {
                CloudHypervisorError::VsockRecv {
                    source: io::Error::new(ErrorKind::UnexpectedEof, "connection closed"),
                }
            } else {
                CloudHypervisorError::VsockRecv { source: e }
            }
        })?;

        let len = u32::from_be_bytes(len_bytes);
        if len > MAX_MESSAGE_SIZE {
            return Err(CloudHypervisorError::GuestAgentError {
                message: format!("message too large: {} bytes", len),
            });
        }

        // Read JSON payload
        let mut buf = vec![0u8; len as usize];
        reader.read_exact(&mut buf).await.map_err(|e| CloudHypervisorError::VsockRecv { source: e })?;

        let msg: AgentMessage =
            serde_json::from_slice(&buf).map_err(|e| CloudHypervisorError::DeserializeResponse { source: e })?;

        Ok(msg)
    }
}

#[async_trait]
impl Worker for CloudHypervisorWorker {
    async fn execute(&self, job: Job) -> JobResult {
        let job_id = job.id.to_string();
        info!(job_id = %job_id, job_type = %job.spec.job_type, "executing Cloud Hypervisor job");

        // Parse payload
        let payload: CloudHypervisorPayload = match serde_json::from_value(job.spec.payload.clone()) {
            Ok(p) => p,
            Err(e) => {
                error!(job_id = %job_id, error = ?e, "failed to parse job payload");
                return JobResult::failure(format!("invalid job payload: {}", e));
            }
        };

        // Validate payload
        if let Err(e) = payload.validate() {
            error!(job_id = %job_id, error = ?e, "invalid job payload");
            return JobResult::failure(format!("invalid job payload: {}", e));
        }

        // Execute in VM
        match self.execute_in_vm(&job, &payload).await {
            Ok((result, artifacts, upload_result)) => {
                if result.exit_code == 0 && result.error.is_none() {
                    // Build artifact list for output (include blob hashes if uploaded)
                    let artifact_list: Vec<_> = if let Some(ref upload) = upload_result {
                        // Use uploaded artifacts with blob references
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
                        // No upload - use collected artifacts without blob refs
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

                    // Include upload stats in output if available
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
                            ("vm_execution".to_string(), "true".to_string()),
                            ("duration_ms".to_string(), result.duration_ms.to_string()),
                            ("artifacts_count".to_string(), artifacts.artifacts.len().to_string()),
                            ("artifacts_total_size".to_string(), artifacts.total_size.to_string()),
                            (
                                "artifacts_uploaded".to_string(),
                                upload_result.as_ref().map(|u| u.uploaded.len()).unwrap_or(0).to_string(),
                            ),
                        ]),
                    };
                    JobResult::Success(output)
                } else {
                    // Include stderr/stdout in failure message for debugging.
                    // Truncate to reasonable length to avoid huge error messages.
                    let stderr_preview: String = result.stderr.chars().take(2048).collect();
                    let stdout_preview: String = result.stdout.chars().take(512).collect();

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
                error!(job_id = %job_id, error = ?e, "VM execution failed");
                JobResult::failure(format!("VM execution failed: {}", e))
            }
        }
    }

    async fn on_start(&self) -> std::result::Result<(), JobError> {
        info!(
            pool_size = self.config.pool_size,
            max_vms = self.config.max_vms,
            "initializing Cloud Hypervisor worker"
        );

        if let Err(e) = self.pool.initialize().await {
            error!(error = ?e, "failed to initialize VM pool");
            return Err(JobError::WorkerRegistrationFailed {
                reason: format!("VM pool initialization failed: {}", e),
            });
        }

        // Start pool maintenance background task
        self.start_maintenance_task().await;

        info!("Cloud Hypervisor worker initialized");
        Ok(())
    }

    async fn on_shutdown(&self) -> std::result::Result<(), JobError> {
        info!("shutting down Cloud Hypervisor worker");

        // Stop maintenance task first
        self.stop_maintenance_task().await;

        // Then shutdown the pool
        if let Err(e) = self.pool.shutdown().await {
            warn!(error = ?e, "error shutting down VM pool");
        }

        info!("Cloud Hypervisor worker shutdown complete");
        Ok(())
    }

    fn job_types(&self) -> Vec<String> {
        vec!["ci_vm".to_string(), "cloud_hypervisor".to_string()]
    }
}

/// Copy contents of a directory to another directory.
///
/// Returns the number of files/directories copied.
async fn copy_directory_contents(src: &std::path::Path, dst: &std::path::Path) -> io::Result<usize> {
    use tokio::fs;

    // Create destination if it doesn't exist
    fs::create_dir_all(dst).await?;

    let mut count = 0;
    let mut entries = fs::read_dir(src).await?;

    while let Some(entry) = entries.next_entry().await? {
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        let file_type = entry.file_type().await?;

        if file_type.is_dir() {
            // Recursively copy subdirectories
            // Use Box::pin for async recursion
            count += Box::pin(copy_directory_contents(&src_path, &dst_path)).await?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path).await?;
            count += 1;
        } else if file_type.is_symlink() {
            // Copy symlinks as symlinks
            let target = fs::read_link(&src_path).await?;
            // Remove existing symlink/file if present
            let _ = fs::remove_file(&dst_path).await;
            #[cfg(unix)]
            {
                tokio::fs::symlink(&target, &dst_path).await?;
            }
            #[cfg(not(unix))]
            {
                // On non-Unix, copy the target instead
                if src_path.is_file() {
                    fs::copy(&src_path, &dst_path).await?;
                }
            }
            count += 1;
        }
    }

    Ok(count)
}

/// Extract or construct the flake reference from payload.
///
/// Priority:
/// 1. Use explicit `flake_attr` from payload if set
/// 2. Parse from `args` for nix commands (look for .#... pattern)
/// 3. Default based on subcommand (develop -> devShells, build -> packages)
fn extract_flake_ref(payload: &CloudHypervisorPayload) -> String {
    // If explicit flake_attr is set, use it
    if let Some(ref attr) = payload.flake_attr {
        return format!(".#{}", attr);
    }

    // Try to parse from args
    if payload.command == "nix" && !payload.args.is_empty() {
        let subcommand = &payload.args[0];

        // Look for flake reference in args (starts with ".#" or contains "#")
        for arg in &payload.args[1..] {
            if arg.starts_with('-') {
                continue; // Skip flags
            }
            if arg.starts_with(".#") || arg.contains('#') {
                return arg.clone();
            }
        }

        // Default based on subcommand
        match subcommand.as_str() {
            "develop" | "shell" => return ".#devShells.x86_64-linux.default".to_string(),
            _ => {}
        }
    }

    ".#packages.x86_64-linux.default".to_string()
}

/// Pre-fetch flake inputs on the host so they're available in /nix/store.
///
/// CI VMs have no network access, so all flake inputs must be present in the
/// /nix/store before execution. This function runs `nix build --dry-run` on
/// the host to fetch and evaluate everything needed, which is then accessible
/// to the VM via the virtiofs mount of /nix/store.
///
/// We use `nix build --dry-run` instead of `nix flake archive` because:
/// - `nix flake archive` only fetches flake metadata, not source tarballs
/// - `nix build --dry-run` does full evaluation, fetching all inputs
/// - The `--dry-run` flag prevents actual building, just fetches dependencies
///
/// # Arguments
///
/// * `workspace` - Path to the workspace containing flake.nix
/// * `flake_ref` - The flake reference to prefetch (e.g., ".#packages.x86_64-linux.default")
async fn prefetch_flake_inputs(workspace: &std::path::Path, flake_ref: &str) -> io::Result<()> {
    use std::process::Stdio;
    use tokio::process::Command;

    // Use `nix build --dry-run` which evaluates the flake and fetches all inputs
    // without actually building. This ensures everything is in the store.
    let output = Command::new("nix")
        .args([
            "build",
            "--dry-run",
            "--no-link",
            "--no-write-lock-file",
            "--no-update-lock-file",
            flake_ref,
        ])
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(format!(
            "nix build --dry-run failed for {}: {}",
            flake_ref,
            stderr.chars().take(500).collect::<String>()
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> CloudHypervisorWorkerConfig {
        CloudHypervisorWorkerConfig {
            node_id: 1,
            state_dir: PathBuf::from("/tmp/aspen-ci-test"),
            pool_size: 2,
            max_vms: 8,
            kernel_path: PathBuf::new(),
            initrd_path: PathBuf::new(),
            toplevel_path: PathBuf::new(),
            ..Default::default()
        }
    }

    #[test]
    fn test_payload_validation() {
        // Valid payload
        let payload = CloudHypervisorPayload {
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
        let invalid = CloudHypervisorPayload {
            command: "".to_string(),
            ..payload.clone()
        };
        assert!(invalid.validate().is_err());

        // Command too long
        let invalid = CloudHypervisorPayload {
            command: "x".repeat(MAX_COMMAND_LENGTH + 1),
            ..payload.clone()
        };
        assert!(invalid.validate().is_err());

        // Timeout too long
        let invalid = CloudHypervisorPayload {
            timeout_secs: CI_VM_MAX_EXECUTION_TIMEOUT_MS / 1000 + 1,
            ..payload.clone()
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_worker_job_types() {
        let config = test_config();
        let worker = CloudHypervisorWorker::new(config).unwrap();

        let types = worker.job_types();
        assert!(types.contains(&"ci_vm".to_string()));
        assert!(types.contains(&"cloud_hypervisor".to_string()));
    }

    #[test]
    fn test_nix_offline_injection() {
        // Test that --offline is injected for nix commands
        let payload = CloudHypervisorPayload {
            job_name: Some("test".to_string()),
            command: "nix".to_string(),
            args: vec!["build".to_string(), "-L".to_string(), ".#default".to_string()],
            working_dir: ".".to_string(),
            env: HashMap::new(),
            timeout_secs: 3600,
            artifacts: vec![],
            source_hash: None,
            checkout_dir: None,
            flake_attr: None,
        };

        // Simulate the injection logic from execute_on_vm
        let args = if payload.command == "nix" && !payload.args.iter().any(|a| a == "--offline") {
            let mut args = payload.args.clone();
            if !args.is_empty() {
                args.insert(1, "--offline".to_string());
            }
            args
        } else {
            payload.args.clone()
        };

        assert_eq!(args, vec!["build", "--offline", "-L", ".#default"]);
    }

    #[test]
    fn test_nix_offline_not_duplicated() {
        // Test that --offline is not duplicated if already present
        let payload = CloudHypervisorPayload {
            job_name: Some("test".to_string()),
            command: "nix".to_string(),
            args: vec!["build".to_string(), "--offline".to_string(), ".#default".to_string()],
            working_dir: ".".to_string(),
            env: HashMap::new(),
            timeout_secs: 3600,
            artifacts: vec![],
            source_hash: None,
            checkout_dir: None,
            flake_attr: None,
        };

        // Simulate the injection logic from execute_on_vm
        let args = if payload.command == "nix" && !payload.args.iter().any(|a| a == "--offline") {
            let mut args = payload.args.clone();
            if !args.is_empty() {
                args.insert(1, "--offline".to_string());
            }
            args
        } else {
            payload.args.clone()
        };

        // Should remain unchanged since --offline is already present
        assert_eq!(args, vec!["build", "--offline", ".#default"]);
    }

    #[test]
    fn test_non_nix_command_unchanged() {
        // Test that non-nix commands are not modified
        let payload = CloudHypervisorPayload {
            job_name: Some("test".to_string()),
            command: "cargo".to_string(),
            args: vec!["build".to_string(), "--release".to_string()],
            working_dir: ".".to_string(),
            env: HashMap::new(),
            timeout_secs: 3600,
            artifacts: vec![],
            source_hash: None,
            checkout_dir: None,
            flake_attr: None,
        };

        // Simulate the injection logic from execute_on_vm
        let args = if payload.command == "nix" && !payload.args.iter().any(|a| a == "--offline") {
            let mut args = payload.args.clone();
            if !args.is_empty() {
                args.insert(1, "--offline".to_string());
            }
            args
        } else {
            payload.args.clone()
        };

        // Should remain unchanged
        assert_eq!(args, vec!["build", "--release"]);
    }

    #[test]
    fn test_extract_flake_ref_explicit_attr() {
        // Explicit flake_attr takes priority
        let payload = CloudHypervisorPayload {
            job_name: None,
            command: "nix".to_string(),
            args: vec!["build".to_string()],
            working_dir: ".".to_string(),
            env: HashMap::new(),
            timeout_secs: 3600,
            artifacts: vec![],
            source_hash: None,
            checkout_dir: None,
            flake_attr: Some("devShells.x86_64-linux.default".to_string()),
        };
        assert_eq!(extract_flake_ref(&payload), ".#devShells.x86_64-linux.default");
    }

    #[test]
    fn test_extract_flake_ref_from_args() {
        // Parse flake reference from args
        let payload = CloudHypervisorPayload {
            job_name: None,
            command: "nix".to_string(),
            args: vec![
                "build".to_string(),
                "-L".to_string(),
                ".#checks.x86_64-linux.default".to_string(),
            ],
            working_dir: ".".to_string(),
            env: HashMap::new(),
            timeout_secs: 3600,
            artifacts: vec![],
            source_hash: None,
            checkout_dir: None,
            flake_attr: None,
        };
        assert_eq!(extract_flake_ref(&payload), ".#checks.x86_64-linux.default");
    }

    #[test]
    fn test_extract_flake_ref_develop_default() {
        // nix develop defaults to devShells
        let payload = CloudHypervisorPayload {
            job_name: None,
            command: "nix".to_string(),
            args: vec![
                "develop".to_string(),
                "-c".to_string(),
                "cargo".to_string(),
                "build".to_string(),
            ],
            working_dir: ".".to_string(),
            env: HashMap::new(),
            timeout_secs: 3600,
            artifacts: vec![],
            source_hash: None,
            checkout_dir: None,
            flake_attr: None,
        };
        assert_eq!(extract_flake_ref(&payload), ".#devShells.x86_64-linux.default");
    }

    #[test]
    fn test_extract_flake_ref_build_default() {
        // nix build defaults to packages
        let payload = CloudHypervisorPayload {
            job_name: None,
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
        assert_eq!(extract_flake_ref(&payload), ".#packages.x86_64-linux.default");
    }

    #[test]
    fn test_extract_flake_ref_non_nix_command() {
        // Non-nix commands still get a default (though prefetch won't be called)
        let payload = CloudHypervisorPayload {
            job_name: None,
            command: "cargo".to_string(),
            args: vec!["build".to_string()],
            working_dir: ".".to_string(),
            env: HashMap::new(),
            timeout_secs: 3600,
            artifacts: vec![],
            source_hash: None,
            checkout_dir: None,
            flake_attr: None,
        };
        assert_eq!(extract_flake_ref(&payload), ".#packages.x86_64-linux.default");
    }
}
