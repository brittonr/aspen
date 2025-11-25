// Firecracker MicroVM Worker Backend - V2 using microvm.nix
//
// This implementation uses microvm.nix for declarative VM management
// and shared directories (9p) for job data passing

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::process::Command;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::worker_trait::{WorkerBackend, WorkResult};
use crate::Job;

/// Configuration for Firecracker worker using microvm.nix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirecrackerConfig {
    /// Path to the microvm flake directory
    pub flake_dir: PathBuf,

    /// Path for job data exchange (should match flake's share source)
    /// In production: /var/lib/mvm-ci/jobs
    /// For testing: ~/mvm-ci-test/jobs
    pub job_dir: PathBuf,

    /// Path for VM state files
    /// In production: /var/lib/mvm-ci/vms
    /// For testing: ~/mvm-ci-test/vms
    pub vm_state_dir: PathBuf,

    /// Default memory allocation for VMs (MB)
    pub default_memory_mb: u32,

    /// Default vCPU count for VMs
    pub default_vcpus: u32,

    /// Control plane ticket URL for workers to connect
    pub control_plane_ticket: String,

    /// Maximum number of concurrent VMs
    pub max_concurrent_vms: usize,

    /// Hypervisor to use (cloud-hypervisor for production, qemu for testing)
    pub hypervisor: String,
}

impl Default for FirecrackerConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        Self {
            flake_dir: PathBuf::from("./microvms"),
            job_dir: PathBuf::from(format!("{}/mvm-ci-test/jobs", home)),
            vm_state_dir: PathBuf::from(format!("{}/mvm-ci-test/vms", home)),
            default_memory_mb: 512,
            default_vcpus: 1,
            control_plane_ticket: "http://localhost:3020".to_string(),
            max_concurrent_vms: 10,
            hypervisor: "cloud-hypervisor".to_string(),
        }
    }
}

/// Worker backend that executes jobs using microvm.nix-managed Firecracker VMs
pub struct FirecrackerWorker {
    config: FirecrackerConfig,
    /// Semaphore to limit concurrent VMs
    vm_semaphore: Arc<Semaphore>,
}

impl FirecrackerWorker {
    /// Create a new Firecracker worker
    pub fn new(config: FirecrackerConfig) -> Result<Self> {
        // Validate configuration
        if !config.flake_dir.exists() {
            return Err(anyhow!(
                "Flake directory does not exist: {:?}",
                config.flake_dir
            ));
        }

        // Create necessary directories
        std::fs::create_dir_all(&config.job_dir)?;
        std::fs::create_dir_all(&config.vm_state_dir)?;

        // Create semaphore to limit concurrent VMs
        let vm_semaphore = Arc::new(Semaphore::new(config.max_concurrent_vms));

        tracing::info!(
            flake_dir = ?config.flake_dir,
            job_dir = ?config.job_dir,
            vm_state_dir = ?config.vm_state_dir,
            max_concurrent_vms = config.max_concurrent_vms,
            hypervisor = %config.hypervisor,
            "Firecracker worker v2 initialized with microvm.nix"
        );

        Ok(Self {
            config,
            vm_semaphore,
        })
    }

    /// Execute a job in a microVM
    async fn execute_vm(&self, job: &Job) -> Result<WorkResult> {
        let vm_id = format!("job-{}-{}", job.id, Uuid::new_v4());
        let job_dir = self.config.job_dir.join(&vm_id);
        let vm_state_dir = self.config.vm_state_dir.join(&vm_id);

        tracing::info!(
            job_id = %job.id,
            vm_id = %vm_id,
            "Starting microVM for job execution"
        );

        // Create directories
        fs::create_dir_all(&job_dir).await?;
        fs::create_dir_all(&vm_state_dir).await?;

        // Prepare job execution
        let result = self.run_job_in_vm(&vm_id, job, &job_dir, &vm_state_dir).await;

        // Cleanup (best effort)
        if let Err(e) = self.cleanup_vm(&vm_id, &job_dir, &vm_state_dir).await {
            tracing::warn!(
                vm_id = %vm_id,
                error = %e,
                "Failed to cleanup VM resources"
            );
        }

        result
    }

    /// Run a job in a VM using microvm.nix runner
    async fn run_job_in_vm(
        &self,
        vm_id: &str,
        job: &Job,
        job_dir: &PathBuf,
        vm_state_dir: &PathBuf,
    ) -> Result<WorkResult> {
        // Write job data to shared directory
        let job_file = job_dir.join("job.json");
        let job_json = serde_json::to_string_pretty(job)?;
        fs::write(&job_file, &job_json).await?;

        // Write control plane ticket
        let ticket_file = job_dir.join("ticket");
        fs::write(&ticket_file, &self.config.control_plane_ticket).await?;

        tracing::debug!(
            vm_id = %vm_id,
            job_size = job_json.len(),
            "Job data written to shared directory"
        );

        // Build the command to run the VM
        let mut cmd = Command::new("nix");
        cmd.args(&[
            "run",
            &format!("{}#run-worker-vm", self.config.flake_dir.display()),
            "--",
            job_file.to_str().unwrap(),
            vm_id,
            &self.config.control_plane_ticket,
            &self.config.default_memory_mb.to_string(),
            &self.config.default_vcpus.to_string(),
        ]);

        // Override shared directory paths for testing
        cmd.env("NIX_CONFIG", "experimental-features = nix-command flakes");

        tracing::info!(
            vm_id = %vm_id,
            memory_mb = self.config.default_memory_mb,
            vcpus = self.config.default_vcpus,
            "Launching microVM with microvm.nix"
        );

        // Execute the VM
        let output = cmd.output().await?;

        tracing::debug!(
            vm_id = %vm_id,
            exit_code = output.status.code().unwrap_or(-1),
            stdout_size = output.stdout.len(),
            stderr_size = output.stderr.len(),
            "VM execution completed"
        );

        // Check for completion marker
        let completed_file = job_dir.join("completed");
        if !completed_file.exists() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(WorkResult::failure(format!(
                "VM did not complete job execution: {}",
                stderr
            )));
        }

        // Read exit code
        let exit_code_file = job_dir.join("exit_code");
        let exit_code = if exit_code_file.exists() {
            fs::read_to_string(&exit_code_file)
                .await?
                .trim()
                .parse::<i32>()
                .unwrap_or(-1)
        } else {
            output.status.code().unwrap_or(-1)
        };

        // Read output log for debugging
        let output_log_file = job_dir.join("output.log");
        let output_log = if output_log_file.exists() {
            fs::read_to_string(&output_log_file)
                .await
                .unwrap_or_else(|e| format!("Failed to read output log: {}", e))
        } else {
            String::from_utf8_lossy(&output.stdout).to_string()
        };

        tracing::info!(
            vm_id = %vm_id,
            exit_code = exit_code,
            output_preview = %output_log.lines().take(5).collect::<Vec<_>>().join("\n"),
            "Job execution completed"
        );

        if exit_code == 0 {
            Ok(WorkResult::success())
        } else {
            Ok(WorkResult::failure(format!(
                "Job failed with exit code {}: {}",
                exit_code,
                output_log.lines().last().unwrap_or("Unknown error")
            )))
        }
    }

    /// Cleanup VM resources
    async fn cleanup_vm(
        &self,
        vm_id: &str,
        job_dir: &PathBuf,
        vm_state_dir: &PathBuf,
    ) -> Result<()> {
        tracing::debug!(vm_id = %vm_id, "Cleaning up VM resources");

        // Remove job directory
        if job_dir.exists() {
            fs::remove_dir_all(job_dir).await?;
        }

        // Remove VM state directory
        if vm_state_dir.exists() {
            fs::remove_dir_all(vm_state_dir).await?;
        }

        tracing::debug!(vm_id = %vm_id, "VM cleanup completed");
        Ok(())
    }
}

#[async_trait]
impl WorkerBackend for FirecrackerWorker {
    async fn execute(&self, job: Job) -> Result<WorkResult> {
        tracing::info!(
            job_id = %job.id,
            "Executing job in microVM (v2 with microvm.nix)"
        );

        // Acquire permit from semaphore to limit concurrent VMs
        let _permit = self
            .vm_semaphore
            .acquire()
            .await
            .map_err(|e| anyhow!("Failed to acquire VM semaphore permit: {}", e))?;

        tracing::debug!(
            job_id = %job.id,
            available_permits = self.vm_semaphore.available_permits(),
            "Acquired VM permit"
        );

        // Execute the job in a VM
        let result = self.execute_vm(&job).await;

        // Permit is automatically released when _permit goes out of scope
        tracing::debug!(
            job_id = %job.id,
            available_permits = self.vm_semaphore.available_permits() + 1,
            "Released VM permit"
        );

        result
    }

    async fn initialize(&self) -> Result<()> {
        tracing::info!("Initializing Firecracker worker v2 with microvm.nix");

        // Verify Nix is available
        let nix_version = Command::new("nix").args(&["--version"]).output().await?;

        if !nix_version.status.success() {
            return Err(anyhow!("Nix is not available"));
        }

        let version = String::from_utf8_lossy(&nix_version.stdout);
        tracing::info!(nix_version = %version.trim(), "Nix detected");

        // Verify the microvm flake is valid
        let flake_check = Command::new("nix")
            .args(&[
                "flake",
                "metadata",
                &self.config.flake_dir.display().to_string(),
                "--json",
            ])
            .output()
            .await?;

        if !flake_check.status.success() {
            let stderr = String::from_utf8_lossy(&flake_check.stderr);
            return Err(anyhow!(
                "Failed to access microvm flake at {:?}: {}",
                self.config.flake_dir,
                stderr
            ));
        }

        // Check if QEMU or Firecracker is available based on config
        if self.config.hypervisor == "qemu" {
            let qemu_check = Command::new("which").arg("qemu-system-x86_64").output().await?;
            if !qemu_check.status.success() {
                tracing::warn!("QEMU not found in PATH, VMs may fail to start");
            } else {
                tracing::info!("QEMU available for VM execution");
            }
        } else if self.config.hypervisor == "firecracker" {
            let fc_check = Command::new("which").arg("firecracker").output().await?;
            if !fc_check.status.success() {
                tracing::warn!("Firecracker not found in PATH, VMs may fail to start");
            } else {
                tracing::info!("Firecracker available for VM execution");
            }
        }

        tracing::info!(
            job_dir = ?self.config.job_dir,
            vm_state_dir = ?self.config.vm_state_dir,
            hypervisor = %self.config.hypervisor,
            "Firecracker worker v2 initialized successfully"
        );
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down Firecracker worker v2");

        // Clean up any remaining job directories
        if let Ok(mut entries) = fs::read_dir(&self.config.job_dir).await {
            while let Some(entry) = entries.next_entry().await? {
                if entry.file_type().await?.is_dir() {
                    let path = entry.path();
                    tracing::debug!("Cleaning up leftover job directory: {:?}", path);
                    let _ = fs::remove_dir_all(path).await;
                }
            }
        }

        // Clean up any remaining VM state directories
        if let Ok(mut entries) = fs::read_dir(&self.config.vm_state_dir).await {
            while let Some(entry) = entries.next_entry().await? {
                if entry.file_type().await?.is_dir() {
                    let path = entry.path();
                    tracing::debug!("Cleaning up leftover VM state directory: {:?}", path);
                    let _ = fs::remove_dir_all(path).await;
                }
            }
        }

        tracing::info!("Firecracker worker v2 shutdown complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JobStatus};

    #[tokio::test]
    async fn test_firecracker_worker_creation() {
        // Create test config
        let config = FirecrackerConfig::default();

        // Verify worker can be created
        let worker = FirecrackerWorker::new(config);
        assert!(worker.is_ok(), "Should create worker successfully");
    }

    #[tokio::test]
    async fn test_job_directory_creation() {
        let config = FirecrackerConfig::default();
        let _worker = FirecrackerWorker::new(config.clone()).unwrap();

        // Create a test job
        let job = Job {
            id: "test-job-001".to_string(),
            status: JobStatus::Pending,
            claimed_by: None,
            assigned_worker_id: None,
            completed_by: None,
            created_at: 0,
            updated_at: 0,
            started_at: None,
            error_message: None,
            retry_count: 0,
            payload: serde_json::json!({
                "task": "echo 'test'",
                "type": "simple"
            }),
            compatible_worker_types: vec![],
        };

        // Test job directory preparation
        let vm_id = "test-vm-001";
        let job_dir = config.job_dir.join(vm_id);
        let vm_state_dir = config.vm_state_dir.join(vm_id);

        // Create directories
        std::fs::create_dir_all(&job_dir).unwrap();
        std::fs::create_dir_all(&vm_state_dir).unwrap();

        // Write job file
        let job_file = job_dir.join("job.json");
        std::fs::write(&job_file, serde_json::to_string(&job).unwrap()).unwrap();

        // Verify files exist
        assert!(job_file.exists(), "Job file should exist");

        // Cleanup
        std::fs::remove_dir_all(&job_dir).unwrap();
        std::fs::remove_dir_all(&vm_state_dir).unwrap();
    }
}