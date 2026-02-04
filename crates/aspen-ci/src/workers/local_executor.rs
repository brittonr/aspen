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

use std::collections::HashMap;
use std::io;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;

use aspen_api::KeyValueStore;
use aspen_blob::BlobStore;
use aspen_cache::CacheIndex;
use aspen_core::CI_VM_DEFAULT_EXECUTION_TIMEOUT_MS;
use aspen_core::CI_VM_MAX_EXECUTION_TIMEOUT_MS;
use aspen_jobs::Job;
use aspen_jobs::JobError;
use aspen_jobs::JobOutput;
use aspen_jobs::JobResult;
use aspen_jobs::Worker;
use async_trait::async_trait;
use iroh::Endpoint;
use iroh::PublicKey;
use nix_compat::store_path::StorePath as SnixStorePath;
use serde::Deserialize;
use serde::Serialize;
use snix_castore::blobservice::BlobService;
use snix_castore::directoryservice::DirectoryService;
use snix_store::nar::ingest_nar_and_hash;
use snix_store::pathinfoservice::PathInfo as SnixPathInfo;
use snix_store::pathinfoservice::PathInfoService;
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::CacheProxy;
use super::cloud_hypervisor::artifacts::ArtifactCollectionResult;
use super::cloud_hypervisor::artifacts::ArtifactUploadResult;
use super::cloud_hypervisor::artifacts::collect_artifacts;
use super::cloud_hypervisor::artifacts::upload_artifacts_to_blob_store;
use super::cloud_hypervisor::workspace::seed_workspace_from_blob;
use crate::agent::executor::Executor;
use crate::agent::protocol::ExecutionRequest;
use crate::agent::protocol::ExecutionResult;
use crate::agent::protocol::LogMessage;

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
pub struct LocalExecutorWorkerConfig {
    /// Base workspace directory where jobs run.
    /// Each job gets a subdirectory under this path.
    pub workspace_dir: PathBuf,

    /// Whether to clean up job workspaces after completion.
    pub cleanup_workspaces: bool,

    // --- SNIX services for Nix binary cache integration ---
    /// SNIX blob service for decomposed content-addressed storage.
    /// When set along with directory and pathinfo services, built store paths
    /// are ingested as NAR archives directly into the SNIX storage layer.
    pub snix_blob_service: Option<Arc<dyn BlobService>>,

    /// SNIX directory service for storing directory metadata.
    pub snix_directory_service: Option<Arc<dyn DirectoryService>>,

    /// SNIX path info service for storing Nix store path metadata.
    pub snix_pathinfo_service: Option<Arc<dyn PathInfoService>>,

    /// Optional cache index for registering built store paths.
    /// When set, built store paths are automatically registered in the
    /// distributed Nix binary cache (legacy format).
    pub cache_index: Option<Arc<dyn CacheIndex>>,

    // --- Cache/store access ---
    /// KV store for cache metadata (Cargo cache, etc).
    pub kv_store: Option<Arc<dyn KeyValueStore>>,

    // --- Nix cache substituter configuration ---
    /// Whether to use the cluster's Nix binary cache as a substituter.
    /// When enabled, nix commands will be configured to use the cluster cache.
    pub use_cluster_cache: bool,

    /// Iroh endpoint for connecting to the cache gateway.
    /// Required when `use_cluster_cache` is true.
    pub iroh_endpoint: Option<Arc<Endpoint>>,

    /// NodeId of the nix-cache-gateway service.
    /// Required when `use_cluster_cache` is true.
    pub gateway_node: Option<PublicKey>,

    /// Trusted public key for the cache (e.g., "aspen-cache:base64key").
    /// Required when `use_cluster_cache` is true to verify signed narinfo.
    pub cache_public_key: Option<String>,
}

impl Default for LocalExecutorWorkerConfig {
    fn default() -> Self {
        Self {
            workspace_dir: PathBuf::from("/workspace"),
            cleanup_workspaces: true,
            snix_blob_service: None,
            snix_directory_service: None,
            snix_pathinfo_service: None,
            cache_index: None,
            kv_store: None,
            use_cluster_cache: false,
            iroh_endpoint: None,
            gateway_node: None,
            cache_public_key: None,
        }
    }
}

impl LocalExecutorWorkerConfig {
    /// Check if cache proxy can be started.
    ///
    /// Returns true if all required components are available:
    /// - use_cluster_cache is enabled
    /// - iroh endpoint is configured
    /// - gateway node is known
    /// - cache public key is set
    pub fn can_use_cache_proxy(&self) -> bool {
        self.use_cluster_cache
            && self.iroh_endpoint.is_some()
            && self.gateway_node.is_some()
            && self.cache_public_key.is_some()
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

    /// Command executor (from agent module).
    executor: Executor,

    /// Optional blob store for workspace seeding and artifact storage.
    blob_store: Option<Arc<dyn BlobStore>>,
}

impl LocalExecutorWorker {
    /// Create a new local executor worker.
    pub fn new(config: LocalExecutorWorkerConfig) -> Self {
        let executor = Executor::with_workspace_root(config.workspace_dir.clone());
        Self {
            config,
            executor,
            blob_store: None,
        }
    }

    /// Create a new local executor worker with a blob store.
    pub fn with_blob_store(config: LocalExecutorWorkerConfig, blob_store: Arc<dyn BlobStore>) -> Self {
        let executor = Executor::with_workspace_root(config.workspace_dir.clone());
        Self {
            config,
            executor,
            blob_store: Some(blob_store),
        }
    }

    /// Store output in blob store if large, inline if small.
    ///
    /// Outputs <= 64KB are stored inline. Larger outputs are stored in the blob
    /// store (if available) with only a hash reference kept in the job record.
    /// Falls back to truncation if blob storage fails.
    async fn store_output(&self, data: &str, job_id: &str, stream: &str) -> aspen_jobs::OutputRef {
        use aspen_jobs::INLINE_OUTPUT_THRESHOLD;
        use aspen_jobs::OutputRef;

        let bytes = data.as_bytes();

        if bytes.len() as u64 <= INLINE_OUTPUT_THRESHOLD {
            return OutputRef::Inline(data.to_string());
        }

        if let Some(ref blob_store) = self.blob_store {
            match blob_store.add_bytes(bytes).await {
                Ok(result) => {
                    info!(
                        job_id,
                        stream,
                        hash = %result.blob_ref.hash.to_hex(),
                        size = bytes.len(),
                        "Stored output in blob store"
                    );
                    return OutputRef::Blob {
                        hash: result.blob_ref.hash.to_hex().to_string(),
                        size: bytes.len() as u64,
                    };
                }
                Err(e) => {
                    warn!(job_id, stream, error = ?e, "Failed to store output in blob store");
                }
            }
        }

        // Fallback: truncate keeping tail (where errors typically appear)
        let max = INLINE_OUTPUT_THRESHOLD as usize;
        let skip = bytes.len().saturating_sub(max);
        let truncated = format!("...[{} bytes truncated]...\n{}", skip, &data[skip..]);
        OutputRef::Inline(truncated)
    }

    /// Execute a job and collect artifacts.
    ///
    /// This is the main orchestration function that coordinates:
    /// 1. Workspace setup (directory creation, checkout copying, blob seeding)
    /// 2. Cache proxy startup (for nix commands with cluster cache)
    /// 3. Command execution with log streaming
    /// 4. SNIX store path upload (for successful nix builds)
    /// 5. Artifact collection and upload
    /// 6. Workspace and proxy cleanup
    async fn execute_job(
        &self,
        job: &Job,
        payload: &LocalExecutorPayload,
    ) -> Result<(ExecutionResult, ArtifactCollectionResult, Option<ArtifactUploadResult>), String> {
        let job_id = job.id.to_string();

        // Phase 1: Set up workspace (returns workspace path and optional flake store path)
        let (job_workspace, flake_store_path) = self
            .setup_job_workspace(&job_id, payload)
            .await
            .map_err(|e| format!("workspace setup failed: {}", e))?;

        // Phase 2: Start cache proxy for nix commands if configured
        let cache_proxy = if payload.command == "nix" && self.config.can_use_cache_proxy() {
            let endpoint = self.config.iroh_endpoint.as_ref().expect("validated by can_use_cache_proxy");
            let gateway = self.config.gateway_node.expect("validated by can_use_cache_proxy");

            match CacheProxy::start(Arc::clone(endpoint), gateway).await {
                Ok(proxy) => {
                    info!(
                        job_id = %job_id,
                        substituter_url = %proxy.substituter_url(),
                        "Started cache proxy for nix command"
                    );
                    Some(proxy)
                }
                Err(e) => {
                    warn!(job_id = %job_id, error = ?e, "Failed to start cache proxy, proceeding without substituter");
                    None
                }
            }
        } else {
            None
        };

        // Phase 3: Build and execute request
        let request = self.build_execution_request(
            &job_id,
            payload,
            &job_workspace,
            flake_store_path.as_ref(),
            cache_proxy.as_ref(),
        );
        let result = self.execute_with_streaming(&job_id, request, payload).await;

        // Phase 4: Shut down cache proxy
        if let Some(proxy) = cache_proxy {
            proxy.shutdown().await;
        }

        let result = result?;

        info!(
            job_id = %job_id,
            exit_code = result.exit_code,
            duration_ms = result.duration_ms,
            "job completed"
        );

        // Phase 3: Upload nix store paths to SNIX (on successful nix builds)
        if result.exit_code == 0 && result.error.is_none() && payload.command == "nix" {
            info!(
                job_id = %job_id,
                command = %payload.command,
                "checking for SNIX upload - nix build succeeded"
            );

            // Parse output paths from nix build output
            let output_paths = self.parse_nix_output_paths(&result.stdout);
            if !output_paths.is_empty() {
                info!(
                    job_id = %job_id,
                    output_paths = ?output_paths,
                    count = output_paths.len(),
                    "found nix output paths for SNIX upload"
                );
                let uploaded = self.upload_store_paths_snix(&job_id, &output_paths).await;
                if !uploaded.is_empty() {
                    info!(
                        job_id = %job_id,
                        count = uploaded.len(),
                        "Uploaded store paths to SNIX binary cache"
                    );
                } else {
                    warn!(
                        job_id = %job_id,
                        paths_count = output_paths.len(),
                        "SNIX upload returned empty - no paths were uploaded"
                    );
                }
            } else {
                warn!(
                    job_id = %job_id,
                    stdout_len = result.stdout.len(),
                    "no nix output paths found in stdout (expected with --print-out-paths)"
                );
                debug!(
                    job_id = %job_id,
                    stdout = %result.stdout.chars().take(500).collect::<String>(),
                    "stdout preview for SNIX path parsing"
                );
            }
        } else if payload.command == "nix" {
            debug!(
                job_id = %job_id,
                exit_code = result.exit_code,
                has_error = result.error.is_some(),
                "skipping SNIX upload - nix build did not succeed"
            );
        }

        // Phase 4: Collect artifacts (only on success)
        let (artifacts, upload_result) =
            self.collect_and_upload_artifacts(&job_id, &result, payload, &job_workspace).await;

        // Phase 5: Clean up workspace
        self.cleanup_workspace(&job_id, &job_workspace).await;

        Ok((result, artifacts, upload_result))
    }

    /// Set up the job workspace directory.
    ///
    /// Creates a per-job directory, copies checkout contents if provided,
    /// pre-fetches flake inputs for nix commands, and seeds from blob store.
    ///
    /// Returns the workspace path and optionally the flake source store path
    /// (if a flake.nix was found and archived successfully).
    async fn setup_job_workspace(
        &self,
        job_id: &str,
        payload: &LocalExecutorPayload,
    ) -> Result<(PathBuf, Option<PathBuf>), String> {
        let job_workspace = self.config.workspace_dir.join(job_id);

        // Create workspace directory
        tokio::fs::create_dir_all(&job_workspace)
            .await
            .map_err(|e| format!("failed to create job workspace: {}", e))?;

        info!(job_id = %job_id, workspace = %job_workspace.display(), "created job workspace");

        // Copy checkout directory if provided and get flake store path
        let flake_store_path = if let Some(ref checkout_dir) = payload.checkout_dir {
            self.copy_checkout_to_workspace(job_id, checkout_dir, &job_workspace, payload).await
        } else {
            None
        };

        // Seed from blob store if source_hash provided
        if let Some(ref source_hash) = payload.source_hash {
            self.seed_workspace_from_source(job_id, source_hash, &job_workspace).await;
        }

        Ok((job_workspace, flake_store_path))
    }

    /// Copy checkout directory contents to workspace and pre-fetch flake inputs.
    ///
    /// Returns the flake source store path if a flake was archived successfully.
    async fn copy_checkout_to_workspace(
        &self,
        job_id: &str,
        checkout_dir: &str,
        job_workspace: &std::path::Path,
        payload: &LocalExecutorPayload,
    ) -> Option<PathBuf> {
        let checkout_path = PathBuf::from(checkout_dir);
        if !checkout_path.exists() {
            return None;
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
                return None;
            }
        }

        // Pre-fetch flake inputs for nix commands and get the flake store path
        if payload.command == "nix" && job_workspace.join("flake.nix").exists() {
            match prefetch_and_rewrite_flake_lock(job_workspace).await {
                Ok(store_path) => {
                    info!(job_id = %job_id, store_path = ?store_path, "pre-fetched flake inputs");
                    return store_path;
                }
                Err(e) => {
                    warn!(job_id = %job_id, error = ?e, "failed to pre-fetch flake");
                }
            }
        }

        None
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
    ///
    /// If `flake_store_path` is provided, flake references like `.#attr` in the command args
    /// will be rewritten to use the store path directly (e.g., `/nix/store/xxx#attr`).
    ///
    /// If `cache_proxy` is provided, the nix command will be configured to use the
    /// cluster's binary cache as a substituter.
    fn build_execution_request(
        &self,
        job_id: &str,
        payload: &LocalExecutorPayload,
        job_workspace: &std::path::Path,
        flake_store_path: Option<&PathBuf>,
        cache_proxy: Option<&CacheProxy>,
    ) -> ExecutionRequest {
        let working_dir = if payload.working_dir.starts_with('/') {
            PathBuf::from(&payload.working_dir)
        } else {
            job_workspace.join(&payload.working_dir)
        };

        let (command, mut args) = if payload.command == "nix" {
            inject_nix_flags_with_flake_rewrite(&payload.args, flake_store_path, job_id)
        } else {
            (payload.command.clone(), payload.args.clone())
        };

        // Add cache substituter args for nix commands if proxy is running
        if payload.command == "nix" {
            if let Some(proxy) = cache_proxy {
                let substituter_url = proxy.substituter_url();
                let public_key = self.config.cache_public_key.as_ref().expect("validated by can_use_cache_proxy");

                // Prepend Aspen cache, with cache.nixos.org as fallback
                args.push("--substituters".to_string());
                args.push(format!("{} https://cache.nixos.org", substituter_url));

                // Include both keys for verification
                args.push("--trusted-public-keys".to_string());
                args.push(format!("{} cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=", public_key));

                // Enable fallback to build from source if cache doesn't have it
                args.push("--fallback".to_string());

                debug!(
                    job_id = %job_id,
                    substituter = %substituter_url,
                    "Added cache substituter to nix command"
                );
            }
        }

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

    /// Upload store paths to SNIX storage as NAR archives.
    ///
    /// Uses `nix nar dump-path` to create a NAR archive of each store path,
    /// then ingests directly into SNIX storage using `ingest_nar_and_hash`.
    /// Creates PathInfo entries with proper metadata.
    ///
    /// This enables the cluster's Nix binary cache to serve built paths to
    /// other jobs and developers.
    async fn upload_store_paths_snix(&self, job_id: &str, output_paths: &[String]) -> Vec<UploadedStorePathSnix> {
        let mut uploaded = Vec::new();

        info!(
            job_id = %job_id,
            paths_count = output_paths.len(),
            "starting SNIX upload for store paths"
        );

        // Check if all SNIX services are configured
        let has_blob = self.config.snix_blob_service.is_some();
        let has_dir = self.config.snix_directory_service.is_some();
        let has_pathinfo = self.config.snix_pathinfo_service.is_some();

        debug!(
            job_id = %job_id,
            has_blob_service = has_blob,
            has_directory_service = has_dir,
            has_pathinfo_service = has_pathinfo,
            "SNIX service configuration check"
        );

        let (blob_service, directory_service, pathinfo_service) = match (
            &self.config.snix_blob_service,
            &self.config.snix_directory_service,
            &self.config.snix_pathinfo_service,
        ) {
            (Some(bs), Some(ds), Some(ps)) => {
                info!(job_id = %job_id, "all SNIX services configured, proceeding with upload");
                (bs, ds, ps)
            }
            _ => {
                warn!(
                    job_id = %job_id,
                    has_blob_service = has_blob,
                    has_directory_service = has_dir,
                    has_pathinfo_service = has_pathinfo,
                    "SNIX services not fully configured, skipping SNIX upload"
                );
                return uploaded;
            }
        };

        for store_path in output_paths {
            info!(
                job_id = %job_id,
                store_path = %store_path,
                "Uploading store path to SNIX storage"
            );

            // Use nix nar dump-path to create a NAR archive
            let output = match Command::new("nix").args(["nar", "dump-path", store_path]).output().await {
                Ok(output) => output,
                Err(e) => {
                    warn!(job_id = %job_id, store_path = %store_path, error = %e, "Failed to create NAR archive for SNIX");
                    continue;
                }
            };

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!(
                    job_id = %job_id,
                    store_path = %store_path,
                    stderr = %stderr,
                    "nix nar dump-path failed for SNIX upload"
                );
                continue;
            }

            let nar_data = output.stdout;
            let nar_size = nar_data.len() as u64;

            // Ingest NAR into SNIX storage
            let mut nar_reader = Cursor::new(&nar_data);
            let (root_node, nar_sha256, actual_nar_size) = match ingest_nar_and_hash(
                Arc::clone(blob_service),
                Arc::clone(directory_service),
                &mut nar_reader,
                &None, // No expected CA hash
            )
            .await
            {
                Ok((node, hash, size)) => (node, hash, size),
                Err(e) => {
                    warn!(
                        job_id = %job_id,
                        store_path = %store_path,
                        error = %e,
                        "Failed to ingest NAR into SNIX storage"
                    );
                    continue;
                }
            };

            // Verify size matches
            if actual_nar_size != nar_size {
                warn!(
                    job_id = %job_id,
                    store_path = %store_path,
                    expected_size = nar_size,
                    actual_size = actual_nar_size,
                    "NAR size mismatch after SNIX ingestion"
                );
                continue;
            }

            // Parse the store path
            let store_path_parsed: SnixStorePath<String> = match SnixStorePath::from_bytes(store_path.as_bytes()) {
                Ok(sp) => sp,
                Err(e) => {
                    warn!(
                        job_id = %job_id,
                        store_path = %store_path,
                        error = %e,
                        "Failed to parse store path for SNIX"
                    );
                    continue;
                }
            };

            // Query nix path-info for references and deriver
            let path_info_extra = self.query_path_info(store_path).await;

            // Create PathInfo for SNIX
            let path_info = SnixPathInfo {
                store_path: store_path_parsed.to_owned(),
                node: root_node,
                references: path_info_extra
                    .as_ref()
                    .map(|info| {
                        info.references.iter().filter_map(|r| SnixStorePath::from_bytes(r.as_bytes()).ok()).collect()
                    })
                    .unwrap_or_default(),
                nar_size: actual_nar_size,
                nar_sha256,
                signatures: vec![], // No signatures for CI builds
                deriver: path_info_extra
                    .as_ref()
                    .and_then(|info| info.deriver.as_ref().and_then(|d| SnixStorePath::from_bytes(d.as_bytes()).ok())),
                ca: None, // No content addressing for CI builds
            };

            // Store PathInfo in SNIX
            match pathinfo_service.put(path_info).await {
                Ok(stored_path_info) => {
                    info!(
                        job_id = %job_id,
                        store_path = %store_path,
                        nar_size = actual_nar_size,
                        nar_sha256 = hex::encode(nar_sha256),
                        "Store path uploaded to SNIX storage successfully"
                    );

                    uploaded.push(UploadedStorePathSnix {
                        store_path: store_path.clone(),
                        nar_size: actual_nar_size,
                        nar_sha256: hex::encode(nar_sha256),
                        references_count: stored_path_info.references.len(),
                        has_deriver: stored_path_info.deriver.is_some(),
                    });
                }
                Err(e) => {
                    warn!(
                        job_id = %job_id,
                        store_path = %store_path,
                        error = %e,
                        "Failed to store PathInfo in SNIX"
                    );
                }
            }
        }

        uploaded
    }

    /// Query nix path-info for a store path to get references and deriver.
    async fn query_path_info(&self, store_path: &str) -> Option<NixPathInfo> {
        let output = Command::new("nix").args(["path-info", "--json", store_path]).output().await.ok()?;

        if !output.status.success() {
            debug!(
                store_path = %store_path,
                stderr = %String::from_utf8_lossy(&output.stderr),
                "nix path-info failed"
            );
            return None;
        }

        // Parse JSON output - nix path-info --json returns an array
        let json_str = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).ok()?;

        // Get the first (and typically only) entry
        let entry = parsed.as_array()?.first()?;

        let references: Vec<String> = entry
            .get("references")?
            .as_array()?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        let deriver = entry.get("deriver").and_then(|v| v.as_str()).map(|s| s.to_string());

        Some(NixPathInfo { references, deriver })
    }

    /// Parse nix build output to extract store paths.
    ///
    /// Nix build with --print-out-paths outputs one store path per line.
    /// This method extracts valid /nix/store/* paths from the output.
    fn parse_nix_output_paths(&self, stdout: &str) -> Vec<String> {
        stdout
            .lines()
            .filter(|line| line.starts_with("/nix/store/"))
            .map(|line| line.trim().to_string())
            .collect()
    }
}

/// Information from nix path-info.
struct NixPathInfo {
    /// Store paths this entry references.
    references: Vec<String>,
    /// Deriver store path.
    deriver: Option<String>,
}

/// Information about an uploaded store path to SNIX storage.
#[derive(Debug, Clone, Serialize)]
struct UploadedStorePathSnix {
    /// The Nix store path.
    store_path: String,
    /// Size of the NAR archive in bytes.
    nar_size: u64,
    /// SHA256 hash of the NAR archive (Nix's native format).
    nar_sha256: String,
    /// Number of references (dependencies).
    references_count: usize,
    /// Whether this path has a deriver.
    has_deriver: bool,
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

                    // Store outputs in blob store if large, inline if small.
                    // Large outputs (>64KB) are stored in blobs with only a hash reference
                    // kept in the job record. This avoids the 1MB KV store value limit.
                    let stdout_ref = self.store_output(&result.stdout, &job_id, "stdout").await;
                    let stderr_ref = self.store_output(&result.stderr, &job_id, "stderr").await;

                    let output = JobOutput {
                        data: serde_json::json!({
                            "exit_code": result.exit_code,
                            "stdout": stdout_ref,
                            "stderr": stderr_ref,
                            "stdout_full_size": result.stdout.len(),
                            "stderr_full_size": result.stderr.len(),
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

/// Inject nix flags for offline execution and optionally rewrite flake references.
///
/// If `flake_store_path` is provided, flake references like `.#attr` in the args
/// will be rewritten to use the store path directly (e.g., `/nix/store/xxx#attr`).
/// This avoids Nix trying to copy the workspace to the store, which can fail
/// on read-only overlay filesystems.
fn inject_nix_flags_with_flake_rewrite(
    args: &[String],
    flake_store_path: Option<&PathBuf>,
    job_id: &str,
) -> (String, Vec<String>) {
    let mut nix_args = args.to_vec();

    // Rewrite flake references to use the pre-archived store path
    if let Some(store_path) = flake_store_path {
        for arg in &mut nix_args {
            if arg.starts_with(".#") {
                // .#attr -> /nix/store/xxx#attr
                let attr = arg[2..].to_string();
                let new_arg = format!("{}#{}", store_path.display(), attr);
                debug!(job_id = %job_id, store_path = %store_path.display(), attr = %attr, "rewrote flake reference");
                *arg = new_arg;
            } else if arg == "." {
                // . -> /nix/store/xxx
                let new_arg = store_path.display().to_string();
                debug!(job_id = %job_id, store_path = %store_path.display(), "rewrote bare flake reference");
                *arg = new_arg;
            }
        }
    }

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
            insert_pos += 1;
        }

        // Redirect lock file to /tmp to avoid read-only filesystem errors.
        // Even with --no-write-lock-file, Nix tries to open the lock file for
        // process synchronization (flock), which fails on read-only paths.
        // This redirects the lock file to a writable location.
        if !nix_args.iter().any(|a| a == "--output-lock-file") {
            nix_args.insert(insert_pos, "--output-lock-file".to_string());
            nix_args.insert(insert_pos + 1, "/tmp/flake.lock".to_string());
        }
    }

    ("nix".to_string(), nix_args)
}

/// Inject nix flags for offline execution (without flake rewriting).
#[allow(dead_code)]
fn inject_nix_flags(args: &[String]) -> (String, Vec<String>) {
    inject_nix_flags_with_flake_rewrite(args, None, "")
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
///
/// Returns the store path of the flake source itself (from archive output `path` field).
/// This can be used to rewrite `.#attr` references to `/nix/store/xxx#attr` to avoid
/// Nix trying to copy the workspace to the store (which fails on read-only overlay).
async fn prefetch_and_rewrite_flake_lock(workspace: &std::path::Path) -> io::Result<Option<PathBuf>> {
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

    // Extract the root flake store path from archive output.
    let flake_store_path = archive_json.get("path").and_then(|v| v.as_str()).map(PathBuf::from);

    rewrite_flake_lock_for_offline(workspace, &input_paths)?;

    // Sync to ensure virtiofsd sees the changes
    let _ = Command::new("sync").output().await;

    Ok(flake_store_path)
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
