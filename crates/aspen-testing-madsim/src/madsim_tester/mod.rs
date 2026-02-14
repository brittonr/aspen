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
//! - [FoundationDB Testing](https://apple.github.io/foundationdb/testing.html) - BUGGIFY
//!   inspiration
//! - [RisingWave DST](https://www.risingwave.com/blog/deterministic-simulation-a-new-era-of-distributed-system-testing/)

mod buggify;
mod cluster_ops;
mod config;
mod fault_injection;
mod liveness;
mod liveness_ops;
mod metrics;
mod network;
mod node;

// Re-export all public types
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use aspen_core::simulation::SimulationArtifact;
use aspen_core::simulation::SimulationArtifactBuilder;
#[cfg(feature = "sql")]
use aspen_raft::StateMachineVariant;
use aspen_raft::madsim_network::ByzantineFailureInjector;
use aspen_raft::madsim_network::FailureInjector;
use aspen_raft::madsim_network::MadsimNetworkFactory;
use aspen_raft::madsim_network::MadsimRaftRouter;
#[cfg(feature = "sql")]
use aspen_raft::node::RaftNode;
use aspen_raft::storage::InMemoryLogStore;
use aspen_raft::storage::InMemoryStateMachine;
use aspen_raft::storage::StorageBackend;
use aspen_raft::storage_shared::SharedRedbStorage;
use aspen_raft::types::AppTypeConfig;
use aspen_raft::types::NodeId;
use aspen_raft::types::RaftMemberInfo;
pub use buggify::BuggifyConfig;
pub use buggify::BuggifyFault;
pub use config::TesterConfig;
pub use liveness::LivenessConfig;
pub use liveness::LivenessMetrics;
pub use liveness::LivenessMode;
pub use liveness::LivenessReport;
pub use liveness::LivenessViolation;
pub use liveness::ViolationType;
pub use metrics::SimulationMetrics;
pub use node::create_test_raft_member_info;
use openraft::Config;
use openraft::Raft;

use self::buggify::BuggifyConfig as BuggifyConfigInner;
use self::config::MAX_TESTER_NODES;
use self::liveness::LivenessMode as LivenessModeInner;
use self::liveness::LivenessState;
use self::metrics::SimulationMetrics as SimulationMetricsInner;
use self::node::RedbStoragePath;
use self::node::TestNode;
use self::node::create_test_raft_member_info as create_member_info;
use self::node::empty_artifact_builder;

/// High-level tester abstraction for madsim Raft simulations.
///
/// Reduces test boilerplate from 40+ lines to <10 lines by encapsulating:
/// - Router and failure injector creation
/// - Node creation and registration
/// - Cluster initialization
/// - Artifact capture
/// - Metrics collection
/// - Byzantine failure injection
/// - Liveness mode testing (TigerBeetle-style two-phase testing)
pub struct AspenRaftTester {
    /// Router managing all nodes in the simulation.
    pub(crate) router: Arc<MadsimRaftRouter>,
    /// Failure injector for chaos testing.
    pub(crate) injector: Arc<FailureInjector>,
    /// Byzantine failure injector for message corruption testing.
    pub(crate) byzantine_injector: Arc<ByzantineFailureInjector>,
    /// BUGGIFY configuration for systematic fault injection.
    pub(crate) buggify: Arc<BuggifyConfigInner>,
    /// All nodes in the cluster.
    pub(crate) nodes: Vec<TestNode>,
    /// Artifact builder for event trace capture.
    pub(crate) artifact: SimulationArtifactBuilder,
    /// Simulation start time for duration tracking.
    pub(crate) start_time: Instant,
    /// Deterministic seed for this simulation.
    pub(crate) seed: u64,
    /// Test name for identification.
    pub(crate) test_name: String,
    /// Metrics collected during simulation.
    pub(crate) metrics: SimulationMetricsInner,
    /// Liveness testing configuration.
    pub(crate) liveness_config: liveness::LivenessConfig,
    /// Liveness tracking state.
    pub(crate) liveness_state: LivenessState,
    /// Track restart counts per node for unique storage paths.
    pub(crate) restart_counts: HashMap<usize, usize>,
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

    /// Create a new tester with an explicit seed for deterministic testing.
    pub async fn new_with_seed(n: usize, test_name: &str, seed: u64) -> Self {
        let mut config = TesterConfig::new(n, test_name);
        config.seed = Some(seed);
        Self::with_config(config).await
    }

    /// Create a new tester with custom configuration.
    pub async fn with_config(config: TesterConfig) -> Self {
        assert!(
            config.node_count > 0 && config.node_count <= MAX_TESTER_NODES,
            "node_count must be between 1 and {MAX_TESTER_NODES}"
        );

        let seed = resolve_seed(&config);

        eprintln!("Starting test '{}' with seed {} ({} nodes)", config.test_name, seed, config.node_count);
        eprintln!("To reproduce: MADSIM_TEST_SEED={} cargo nextest run {}", seed, config.test_name);

        let mut artifact = SimulationArtifactBuilder::new(&config.test_name, seed).start();
        artifact = artifact.add_event(format!("create: {} nodes with seed {}", config.node_count, seed));

        let router = Arc::new(MadsimRaftRouter::new());
        let injector = Arc::new(FailureInjector::new());
        let byzantine_injector = Arc::new(ByzantineFailureInjector::new());

        let raft_config = create_raft_config(&config);
        let nodes = create_all_nodes(&config, &router, &injector, &raft_config).await;

        artifact = artifact.add_event("register: all nodes with router");

        // Initialize cluster with first node
        let initial_members: BTreeMap<NodeId, RaftMemberInfo> = (0..config.node_count)
            .map(|i| {
                let id = NodeId::from(i as u64 + 1);
                (id, create_member_info(id))
            })
            .collect();

        nodes[0].raft().initialize(initial_members).await.expect("failed to initialize cluster");

        artifact = artifact.add_event("init: cluster initialized");

        // Initialize BUGGIFY with the test seed
        let buggify = Arc::new(BuggifyConfigInner::new(seed));

        // Determine if liveness tracking is enabled
        let liveness_active = config.liveness.mode != LivenessModeInner::Disabled;

        Self {
            router,
            injector,
            byzantine_injector,
            buggify,
            nodes,
            artifact,
            start_time: Instant::now(),
            seed,
            test_name: config.test_name,
            metrics: SimulationMetricsInner {
                node_count: config.node_count as u32,
                ..Default::default()
            },
            liveness_config: config.liveness,
            liveness_state: LivenessState {
                active: liveness_active,
                ..Default::default()
            },
            restart_counts: HashMap::new(),
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

    /// Finalize the simulation and return the collected artifact.
    pub fn end(mut self) -> SimulationArtifact {
        let duration = self.start_time.elapsed();
        self.metrics.duration_ms = duration.as_millis() as u64;
        self.metrics.max_log_size = self.max_log_size();
        self.metrics.byzantine_corruptions = self.byzantine_injector.total_corruptions();

        // Add BUGGIFY metrics if it was used
        if self.buggify.total_triggers() > 0 {
            self.metrics.buggify_triggers = self.buggify.total_triggers();
        }

        // Collect final metrics as JSON
        let metrics_json = serde_json::to_string_pretty(&self.metrics).unwrap_or_else(|_| "{}".to_string());

        let artifact =
            std::mem::replace(&mut self.artifact, empty_artifact_builder()).with_metrics(metrics_json).build();

        eprintln!(
            "Test '{}' finished in {:.2}s with {} nodes ({} byzantine corruptions, {} BUGGIFY triggers)",
            self.test_name,
            duration.as_secs_f64(),
            self.nodes.len(),
            self.metrics.byzantine_corruptions,
            self.metrics.buggify_triggers,
        );

        // Persist artifact
        if let Ok(path) = artifact.persist("docs/simulations") {
            eprintln!("Artifact saved to: {}", path.display());
        }

        artifact
    }
}

/// Determine the seed for a test with priority: env var > config > hash.
fn resolve_seed(config: &TesterConfig) -> u64 {
    config.seed.unwrap_or_else(|| {
        std::env::var("MADSIM_TEST_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .or_else(|| std::env::var("ASPEN_TEST_SEED").ok().and_then(|s| s.parse().ok()))
            .unwrap_or_else(|| {
                // Deterministic seed from test name
                let mut hasher = std::hash::DefaultHasher::new();
                config.test_name.hash(&mut hasher);
                hasher.finish()
            })
    })
}

/// Create a validated Raft config from tester config.
fn create_raft_config(config: &TesterConfig) -> Arc<openraft::Config> {
    let raft_config = Config {
        heartbeat_interval: config.heartbeat_interval_ms,
        election_timeout_min: config.election_timeout_min_ms,
        election_timeout_max: config.election_timeout_max_ms,
        ..Default::default()
    };
    Arc::new(raft_config.validate().expect("invalid raft config"))
}

/// Create and register all nodes for the cluster.
async fn create_all_nodes(
    config: &TesterConfig,
    router: &Arc<MadsimRaftRouter>,
    injector: &Arc<FailureInjector>,
    raft_config: &Arc<openraft::Config>,
) -> Vec<TestNode> {
    let mut nodes = Vec::with_capacity(config.node_count);

    for i in 0..config.node_count {
        let node_id = NodeId::from(i as u64 + 1);

        let node = match config.storage_backend {
            StorageBackend::InMemory => create_in_memory_node(node_id, router, injector, raft_config).await,
            StorageBackend::Redb => create_redb_node(node_id, i, config, router, injector, raft_config).await,
        };

        router
            .register_node(node_id, format!("127.0.0.1:{}", 26000 + i), node.raft().clone())
            .expect("failed to register node");

        nodes.push(node);
    }

    nodes
}

/// Create an in-memory test node.
async fn create_in_memory_node(
    node_id: NodeId,
    router: &Arc<MadsimRaftRouter>,
    injector: &Arc<FailureInjector>,
    raft_config: &Arc<openraft::Config>,
) -> TestNode {
    let log_store = InMemoryLogStore::default();
    let state_machine = InMemoryStateMachine::new();

    let network_factory = MadsimNetworkFactory::new(node_id, router.clone(), injector.clone());

    let raft = Raft::new(node_id, raft_config.clone(), network_factory, log_store, state_machine.clone())
        .await
        .expect("failed to create raft instance");

    TestNode::InMemory {
        raft,
        state_machine,
        connected: AtomicBool::new(true),
    }
}

/// Create a Redb-backed test node.
async fn create_redb_node(
    node_id: NodeId,
    i: usize,
    config: &TesterConfig,
    router: &Arc<MadsimRaftRouter>,
    injector: &Arc<FailureInjector>,
    raft_config: &Arc<openraft::Config>,
) -> TestNode {
    // Use real Redb storage for SQL testing with single-fsync architecture.
    // SharedRedbStorage implements both RaftLogStorage AND RaftStateMachine.
    let storage_dir = config.storage_dir.as_ref().expect("storage_dir must be set for Redb backend");

    // Create unique paths for this node
    let node_dir = storage_dir.join(format!("node-{}", i));
    std::fs::create_dir_all(&node_dir).expect("failed to create node storage directory");

    let db_path = node_dir.join("shared.redb");

    // SharedRedbStorage is the single-fsync backend - it implements
    // BOTH log storage AND state machine in a single struct.
    // SharedRedbStorage derives Clone (uses Arc internally for database).
    let storage = SharedRedbStorage::new(&db_path, &node_id.to_string()).expect("failed to create Redb storage");

    let network_factory = MadsimNetworkFactory::new(node_id, router.clone(), injector.clone());

    // Pass clones of SharedRedbStorage for BOTH log_store and state_machine.
    // SharedRedbStorage is Clone (internally Arc-wrapped), so this is cheap.
    // This is what enables single-fsync writes.
    let raft = Raft::new(node_id, raft_config.clone(), network_factory, storage.clone(), storage.clone())
        .await
        .expect("failed to create raft instance");

    // Wrap storage in Arc for StateMachineVariant and storage in TestNode
    let storage = Arc::new(storage);

    // Create RaftNode wrapper to expose SqlQueryExecutor trait (only when sql feature is enabled)
    #[cfg(feature = "sql")]
    let raft_node =
        Arc::new(RaftNode::new(node_id, Arc::new(raft.clone()), StateMachineVariant::Redb(storage.clone())));

    TestNode::Redb {
        raft,
        #[cfg(feature = "sql")]
        raft_node,
        storage,
        connected: AtomicBool::new(true),
        storage_path: RedbStoragePath { db_path },
    }
}

#[cfg(test)]
mod tests {
    // Note: These tests require madsim runtime, which is only available
    // with the madsim feature enabled. They are tested via the integration
    // test suite in tests/madsim_tester_test.rs
}
