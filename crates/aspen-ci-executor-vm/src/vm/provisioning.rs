//! VM provisioning: virtiofsd and Cloud Hypervisor process management.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use aspen_core::CI_VM_NIX_STORE_TAG;
use aspen_core::CI_VM_WORKSPACE_TAG;
use snafu::ResultExt;
use tokio::process::Child;
use tokio::process::Command;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::ManagedCiVm;
use crate::NetworkMode;
use crate::error::CloudHypervisorError;
use crate::error::Result;
use crate::error::{self};

const VIRTIOFSD_NOFILE_LIMIT: libc::rlim_t = 1_048_576;

impl ManagedCiVm {
    pub(super) fn ip_command_path() -> PathBuf {
        if let Some(path) = std::env::var_os("ASPEN_CI_IP_PATH") {
            return PathBuf::from(path);
        }

        let system_ip = PathBuf::from("/run/current-system/sw/bin/ip");
        if system_ip.exists() {
            return system_ip;
        }

        PathBuf::from("ip")
    }

    async fn run_ip_command(&self, args: &[&str], context: &str) -> Result<std::process::Output> {
        let ip_path = Self::ip_command_path();
        let output = Command::new(&ip_path)
            .args(args)
            .output()
            .await
            .map_err(|source| CloudHypervisorError::StartCloudHypervisor { source })?;

        if output.status.success() {
            return Ok(output);
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(CloudHypervisorError::InvalidConfig {
            message: format!("{context}: ip {} failed: {stderr}", args.join(" ")),
        })
    }

    pub(super) async fn run_tap_helper(&self, args: &[&str], context: &str) -> Result<std::process::Output> {
        let helper_path =
            self.config.tap_helper_path.as_deref().ok_or_else(|| CloudHypervisorError::InvalidConfig {
                message: "NetworkMode::TapWithHelper requires tap_helper_path".to_string(),
            })?;
        let output = Command::new(helper_path)
            .args(args)
            .output()
            .await
            .map_err(|source| CloudHypervisorError::StartCloudHypervisor { source })?;

        if output.status.success() {
            return Ok(output);
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(CloudHypervisorError::InvalidConfig {
            message: format!("{context}: helper {} failed: {stderr}", args.join(" ")),
        })
    }

    pub(super) async fn ensure_tap_attached_to_bridge(&self, tap_name: &str) -> Result<()> {
        match self.config.network_mode {
            NetworkMode::TapWithHelper => {
                self.run_tap_helper(&["ensure", tap_name, &self.config.bridge_name], "prepare CI VM TAP device")
                    .await?;
            }
            _ => {
                let show = Command::new(Self::ip_command_path())
                    .args(["link", "show", "dev", tap_name])
                    .output()
                    .await
                    .map_err(|source| CloudHypervisorError::StartCloudHypervisor { source })?;

                if !show.status.success() {
                    self.run_ip_command(&["tuntap", "add", "dev", tap_name, "mode", "tap"], "create CI VM TAP device")
                        .await?;
                }

                self.run_ip_command(
                    &["link", "set", "dev", tap_name, "master", &self.config.bridge_name],
                    "attach CI VM TAP device to bridge",
                )
                .await?;
                self.run_ip_command(&["link", "set", "dev", tap_name, "up"], "bring CI VM TAP device up").await?;
            }
        }

        info!(
            vm_id = %self.id,
            tap = %tap_name,
            bridge = %self.config.bridge_name,
            network_mode = ?self.config.network_mode,
            "attached CI VM TAP device to bridge"
        );

        Ok(())
    }

    /// Start virtiofsd for a directory share.
    ///
    /// The workspace share is handled by an in-process `AspenVirtioFsHandler`
    /// backed by `AspenFs` (KV + iroh-blobs) -- see `vm/lifecycle.rs`. This
    /// method is used for the Nix store share which still uses external virtiofsd.
    ///
    /// DEFERRED: Replace plain virtiofsd for the Nix store with `AspenVirtioFsHandler`
    /// backed by SNIX (store path metadata in KV, NAR content in blobs). Requires
    /// SNIX PathInfoService integration with the virtio-fs layer.
    ///
    /// The `cache_mode` parameter controls guest-side caching:
    /// - "auto": Default caching based on modification times (good for static content like
    ///   /nix/store)
    /// - "never": No caching, always request from host (needed for dynamic content like nix cache)
    pub(super) async fn start_virtiofsd(&self, source_dir: &str, tag: &str, cache_mode: &str) -> Result<Child> {
        let socket_path = self.config.virtiofs_socket_path(&self.id, tag);

        if let Some(parent) = socket_path.parent() {
            tokio::fs::create_dir_all(parent).await.context(error::WorkspaceSetupSnafu)?;
        }

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

        let mut command = Command::new(virtiofsd_path);
        command
            .arg("--socket-path")
            .arg(&socket_path)
            .arg("--shared-dir")
            .arg(source_dir)
            .arg("--cache")
            .arg(cache_mode)
            // Avoid holding one O_PATH file descriptor per indexed inode when
            // traversing large /nix/store source trees. This keeps virtiofsd
            // below ordinary unprivileged RLIMIT_NOFILE ceilings during VM-CI
            // Nix evaluation/copy operations; it falls back to descriptors if
            // the backing filesystem cannot provide handles.
            .arg("--inode-file-handles=prefer")
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
            .kill_on_drop(true);

        // Nix evaluates/copies large nixpkgs source trees through the host-side
        // /nix/store virtiofsd. The guest's sysctl/ulimit settings do not raise
        // this host process limit, so the daemon can still return ENFILE as a
        // guest `chmod ...: Too many open files in system` failure. Raise the
        // daemon soft limit before exec, but never try to raise the inherited
        // hard limit from the unprivileged worker process: doing so makes
        // `setrlimit` fail with EPERM and prevents VM provisioning entirely.
        #[cfg(unix)]
        unsafe {
            command.pre_exec(|| {
                let mut current = libc::rlimit {
                    rlim_cur: 0,
                    rlim_max: 0,
                };
                if libc::getrlimit(libc::RLIMIT_NOFILE, &mut current) != 0 {
                    return Err(std::io::Error::last_os_error());
                }

                let requested = VIRTIOFSD_NOFILE_LIMIT;
                let target_soft = if current.rlim_max == libc::RLIM_INFINITY {
                    requested
                } else {
                    requested.min(current.rlim_max)
                };
                let limit = libc::rlimit {
                    rlim_cur: target_soft.max(current.rlim_cur),
                    rlim_max: current.rlim_max,
                };
                if libc::setrlimit(libc::RLIMIT_NOFILE, &limit) == 0 {
                    Ok(())
                } else {
                    Err(std::io::Error::last_os_error())
                }
            });
        }

        let mut child = command.spawn().context(error::StartVirtiofsdSnafu)?;

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
    pub(super) async fn wait_for_virtiofsd_sockets(&self) -> Result<()> {
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
    pub(super) async fn start_cloud_hypervisor(&self) -> Result<Child> {
        let api_socket = self.config.api_socket_path(&self.id);
        let serial_log = self.config.serial_log_path(&self.id);
        let vsock_socket = self.config.vsock_socket_path(&self.id);
        let nix_store_socket = self.config.virtiofs_socket_path(&self.id, CI_VM_NIX_STORE_TAG);
        let workspace_socket = self.config.virtiofs_socket_path(&self.id, CI_VM_WORKSPACE_TAG);
        // Note: rw-store uses tmpfs inside the VM, not virtiofs

        for socket in [&api_socket, &vsock_socket, &nix_store_socket, &workspace_socket] {
            if let Some(parent) = socket.parent() {
                tokio::fs::create_dir_all(parent).await.context(error::WorkspaceSetupSnafu)?;
            }
        }

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

        // Add network interface based on configured network mode.
        // TAP device allows VM to access cache.nixos.org for substitutes.
        let mac = self.config.vm_mac(self.vm_index);
        match self.config.network_mode {
            NetworkMode::Tap => {
                // Standard TAP mode: create and attach the TAP device before
                // handing it to Cloud Hypervisor. Cloud Hypervisor uses the
                // existing TAP by name and does not bridge it for us.
                // Requires: host bridge (aspen-ci-br0) with NAT configured.
                // Requires: CAP_NET_ADMIN capability or root privileges.
                let tap_name = format!("{}-tap", self.id);
                self.ensure_tap_attached_to_bridge(&tap_name).await?;
                cmd.arg("--net").arg(format!("tap={tap_name},mac={mac}"));
                debug!(
                    vm_id = %self.id,
                    tap_name = %tap_name,
                    "using TAP network mode (requires CAP_NET_ADMIN)"
                );
            }
            NetworkMode::TapWithHelper => {
                // Helper-backed TAP mode: the helper owns the privileged
                // lifecycle operation and this process remains unprivileged.
                let tap_name = format!("{}-tap", self.id);
                self.ensure_tap_attached_to_bridge(&tap_name).await?;
                cmd.arg("--net").arg(format!("tap={tap_name},mac={mac}"));
                debug!(
                    vm_id = %self.id,
                    tap_name = %tap_name,
                    helper = ?self.config.tap_helper_path,
                    "using TAP helper network mode"
                );
            }
            NetworkMode::None => {
                // No network: VM runs in complete isolation.
                // All required store paths must be available via virtiofs.
                info!(
                    vm_id = %self.id,
                    "VM starting without network (isolated mode)"
                );
                // Don't add --net argument at all
            }
        }

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
    /// - `systemd.unit=multi-user.target` - NixOS does not provide a default.target symlink
    /// - `SYSTEMD_UNIT_PATH=...:` - pass the generated NixOS unit tree as an init environment
    ///   variable so stage-2 systemd can find the store-backed unit graph
    ///
    /// Without the correct init path, the VM will boot the kernel and initrd
    /// but fail to transition to the NixOS system (systemd won't start).
    ///
    /// Network is configured via kernel ip= parameter for early boot networking
    /// (only when network mode is not None).
    /// The host bridge (aspen-ci-br0) has IP 10.200.0.1 and provides NAT.
    pub(super) fn build_kernel_cmdline(&self) -> String {
        // Base kernel parameters (always needed)
        let init_path = self.config.toplevel_path.join("init");
        let systemd_unit_path = self.config.toplevel_path.join("etc/systemd/system");
        let base_params = format!(
            "console=ttyS0 loglevel=4 systemd.log_level=info systemd.unit=multi-user.target SYSTEMD_UNIT_PATH={}: panic=1 init={}",
            systemd_unit_path.display(),
            init_path.display()
        );

        // Add network configuration if network is enabled
        match self.config.network_mode {
            NetworkMode::None => {
                // No network: skip ip= and net.ifnames parameters
                base_params
            }
            NetworkMode::Tap | NetworkMode::TapWithHelper => {
                // Network enabled: configure static IP via kernel parameters
                let ip = self.config.vm_ip(self.vm_index);
                let gateway = format!("{}.1", self.config.network_base);

                // ip=<client-IP>:<server-IP>:<gw-IP>:<netmask>:<hostname>:<device>:<autoconf>
                // Using 'off' for autoconf means no DHCP/BOOTP, just static config
                format!("{} net.ifnames=0 ip={}::{}:255.255.255.0::eth0:off", base_params, ip, gateway)
            }
        }
    }
}
