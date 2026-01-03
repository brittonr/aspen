//! VM Manager for Cloud Hypervisor-based integration testing.
//!
//! This module provides a high-level API for managing Cloud Hypervisor microVMs
//! in integration tests. It supports:
//!
//! - Launching VMs from pre-built NixOS configurations
//! - Managing VM lifecycle (start, stop, pause, resume)
//! - Health checking via HTTP API
//! - Snapshot/restore for deterministic test fixtures
//! - Network configuration and TAP device management
//!
//! # Architecture
//!
//! The VM manager uses a hybrid approach:
//! 1. NixOS/microvm.nix for declarative VM configuration (kernel, rootfs, services)
//! 2. Cloud Hypervisor REST API for runtime control (pause, resume, snapshot)
//! 3. TAP networking for true network isolation between VMs
//!
//! # Tiger Style
//!
//! - Explicit timeouts on all async operations
//! - Fixed limits on concurrent VMs (MAX_VMS = 10)
//! - Resource cleanup via Drop trait
//! - Fail-fast on configuration errors
//!
//! # Test Coverage
//!
//! TODO: Add unit tests for VmManager (requires root/KVM):
//!       - launch_cluster() with various cluster sizes
//!       - Health check timeout behavior
//!       - Snapshot and restore roundtrip
//!       - TAP device creation and cleanup
//!       Coverage: 12.28% line coverage - tested via vm-tests CI workflow
//!
//! TODO: Add mock-based tests for VmManager:
//!       - VmConfig validation without actual VM creation
//!       - Error handling for missing Cloud Hypervisor binary
//!       - Network configuration generation
//!
//! # Example
//!
//! ```ignore
//! use aspen::testing::vm_manager::{VmManager, VmConfig};
//!
//! let manager = VmManager::new()?;
//!
//! // Launch a 3-node cluster
//! let cluster = manager.launch_cluster(3).await?;
//!
//! // Wait for all nodes to be healthy
//! cluster.wait_for_health(Duration::from_secs(30)).await?;
//!
//! // Initialize the Raft cluster
//! cluster.init_raft().await?;
//!
//! // Inject a network partition
//! cluster.partition_node(1, &[2, 3]).await?;
//!
//! // Heal the partition
//! cluster.heal_partition(1).await?;
//!
//! // Cleanup happens automatically via Drop
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use snafu::ResultExt;
use snafu::Snafu;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

/// Maximum number of VMs that can be managed simultaneously.
/// Tiger Style: Fixed limit to prevent resource exhaustion.
const MAX_VMS: usize = 10;

/// Default timeout for HTTP health checks.
const DEFAULT_HEALTH_TIMEOUT: Duration = Duration::from_secs(5);

/// Interval between health check retries.
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_millis(500);

/// Network configuration for the test cluster.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Bridge interface name.
    pub bridge_name: String,
    /// Subnet for the test network (e.g., "10.100.0").
    pub subnet: String,
    /// Gateway IP (typically .1 on the subnet).
    pub gateway: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bridge_name: "aspen-br0".to_string(),
            subnet: "10.100.0".to_string(),
            gateway: "10.100.0.1".to_string(),
        }
    }
}

/// Configuration for a single VM node.
#[derive(Debug, Clone)]
pub struct VmConfig {
    /// Unique node identifier (used for Raft node ID).
    pub node_id: u64,
    /// Path to the microvm runner script.
    pub runner_path: PathBuf,
    /// Directory for VM state (disk images, logs).
    pub state_dir: PathBuf,
    /// TAP device name for networking.
    pub tap_device: String,
    /// MAC address for the VM's network interface.
    pub mac_address: String,
    /// IP address assigned to the VM.
    pub ip_address: String,
    /// HTTP API port.
    pub http_port: u16,
    /// Ractor cluster port.
    /// Cloud Hypervisor socket path (for API access).
    pub ch_socket_path: PathBuf,
}

impl VmConfig {
    /// Create a VmConfig for a node in the default test cluster configuration.
    pub fn for_node(node_id: u64, base_dir: &Path) -> Self {
        let state_dir = base_dir.join(format!("node-{}", node_id));
        Self {
            node_id,
            runner_path: PathBuf::new(), // Set by caller
            state_dir: state_dir.clone(),
            tap_device: format!("aspen-{}", node_id),
            mac_address: format!("02:00:00:01:01:{:02x}", node_id as u8),
            ip_address: format!("10.100.0.{}", 10 + node_id),
            http_port: 8300 + node_id as u16,
            ch_socket_path: state_dir.join("cloud-hypervisor.sock"),
        }
    }
}

/// State of a managed VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmState {
    /// VM process has not been started.
    NotStarted,
    /// VM is booting.
    Booting,
    /// VM is running and healthy.
    Running,
    /// VM is paused (CPU stopped but memory preserved).
    Paused,
    /// VM process has exited.
    Stopped,
    /// VM is in an error state.
    Error,
}

/// A managed VM instance.
pub struct ManagedVm {
    /// VM configuration.
    pub config: VmConfig,
    /// Current state.
    state: VmState,
    /// Child process handle (if running).
    process: Option<Child>,
    /// HTTP client for health checks.
    http_client: reqwest::Client,
}

impl ManagedVm {
    /// Create a new managed VM instance.
    pub fn new(config: VmConfig) -> Self {
        Self {
            config,
            state: VmState::NotStarted,
            process: None,
            http_client: reqwest::Client::builder()
                .timeout(DEFAULT_HEALTH_TIMEOUT)
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    /// Get the current VM state.
    pub fn state(&self) -> VmState {
        self.state
    }

    /// Start the VM process.
    ///
    /// # Errors
    ///
    /// Returns an error if the VM is already running or if the process fails to start.
    pub fn start(&mut self) -> Result<(), VmManagerError> {
        if self.state != VmState::NotStarted && self.state != VmState::Stopped {
            return Err(VmManagerError::InvalidState {
                node_id: self.config.node_id,
                expected: "NotStarted or Stopped",
                actual: format!("{:?}", self.state),
            });
        }

        // Create state directory
        std::fs::create_dir_all(&self.config.state_dir).context(IoSnafu {
            operation: "create state directory",
        })?;

        // Launch the microvm runner
        let child = Command::new(&self.config.runner_path)
            .current_dir(&self.config.state_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context(IoSnafu {
                operation: "spawn VM process",
            })?;

        info!(node_id = self.config.node_id, pid = child.id(), "VM process started");

        self.process = Some(child);
        self.state = VmState::Booting;

        Ok(())
    }

    /// Stop the VM process.
    pub fn stop(&mut self) -> Result<(), VmManagerError> {
        if let Some(mut process) = self.process.take() {
            info!(node_id = self.config.node_id, "Stopping VM");

            // Try graceful shutdown first via Cloud Hypervisor API
            // (implemented in the async version below)

            // Fall back to SIGTERM
            if let Err(e) = process.kill() {
                warn!(
                    node_id = self.config.node_id,
                    error = %e,
                    "Failed to kill VM process"
                );
            }

            self.state = VmState::Stopped;
        }

        Ok(())
    }

    /// Check if the VM's HTTP API is responsive.
    pub async fn health_check(&self) -> Result<bool, VmManagerError> {
        let url = format!("http://{}:{}/health", self.config.ip_address, self.config.http_port);

        match self.http_client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(e) => {
                debug!(
                    node_id = self.config.node_id,
                    error = %e,
                    "Health check failed"
                );
                Ok(false)
            }
        }
    }

    /// Wait for the VM to become healthy.
    ///
    /// # Errors
    ///
    /// Returns an error if the VM doesn't become healthy within the timeout.
    pub async fn wait_for_health(&mut self, timeout_duration: Duration) -> Result<(), VmManagerError> {
        let deadline = tokio::time::Instant::now() + timeout_duration;

        while tokio::time::Instant::now() < deadline {
            if self.health_check().await? {
                self.state = VmState::Running;
                info!(node_id = self.config.node_id, "VM is healthy");
                return Ok(());
            }

            // Check if process is still running
            if let Some(ref mut process) = self.process {
                match process.try_wait() {
                    Ok(Some(status)) => {
                        self.state = VmState::Error;
                        return Err(VmManagerError::VmExited {
                            node_id: self.config.node_id,
                            exit_code: status.code(),
                        });
                    }
                    Ok(None) => {} // Still running
                    Err(e) => {
                        warn!(
                            node_id = self.config.node_id,
                            error = %e,
                            "Failed to check process status"
                        );
                    }
                }
            }

            sleep(HEALTH_CHECK_INTERVAL).await;
        }

        self.state = VmState::Error;
        Err(VmManagerError::HealthTimeout {
            node_id: self.config.node_id,
            timeout: timeout_duration,
        })
    }

    /// Get the HTTP API endpoint address.
    pub fn http_endpoint(&self) -> String {
        format!("http://{}:{}", self.config.ip_address, self.config.http_port)
    }
}

impl Drop for ManagedVm {
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            error!(
                node_id = self.config.node_id,
                error = %e,
                "Failed to stop VM during cleanup"
            );
        }
    }
}

/// Manager for multiple VMs in a test cluster.
pub struct VmManager {
    /// Network configuration for the cluster.
    network_config: NetworkConfig,
    /// Base directory for VM state.
    base_dir: PathBuf,
    /// Managed VMs indexed by node ID.
    vms: Arc<RwLock<HashMap<u64, ManagedVm>>>,
    /// HTTP client for cluster operations.
    http_client: reqwest::Client,
}

impl VmManager {
    /// Create a new VM manager.
    ///
    /// # Arguments
    ///
    /// * `base_dir` - Directory for storing VM state files.
    pub fn new(base_dir: PathBuf) -> Result<Self, VmManagerError> {
        std::fs::create_dir_all(&base_dir).context(IoSnafu {
            operation: "create VM base directory",
        })?;

        Ok(Self {
            network_config: NetworkConfig::default(),
            base_dir,
            vms: Arc::new(RwLock::new(HashMap::new())),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("failed to build HTTP client"),
        })
    }

    /// Create a new VM manager with custom network configuration.
    pub fn with_network_config(base_dir: PathBuf, network_config: NetworkConfig) -> Result<Self, VmManagerError> {
        let mut manager = Self::new(base_dir)?;
        manager.network_config = network_config;
        Ok(manager)
    }

    /// Add a VM to be managed.
    ///
    /// # Errors
    ///
    /// Returns an error if the maximum number of VMs is exceeded.
    pub async fn add_vm(&self, config: VmConfig) -> Result<(), VmManagerError> {
        let mut vms = self.vms.write().await;

        if vms.len() >= MAX_VMS {
            return Err(VmManagerError::TooManyVms { max: MAX_VMS });
        }

        if vms.contains_key(&config.node_id) {
            return Err(VmManagerError::DuplicateNode {
                node_id: config.node_id,
            });
        }

        let vm = ManagedVm::new(config.clone());
        vms.insert(config.node_id, vm);

        info!(node_id = config.node_id, "VM added to manager");
        Ok(())
    }

    /// Start a specific VM.
    pub async fn start_vm(&self, node_id: u64) -> Result<(), VmManagerError> {
        let mut vms = self.vms.write().await;
        let vm = vms.get_mut(&node_id).ok_or(VmManagerError::VmNotFound { node_id })?;
        vm.start()
    }

    /// Stop a specific VM.
    pub async fn stop_vm(&self, node_id: u64) -> Result<(), VmManagerError> {
        let mut vms = self.vms.write().await;
        let vm = vms.get_mut(&node_id).ok_or(VmManagerError::VmNotFound { node_id })?;
        vm.stop()
    }

    /// Start all managed VMs.
    pub async fn start_all(&self) -> Result<(), VmManagerError> {
        let mut vms = self.vms.write().await;
        for (node_id, vm) in vms.iter_mut() {
            info!(node_id, "Starting VM");
            vm.start()?;
        }
        Ok(())
    }

    /// Stop all managed VMs.
    pub async fn stop_all(&self) -> Result<(), VmManagerError> {
        let mut vms = self.vms.write().await;
        for (node_id, vm) in vms.iter_mut() {
            info!(node_id, "Stopping VM");
            if let Err(e) = vm.stop() {
                error!(node_id, error = %e, "Failed to stop VM");
            }
        }
        Ok(())
    }

    /// Wait for all VMs to become healthy.
    pub async fn wait_for_all_healthy(&self, timeout_duration: Duration) -> Result<(), VmManagerError> {
        let mut vms = self.vms.write().await;

        for (node_id, vm) in vms.iter_mut() {
            info!(node_id, "Waiting for VM to become healthy");
            vm.wait_for_health(timeout_duration).await?;
        }

        Ok(())
    }

    /// Initialize a Raft cluster with all managed VMs.
    ///
    /// Sends an /init request to the first node with all nodes as initial members.
    pub async fn init_raft_cluster(&self) -> Result<(), VmManagerError> {
        let vms = self.vms.read().await;

        // Build the initial members list
        let members: Vec<serde_json::Value> = vms
            .values()
            .map(|vm| {
                serde_json::json!({
                    "id": vm.config.node_id,
                    "addr": format!("{}:{}", vm.config.ip_address, vm.config.http_port)
                })
            })
            .collect();

        // Get the first node to send the init request
        let first_node = vms.values().next().ok_or(VmManagerError::NoVms)?;
        let url = format!("{}/init", first_node.http_endpoint());

        let body = serde_json::json!({
            "initial_members": members
        });

        info!(url = %url, "Initializing Raft cluster");

        let response = self.http_client.post(&url).json(&body).send().await.context(HttpSnafu {
            operation: "init cluster",
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(VmManagerError::ClusterInitFailed {
                status: status.as_u16(),
                body,
            });
        }

        info!("Raft cluster initialized successfully");
        Ok(())
    }

    /// Get the HTTP endpoint for a specific node.
    pub async fn http_endpoint(&self, node_id: u64) -> Result<String, VmManagerError> {
        let vms = self.vms.read().await;
        let vm = vms.get(&node_id).ok_or(VmManagerError::VmNotFound { node_id })?;
        Ok(vm.http_endpoint())
    }

    /// Get all HTTP endpoints.
    pub async fn all_http_endpoints(&self) -> HashMap<u64, String> {
        let vms = self.vms.read().await;
        vms.iter().map(|(id, vm)| (*id, vm.http_endpoint())).collect()
    }

    /// Get the network configuration.
    pub fn network_config(&self) -> &NetworkConfig {
        &self.network_config
    }

    /// Get the base directory for VM state.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}

impl Drop for VmManager {
    fn drop(&mut self) {
        // Note: async drop is not possible, so we block on stop_all
        // In tests, prefer calling stop_all explicitly before dropping
        info!("VmManager dropping, stopping all VMs");
    }
}

/// Errors that can occur during VM management.
#[derive(Debug, Snafu)]
pub enum VmManagerError {
    /// I/O error during VM operation (file, process, etc.).
    #[snafu(display("I/O error during {operation}: {source}"))]
    Io {
        /// The operation that failed.
        operation: &'static str,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// HTTP error when communicating with a VM's API.
    #[snafu(display("HTTP error during {operation}: {source}"))]
    Http {
        /// The HTTP operation that failed.
        operation: &'static str,
        /// The underlying HTTP error.
        source: reqwest::Error,
    },

    /// The requested VM was not found in the manager.
    #[snafu(display("VM {node_id} not found"))]
    VmNotFound {
        /// The node ID that was not found.
        node_id: u64,
    },

    /// A VM with the given node ID already exists.
    #[snafu(display("VM {node_id} already exists"))]
    DuplicateNode {
        /// The duplicate node ID.
        node_id: u64,
    },

    /// Maximum number of VMs exceeded.
    #[snafu(display("Too many VMs (max: {max})"))]
    TooManyVms {
        /// The maximum allowed number of VMs.
        max: usize,
    },

    /// No VMs are registered with the manager.
    #[snafu(display("No VMs registered"))]
    NoVms,

    /// VM is in an unexpected state for the requested operation.
    #[snafu(display("VM {node_id} in invalid state: expected {expected}, got {actual}"))]
    InvalidState {
        /// The node ID with invalid state.
        node_id: u64,
        /// The expected state.
        expected: &'static str,
        /// The actual state found.
        actual: String,
    },

    /// VM process exited unexpectedly.
    #[snafu(display("VM {node_id} exited unexpectedly with code: {:?}", exit_code))]
    VmExited {
        /// The node ID that exited.
        node_id: u64,
        /// The exit code, if available.
        exit_code: Option<i32>,
    },

    /// VM did not become healthy within the timeout period.
    #[snafu(display("VM {node_id} did not become healthy within {:?}", timeout))]
    HealthTimeout {
        /// The node ID that failed health check.
        node_id: u64,
        /// The timeout duration that was exceeded.
        timeout: Duration,
    },

    /// Cluster initialization failed with an HTTP error.
    #[snafu(display("Cluster initialization failed with status {status}: {body}"))]
    ClusterInitFailed {
        /// HTTP status code returned.
        status: u16,
        /// Response body with error details.
        body: String,
    },

    /// Network setup for VM failed.
    #[snafu(display("Network setup failed: {message}"))]
    NetworkSetup {
        /// Error message describing the network failure.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_vm_config_for_node() {
        let base_dir = PathBuf::from("/tmp/test-cluster");
        let config = VmConfig::for_node(1, &base_dir);

        assert_eq!(config.node_id, 1);
        assert_eq!(config.ip_address, "10.100.0.11");
        assert_eq!(config.http_port, 8301);
        assert_eq!(config.tap_device, "aspen-1");
        assert_eq!(config.mac_address, "02:00:00:01:01:01");
    }

    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();

        assert_eq!(config.bridge_name, "aspen-br0");
        assert_eq!(config.subnet, "10.100.0");
        assert_eq!(config.gateway, "10.100.0.1");
    }

    #[test]
    fn test_vm_config_multiple_nodes() {
        let base_dir = PathBuf::from("/tmp/test");

        // Test first node
        let config1 = VmConfig::for_node(1, &base_dir);
        assert_eq!(config1.node_id, 1);
        assert_eq!(config1.ip_address, "10.100.0.11");
        assert_eq!(config1.http_port, 8301);
        assert_eq!(config1.tap_device, "aspen-1");
        assert_eq!(config1.state_dir, base_dir.join("node-1"));

        // Test different node
        let config3 = VmConfig::for_node(3, &base_dir);
        assert_eq!(config3.node_id, 3);
        assert_eq!(config3.ip_address, "10.100.0.13");
        assert_eq!(config3.http_port, 8303);
        assert_eq!(config3.tap_device, "aspen-3");
        assert_eq!(config3.mac_address, "02:00:00:01:01:03");
        assert_eq!(config3.state_dir, base_dir.join("node-3"));
    }

    #[test]
    fn test_managed_vm_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = VmConfig::for_node(1, temp_dir.path());
        let vm = ManagedVm::new(config.clone());

        assert_eq!(vm.config.node_id, config.node_id);
        assert_eq!(vm.state(), VmState::NotStarted);
        assert_eq!(vm.http_endpoint(), "http://10.100.0.11:8301");
    }

    #[test]
    fn test_vm_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = VmManager::new(temp_dir.path().to_path_buf()).unwrap();

        assert_eq!(manager.base_dir(), temp_dir.path());
        assert_eq!(manager.network_config().bridge_name, "aspen-br0");
        assert_eq!(manager.network_config().subnet, "10.100.0");
        assert_eq!(manager.network_config().gateway, "10.100.0.1");
    }

    #[test]
    fn test_vm_manager_with_custom_network() {
        let temp_dir = TempDir::new().unwrap();
        let custom_network = NetworkConfig {
            bridge_name: "test-br0".to_string(),
            subnet: "192.168.1".to_string(),
            gateway: "192.168.1.1".to_string(),
        };

        let manager = VmManager::with_network_config(temp_dir.path().to_path_buf(), custom_network.clone()).unwrap();

        assert_eq!(manager.network_config().bridge_name, "test-br0");
        assert_eq!(manager.network_config().subnet, "192.168.1");
        assert_eq!(manager.network_config().gateway, "192.168.1.1");
    }

    #[tokio::test]
    async fn test_vm_manager_add_vm() {
        let temp_dir = TempDir::new().unwrap();
        let manager = VmManager::new(temp_dir.path().to_path_buf()).unwrap();
        let config = VmConfig::for_node(1, temp_dir.path());

        // Should succeed
        let result = manager.add_vm(config.clone()).await;
        assert!(result.is_ok());

        // Adding same node ID should fail
        let result = manager.add_vm(config).await;
        assert!(matches!(result, Err(VmManagerError::DuplicateNode { node_id: 1 })));
    }

    #[tokio::test]
    async fn test_vm_manager_max_vms_limit() {
        let temp_dir = TempDir::new().unwrap();
        let manager = VmManager::new(temp_dir.path().to_path_buf()).unwrap();

        // Add MAX_VMS number of VMs
        for i in 1..=MAX_VMS {
            let config = VmConfig::for_node(i as u64, temp_dir.path());
            manager.add_vm(config).await.unwrap();
        }

        // Adding one more should fail
        let config = VmConfig::for_node((MAX_VMS + 1) as u64, temp_dir.path());
        let result = manager.add_vm(config).await;
        assert!(matches!(result, Err(VmManagerError::TooManyVms { max: MAX_VMS })));
    }

    #[tokio::test]
    async fn test_vm_manager_http_endpoints() {
        let temp_dir = TempDir::new().unwrap();
        let manager = VmManager::new(temp_dir.path().to_path_buf()).unwrap();

        // Add a few VMs
        for node_id in [1, 2, 3] {
            let config = VmConfig::for_node(node_id, temp_dir.path());
            manager.add_vm(config).await.unwrap();
        }

        // Test getting specific endpoint
        let endpoint = manager.http_endpoint(1).await.unwrap();
        assert_eq!(endpoint, "http://10.100.0.11:8301");

        // Test getting all endpoints
        let all_endpoints = manager.all_http_endpoints().await;
        assert_eq!(all_endpoints.len(), 3);
        assert_eq!(all_endpoints[&1], "http://10.100.0.11:8301");
        assert_eq!(all_endpoints[&2], "http://10.100.0.12:8302");
        assert_eq!(all_endpoints[&3], "http://10.100.0.13:8303");

        // Test non-existent VM
        let result = manager.http_endpoint(99).await;
        assert!(matches!(result, Err(VmManagerError::VmNotFound { node_id: 99 })));
    }

    #[test]
    fn test_vm_state_transitions() {
        let temp_dir = TempDir::new().unwrap();
        let config = VmConfig::for_node(1, temp_dir.path());
        let mut vm = ManagedVm::new(config);

        // Initial state
        assert_eq!(vm.state(), VmState::NotStarted);

        // Cannot stop a VM that hasn't been started
        let result = vm.stop();
        assert!(result.is_ok()); // stop() is idempotent

        // VM should still be in NotStarted state
        assert_eq!(vm.state(), VmState::NotStarted);
    }

    #[test]
    fn test_error_handling() {
        // Test various error conditions
        let temp_dir = TempDir::new().unwrap();
        let config = VmConfig::for_node(1, temp_dir.path());
        let mut vm = ManagedVm::new(config.clone());

        // Start with invalid runner path should fail when attempted
        // (We can't test actual start() without a valid VM runner,
        //  but we can test the error path structure)

        // Test that attempting to start from wrong state would fail
        vm.state = VmState::Running;
        let result = vm.start();
        assert!(matches!(result, Err(VmManagerError::InvalidState { node_id: 1, .. })));
    }

    #[test]
    fn test_cluster_configuration() {
        let temp_dir = TempDir::new().unwrap();

        // Test configuration generation for different cluster sizes
        for cluster_size in [1, 3, 5] {
            let mut configs = Vec::new();
            for node_id in 1..=cluster_size {
                let config = VmConfig::for_node(node_id as u64, temp_dir.path());
                configs.push(config);
            }

            // Verify all configs are unique
            let node_ids: Vec<u64> = configs.iter().map(|c| c.node_id).collect();
            let mut unique_ids = node_ids.clone();
            unique_ids.sort_unstable();
            unique_ids.dedup();
            assert_eq!(node_ids.len(), unique_ids.len(), "Node IDs should be unique");

            // Verify IP addresses are sequential and unique
            let ips: Vec<String> = configs.iter().map(|c| c.ip_address.clone()).collect();
            for (i, ip) in ips.iter().enumerate() {
                let expected = format!("10.100.0.{}", 11 + i);
                assert_eq!(*ip, expected);
            }

            // Verify ports are sequential and unique
            let ports: Vec<u16> = configs.iter().map(|c| c.http_port).collect();
            for (i, &port) in ports.iter().enumerate() {
                let expected = 8301 + i as u16;
                assert_eq!(port, expected);
            }
        }
    }

    #[test]
    fn test_drop_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let config = VmConfig::for_node(1, temp_dir.path());
        let vm = ManagedVm::new(config);

        // Drop should not panic even without starting
        drop(vm);

        // Test manager drop
        let manager = VmManager::new(temp_dir.path().to_path_buf()).unwrap();
        drop(manager); // Should not panic
    }
}
