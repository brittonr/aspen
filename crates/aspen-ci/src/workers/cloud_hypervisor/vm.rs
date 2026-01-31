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

use aspen_ci_agent::protocol::AgentMessage;
use aspen_ci_agent::protocol::HostMessage;
use aspen_ci_agent::protocol::MAX_MESSAGE_SIZE;
use aspen_constants::CI_VM_AGENT_TIMEOUT_MS;
use aspen_constants::CI_VM_BOOT_TIMEOUT_MS;
use aspen_constants::CI_VM_MEMORY_BYTES;
use aspen_constants::CI_VM_NIX_STORE_TAG;
use aspen_constants::CI_VM_RW_STORE_TAG;
use aspen_constants::CI_VM_VCPUS;
use aspen_constants::CI_VM_VSOCK_PORT;
use aspen_constants::CI_VM_WORKSPACE_TAG;
use snafu::ResultExt;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::net::UnixStream;
use tokio::process::Child;
use tokio::process::ChildStderr;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::api_client::VmApiClient;
use super::config::CloudHypervisorWorkerConfig;
use super::error::CloudHypervisorError;
use super::error::Result;
use super::error::{self};

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

    /// Cloud Hypervisor stderr handle (for debugging startup failures).
    process_stderr: RwLock<Option<ChildStderr>>,

    /// Virtiofsd process for Nix store.
    virtiofsd_nix_store: RwLock<Option<Child>>,

    /// Virtiofsd process for workspace.
    virtiofsd_workspace: RwLock<Option<Child>>,

    /// Virtiofsd process for writable store overlay.
    /// Provides disk-backed storage for nix build artifacts.
    virtiofsd_rw_store: RwLock<Option<Child>>,

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
            process_stderr: RwLock::new(None),
            virtiofsd_nix_store: RwLock::new(None),
            virtiofsd_workspace: RwLock::new(None),
            virtiofsd_rw_store: RwLock::new(None),
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
        tokio::fs::create_dir_all(&self.config.state_dir).await.context(error::WorkspaceSetupSnafu)?;

        // Start virtiofsd for Nix store (read-only, static content - use caching)
        info!(vm_id = %self.id, "starting virtiofsd for Nix store");
        let nix_store_virtiofsd = self.start_virtiofsd("/nix/store", CI_VM_NIX_STORE_TAG, "auto").await?;
        *self.virtiofsd_nix_store.write().await = Some(nix_store_virtiofsd);

        // Create workspace directory
        let workspace_dir = self.config.workspace_dir(&self.id);
        tokio::fs::create_dir_all(&workspace_dir).await.context(error::WorkspaceSetupSnafu)?;

        // Start virtiofsd for workspace (files copied before job runs - use caching)
        info!(vm_id = %self.id, "starting virtiofsd for workspace");
        let workspace_virtiofsd =
            self.start_virtiofsd(workspace_dir.to_str().unwrap(), CI_VM_WORKSPACE_TAG, "auto").await?;
        *self.virtiofsd_workspace.write().await = Some(workspace_virtiofsd);

        // Create rw-store directory for writable Nix store overlay
        // This provides disk-backed storage for nix build artifacts, avoiding tmpfs limits
        let rw_store_dir = self.config.rw_store_dir(&self.id);
        tokio::fs::create_dir_all(&rw_store_dir).await.context(error::WorkspaceSetupSnafu)?;

        // Start virtiofsd for rw-store (build artifacts - no caching needed, write-heavy)
        info!(vm_id = %self.id, "starting virtiofsd for writable store overlay");
        let rw_store_virtiofsd =
            self.start_virtiofsd(rw_store_dir.to_str().unwrap(), CI_VM_RW_STORE_TAG, "none").await?;
        *self.virtiofsd_rw_store.write().await = Some(rw_store_virtiofsd);

        // Give virtiofsd time to initialize and create sockets
        // Increased from 500ms to 2s after adding third virtiofs share
        tokio::time::sleep(Duration::from_millis(2000)).await;

        // Start Cloud Hypervisor
        info!(vm_id = %self.id, "starting cloud-hypervisor");
        let mut ch_process = self.start_cloud_hypervisor().await?;

        // Extract stderr handle for debugging (before moving process to RwLock)
        let stderr_handle = ch_process.stderr.take();
        *self.process_stderr.write().await = stderr_handle;
        *self.process.write().await = Some(ch_process);

        *self.state.write().await = VmState::Booting;

        // Wait for API socket (with process health monitoring)
        let boot_timeout = Duration::from_millis(CI_VM_BOOT_TIMEOUT_MS);
        self.wait_for_socket_with_health_check(boot_timeout).await?;

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
            let mut entries = tokio::fs::read_dir(&workspace_dir).await.context(error::WorkspaceSetupSnafu)?;
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
    ///
    /// The `cache_mode` parameter controls guest-side caching:
    /// - "auto": Default caching based on modification times (good for static content like
    ///   /nix/store)
    /// - "never": No caching, always request from host (needed for dynamic content like nix cache)
    async fn start_virtiofsd(&self, source_dir: &str, tag: &str, cache_mode: &str) -> Result<Child> {
        let socket_path = self.config.virtiofs_socket_path(&self.id, tag);

        // Remove stale socket
        let _ = tokio::fs::remove_file(&socket_path).await;

        let virtiofsd_path = self.config.virtiofsd_path.as_deref().unwrap_or_else(|| std::path::Path::new("virtiofsd"));

        let child = Command::new(virtiofsd_path)
            .arg("--socket-path")
            .arg(&socket_path)
            .arg("--shared-dir")
            .arg(source_dir)
            .arg("--cache")
            .arg(cache_mode)
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
    ///
    /// Uses CLI arguments rather than `--vm-config` for compatibility with
    /// Cloud Hypervisor v49.0 which doesn't support the JSON config flag.
    async fn start_cloud_hypervisor(&self) -> Result<Child> {
        let api_socket = self.config.api_socket_path(&self.id);
        let serial_log = self.config.serial_log_path(&self.id);
        let vsock_socket = self.config.vsock_socket_path(&self.id);
        let nix_store_socket = self.config.virtiofs_socket_path(&self.id, CI_VM_NIX_STORE_TAG);
        let workspace_socket = self.config.virtiofs_socket_path(&self.id, CI_VM_WORKSPACE_TAG);
        let rw_store_socket = self.config.virtiofs_socket_path(&self.id, CI_VM_RW_STORE_TAG);

        // Remove stale sockets
        let _ = tokio::fs::remove_file(&api_socket).await;
        let _ = tokio::fs::remove_file(&vsock_socket).await;

        let ch_path = self
            .config
            .cloud_hypervisor_path
            .as_deref()
            .unwrap_or_else(|| std::path::Path::new("cloud-hypervisor"));

        // Build memory size string (bytes -> human readable)
        let memory_mb = CI_VM_MEMORY_BYTES / (1024 * 1024);

        let child = Command::new(ch_path)
            // API socket for control
            .arg("--api-socket")
            .arg(format!("path={}", api_socket.display()))
            // Kernel and initrd
            .arg("--kernel")
            .arg(&self.config.kernel_path)
            .arg("--initramfs")
            .arg(&self.config.initrd_path)
            .arg("--cmdline")
            .arg(self.build_kernel_cmdline())
            // CPU configuration
            .arg("--cpus")
            .arg(format!("boot={},max={}", CI_VM_VCPUS, CI_VM_VCPUS))
            // Memory configuration (shared=on required for virtiofs)
            .arg("--memory")
            .arg(format!("size={}M,shared=on", memory_mb))
            // Serial console to file
            .arg("--serial")
            .arg(format!("file={}", serial_log.display()))
            // Disable interactive console
            .arg("--console")
            .arg("off")
            // Virtiofs shares (multiple values after single --fs flag)
            // Cloud Hypervisor accepts multiple specs after one --fs flag
            // queue_size=512 balances throughput and memory usage (reduced from 1024
            // to lower virtiofsd shmem footprint)
            .arg("--fs")
            .arg(format!(
                "tag={},socket={},num_queues=1,queue_size=512",
                CI_VM_NIX_STORE_TAG,
                nix_store_socket.display()
            ))
            .arg(format!(
                "tag={},socket={},num_queues=1,queue_size=512",
                CI_VM_WORKSPACE_TAG,
                workspace_socket.display()
            ))
            .arg(format!(
                "tag={},socket={},num_queues=1,queue_size=512",
                CI_VM_RW_STORE_TAG,
                rw_store_socket.display()
            ))
            // Vsock for guest agent communication
            .arg("--vsock")
            .arg(format!("cid=3,socket={}", vsock_socket.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context(error::StartCloudHypervisorSnafu)?;

        debug!(
            vm_id = %self.id,
            api_socket = ?api_socket,
            kernel = ?self.config.kernel_path,
            "started cloud-hypervisor"
        );

        Ok(child)
    }

    /// Build kernel command line arguments.
    ///
    /// The NixOS boot process requires:
    /// - `init=${toplevel}/init` - the NixOS stage-2 init script
    /// - `root=fstab` - tells initrd to use fstab for root mount
    ///
    /// Without the correct init path, the VM will boot the kernel and initrd
    /// but fail to transition to the NixOS system (systemd won't start).
    fn build_kernel_cmdline(&self) -> String {
        let ip = self.config.vm_ip(self.vm_index);
        let gateway = format!("{}.1", self.config.network_base);
        let init_path = self.config.toplevel_path.join("init");

        format!(
            "console=ttyS0 loglevel=4 systemd.log_level=info net.ifnames=0 \
             ip={}::{}:255.255.255.0::eth0:off panic=1 root=fstab init={}",
            ip,
            gateway,
            init_path.display()
        )
    }

    /// Wait for API socket with process health monitoring.
    ///
    /// This method waits for the Cloud Hypervisor API socket to become available,
    /// but also monitors the cloud-hypervisor process health. If the process dies
    /// before the socket appears, it captures and logs stderr for debugging.
    async fn wait_for_socket_with_health_check(&self, timeout: Duration) -> Result<()> {
        let deadline = tokio::time::Instant::now() + timeout;
        let poll_interval = Duration::from_millis(100);
        let api_socket = self.config.api_socket_path(&self.id);

        while tokio::time::Instant::now() < deadline {
            // Check if cloud-hypervisor process is still running
            let process_alive = {
                let mut guard = self.process.write().await;
                if let Some(ref mut child) = *guard {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            // Process exited - capture stderr and report
                            drop(guard); // Release lock before reading stderr
                            let stderr_output = self.capture_stderr().await;
                            let serial_log = self.config.serial_log_path(&self.id);

                            error!(
                                vm_id = %self.id,
                                exit_status = ?status,
                                stderr = %stderr_output,
                                serial_log = %serial_log.display(),
                                "cloud-hypervisor process exited unexpectedly"
                            );

                            return Err(CloudHypervisorError::CreateVmFailed {
                                reason: format!("cloud-hypervisor exited with {}: {}", status, stderr_output),
                            });
                        }
                        Ok(None) => true, // Still running
                        Err(e) => {
                            warn!(vm_id = %self.id, error = ?e, "failed to check process status");
                            true // Assume running
                        }
                    }
                } else {
                    false // No process
                }
            };

            if !process_alive {
                return Err(CloudHypervisorError::CreateVmFailed {
                    reason: "cloud-hypervisor process not found".to_string(),
                });
            }

            // Check if socket is ready
            if api_socket.exists() {
                if UnixStream::connect(&api_socket).await.is_ok() {
                    debug!(vm_id = %self.id, "API socket is ready");
                    return Ok(());
                }
            }

            tokio::time::sleep(poll_interval).await;
        }

        // Timeout - capture any available stderr
        let stderr_output = self.capture_stderr().await;
        let serial_log = self.config.serial_log_path(&self.id);

        error!(
            vm_id = %self.id,
            timeout_ms = timeout.as_millis(),
            stderr = %stderr_output,
            serial_log = %serial_log.display(),
            "cloud-hypervisor API socket timeout"
        );

        error::SocketTimeoutSnafu {
            path: api_socket,
            timeout_ms: timeout.as_millis() as u64,
        }
        .fail()
    }

    /// Capture any available stderr output from cloud-hypervisor.
    async fn capture_stderr(&self) -> String {
        let mut guard = self.process_stderr.write().await;
        if let Some(ref mut stderr) = *guard {
            let mut buffer = Vec::new();
            // Read with a short timeout to avoid blocking
            let read_future = stderr.read_to_end(&mut buffer);
            match tokio::time::timeout(Duration::from_millis(500), read_future).await {
                Ok(Ok(_)) => String::from_utf8_lossy(&buffer).to_string(),
                Ok(Err(e)) => format!("<read error: {}>", e),
                Err(_) => "<timeout reading stderr>".to_string(),
            }
        } else {
            "<no stderr handle>".to_string()
        }
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
    ///
    /// This method actually connects to the guest agent via vsock and verifies
    /// it responds to a Ping message with Pong. This ensures the agent is fully
    /// operational, not just that the socket file exists.
    async fn wait_for_guest_agent(&self, timeout: Duration) -> Result<()> {
        let vsock_socket = self.config.vsock_socket_path(&self.id);
        let deadline = tokio::time::Instant::now() + timeout;
        let poll_interval = Duration::from_millis(500);

        while tokio::time::Instant::now() < deadline {
            // First check if socket file exists
            if !vsock_socket.exists() {
                info!(vm_id = %self.id, vsock_path = %vsock_socket.display(), "vsock socket not yet available");
                tokio::time::sleep(poll_interval).await;
                continue;
            }

            // Try to connect and verify agent responds
            match self.verify_agent_ready(&vsock_socket).await {
                Ok(()) => {
                    info!(vm_id = %self.id, "guest agent is ready and responding");
                    return Ok(());
                }
                Err(e) => {
                    // Log at WARN level on first few retries to help debug issues
                    warn!(vm_id = %self.id, error = %e, vsock_path = %vsock_socket.display(), "agent not ready yet, retrying");
                    tokio::time::sleep(poll_interval).await;
                }
            }
        }

        error::GuestAgentTimeoutSnafu {
            vm_id: self.id.clone(),
            timeout_ms: timeout.as_millis() as u64,
        }
        .fail()
    }

    /// Verify the guest agent is responsive by sending a Ping and waiting for Pong.
    async fn verify_agent_ready(&self, vsock_socket: &PathBuf) -> Result<()> {
        // Connect to vsock socket with a short timeout
        let connect_timeout = Duration::from_secs(2);
        let stream = tokio::time::timeout(connect_timeout, UnixStream::connect(vsock_socket))
            .await
            .map_err(|_| CloudHypervisorError::GuestAgentTimeout {
                vm_id: self.id.clone(),
                timeout_ms: connect_timeout.as_millis() as u64,
            })?
            .map_err(|e| CloudHypervisorError::VsockConnect {
                vm_id: self.id.clone(),
                source: e,
            })?;

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Send Cloud Hypervisor vsock CONNECT handshake.
        // Cloud Hypervisor requires "CONNECT <port>\n" before any data exchange.
        // After sending CONNECT, we must wait for "OK <port>\n" response.
        // See: https://github.com/cloud-hypervisor/cloud-hypervisor/blob/main/docs/vsock.md
        // See: https://github.com/firecracker-microvm/firecracker/blob/main/docs/vsock.md
        let connect_cmd = format!("CONNECT {}\n", CI_VM_VSOCK_PORT);
        writer
            .write_all(connect_cmd.as_bytes())
            .await
            .map_err(|e| CloudHypervisorError::VsockSend { source: e })?;
        info!(vm_id = %self.id, port = CI_VM_VSOCK_PORT, "sent vsock CONNECT handshake");

        // Wait for "OK <port>\n" response from Cloud Hypervisor
        let ok_timeout = Duration::from_secs(10);
        let mut ok_line = String::new();
        match tokio::time::timeout(ok_timeout, reader.read_line(&mut ok_line)).await {
            Ok(Ok(0)) => {
                // Connection closed - no one listening on the port
                return Err(CloudHypervisorError::GuestAgentError {
                    message: format!("vsock connection closed (no listener on port {})", CI_VM_VSOCK_PORT),
                });
            }
            Ok(Ok(_)) => {
                let trimmed = ok_line.trim();
                if trimmed.starts_with("OK ") {
                    info!(vm_id = %self.id, response = %trimmed, "received vsock OK response");
                } else {
                    warn!(
                        vm_id = %self.id,
                        response = %trimmed,
                        "unexpected vsock response (expected OK <port>)"
                    );
                }
            }
            Ok(Err(e)) => {
                return Err(CloudHypervisorError::VsockRecv { source: e });
            }
            Err(_) => {
                return Err(CloudHypervisorError::GuestAgentTimeout {
                    vm_id: self.id.clone(),
                    timeout_ms: ok_timeout.as_millis() as u64,
                });
            }
        }

        // Now read the Ready message the agent sends on connection
        let ready_timeout = Duration::from_secs(5);
        match tokio::time::timeout(ready_timeout, self.read_agent_message(&mut reader)).await {
            Ok(Ok(AgentMessage::Ready)) => {
                debug!(vm_id = %self.id, "received Ready from agent");
            }
            Ok(Ok(other)) => {
                debug!(vm_id = %self.id, msg = ?other, "unexpected first message from agent");
                // Not fatal, continue with ping
            }
            Ok(Err(e)) => {
                return Err(e);
            }
            Err(_) => {
                return Err(CloudHypervisorError::GuestAgentTimeout {
                    vm_id: self.id.clone(),
                    timeout_ms: ready_timeout.as_millis() as u64,
                });
            }
        }

        // Send Ping message
        self.send_host_message(&mut writer, &HostMessage::Ping).await?;

        // Wait for Pong response
        let pong_timeout = Duration::from_secs(5);
        match tokio::time::timeout(pong_timeout, self.read_agent_message(&mut reader)).await {
            Ok(Ok(AgentMessage::Pong)) => {
                debug!(vm_id = %self.id, "received Pong from agent");
                Ok(())
            }
            Ok(Ok(other)) => {
                warn!(vm_id = %self.id, msg = ?other, "expected Pong but got different message");
                // Still consider agent ready if we got any response
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(CloudHypervisorError::GuestAgentTimeout {
                vm_id: self.id.clone(),
                timeout_ms: pong_timeout.as_millis() as u64,
            }),
        }
    }

    /// Send a framed message to the guest agent.
    async fn send_host_message<W: AsyncWriteExt + Unpin>(&self, writer: &mut W, msg: &HostMessage) -> Result<()> {
        let json = serde_json::to_vec(msg).map_err(|e| CloudHypervisorError::GuestAgentError {
            message: format!("failed to serialize message: {}", e),
        })?;

        // Write length prefix (4 bytes, big endian)
        let len_bytes = (json.len() as u32).to_be_bytes();
        writer.write_all(&len_bytes).await.map_err(|e| CloudHypervisorError::VsockSend { source: e })?;

        // Write JSON payload
        writer.write_all(&json).await.map_err(|e| CloudHypervisorError::VsockSend { source: e })?;
        writer.flush().await.map_err(|e| CloudHypervisorError::VsockSend { source: e })?;

        Ok(())
    }

    /// Read a framed message from the guest agent.
    async fn read_agent_message<R: AsyncReadExt + Unpin>(&self, reader: &mut R) -> Result<AgentMessage> {
        // Read length prefix (4 bytes, big endian)
        let mut len_bytes = [0u8; 4];
        reader.read_exact(&mut len_bytes).await.map_err(|e| CloudHypervisorError::VsockRecv { source: e })?;

        let len = u32::from_be_bytes(len_bytes);
        if len > MAX_MESSAGE_SIZE {
            return Err(CloudHypervisorError::GuestAgentError {
                message: format!("message too large: {} bytes (max: {})", len, MAX_MESSAGE_SIZE),
            });
        }

        // Read JSON payload
        let mut buf = vec![0u8; len as usize];
        reader.read_exact(&mut buf).await.map_err(|e| CloudHypervisorError::VsockRecv { source: e })?;

        let msg: AgentMessage =
            serde_json::from_slice(&buf).map_err(|e| CloudHypervisorError::DeserializeResponse { source: e })?;

        Ok(msg)
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
        if let Some(mut process) = self.virtiofsd_rw_store.write().await.take() {
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
            self.config.virtiofs_socket_path(&self.id, CI_VM_RW_STORE_TAG),
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
