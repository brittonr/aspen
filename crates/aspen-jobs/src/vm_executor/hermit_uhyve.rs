//! Hermit/Uhyve unikernel worker implementation.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use aspen_blob::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::info;

use crate::error::JobError;
use crate::error::Result;
use crate::job::Job;
use crate::job::JobResult;
use crate::worker::Worker;

/// Job type routed to [`HermitUhyveWorker`].
pub const HERMIT_UHYVE_JOB_TYPE: &str = "hermit_uhyve";

/// Product-visible marker recorded after a Hermit guest reaches Uhyve execution.
pub const HERMIT_UHYVE_RUNTIME_HOST_EXECUTED_MARKER: &str = "ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED";

/// Guard marker for product-path negative tests.
pub const HERMIT_UHYVE_RUNTIME_HOST_PRODUCT_PATH_GUARD_MARKER: &str =
    "ASPEN_HERMIT_UHYVE_RUNTIME_HOST_PRODUCT_PATH_GUARD";

const HERMIT_UHYVE_RUNTIME_HOST_ABI: &str = "aspen:runtime-host/hermit-uhyve-v1";
const HERMIT_UHYVE_ENGINE: &str = "uhyve";
const DEFAULT_UHYVE_TIMEOUT_SECS: u64 = 30;
const MAX_HERMIT_IMAGE_SIZE: usize = 64 * 1024 * 1024;
const MAX_SERIAL_LOG_BYTES: usize = 16 * 1024;

/// Blob-backed Hermit/Uhyve job payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HermitUhyveJobPayload {
    /// BLAKE3 blob hash for the Hermit unikernel image.
    pub image_hash: String,
    /// Expected image size in bytes. Use 0 to skip size validation.
    pub image_size: u64,
    /// Immutable runtime artifact hash recorded in receipts.
    pub artifact_hash: String,
    /// Declared runner capability handle.
    pub runner_capability: String,
    /// Optional proof marker the guest is expected to print.
    pub expected_marker: Option<String>,
    /// Extra boot arguments passed after the image path.
    #[serde(default)]
    pub boot_args: Vec<String>,
    /// Timeout in seconds for the Uhyve process.
    pub timeout_secs: Option<u64>,
    /// Maximum serial log bytes to include in the receipt.
    pub serial_log_limit_bytes: Option<usize>,
}

impl HermitUhyveJobPayload {
    /// Construct a minimal blob-backed Hermit/Uhyve payload.
    #[must_use]
    pub fn blob_image(image_hash: impl Into<String>, image_size: u64, artifact_hash: impl Into<String>) -> Self {
        Self {
            image_hash: image_hash.into(),
            image_size,
            artifact_hash: artifact_hash.into(),
            runner_capability: "runner:hermit-uhyve".to_string(),
            expected_marker: Some(HERMIT_UHYVE_RUNTIME_HOST_EXECUTED_MARKER.to_string()),
            boot_args: vec![],
            timeout_secs: Some(DEFAULT_UHYVE_TIMEOUT_SECS),
            serial_log_limit_bytes: Some(MAX_SERIAL_LOG_BYTES),
        }
    }
}

/// Worker that executes Hermit unikernel images via Uhyve.
pub struct HermitUhyveWorker {
    blob_store: Arc<dyn BlobStore>,
    uhyve_command: PathBuf,
}

impl HermitUhyveWorker {
    /// Create a worker that resolves `uhyve` from the host path.
    pub fn new(blob_store: Arc<dyn BlobStore>) -> Result<Self> {
        Ok(Self::with_uhyve_command(blob_store, PathBuf::from("uhyve")))
    }

    /// Create a worker with an explicit Uhyve command path.
    #[must_use]
    pub fn with_uhyve_command(blob_store: Arc<dyn BlobStore>, uhyve_command: PathBuf) -> Self {
        Self {
            blob_store,
            uhyve_command,
        }
    }

    async fn retrieve_image(&self, hash: &str, expected_size: u64) -> Result<Vec<u8>> {
        let blob_hash = hash.parse::<iroh_blobs::Hash>().map_err(|source| JobError::VmExecutionFailed {
            reason: format!("Invalid Hermit image blob hash '{hash}': {source}"),
        })?;
        let bytes = self
            .blob_store
            .get_bytes(&blob_hash)
            .await
            .map_err(|source| JobError::VmExecutionFailed {
                reason: format!("Failed to retrieve Hermit image blob: {source}"),
            })?
            .ok_or_else(|| JobError::VmExecutionFailed {
                reason: format!("Hermit image blob not found: {hash}"),
            })?;
        if expected_size > 0 && bytes.len() as u64 != expected_size {
            return Err(JobError::VmExecutionFailed {
                reason: format!("Hermit image blob size mismatch: expected {expected_size}, got {}", bytes.len()),
            });
        }
        if bytes.len() > MAX_HERMIT_IMAGE_SIZE {
            return Err(JobError::BinaryTooLarge {
                size_bytes: bytes.len() as u64,
                max_bytes: MAX_HERMIT_IMAGE_SIZE as u64,
            });
        }
        Ok(bytes.to_vec())
    }

    async fn write_image(bytes: &[u8]) -> Result<NamedTempFile> {
        let temp = NamedTempFile::new().map_err(|source| JobError::IoError {
            path: "<temp-hermit-image>".to_string(),
            source,
        })?;
        let path = temp.path().to_path_buf();
        let mut file = tokio::fs::File::create(&path).await.map_err(|source| JobError::IoError {
            path: path.display().to_string(),
            source,
        })?;
        file.write_all(bytes).await.map_err(|source| JobError::IoError {
            path: path.display().to_string(),
            source,
        })?;
        file.flush().await.map_err(|source| JobError::IoError {
            path: path.display().to_string(),
            source,
        })?;
        drop(file);
        Ok(temp)
    }

    async fn run_uhyve(&self, image_path: PathBuf, payload: &HermitUhyveJobPayload) -> Result<UhyveRun> {
        let timeout_secs = payload.timeout_secs.unwrap_or(DEFAULT_UHYVE_TIMEOUT_SECS).max(1);
        let limit = payload.serial_log_limit_bytes.unwrap_or(MAX_SERIAL_LOG_BYTES).min(MAX_SERIAL_LOG_BYTES);
        let started = Instant::now();
        let mut command = Command::new(&self.uhyve_command);
        command
            .env_clear()
            .arg(image_path)
            .args(&payload.boot_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let output = tokio::time::timeout(Duration::from_secs(timeout_secs), command.output())
            .await
            .map_err(|_| JobError::VmExecutionFailed {
                reason: format!("Uhyve timed out after {timeout_secs}s"),
            })?
            .map_err(|source| JobError::VmExecutionFailed {
                reason: format!("Failed to spawn Uhyve: {source}"),
            })?;
        let duration_ms = started.elapsed().as_millis() as u64;
        let stdout = bounded_utf8(&output.stdout, limit);
        let stderr = bounded_utf8(&output.stderr, limit);
        let exit_code = output.status.code().unwrap_or(-1);
        Ok(UhyveRun {
            exit_code,
            stdout,
            stderr,
            duration_ms,
            success: output.status.success(),
        })
    }

    fn receipt(payload: &HermitUhyveJobPayload, run: UhyveRun) -> serde_json::Value {
        let observed_marker = payload
            .expected_marker
            .as_deref()
            .filter(|expected| run.stdout.contains(*expected) || run.stderr.contains(*expected));
        let marker = observed_marker.unwrap_or("missing");
        serde_json::json!({
            "abi": HERMIT_UHYVE_RUNTIME_HOST_ABI,
            "engine": HERMIT_UHYVE_ENGINE,
            "artifact_hash": payload.artifact_hash,
            "image_blob_hash": payload.image_hash,
            "runner_capability": payload.runner_capability,
            "lifecycle_state": if run.success { "completed" } else { "failed" },
            "exit_status": run.exit_code,
            "duration_ms": run.duration_ms,
            "marker": marker,
            "serial_stdout": redact_secret_like(&run.stdout),
            "serial_stderr": redact_secret_like(&run.stderr),
        })
    }
}

#[async_trait]
impl Worker for HermitUhyveWorker {
    async fn execute(&self, job: Job) -> JobResult {
        match self.execute_inner(job).await {
            Ok(result) => result,
            Err(err) => JobResult::failure(err.to_string()),
        }
    }

    fn job_types(&self) -> Vec<String> {
        vec![HERMIT_UHYVE_JOB_TYPE.to_string()]
    }
}

impl HermitUhyveWorker {
    async fn execute_inner(&self, job: Job) -> Result<JobResult> {
        let payload: HermitUhyveJobPayload = serde_json::from_value(job.spec.payload.clone())
            .map_err(|source| JobError::SerializationError { source })?;
        if !payload.artifact_hash.starts_with("sha256:") {
            return Err(JobError::VmExecutionFailed {
                reason: "Hermit artifact hash must be immutable sha256 identity".to_string(),
            });
        }
        info!(artifact_hash = %payload.artifact_hash, engine = HERMIT_UHYVE_ENGINE, "executing Hermit/Uhyve job");
        let image = self.retrieve_image(&payload.image_hash, payload.image_size).await?;
        let temp = Self::write_image(&image).await?;
        let run = self.run_uhyve(temp.path().to_path_buf(), &payload).await?;
        let success = run.success;
        let receipt = Self::receipt(&payload, run);
        if success
            && payload.expected_marker.is_some()
            && receipt["marker"].as_str() != payload.expected_marker.as_deref()
        {
            return Err(JobError::VmExecutionFailed {
                reason: format!("Hermit/Uhyve proof marker missing from serial output: {receipt}"),
            });
        }
        if success {
            Ok(JobResult::success(receipt))
        } else {
            Err(JobError::VmExecutionFailed {
                reason: format!("Uhyve exited unsuccessfully: {receipt}"),
            })
        }
    }
}

struct UhyveRun {
    exit_code: i32,
    stdout: String,
    stderr: String,
    duration_ms: u64,
    success: bool,
}

fn bounded_utf8(bytes: &[u8], limit: usize) -> String {
    let len = bytes.len().min(limit);
    String::from_utf8_lossy(&bytes[..len]).to_string()
}

fn redact_secret_like(input: &str) -> String {
    let mut redacted = Vec::new();
    for token in input.split_whitespace() {
        let lower = token.to_ascii_lowercase();
        if lower.contains("secret")
            || lower.contains("token")
            || lower.contains("password")
            || lower.contains("private_key")
        {
            redacted.push("[REDACTED]".to_string());
        } else {
            redacted.push(token.to_string());
        }
    }
    redacted.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_secret_like_serial_tokens() {
        assert_eq!(redact_secret_like("ok SECRET=value token=abc"), "ok [REDACTED] [REDACTED]");
    }

    #[test]
    fn payload_defaults_to_product_marker() {
        let payload = HermitUhyveJobPayload::blob_image("hash", 1, "sha256:image");
        assert_eq!(payload.expected_marker.as_deref(), Some(HERMIT_UHYVE_RUNTIME_HOST_EXECUTED_MARKER));
    }
}
