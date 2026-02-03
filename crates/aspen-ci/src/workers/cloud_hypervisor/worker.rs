//! CloudHypervisorWorker - VM Pool Manager for Cloud Hypervisor CI Workers.
//!
//! This module manages a pool of Cloud Hypervisor microVMs that run as
//! ephemeral Aspen cluster workers. The architecture is:
//!
//! 1. CloudHypervisorWorker maintains a pool of warm VMs
//! 2. Each VM runs `aspen-node --worker-only`
//! 3. VMs join the cluster via Iroh ticket and register as workers
//! 4. Jobs are routed to VM workers through the normal job queue
//! 5. VMs execute jobs and upload artifacts directly to SNIX
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Host Node                                │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │            CloudHypervisorWorker                    │   │
//! │  │  (VM Pool Manager - maintains warm VMs)             │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! │           │                    │                    │       │
//! │     ┌─────┴─────┐        ┌─────┴─────┐       ┌─────┴─────┐ │
//! │     │   VM 0    │        │   VM 1    │       │   VM N    │ │
//! │     │ aspen-node│        │ aspen-node│       │ aspen-node│ │
//! │     │ (worker)  │        │ (worker)  │       │ (worker)  │ │
//! │     └───────────┘        └───────────┘       └───────────┘ │
//! │           │                    │                    │       │
//! │           └────────────────────┼────────────────────┘       │
//! │                                │                            │
//! │                    Iroh Cluster Connection                  │
//! └────────────────────────────────┼────────────────────────────┘
//!                                  │
//!                     ┌────────────┴────────────┐
//!                     │     Aspen Cluster       │
//!                     │  (Job Queue + SNIX)     │
//!                     └─────────────────────────┘
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aspen_blob::BlobStore;
use aspen_constants::CI_VM_DEFAULT_EXECUTION_TIMEOUT_MS;
use aspen_constants::CI_VM_MAX_EXECUTION_TIMEOUT_MS;
use aspen_jobs::Job;
use aspen_jobs::JobError;
use aspen_jobs::JobResult;
use aspen_jobs::Worker;
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::config::CloudHypervisorWorkerConfig;
use super::error::CloudHypervisorError;
use super::error::Result;
use super::pool::PoolStatus;
use super::pool::VmPool;

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

/// VM pool manager for Cloud Hypervisor CI workers.
///
/// Manages a pool of warm microVMs that run as ephemeral cluster workers.
/// VMs join the cluster via Iroh and handle jobs directly - this worker
/// only manages VM lifecycle, not job execution.
pub struct CloudHypervisorWorker {
    /// Worker configuration.
    config: CloudHypervisorWorkerConfig,

    /// VM pool for warm VM management.
    pool: Arc<VmPool>,

    /// Handle for the pool maintenance background task.
    /// Dropped on worker shutdown.
    maintenance_task: tokio::sync::RwLock<Option<tokio::task::JoinHandle<()>>>,
}

/// Interval between pool maintenance cycles (30 seconds).
const POOL_MAINTENANCE_INTERVAL_SECS: u64 = 30;

impl CloudHypervisorWorker {
    /// Create a new Cloud Hypervisor VM pool manager.
    pub fn new(config: CloudHypervisorWorkerConfig) -> Result<Self> {
        config.validate().map_err(|e| CloudHypervisorError::InvalidConfig { message: e })?;

        let pool = Arc::new(VmPool::new(config.clone()));

        Ok(Self {
            config,
            pool,
            maintenance_task: tokio::sync::RwLock::new(None),
        })
    }

    /// Create a new Cloud Hypervisor VM pool manager with an optional blob store.
    ///
    /// Note: blob_store is no longer used - VMs handle artifact upload directly via SNIX.
    /// This method is kept for API compatibility but ignores the blob_store parameter.
    #[deprecated(note = "blob_store is no longer used - VMs upload artifacts directly to SNIX")]
    pub fn with_blob_store(
        config: CloudHypervisorWorkerConfig,
        _blob_store: Option<Arc<dyn BlobStore>>,
    ) -> Result<Self> {
        Self::new(config)
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
}

/// VM pool manager interface.
///
/// CloudHypervisorWorker now acts purely as a VM pool manager. Jobs are no longer
/// executed via this worker - instead, VMs join the cluster as ephemeral workers
/// and handle jobs directly.
///
/// This trait implementation is kept for backwards compatibility and pool lifecycle
/// management. The `execute()` method will return an error if called directly.
#[async_trait]
impl Worker for CloudHypervisorWorker {
    async fn execute(&self, job: Job) -> JobResult {
        // CloudHypervisorWorker no longer executes jobs directly.
        // VMs now run aspen-node --worker-only and handle jobs themselves.
        //
        // This method should never be called since job_types() returns an empty list.
        // If it is called, return an error directing to the new architecture.
        let job_id = job.id.to_string();
        warn!(
            job_id = %job_id,
            job_type = %job.spec.job_type,
            "CloudHypervisorWorker.execute() called - this worker is now a VM pool manager only"
        );

        JobResult::failure(
            "CloudHypervisorWorker no longer executes jobs directly. \
             Jobs should be routed to VM workers that have joined the cluster. \
             Ensure VMs have booted and registered as workers for 'ci_vm' job type."
                .to_string(),
        )
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
        // CloudHypervisorWorker no longer handles jobs directly.
        // VMs register themselves as workers and handle 'ci_vm' jobs.
        // Return empty list so no jobs get routed to this worker.
        vec![]
    }
}

impl CloudHypervisorWorker {
    /// Get the current status of the VM pool.
    ///
    /// Returns information about idle VMs, total VMs, and capacity.
    pub async fn pool_status(&self) -> PoolStatus {
        self.pool.status().await
    }

    /// Check if the cluster ticket is configured.
    ///
    /// VMs need a cluster ticket to join the cluster and register as workers.
    pub fn has_cluster_ticket(&self) -> bool {
        self.config.cluster_ticket.is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

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
        // CloudHypervisorWorker is a VM pool manager, not a job executor.
        // VMs register themselves as workers and handle ci_vm jobs directly.
        assert!(types.is_empty(), "should return empty (VM pool manager doesn't handle jobs)");
    }

    /// Helper to simulate the nix flag injection logic from execute_on_vm
    fn inject_nix_flags(command: &str, args: Vec<String>) -> Vec<String> {
        if command == "nix" {
            let mut args = args;
            if !args.is_empty() {
                let mut insert_pos = 1;

                if !args.iter().any(|a| a == "--offline") {
                    args.insert(insert_pos, "--offline".to_string());
                    insert_pos += 1;
                }

                if !args.iter().any(|a| a.contains("experimental-features")) {
                    args.insert(insert_pos, "--extra-experimental-features".to_string());
                    insert_pos += 1;
                    args.insert(insert_pos, "nix-command flakes".to_string());
                    insert_pos += 1;
                }

                if !args.iter().any(|a| a == "--accept-flake-config") {
                    args.insert(insert_pos, "--accept-flake-config".to_string());
                    insert_pos += 1;
                }

                if !args.iter().any(|a| a == "--no-write-lock-file") {
                    args.insert(insert_pos, "--no-write-lock-file".to_string());
                }
            }
            args
        } else {
            args
        }
    }

    #[test]
    fn test_nix_flag_injection() {
        // Test that all required flags are injected for nix commands
        let args = inject_nix_flags("nix", vec!["build".to_string(), "-L".to_string(), ".#default".to_string()]);

        assert_eq!(args, vec![
            "build",
            "--offline",
            "--extra-experimental-features",
            "nix-command flakes",
            "--accept-flake-config",
            "--no-write-lock-file",
            "-L",
            ".#default"
        ]);
    }

    #[test]
    fn test_nix_flags_not_duplicated() {
        // Test that flags are not duplicated if already present
        let args = inject_nix_flags("nix", vec![
            "build".to_string(),
            "--offline".to_string(),
            "--extra-experimental-features".to_string(),
            "nix-command flakes".to_string(),
            "--accept-flake-config".to_string(),
            "--no-write-lock-file".to_string(),
            ".#default".to_string(),
        ]);

        // Should remain unchanged since all flags are already present
        assert_eq!(args, vec![
            "build",
            "--offline",
            "--extra-experimental-features",
            "nix-command flakes",
            "--accept-flake-config",
            "--no-write-lock-file",
            ".#default"
        ]);
    }

    #[test]
    fn test_non_nix_command_unchanged() {
        // Test that non-nix commands are not modified
        let args = inject_nix_flags("cargo", vec!["build".to_string(), "--release".to_string()]);

        // Should remain unchanged
        assert_eq!(args, vec!["build", "--release"]);
    }

    /// Test flake attribute extraction from payload args (mirrors production logic)
    fn extract_flake_attr(payload: &CloudHypervisorPayload) -> String {
        let attr = payload
            .flake_attr
            .clone()
            .or_else(|| {
                // Try to find flake attr in args (commonly the last arg starting with . or #)
                payload.args.iter().find(|a| a.starts_with('.') || a.starts_with('#')).cloned()
            })
            .unwrap_or_else(|| ".#default".to_string());

        // Normalize: ensure .# prefix
        if attr.starts_with('.') || attr.starts_with('#') {
            attr
        } else {
            format!(".#{}", attr)
        }
    }

    #[test]
    fn test_flake_attr_extraction_from_args() {
        // Test extraction from args when flake_attr is not set
        let payload = CloudHypervisorPayload {
            job_name: None,
            command: "nix".to_string(),
            args: vec![
                "build".to_string(),
                "-L".to_string(),
                ".#packages.x86_64-linux.default".to_string(),
            ],
            working_dir: ".".to_string(),
            env: HashMap::new(),
            timeout_secs: 3600,
            artifacts: vec![],
            source_hash: None,
            checkout_dir: None,
            flake_attr: None,
        };

        assert_eq!(extract_flake_attr(&payload), ".#packages.x86_64-linux.default");
    }

    #[test]
    fn test_flake_attr_explicit() {
        // Test that explicit flake_attr takes precedence
        let payload = CloudHypervisorPayload {
            job_name: None,
            command: "nix".to_string(),
            args: vec!["build".to_string(), ".#other".to_string()],
            working_dir: ".".to_string(),
            env: HashMap::new(),
            timeout_secs: 3600,
            artifacts: vec![],
            source_hash: None,
            checkout_dir: None,
            flake_attr: Some(".#explicit".to_string()),
        };

        assert_eq!(extract_flake_attr(&payload), ".#explicit");
    }

    #[test]
    fn test_flake_attr_default_fallback() {
        // Test default fallback when no flake attr in args
        let payload = CloudHypervisorPayload {
            job_name: None,
            command: "nix".to_string(),
            args: vec!["build".to_string(), "-L".to_string()],
            working_dir: ".".to_string(),
            env: HashMap::new(),
            timeout_secs: 3600,
            artifacts: vec![],
            source_hash: None,
            checkout_dir: None,
            flake_attr: None,
        };

        assert_eq!(extract_flake_attr(&payload), ".#default");
    }

    #[test]
    fn test_flake_attr_normalization_without_prefix() {
        // Test that flake_attr without .# prefix is normalized
        let payload = CloudHypervisorPayload {
            job_name: None,
            command: "nix".to_string(),
            args: vec!["build".to_string(), "-L".to_string()],
            working_dir: ".".to_string(),
            env: HashMap::new(),
            timeout_secs: 3600,
            artifacts: vec![],
            source_hash: None,
            checkout_dir: None,
            flake_attr: Some("packages.x86_64-linux.default".to_string()),
        };

        // Should be normalized to include .# prefix
        assert_eq!(extract_flake_attr(&payload), ".#packages.x86_64-linux.default");
    }

    #[test]
    fn test_extract_fod_derivations_identifies_fods() {
        // Simulated `nix derivation show --recursive` output with FODs and non-FODs.
        // FODs have "hash" or "hashAlgo" in their outputs, or outputHash in env.
        let json = r#"{
            "/nix/store/abc123-bash-5.3.tar.gz.drv": {
                "outputs": {
                    "out": {
                        "hash": "sha256-ABCDEF123456",
                        "hashAlgo": "sha256",
                        "path": "/nix/store/xyz-bash-5.3.tar.gz"
                    }
                },
                "env": {}
            },
            "/nix/store/def456-glibc.drv": {
                "outputs": {
                    "out": {
                        "path": "/nix/store/glibc-out"
                    }
                },
                "env": {}
            },
            "/nix/store/ghi789-patch.drv": {
                "outputs": {
                    "out": {
                        "path": "/nix/store/patch-out"
                    }
                },
                "env": {
                    "outputHash": "sha256:deadbeef",
                    "outputHashAlgo": "sha256"
                }
            }
        }"#;

        let fods = extract_fod_derivations(json);

        // Should find the two FODs (bash tarball and patch) but not glibc
        assert_eq!(fods.len(), 2);
        assert!(fods.contains(&"/nix/store/abc123-bash-5.3.tar.gz.drv".to_string()));
        assert!(fods.contains(&"/nix/store/ghi789-patch.drv".to_string()));
        assert!(!fods.contains(&"/nix/store/def456-glibc.drv".to_string()));
    }

    #[test]
    fn test_extract_fod_derivations_empty_input() {
        // Empty or invalid JSON should return empty vec
        assert!(extract_fod_derivations("").is_empty());
        assert!(extract_fod_derivations("not json").is_empty());
        assert!(extract_fod_derivations("[]").is_empty()); // Array, not object
        assert!(extract_fod_derivations("{}").is_empty()); // Empty object
    }

    #[test]
    fn test_extract_fod_derivations_no_fods() {
        // Derivations without FOD indicators should be ignored
        let json = r#"{
            "/nix/store/regular.drv": {
                "outputs": {
                    "out": { "path": "/nix/store/out" }
                },
                "env": { "name": "regular" }
            }
        }"#;

        assert!(extract_fod_derivations(json).is_empty());
    }

    #[test]
    fn test_detect_dynamic_derivations_none() {
        // Standard derivation with empty dynamicOutputs should not be flagged
        let json = r#"{
            "/nix/store/abc123-hello.drv": {
                "outputs": {
                    "out": { "path": "/nix/store/xyz-hello" }
                },
                "inputDrvs": {
                    "/nix/store/dep.drv": {
                        "dynamicOutputs": {},
                        "outputs": ["out"]
                    }
                }
            }
        }"#;

        assert!(detect_dynamic_derivations(json).is_empty());
    }

    #[test]
    fn test_detect_dynamic_derivations_with_dynamic_outputs() {
        // Derivation depending on dynamicOutputs should be detected
        let json = r#"{
            "/nix/store/consumer.drv": {
                "outputs": {
                    "out": { "path": "/nix/store/consumer-out" }
                },
                "inputDrvs": {
                    "/nix/store/producer.drv": {
                        "dynamicOutputs": {
                            "out": {
                                "dynamicOutputs": {},
                                "outputs": ["out"]
                            }
                        },
                        "outputs": []
                    }
                }
            }
        }"#;

        let result = detect_dynamic_derivations(json);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("consumer.drv"));
    }

    #[test]
    fn test_detect_dynamic_derivations_drv_output() {
        // Derivation producing a .drv file should be detected
        let json = r#"{
            "/nix/store/producer.drv": {
                "outputs": {
                    "out": { "path": "/nix/store/xyz-inner.drv" }
                },
                "inputDrvs": {}
            }
        }"#;

        let result = detect_dynamic_derivations(json);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("producer.drv"));
    }

    #[test]
    fn test_detect_dynamic_derivations_empty_input() {
        // Empty or invalid JSON should return empty vec
        assert!(detect_dynamic_derivations("").is_empty());
        assert!(detect_dynamic_derivations("not json").is_empty());
        assert!(detect_dynamic_derivations("{}").is_empty());
    }
}
