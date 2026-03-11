//! LocalExecutorWorker - Worker implementation for direct command execution.
//!
//! This module provides a Worker that executes CI jobs directly on the host
//! without nested VMs. It reuses the Executor from the agent module for process
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

mod artifacts;
mod config;
mod execution;
pub(crate) mod nix;
mod output;
mod payload;
#[cfg(feature = "snix")]
mod snix;
mod workspace;

use std::collections::HashMap;
use std::sync::Arc;

use aspen_blob::prelude::*;
use aspen_jobs::Job;
use aspen_jobs::JobError;
use aspen_jobs::JobOutput;
use aspen_jobs::JobResult;
use aspen_jobs::Worker;
use async_trait::async_trait;
pub use config::LocalExecutorWorkerConfig;
pub use output::OutputRef;
pub use payload::LocalExecutorPayload;
use tokio::sync::mpsc;
use tracing::error;
use tracing::info;

use crate::agent::executor::Executor;
use crate::agent::protocol::LogMessage;

/// External log sink for real-time log streaming.
///
/// When set on `LocalExecutorWorker`, log messages (stdout/stderr chunks)
/// are forwarded to this sender during execution. This enables VM workers
/// to stream build output to the cluster in real-time.
pub type LogSink = mpsc::Sender<LogMessage>;

/// Local executor worker.
///
/// Executes CI jobs directly using process execution, without nested VMs.
/// Suitable for environments that are already isolated (e.g., running inside
/// a dogfood VM).
pub struct LocalExecutorWorker {
    /// Configuration.
    pub(super) config: LocalExecutorWorkerConfig,

    /// Command executor (from agent module).
    pub(super) executor: Executor,

    /// Optional blob store for workspace seeding and artifact storage.
    pub(super) blob_store: Option<Arc<dyn BlobStore>>,

    /// Optional external log sink for real-time streaming.
    ///
    /// When set, log messages are forwarded to this sender during execution,
    /// in addition to the internal log consumer. This enables VM workers to
    /// stream build output to the cluster KV in real-time via RPC.
    pub(super) log_sink: Option<LogSink>,
}

impl LocalExecutorWorker {
    /// Create a new local executor worker.
    pub fn new(config: LocalExecutorWorkerConfig) -> Self {
        let executor = Executor::with_workspace_root(config.workspace_dir.clone());
        Self {
            config,
            executor,
            blob_store: None,
            log_sink: None,
        }
    }

    /// Create a new local executor worker with a blob store.
    pub fn with_blob_store(config: LocalExecutorWorkerConfig, blob_store: Arc<dyn BlobStore>) -> Self {
        let executor = Executor::with_workspace_root(config.workspace_dir.clone());
        Self {
            config,
            executor,
            blob_store: Some(blob_store),
            log_sink: None,
        }
    }

    /// Set an external log sink for real-time log streaming.
    ///
    /// When set, all stdout/stderr log messages during job execution are
    /// forwarded to this sender. Used by VM workers to stream build output
    /// to the cluster KV store via RPC.
    pub fn set_log_sink(&mut self, sink: LogSink) {
        self.log_sink = Some(sink);
    }
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
            Ok((result, artifacts, upload_result, nix_output)) => {
                if result.exit_code == 0 && result.error.is_none() {
                    // Build artifact list for output
                    let artifact_list: Vec<_> = if let Some(ref upload) = upload_result {
                        upload
                            .uploaded
                            .iter()
                            .map(|a| {
                                serde_json::json!({
                                    "path": a.relative_path.display().to_string(),
                                    "size": a.blob_ref.size_bytes,
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
                                    "size": a.size_bytes,
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

                    // Store outputs in blob store if large, inline if small.
                    // Large outputs (>64KB) are stored in blobs with only a hash reference
                    // kept in the job record. This avoids the 1MB KV store value limit.
                    let stdout_ref = self.store_output(&result.stdout, &job_id, "stdout").await;
                    let stderr_ref = self.store_output(&result.stderr, &job_id, "stderr").await;

                    // Build nix output metadata if paths were captured
                    let nix_output_meta = if !nix_output.output_paths.is_empty() {
                        Some(serde_json::json!({
                            "output_paths": nix_output.output_paths,
                            "binary_blob_hash": nix_output.binary_blob_hash,
                            "binary_size": nix_output.binary_size,
                            "binary_path": nix_output.binary_path,
                        }))
                    } else {
                        None
                    };

                    let output = JobOutput {
                        data: serde_json::json!({
                            "exit_code": result.exit_code,
                            "stdout": stdout_ref,
                            "stderr": stderr_ref,
                            "stdout_full_size": result.stdout.len(),
                            "stderr_full_size": result.stderr.len(),
                            "duration_ms": result.duration_ms,
                            "artifacts": artifact_list,
                            "artifacts_total_size": artifacts.total_size_bytes,
                            "artifacts_skipped": artifacts.skipped_files.len(),
                            "artifacts_unmatched_patterns": artifacts.unmatched_patterns,
                            "artifacts_upload": upload_stats,
                            "nix_output": nix_output_meta,
                        }),
                        metadata: HashMap::from([
                            ("local_execution".to_string(), "true".to_string()),
                            ("duration_ms".to_string(), result.duration_ms.to_string()),
                            ("artifacts_count".to_string(), artifacts.artifacts.len().to_string()),
                            ("artifacts_total_size".to_string(), artifacts.total_size_bytes.to_string()),
                        ]),
                    };
                    JobResult::Success(output)
                } else {
                    // For failures, show last 16 KB of stderr and 4 KB of stdout for diagnosis.
                    // We take from the end since error messages typically appear at the end.
                    // Note: Full outputs are streamed during execution and stored as chunked logs.
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
        // Shell executor only claims shell job types.
        // ci_nix_build → NixBuildWorker (dedicated executor)
        // ci_vm → VM workers (self-register via aspen-node --worker-only)
        // cloud_hypervisor → CloudHypervisorWorker (VM pool manager)
        vec!["shell_command".to_string(), "local_executor".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::nix::inject_nix_flags;
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
            run_id: None,
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
            command: "x".repeat(4096 + 1),
            ..payload.clone()
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_inject_nix_flags() {
        let (cmd, args) = inject_nix_flags(&["build".to_string(), "-L".to_string(), ".#default".to_string()]);

        assert_eq!(cmd, "nix");
        // --offline is intentionally NOT injected (causes tarball read failures in VM CI)
        assert!(!args.contains(&"--offline".to_string()));
        assert!(args.contains(&"--accept-flake-config".to_string()));
        assert!(args.contains(&"--no-write-lock-file".to_string()));
        assert!(args.contains(&"--output-lock-file".to_string()));
    }

    #[test]
    fn test_worker_job_types() {
        let config = LocalExecutorWorkerConfig::default();
        let worker = LocalExecutorWorker::new(config);

        let types = worker.job_types();
        assert!(types.contains(&"shell_command".to_string()));
        assert!(types.contains(&"local_executor".to_string()));
        // Shell executor must NOT claim types belonging to dedicated executors
        assert!(!types.contains(&"ci_nix_build".to_string()), "ci_nix_build belongs to NixBuildWorker");
        assert!(!types.contains(&"ci_vm".to_string()), "ci_vm belongs to VM workers");
        assert!(
            !types.contains(&"cloud_hypervisor".to_string()),
            "cloud_hypervisor belongs to CloudHypervisorWorker"
        );
        assert_eq!(types.len(), 2);
    }
}
