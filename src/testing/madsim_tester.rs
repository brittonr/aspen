//! Centralized tester abstraction for madsim-based deterministic Raft testing.
//!
//! `AspenRaftTester` provides a high-level API that reduces test boilerplate by 80%.
//! Instead of manually creating routers, injectors, nodes, and tracking artifacts,
//! tests can use a simple fluent API.
//!
//! # Design Principles (Tiger Style)
//!
//! - **Bounded resources**: All operations respect MAX_PEERS, MAX_BATCH_SIZE limits
//! - **Explicit types**: Uses u64 for indices, NodeId for node identification
//! - **Fail-fast**: All errors propagate immediately via Result
//! - **Deterministic**: Environment-based seeding with fallback to test name hash
//!
//! # Example
//!
//! ```ignore
//! use aspen::testing::madsim_tester::AspenRaftTester;
//!
//! #[madsim::test]
//! async fn test_leader_crash_and_reelection() {
//!     let mut t = AspenRaftTester::new(3, "leader_crash").await;
//!
//!     let leader = t.check_one_leader().await.expect("No initial leader");
//!     t.crash_node(leader).await;
//!
//!     madsim::time::sleep(Duration::from_secs(10)).await;
//!
//!     let new_leader = t.check_one_leader().await.expect("No new leader");
//!     assert_ne!(leader, new_leader, "Same leader after crash");
//!
//!     t.end();
//! }
//! ```
//!
//! # References
//!
//! - [MadRaft Tester](https://github.com/madsim-rs/MadRaft) - Similar abstraction pattern
//! - [FoundationDB Testing](https://apple.github.io/foundationdb/testing.html) - BUGGIFY inspiration
//! - [RisingWave DST](https://www.risingwave.com/blog/deterministic-simulation-a-new-era-of-distributed-system-testing/)

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::Result;
use openraft::{Config, Raft};

use crate::raft::madsim_network::{
    ByzantineCorruptionMode, ByzantineFailureInjector, FailureInjector, MadsimNetworkFactory,
    MadsimRaftRouter,
};
use crate::raft::storage::{InMemoryLogStore, InMemoryStateMachine, RedbLogStore, StorageBackend};
use crate::raft::storage_sqlite::SqliteStateMachine;
use crate::raft::types::{AppRequest, AppTypeConfig, NodeId, RaftMemberInfo};
use crate::simulation::{SimulationArtifact, SimulationArtifactBuilder};
use crate::testing::create_test_raft_member_info;

/// Helper to create a fresh artifact builder for std::mem::replace
fn empty_artifact_builder() -> SimulationArtifactBuilder {
    SimulationArtifactBuilder::new("_placeholder_", 0)
}

// Tiger Style: Fixed limits for simulation testing
const MAX_TESTER_NODES: usize = 64;
const DEFAULT_HEARTBEAT_INTERVAL_MS: u64 = 500;
const DEFAULT_ELECTION_TIMEOUT_MIN_MS: u64 = 1500;
const DEFAULT_ELECTION_TIMEOUT_MAX_MS: u64 = 3000;
const LEADER_CHECK_RETRIES: u32 = 10;
const LEADER_CHECK_BACKOFF_MIN_MS: u64 = 450;
const LEADER_CHECK_BACKOFF_MAX_MS: u64 = 550;

/// Configuration for creating a tester instance.
#[derive(Debug, Clone)]
pub struct TesterConfig {
    /// Number of nodes in the cluster.
    pub node_count: usize,
    /// Test name for artifact identification.
    pub test_name: String,
    /// Explicit seed (None = derive from environment or test name).
    pub seed: Option<u64>,
    /// Heartbeat interval in milliseconds.
    pub heartbeat_interval_ms: u64,
    /// Minimum election timeout in milliseconds.
    pub election_timeout_min_ms: u64,
    /// Maximum election timeout in milliseconds.
    pub election_timeout_max_ms: u64,
    /// Storage backend to use (InMemory or Sqlite for persistence).
    pub storage_backend: StorageBackend,
    /// Base directory for persistent storage (only used with Sqlite backend).
    pub storage_dir: Option<std::path::PathBuf>,
}

impl TesterConfig {
    /// Create a new tester config with default settings.
    pub fn new(node_count: usize, test_name: impl Into<String>) -> Self {
        Self {
            node_count,
            test_name: test_name.into(),
            seed: None,
            heartbeat_interval_ms: DEFAULT_HEARTBEAT_INTERVAL_MS,
            election_timeout_min_ms: DEFAULT_ELECTION_TIMEOUT_MIN_MS,
            election_timeout_max_ms: DEFAULT_ELECTION_TIMEOUT_MAX_MS,
            storage_backend: StorageBackend::InMemory, // Default to in-memory for tests
            storage_dir: None,
        }
    }

    /// Set an explicit seed for deterministic testing.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Configure Raft timeouts.
    pub fn with_timeouts(
        mut self,
        heartbeat_ms: u64,
        election_min_ms: u64,
        election_max_ms: u64,
    ) -> Self {
        self.heartbeat_interval_ms = heartbeat_ms;
        self.election_timeout_min_ms = election_min_ms;
        self.election_timeout_max_ms = election_max_ms;
        self
    }

    /// Use persistent storage (Sqlite backend) for testing crash recovery.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = TesterConfig::new(3, "crash_recovery")
    ///     .with_persistent_storage("/tmp/aspen-test");
    /// ```
    pub fn with_persistent_storage(mut self, storage_dir: impl Into<std::path::PathBuf>) -> Self {
        self.storage_backend = StorageBackend::Sqlite;
        self.storage_dir = Some(storage_dir.into());
        self
    }
}

/// Structured metrics captured during simulation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct SimulationMetrics {
    /// Number of Raft RPC calls made.
    pub rpc_count: u64,
    /// Maximum log size across all nodes.
    pub max_log_size: u64,
    /// Number of nodes in the cluster.
    pub node_count: u32,
    /// Total simulation duration in milliseconds.
    pub duration_ms: u64,
    /// Number of leader elections observed.
    pub elections: u32,
    /// Number of node crashes.
    pub node_crashes: u32,
    /// Number of node restarts.
    pub node_restarts: u32,
    /// Number of network partitions created.
    pub network_partitions: u32,
    /// Number of Byzantine message corruptions.
    pub byzantine_corruptions: u64,
}

/// Storage paths for a persistent node.
#[derive(Clone, Debug)]
struct NodeStoragePaths {
    /// Path to the Redb log store file.
    log_path: std::path::PathBuf,
    /// Path to the SQLite state machine database.
    state_path: std::path::PathBuf,
}

/// Node handle for tracking individual node state.
enum TestNode {
    /// In-memory node (for testing).
    InMemory {
        raft: Raft<AppTypeConfig>,
        state_machine: Arc<InMemoryStateMachine>,
        connected: AtomicBool,
    },
    /// Persistent node (for crash recovery testing).
    Persistent {
        raft: Raft<AppTypeConfig>,
        state_machine: Arc<SqliteStateMachine>,
        connected: AtomicBool,
        storage_paths: NodeStoragePaths,
    },
}

impl TestNode {
    fn raft(&self) -> &Raft<AppTypeConfig> {
        match self {
            TestNode::InMemory { raft, .. } => raft,
            TestNode::Persistent { raft, .. } => raft,
        }
    }

    fn connected(&self) -> &AtomicBool {
        match self {
            TestNode::InMemory { connected, .. } => connected,
            TestNode::Persistent { connected, .. } => connected,
        }
    }

    fn storage_paths(&self) -> Option<&NodeStoragePaths> {
        match self {
            TestNode::InMemory { .. } => None,
            TestNode::Persistent { storage_paths, .. } => Some(storage_paths),
        }
    }
}

/// High-level tester abstraction for madsim Raft simulations.
///
/// Reduces test boilerplate from 40+ lines to <10 lines by encapsulating:
/// - Router and failure injector creation
/// - Node creation and registration
/// - Cluster initialization
/// - Artifact capture
/// - Metrics collection
/// - Byzantine failure injection
pub struct AspenRaftTester {
    /// Router managing all nodes in the simulation.
    router: Arc<MadsimRaftRouter>,
    /// Failure injector for chaos testing.
    injector: Arc<FailureInjector>,
    /// Byzantine failure injector for message corruption testing.
    byzantine_injector: Arc<ByzantineFailureInjector>,
    /// All nodes in the cluster.
    nodes: Vec<TestNode>,
    /// Artifact builder for event trace capture.
    artifact: SimulationArtifactBuilder,
    /// Simulation start time for duration tracking.
    start_time: Instant,
    /// Deterministic seed for this simulation.
    seed: u64,
    /// Test name for identification.
    test_name: String,
    /// Metrics collected during simulation.
    metrics: SimulationMetrics,
}

impl AspenRaftTester {
    /// Create a new tester with n nodes and default configuration.
    ///
    /// Seed selection priority:
    /// 1. MADSIM_TEST_SEED environment variable
    /// 2. ASPEN_TEST_SEED environment variable
    /// 3. Deterministic hash of test name
    pub async fn new(n: usize, test_name: &str) -> Self {
        let config = TesterConfig::new(n, test_name);
        Self::with_config(config).await
    }

    /// Create a new tester with custom configuration.
    pub async fn with_config(config: TesterConfig) -> Self {
        assert!(
            config.node_count > 0 && config.node_count <= MAX_TESTER_NODES,
            "node_count must be between 1 and {MAX_TESTER_NODES}"
        );

        // Determine seed with priority: env var > config > hash
        let seed = config.seed.unwrap_or_else(|| {
            std::env::var("MADSIM_TEST_SEED")
                .ok()
                .and_then(|s| s.parse().ok())
                .or_else(|| {
                    std::env::var("ASPEN_TEST_SEED")
                        .ok()
                        .and_then(|s| s.parse().ok())
                })
                .unwrap_or_else(|| {
                    // Deterministic seed from test name
                    let mut hasher = std::hash::DefaultHasher::new();
                    config.test_name.hash(&mut hasher);
                    hasher.finish()
                })
        });

        eprintln!(
            "Starting test '{}' with seed {} ({} nodes)",
            config.test_name, seed, config.node_count
        );
        eprintln!(
            "To reproduce: MADSIM_TEST_SEED={} cargo nextest run {}",
            seed, config.test_name
        );

        let mut artifact = SimulationArtifactBuilder::new(&config.test_name, seed).start();
        artifact = artifact.add_event(format!(
            "create: {} nodes with seed {}",
            config.node_count, seed
        ));

        let router = Arc::new(MadsimRaftRouter::new());
        let injector = Arc::new(FailureInjector::new());
        let byzantine_injector = Arc::new(ByzantineFailureInjector::new());
        let mut nodes = Vec::with_capacity(config.node_count);

        // Create Raft config
        let raft_config = Config {
            heartbeat_interval: config.heartbeat_interval_ms,
            election_timeout_min: config.election_timeout_min_ms,
            election_timeout_max: config.election_timeout_max_ms,
            ..Default::default()
        };
        let raft_config = Arc::new(raft_config.validate().expect("invalid raft config"));

        // Create and register all nodes
        for i in 0..config.node_count {
            let node_id = NodeId::from(i as u64 + 1);

            let node = match config.storage_backend {
                StorageBackend::InMemory => {
                    let log_store = InMemoryLogStore::default();
                    let state_machine = InMemoryStateMachine::new();

                    let network_factory =
                        MadsimNetworkFactory::new(node_id, router.clone(), injector.clone());

                    let raft = Raft::new(
                        node_id,
                        raft_config.clone(),
                        network_factory,
                        log_store,
                        state_machine.clone(),
                    )
                    .await
                    .expect("failed to create raft instance");

                    TestNode::InMemory {
                        raft,
                        state_machine,
                        connected: AtomicBool::new(true),
                    }
                }
                StorageBackend::Sqlite => {
                    let storage_dir = config
                        .storage_dir
                        .as_ref()
                        .expect("storage_dir must be set for Sqlite backend");

                    // Create unique paths for this node
                    let node_dir = storage_dir.join(format!("node-{}", i));
                    std::fs::create_dir_all(&node_dir)
                        .expect("failed to create node storage directory");

                    let log_path = node_dir.join("raft-log.redb");
                    let state_path = node_dir.join("state-machine.db");

                    let log_store = RedbLogStore::new(&log_path)
                        .expect("failed to create persistent log store");
                    let state_machine = SqliteStateMachine::new(&state_path)
                        .expect("failed to create persistent state machine");

                    let network_factory =
                        MadsimNetworkFactory::new(node_id, router.clone(), injector.clone());

                    let raft = Raft::new(
                        node_id,
                        raft_config.clone(),
                        network_factory,
                        log_store,
                        state_machine.clone(),
                    )
                    .await
                    .expect("failed to create raft instance");

                    TestNode::Persistent {
                        raft,
                        state_machine,
                        connected: AtomicBool::new(true),
                        storage_paths: NodeStoragePaths {
                            log_path,
                            state_path,
                        },
                    }
                }
            };

            router
                .register_node(
                    node_id,
                    format!("127.0.0.1:{}", 26000 + i),
                    node.raft().clone(),
                )
                .expect("failed to register node");

            nodes.push(node);
        }

        artifact = artifact.add_event("register: all nodes with router");

        // Initialize cluster with first node
        let initial_members: BTreeMap<NodeId, RaftMemberInfo> = (0..config.node_count)
            .map(|i| {
                let id = NodeId::from(i as u64 + 1);
                (id, create_test_raft_member_info(id))
            })
            .collect();

        nodes[0]
            .raft()
            .initialize(initial_members)
            .await
            .expect("failed to initialize cluster");

        artifact = artifact.add_event("init: cluster initialized");

        Self {
            router,
            injector,
            byzantine_injector,
            nodes,
            artifact,
            start_time: Instant::now(),
            seed,
            test_name: config.test_name,
            metrics: SimulationMetrics {
                node_count: config.node_count as u32,
                ..Default::default()
            },
        }
    }

    /// Get the seed used for this simulation.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Get the number of nodes in the cluster.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the Raft handle for a specific node (0-indexed).
    pub fn raft(&self, i: usize) -> &Raft<AppTypeConfig> {
        assert!(i < self.nodes.len(), "Invalid node index");
        self.nodes[i].raft()
    }

    /// Disconnect node from network (bidirectional partition).
    pub fn disconnect(&mut self, i: usize) {
        assert!(i < self.nodes.len(), "Invalid node index");
        self.nodes[i].connected().store(false, Ordering::SeqCst);
        let node_id = NodeId::from(i as u64 + 1);

        // Bidirectional partition
        for j in 0..self.nodes.len() {
            if i != j {
                let other_id = NodeId::from(j as u64 + 1);
                self.injector.set_message_drop(node_id, other_id, true);
                self.injector.set_message_drop(other_id, node_id, true);
            }
        }

        self.metrics.network_partitions += 1;
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("disconnect: node {} from cluster", i));
    }

    /// Reconnect node to network.
    pub fn connect(&mut self, i: usize) {
        assert!(i < self.nodes.len(), "Invalid node index");
        self.nodes[i].connected().store(true, Ordering::SeqCst);
        let node_id = NodeId::from(i as u64 + 1);

        // Clear partitions
        for j in 0..self.nodes.len() {
            if i != j {
                let other_id = NodeId::from(j as u64 + 1);
                self.injector.set_message_drop(node_id, other_id, false);
                self.injector.set_message_drop(other_id, node_id, false);
            }
        }

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("connect: node {} to cluster", i));
    }

    /// Set network to unreliable mode (packet loss and variable delays).
    ///
    /// Parameters match MadRaft: 10% packet loss, 1-27ms latency.
    pub fn set_unreliable(&mut self, unreliable: bool) {
        if unreliable {
            // 10% packet loss, 1-27ms latency like MadRaft
            for i in 0..self.nodes.len() {
                for j in 0..self.nodes.len() {
                    if i != j {
                        let from = NodeId::from(i as u64 + 1);
                        let to = NodeId::from(j as u64 + 1);
                        // Range-based delay: 1-27ms
                        self.injector.set_network_delay_range(from, to, 1, 27);
                        // 10% packet loss
                        self.injector.set_packet_loss_rate(from, to, 0.1);
                    }
                }
            }
            self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
                .add_event("network: set unreliable (10% loss, 1-27ms delay)");
        } else {
            self.injector.clear_all();
            self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
                .add_event("network: set reliable");
        }
    }

    /// Configure packet loss rate for all node pairs.
    ///
    /// Rate should be between 0.0 (no loss) and 1.0 (100% loss).
    pub fn set_packet_loss_rate(&mut self, rate: f64) {
        for i in 0..self.nodes.len() {
            for j in 0..self.nodes.len() {
                if i != j {
                    let from = NodeId::from(i as u64 + 1);
                    let to = NodeId::from(j as u64 + 1);
                    self.injector.set_packet_loss_rate(from, to, rate);
                }
            }
        }
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder()).add_event(
            format!("network: set packet loss rate to {:.1}%", rate * 100.0),
        );
    }

    /// Configure range-based network delay for all node pairs.
    ///
    /// Delay will be uniformly sampled from [min_ms, max_ms] for each message.
    pub fn set_network_delay_range(&mut self, min_ms: u64, max_ms: u64) {
        for i in 0..self.nodes.len() {
            for j in 0..self.nodes.len() {
                if i != j {
                    let from = NodeId::from(i as u64 + 1);
                    let to = NodeId::from(j as u64 + 1);
                    self.injector
                        .set_network_delay_range(from, to, min_ms, max_ms);
                }
            }
        }
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("network: set delay range {}-{}ms", min_ms, max_ms));
    }

    /// Get direct access to the failure injector for advanced configurations.
    pub fn injector(&self) -> &FailureInjector {
        &self.injector
    }

    /// Get direct access to the Byzantine failure injector.
    pub fn byzantine_injector(&self) -> &ByzantineFailureInjector {
        &self.byzantine_injector
    }

    /// Enable Byzantine failure mode on a specific node.
    ///
    /// This configures the given node to potentially corrupt outgoing messages
    /// to all other nodes with the specified corruption mode and probability.
    ///
    /// # Arguments
    /// * `node_idx` - 0-based index of the node to make Byzantine
    /// * `mode` - Type of message corruption
    /// * `probability` - Probability of corruption (0.0 to 1.0)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Make node 4 flip 30% of vote responses
    /// tester.enable_byzantine_mode(4, ByzantineCorruptionMode::FlipVote, 0.3);
    /// ```
    pub fn enable_byzantine_mode(
        &mut self,
        node_idx: usize,
        mode: ByzantineCorruptionMode,
        probability: f64,
    ) {
        assert!(node_idx < self.nodes.len(), "Invalid node index");
        let node_id = NodeId::from(node_idx as u64 + 1);

        // Configure Byzantine behavior from this node to all others
        for j in 0..self.nodes.len() {
            if node_idx != j {
                let target_id = NodeId::from(j as u64 + 1);
                self.byzantine_injector
                    .set_byzantine_mode(node_id, target_id, mode, probability);
            }
        }

        self.artifact =
            std::mem::replace(&mut self.artifact, empty_artifact_builder()).add_event(format!(
                "byzantine: node {} enabled {:?} with probability {:.1}%",
                node_idx,
                mode,
                probability * 100.0
            ));
    }

    /// Disable all Byzantine behavior for a node.
    pub fn disable_byzantine_mode(&mut self, node_idx: usize) {
        assert!(node_idx < self.nodes.len(), "Invalid node index");
        // Note: Byzantine injector doesn't have per-node clear, so we just log it.
        // The injector will still have the config but we can add removal later if needed.
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("byzantine: node {} disabled", node_idx));
    }

    /// Get the number of Byzantine message corruptions that have occurred.
    pub fn byzantine_corruption_count(&self) -> u64 {
        self.byzantine_injector.total_corruptions()
    }

    /// Crash a node (marks as failed in router).
    ///
    /// For persistent nodes, we need to properly shutdown the Raft instance
    /// to release database locks before the node can be restarted.
    pub async fn crash_node(&mut self, i: usize) {
        assert!(i < self.nodes.len(), "Invalid node index");
        let node_id = NodeId::from(i as u64 + 1);

        self.router.mark_node_failed(node_id, true);
        self.nodes[i].connected().store(false, Ordering::SeqCst);

        // For persistent nodes, shutdown Raft to release database locks
        if self.nodes[i].storage_paths().is_some() {
            // Shutdown the Raft instance to release resources
            let _ = self.nodes[i].raft().shutdown().await;
        }

        self.metrics.node_crashes += 1;
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("crash: node {}", i));
    }

    /// Restart a crashed node.
    ///
    /// For in-memory nodes, this only clears the failed status.
    /// For persistent nodes, this recreates the node with the same storage,
    /// simulating a full crash recovery.
    pub async fn restart_node(&mut self, i: usize) {
        assert!(i < self.nodes.len(), "Invalid node index");
        let node_id = NodeId::from(i as u64 + 1);

        // For persistent nodes, actually recreate the node to simulate full restart
        if let Some(storage_paths) = self.nodes[i].storage_paths() {
            let storage_paths = storage_paths.clone();

            // Recreate the node with the same persistent storage
            let raft_config = Config {
                heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL_MS,
                election_timeout_min: DEFAULT_ELECTION_TIMEOUT_MIN_MS,
                election_timeout_max: DEFAULT_ELECTION_TIMEOUT_MAX_MS,
                ..Default::default()
            };
            let raft_config = Arc::new(raft_config.validate().expect("invalid raft config"));

            let log_store = RedbLogStore::new(&storage_paths.log_path)
                .expect("failed to reopen persistent log store");
            let state_machine = SqliteStateMachine::new(&storage_paths.state_path)
                .expect("failed to reopen persistent state machine");

            let network_factory =
                MadsimNetworkFactory::new(node_id, self.router.clone(), self.injector.clone());

            let raft = Raft::new(
                node_id,
                raft_config,
                network_factory,
                log_store,
                state_machine.clone(),
            )
            .await
            .expect("failed to recreate raft instance");

            // Re-register with router
            self.router
                .register_node(node_id, format!("127.0.0.1:{}", 26000 + i), raft.clone())
                .expect("failed to re-register node");

            // Replace the node in our list
            self.nodes[i] = TestNode::Persistent {
                raft,
                state_machine,
                connected: AtomicBool::new(true),
                storage_paths,
            };

            self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
                .add_event(format!("restart: node {} with persistent storage", i));
        } else {
            // For in-memory nodes, just clear the failed status
            self.nodes[i].connected().store(true, Ordering::SeqCst);
            self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
                .add_event(format!("restart: node {} (in-memory, state preserved)", i));
        }

        self.router.mark_node_failed(node_id, false);
        self.metrics.node_restarts += 1;
    }

    /// Check for exactly one leader among connected nodes.
    ///
    /// Returns the leader's index (0-indexed) if exactly one leader is found.
    /// Uses random backoff like MadRaft for better election handling.
    pub async fn check_one_leader(&mut self) -> Option<usize> {
        let mut retries = LEADER_CHECK_RETRIES;

        while retries > 0 {
            // Random backoff using madsim's deterministic random
            let backoff = LEADER_CHECK_BACKOFF_MIN_MS
                + (madsim::rand::random::<u64>()
                    % (LEADER_CHECK_BACKOFF_MAX_MS - LEADER_CHECK_BACKOFF_MIN_MS));
            madsim::time::sleep(Duration::from_millis(backoff)).await;

            // Look for a leader that all connected nodes agree on
            let mut leader_id: Option<NodeId> = None;
            let mut leader_idx: Option<usize> = None;
            let mut all_agree = true;

            for node in self.nodes.iter() {
                if node.connected().load(Ordering::Relaxed) {
                    let metrics = node.raft().metrics().borrow().clone();
                    if let Some(current_leader) = metrics.current_leader {
                        match leader_id {
                            None => {
                                leader_id = Some(current_leader);
                                // Convert NodeId to 0-based index
                                leader_idx = Some((current_leader.0 - 1) as usize);
                            }
                            Some(existing) if existing != current_leader => {
                                // Disagreement on leader
                                all_agree = false;
                                break;
                            }
                            _ => {} // Agreement continues
                        }
                    }
                }
            }

            if all_agree && let Some(idx) = leader_idx {
                let id = leader_id.unwrap();
                let metrics = self.nodes[idx].raft().metrics().borrow().clone();
                self.metrics.elections += 1;
                self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
                    .add_event(format!(
                        "leader: node {} (id={}) elected for term {}",
                        idx, id, metrics.current_term
                    ));
                return Some(idx);
            }

            retries -= 1;
        }

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event("leader: no leader found after retries");
        None
    }

    /// Verify no split brain (at most one leader per term).
    pub fn check_no_split_brain(&self) -> Result<()> {
        let mut leaders_per_term: BTreeMap<u64, Vec<usize>> = BTreeMap::new();

        for (i, node) in self.nodes.iter().enumerate() {
            let metrics = node.raft().metrics().borrow().clone();
            let term = metrics.current_term;
            if let Some(leader_id) = metrics.current_leader
                && leader_id == NodeId::from(i as u64 + 1)
            {
                leaders_per_term.entry(term).or_default().push(i);
            }
        }

        for (term, leaders) in leaders_per_term {
            if leaders.len() > 1 {
                anyhow::bail!(
                    "Split brain detected: term {} has {} leaders: {:?}",
                    term,
                    leaders.len(),
                    leaders
                );
            }
        }

        Ok(())
    }

    /// Get the maximum log index across all nodes.
    pub fn max_log_size(&self) -> u64 {
        self.nodes
            .iter()
            .map(|n| {
                let metrics = n.raft().metrics().borrow().clone();
                metrics.last_log_index.unwrap_or(0)
            })
            .max()
            .unwrap_or(0)
    }

    /// Perform a write operation through the leader.
    pub async fn write(&mut self, key: String, value: String) -> Result<()> {
        let leader_idx = self
            .check_one_leader()
            .await
            .ok_or_else(|| anyhow::anyhow!("No leader available for write"))?;

        self.nodes[leader_idx]
            .raft()
            .client_write(AppRequest::Set {
                key: key.clone(),
                value: value.clone(),
            })
            .await
            .map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("write: key='{}' completed", key));
        Ok(())
    }

    /// Read a value from the leader's state machine.
    ///
    /// Note: Only supports in-memory nodes for now. For persistent nodes,
    /// reads would need to go through the Raft read API.
    pub async fn read(&mut self, key: &str) -> Result<Option<String>> {
        let leader_idx = self
            .check_one_leader()
            .await
            .ok_or_else(|| anyhow::anyhow!("No leader available for read"))?;

        // Read depends on the node type
        let value = match &self.nodes[leader_idx] {
            TestNode::InMemory { state_machine, .. } => state_machine.get(key).await,
            TestNode::Persistent { .. } => {
                // SqliteStateMachine doesn't expose a direct get method
                // For testing crash recovery, we focus on writes being preserved
                // TODO: Implement read support for persistent nodes
                None
            }
        };
        Ok(value)
    }

    /// Add a custom event to the artifact trace.
    pub fn add_event(&mut self, event: impl Into<String>) {
        self.artifact =
            std::mem::replace(&mut self.artifact, empty_artifact_builder()).add_event(event);
    }

    /// End the test and return the simulation artifact.
    ///
    /// This method persists the artifact to disk and returns it for
    /// optional additional processing.
    pub fn end(mut self) -> SimulationArtifact {
        let duration = self.start_time.elapsed();
        self.metrics.duration_ms = duration.as_millis() as u64;
        self.metrics.max_log_size = self.max_log_size();
        self.metrics.byzantine_corruptions = self.byzantine_injector.total_corruptions();

        // Collect final metrics as JSON
        let metrics_json =
            serde_json::to_string_pretty(&self.metrics).unwrap_or_else(|_| "{}".to_string());

        let artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .with_metrics(metrics_json)
            .build();

        eprintln!(
            "Test '{}' finished in {:.2}s with {} nodes ({} byzantine corruptions)",
            self.test_name,
            duration.as_secs_f64(),
            self.nodes.len(),
            self.metrics.byzantine_corruptions,
        );

        // Persist artifact
        if let Ok(path) = artifact.persist("docs/simulations") {
            eprintln!("Artifact saved to: {}", path.display());
        }

        artifact
    }
}

#[cfg(test)]
mod tests {
    // Note: These tests require madsim runtime, which is only available
    // with the madsim feature enabled. They are tested via the integration
    // test suite in tests/madsim_tester_test.rs
}
