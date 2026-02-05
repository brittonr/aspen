//! Configuration for Cloud Hypervisor worker.

use std::net::IpAddr;
use std::net::SocketAddr;
use std::path::PathBuf;

use aspen_constants::CI_VM_DEFAULT_MEMORY_BYTES;
use aspen_constants::CI_VM_DEFAULT_VCPUS;
use aspen_constants::CI_VM_MAX_MEMORY_BYTES;
use aspen_constants::CI_VM_MAX_VCPUS;

/// Network mode for VM connectivity.
///
/// Determines how the VM gets network access. The default TAP mode requires
/// either root privileges or CAP_NET_ADMIN capability. Alternative modes
/// provide unprivileged networking at the cost of some functionality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetworkMode {
    /// TAP networking with host bridge (requires CAP_NET_ADMIN or root).
    ///
    /// This is the default mode that provides full network access via a TAP
    /// device connected to a host bridge with NAT. VMs can access the internet
    /// (for cache.nixos.org) and the host's Iroh endpoint.
    ///
    /// Requires:
    /// - Host bridge (aspen-ci-br0) configured with NAT
    /// - Either root privileges or CAP_NET_ADMIN on cloud-hypervisor binary
    #[default]
    Tap,

    /// TAP networking with pre-created file descriptor (reduced privileges).
    ///
    /// Uses a helper process with CAP_NET_ADMIN to create the TAP device and
    /// pass the file descriptor to cloud-hypervisor via `fd=` parameter. This
    /// allows the main process to run without elevated privileges.
    ///
    /// The `tap_helper_path` config option must point to a binary with
    /// CAP_NET_ADMIN that creates TAP devices and outputs the FD number.
    TapWithHelper,

    /// No network (isolated VMs).
    ///
    /// VMs have no network access. This mode is useful for builds that don't
    /// need to fetch from cache.nixos.org (all dependencies are in virtiofs
    /// shared /nix/store). Jobs must have all required store paths available
    /// via the host's /nix/store share.
    None,
}

/// Configuration for the CloudHypervisorWorker.
#[derive(Debug, Clone)]
pub struct CloudHypervisorWorkerConfig {
    /// Unique node identifier (for generating VM IDs).
    pub node_id: u64,

    /// Directory for VM state (sockets, logs, images).
    /// Default: /var/lib/aspen/ci/vms
    pub state_dir: PathBuf,

    /// Cluster ticket for VM workers to join the cluster.
    ///
    /// When set, this ticket is written to `/workspace/.aspen-cluster-ticket`
    /// in each VM's workspace directory. The VM's aspen-node reads this ticket
    /// and joins the cluster as an ephemeral worker.
    ///
    /// Without a ticket, VMs cannot join the cluster and will fail to process jobs.
    pub cluster_ticket: Option<String>,

    /// Path to a file containing the cluster ticket.
    ///
    /// If `cluster_ticket` is None and this path is set, the ticket will be
    /// read from this file when VMs are started. This is useful because the
    /// ticket file is typically written after the Iroh endpoint is ready,
    /// which happens after the CloudHypervisorWorker is created.
    ///
    /// The file path is typically `{data_dir}/cluster-ticket.txt`.
    pub cluster_ticket_file: Option<PathBuf>,

    /// Path to the cloud-hypervisor binary.
    /// Default: discovered from PATH
    pub cloud_hypervisor_path: Option<PathBuf>,

    /// Path to the virtiofsd binary.
    /// Default: discovered from PATH
    pub virtiofsd_path: Option<PathBuf>,

    /// Path to the VM kernel.
    /// Built from nix/vms/ci-worker-node.nix.
    pub kernel_path: PathBuf,

    /// Path to the VM initrd.
    /// Built from nix/vms/ci-worker-node.nix.
    pub initrd_path: PathBuf,

    /// Path to the NixOS system toplevel (contains init script).
    /// The kernel cmdline will use ${toplevel}/init.
    /// Built from nix/vms/ci-worker-node.nix.
    pub toplevel_path: PathBuf,

    /// Number of warm VMs to maintain in the pool.
    /// Default: 2
    pub pool_size: u32,

    /// Maximum number of VMs (pool_size + extra for spikes).
    /// Default: 8
    pub max_vms: u32,

    /// Memory per VM in MiB.
    /// Default: 24576 (24GB) - required for large Rust builds with tmpfs overlay.
    /// Configurable via ASPEN_CI_VM_MEMORY_MIB environment variable.
    pub vm_memory_mib: u32,

    /// vCPUs per VM.
    /// Default: 4 (matches dogfood VM configuration).
    /// Configurable via ASPEN_CI_VM_VCPUS environment variable.
    pub vm_vcpus: u32,

    /// VM boot timeout in milliseconds.
    /// Default: 60_000 (60 seconds)
    pub boot_timeout_ms: u64,

    /// Guest agent connection timeout in milliseconds.
    /// Default: 30_000 (30 seconds)
    pub agent_timeout_ms: u64,

    /// Default job execution timeout in milliseconds.
    /// Default: 30 * 60 * 1000 (30 minutes)
    pub default_execution_timeout_ms: u64,

    /// Maximum job execution timeout in milliseconds.
    /// Default: 4 * 60 * 60 * 1000 (4 hours)
    pub max_execution_timeout_ms: u64,

    /// Enable VM snapshot/restore for fast startup.
    /// Default: true
    pub enable_snapshots: bool,

    /// Path to golden snapshot for fast VM creation.
    /// Created on first boot if enable_snapshots is true.
    pub snapshot_path: Option<PathBuf>,

    /// Network bridge name for VM networking.
    /// Default: aspen-ci-br0
    pub bridge_name: String,

    /// Base IP for VM network (VMs get .11, .12, etc.).
    /// Default: 10.200.0
    pub network_base: String,

    /// Destroy VMs after each job instead of reusing them.
    /// Default: true (recommended for CI to ensure clean state)
    ///
    /// When true, each job gets a fresh VM with clean overlay state.
    /// This prevents issues where the tmpfs overlay accumulates state
    /// between jobs, causing library loading failures and other issues.
    ///
    /// When false, VMs are returned to the pool for reuse, which is faster
    /// but can cause state leakage between jobs (e.g., overlay whiteout
    /// files causing "cannot open shared object file" errors).
    pub destroy_after_job: bool,

    /// Port the host's Iroh endpoint is bound to.
    ///
    /// Required for VMs to connect back to the host via the bridge IP.
    /// When set, the bridge address (e.g., 10.200.0.1:PORT) is injected into
    /// the cluster ticket written to VMs, allowing them to reach the host's
    /// Iroh endpoint from the isolated 10.200.0.0/24 network.
    ///
    /// If not set, VMs will only use the addresses directly from the ticket,
    /// which may not be reachable from the VM network.
    pub host_iroh_port: Option<u16>,

    /// Network mode for VM connectivity.
    ///
    /// Controls how VMs get network access:
    /// - `Tap`: Direct TAP device (requires CAP_NET_ADMIN)
    /// - `TapWithHelper`: Use helper binary to create TAP (reduced privileges)
    /// - `None`: No network (fully isolated)
    ///
    /// Default: `NetworkMode::Tap`
    pub network_mode: NetworkMode,

    /// Path to TAP helper binary (for TapWithHelper mode).
    ///
    /// This binary must have CAP_NET_ADMIN capability and will be invoked to
    /// create TAP devices. It receives the TAP device name as an argument and
    /// outputs the file descriptor number on stdout.
    ///
    /// Example helper: `aspen-tap-helper <tap-name> <bridge-name>`
    /// Output: FD number (e.g., "3")
    ///
    /// If None and TapWithHelper mode is selected, falls back to Tap mode.
    pub tap_helper_path: Option<PathBuf>,
}

impl Default for CloudHypervisorWorkerConfig {
    fn default() -> Self {
        Self {
            node_id: 1,
            state_dir: PathBuf::from("/var/lib/aspen/ci/vms"),
            cluster_ticket: None,
            cluster_ticket_file: None,
            cloud_hypervisor_path: None,
            virtiofsd_path: None,
            kernel_path: PathBuf::new(),
            initrd_path: PathBuf::new(),
            toplevel_path: PathBuf::new(),
            pool_size: 2,
            max_vms: 8,
            // 24GB - matches NixOS VM config (ci-worker-node.nix)
            vm_memory_mib: (CI_VM_DEFAULT_MEMORY_BYTES / (1024 * 1024)) as u32,
            vm_vcpus: CI_VM_DEFAULT_VCPUS,
            boot_timeout_ms: 60_000,
            agent_timeout_ms: 30_000,
            default_execution_timeout_ms: 30 * 60 * 1000,
            max_execution_timeout_ms: 4 * 60 * 60 * 1000,
            enable_snapshots: true,
            snapshot_path: None,
            bridge_name: "aspen-ci-br0".to_string(),
            network_base: "10.200.0".to_string(),
            // Destroy VMs after each job by default to ensure clean overlay state.
            // This prevents library loading failures from accumulated whiteout files.
            destroy_after_job: true,
            // No host Iroh port by default - must be set from the running endpoint
            host_iroh_port: None,
            // Default to TAP networking (requires CAP_NET_ADMIN or root)
            network_mode: NetworkMode::default(),
            // No TAP helper by default
            tap_helper_path: None,
        }
    }
}

impl CloudHypervisorWorkerConfig {
    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.pool_size == 0 {
            return Err("pool_size must be at least 1".to_string());
        }
        if self.max_vms < self.pool_size {
            return Err("max_vms must be >= pool_size".to_string());
        }
        if self.vm_memory_mib < 1024 {
            return Err("vm_memory_mib must be at least 1024 (1GB)".to_string());
        }
        let max_memory_mib = (CI_VM_MAX_MEMORY_BYTES / (1024 * 1024)) as u32;
        if self.vm_memory_mib > max_memory_mib {
            return Err(format!("vm_memory_mib {} exceeds maximum {} MiB", self.vm_memory_mib, max_memory_mib));
        }
        if self.vm_vcpus == 0 {
            return Err("vm_vcpus must be at least 1".to_string());
        }
        if self.vm_vcpus > CI_VM_MAX_VCPUS {
            return Err(format!("vm_vcpus {} exceeds maximum {}", self.vm_vcpus, CI_VM_MAX_VCPUS));
        }
        if !self.kernel_path.as_os_str().is_empty() && !self.kernel_path.exists() {
            return Err(format!("kernel_path does not exist: {:?}", self.kernel_path));
        }
        if !self.initrd_path.as_os_str().is_empty() && !self.initrd_path.exists() {
            return Err(format!("initrd_path does not exist: {:?}", self.initrd_path));
        }
        if !self.toplevel_path.as_os_str().is_empty() && !self.toplevel_path.exists() {
            return Err(format!("toplevel_path does not exist: {:?}", self.toplevel_path));
        }
        Ok(())
    }

    /// Get the API socket path for a VM.
    pub fn api_socket_path(&self, vm_id: &str) -> PathBuf {
        self.state_dir.join(format!("{}-api.sock", vm_id))
    }

    /// Get the console socket path for a VM.
    pub fn console_socket_path(&self, vm_id: &str) -> PathBuf {
        self.state_dir.join(format!("{}-console.sock", vm_id))
    }

    /// Get the virtiofs socket path for a VM share.
    pub fn virtiofs_socket_path(&self, vm_id: &str, tag: &str) -> PathBuf {
        self.state_dir.join(format!("{}-virtiofs-{}.sock", vm_id, tag))
    }

    /// Get the vsock path for a VM.
    pub fn vsock_socket_path(&self, vm_id: &str) -> PathBuf {
        self.state_dir.join(format!("{}-vsock.sock", vm_id))
    }

    /// Get the serial log path for a VM.
    pub fn serial_log_path(&self, vm_id: &str) -> PathBuf {
        self.state_dir.join(format!("{}-serial.log", vm_id))
    }

    /// Get the workspace directory for a VM.
    pub fn workspace_dir(&self, vm_id: &str) -> PathBuf {
        self.state_dir.join("workspaces").join(vm_id)
    }

    /// Get the cluster ticket file path for a VM.
    ///
    /// This is the path where the cluster ticket is written for the VM's
    /// aspen-node to read. Located at `/workspace/.aspen-cluster-ticket`
    /// inside the VM (which maps to `workspace_dir/.aspen-cluster-ticket`
    /// on the host).
    pub fn cluster_ticket_path(&self, vm_id: &str) -> PathBuf {
        self.workspace_dir(vm_id).join(".aspen-cluster-ticket")
    }

    /// Get the writable store overlay directory for a VM.
    ///
    /// This provides disk-backed storage for nix build artifacts,
    /// avoiding the tmpfs memory limits that cause "auto-GC" failures.
    pub fn rw_store_dir(&self, vm_id: &str) -> PathBuf {
        self.state_dir.join("rw-stores").join(vm_id)
    }

    /// Generate a unique VM ID for this node.
    /// Note: TAP device names have a 15-char limit (IFNAMSIZ-1), so we use
    /// a short prefix. Format: "ci-n{node}-vm{index}" -> TAP: "ci-n{node}-vm{index}"
    /// Example: ci-n1-vm0 (9 chars) allows TAP suffix to stay under limit.
    pub fn generate_vm_id(&self, index: u32) -> String {
        format!("ci-n{}-vm{}", self.node_id, index)
    }

    /// Get the IP address for a VM.
    pub fn vm_ip(&self, vm_index: u32) -> String {
        format!("{}.{}", self.network_base, 10 + vm_index)
    }

    /// Get the MAC address for a VM.
    pub fn vm_mac(&self, vm_index: u32) -> String {
        format!("02:00:00:c1:{:02x}:{:02x}", self.node_id as u8, vm_index as u8)
    }

    /// Get the bridge socket address for VM connectivity.
    ///
    /// Computes the bridge IP (e.g., 10.200.0.1) from `network_base` and combines
    /// it with `host_iroh_port` to create a socket address that VMs can use to
    /// reach the host's Iroh endpoint.
    ///
    /// Returns `None` if `host_iroh_port` is not set.
    pub fn bridge_socket_addr(&self) -> Option<SocketAddr> {
        self.host_iroh_port.map(|port| {
            let bridge_ip = format!("{}.1", self.network_base);
            let ip: IpAddr = bridge_ip.parse().expect("network_base should produce valid IP");
            SocketAddr::new(ip, port)
        })
    }

    /// Get the cluster ticket, reading from file if necessary.
    ///
    /// Returns the ticket in this order of preference:
    /// 1. `cluster_ticket` if set directly
    /// 2. Contents of `cluster_ticket_file` if set and file exists
    /// 3. None if neither is available
    pub fn get_cluster_ticket(&self) -> Option<String> {
        // First, check if ticket is set directly
        if let Some(ref ticket) = self.cluster_ticket {
            return Some(ticket.clone());
        }

        // Otherwise, try to read from file
        if let Some(ref ticket_file) = self.cluster_ticket_file {
            if ticket_file.exists() {
                match std::fs::read_to_string(ticket_file) {
                    Ok(contents) => {
                        let ticket = contents.trim().to_string();
                        if !ticket.is_empty() {
                            return Some(ticket);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            path = %ticket_file.display(),
                            error = %e,
                            "failed to read cluster ticket file"
                        );
                    }
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_valid() {
        // Set required paths for validation (empty paths pass validation)
        let config = CloudHypervisorWorkerConfig {
            kernel_path: PathBuf::new(),
            initrd_path: PathBuf::new(),
            ..Default::default()
        };
        // Should pass with empty paths (won't check existence for empty)
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_socket_paths() {
        let config = CloudHypervisorWorkerConfig::default();
        let vm_id = "ci-n1-vm0";

        assert!(config.api_socket_path(vm_id).to_string_lossy().contains("-api.sock"));
        assert!(config.vsock_socket_path(vm_id).to_string_lossy().contains("-vsock.sock"));
    }

    #[test]
    fn test_vm_id_length_for_tap() {
        let config = CloudHypervisorWorkerConfig::default();
        // TAP device names have a 15-char limit (IFNAMSIZ-1)
        // VM ID format: ci-n{node}-vm{index} (e.g., ci-n1-vm0 = 9 chars)
        // TAP format: {vm_id}-tap (e.g., ci-n1-vm0-tap = 13 chars) - under limit
        for node_id in 1..=9u64 {
            for vm_idx in 0..=9u32 {
                let mut cfg = config.clone();
                cfg.node_id = node_id;
                let vm_id = cfg.generate_vm_id(vm_idx);
                let tap_name = format!("{}-tap", vm_id);
                assert!(
                    tap_name.len() <= 15,
                    "TAP name {} is {} chars, exceeds 15-char limit",
                    tap_name,
                    tap_name.len()
                );
            }
        }
    }

    #[test]
    fn test_vm_networking() {
        let config = CloudHypervisorWorkerConfig::default();

        assert_eq!(config.vm_ip(0), "10.200.0.10");
        assert_eq!(config.vm_ip(1), "10.200.0.11");
        assert!(config.vm_mac(0).starts_with("02:00:00:c1:"));
    }
}
