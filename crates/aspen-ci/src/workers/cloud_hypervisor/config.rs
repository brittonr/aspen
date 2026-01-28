//! Configuration for Cloud Hypervisor worker.

use std::path::PathBuf;

/// Configuration for the CloudHypervisorWorker.
#[derive(Debug, Clone)]
pub struct CloudHypervisorWorkerConfig {
    /// Unique node identifier (for generating VM IDs).
    pub node_id: u64,

    /// Directory for VM state (sockets, logs, images).
    /// Default: /var/lib/aspen/ci/vms
    pub state_dir: PathBuf,

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
    /// Default: 8192 (8GB, matches dogfood)
    pub vm_memory_mib: u32,

    /// vCPUs per VM.
    /// Default: 4 (matches dogfood)
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
}

impl Default for CloudHypervisorWorkerConfig {
    fn default() -> Self {
        Self {
            node_id: 1,
            state_dir: PathBuf::from("/var/lib/aspen/ci/vms"),
            cloud_hypervisor_path: None,
            virtiofsd_path: None,
            kernel_path: PathBuf::new(),
            initrd_path: PathBuf::new(),
            toplevel_path: PathBuf::new(),
            pool_size: 2,
            max_vms: 8,
            vm_memory_mib: 8192,
            vm_vcpus: 4,
            boot_timeout_ms: 60_000,
            agent_timeout_ms: 30_000,
            default_execution_timeout_ms: 30 * 60 * 1000,
            max_execution_timeout_ms: 4 * 60 * 60 * 1000,
            enable_snapshots: true,
            snapshot_path: None,
            bridge_name: "aspen-ci-br0".to_string(),
            network_base: "10.200.0".to_string(),
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
        if self.vm_vcpus == 0 {
            return Err("vm_vcpus must be at least 1".to_string());
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

    /// Generate a unique VM ID for this node.
    pub fn generate_vm_id(&self, index: u32) -> String {
        format!("aspen-ci-n{}-vm{}", self.node_id, index)
    }

    /// Get the IP address for a VM.
    pub fn vm_ip(&self, vm_index: u32) -> String {
        format!("{}.{}", self.network_base, 10 + vm_index)
    }

    /// Get the MAC address for a VM.
    pub fn vm_mac(&self, vm_index: u32) -> String {
        format!("02:00:00:c1:{:02x}:{:02x}", self.node_id as u8, vm_index as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_valid() {
        let mut config = CloudHypervisorWorkerConfig::default();
        // Set required paths for validation
        config.kernel_path = PathBuf::new();
        config.initrd_path = PathBuf::new();
        // Should pass with empty paths (won't check existence for empty)
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_socket_paths() {
        let config = CloudHypervisorWorkerConfig::default();
        let vm_id = "aspen-ci-n1-vm0";

        assert!(config.api_socket_path(vm_id).to_string_lossy().contains("-api.sock"));
        assert!(config.vsock_socket_path(vm_id).to_string_lossy().contains("-vsock.sock"));
    }

    #[test]
    fn test_vm_networking() {
        let config = CloudHypervisorWorkerConfig::default();

        assert_eq!(config.vm_ip(0), "10.200.0.10");
        assert_eq!(config.vm_ip(1), "10.200.0.11");
        assert!(config.vm_mac(0).starts_with("02:00:00:c1:"));
    }
}
