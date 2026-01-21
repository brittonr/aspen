//! Nix build worker for CI/CD.
//!
//! This worker executes Nix flake builds and stores artifacts in the blob store.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use aspen_blob::BlobStore;
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command;
use tracing::debug;
use tracing::info;
use tracing::warn;

use aspen_jobs::Job;
use aspen_jobs::JobOutput;
use aspen_jobs::JobResult;
use aspen_jobs::Worker;

use crate::error::CiError;
use crate::error::Result;

// Tiger Style: All limits explicit and bounded
/// Maximum flake URL length.
const MAX_FLAKE_URL_LENGTH: usize = 4096;
/// Maximum attribute path length.
const MAX_ATTR_LENGTH: usize = 1024;
/// Maximum build log size to capture inline (64 KB).
const INLINE_LOG_THRESHOLD: usize = 64 * 1024;
/// Maximum total log size (10 MB).
const MAX_LOG_SIZE: usize = 10 * 1024 * 1024;
/// Default build timeout (30 minutes).
const DEFAULT_TIMEOUT_SECS: u64 = 1800;
/// Maximum build timeout (4 hours).
const MAX_TIMEOUT_SECS: u64 = 14400;

/// Job payload for Nix builds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NixBuildPayload {
    /// CI job name for status tracking.
    #[serde(default)]
    pub job_name: Option<String>,

    /// Flake URL (e.g., ".", "github:owner/repo", "path:/some/path").
    pub flake_url: String,

    /// Attribute path within the flake (e.g., "packages.x86_64-linux.default").
    pub attribute: String,

    /// Extra arguments to pass to `nix build`.
    #[serde(default)]
    pub extra_args: Vec<String>,

    /// Working directory for the build.
    #[serde(default)]
    pub working_dir: Option<PathBuf>,

    /// Build timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Whether to use sandbox mode.
    #[serde(default = "default_true")]
    pub sandbox: bool,

    /// Cache key for build caching.
    #[serde(default)]
    pub cache_key: Option<String>,

    /// Glob patterns for artifacts to collect.
    #[serde(default)]
    pub artifacts: Vec<String>,

    /// Whether to upload build results to the blob store as NAR archives.
    /// Defaults to true when a blob store is configured.
    #[serde(default = "default_true")]
    pub upload_result: bool,
}

fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_SECS
}

fn default_true() -> bool {
    true
}

impl NixBuildPayload {
    /// Validate the payload.
    pub fn validate(&self) -> Result<()> {
        if self.flake_url.is_empty() {
            return Err(CiError::InvalidConfig {
                reason: "flake_url cannot be empty".to_string(),
            });
        }

        if self.flake_url.len() > MAX_FLAKE_URL_LENGTH {
            return Err(CiError::InvalidConfig {
                reason: format!("flake_url too long: {} bytes (max: {})", self.flake_url.len(), MAX_FLAKE_URL_LENGTH),
            });
        }

        if self.attribute.len() > MAX_ATTR_LENGTH {
            return Err(CiError::InvalidConfig {
                reason: format!("attribute too long: {} bytes (max: {})", self.attribute.len(), MAX_ATTR_LENGTH),
            });
        }

        if self.timeout_secs > MAX_TIMEOUT_SECS {
            return Err(CiError::InvalidConfig {
                reason: format!("timeout too long: {} seconds (max: {})", self.timeout_secs, MAX_TIMEOUT_SECS),
            });
        }

        Ok(())
    }

    /// Build the flake reference string.
    pub fn flake_ref(&self) -> String {
        if self.attribute.is_empty() {
            self.flake_url.clone()
        } else {
            format!("{}#{}", self.flake_url, self.attribute)
        }
    }
}

/// Configuration for NixBuildWorker.
pub struct NixBuildWorkerConfig {
    /// Node ID for logging/metrics.
    pub node_id: u64,

    /// Cluster ID (cookie) for identifying the cluster.
    pub cluster_id: String,

    /// Optional blob store for artifact storage.
    pub blob_store: Option<Arc<dyn BlobStore>>,

    /// Directory for build outputs.
    pub output_dir: PathBuf,

    /// Nix binary path (defaults to "nix").
    pub nix_binary: String,

    /// Whether to enable verbose logging.
    pub verbose: bool,
}

impl Default for NixBuildWorkerConfig {
    fn default() -> Self {
        Self {
            node_id: 0,
            cluster_id: String::new(),
            blob_store: None,
            output_dir: PathBuf::from("/tmp/aspen-ci/builds"),
            nix_binary: "nix".to_string(),
            verbose: false,
        }
    }
}

/// Worker that executes Nix flake builds.
///
/// This worker:
/// 1. Validates the build payload
/// 2. Executes `nix build` with the specified flake reference
/// 3. Captures build logs
/// 4. Optionally stores artifacts in the blob store
/// 5. Returns build output paths and artifact hashes
pub struct NixBuildWorker {
    config: NixBuildWorkerConfig,
}

impl NixBuildWorker {
    /// Create a new Nix build worker with the given configuration.
    pub fn new(config: NixBuildWorkerConfig) -> Self {
        Self { config }
    }

    /// Create a worker with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(NixBuildWorkerConfig::default())
    }

    /// Execute a Nix build.
    async fn execute_build(&self, payload: &NixBuildPayload) -> Result<NixBuildOutput> {
        payload.validate()?;

        let flake_ref = payload.flake_ref();
        info!(
            cluster_id = %self.config.cluster_id,
            node_id = self.config.node_id,
            flake_ref = %flake_ref,
            "Starting Nix build"
        );

        // Build the command
        let mut cmd = Command::new(&self.config.nix_binary);
        cmd.arg("build").arg(&flake_ref).arg("--out-link").arg("result").arg("--print-out-paths");

        // Add sandbox flag
        if payload.sandbox {
            cmd.arg("--sandbox");
        } else {
            cmd.arg("--no-sandbox");
        }

        // Add verbose flag if configured
        if self.config.verbose {
            cmd.arg("-L"); // Print build logs
        }

        // Add extra arguments
        for arg in &payload.extra_args {
            cmd.arg(arg);
        }

        // Set working directory
        if let Some(ref dir) = payload.working_dir {
            cmd.current_dir(dir);
        }

        // Capture output
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn process
        let mut child = cmd.spawn().map_err(|e| CiError::NixBuildFailed {
            flake: flake_ref.clone(),
            reason: format!("Failed to spawn nix: {e}"),
        })?;

        // Capture stdout and stderr
        let stdout = child.stdout.take().expect("stdout piped");
        let stderr = child.stderr.take().expect("stderr piped");

        let mut stdout_reader = BufReader::new(stdout);
        let mut stderr_reader = BufReader::new(stderr);

        let mut output_paths = Vec::new();
        let mut log_lines = Vec::new();
        let mut log_size = 0usize;

        // Read stdout for output paths
        let mut line = String::new();
        while stdout_reader.read_line(&mut line).await.map_err(|e| CiError::NixBuildFailed {
            flake: flake_ref.clone(),
            reason: format!("Failed to read stdout: {e}"),
        })? > 0
        {
            let trimmed = line.trim();
            if !trimmed.is_empty() && trimmed.starts_with("/nix/store/") {
                output_paths.push(trimmed.to_string());
            }
            line.clear();
        }

        // Read stderr for build logs
        let mut line = String::new();
        while stderr_reader.read_line(&mut line).await.map_err(|e| CiError::NixBuildFailed {
            flake: flake_ref.clone(),
            reason: format!("Failed to read stderr: {e}"),
        })? > 0
        {
            if log_size < MAX_LOG_SIZE {
                log_lines.push(line.clone());
                log_size += line.len();
            }
            if self.config.verbose {
                debug!(line = %line.trim(), "nix build");
            }
            line.clear();
        }

        // Wait for completion with timeout
        let timeout = Duration::from_secs(payload.timeout_secs);
        let status = tokio::time::timeout(timeout, child.wait())
            .await
            .map_err(|_| CiError::Timeout {
                timeout_secs: payload.timeout_secs,
            })?
            .map_err(|e| CiError::NixBuildFailed {
                flake: flake_ref.clone(),
                reason: format!("Failed to wait for nix: {e}"),
            })?;

        let log = log_lines.join("");

        if !status.success() {
            let exit_code = status.code().unwrap_or(-1);
            return Err(CiError::NixBuildFailed {
                flake: flake_ref,
                reason: format!("Build failed with exit code {exit_code}\n{log}"),
            });
        }

        info!(
            cluster_id = %self.config.cluster_id,
            node_id = self.config.node_id,
            output_paths = ?output_paths,
            "Nix build completed successfully"
        );

        Ok(NixBuildOutput {
            output_paths,
            log,
            log_truncated: log_size >= MAX_LOG_SIZE,
        })
    }

    /// Collect artifacts matching the specified patterns.
    async fn collect_artifacts(&self, output_paths: &[String], patterns: &[String]) -> Result<Vec<CollectedArtifact>> {
        let mut artifacts = Vec::new();

        for output_path in output_paths {
            let path = PathBuf::from(output_path);

            for pattern in patterns {
                // Use glob to match files
                let glob_pattern = if pattern.starts_with('/') {
                    pattern.clone()
                } else {
                    format!("{}/{}", output_path, pattern)
                };

                match glob::glob(&glob_pattern) {
                    Ok(entries) => {
                        for entry in entries.flatten() {
                            if entry.is_file() {
                                let artifact = CollectedArtifact {
                                    path: entry.clone(),
                                    relative_path: entry.strip_prefix(&path).unwrap_or(&entry).to_path_buf(),
                                    blob_hash: None, // Will be set after upload
                                };
                                artifacts.push(artifact);
                            }
                        }
                    }
                    Err(e) => {
                        warn!(pattern = %glob_pattern, error = %e, "Failed to glob artifacts");
                    }
                }
            }
        }

        // Upload to blob store if configured
        if let Some(ref blob_store) = self.config.blob_store {
            for artifact in &mut artifacts {
                match tokio::fs::read(&artifact.path).await {
                    Ok(data) => match blob_store.add_bytes(&data).await {
                        Ok(result) => {
                            artifact.blob_hash = Some(result.blob_ref.hash.to_string());
                            debug!(
                                path = ?artifact.path,
                                hash = ?artifact.blob_hash,
                                "Uploaded artifact to blob store"
                            );
                        }
                        Err(e) => {
                            warn!(path = ?artifact.path, error = %e, "Failed to upload artifact");
                        }
                    },
                    Err(e) => {
                        warn!(path = ?artifact.path, error = %e, "Failed to read artifact");
                    }
                }
            }
        }

        Ok(artifacts)
    }

    /// Upload store paths to the blob store as NAR archives.
    ///
    /// Uses `nix nar dump-path` to create a NAR archive of each store path,
    /// then uploads the archive to the blob store.
    async fn upload_store_paths(&self, output_paths: &[String]) -> Vec<UploadedStorePath> {
        let mut uploaded = Vec::new();

        let Some(ref blob_store) = self.config.blob_store else {
            debug!("No blob store configured, skipping store path upload");
            return uploaded;
        };

        for store_path in output_paths {
            info!(
                cluster_id = %self.config.cluster_id,
                node_id = self.config.node_id,
                store_path = %store_path,
                "Uploading store path as NAR"
            );

            // Use nix nar dump-path to create a NAR archive
            let output =
                match Command::new(&self.config.nix_binary).args(["nar", "dump-path", store_path]).output().await {
                    Ok(output) => output,
                    Err(e) => {
                        warn!(store_path = %store_path, error = %e, "Failed to create NAR archive");
                        continue;
                    }
                };

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!(
                    store_path = %store_path,
                    stderr = %stderr,
                    "nix nar dump-path failed"
                );
                continue;
            }

            let nar_data = output.stdout;
            let nar_size = nar_data.len() as u64;

            // Upload to blob store
            match blob_store.add_bytes(&nar_data).await {
                Ok(result) => {
                    let blob_hash = result.blob_ref.hash.to_string();
                    info!(
                        cluster_id = %self.config.cluster_id,
                        node_id = self.config.node_id,
                        store_path = %store_path,
                        blob_hash = %blob_hash,
                        nar_size = nar_size,
                        "Store path uploaded as NAR"
                    );
                    uploaded.push(UploadedStorePath {
                        store_path: store_path.clone(),
                        blob_hash,
                        nar_size,
                    });
                }
                Err(e) => {
                    warn!(
                        store_path = %store_path,
                        error = %e,
                        "Failed to upload NAR to blob store"
                    );
                }
            }
        }

        uploaded
    }
}

/// Output from a Nix build.
#[derive(Debug, Clone)]
struct NixBuildOutput {
    /// Paths to build outputs in /nix/store.
    output_paths: Vec<String>,
    /// Build log.
    log: String,
    /// Whether the log was truncated.
    log_truncated: bool,
}

/// Information about an uploaded store path.
#[derive(Debug, Clone, Serialize)]
struct UploadedStorePath {
    /// The Nix store path.
    store_path: String,
    /// Blob hash of the NAR archive.
    blob_hash: String,
    /// Size of the NAR archive in bytes.
    nar_size: u64,
}

/// A collected artifact.
#[derive(Debug, Clone)]
struct CollectedArtifact {
    /// Full path to the artifact.
    path: PathBuf,
    /// Path relative to output directory.
    relative_path: PathBuf,
    /// Blob hash if uploaded.
    blob_hash: Option<String>,
}

#[async_trait]
impl Worker for NixBuildWorker {
    fn job_types(&self) -> Vec<String> {
        vec!["ci_nix_build".into()]
    }

    async fn execute(&self, job: Job) -> JobResult {
        // Parse payload
        let payload: NixBuildPayload = match serde_json::from_value(job.spec.payload.clone()) {
            Ok(p) => p,
            Err(e) => {
                return JobResult::failure(format!("Invalid NixBuildPayload: {e}"));
            }
        };

        // Execute build
        let build_output = match self.execute_build(&payload).await {
            Ok(output) => output,
            Err(e) => {
                return JobResult::failure(format!("Nix build failed: {e}"));
            }
        };

        // Upload store paths to blob store if requested
        let uploaded_store_paths = if payload.upload_result {
            self.upload_store_paths(&build_output.output_paths).await
        } else {
            vec![]
        };

        // Collect artifacts
        let artifacts = match self.collect_artifacts(&build_output.output_paths, &payload.artifacts).await {
            Ok(a) => a,
            Err(e) => {
                warn!(error = %e, "Failed to collect artifacts");
                vec![]
            }
        };

        // Build output JSON
        let artifact_info: Vec<serde_json::Value> = artifacts
            .iter()
            .map(|a| {
                serde_json::json!({
                    "path": a.relative_path.display().to_string(),
                    "blob_hash": a.blob_hash,
                })
            })
            .collect();

        // Truncate log if too large for inline storage
        let log = if build_output.log.len() > INLINE_LOG_THRESHOLD {
            format!(
                "{}...\n[Log truncated, {} bytes total]",
                &build_output.log[..INLINE_LOG_THRESHOLD],
                build_output.log.len()
            )
        } else {
            build_output.log
        };

        JobResult::Success(JobOutput {
            data: serde_json::json!({
                "output_paths": build_output.output_paths,
                "uploaded_store_paths": uploaded_store_paths,
                "artifacts": artifact_info,
                "log_truncated": build_output.log_truncated,
                "built_by_node": self.config.node_id,
                "cluster_id": self.config.cluster_id,
            }),
            metadata: [
                ("build_log".to_string(), log),
                ("node_id".to_string(), self.config.node_id.to_string()),
                ("cluster_id".to_string(), self.config.cluster_id.clone()),
            ]
            .into_iter()
            .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_validation() {
        let valid = NixBuildPayload {
            flake_url: ".".to_string(),
            attribute: "packages.x86_64-linux.default".to_string(),
            extra_args: vec![],
            working_dir: None,
            timeout_secs: 1800,
            sandbox: true,
            cache_key: None,
            artifacts: vec![],
        };

        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_payload_validation_empty_url() {
        let invalid = NixBuildPayload {
            flake_url: "".to_string(),
            attribute: "default".to_string(),
            extra_args: vec![],
            working_dir: None,
            timeout_secs: 1800,
            sandbox: true,
            cache_key: None,
            artifacts: vec![],
        };

        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_payload_validation_timeout_too_long() {
        let invalid = NixBuildPayload {
            flake_url: ".".to_string(),
            attribute: "default".to_string(),
            extra_args: vec![],
            working_dir: None,
            timeout_secs: 100000, // Way too long
            sandbox: true,
            cache_key: None,
            artifacts: vec![],
        };

        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_flake_ref() {
        let payload = NixBuildPayload {
            flake_url: "github:owner/repo".to_string(),
            attribute: "packages.x86_64-linux.default".to_string(),
            extra_args: vec![],
            working_dir: None,
            timeout_secs: 1800,
            sandbox: true,
            cache_key: None,
            artifacts: vec![],
        };

        assert_eq!(payload.flake_ref(), "github:owner/repo#packages.x86_64-linux.default");
    }

    #[test]
    fn test_flake_ref_no_attribute() {
        let payload = NixBuildPayload {
            flake_url: ".".to_string(),
            attribute: "".to_string(),
            extra_args: vec![],
            working_dir: None,
            timeout_secs: 1800,
            sandbox: true,
            cache_key: None,
            artifacts: vec![],
        };

        assert_eq!(payload.flake_ref(), ".");
    }
}
