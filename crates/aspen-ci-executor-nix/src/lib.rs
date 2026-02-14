//! Nix build executor for Aspen CI jobs.
//!
//! This crate provides the `NixBuildWorker` for executing Nix flake builds
//! and storing artifacts in the distributed blob store and Nix binary cache.
//!
//! # Features
//!
//! - Nix flake build execution with configurable timeout and sandbox settings
//! - Artifact collection with glob pattern matching
//! - NAR archive generation and upload to blob store
//! - Cache registration for distributed Nix binary cache
//! - SNIX integration for decomposed content-addressed storage
//! - Cache proxy support for using the cluster's binary cache as a substituter
//!
//! # Example
//!
//! ```ignore
//! use aspen_ci_executor_nix::{NixBuildWorker, NixBuildWorkerConfig};
//! use std::path::PathBuf;
//!
//! let config = NixBuildWorkerConfig {
//!     node_id: 1,
//!     cluster_id: "my-cluster".to_string(),
//!     output_dir: PathBuf::from("/tmp/aspen-ci/builds"),
//!     ..Default::default()
//! };
//!
//! let worker = NixBuildWorker::new(config);
//! ```

use std::io::Cursor;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use aspen_blob::prelude::*;
use aspen_cache::CacheEntry;
use aspen_cache::CacheIndex;
use aspen_ci_core::CiCoreError;
use aspen_ci_core::Result;
use aspen_ci_executor_shell::CacheProxy;
use aspen_jobs::Job;
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
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::info;
use tracing::warn;

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
/// Maximum NAR size to upload (matches MAX_BLOB_SIZE from aspen-blob).
/// Store paths larger than this are skipped to avoid memory exhaustion.
const MAX_NAR_UPLOAD_SIZE: u64 = 1_073_741_824; // 1 GB

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
            return Err(CiCoreError::InvalidConfig {
                reason: "flake_url cannot be empty".to_string(),
            });
        }

        if self.flake_url.len() > MAX_FLAKE_URL_LENGTH {
            return Err(CiCoreError::InvalidConfig {
                reason: format!("flake_url too long: {} bytes (max: {})", self.flake_url.len(), MAX_FLAKE_URL_LENGTH),
            });
        }

        if self.attribute.len() > MAX_ATTR_LENGTH {
            return Err(CiCoreError::InvalidConfig {
                reason: format!("attribute too long: {} bytes (max: {})", self.attribute.len(), MAX_ATTR_LENGTH),
            });
        }

        if self.timeout_secs > MAX_TIMEOUT_SECS {
            return Err(CiCoreError::InvalidConfig {
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

    /// Optional cache index for registering built store paths.
    /// When set, built store paths are automatically registered in the
    /// distributed Nix binary cache.
    pub cache_index: Option<Arc<dyn CacheIndex>>,

    /// SNIX blob service for decomposed content-addressed storage.
    /// When set along with directory and pathinfo services, built store paths
    /// are ingested as NAR archives directly into the SNIX storage layer.
    pub snix_blob_service: Option<Arc<dyn BlobService>>,
    /// SNIX directory service for storing directory metadata.
    pub snix_directory_service: Option<Arc<dyn DirectoryService>>,
    /// SNIX path info service for storing Nix store path metadata.
    pub snix_pathinfo_service: Option<Arc<dyn PathInfoService>>,

    /// Directory for build outputs.
    pub output_dir: PathBuf,

    /// Nix binary path (defaults to "nix").
    pub nix_binary: String,

    /// Whether to enable verbose logging.
    pub verbose: bool,

    // --- Cache Proxy Configuration (Phase 1) ---
    /// Whether to use the cluster's Nix binary cache as a substituter.
    /// When enabled, the worker starts a local HTTP proxy that bridges
    /// Nix's HTTP requests to the Aspen cache gateway over Iroh H3.
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

impl Default for NixBuildWorkerConfig {
    fn default() -> Self {
        Self {
            node_id: 0,
            cluster_id: String::new(),
            blob_store: None,
            cache_index: None,
            snix_blob_service: None,
            snix_directory_service: None,
            snix_pathinfo_service: None,
            output_dir: PathBuf::from("/tmp/aspen-ci/builds"),
            nix_binary: "nix".to_string(),
            verbose: false,
            // Cache proxy disabled by default until gateway is configured
            use_cluster_cache: false,
            iroh_endpoint: None,
            gateway_node: None,
            cache_public_key: None,
        }
    }
}

impl NixBuildWorkerConfig {
    /// Validate the configuration and log warnings for missing optional services.
    ///
    /// Returns `true` if the configuration is valid for full artifact storage,
    /// `false` if some services are missing (worker will still function but
    /// with reduced capabilities).
    ///
    /// # Warnings logged
    ///
    /// - Missing `blob_store`: Artifacts will not be uploaded to distributed storage
    /// - Missing `cache_index`: Store paths will not be registered in binary cache
    /// - Missing SNIX services: Store paths will not be ingested to SNIX storage
    pub fn validate(&self) -> bool {
        let mut all_services_available = true;

        if self.blob_store.is_none() {
            warn!(
                node_id = self.node_id,
                "NixBuildWorkerConfig: blob_store is None - CI artifacts will not be uploaded to distributed storage"
            );
            all_services_available = false;
        }

        if self.cache_index.is_none() {
            warn!(
                node_id = self.node_id,
                "NixBuildWorkerConfig: cache_index is None - store paths will not be registered in binary cache"
            );
            all_services_available = false;
        }

        // Check SNIX services (all three must be present for SNIX storage)
        let has_snix = self.snix_blob_service.is_some()
            && self.snix_directory_service.is_some()
            && self.snix_pathinfo_service.is_some();

        // Partial SNIX config is a misconfiguration (missing SNIX entirely is fine)
        if !has_snix
            && (self.snix_blob_service.is_some()
                || self.snix_directory_service.is_some()
                || self.snix_pathinfo_service.is_some())
        {
            warn!(
                node_id = self.node_id,
                has_blob_service = self.snix_blob_service.is_some(),
                has_directory_service = self.snix_directory_service.is_some(),
                has_pathinfo_service = self.snix_pathinfo_service.is_some(),
                "NixBuildWorkerConfig: partial SNIX services configured - all three services required for SNIX storage"
            );
        }

        // Check cache proxy config (all three must be present if enabled)
        if self.use_cluster_cache {
            let has_endpoint = self.iroh_endpoint.is_some();
            let has_gateway = self.gateway_node.is_some();
            let has_key = self.cache_public_key.is_some();

            if !has_endpoint || !has_gateway || !has_key {
                warn!(
                    node_id = self.node_id,
                    has_endpoint = has_endpoint,
                    has_gateway = has_gateway,
                    has_key = has_key,
                    "NixBuildWorkerConfig: use_cluster_cache enabled but missing required config. \
                     Ensure nix_cache.enabled is true and public key is stored at _system:nix-cache:public-key. \
                     Cache substituter will be disabled for builds."
                );
            } else {
                let gateway_short = self
                    .gateway_node
                    .as_ref()
                    .map(|g| g.fmt_short().to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                info!(
                    node_id = self.node_id,
                    gateway_node = %gateway_short,
                    "NixBuildWorkerConfig: cluster cache substituter enabled"
                );
            }
        }

        all_services_available
    }

    /// Check if cache proxy can be started.
    pub fn can_use_cache_proxy(&self) -> bool {
        self.use_cluster_cache
            && self.iroh_endpoint.is_some()
            && self.gateway_node.is_some()
            && self.cache_public_key.is_some()
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

        // Start cache proxy if configured
        let cache_proxy = if self.config.can_use_cache_proxy() {
            let endpoint = self.config.iroh_endpoint.as_ref().expect("validated");
            let gateway = self.config.gateway_node.expect("validated");

            match CacheProxy::start(Arc::clone(endpoint), gateway).await {
                Ok(proxy) => {
                    info!(
                        substituter_url = %proxy.substituter_url(),
                        "Started cache proxy for Nix build"
                    );
                    Some(proxy)
                }
                Err(e) => {
                    warn!(error = %e, "Failed to start cache proxy, continuing without substituter");
                    None
                }
            }
        } else {
            None
        };

        // Build the command
        let mut cmd = Command::new(&self.config.nix_binary);
        cmd.arg("build").arg(&flake_ref).arg("--out-link").arg("result").arg("--print-out-paths");

        // Add cache substituter args if proxy is running
        if let Some(ref proxy) = cache_proxy {
            let substituter_url = proxy.substituter_url();
            let public_key = self.config.cache_public_key.as_ref().expect("validated");

            // Prepend Aspen cache, with cache.nixos.org as fallback
            cmd.arg("--substituters").arg(format!("{} https://cache.nixos.org", substituter_url));

            // Include both keys for verification
            cmd.arg("--trusted-public-keys")
                .arg(format!("{} cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=", public_key));

            // Enable fallback to build from source if cache doesn't have it
            cmd.arg("--fallback");

            debug!(
                substituter = %substituter_url,
                "Configured Aspen cache as primary substituter"
            );
        }

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
        let mut child = cmd.spawn().map_err(|e| CiCoreError::NixBuildFailed {
            flake: flake_ref.clone(),
            reason: format!("Failed to spawn nix: {e}"),
        })?;

        // Capture stdout and stderr concurrently to avoid deadlock.
        // If we read them sequentially, the process can block if one pipe buffer fills
        // while we're waiting on the other.
        let stdout = child.stdout.take().expect("stdout piped");
        let stderr = child.stderr.take().expect("stderr piped");

        // Use channels to collect output from concurrent tasks
        let (stdout_tx, mut stdout_rx) = mpsc::channel::<String>(100);
        let (stderr_tx, mut stderr_rx) = mpsc::channel::<String>(1000);

        let flake_ref_clone = flake_ref.clone();
        let stdout_task = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                match reader.read_line(&mut line).await {
                    Ok(0) => break Ok(()), // EOF
                    Ok(_) => {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() && trimmed.starts_with("/nix/store/") {
                            // Ignore send errors - receiver may have dropped
                            let _ = stdout_tx.send(trimmed.to_string()).await;
                        }
                        line.clear();
                    }
                    Err(e) => {
                        break Err(CiCoreError::NixBuildFailed {
                            flake: flake_ref_clone.clone(),
                            reason: format!("Failed to read stdout: {e}"),
                        });
                    }
                }
            }
        });

        let flake_ref_clone = flake_ref.clone();
        let verbose = self.config.verbose;
        let stderr_task = tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                match reader.read_line(&mut line).await {
                    Ok(0) => break Ok(()), // EOF
                    Ok(_) => {
                        if verbose {
                            debug!(line = %line.trim(), "nix build");
                        }
                        // Ignore send errors - receiver may have dropped
                        let _ = stderr_tx.send(line.clone()).await;
                        line.clear();
                    }
                    Err(e) => {
                        break Err(CiCoreError::NixBuildFailed {
                            flake: flake_ref_clone.clone(),
                            reason: format!("Failed to read stderr: {e}"),
                        });
                    }
                }
            }
        });

        // Collect results while tasks run
        let mut output_paths = Vec::new();
        let mut log_lines = Vec::new();
        let mut log_size = 0usize;

        // Collect from both channels concurrently
        loop {
            tokio::select! {
                Some(path) = stdout_rx.recv() => {
                    output_paths.push(path);
                }
                Some(line) = stderr_rx.recv() => {
                    if log_size < MAX_LOG_SIZE {
                        log_size += line.len();
                        log_lines.push(line);
                    }
                }
                else => break, // Both channels closed
            }
        }

        // Wait for tasks to complete and check for errors
        let stdout_result = stdout_task.await.map_err(|e| CiCoreError::NixBuildFailed {
            flake: flake_ref.clone(),
            reason: format!("stdout task panicked: {e}"),
        })?;
        stdout_result?;

        let stderr_result = stderr_task.await.map_err(|e| CiCoreError::NixBuildFailed {
            flake: flake_ref.clone(),
            reason: format!("stderr task panicked: {e}"),
        })?;
        stderr_result?;

        // Wait for completion with timeout
        let timeout = Duration::from_secs(payload.timeout_secs);
        let status = tokio::time::timeout(timeout, child.wait())
            .await
            .map_err(|_| CiCoreError::Timeout {
                timeout_secs: payload.timeout_secs,
            })?
            .map_err(|e| CiCoreError::NixBuildFailed {
                flake: flake_ref.clone(),
                reason: format!("Failed to wait for nix: {e}"),
            })?;

        let log = log_lines.join("");

        if !status.success() {
            let exit_code = status.code().unwrap_or(-1);
            return Err(CiCoreError::NixBuildFailed {
                flake: flake_ref,
                reason: format!("Build failed with exit code {exit_code}\n{log}"),
            });
        }

        // Shutdown cache proxy gracefully
        if let Some(proxy) = cache_proxy {
            debug!("Shutting down cache proxy");
            proxy.shutdown().await;
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

    /// Check the NAR size of a store path using `nix path-info --json`.
    ///
    /// Returns `None` if the command fails or the output can't be parsed.
    async fn check_store_path_size(&self, store_path: &str) -> Option<u64> {
        let output = Command::new(&self.config.nix_binary)
            .args(["path-info", "--json", store_path])
            .output()
            .await
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).ok()?;
        let entry = parsed.as_array()?.first()?;
        entry.get("narSize")?.as_u64()
    }

    /// Upload store paths to the blob store as NAR archives.
    ///
    /// Uses `nix nar dump-path` to create a NAR archive of each store path,
    /// then uploads the archive to the blob store. If a cache index is configured,
    /// also registers the store path in the distributed Nix binary cache.
    async fn upload_store_paths(&self, output_paths: &[String], ci_job_id: Option<&str>) -> Vec<UploadedStorePath> {
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

            // Check NAR size before dumping to avoid OOM on large paths
            if let Some(nar_size) = self.check_store_path_size(store_path).await
                && nar_size > MAX_NAR_UPLOAD_SIZE
            {
                warn!(
                    store_path = %store_path,
                    nar_size_bytes = nar_size,
                    max_size_bytes = MAX_NAR_UPLOAD_SIZE,
                    "Skipping store path upload: NAR size exceeds maximum"
                );
                continue;
            }

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

            // Compute SHA256 hash of NAR (Nix's native format)
            use sha2::Digest;
            use sha2::Sha256;
            let nar_hash = {
                let mut hasher = Sha256::new();
                hasher.update(&nar_data);
                let hash = hasher.finalize();
                format!("sha256:{}", hex::encode(hash))
            };

            // Upload to blob store
            let blob_hash = match blob_store.add_bytes(&nar_data).await {
                Ok(result) => {
                    let blob_hash = result.blob_ref.hash.to_string();
                    info!(
                        cluster_id = %self.config.cluster_id,
                        node_id = self.config.node_id,
                        store_path = %store_path,
                        blob_hash = %blob_hash,
                        nar_size = nar_size,
                        nar_hash = %nar_hash,
                        "Store path uploaded as NAR"
                    );
                    blob_hash
                }
                Err(e) => {
                    warn!(
                        store_path = %store_path,
                        error = %e,
                        "Failed to upload NAR to blob store"
                    );
                    continue;
                }
            };

            // Register in cache index if configured
            let cache_registered = if let Some(ref cache_index) = self.config.cache_index {
                match self.register_in_cache(cache_index, store_path, &blob_hash, nar_size, &nar_hash, ci_job_id).await
                {
                    Ok(()) => {
                        info!(
                            store_path = %store_path,
                            "Store path registered in cache"
                        );
                        true
                    }
                    Err(e) => {
                        warn!(
                            store_path = %store_path,
                            error = %e,
                            "Failed to register store path in cache"
                        );
                        false
                    }
                }
            } else {
                false
            };

            uploaded.push(UploadedStorePath {
                store_path: store_path.clone(),
                blob_hash,
                nar_size,
                nar_hash,
                cache_registered,
            });
        }

        uploaded
    }

    /// Register a store path in the cache index.
    ///
    /// Queries `nix path-info --json` to get references and deriver, then
    /// creates a cache entry and stores it.
    async fn register_in_cache(
        &self,
        cache_index: &Arc<dyn CacheIndex>,
        store_path: &str,
        blob_hash: &str,
        nar_size: u64,
        nar_hash: &str,
        ci_job_id: Option<&str>,
    ) -> Result<()> {
        // Parse store path to extract hash
        let (store_hash, _name) =
            aspen_cache::parse_store_path(store_path).map_err(|e| CiCoreError::ArtifactStorage {
                reason: format!("Invalid store path: {e}"),
            })?;

        // Query nix path-info for references and deriver
        let path_info = self.query_path_info(store_path).await;

        let created_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        // Create cache entry
        let mut entry = CacheEntry::new(
            store_path.to_string(),
            store_hash,
            blob_hash.to_string(),
            nar_size,
            nar_hash.to_string(),
            created_at,
            self.config.node_id,
        );

        // Add references and deriver if available
        if let Some(info) = path_info {
            entry = entry.with_references(info.references).map_err(|e| CiCoreError::ArtifactStorage {
                reason: format!("Failed to set references: {e}"),
            })?;
            entry = entry.with_deriver(info.deriver).map_err(|e| CiCoreError::ArtifactStorage {
                reason: format!("Failed to set deriver: {e}"),
            })?;
        }

        // Add CI metadata if available
        if let Some(job_id) = ci_job_id {
            entry = entry.with_ci_metadata(Some(job_id.to_string()), None);
        }

        // Store in cache
        cache_index.put(entry).await.map_err(|e| CiCoreError::ArtifactStorage {
            reason: format!("Failed to store cache entry: {e}"),
        })?;

        Ok(())
    }

    /// Query nix path-info for a store path to get references and deriver.
    async fn query_path_info(&self, store_path: &str) -> Option<PathInfo> {
        let output = Command::new(&self.config.nix_binary)
            .args(["path-info", "--json", store_path])
            .output()
            .await
            .ok()?;

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

        Some(PathInfo { references, deriver })
    }

    /// Upload store paths to SNIX storage as NAR archives.
    ///
    /// Uses `nix nar dump-path` to create a NAR archive of each store path,
    /// then ingests directly into SNIX storage using `ingest_nar_and_hash`.
    /// Creates PathInfo entries with proper metadata.
    async fn upload_store_paths_snix(&self, output_paths: &[String]) -> Vec<UploadedStorePathSnix> {
        let mut uploaded = Vec::new();

        // Check if all SNIX services are configured
        let (blob_service, directory_service, pathinfo_service) = match (
            &self.config.snix_blob_service,
            &self.config.snix_directory_service,
            &self.config.snix_pathinfo_service,
        ) {
            (Some(bs), Some(ds), Some(ps)) => (bs, ds, ps),
            _ => {
                debug!("SNIX services not fully configured, skipping SNIX upload");
                return uploaded;
            }
        };

        for store_path in output_paths {
            info!(
                cluster_id = %self.config.cluster_id,
                node_id = self.config.node_id,
                store_path = %store_path,
                "Uploading store path to SNIX storage"
            );

            // Use nix nar dump-path to create a NAR archive
            let output =
                match Command::new(&self.config.nix_binary).args(["nar", "dump-path", store_path]).output().await {
                    Ok(output) => output,
                    Err(e) => {
                        warn!(store_path = %store_path, error = %e, "Failed to create NAR archive for SNIX");
                        continue;
                    }
                };

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!(
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
                        cluster_id = %self.config.cluster_id,
                        node_id = self.config.node_id,
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
                        store_path = %store_path,
                        error = %e,
                        "Failed to store PathInfo in SNIX"
                    );
                }
            }
        }

        uploaded
    }
}

/// Information from nix path-info.
struct PathInfo {
    /// Store paths this entry references.
    references: Vec<String>,
    /// Deriver store path.
    deriver: Option<String>,
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
pub struct UploadedStorePath {
    /// The Nix store path.
    pub store_path: String,
    /// Blob hash of the NAR archive (BLAKE3).
    pub blob_hash: String,
    /// Size of the NAR archive in bytes.
    pub nar_size: u64,
    /// SHA256 hash of the NAR archive (Nix's native format).
    pub nar_hash: String,
    /// Whether this path was registered in the cache.
    pub cache_registered: bool,
}

/// Information about an uploaded store path to SNIX storage.
#[derive(Debug, Clone, Serialize)]
pub struct UploadedStorePathSnix {
    /// The Nix store path.
    pub store_path: String,
    /// Size of the NAR archive in bytes.
    pub nar_size: u64,
    /// SHA256 hash of the NAR archive (Nix's native format).
    pub nar_sha256: String,
    /// Number of references this store path has.
    pub references_count: usize,
    /// Whether this store path has a deriver.
    pub has_deriver: bool,
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

        // Upload store paths to blob store if requested (legacy)
        let uploaded_store_paths = if payload.upload_result {
            self.upload_store_paths(&build_output.output_paths, payload.job_name.as_deref()).await
        } else {
            vec![]
        };

        // Upload store paths to SNIX storage if requested and configured
        let uploaded_store_paths_snix = if payload.upload_result {
            self.upload_store_paths_snix(&build_output.output_paths).await
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
                "uploaded_store_paths_snix": uploaded_store_paths_snix,
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
            job_name: None,
            flake_url: ".".to_string(),
            attribute: "packages.x86_64-linux.default".to_string(),
            extra_args: vec![],
            working_dir: None,
            timeout_secs: 1800,
            sandbox: true,
            cache_key: None,
            artifacts: vec![],
            upload_result: true,
        };

        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_payload_validation_empty_url() {
        let invalid = NixBuildPayload {
            job_name: None,
            flake_url: "".to_string(),
            attribute: "default".to_string(),
            extra_args: vec![],
            working_dir: None,
            timeout_secs: 1800,
            sandbox: true,
            cache_key: None,
            artifacts: vec![],
            upload_result: true,
        };

        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_payload_validation_timeout_too_long() {
        let invalid = NixBuildPayload {
            job_name: None,
            flake_url: ".".to_string(),
            attribute: "default".to_string(),
            extra_args: vec![],
            working_dir: None,
            timeout_secs: 100000, // Way too long
            sandbox: true,
            cache_key: None,
            artifacts: vec![],
            upload_result: true,
        };

        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_flake_ref() {
        let payload = NixBuildPayload {
            job_name: None,
            flake_url: "github:owner/repo".to_string(),
            attribute: "packages.x86_64-linux.default".to_string(),
            extra_args: vec![],
            working_dir: None,
            timeout_secs: 1800,
            sandbox: true,
            cache_key: None,
            artifacts: vec![],
            upload_result: true,
        };

        assert_eq!(payload.flake_ref(), "github:owner/repo#packages.x86_64-linux.default");
    }

    #[test]
    fn test_flake_ref_no_attribute() {
        let payload = NixBuildPayload {
            job_name: None,
            flake_url: ".".to_string(),
            attribute: "".to_string(),
            extra_args: vec![],
            working_dir: None,
            timeout_secs: 1800,
            sandbox: true,
            cache_key: None,
            artifacts: vec![],
            upload_result: true,
        };

        assert_eq!(payload.flake_ref(), ".");
    }
}
