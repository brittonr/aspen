/// Testing infrastructure for Aspen distributed system tests.
///
/// This module provides testing primitives at multiple levels:
///
/// ## In-Memory Testing (Fast, Deterministic)
///
/// - `AspenRouter`: Manages multiple in-memory Raft nodes with simulated networking
/// - Wait helpers: Metrics-based assertions via OpenRaft's `Wait` API
/// - Network simulation: Configurable delays, failures, and partitions
/// - `create_test_raft_member_info`: Helper for creating test node metadata
///
/// ## VM-Based Testing (Realistic, Isolated)
///
/// - `VmManager`: Manages Cloud Hypervisor microVMs for integration testing
/// - `NetworkBridge`, `TapDevice`: Network infrastructure for VM isolation
/// - `NetworkPartition`, `LatencyInjection`: Fault injection utilities
///
/// ## Usage Patterns
///
/// ### In-Memory Testing
///
/// ```ignore
/// let config = Arc::new(Config::default().validate()?);
/// let mut router = AspenRouter::new(config);
///
/// router.new_raft_node(0).await;
/// router.new_raft_node(1).await;
/// router.new_raft_node(2).await;
///
/// let node0 = router.get_raft_handle(&0)?;
/// node0.initialize(btreeset! {0,1,2}).await?;
///
/// // Use wait helpers instead of sleep
/// router.wait(&0, timeout()).applied_index(Some(1), "initialized").await?;
/// router.wait(&0, timeout()).current_leader(Some(0), "leader elected").await?;
/// ```
///
/// ### VM-Based Testing
///
/// ```ignore
/// use aspen::testing::vm_manager::{VmManager, VmConfig};
/// use aspen::testing::fault_injection::NetworkPartition;
///
/// let manager = VmManager::new(PathBuf::from("/tmp/aspen-test"))?;
/// manager.add_vm(VmConfig::for_node(1, &base_dir)).await?;
/// manager.add_vm(VmConfig::for_node(2, &base_dir)).await?;
/// manager.add_vm(VmConfig::for_node(3, &base_dir)).await?;
///
/// manager.start_all().await?;
/// manager.wait_for_all_healthy(Duration::from_secs(60)).await?;
/// manager.init_raft_cluster().await?;
///
/// // Inject a network partition
/// let partition = NetworkPartition::create("10.100.0.11", &["10.100.0.12", "10.100.0.13"])?;
/// // ... run tests ...
/// partition.heal()?;
/// ```
pub mod router;

// Madsim-based deterministic testing
pub mod madsim_tester;

// VM-based testing modules (only available for testing)
#[cfg(any(test, feature = "testing"))]
pub mod fault_injection;
#[cfg(any(test, feature = "testing"))]
pub mod network_utils;
#[cfg(any(test, feature = "testing"))]
pub mod vm_manager;

// Re-export VM testing types when available
#[cfg(any(test, feature = "testing"))]
pub use fault_injection::FaultScenario;
#[cfg(any(test, feature = "testing"))]
pub use fault_injection::LatencyInjection;
#[cfg(any(test, feature = "testing"))]
pub use fault_injection::NetworkPartition;
#[cfg(any(test, feature = "testing"))]
pub use fault_injection::PacketLossInjection;
pub use madsim_tester::AspenRaftTester;
pub use madsim_tester::BuggifyConfig;
pub use madsim_tester::BuggifyFault;
pub use madsim_tester::LivenessConfig;
pub use madsim_tester::LivenessMetrics;
pub use madsim_tester::LivenessMode;
pub use madsim_tester::LivenessReport;
pub use madsim_tester::LivenessViolation;
pub use madsim_tester::SimulationMetrics;
pub use madsim_tester::TesterConfig;
pub use madsim_tester::ViolationType;
#[cfg(any(test, feature = "testing"))]
pub use network_utils::NetworkBridge;
#[cfg(any(test, feature = "testing"))]
pub use network_utils::TapDevice;
pub use router::AspenRouter;
#[cfg(any(test, feature = "testing"))]
pub use vm_manager::ManagedVm;
#[cfg(any(test, feature = "testing"))]
pub use vm_manager::NetworkConfig;
#[cfg(any(test, feature = "testing"))]
pub use vm_manager::VmConfig;
#[cfg(any(test, feature = "testing"))]
pub use vm_manager::VmManager;
#[cfg(any(test, feature = "testing"))]
pub use vm_manager::VmState;

// Re-export Byzantine types for testing
pub use crate::raft::madsim_network::{ByzantineCorruptionMode, ByzantineFailureInjector};
use crate::raft::types::NodeId;
use crate::raft::types::RaftMemberInfo;

/// Create a test `RaftMemberInfo` with a deterministic Iroh address derived from the node ID.
///
/// This is used in tests where we don't have real Iroh endpoints.
/// The address is deterministically generated from the node ID to ensure consistency.
///
/// # Example
///
/// ```ignore
/// use aspen::testing::create_test_raft_member_info;
/// use std::collections::BTreeMap;
///
/// let mut nodes = BTreeMap::new();
/// nodes.insert(0, create_test_raft_member_info(0));
/// nodes.insert(1, create_test_raft_member_info(1));
/// raft.initialize(nodes).await?;
/// ```
pub fn create_test_raft_member_info(node_id: impl Into<NodeId>) -> RaftMemberInfo {
    use iroh::EndpointAddr;
    use iroh::EndpointId;
    use iroh::SecretKey;

    let node_id = node_id.into();
    // Generate a deterministic secret key from the node ID
    let mut seed = [0u8; 32];
    seed[..8].copy_from_slice(&node_id.0.to_le_bytes());
    let secret_key = SecretKey::from(seed);
    let endpoint_id: EndpointId = secret_key.public();

    // Create an EndpointAddr with just the ID (no relay URLs or direct addresses for tests)
    let endpoint_addr = EndpointAddr::new(endpoint_id);

    RaftMemberInfo::new(endpoint_addr)
}
