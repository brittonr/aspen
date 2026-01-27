//! ManagedCiVm - State machine for Cloud Hypervisor CI VMs.
//!
//! This module manages the lifecycle of a single Cloud Hypervisor microVM
//! used for CI job execution. It handles:
//!
//! - VM creation and boot
//! - Guest agent communication via vsock
//! - State transitions (Idle -> Assigned -> Running -> Cleanup -> Idle)
//! - Graceful shutdown and cleanup

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use aspen_constants::{
    CI_VM_AGENT_TIMEOUT_MS, CI_VM_BOOT_TIMEOUT_MS, CI_VM_MEMORY_BYTES, CI_VM_NIX_STORE_TAG,
    CI_VM_VCPUS, CI_VM_WORKSPACE_TAG,
};
use snafu::ResultExt;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::api_client::{
    ConsoleConfig, CpusConfig, FsConfig, MemoryConfig, PayloadConfig, VmApiClient, VmConfig,
    VsockConfig,
};
use super::config::CloudHypervisorWorkerConfig;
use super::error::{self, CloudHypervisorError, Result};

/// State of a managed CI VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmState {
    /// VM process is starting up.
    Creating,
    /// VM is booted, waiting for guest agent.
    Booting,
    /// VM is ready for job assignment.
    Idle,
    /// Job has been assigned, preparing workspace.
    Assigned,
    /// Job is executing in the VM.
    Running,
    /// Job completed, cleaning up workspace.
    Cleanup,
    /// VM is paused (for snapshot).
    Paused,
    /// VM has been shut down.
    Stopped,
    /// VM is in an unrecoverable error state.
    Error,
}

impl std::fmt::Display for VmState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmState::Creating => write!(f, "Creating"),
            VmState::Booting => write!(f, "Booting"),
            VmState::Idle => write!(f, "Idle"),
            VmState::Assigned => write!(f, "Assigned"),
            VmState::Running => write!(f, "Running"),
            VmState::Cleanup => write!(f, "Cleanup"),
            VmState::Paused => write!(f, "Paused"),
            VmState::Stopped => write!(f, "Stopped"),
            VmState::Error => write!(f, "Error"),
        }
    }
}

/// A managed Cloud Hypervisor CI VM.
pub struct ManagedCiVm {
    /// Unique VM identifier.
    pub id: String,

    /// VM configuration.
    config: CloudHypervisorWorkerConfig,

    /// Current VM state.
    state: RwLock<VmState>,

    /// API client for Cloud Hypervisor.
    api: VmApiClient,

    /// Cloud Hypervisor process handle.
    process: RwLock<Option<Child>>,

    /// Virtiofsd process for Nix store.
    virtiofsd_nix_store: RwLock<Option<Child>>,

    /// Virtiofsd process for workspace.
    virtiofsd_workspace: RwLock<Option<Child>>,

    /// Currently assigned job ID.
    current_job: RwLock<Option<String>>,

    /// VM index (for networking).
    vm_index: u32,
}

impl ManagedCiVm {
    /// Create a new managed VM (not yet started).
    pub fn new(config: CloudHypervisorWorkerConfig, vm_index: u32) -> Self {
        let id = config.generate_vm_id(vm_index);
        let api_socket = config.api_socket_path(&id);

        Self {
            id: id.clone(),
            config,
            state: RwLock::new(VmState::Stopped),
            api: VmApiClient::new(api_socket),
            process: RwLock::new(None),
            virtiofsd_nix_store: RwLock::new(None),
            virtiofsd_workspace: RwLock::new(None),
            current_job: RwLock::new(None),
            vm_index,
        }
    }

    /// Get the current VM state.
    pub async fn state(&self) -> VmState {
        *self.state.read().await
    }

    /// Get the currently assigned job ID.
    pub async fn current_job(&self) -> Option<String> {
        self.current_job.read().await.clone()
    }

    /// Start the VM and wait for it to be ready.
    pub async fn start(&self) -> Result<()> {
        let current = self.state().await;
        if current != VmState::Stopped {
            return Err(CloudHypervisorError::InvalidState {
                vm_id: self.id.clone(),
                state: current.to_string(),
                operation: "start".to_string(),
            });
        }

        *self.state.write().await = VmState::Creating;

        // Ensure state directory exists
        tokio::fs::create_dir_all(&self.config.state_dir)
            .await
            .context(error::WorkspaceSetupSnafu)?;

        // Start virtiofsd for Nix store
        info!(vm_id = %self.id, "starting virtiofsd for Nix store");
        let nix_store_virtiofsd = self.start_virtiofsd("/nix/store", CI_VM_NIX_STORE_TAG).await?;
        *self.virtiofsd_nix_store.write().await = Some(nix_store_virtiofsd);

        // Create workspace directory
        let workspace_dir = self.config.workspace_dir(&self.id);
        tokio::fs::create_dir_all(&workspace_dir)
            .await
            .context(error::WorkspaceSetupSnafu)?;

        // Start virtiofsd for workspace
        info!(vm_id = %self.id, "starting virtiofsd for workspace");
        let workspace_virtiofsd = self
            .start_virtiofsd(workspace_dir.to_str().unwrap(), CI_VM_WORKSPACE_TAG)
            .await?;
        *self.virtiofsd_workspace.write().await = Some(workspace_virtiofsd);

        // Give virtiofsd time to initialize
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Start Cloud Hypervisor
        info!(vm_id = %self.id, "starting cloud-hypervisor");
        let ch_process = self.start_cloud_hypervisor().await?;
        *self.process.write().await = Some(ch_process);

        *self.state.write().await = VmState::Booting;

        // Wait for API socket
        let boot_timeout = Duration::from_millis(CI_VM_BOOT_TIMEOUT_MS);
        self.api.wait_for_socket(boot_timeout).await?;

        // Wait for VM to be running
        self.wait_for_vm_running(boot_timeout).await?;

        // Wait for guest agent
        info!(vm_id = %self.id, "waiting for guest agent");
        let agent_timeout = Duration::from_millis(CI_VM_AGENT_TIMEOUT_MS);
        self.wait_for_guest_agent(agent_timeout).await?;

        *self.state.write().await = VmState::Idle;
        info!(vm_id = %self.id, "VM is ready");

        Ok(())
    }

    /// Assign a job to this VM.
    pub async fn assign(&self, job_id: String) -> Result<()> {
        let current = self.state().await;
        if current != VmState::Idle {
            return Err(CloudHypervisorError::InvalidState {
                vm_id: self.id.clone(),
                state: current.to_string(),
                operation: "assign".to_string(),
            });
        }

        *self.state.write().await = VmState::Assigned;
        *self.current_job.write().await = Some(job_id.clone());

        debug!(vm_id = %self.id, job_id = %job_id, "job assigned");
        Ok(())
    }

    /// Mark the VM as running a job.
    pub async fn mark_running(&self) -> Result<()> {
        let current = self.state().await;
        if current != VmState::Assigned {
            return Err(CloudHypervisorError::InvalidState {
                vm_id: self.id.clone(),
                state: current.to_string(),
                operation: "mark_running".to_string(),
            });
        }

        *self.state.write().await = VmState::Running;
        Ok(())
    }

    /// Release the VM back to idle state after job completion.
    pub async fn release(&self) -> Result<()> {
        let current = self.state().await;
        if current != VmState::Running && current != VmState::Assigned {
            return Err(CloudHypervisorError::InvalidState {
                vm_id: self.id.clone(),
                state: current.to_string(),
                operation: "release".to_string(),
            });
        }

        *self.state.write().await = VmState::Cleanup;

        // Clean workspace
        let workspace_dir = self.config.workspace_dir(&self.id);
        if workspace_dir.exists() {
            debug!(vm_id = %self.id, path = ?workspace_dir, "cleaning workspace");
            // Remove contents but keep directory
            let mut entries = tokio::fs::read_dir(&workspace_dir)
                .await
                .context(error::WorkspaceSetupSnafu)?;
            while let Some(entry) = entries.next_entry().await.context(error::WorkspaceSetupSnafu)? {
                let path = entry.path();
                if path.is_dir() {
                    let _ = tokio::fs::remove_dir_all(&path).await;
                } else {
                    let _ = tokio::fs::remove_file(&path).await;
                }
            }
        }

        let job_id = self.current_job.write().await.take();
        *self.state.write().await = VmState::Idle;

        debug!(vm_id = %self.id, job_id = ?job_id, "VM released to pool");
        Ok(())
    }

    /// Pause the VM (for snapshot).
    pub async fn pause(&self) -> Result<()> {
        let current = self.state().await;
        if current != VmState::Idle {
            return Err(CloudHypervisorError::InvalidState {
                vm_id: self.id.clone(),
                state: current.to_string(),
                operation: "pause".to_string(),
            });
        }

        self.api.pause().await?;
        *self.state.write().await = VmState::Paused;

        debug!(vm_id = %self.id, "VM paused");
        Ok(())
    }

    /// Resume the VM from paused state.
    pub async fn resume(&self) -> Result<()> {
        let current = self.state().await;
        if current != VmState::Paused {
            return Err(CloudHypervisorError::InvalidState {
                vm_id: self.id.clone(),
                state: current.to_string(),
                operation: "resume".to_string(),
            });
        }

        self.api.resume().await?;
        *self.state.write().await = VmState::Idle;

        debug!(vm_id = %self.id, "VM resumed");
        Ok(())
    }

    /// Create a snapshot of the VM.
    pub async fn snapshot(&self, dest: &PathBuf) -> Result<()> {
        let current = self.state().await;
        if current != VmState::Paused {
            return Err(CloudHypervisorError::InvalidState {
                vm_id: self.id.clone(),
                state: current.to_string(),
                operation: "snapshot".to_string(),
            });
        }

        let dest_url = format!("file://{}", dest.display());
        self.api.snapshot(&dest_url).await?;

        info!(vm_id = %self.id, dest = ?dest, "snapshot created");
        Ok(())
    }

    /// Shutdown the VM gracefully.
    pub async fn shutdown(&self) -> Result<()> {
        let current = self.state().await;
        if current == VmState::Stopped {
            return Ok(());
        }

        info!(vm_id = %self.id, "shutting down VM");

        // Try graceful shutdown via API
        if let Err(e) = self.api.shutdown().await {
            warn!(vm_id = %self.id, error = ?e, "graceful shutdown failed, force killing");
        }

        // Wait a bit for graceful shutdown
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Kill processes if still running
        self.kill_processes().await;

        // Clean up socket files
        self.cleanup_sockets().await;

        *self.state.write().await = VmState::Stopped;
        *self.current_job.write().await = None;

        info!(vm_id = %self.id, "VM shutdown complete");
        Ok(())
    }

    /// Get the workspace directory path.
    pub fn workspace_dir(&self) -> PathBuf {
        self.config.workspace_dir(&self.id)
    }

    /// Get the vsock socket path for guest agent communication.
    pub fn vsock_socket_path(&self) -> PathBuf {
        self.config.vsock_socket_path(&self.id)
    }

    /// Get the API client for direct VM control.
    pub fn api(&self) -> &VmApiClient {
        &self.api
    }

    // Private methods

    /// Start virtiofsd for a directory share.
    async fn start_virtiofsd(&self, source_dir: &str, tag: &str) -> Result<Child> {
        let socket_path = self.config.virtiofs_socket_path(&self.id, tag);

        // Remove stale socket
        let _ = tokio::fs::remove_file(&socket_path).await;

        let virtiofsd_path = self
            .config
            .virtiofsd_path
            .as_deref()
            .unwrap_or_else(|| std::path::Path::new("virtiofsd"));

        let child = Command::new(virtiofsd_path)
            .arg("--socket-path")
            .arg(&socket_path)
            .arg("--shared-dir")
            .arg(source_dir)
            .arg("--cache")
            .arg("auto")
            .arg("--sandbox")
            .arg("none")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context(error::StartVirtiofsdSnafu)?;

        debug!(
            vm_id = %self.id,
            tag = %tag,
            socket = ?socket_path,
            source = %source_dir,
            "started virtiofsd"
        );

        Ok(child)
    }

    /// Start the Cloud Hypervisor process.
    async fn start_cloud_hypervisor(&self) -> Result<Child> {
        let api_socket = self.config.api_socket_path(&self.id);
        let serial_log = self.config.serial_log_path(&self.id);
        let vsock_socket = self.config.vsock_socket_path(&self.id);

        // Remove stale sockets
        let _ = tokio::fs::remove_file(&api_socket).await;
        let _ = tokio::fs::remove_file(&vsock_socket).await;

        // Build VM config
        let vm_config = self.build_vm_config();
        let config_json = serde_json::to_string(&vm_config).map_err(|e| {
            CloudHypervisorError::CreateVmFailed {
                reason: format!("failed to serialize config: {}", e),
            }
        })?;

        let ch_path = self
            .config
            .cloud_hypervisor_path
            .as_deref()
            .unwrap_or_else(|| std::path::Path::new("cloud-hypervisor"));

        let child = Command::new(ch_path)
            .arg("--api-socket")
            .arg(&api_socket)
            .arg("--serial")
            .arg(format!("file={}", serial_log.display()))
            .arg("--console")
            .arg("off")
            .arg("--vm-config")
            .arg(&config_json)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context(error::StartCloudHypervisorSnafu)?;

        debug!(
            vm_id = %self.id,
            api_socket = ?api_socket,
            "started cloud-hypervisor"
        );

        Ok(child)
    }

    /// Build VM configuration for Cloud Hypervisor.
    fn build_vm_config(&self) -> VmConfig {
        let nix_store_socket = self.config.virtiofs_socket_path(&self.id, CI_VM_NIX_STORE_TAG);
        let workspace_socket = self.config.virtiofs_socket_path(&self.id, CI_VM_WORKSPACE_TAG);
        let vsock_socket = self.config.vsock_socket_path(&self.id);

        VmConfig {
            payload: Some(PayloadConfig {
                kernel: self.config.kernel_path.to_string_lossy().to_string(),
                initramfs: Some(self.config.initrd_path.to_string_lossy().to_string()),
                cmdline: Some(self.build_kernel_cmdline()),
            }),
            cpus: Some(CpusConfig {
                boot_vcpus: CI_VM_VCPUS as u8,
                max_vcpus: CI_VM_VCPUS as u8,
            }),
            memory: Some(MemoryConfig {
                size: CI_VM_MEMORY_BYTES,
                hugepages: None,
                shared: Some(true), // Required for virtiofs
            }),
            serial: Some(ConsoleConfig {
                file: None,
                mode: Some("File".to_string()),
            }),
            console: Some(ConsoleConfig {
                file: None,
                mode: Some("Off".to_string()),
            }),
            disks: None,
            net: None, // TAP networking configured separately
            fs: Some(vec![
                FsConfig {
                    tag: CI_VM_NIX_STORE_TAG.to_string(),
                    socket: nix_store_socket.to_string_lossy().to_string(),
                    num_queues: 1,
                    queue_size: 1024,
                    id: None,
                    pci_segment: None,
                },
                FsConfig {
                    tag: CI_VM_WORKSPACE_TAG.to_string(),
                    socket: workspace_socket.to_string_lossy().to_string(),
                    num_queues: 1,
                    queue_size: 1024,
                    id: None,
                    pci_segment: None,
                },
            ]),
            vsock: Some(VsockConfig {
                cid: 3, // Guest CID (host is always 2)
                socket: vsock_socket.to_string_lossy().to_string(),
                id: None,
                pci_segment: None,
            }),
        }
    }

    /// Build kernel command line arguments.
    fn build_kernel_cmdline(&self) -> String {
        let ip = self.config.vm_ip(self.vm_index);
        let gateway = format!("{}.1", self.config.network_base);

        format!(
            "console=ttyS0 loglevel=4 systemd.log_level=info net.ifnames=0 \
             ip={}::{}:255.255.255.0::eth0:off panic=1 init=/init",
            ip, gateway
        )
    }

    /// Wait for VM to reach Running state.
    async fn wait_for_vm_running(&self, timeout: Duration) -> Result<()> {
        let deadline = tokio::time::Instant::now() + timeout;
        let poll_interval = Duration::from_millis(500);

        while tokio::time::Instant::now() < deadline {
            match self.api.vm_info().await {
                Ok(info) => {
                    debug!(vm_id = %self.id, state = %info.state, "VM state");
                    if info.state == "Running" {
                        return Ok(());
                    }
                }
                Err(e) => {
                    debug!(vm_id = %self.id, error = ?e, "waiting for VM info");
                }
            }
            tokio::time::sleep(poll_interval).await;
        }

        error::BootTimeoutSnafu {
            vm_id: self.id.clone(),
            timeout_ms: timeout.as_millis() as u64,
        }
        .fail()
    }

    /// Wait for guest agent to be ready.
    async fn wait_for_guest_agent(&self, timeout: Duration) -> Result<()> {
        let vsock_socket = self.config.vsock_socket_path(&self.id);
        let deadline = tokio::time::Instant::now() + timeout;
        let poll_interval = Duration::from_secs(1);

        while tokio::time::Instant::now() < deadline {
            // Check if vsock socket exists and is connectable
            if vsock_socket.exists() {
                // TODO: Actually connect to guest agent and verify it responds
                // For now, just check socket exists
                info!(vm_id = %self.id, "guest agent socket available");
                return Ok(());
            }
            tokio::time::sleep(poll_interval).await;
        }

        error::GuestAgentTimeoutSnafu {
            vm_id: self.id.clone(),
            timeout_ms: timeout.as_millis() as u64,
        }
        .fail()
    }

    /// Kill all VM-related processes.
    async fn kill_processes(&self) {
        // Kill cloud-hypervisor
        if let Some(mut process) = self.process.write().await.take() {
            let _ = process.kill().await;
        }

        // Kill virtiofsd processes
        if let Some(mut process) = self.virtiofsd_nix_store.write().await.take() {
            let _ = process.kill().await;
        }
        if let Some(mut process) = self.virtiofsd_workspace.write().await.take() {
            let _ = process.kill().await;
        }
    }

    /// Clean up socket files.
    async fn cleanup_sockets(&self) {
        let sockets = [
            self.config.api_socket_path(&self.id),
            self.config.vsock_socket_path(&self.id),
            self.config.virtiofs_socket_path(&self.id, CI_VM_NIX_STORE_TAG),
            self.config.virtiofs_socket_path(&self.id, CI_VM_WORKSPACE_TAG),
            self.config.console_socket_path(&self.id),
        ];

        for socket in &sockets {
            let _ = tokio::fs::remove_file(socket).await;
        }
    }
}

impl Drop for ManagedCiVm {
    fn drop(&mut self) {
        // Processes are killed on drop due to kill_on_drop(true)
        // Socket cleanup happens in shutdown()
    }
}

/// Shared reference to a managed VM.
pub type SharedVm = Arc<ManagedCiVm>;
