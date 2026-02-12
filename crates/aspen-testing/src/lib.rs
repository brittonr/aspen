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
/// - `DeterministicClusterController`: In-memory cluster controller for testing
/// - `DeterministicKeyValueStore`: In-memory KV store for testing
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
// Deterministic in-memory implementations for testing
pub mod deterministic;

pub mod router;

// Madsim-based deterministic testing (requires simulation feature)
#[cfg(feature = "simulation")]
pub mod madsim_tester;

// Job worker testing utilities (requires simulation feature)
#[cfg(feature = "simulation")]
pub mod job_worker_tester;

// Federation testing utilities
pub mod federation_tester;

// CI pipeline testing utilities (requires ci feature which implies simulation)
#[cfg(feature = "ci")]
pub mod ci_pipeline_tester;

// Pijul multi-node testing (requires pijul feature)
#[cfg(feature = "pijul")]
pub mod pijul_tester;

// VM-based testing modules (only available for testing)
#[cfg(any(test, feature = "testing"))]
pub mod fault_injection;
#[cfg(any(test, feature = "testing"))]
pub mod network_utils;
#[cfg(any(test, feature = "testing"))]
pub mod vm_manager;

// Re-export VM testing types when available
// Re-export Byzantine types for testing (requires simulation)
#[cfg(feature = "simulation")]
pub use aspen_raft::madsim_network::ByzantineCorruptionMode;
#[cfg(feature = "simulation")]
pub use aspen_raft::madsim_network::ByzantineFailureInjector;
use aspen_raft::types::NodeId;
use aspen_raft::types::RaftMemberInfo;
#[cfg(feature = "ci")]
pub use ci_pipeline_tester::CiPipelineTestConfig;
#[cfg(feature = "ci")]
pub use ci_pipeline_tester::CiPipelineTester;
#[cfg(feature = "ci")]
pub use ci_pipeline_tester::MockConfigFetcher;
#[cfg(feature = "ci")]
pub use ci_pipeline_tester::MockPipelineStarter;
#[cfg(feature = "ci")]
pub use ci_pipeline_tester::PipelineExecutionEvent;
#[cfg(feature = "ci")]
pub use ci_pipeline_tester::SimulatedPipelineTracker;
#[cfg(feature = "ci")]
pub use ci_pipeline_tester::multi_stage_test_pipeline;
#[cfg(feature = "ci")]
pub use ci_pipeline_tester::simple_test_pipeline;
#[cfg(feature = "ci")]
pub use ci_pipeline_tester::test_pipeline_context;
// Re-export deterministic implementations
pub use deterministic::DeterministicClusterController;
pub use deterministic::DeterministicKeyValueStore;
#[cfg(any(test, feature = "testing"))]
pub use fault_injection::FaultScenario;
#[cfg(any(test, feature = "testing"))]
pub use fault_injection::LatencyInjection;
#[cfg(any(test, feature = "testing"))]
pub use fault_injection::NetworkPartition;
#[cfg(any(test, feature = "testing"))]
pub use fault_injection::PacketLossInjection;
pub use federation_tester::ClusterContext;
pub use federation_tester::FederationTester;
pub use federation_tester::FederationTesterConfig;
pub use federation_tester::MockDiscoveryService;
pub use federation_tester::NetworkPartitions;
pub use federation_tester::ResourceDataStore;
pub use federation_tester::SyncResult;
pub use federation_tester::SyncStatistics;
pub use federation_tester::SyncableObject;
#[cfg(feature = "simulation")]
pub use job_worker_tester::DeterministicTestWorker;
#[cfg(feature = "simulation")]
pub use job_worker_tester::JobExecutionEvent;
#[cfg(feature = "simulation")]
pub use job_worker_tester::JobExecutionResult;
#[cfg(feature = "simulation")]
pub use job_worker_tester::JobWorkerTestConfig;
#[cfg(feature = "simulation")]
pub use job_worker_tester::JobWorkerTester;
#[cfg(feature = "simulation")]
pub use job_worker_tester::SimulatedJobTracker;
#[cfg(feature = "simulation")]
pub use job_worker_tester::WorkStealingResult;
#[cfg(feature = "simulation")]
pub use job_worker_tester::WorkerLoadStats;
#[cfg(feature = "simulation")]
pub use madsim_tester::AspenRaftTester;
#[cfg(feature = "simulation")]
pub use madsim_tester::BuggifyConfig;
#[cfg(feature = "simulation")]
pub use madsim_tester::BuggifyFault;
#[cfg(feature = "simulation")]
pub use madsim_tester::LivenessConfig;
#[cfg(feature = "simulation")]
pub use madsim_tester::LivenessMetrics;
#[cfg(feature = "simulation")]
pub use madsim_tester::LivenessMode;
#[cfg(feature = "simulation")]
pub use madsim_tester::LivenessReport;
#[cfg(feature = "simulation")]
pub use madsim_tester::LivenessViolation;
#[cfg(feature = "simulation")]
pub use madsim_tester::SimulationMetrics;
#[cfg(feature = "simulation")]
pub use madsim_tester::TesterConfig;
#[cfg(feature = "simulation")]
pub use madsim_tester::ViolationType;
#[cfg(any(test, feature = "testing"))]
pub use network_utils::NetworkBridge;
#[cfg(any(test, feature = "testing"))]
pub use network_utils::TapDevice;
#[cfg(feature = "pijul")]
pub use pijul_tester::PijulMultiNodeTester;
#[cfg(feature = "pijul")]
pub use pijul_tester::PijulTestNode;
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
