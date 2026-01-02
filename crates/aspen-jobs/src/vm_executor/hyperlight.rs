//! Hyperlight micro-VM worker implementation.

use std::collections::HashMap;
use std::time::Duration;
use std::sync::Mutex;

use async_trait::async_trait;
use hyperlight_host::{GuestBinary, MultiUseSandbox, UninitializedSandbox};
use hyperlight_host::sandbox::SandboxConfiguration;
use tempfile::NamedTempFile;
use tokio::fs;
use tracing::{debug, info};

use crate::error::{JobError, Result};
use crate::job::{Job, JobResult};
use crate::worker::Worker;
use crate::vm_executor::types::{JobPayload, NixBuildOutput};

/// Maximum size for a built binary (50MB).
const MAX_BINARY_SIZE: usize = 50 * 1024 * 1024;

/// Worker that executes jobs in Hyperlight micro-VMs.
pub struct HyperlightWorker {
    /// Cache of built binaries (flake_url -> binary).
    /// Using Mutex for interior mutability since Worker trait expects &self.
    build_cache: Mutex<HashMap<String, Vec<u8>>>,
}

impl HyperlightWorker {
    /// Create a new Hyperlight worker.
    pub fn new() -> Result<Self> {
        Ok(Self {
            build_cache: Mutex::new(HashMap::new()),
        })
    }

    /// Build a binary from a Nix flake.
    async fn build_from_nix(&self, flake_url: &str, attribute: &str) -> Result<Vec<u8>> {
        let cache_key = format!("{}#{}", flake_url, attribute);

        // Check cache first
        if let Ok(cache) = self.build_cache.lock() {
            if let Some(binary) = cache.get(&cache_key) {
                debug!(flake_url, attribute, "Using cached binary");
                return Ok(binary.clone());
            }
        }

        info!(flake_url, attribute, "Building from Nix flake");

        // Run nix build command
        let output = tokio::process::Command::new("nix")
            .args(&["build", &cache_key])
            .args(&["--json", "--no-link"])
            .output()
            .await
            .map_err(|e| JobError::BuildFailed {
                reason: format!("Failed to run nix build: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(JobError::BuildFailed {
                reason: format!("Nix build failed: {}", stderr),
            });
        }

        // Parse the JSON output
        let build_result: Vec<NixBuildOutput> = serde_json::from_slice(&output.stdout)
            .map_err(|e| JobError::BuildFailed {
                reason: format!("Failed to parse nix build output: {}", e),
            })?;

        let store_path = build_result
            .first()
            .ok_or_else(|| JobError::BuildFailed {
                reason: "No build output from nix".to_string(),
            })?
            .out_path
            .clone();

        // Try to find the binary in the store path
        let binary_path = format!("{}/bin/*", store_path);
        let glob_pattern = glob::glob(&binary_path)
            .map_err(|e| JobError::BuildFailed {
                reason: format!("Invalid glob pattern: {}", e),
            })?;

        let binary_file = glob_pattern
            .filter_map(|p| p.ok())
            .next()
            .ok_or_else(|| JobError::BuildFailed {
                reason: format!("No binary found in {}/bin/", store_path),
            })?;

        // Read the binary
        let binary = fs::read(&binary_file).await
            .map_err(|e| JobError::BuildFailed {
                reason: format!("Failed to read binary: {}", e),
            })?;

        // Check size
        if binary.len() > MAX_BINARY_SIZE {
            return Err(JobError::BinaryTooLarge {
                size: binary.len(),
                max: MAX_BINARY_SIZE,
            });
        }

        // Cache it
        if let Ok(mut cache) = self.build_cache.lock() {
            cache.insert(cache_key, binary.clone());
        }

        Ok(binary)
    }

    /// Build a binary from an inline Nix expression.
    async fn build_from_nix_expr(&self, content: &str) -> Result<Vec<u8>> {
        info!("Building from inline Nix expression");

        // Write to temporary file
        let temp_file = NamedTempFile::new()
            .map_err(|e| JobError::BuildFailed {
                reason: format!("Failed to create temp file: {}", e),
            })?;

        fs::write(temp_file.path(), content).await
            .map_err(|e| JobError::BuildFailed {
                reason: format!("Failed to write Nix expression: {}", e),
            })?;

        // Build it
        let output = tokio::process::Command::new("nix-build")
            .arg(temp_file.path())
            .arg("--no-out-link")
            .output()
            .await
            .map_err(|e| JobError::BuildFailed {
                reason: format!("Failed to run nix-build: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(JobError::BuildFailed {
                reason: format!("Nix build failed: {}", stderr),
            });
        }

        let store_path = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();

        // Find the binary
        let binary_path = format!("{}/bin/*", store_path);
        let glob_pattern = glob::glob(&binary_path)
            .map_err(|e| JobError::BuildFailed {
                reason: format!("Invalid glob pattern: {}", e),
            })?;

        let binary_file = glob_pattern
            .filter_map(|p| p.ok())
            .next()
            .ok_or_else(|| JobError::BuildFailed {
                reason: format!("No binary found in {}/bin/", store_path),
            })?;

        // Read the binary
        let binary = fs::read(&binary_file).await
            .map_err(|e| JobError::BuildFailed {
                reason: format!("Failed to read binary: {}", e),
            })?;

        // Check size
        if binary.len() > MAX_BINARY_SIZE {
            return Err(JobError::BinaryTooLarge {
                size: binary.len(),
                max: MAX_BINARY_SIZE,
            });
        }

        Ok(binary)
    }

    /// Execute a native binary in a micro-VM.
    async fn execute_binary(&self, binary: Vec<u8>, job_config: &crate::job::JobConfig) -> Result<JobResult> {
        // Create VM configuration
        let timeout = job_config.timeout.unwrap_or(Duration::from_secs(5));

        // Create sandbox configuration
        let config = SandboxConfiguration::default();

        // Create uninitialized sandbox with guest binary
        let mut sandbox = UninitializedSandbox::new(
            GuestBinary::Buffer(&binary),
            Some(config),
        ).map_err(|e| JobError::VmExecutionFailed {
            reason: format!("Failed to create sandbox: {}", e),
        })?;

        // Register host functions that the guest can call
        self.register_host_functions(&mut sandbox)?;

        // Initialize and evolve to MultiUseSandbox
        let mut sandbox: MultiUseSandbox = sandbox.evolve()
            .map_err(|e| JobError::VmExecutionFailed {
                reason: format!("Failed to initialize sandbox: {}", e),
            })?;

        // Prepare job input
        let input = serde_json::to_vec(&job_config)
            .map_err(|e| JobError::VmExecutionFailed {
                reason: format!("Failed to serialize input: {}", e),
            })?;

        // Call the guest's execute function
        let output: Vec<u8> = sandbox.call("execute", input)
            .map_err(|e| JobError::VmExecutionFailed {
                reason: format!("Guest execution failed: {}", e),
            })?;

        // Parse the result
        let result: serde_json::Value = serde_json::from_slice(&output)
            .unwrap_or_else(|_| serde_json::json!({
                "raw_output": String::from_utf8_lossy(&output)
            }));

        Ok(JobResult::success(result))
    }

    /// Register host functions that guest code can call.
    fn register_host_functions(&self, sandbox: &mut UninitializedSandbox) -> Result<()> {
        // Provide a print function for guest logging
        sandbox.register("hl_println", |msg: String| {
            info!(guest_message = %msg, "Guest output");
            Ok(())
        }).map_err(|e| JobError::VmExecutionFailed {
            reason: format!("Failed to register host function: {}", e),
        })?;

        // Provide a way to get current time
        sandbox.register("hl_get_time", || {
            Ok(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs())
        }).map_err(|e| JobError::VmExecutionFailed {
            reason: format!("Failed to register host function: {}", e),
        })?;

        Ok(())
    }

    /// Execute a WASM module in a micro-VM.
    async fn execute_wasm(&self, module: Vec<u8>, job_config: &crate::job::JobConfig) -> Result<JobResult> {
        // Create sandbox configuration
        let config = SandboxConfiguration::default();

        // Create sandbox with WASM module
        let mut sandbox = UninitializedSandbox::new(
            GuestBinary::Buffer(&module),
            Some(config),
        ).map_err(|e| JobError::VmExecutionFailed {
            reason: format!("Failed to create WASM sandbox: {}", e),
        })?;

        // Register host functions
        self.register_host_functions(&mut sandbox)?;

        // Initialize sandbox
        let mut sandbox: MultiUseSandbox = sandbox.evolve()
            .map_err(|e| JobError::VmExecutionFailed {
                reason: format!("Failed to initialize WASM sandbox: {}", e),
            })?;

        // Execute WASM function
        let input = serde_json::to_vec(&job_config)?;
        let output: Vec<u8> = sandbox.call("execute", input)
            .map_err(|e| JobError::VmExecutionFailed {
                reason: format!("WASM execution failed: {}", e),
            })?;

        let result: serde_json::Value = serde_json::from_slice(&output)
            .unwrap_or_else(|_| serde_json::json!({
                "raw_output": String::from_utf8_lossy(&output)
            }));

        Ok(JobResult::success(result))
    }
}

#[async_trait]
impl Worker for HyperlightWorker {
    async fn execute(&self, job: Job) -> JobResult {
        // Parse the job payload
        let payload: JobPayload = match serde_json::from_value(job.spec.payload.clone()) {
            Ok(p) => p,
            Err(e) => {
                return JobResult::failure(format!("Failed to parse job payload: {}", e));
            }
        };

        // Get the binary (build if needed)
        let result = match payload {
            JobPayload::NativeBinary { binary } => {
                // Direct execution
                self.execute_binary(binary, &job.spec.config).await
            },

            JobPayload::NixExpression { flake_url, attribute } => {
                // Build from flake then execute
                match self.build_from_nix(&flake_url, &attribute).await {
                    Ok(binary) => self.execute_binary(binary, &job.spec.config).await,
                    Err(e) => Err(e),
                }
            },

            JobPayload::NixDerivation { content } => {
                // Build from inline Nix then execute
                match self.build_from_nix_expr(&content).await {
                    Ok(binary) => self.execute_binary(binary, &job.spec.config).await,
                    Err(e) => Err(e),
                }
            },

            JobPayload::WasmModule { module } => {
                // Execute WASM
                self.execute_wasm(module, &job.spec.config).await
            },
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

impl Default for HyperlightWorker {
    fn default() -> Self {
        Self::new().expect("Failed to create HyperlightWorker")
    }
}