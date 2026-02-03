//! ManagedCiVm - State machine for Cloud Hypervisor CI VMs.
//!
//! This module manages the lifecycle of a single Cloud Hypervisor microVM
//! used as an ephemeral CI worker. It handles:
//!
//! - VM creation and boot
//! - Cluster ticket provisioning for worker mode
//! - State transitions (Idle -> Assigned -> Running -> Cleanup -> Idle)
//! - Graceful shutdown and cleanup
//!
//! # Architecture
//!
//! VMs run `aspen-node --worker-only` which:
//! 1. Reads cluster ticket from `/workspace/.aspen-cluster-ticket`
//! 2. Joins the Aspen cluster via Iroh
//! 3. Registers as a worker for `ci_vm` jobs
//! 4. Executes jobs and uploads artifacts to SNIX
//!
//! The VM is an ephemeral cluster participant - it has full access to SNIX
//! for artifact upload but does not participate in Raft consensus.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use aspen_constants::CI_VM_BOOT_TIMEOUT_MS;
use aspen_constants::CI_VM_NIX_STORE_TAG;
use aspen_constants::CI_VM_WORKSPACE_TAG;
use snafu::ResultExt;
use tokio::io::AsyncReadExt;
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

        // Write cluster ticket to workspace for VM's aspen-node to read.
        // The VM runs in worker-only mode and needs the ticket to join the cluster.
        // The ticket is read from config or from a file (since the file may be written
        // after CloudHypervisorWorker is created but before VMs start).
        if let Some(ticket_str) = self.config.get_cluster_ticket() {
            let ticket_path = self.config.cluster_ticket_path(&self.id);

            // If bridge socket address is configured, inject it into the ticket
            // so VMs can reach the host's Iroh endpoint via the bridge IP.
            let final_ticket = if let Some(bridge_addr) = self.config.bridge_socket_addr() {
                // Parse V2 ticket, inject bridge address, re-serialize
                match aspen_ticket::AspenClusterTicketV2::deserialize(&ticket_str) {
                    Ok(mut ticket) => {
                        info!(
                            vm_id = %self.id,
                            bridge_addr = %bridge_addr,
                            "injecting bridge address into VM ticket"
                        );
                        ticket.inject_direct_addr(bridge_addr);
                        ticket.serialize()
                    }
                    Err(e) => {
                        // Fall back to original ticket if parsing fails
                        // (might be V1 ticket or invalid format)
                        warn!(
                            vm_id = %self.id,
                            error = %e,
                            "failed to parse ticket for bridge injection, using original"
                        );
                        ticket_str
                    }
                }
            } else {
                ticket_str
            };

            info!(vm_id = %self.id, ticket_path = %ticket_path.display(), "writing cluster ticket to workspace");
            tokio::fs::write(&ticket_path, &final_ticket).await.context(error::WorkspaceSetupSnafu)?;
        } else {
            warn!(
                vm_id = %self.id,
                ticket_file = ?self.config.cluster_ticket_file,
                "no cluster ticket configured - VM will not be able to join cluster"
            );
        }

        // Note: We use tmpfs for /nix/.rw-store inside the VM instead of virtiofs.
        // virtiofs lacks the filesystem features required by overlayfs for its upper layer
        // (see microvm.nix issue #43). The VM config mounts tmpfs at /nix/.rw-store.

        // Wait for all virtiofsd sockets to be ready before starting Cloud Hypervisor.
        // This is critical for nested virtualization scenarios where socket creation
        // may take longer than the default timeout.
        self.wait_for_virtiofsd_sockets().await?;

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

        // Boot the VM via API (if not already running)
        // Cloud Hypervisor behavior varies by version:
        // - Some versions create VM in "Created" state, requiring explicit boot
        // - Some versions auto-boot with --kernel CLI args
        // Check state first to handle both cases
        let vm_info = self.api.vm_info().await?;
        if vm_info.state == "Running" {
            info!(vm_id = %self.id, "VM already running (auto-booted)");
        } else {
            info!(vm_id = %self.id, state = %vm_info.state, "sending boot command via API");
            self.api.boot().await?;
        }

        // Wait for VM to be running
        self.wait_for_vm_running(boot_timeout).await?;

        // VM is now running aspen-node --worker-only which will:
        // 1. Read cluster ticket from /workspace/.aspen-cluster-ticket
        // 2. Join the cluster via Iroh
        // 3. Register as a worker for ci_vm jobs
        // No guest agent verification needed - the VM is an autonomous cluster participant.

        *self.state.write().await = VmState::Idle;
        info!(vm_id = %self.id, "VM is running (aspen-node will join cluster autonomously)");

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

        info!(
            vm_id = %self.id,
            tag = %tag,
            socket = ?socket_path,
            source = %source_dir,
            virtiofsd = ?virtiofsd_path,
            "starting virtiofsd"
        );

        let mut child = Command::new(virtiofsd_path)
            .arg("--socket-path")
            .arg(&socket_path)
            .arg("--shared-dir")
            .arg(source_dir)
            .arg("--cache")
            .arg(cache_mode)
            .arg("--sandbox")
            .arg("none")
            // Enable POSIX ACL and xattr support - required for overlayfs to work on virtiofs.
            // The --posix-acl flag implies --xattr. Without these, the guest's overlay mount
            // at /nix/store fails with "upper fs missing required features".
            // This matches microvm.nix's virtiofsd configuration.
            .arg("--posix-acl")
            .arg("--xattr")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context(error::StartVirtiofsdSnafu)?;

        // Give virtiofsd a moment to start, then check if it's still running
        // This catches immediate startup failures like missing directories
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check if process exited immediately (indicates startup failure)
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process exited - capture stderr for diagnostics
                let mut stderr_msg = String::new();
                if let Some(mut stderr) = child.stderr.take() {
                    use tokio::io::AsyncReadExt;
                    let mut buf = vec![0u8; 4096];
                    if let Ok(n) = stderr.read(&mut buf).await {
                        stderr_msg = String::from_utf8_lossy(&buf[..n]).to_string();
                    }
                }
                warn!(
                    vm_id = %self.id,
                    tag = %tag,
                    exit_status = ?status,
                    stderr = %stderr_msg,
                    source_dir = %source_dir,
                    "virtiofsd exited immediately"
                );
                return Err(CloudHypervisorError::StartVirtiofsd {
                    source: std::io::Error::other(format!(
                        "virtiofsd exited immediately with status {:?}: {}",
                        status, stderr_msg
                    )),
                });
            }
            Ok(None) => {
                // Still running - good
                debug!(
                    vm_id = %self.id,
                    tag = %tag,
                    socket = ?socket_path,
                    "virtiofsd started successfully"
                );
            }
            Err(e) => {
                warn!(vm_id = %self.id, tag = %tag, error = %e, "failed to check virtiofsd status");
            }
        }

        Ok(child)
    }

    /// Wait for all virtiofsd sockets to become available.
    ///
    /// This method actively polls for each virtiofsd socket to exist and be connectable,
    /// rather than relying on a fixed sleep. This is important for nested virtualization
    /// where socket creation timing can be unpredictable.
    async fn wait_for_virtiofsd_sockets(&self) -> Result<()> {
        use std::os::unix::fs::FileTypeExt;

        // Socket timeout: virtiofsd sockets typically ready in <500ms.
        // 5 seconds provides margin for slow systems while avoiding the
        // previous 30-second timeout that dominated VM startup time.
        const VIRTIOFSD_SOCKET_TIMEOUT_MS: u64 = 5_000;
        // Poll more frequently for faster detection
        const POLL_INTERVAL_MS: u64 = 50;

        // Note: rw-store uses tmpfs inside the VM, not virtiofs
        let sockets = [
            (CI_VM_NIX_STORE_TAG, self.config.virtiofs_socket_path(&self.id, CI_VM_NIX_STORE_TAG)),
            (CI_VM_WORKSPACE_TAG, self.config.virtiofs_socket_path(&self.id, CI_VM_WORKSPACE_TAG)),
        ];

        for (tag, socket_path) in &sockets {
            info!(vm_id = %self.id, tag = %tag, socket = ?socket_path, "waiting for virtiofsd socket");

            // Per-socket deadline: each socket gets its own 30-second timeout
            // Previously a shared deadline was used, causing spurious timeouts
            // when earlier sockets consumed most of the time budget
            let deadline = tokio::time::Instant::now() + Duration::from_millis(VIRTIOFSD_SOCKET_TIMEOUT_MS);

            loop {
                if tokio::time::Instant::now() >= deadline {
                    return Err(CloudHypervisorError::VirtiofsdSocketNotReady {
                        vm_id: self.id.clone(),
                        tag: tag.to_string(),
                        path: socket_path.clone(),
                        timeout_ms: VIRTIOFSD_SOCKET_TIMEOUT_MS,
                    });
                }

                // Check if socket file exists and is a socket file type.
                //
                // IMPORTANT: Do NOT connect to the socket to verify it's ready!
                // virtiofsd uses the vhost-user protocol which expects exactly one client.
                // If we connect and then disconnect, virtiofsd interprets this as
                // "client disconnected" and shuts down, causing Cloud Hypervisor to fail
                // with "Connection to socket failed" when it tries to connect later.
                //
                // Instead, we verify the socket is ready by:
                // 1. Checking the file exists
                // 2. Checking it's a socket file type (not a regular file)
                // 3. Verifying the virtiofsd process is still running (done elsewhere)
                if socket_path.exists() {
                    match tokio::fs::metadata(&socket_path).await {
                        Ok(metadata) => {
                            if metadata.file_type().is_socket() {
                                info!(vm_id = %self.id, tag = %tag, "virtiofsd socket ready");
                                break;
                            } else {
                                debug!(
                                    vm_id = %self.id,
                                    tag = %tag,
                                    "path exists but is not a socket file"
                                );
                            }
                        }
                        Err(e) => {
                            debug!(
                                vm_id = %self.id,
                                tag = %tag,
                                error = %e,
                                "failed to get socket metadata"
                            );
                        }
                    }
                }

                tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
            }
        }

        info!(vm_id = %self.id, "all virtiofsd sockets ready");
        Ok(())
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
        // Note: rw-store uses tmpfs inside the VM, not virtiofs

        // Remove stale sockets
        let _ = tokio::fs::remove_file(&api_socket).await;
        let _ = tokio::fs::remove_file(&vsock_socket).await;

        let ch_path = self
            .config
            .cloud_hypervisor_path
            .as_deref()
            .unwrap_or_else(|| std::path::Path::new("cloud-hypervisor"));

        // Use config values for VM resources (not hardcoded constants)
        // Default is 24GB RAM (matches ci-worker-node.nix), configurable via env vars
        let memory_mib = self.config.vm_memory_mib;
        let vcpus = self.config.vm_vcpus;

        info!(
            vm_id = %self.id,
            memory_mib = memory_mib,
            vcpus = vcpus,
            "starting cloud-hypervisor with configured resources"
        );

        // Build the command with all options
        let mut cmd = Command::new(ch_path);
        cmd
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
            // CPU configuration (from config, default 4 vCPUs)
            .arg("--cpus")
            .arg(format!("boot={},max={}", vcpus, vcpus))
            // Memory configuration (from config, default 24GB; shared=on required for virtiofs)
            .arg("--memory")
            .arg(format!("size={}M,shared=on", memory_mib))
            // Serial console to file
            .arg("--serial")
            .arg(format!("file={}", serial_log.display()))
            // Disable interactive console
            .arg("--console")
            .arg("off")
            // Virtiofs shares - Cloud Hypervisor accepts multiple specs as separate args
            // after a single --fs flag (e.g., --fs spec1 spec2 spec3)
            // queue_size=512 balances throughput and memory usage
            // Note: rw-store uses tmpfs inside the VM (not virtiofs) because virtiofs
            // lacks the filesystem features required by overlayfs for its upper layer
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
            // Vsock for guest agent communication
            .arg("--vsock")
            .arg(format!("cid=3,socket={}", vsock_socket.display()));

        // Add network interface if bridge is configured.
        // TAP device allows VM to access cache.nixos.org for substitutes.
        // Requires: host bridge (aspen-ci-br0) with NAT configured.
        let tap_name = format!("{}-tap", self.id);
        let mac = self.config.vm_mac(self.vm_index);
        cmd.arg("--net").arg(format!("tap={tap_name},mac={mac}"));

        let child = cmd
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
    ///
    /// Network is configured via kernel ip= parameter for early boot networking.
    /// The host bridge (aspen-ci-br0) has IP 10.200.0.1 and provides NAT.
    fn build_kernel_cmdline(&self) -> String {
        let ip = self.config.vm_ip(self.vm_index);
        let gateway = format!("{}.1", self.config.network_base);
        let init_path = self.config.toplevel_path.join("init");

        // ip=<client-IP>:<server-IP>:<gw-IP>:<netmask>:<hostname>:<device>:<autoconf>
        // Using 'off' for autoconf means no DHCP/BOOTP, just static config
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
        let mut last_state = String::new();
        let mut polls = 0u32;

        while tokio::time::Instant::now() < deadline {
            polls += 1;
            match self.api.vm_info().await {
                Ok(info) => {
                    // Log state changes at info level for visibility
                    if info.state != last_state {
                        info!(vm_id = %self.id, state = %info.state, polls = polls, "VM state changed");
                        last_state = info.state.clone();
                    } else if polls.is_multiple_of(20) {
                        // Log every 10 seconds (20 polls * 500ms) if still waiting
                        info!(vm_id = %self.id, state = %info.state, polls = polls, elapsed_s = polls / 2, "still waiting for VM Running state");
                    }
                    if info.state == "Running" {
                        return Ok(());
                    }
                }
                Err(e) => {
                    if polls.is_multiple_of(20) {
                        warn!(vm_id = %self.id, error = ?e, polls = polls, "API query failed while waiting for VM");
                    } else {
                        debug!(vm_id = %self.id, error = ?e, "waiting for VM info");
                    }
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
