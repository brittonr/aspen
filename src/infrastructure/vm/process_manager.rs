//! VM Process Management
//!
//! Handles process spawning, lifecycle, and signal handling for VMs

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use uuid::Uuid;

/// Manages VM processes
#[derive(Clone)]
pub struct VmProcessManager {
    flake_dir: PathBuf,
}

impl VmProcessManager {
    /// Create a new process manager
    pub fn new(flake_dir: PathBuf) -> Self {
        Self { flake_dir }
    }

    /// Build the worker VM if not already built
    pub async fn ensure_worker_vm_built(&self) -> Result<PathBuf> {
        let runner_path = self.flake_dir.join("result/bin/microvm-run");

        if !runner_path.exists() {
            tracing::info!("Building worker VM from flake");
            let build_output = Command::new("nix")
                .args(&[
                    "build",
                    ".#worker-vm",
                    "--out-link",
                    "./result",
                ])
                .current_dir(&self.flake_dir)
                .output()
                .await?;

            if !build_output.status.success() {
                return Err(anyhow!(
                    "Failed to build worker VM: {}",
                    String::from_utf8_lossy(&build_output.stderr)
                ));
            }
        }

        Ok(runner_path)
    }

    /// Build the service VM if not already built
    pub async fn ensure_service_vm_built(&self) -> Result<(PathBuf, PathBuf)> {
        let runner_path = self.flake_dir.join("result-service/bin/microvm-run");
        let virtiofsd_path = self.flake_dir.join("result-service/bin/virtiofsd-run");

        if !runner_path.exists() {
            tracing::info!("Building service VM from flake");
            let build_output = Command::new("nix")
                .args(&[
                    "build",
                    ".#service-vm",
                    "--out-link",
                    "./result-service",
                ])
                .current_dir(&self.flake_dir)
                .output()
                .await?;

            if !build_output.status.success() {
                return Err(anyhow!(
                    "Failed to build service VM: {}",
                    String::from_utf8_lossy(&build_output.stderr)
                ));
            }
        }

        Ok((runner_path, virtiofsd_path))
    }

    /// Spawn ephemeral VM process
    pub async fn spawn_ephemeral_vm(
        &self,
        vm_id: Uuid,
        memory_mb: u32,
        vcpus: u32,
        job_dir: &Path,
        log_dir: &Path,
    ) -> Result<u32> {
        let runner_path = self.ensure_worker_vm_built().await?;

        let mut cmd = Command::new(&runner_path);

        // Set environment variables for the VM
        cmd.env("VM_TYPE", "ephemeral");
        cmd.env("VM_ID", vm_id.to_string());
        cmd.env("VM_MEM_MB", memory_mb.to_string());
        cmd.env("VM_VCPUS", vcpus.to_string());
        cmd.env("JOB_DIR", job_dir.to_str()
            .ok_or_else(|| anyhow!("Job directory path contains invalid UTF-8"))?);

        // Set working directory to flake directory
        cmd.current_dir(&self.flake_dir);

        // Redirect stdout/stderr to files for debugging
        let stdout_file = tokio::fs::File::create(log_dir.join("stdout.log")).await?;
        let stderr_file = tokio::fs::File::create(log_dir.join("stderr.log")).await?;
        cmd.stdout(stdout_file.into_std().await);
        cmd.stderr(stderr_file.into_std().await);

        tracing::info!(
            vm_id = %vm_id,
            runner = %runner_path.display(),
            mem_mb = memory_mb,
            vcpus = vcpus,
            "Starting ephemeral cloud-hypervisor VM"
        );

        let child = cmd.spawn()?;
        let pid = child.id().ok_or_else(|| anyhow!("Failed to get child PID"))?;

        Ok(pid)
    }

    /// Spawn service VM process
    pub async fn spawn_service_vm(
        &self,
        vm_id: Uuid,
        memory_mb: u32,
        vcpus: u32,
        job_dir: &Path,
        control_socket: &Path,
        log_dir: &Path,
        queue_name: &str,
    ) -> Result<u32> {
        let (runner_path, _virtiofsd_path) = self.ensure_service_vm_built().await?;

        let mut cmd = Command::new(&runner_path);

        // Set environment variables for the VM
        cmd.env("VM_TYPE", "service");
        cmd.env("VM_ID", vm_id.to_string());
        cmd.env("VM_MEM_MB", memory_mb.to_string());
        cmd.env("VM_VCPUS", vcpus.to_string());
        cmd.env("CONTROL_SOCKET", control_socket.to_str()
            .ok_or_else(|| anyhow!("Control socket path contains invalid UTF-8"))?);
        cmd.env("JOB_DIR", job_dir.to_str()
            .ok_or_else(|| anyhow!("Job directory path contains invalid UTF-8"))?);

        // Set working directory to flake directory
        cmd.current_dir(&self.flake_dir);

        // Redirect stdout/stderr to files for debugging
        let stdout_file = tokio::fs::File::create(log_dir.join("stdout.log")).await?;
        let stderr_file = tokio::fs::File::create(log_dir.join("stderr.log")).await?;
        cmd.stdout(stdout_file.into_std().await);
        cmd.stderr(stderr_file.into_std().await);

        tracing::info!(
            vm_id = %vm_id,
            runner = %runner_path.display(),
            mem_mb = memory_mb,
            vcpus = vcpus,
            queue = queue_name,
            "Starting service cloud-hypervisor VM"
        );

        let child = cmd.spawn()?;
        let pid = child.id().ok_or_else(|| anyhow!("Failed to get child PID"))?;

        Ok(pid)
    }

    /// Check if process is still running
    pub fn is_process_running(&self, pid: u32) -> bool {
        let pid = nix::unistd::Pid::from_raw(pid as i32);
        nix::sys::signal::kill(pid, None).is_ok()
    }

    /// Send SIGTERM to process
    pub fn send_sigterm(&self, pid: u32) -> Result<()> {
        let pid = nix::unistd::Pid::from_raw(pid as i32);
        nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGTERM)
            .map_err(|e| anyhow!("Failed to send SIGTERM: {}", e))
    }

    /// Send SIGKILL to process
    pub fn send_sigkill(&self, pid: u32) -> Result<()> {
        let pid = nix::unistd::Pid::from_raw(pid as i32);
        match nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGKILL) {
            Ok(_) => Ok(()),
            Err(nix::errno::Errno::ESRCH) => {
                // Process doesn't exist, which is fine
                Ok(())
            }
            Err(e) => Err(anyhow!("Failed to send SIGKILL: {}", e)),
        }
    }
}
