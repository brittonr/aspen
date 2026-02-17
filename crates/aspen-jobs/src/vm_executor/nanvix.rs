//! Nanvix micro-VM worker implementation.
//!
//! Executes JavaScript, Python, and native binary workloads inside
//! hardware-isolated micro-VMs via hyperlight-nanvix. Guest I/O is
//! captured from the console log file written by the Nanvix microkernel.

use std::sync::Arc;

use aspen_blob::prelude::*;
use async_trait::async_trait;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::error::JobError;
use crate::error::Result;
use crate::job::Job;
use crate::job::JobResult;
use crate::vm_executor::types::JobPayload;
use crate::worker::Worker;

/// Maximum size for a workload file (50MB).
const MAX_WORKLOAD_SIZE: usize = 50 * 1024 * 1024;

/// Valid workload types accepted by the NanvixWorker.
const VALID_WORKLOAD_TYPES: &[&str] = &["javascript", "python", "binary"];

/// Worker that executes workloads in Nanvix micro-VMs via hyperlight-nanvix.
///
/// Supports JavaScript (QuickJS), Python (CPython 3.12), and native ELF
/// binaries. Requires KVM at runtime for hardware isolation.
pub struct NanvixWorker {
    /// Blob store for retrieving workload files.
    blob_store: Arc<dyn BlobStore>,
}

impl NanvixWorker {
    /// Create a new Nanvix worker.
    /// Requires a blob store for retrieving workload files.
    pub fn new(blob_store: Arc<dyn BlobStore>) -> Result<Self> {
        Ok(Self { blob_store })
    }

    /// Map a workload type string to the file extension that nanvix
    /// `WorkloadType::from_path()` will recognize.
    fn workload_extension(workload_type: &str) -> Result<&'static str> {
        match workload_type {
            "javascript" => Ok(".js"),
            "python" => Ok(".py"),
            "binary" => Ok(".elf"),
            other => Err(JobError::VmExecutionFailed {
                reason: format!("unknown workload_type '{}': expected one of {:?}", other, VALID_WORKLOAD_TYPES),
            }),
        }
    }

    /// Retrieve workload bytes from the blob store.
    async fn retrieve_workload(&self, hash: &str, expected_size: u64) -> Result<Vec<u8>> {
        info!(hash, expected_size, "retrieving nanvix workload from blob store");

        let blob_hash = hash.parse::<iroh_blobs::Hash>().map_err(|e| JobError::VmExecutionFailed {
            reason: format!("invalid blob hash '{}': {}", hash, e),
        })?;

        let bytes = self
            .blob_store
            .get_bytes(&blob_hash)
            .await
            .map_err(|e| JobError::VmExecutionFailed {
                reason: format!("failed to retrieve blob: {}", e),
            })?
            .ok_or_else(|| JobError::VmExecutionFailed {
                reason: format!("blob not found: {}", hash),
            })?;

        // Validate size if provided (0 means skip validation).
        if expected_size > 0 && bytes.len() as u64 != expected_size {
            return Err(JobError::VmExecutionFailed {
                reason: format!("blob size mismatch: expected {} bytes, got {} bytes", expected_size, bytes.len()),
            });
        }

        if bytes.len() > MAX_WORKLOAD_SIZE {
            return Err(JobError::BinaryTooLarge {
                size_bytes: bytes.len() as u64,
                max_bytes: MAX_WORKLOAD_SIZE as u64,
            });
        }

        Ok(bytes.to_vec())
    }

    /// Execute a workload in a Nanvix sandbox.
    ///
    /// Writes the workload to a temp file with the correct extension so that
    /// `WorkloadType::from_path()` can auto-detect the language runtime.
    /// Console output is read from `{log_directory}/guest-console.log`.
    async fn execute_workload(&self, workload_bytes: Vec<u8>, workload_type: &str) -> Result<JobResult> {
        let extension = Self::workload_extension(workload_type)?;

        info!(workload_type, size = workload_bytes.len(), "executing nanvix workload");

        // Write workload to a temp file with the correct extension.
        let temp_file =
            tempfile::Builder::new().suffix(extension).tempfile().map_err(|e| JobError::VmExecutionFailed {
                reason: format!("failed to create temp file: {}", e),
            })?;

        tokio::fs::write(temp_file.path(), &workload_bytes).await.map_err(|e| JobError::VmExecutionFailed {
            reason: format!("failed to write workload to temp file: {}", e),
        })?;

        // Create RuntimeConfig with unique temp directories.
        let config = hyperlight_nanvix::RuntimeConfig::default();
        let log_directory = config.log_directory.clone();

        debug!(
            log_directory = %log_directory,
            workload_path = %temp_file.path().display(),
            "creating nanvix sandbox"
        );

        // Create and run sandbox (requires KVM at runtime).
        let mut sandbox = hyperlight_nanvix::Sandbox::new(config).map_err(|e| JobError::VmExecutionFailed {
            reason: format!("failed to create nanvix sandbox: {}", e),
        })?;

        sandbox.run(temp_file.path()).await.map_err(|e| JobError::VmExecutionFailed {
            reason: format!("nanvix workload execution failed: {}", e),
        })?;

        // Read console output from the guest.
        let console_log_path = format!("{}/guest-console.log", log_directory);
        let console_output = match tokio::fs::read_to_string(&console_log_path).await {
            Ok(content) => {
                debug!(output_len = content.len(), "read guest console output");
                content
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("no guest console output (file not found)");
                String::new()
            }
            Err(e) => {
                warn!(error = %e, path = %console_log_path, "failed to read guest console log");
                String::new()
            }
        };

        // Clean up temp directories (best-effort).
        let _ = tokio::fs::remove_dir_all(&log_directory).await;

        Ok(JobResult::success(serde_json::json!({
            "console_output": console_output,
            "workload_type": workload_type,
        })))
    }
}

#[async_trait]
impl Worker for NanvixWorker {
    async fn execute(&self, job: Job) -> JobResult {
        let payload: JobPayload = match serde_json::from_value(job.spec.payload.clone()) {
            Ok(p) => p,
            Err(e) => {
                return JobResult::failure(format!("failed to parse job payload: {}", e));
            }
        };

        let result = match payload {
            JobPayload::NanvixWorkload {
                hash,
                size,
                workload_type,
            } => {
                let workload_bytes = match self.retrieve_workload(&hash, size).await {
                    Ok(bytes) => bytes,
                    Err(e) => return JobResult::failure(format!("failed to retrieve nanvix workload: {}", e)),
                };

                self.execute_workload(workload_bytes, &workload_type).await
            }

            other => {
                warn!(job_id = %job.id, payload_type = ?other, "NanvixWorker received non-Nanvix payload");
                Err(JobError::VmExecutionFailed {
                    reason: "NanvixWorker only handles NanvixWorkload payloads".to_string(),
                })
            }
        };

        match result {
            Ok(job_result) => job_result,
            Err(e) => JobResult::failure(format!("nanvix execution failed: {}", e)),
        }
    }

    fn job_types(&self) -> Vec<String> {
        vec!["nanvix_execute".to_string()]
    }
}
