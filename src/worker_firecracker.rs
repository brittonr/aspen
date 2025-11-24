// Firecracker MicroVM Worker Backend
//
// Executes jobs using Firecracker MicroVMs via microvm.nix

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::worker_trait::{WorkerBackend, WorkResult};
use crate::Job;

/// Configuration for Firecracker worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirecrackerConfig {
    /// Path to the microvm flake directory
    pub flake_dir: PathBuf,

    /// Path to store VM state and logs
    pub state_dir: PathBuf,

    /// Default memory allocation for VMs (MB)
    pub default_memory_mb: u32,

    /// Default vCPU count for VMs
    pub default_vcpus: u32,

    /// Control plane ticket URL for workers to connect
    pub control_plane_ticket: String,

    /// Maximum number of concurrent VMs
    pub max_concurrent_vms: usize,
}

impl Default for FirecrackerConfig {
    fn default() -> Self {
        Self {
            flake_dir: PathBuf::from("./microvms"),
            state_dir: PathBuf::from("./data/firecracker-vms"),
            default_memory_mb: 512,
            default_vcpus: 1,
            control_plane_ticket: String::new(),
            max_concurrent_vms: 10,
        }
    }
}

/// Worker backend that executes jobs using Firecracker MicroVMs
///
/// Each job runs in an isolated Firecracker VM that is created on-demand
/// and destroyed after job completion.
pub struct FirecrackerWorker {
    config: FirecrackerConfig,
}

impl FirecrackerWorker {
    /// Create a new Firecracker worker
    ///
    /// # Arguments
    /// * `config` - Firecracker configuration
    pub fn new(config: FirecrackerConfig) -> Result<Self> {
        // Validate configuration
        if !config.flake_dir.exists() {
            return Err(anyhow!(
                "Flake directory does not exist: {:?}",
                config.flake_dir
            ));
        }

        if config.control_plane_ticket.is_empty() {
            return Err(anyhow!("Control plane ticket must be configured"));
        }

        // Create state directory if it doesn't exist
        std::fs::create_dir_all(&config.state_dir)?;

        tracing::info!(
            flake_dir = ?config.flake_dir,
            state_dir = ?config.state_dir,
            "Firecracker worker initialized"
        );

        Ok(Self { config })
    }

    /// Build a VM configuration for a specific job
    ///
    /// Uses Nix to build a microVM image with the job payload baked in
    async fn build_vm(&self, job: &Job) -> Result<VmInstance> {
        let vm_id = format!("job-{}", job.id);
        let vm_dir = self.config.state_dir.join(&vm_id);
        std::fs::create_dir_all(&vm_dir)?;

        tracing::info!(
            job_id = %job.id,
            vm_id = %vm_id,
            "Building Firecracker VM"
        );

        // Determine resource requirements from job
        let memory_mb = self.config.default_memory_mb;
        let vcpus = self.config.default_vcpus;

        // Write job payload to a file that will be mounted in the VM
        let payload_path = vm_dir.join("job-payload.json");
        let payload_json = serde_json::to_string_pretty(&job.payload)?;
        std::fs::write(&payload_path, payload_json)?;

        tracing::debug!(
            job_id = %job.id,
            payload_path = ?payload_path,
            "Job payload written"
        );

        // Build the VM using Nix
        // This creates a microVM with the worker binary and necessary environment
        let build_result = Command::new("nix")
            .args(&[
                "build",
                "--no-link",
                "--print-out-paths",
                &format!("{}#worker-vm", self.config.flake_dir.display()),
            ])
            .env("VM_ID", &vm_id)
            .env("MEMORY_MB", memory_mb.to_string())
            .env("VCPUS", vcpus.to_string())
            .env("CONTROL_PLANE_TICKET", &self.config.control_plane_ticket)
            .env("JOB_PAYLOAD_PATH", payload_path.display().to_string())
            .output()
            .await?;

        if !build_result.status.success() {
            let stderr = String::from_utf8_lossy(&build_result.stderr);
            return Err(anyhow!("Failed to build VM: {}", stderr));
        }

        let vm_path = String::from_utf8(build_result.stdout)?
            .trim()
            .to_string();

        tracing::info!(
            job_id = %job.id,
            vm_path = %vm_path,
            "VM built successfully"
        );

        Ok(VmInstance {
            vm_id,
            vm_dir,
            vm_path: PathBuf::from(vm_path),
            memory_mb,
            vcpus,
        })
    }

    /// Start a VM instance
    async fn start_vm(&self, vm: &VmInstance) -> Result<VmProcess> {
        tracing::info!(vm_id = %vm.vm_id, "Starting Firecracker VM");

        let log_path = vm.vm_dir.join("vm.log");
        let log_file = std::fs::File::create(&log_path)?;

        // Run the microVM using the built VM path
        // The VM will run the worker binary which connects to the control plane
        let child = Command::new(&vm.vm_path.join("bin/run-vm"))
            .env("VM_ID", &vm.vm_id)
            .stdin(Stdio::null())
            .stdout(log_file.try_clone()?)
            .stderr(log_file)
            .spawn()?;

        tracing::info!(
            vm_id = %vm.vm_id,
            pid = child.id().unwrap_or(0),
            log_path = ?log_path,
            "VM process started"
        );

        Ok(VmProcess {
            child,
            log_path,
        })
    }

    /// Wait for VM to complete the job
    ///
    /// Monitors the VM process and log output to determine when the job is done
    async fn wait_for_completion(&self, mut vm_process: VmProcess, vm: &VmInstance) -> Result<WorkResult> {
        tracing::info!(vm_id = %vm.vm_id, "Waiting for VM to complete job");

        // Tail the log file to monitor progress
        let log_file = tokio::fs::File::open(&vm_process.log_path).await?;
        let reader = BufReader::new(log_file);
        let mut lines = reader.lines();

        // Wait for completion marker or process exit
        let timeout = tokio::time::Duration::from_secs(300); // 5 minutes
        let result = tokio::time::timeout(timeout, async {
            loop {
                tokio::select! {
                    line = lines.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                tracing::debug!(vm_id = %vm.vm_id, line = %line, "VM log");

                                // Look for completion markers in the log
                                if line.contains("JOB_COMPLETED_SUCCESS") {
                                    return Ok(WorkResult::success());
                                } else if line.contains("JOB_COMPLETED_FAILURE") {
                                    let error = line.split("ERROR:").nth(1)
                                        .unwrap_or("Job failed")
                                        .trim();
                                    return Ok(WorkResult::failure(error));
                                }
                            }
                            Ok(None) => {
                                // Log file ended, wait for process
                                break;
                            }
                            Err(e) => {
                                tracing::warn!(vm_id = %vm.vm_id, error = %e, "Error reading log");
                            }
                        }
                    }
                    status = vm_process.child.wait() => {
                        let exit_status = status?;
                        if exit_status.success() {
                            return Ok(WorkResult::success());
                        } else {
                            return Ok(WorkResult::failure(format!(
                                "VM exited with status: {}",
                                exit_status
                            )));
                        }
                    }
                }
            }

            // Process exited, check exit code
            let exit_status = vm_process.child.wait().await?;
            if exit_status.success() {
                Ok(WorkResult::success())
            } else {
                Ok(WorkResult::failure(format!(
                    "VM exited with status: {}",
                    exit_status
                )))
            }
        })
        .await;

        match result {
            Ok(work_result) => work_result,
            Err(_) => {
                // Timeout - kill the VM
                tracing::warn!(vm_id = %vm.vm_id, "VM execution timed out, killing process");
                let _ = vm_process.child.kill().await;
                Ok(WorkResult::failure("Job execution timed out"))
            }
        }
    }

    /// Cleanup VM resources
    async fn cleanup_vm(&self, vm: &VmInstance) -> Result<()> {
        tracing::info!(vm_id = %vm.vm_id, "Cleaning up VM");

        // Remove VM state directory
        if vm.vm_dir.exists() {
            tokio::fs::remove_dir_all(&vm.vm_dir).await?;
        }

        tracing::debug!(vm_id = %vm.vm_id, "VM cleanup completed");
        Ok(())
    }
}

#[async_trait]
impl WorkerBackend for FirecrackerWorker {
    async fn execute(&self, job: Job) -> Result<WorkResult> {
        tracing::info!(job_id = %job.id, "Executing job in Firecracker VM");

        // Build the VM
        let vm = self.build_vm(&job).await?;

        // Start the VM
        let vm_process = self.start_vm(&vm).await?;

        // Wait for completion
        let result = self.wait_for_completion(vm_process, &vm).await;

        // Cleanup (best effort)
        if let Err(e) = self.cleanup_vm(&vm).await {
            tracing::warn!(
                vm_id = %vm.vm_id,
                error = %e,
                "Failed to cleanup VM"
            );
        }

        result
    }

    async fn initialize(&self) -> Result<()> {
        tracing::info!("Initializing Firecracker worker");

        // Verify Nix is available
        let nix_version = Command::new("nix")
            .args(&["--version"])
            .output()
            .await?;

        if !nix_version.status.success() {
            return Err(anyhow!("Nix is not available"));
        }

        let version = String::from_utf8_lossy(&nix_version.stdout);
        tracing::info!(nix_version = %version.trim(), "Nix version detected");

        // Verify microvm.nix flake is accessible
        let flake_check = Command::new("nix")
            .args(&[
                "flake",
                "metadata",
                &self.config.flake_dir.display().to_string(),
            ])
            .output()
            .await?;

        if !flake_check.status.success() {
            return Err(anyhow!(
                "Failed to access microvm.nix flake at {:?}",
                self.config.flake_dir
            ));
        }

        tracing::info!("Firecracker worker initialized successfully");
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down Firecracker worker");

        // Kill any remaining VMs (emergency cleanup)
        // In production, VMs should already be cleaned up

        tracing::info!("Firecracker worker shutdown complete");
        Ok(())
    }
}

/// Represents a built VM instance
#[derive(Debug)]
struct VmInstance {
    /// Unique VM identifier
    vm_id: String,

    /// Directory for VM state and logs
    vm_dir: PathBuf,

    /// Path to the built VM (Nix store path)
    vm_path: PathBuf,

    /// Memory allocation (MB)
    memory_mb: u32,

    /// vCPU count
    vcpus: u32,
}

/// Represents a running VM process
#[derive(Debug)]
struct VmProcess {
    /// Child process handle
    child: tokio::process::Child,

    /// Path to the VM log file
    log_path: PathBuf,
}
