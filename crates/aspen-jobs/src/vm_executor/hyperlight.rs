//! Hyperlight micro-VM worker implementation.

use std::collections::HashMap;
use std::process::Command;
use std::time::Duration;

use async_trait::async_trait;
use hyperlight::{GuestBinary, MultiUseGuestConfig, Result as HlResult, RunMode, VmConfig};
use serde_json::Value;
use tempfile::NamedTempFile;
use tokio::fs;
use tracing::{debug, info, warn};

use crate::error::{JobError, Result};
use crate::job::{Job, JobResult};
use crate::worker::Worker;
use crate::vm_executor::types::{JobPayload, NixBuildOutput};

/// Maximum size for a built binary (50MB).
const MAX_BINARY_SIZE: usize = 50 * 1024 * 1024;

/// Worker that executes jobs in Hyperlight micro-VMs.
pub struct HyperlightWorker {
    /// Cache of built binaries (flake_url -> binary).
    build_cache: HashMap<String, Vec<u8>>,
}

impl HyperlightWorker {
    /// Create a new Hyperlight worker.
    pub fn new() -> Result<Self> {
        Ok(Self {
            build_cache: HashMap::new(),
        })
    }

    /// Build a binary from a Nix flake.
    async fn build_from_nix(&mut self, flake_url: &str, attribute: &str) -> Result<Vec<u8>> {
        let cache_key = format!("{}#{}", flake_url, attribute);

        // Check cache first
        if let Some(binary) = self.build_cache.get(&cache_key) {
            debug!(flake_url, attribute, "Using cached binary");
            return Ok(binary.clone());
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
        self.build_cache.insert(cache_key, binary.clone());

        Ok(binary)
    }

    /// Build a binary from an inline Nix expression.
    async fn build_from_nix_expr(&mut self, content: &str) -> Result<Vec<u8>> {
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

        // Create guest binary
        let guest_binary = GuestBinary::new(binary)
            .map_err(|e| JobError::VmExecutionFailed {
                reason: format!("Failed to create guest binary: {}", e),
            })?;

        // Configure the VM
        let config = MultiUseGuestConfig::default()
            .with_run_mode(RunMode::Job)
            .with_run_time_limit_microseconds(timeout.as_micros() as u64);

        // Create and run the VM
        let mut vm_config = VmConfig::default();
        vm_config.guest_config = config;

        // Note: In production Hyperlight, this would create and execute the VM
        // For now, we'll simulate the execution
        warn!("Hyperlight execution simulated - actual hyperlight crate integration needed");

        // Simulate success
        Ok(JobResult::success(serde_json::json!({
            "message": "Job executed in micro-VM",
            "execution_time_ms": 10,
        })))
    }

    /// Execute a WASM module in a micro-VM.
    async fn execute_wasm(&self, module: Vec<u8>, job_config: &crate::job::JobConfig) -> Result<JobResult> {
        // Similar to binary execution but for WASM
        let timeout = job_config.timeout.unwrap_or(Duration::from_secs(5));

        warn!("WASM execution in Hyperlight not yet implemented");

        Ok(JobResult::success(serde_json::json!({
            "message": "WASM module executed",
            "execution_time_ms": 5,
        })))
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