//! Hyperlight micro-VM worker implementation.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use aspen_blob::prelude::*;
use async_trait::async_trait;
use hyperlight_host::GuestBinary;
use hyperlight_host::MultiUseSandbox;
use hyperlight_host::UninitializedSandbox;
use hyperlight_host::sandbox::SandboxConfiguration;
use tempfile::NamedTempFile;
use tokio::fs;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::sync::Mutex;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::error::JobError;
use crate::error::Result;
use crate::job::Job;
use crate::job::JobResult;
use crate::vm_executor::types::JobPayload;
use crate::vm_executor::types::NixBuildOutput;
use crate::worker::Worker;

/// Maximum size for a built binary (50MB).
const MAX_BINARY_SIZE: usize = 50 * 1024 * 1024;

/// Default timeout for Nix builds (10 minutes).
const NIX_BUILD_TIMEOUT: Duration = Duration::from_secs(600);

/// Worker that executes jobs in Hyperlight micro-VMs.
pub struct HyperlightWorker {
    /// Blob store for retrieving VM binaries.
    blob_store: Arc<dyn BlobStore>,
    /// Cache of built binaries (flake_url -> blob hash).
    /// Using Mutex for interior mutability since Worker trait expects &self.
    build_cache: Mutex<HashMap<String, String>>,
}

impl HyperlightWorker {
    /// Create a new Hyperlight worker.
    /// Requires a blob store for retrieving VM binaries.
    pub fn new(blob_store: Arc<dyn BlobStore>) -> Result<Self> {
        Ok(Self {
            blob_store,
            build_cache: Mutex::new(HashMap::new()),
        })
    }

    /// Retrieve a binary from the blob store.
    async fn retrieve_binary_from_blob(&self, hash: &str, expected_size: u64, format: &str) -> Result<Vec<u8>> {
        info!("Retrieving binary from blob store: hash={}, format={}", hash, format);

        // Parse the hash
        let blob_hash = hash.parse::<iroh_blobs::Hash>().map_err(|e| JobError::VmExecutionFailed {
            reason: format!("Invalid blob hash '{}': {}", hash, e),
        })?;

        // Retrieve from blob store
        let bytes = self
            .blob_store
            .get_bytes(&blob_hash)
            .await
            .map_err(|e| JobError::VmExecutionFailed {
                reason: format!("Failed to retrieve blob: {}", e),
            })?
            .ok_or_else(|| JobError::VmExecutionFailed {
                reason: format!("Blob not found: {}", hash),
            })?;

        // Validate size if provided (0 means skip validation)
        if expected_size > 0 && bytes.len() as u64 != expected_size {
            return Err(JobError::VmExecutionFailed {
                reason: format!("Blob size mismatch: expected {}, got {}", expected_size, bytes.len()),
            });
        }

        // Validate it's not too large
        if bytes.len() > MAX_BINARY_SIZE {
            return Err(JobError::BinaryTooLarge {
                size_bytes: bytes.len() as u64,
                max_bytes: MAX_BINARY_SIZE as u64,
            });
        }

        Ok(bytes.to_vec())
    }

    /// Execute a nix build command and wait for completion with timeout.
    ///
    /// Returns the build output on success, or an error with collected stderr.
    async fn build_from_nix_execute(flake_url: &str, cache_key: &str) -> Result<std::process::Output> {
        // Run nix build command with streaming stderr and timeout
        let mut child = tokio::process::Command::new("nix")
            .args(["build", cache_key])
            .args(["--json", "--no-link"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| JobError::BuildFailed {
                reason: format!("Failed to spawn nix build: {}", e),
            })?;

        // Stream stderr for build progress while waiting for completion
        let stderr = child.stderr.take().ok_or_else(|| JobError::BuildFailed {
            reason: "stderr not available after spawn".to_string(),
        })?;
        let mut stderr_lines = BufReader::new(stderr).lines();

        // Spawn task to log build progress
        let flake_for_log = flake_url.to_string();
        let stderr_task = tokio::spawn(async move {
            let mut collected_stderr = String::new();
            while let Ok(Some(line)) = stderr_lines.next_line().await {
                debug!(flake_url = %flake_for_log, build_output = %line, "nix build progress");
                collected_stderr.push_str(&line);
                collected_stderr.push('\n');
            }
            collected_stderr
        });

        // Wait for build with timeout
        let wait_result = tokio::time::timeout(NIX_BUILD_TIMEOUT, child.wait_with_output()).await;

        let output = match wait_result {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return Err(JobError::BuildFailed {
                    reason: format!("Failed to wait for nix build: {}", e),
                });
            }
            Err(_) => {
                // Timeout - try to kill the process
                warn!(flake_url, timeout_secs = NIX_BUILD_TIMEOUT.as_secs(), "Nix build timed out");
                return Err(JobError::BuildFailed {
                    reason: format!("Nix build timed out after {} seconds", NIX_BUILD_TIMEOUT.as_secs()),
                });
            }
        };

        // Wait for stderr collection to complete
        let collected_stderr = stderr_task.await.unwrap_or_default();

        if !output.status.success() {
            // Use collected stderr if available, otherwise fall back to output.stderr
            let stderr_msg = if collected_stderr.is_empty() {
                String::from_utf8_lossy(&output.stderr).to_string()
            } else {
                collected_stderr
            };
            return Err(JobError::BuildFailed {
                reason: format!("Nix build failed: {}", stderr_msg),
            });
        }

        Ok(output)
    }

    /// Read the built binary from a Nix store path.
    ///
    /// Locates the binary in the store path's bin/ directory and validates size.
    async fn build_from_nix_read_binary(store_path: &str) -> Result<Vec<u8>> {
        // Try to find the binary in the store path
        let binary_path = format!("{}/bin/*", store_path);
        let glob_pattern = glob::glob(&binary_path).map_err(|e| JobError::BuildFailed {
            reason: format!("Invalid glob pattern: {}", e),
        })?;

        let binary_file = glob_pattern.filter_map(|p| p.ok()).next().ok_or_else(|| JobError::BuildFailed {
            reason: format!("No binary found in {}/bin/", store_path),
        })?;

        // Read the binary
        let binary = fs::read(&binary_file).await.map_err(|e| JobError::BuildFailed {
            reason: format!("Failed to read binary: {}", e),
        })?;

        // Check size
        if binary.len() > MAX_BINARY_SIZE {
            return Err(JobError::BinaryTooLarge {
                size_bytes: binary.len() as u64,
                max_bytes: MAX_BINARY_SIZE as u64,
            });
        }

        Ok(binary)
    }

    /// Build a binary from a Nix flake.
    async fn build_from_nix(&self, flake_url: &str, attribute: &str) -> Result<Vec<u8>> {
        let cache_key = format!("{}#{}", flake_url, attribute);

        // Check cache first (now stores blob hashes)
        let cached_hash = {
            let cache = self.build_cache.lock().await;
            cache.get(&cache_key).cloned()
        };

        if let Some(hash) = cached_hash {
            debug!(flake_url, attribute, "Using cached binary from blob store");
            // Size 0 means we don't validate (we trust our cache)
            return self.retrieve_binary_from_blob(&hash, 0, "nix").await;
        }

        info!(flake_url, attribute, "Building from Nix flake");

        // Execute the nix build
        let output = Self::build_from_nix_execute(flake_url, &cache_key).await?;

        // Parse the JSON output
        let build_result: Vec<NixBuildOutput> =
            serde_json::from_slice(&output.stdout).map_err(|e| JobError::BuildFailed {
                reason: format!("Failed to parse nix build output: {}", e),
            })?;

        let store_path = build_result
            .first()
            .ok_or_else(|| JobError::BuildFailed {
                reason: "No build output from nix".to_string(),
            })?
            .out_path
            .clone();

        // Read the binary from the store path
        let binary = Self::build_from_nix_read_binary(&store_path).await?;

        // Store binary in blob store and cache the hash
        let add_result = self.blob_store.add_bytes(&binary).await.map_err(|e| JobError::VmExecutionFailed {
            reason: format!("Failed to store binary in blob store: {}", e),
        })?;

        {
            let mut cache = self.build_cache.lock().await;
            cache.insert(cache_key, add_result.blob_ref.hash.to_string());
        }

        Ok(binary)
    }

    /// Create a temporary file with the given Nix expression content.
    async fn build_from_nix_expr_create_temp(content: &str) -> Result<NamedTempFile> {
        let temp_file = NamedTempFile::new().map_err(|e| JobError::BuildFailed {
            reason: format!("Failed to create temp file: {}", e),
        })?;

        fs::write(temp_file.path(), content).await.map_err(|e| JobError::BuildFailed {
            reason: format!("Failed to write Nix expression: {}", e),
        })?;

        Ok(temp_file)
    }

    /// Spawn nix-build process and return the child process and stderr lines reader.
    fn build_from_nix_expr_spawn(
        temp_file: &NamedTempFile,
    ) -> Result<(tokio::process::Child, tokio::io::Lines<BufReader<tokio::process::ChildStderr>>)> {
        let mut child = tokio::process::Command::new("nix-build")
            .arg(temp_file.path())
            .arg("--no-out-link")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| JobError::BuildFailed {
                reason: format!("Failed to spawn nix-build: {}", e),
            })?;

        let stderr = child.stderr.take().ok_or_else(|| JobError::BuildFailed {
            reason: "stderr not available after spawn".to_string(),
        })?;
        let stderr_lines = BufReader::new(stderr).lines();

        Ok((child, stderr_lines))
    }

    /// Wait for nix-build process with timeout and return output.
    async fn build_from_nix_expr_wait(
        child: tokio::process::Child,
        stderr_task: tokio::task::JoinHandle<String>,
    ) -> Result<std::process::Output> {
        let wait_result = tokio::time::timeout(NIX_BUILD_TIMEOUT, child.wait_with_output()).await;

        let output = match wait_result {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return Err(JobError::BuildFailed {
                    reason: format!("Failed to wait for nix-build: {}", e),
                });
            }
            Err(_) => {
                warn!(timeout_secs = NIX_BUILD_TIMEOUT.as_secs(), "nix-build timed out");
                return Err(JobError::BuildFailed {
                    reason: format!("nix-build timed out after {} seconds", NIX_BUILD_TIMEOUT.as_secs()),
                });
            }
        };

        let collected_stderr = stderr_task.await.unwrap_or_default();

        if !output.status.success() {
            let stderr_msg = if collected_stderr.is_empty() {
                String::from_utf8_lossy(&output.stderr).to_string()
            } else {
                collected_stderr
            };
            return Err(JobError::BuildFailed {
                reason: format!("Nix build failed: {}", stderr_msg),
            });
        }

        Ok(output)
    }

    /// Build a binary from an inline Nix expression.
    async fn build_from_nix_expr(&self, content: &str) -> Result<Vec<u8>> {
        info!("Building from inline Nix expression");

        // Create temp file with content
        let temp_file = Self::build_from_nix_expr_create_temp(content).await?;

        // Spawn nix-build process
        let (child, mut stderr_lines) = Self::build_from_nix_expr_spawn(&temp_file)?;

        // Spawn task to collect stderr
        let stderr_task = tokio::spawn(async move {
            let mut collected_stderr = String::new();
            while let Ok(Some(line)) = stderr_lines.next_line().await {
                debug!(build_output = %line, "nix-build progress");
                collected_stderr.push_str(&line);
                collected_stderr.push('\n');
            }
            collected_stderr
        });

        // Wait for build with timeout
        let output = Self::build_from_nix_expr_wait(child, stderr_task).await?;

        // Extract store path and read binary
        let store_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Self::build_from_nix_read_binary(&store_path).await
    }

    /// Execute a native binary in a micro-VM.
    async fn execute_binary(
        &self,
        binary: Vec<u8>,
        job_config: &crate::job::JobConfig,
        input_data: Option<&serde_json::Value>,
    ) -> Result<JobResult> {
        // Create VM configuration
        let _timeout = job_config.timeout.unwrap_or(Duration::from_secs(5));

        // Create sandbox configuration
        let config = SandboxConfiguration::default();

        // Create uninitialized sandbox with guest binary
        let mut sandbox = UninitializedSandbox::new(GuestBinary::Buffer(&binary), Some(config)).map_err(|e| {
            JobError::VmExecutionFailed {
                reason: format!("Failed to create sandbox: {}", e),
            }
        })?;

        // Register host functions that the guest can call
        self.register_host_functions(&mut sandbox)?;

        // Initialize and evolve to MultiUseSandbox
        let mut sandbox: MultiUseSandbox = sandbox.evolve().map_err(|e| JobError::VmExecutionFailed {
            reason: format!("Failed to initialize sandbox: {}", e),
        })?;

        // Prepare job input - use provided input_data or empty bytes
        let input: Vec<u8> = if let Some(data) = input_data {
            // If input is a string, use it directly as bytes
            if let Some(s) = data.as_str() {
                s.as_bytes().to_vec()
            } else {
                // Otherwise serialize the JSON value
                serde_json::to_vec(data).unwrap_or_default()
            }
        } else {
            Vec::new()
        };

        // Call the guest's execute function
        let output: Vec<u8> = sandbox.call("execute", input).map_err(|e| JobError::VmExecutionFailed {
            reason: format!("Guest execution failed: {}", e),
        })?;

        // Parse the result
        let result: serde_json::Value = serde_json::from_slice(&output).unwrap_or_else(|_| {
            serde_json::json!({
                "raw_output": String::from_utf8_lossy(&output)
            })
        });

        Ok(JobResult::success(result))
    }

    /// Register host functions that guest code can call.
    fn register_host_functions(&self, sandbox: &mut UninitializedSandbox) -> Result<()> {
        // Provide a print function for guest logging
        sandbox
            .register("hl_println", |msg: String| {
                info!(guest_message = %msg, "Guest output");
                Ok(())
            })
            .map_err(|e| JobError::VmExecutionFailed {
                reason: format!("Failed to register host function: {}", e),
            })?;

        // Provide a way to get current time
        sandbox
            .register("hl_get_time", || {
                Ok(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
            })
            .map_err(|e| JobError::VmExecutionFailed {
                reason: format!("Failed to register host function: {}", e),
            })?;

        Ok(())
    }

    /// Execute a WASM module in a micro-VM.
    #[allow(dead_code)]
    async fn execute_wasm(&self, module: Vec<u8>, job_config: &crate::job::JobConfig) -> Result<JobResult> {
        // Create sandbox configuration
        let config = SandboxConfiguration::default();

        // Create sandbox with WASM module
        let mut sandbox = UninitializedSandbox::new(GuestBinary::Buffer(&module), Some(config)).map_err(|e| {
            JobError::VmExecutionFailed {
                reason: format!("Failed to create WASM sandbox: {}", e),
            }
        })?;

        // Register host functions
        self.register_host_functions(&mut sandbox)?;

        // Initialize sandbox
        let mut sandbox: MultiUseSandbox = sandbox.evolve().map_err(|e| JobError::VmExecutionFailed {
            reason: format!("Failed to initialize WASM sandbox: {}", e),
        })?;

        // Execute WASM function
        let input = serde_json::to_vec(&job_config)?;
        let output: Vec<u8> = sandbox.call("execute", input).map_err(|e| JobError::VmExecutionFailed {
            reason: format!("WASM execution failed: {}", e),
        })?;

        let result: serde_json::Value = serde_json::from_slice(&output).unwrap_or_else(|_| {
            serde_json::json!({
                "raw_output": String::from_utf8_lossy(&output)
            })
        });

        Ok(JobResult::success(result))
    }
}

#[async_trait]
impl Worker for HyperlightWorker {
    async fn execute(&self, job: Job) -> JobResult {
        // Extract input data from payload (if present) before parsing binary location
        let input_data = job.spec.payload.get("input").cloned();

        // Parse the job payload for binary location
        let payload: JobPayload = match serde_json::from_value(job.spec.payload.clone()) {
            Ok(p) => p,
            Err(e) => {
                return JobResult::failure(format!("Failed to parse job payload: {}", e));
            }
        };

        // Get the binary (retrieve from blob store or build if needed)
        let result = match payload {
            JobPayload::BlobBinary { hash, size, format } => {
                // Retrieve binary from blob store
                match self.retrieve_binary_from_blob(&hash, size, &format).await {
                    Ok(binary) => self.execute_binary(binary, &job.spec.config, input_data.as_ref()).await,
                    Err(e) => Err(e),
                }
            }

            JobPayload::NixExpression { flake_url, attribute } => {
                // Build from flake then execute
                match self.build_from_nix(&flake_url, &attribute).await {
                    Ok(binary) => self.execute_binary(binary, &job.spec.config, input_data.as_ref()).await,
                    Err(e) => Err(e),
                }
            }

            JobPayload::NixDerivation { content } => {
                // Build from inline Nix then execute
                match self.build_from_nix_expr(&content).await {
                    Ok(binary) => self.execute_binary(binary, &job.spec.config, input_data.as_ref()).await,
                    Err(e) => Err(e),
                }
            }
        };

        match result {
            Ok(job_result) => job_result,
            Err(e) => JobResult::failure(format!("VM execution failed: {}", e)),
        }
    }

    fn job_types(&self) -> Vec<String> {
        vec!["vm_execute".to_string(), "sandboxed".to_string()]
    }
}

// Note: Default impl removed since HyperlightWorker now requires a blob_store.
// Workers must be created explicitly with HyperlightWorker::new(blob_store).
