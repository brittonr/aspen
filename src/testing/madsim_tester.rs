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

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use openraft::{Config, Raft};

use crate::api::{SqlConsistency, SqlQueryError, SqlQueryRequest, SqlQueryResult};
use crate::raft::StateMachineVariant;
use crate::raft::madsim_network::{
    ByzantineCorruptionMode, ByzantineFailureInjector, FailureInjector, MadsimNetworkFactory,
    MadsimRaftRouter,
};
use crate::raft::node::RaftNode;
use crate::raft::storage::{InMemoryLogStore, InMemoryStateMachine, RedbLogStore, StorageBackend};
use crate::raft::storage_shared::SharedRedbStorage;
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
// Leader check timing - reduced from 10 retries × 450-550ms (5.5s worst case)
// to 5 retries × 200-300ms (1.5s worst case) to avoid test timeouts
const LEADER_CHECK_RETRIES: u32 = 5;
const LEADER_CHECK_BACKOFF_MIN_MS: u64 = 200;
const LEADER_CHECK_BACKOFF_MAX_MS: u64 = 300;

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
    /// Liveness testing configuration (TigerBeetle-style two-phase testing).
    pub liveness: LivenessConfig,
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
            liveness: LivenessConfig::default(),
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

    /// Use Redb storage for testing SQL queries with single-fsync architecture.
    ///
    /// SharedRedbStorage implements both log and state machine in a single struct,
    /// enabling single-fsync writes (~2-3ms vs ~9ms with SQLite).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = TesterConfig::new(3, "sql_test")
    ///     .with_redb_storage("/tmp/aspen-sql-test");
    /// ```
    pub fn with_redb_storage(mut self, storage_dir: impl Into<std::path::PathBuf>) -> Self {
        self.storage_backend = StorageBackend::Redb;
        self.storage_dir = Some(storage_dir.into());
        self
    }

    /// Enable liveness testing (TigerBeetle-style two-phase testing).
    ///
    /// Liveness testing verifies that the cluster makes progress under
    /// various failure conditions. This is separate from safety testing
    /// which only verifies correctness invariants.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Strict liveness: cluster must always have a leader
    /// let config = TesterConfig::new(3, "liveness_test")
    ///     .with_liveness(LivenessConfig::strict());
    ///
    /// // Eventual liveness: cluster may temporarily lack leader
    /// let config = TesterConfig::new(3, "liveness_test")
    ///     .with_liveness(LivenessConfig::eventual());
    ///
    /// // Custom timeout: max 5 seconds without leader
    /// let config = TesterConfig::new(3, "liveness_test")
    ///     .with_liveness(LivenessConfig::with_timeout(5000));
    /// ```
    pub fn with_liveness(mut self, config: LivenessConfig) -> Self {
        self.liveness = config;
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
    /// Number of membership changes (add learner, change membership).
    pub membership_changes: u32,
    /// Number of BUGGIFY faults triggered.
    pub buggify_triggers: u64,
    /// Liveness metrics (populated when liveness mode is used).
    #[serde(default)]
    pub liveness: LivenessMetrics,
}

// =========================================================================
// Liveness Mode Testing (TigerBeetle-Style Two-Phase Testing)
// =========================================================================

/// Metrics for liveness testing.
///
/// These metrics track the system's ability to make progress under
/// various failure conditions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct LivenessMetrics {
    /// Total time (ms) the cluster had no leader.
    pub leaderless_duration_ms: u64,
    /// Number of times the cluster became leaderless.
    pub leaderless_periods: u32,
    /// Time (ms) for first leader election after cluster start.
    pub first_election_ms: u64,
    /// Maximum time (ms) to recover a leader after crash/partition.
    pub max_leader_recovery_ms: u64,
    /// Number of successful writes during liveness testing.
    pub writes_completed: u64,
    /// Number of failed writes due to no leader.
    pub writes_blocked: u64,
    /// Total time (ms) spent with operations blocked.
    pub blocked_duration_ms: u64,
    /// Number of liveness checks performed.
    pub liveness_checks: u64,
    /// Number of liveness checks that passed.
    pub liveness_checks_passed: u64,
}

/// Liveness testing mode.
///
/// TigerBeetle uses a two-phase testing approach:
/// 1. **Safety mode**: Run tests with strict invariant checking (no progress required)
/// 2. **Liveness mode**: Re-run the same tests, requiring progress to be made
///
/// This allows testing both correctness (safety) and progress (liveness) guarantees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, Default)]
pub enum LivenessMode {
    /// Disabled (default): Only check safety invariants, don't require progress.
    #[default]
    Disabled,
    /// Strict: Cluster must always have a leader within timeout.
    /// This is the strongest liveness guarantee.
    Strict,
    /// Eventual: Cluster may temporarily lack a leader, but must recover.
    /// Allows transient unavailability during failures.
    Eventual,
    /// Custom timeout: Like Eventual, but with a custom recovery timeout.
    /// The value is the maximum allowed leaderless duration in milliseconds.
    CustomTimeout(u64),
}

/// Configuration for liveness testing.
#[derive(Debug, Clone)]
pub struct LivenessConfig {
    /// Liveness mode to use.
    pub mode: LivenessMode,
    /// How often to check liveness (ms).
    pub check_interval_ms: u64,
    /// Maximum time to wait for leader recovery (ms).
    /// Used when mode is Eventual.
    pub recovery_timeout_ms: u64,
    /// Whether to track detailed timing metrics.
    pub track_timing: bool,
}

impl Default for LivenessConfig {
    fn default() -> Self {
        Self {
            mode: LivenessMode::Disabled,
            check_interval_ms: 100,      // Check every 100ms
            recovery_timeout_ms: 30_000, // 30 second recovery timeout
            track_timing: true,
        }
    }
}

impl LivenessConfig {
    /// Create a strict liveness configuration.
    pub fn strict() -> Self {
        Self {
            mode: LivenessMode::Strict,
            check_interval_ms: 100,
            recovery_timeout_ms: 10_000, // Strict = shorter timeout
            track_timing: true,
        }
    }

    /// Create an eventual liveness configuration.
    pub fn eventual() -> Self {
        Self {
            mode: LivenessMode::Eventual,
            ..Default::default()
        }
    }

    /// Create a custom liveness configuration with specific timeout.
    pub fn with_timeout(max_leaderless_ms: u64) -> Self {
        Self {
            mode: LivenessMode::CustomTimeout(max_leaderless_ms),
            recovery_timeout_ms: max_leaderless_ms,
            ..Default::default()
        }
    }
}

/// Report from liveness testing.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LivenessReport {
    /// Overall liveness status.
    pub passed: bool,
    /// Liveness mode that was used.
    pub mode: LivenessMode,
    /// Detailed metrics from the test.
    pub metrics: LivenessMetrics,
    /// Any liveness violations detected.
    pub violations: Vec<LivenessViolation>,
    /// Summary message.
    pub summary: String,
}

/// A liveness violation event.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LivenessViolation {
    /// When the violation started (relative to test start, in ms).
    pub started_at_ms: u64,
    /// Duration of the violation (ms).
    pub duration_ms: u64,
    /// Type of violation.
    pub violation_type: ViolationType,
    /// Additional context.
    pub context: String,
}

/// Types of liveness violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ViolationType {
    /// Cluster had no leader for too long.
    LeaderlessTimeout,
    /// Operation was blocked for too long.
    OperationBlocked,
    /// Cluster failed to recover after failure injection.
    RecoveryTimeout,
}

// =========================================================================
// BUGGIFY-Style Fault Injection (Phase 2.2)
// =========================================================================

/// Type of fault that can be injected via BUGGIFY.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
pub enum BuggifyFault {
    /// Inject random network delays.
    NetworkDelay,
    /// Drop random network packets.
    NetworkDrop,
    /// Crash random nodes.
    NodeCrash,
    /// Inject slow disk operations.
    SlowDisk,
    /// Corrupt random messages.
    MessageCorruption,
    /// Force election timeouts.
    ElectionTimeout,
    /// Partition the network randomly.
    NetworkPartition,
    /// Trigger snapshot operations.
    SnapshotTrigger,
}

/// BUGGIFY configuration for systematic fault injection.
///
/// Inspired by FoundationDB's BUGGIFY macro, this provides
/// deterministic fault injection based on the test seed.
#[derive(Clone)]
pub struct BuggifyConfig {
    /// Whether BUGGIFY is enabled globally.
    enabled: Arc<AtomicBool>,
    /// Base probability (0.0 to 1.0) for each fault type.
    probabilities: Arc<Mutex<HashMap<BuggifyFault, f64>>>,
    /// Count of triggers per fault type.
    trigger_counts: Arc<Mutex<HashMap<BuggifyFault, u64>>>,
    /// Seed used to initialize this BUGGIFY instance.
    /// Note: Probability checking now uses madsim's RNG for uniform distribution,
    /// but the seed is retained for debugging and metrics purposes.
    #[allow(dead_code)]
    seed: u64,
}

impl BuggifyConfig {
    /// Create a new BUGGIFY configuration with default settings.
    pub fn new(seed: u64) -> Self {
        let mut default_probs = HashMap::new();
        // Default probabilities for each fault type
        default_probs.insert(BuggifyFault::NetworkDelay, 0.05); // 5%
        default_probs.insert(BuggifyFault::NetworkDrop, 0.02); // 2%
        default_probs.insert(BuggifyFault::NodeCrash, 0.01); // 1%
        default_probs.insert(BuggifyFault::SlowDisk, 0.05); // 5%
        default_probs.insert(BuggifyFault::MessageCorruption, 0.01); // 1%
        default_probs.insert(BuggifyFault::ElectionTimeout, 0.02); // 2%
        default_probs.insert(BuggifyFault::NetworkPartition, 0.005); // 0.5%
        default_probs.insert(BuggifyFault::SnapshotTrigger, 0.02); // 2%

        Self {
            enabled: Arc::new(AtomicBool::new(false)),
            probabilities: Arc::new(Mutex::new(default_probs)),
            trigger_counts: Arc::new(Mutex::new(HashMap::new())),
            seed,
        }
    }

    /// Enable BUGGIFY with optional custom probabilities.
    pub fn enable(&self, custom_probs: Option<HashMap<BuggifyFault, f64>>) {
        if let Some(probs) = custom_probs {
            *self.probabilities.lock().unwrap() = probs;
        }
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable BUGGIFY.
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Check if a specific fault should be triggered.
    ///
    /// Uses madsim's deterministic RNG for uniform probability distribution.
    /// The simulation framework seeds the RNG, ensuring reproducibility.
    pub fn should_trigger(&self, fault: BuggifyFault) -> bool {
        if !self.enabled.load(Ordering::Relaxed) {
            return false;
        }

        let mut counts = self.trigger_counts.lock().unwrap();
        let count = counts.entry(fault).or_insert(0);
        *count += 1;

        let probs = self.probabilities.lock().unwrap();
        let probability = probs.get(&fault).copied().unwrap_or(0.0);

        // Use madsim's deterministic RNG for uniform distribution.
        // This provides proper probability checking unlike DefaultHasher,
        // which has poor distribution for probability thresholding.
        let random_value: f64 = (madsim::rand::random::<u64>() as f64) / (u64::MAX as f64);
        random_value < probability
    }

    /// Get total trigger counts for metrics.
    pub fn total_triggers(&self) -> u64 {
        self.trigger_counts.lock().unwrap().values().sum()
    }
}

/// Storage paths for a persistent node.
#[derive(Clone, Debug)]
struct NodeStoragePaths {
    /// Path to the Redb log store file.
    log_path: std::path::PathBuf,
    /// Path to the SQLite state machine database.
    state_path: std::path::PathBuf,
}

/// Storage paths for a Redb node (for future crash recovery testing).
#[allow(dead_code)]
#[derive(Clone, Debug)]
struct RedbStoragePath {
    /// Path to the shared Redb database file.
    db_path: std::path::PathBuf,
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
        #[allow(dead_code)]
        state_machine: Arc<SqliteStateMachine>,
        connected: AtomicBool,
        storage_paths: NodeStoragePaths,
    },
    /// Redb node (for SQL testing with single-fsync storage).
    Redb {
        raft: Raft<AppTypeConfig>,
        /// RaftNode wrapper for SqlQueryExecutor access.
        raft_node: Arc<RaftNode>,
        /// Shared Redb storage (implements both log and state machine).
        storage: Arc<SharedRedbStorage>,
        connected: AtomicBool,
        /// Storage path (for future crash recovery testing).
        #[allow(dead_code)]
        storage_path: RedbStoragePath,
    },
}

impl TestNode {
    fn raft(&self) -> &Raft<AppTypeConfig> {
        match self {
            TestNode::InMemory { raft, .. } => raft,
            TestNode::Persistent { raft, .. } => raft,
            TestNode::Redb { raft, .. } => raft,
        }
    }

    fn connected(&self) -> &AtomicBool {
        match self {
            TestNode::InMemory { connected, .. } => connected,
            TestNode::Persistent { connected, .. } => connected,
            TestNode::Redb { connected, .. } => connected,
        }
    }

    fn storage_paths(&self) -> Option<&NodeStoragePaths> {
        match self {
            TestNode::InMemory { .. } => None,
            TestNode::Persistent { storage_paths, .. } => Some(storage_paths),
            TestNode::Redb { .. } => None,
        }
    }

    /// Get Redb storage path (for future crash recovery testing).
    #[allow(dead_code)]
    fn redb_storage_path(&self) -> Option<&RedbStoragePath> {
        match self {
            TestNode::InMemory { .. } => None,
            TestNode::Persistent { .. } => None,
            TestNode::Redb { storage_path, .. } => Some(storage_path),
        }
    }

    /// Get the RaftNode wrapper for SQL execution (only available for Redb nodes).
    fn raft_node(&self) -> Option<&Arc<RaftNode>> {
        match self {
            TestNode::InMemory { .. } => None,
            TestNode::Persistent { .. } => None,
            TestNode::Redb { raft_node, .. } => Some(raft_node),
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
/// - Liveness mode testing (TigerBeetle-style two-phase testing)
pub struct AspenRaftTester {
    /// Router managing all nodes in the simulation.
    router: Arc<MadsimRaftRouter>,
    /// Failure injector for chaos testing.
    injector: Arc<FailureInjector>,
    /// Byzantine failure injector for message corruption testing.
    byzantine_injector: Arc<ByzantineFailureInjector>,
    /// BUGGIFY configuration for systematic fault injection.
    buggify: Arc<BuggifyConfig>,
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
    /// Liveness testing configuration.
    liveness_config: LivenessConfig,
    /// Liveness tracking state.
    liveness_state: LivenessState,
}

/// Internal state for liveness tracking.
#[derive(Debug, Clone, Default)]
struct LivenessState {
    /// Whether liveness tracking is active.
    active: bool,
    /// When the cluster last had a leader.
    last_leader_time: Option<Instant>,
    /// When the cluster became leaderless (for tracking duration).
    leaderless_since: Option<Instant>,
    /// Accumulated violations.
    violations: Vec<LivenessViolation>,
    /// First election time (set once on first leader).
    first_election_time: Option<Instant>,
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
                StorageBackend::Redb => {
                    // Use real Redb storage for SQL testing with single-fsync architecture.
                    // SharedRedbStorage implements both RaftLogStorage AND RaftStateMachine.
                    let storage_dir = config
                        .storage_dir
                        .as_ref()
                        .expect("storage_dir must be set for Redb backend");

                    // Create unique paths for this node
                    let node_dir = storage_dir.join(format!("node-{}", i));
                    std::fs::create_dir_all(&node_dir)
                        .expect("failed to create node storage directory");

                    let db_path = node_dir.join("shared.redb");

                    // SharedRedbStorage is the single-fsync backend - it implements
                    // BOTH log storage AND state machine in a single struct.
                    // SharedRedbStorage derives Clone (uses Arc internally for database).
                    let storage =
                        SharedRedbStorage::new(&db_path).expect("failed to create Redb storage");

                    let network_factory =
                        MadsimNetworkFactory::new(node_id, router.clone(), injector.clone());

                    // Pass clones of SharedRedbStorage for BOTH log_store and state_machine.
                    // SharedRedbStorage is Clone (internally Arc-wrapped), so this is cheap.
                    // This is what enables single-fsync writes.
                    let raft = Raft::new(
                        node_id,
                        raft_config.clone(),
                        network_factory,
                        storage.clone(),
                        storage.clone(),
                    )
                    .await
                    .expect("failed to create raft instance");

                    // Wrap storage in Arc for StateMachineVariant and storage in TestNode
                    let storage = Arc::new(storage);

                    // Create RaftNode wrapper to expose SqlQueryExecutor trait
                    let raft_node = Arc::new(RaftNode::new(
                        node_id,
                        Arc::new(raft.clone()),
                        StateMachineVariant::Redb(storage.clone()),
                    ));

                    TestNode::Redb {
                        raft,
                        raft_node,
                        storage,
                        connected: AtomicBool::new(true),
                        storage_path: RedbStoragePath { db_path },
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

        // Initialize BUGGIFY with the test seed
        let buggify = Arc::new(BuggifyConfig::new(seed));

        // Determine if liveness tracking is enabled
        let liveness_active = config.liveness.mode != LivenessMode::Disabled;

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
            metrics: SimulationMetrics {
                node_count: config.node_count as u32,
                ..Default::default()
            },
            liveness_config: config.liveness,
            liveness_state: LivenessState {
                active: liveness_active,
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

    // =========================================================================
    // Clock Drift Simulation Methods
    // =========================================================================

    /// Set clock drift for a specific node.
    ///
    /// Clock drift is simulated by adding asymmetric delays to messages:
    /// - Positive drift (fast clock): Delays OUTGOING messages from this node
    /// - Negative drift (slow clock): Delays INCOMING messages to this node
    ///
    /// This effectively simulates how Raft behaves when a node's clock runs
    /// faster or slower than other nodes in the cluster.
    ///
    /// # Arguments
    /// * `node_idx` - 0-based index of the node
    /// * `drift_ms` - Signed drift in milliseconds. Positive = fast clock, negative = slow clock.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Node 1 has a clock that's 100ms "fast"
    /// // Its heartbeats will appear to arrive late from other nodes' perspective
    /// t.set_clock_drift(1, 100);
    ///
    /// // Node 2 has a clock that's 50ms "slow"
    /// // Messages to it will appear delayed
    /// t.set_clock_drift(2, -50);
    /// ```
    pub fn set_clock_drift(&mut self, node_idx: usize, drift_ms: i64) {
        assert!(node_idx < self.nodes.len(), "Invalid node index");
        let node_id = NodeId::from(node_idx as u64 + 1);

        self.injector.set_clock_drift(node_id, drift_ms);

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("drift: node {} set to {}ms", node_idx, drift_ms));
    }

    /// Clear clock drift for a specific node.
    ///
    /// Returns the node's simulated clock to normal (no drift).
    pub fn clear_clock_drift(&mut self, node_idx: usize) {
        assert!(node_idx < self.nodes.len(), "Invalid node index");
        let node_id = NodeId::from(node_idx as u64 + 1);

        self.injector.clear_clock_drift(node_id);

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("drift: node {} cleared", node_idx));
    }

    /// Set clock drift for all nodes to simulate heterogeneous timing.
    ///
    /// # Arguments
    /// * `drifts` - Slice of (node_idx, drift_ms) tuples
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Simulate realistic mixed clock drift across the cluster
    /// t.set_cluster_clock_drifts(&[
    ///     (0, 0),    // Reference clock
    ///     (1, 50),   // 50ms fast
    ///     (2, -30),  // 30ms slow
    ///     (3, 75),   // 75ms fast
    ///     (4, -20),  // 20ms slow
    /// ]);
    /// ```
    pub fn set_cluster_clock_drifts(&mut self, drifts: &[(usize, i64)]) {
        for &(node_idx, drift_ms) in drifts {
            assert!(
                node_idx < self.nodes.len(),
                "Invalid node index: {}",
                node_idx
            );
            let node_id = NodeId::from(node_idx as u64 + 1);
            self.injector.set_clock_drift(node_id, drift_ms);
        }

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("drift: set cluster drifts {:?}", drifts));
    }

    /// Clear clock drift for all nodes.
    pub fn clear_all_clock_drifts(&mut self) {
        for i in 0..self.nodes.len() {
            let node_id = NodeId::from(i as u64 + 1);
            self.injector.clear_clock_drift(node_id);
        }

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event("drift: cleared all node drifts");
    }

    /// Get the configured clock drift for a node.
    pub fn get_clock_drift(&self, node_idx: usize) -> Option<i64> {
        assert!(node_idx < self.nodes.len(), "Invalid node index");
        let node_id = NodeId::from(node_idx as u64 + 1);
        self.injector.get_clock_drift(node_id)
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
    /// Reads directly from the state machine (bypassing Raft read API).
    /// Works for in-memory, persistent (SQLite), and Redb nodes.
    pub async fn read(&mut self, key: &str) -> Result<Option<String>> {
        let leader_idx = self
            .check_one_leader()
            .await
            .ok_or_else(|| anyhow::anyhow!("No leader available for read"))?;

        // Read depends on the node type
        let value = match &self.nodes[leader_idx] {
            TestNode::InMemory { state_machine, .. } => state_machine.get(key).await,
            TestNode::Persistent { state_machine, .. } => {
                // SqliteStateMachine.get returns Result<Option<String>, SqliteStorageError>
                state_machine.get(key).await.unwrap_or(None)
            }
            TestNode::Redb { storage, .. } => {
                // SharedRedbStorage.get returns Result<Option<KvEntry>, SharedStorageError>
                storage
                    .get(key)
                    .map(|opt| opt.map(|e| e.value))
                    .unwrap_or(None)
            }
        };
        Ok(value)
    }

    // =========================================================================
    // SQL Query Operations (for Redb nodes)
    // =========================================================================

    /// Execute a SQL query through the current leader.
    ///
    /// Uses Linearizable consistency by default. Only works with Redb nodes.
    ///
    /// # Arguments
    /// * `query` - SQL query string (must be SELECT, not mutation)
    ///
    /// # Returns
    /// * `Ok(SqlQueryResult)` - Query results with columns and rows
    /// * `Err` - If no leader, node doesn't support SQL, or query fails
    ///
    /// # Example
    /// ```ignore
    /// let result = t.execute_sql("SELECT * FROM kv WHERE key LIKE 'user:%'").await?;
    /// assert_eq!(result.row_count, 10);
    /// ```
    pub async fn execute_sql(&mut self, query: &str) -> Result<SqlQueryResult> {
        self.execute_sql_with_consistency(query, SqlConsistency::Linearizable)
            .await
    }

    /// Execute a SQL query with specific consistency level.
    ///
    /// # Arguments
    /// * `query` - SQL query string
    /// * `consistency` - `Linearizable` (uses ReadIndex) or `Stale` (local read)
    ///
    /// # Example
    /// ```ignore
    /// // Linearizable read (through Raft)
    /// let result = t.execute_sql_with_consistency(
    ///     "SELECT COUNT(*) FROM kv",
    ///     SqlConsistency::Linearizable,
    /// ).await?;
    ///
    /// // Stale read (direct from state machine)
    /// let result = t.execute_sql_with_consistency(
    ///     "SELECT * FROM kv",
    ///     SqlConsistency::Stale,
    /// ).await?;
    /// ```
    pub async fn execute_sql_with_consistency(
        &mut self,
        query: &str,
        consistency: SqlConsistency,
    ) -> Result<SqlQueryResult> {
        let leader_idx = self
            .check_one_leader()
            .await
            .ok_or_else(|| anyhow::anyhow!("No leader available for SQL query"))?;

        self.execute_sql_on_node(leader_idx, query, consistency)
            .await
    }

    /// Execute a SQL query on a specific node.
    ///
    /// Useful for testing stale reads on followers or verifying data replication.
    ///
    /// # Arguments
    /// * `node_idx` - 0-based index of the node to query
    /// * `query` - SQL query string
    /// * `consistency` - `Linearizable` or `Stale`
    ///
    /// # Example
    /// ```ignore
    /// // Query follower with stale consistency
    /// let result = t.execute_sql_on_node(1, "SELECT * FROM kv", SqlConsistency::Stale).await?;
    /// ```
    pub async fn execute_sql_on_node(
        &mut self,
        node_idx: usize,
        query: &str,
        consistency: SqlConsistency,
    ) -> Result<SqlQueryResult> {
        assert!(node_idx < self.nodes.len(), "Invalid node index");

        let raft_node = self.nodes[node_idx].raft_node().ok_or_else(|| {
            anyhow::anyhow!("SQL not supported on this node type (use Redb backend)")
        })?;

        let request = SqlQueryRequest {
            query: query.to_string(),
            params: vec![],
            consistency,
            limit: None,
            timeout_ms: Some(10_000), // 10 second timeout for tests
        };

        use crate::api::SqlQueryExecutor;
        let result = raft_node.execute_sql(request).await.map_err(|e| match e {
            SqlQueryError::NotLeader { leader } => {
                anyhow::anyhow!("Not leader, leader hint: {:?}", leader)
            }
            SqlQueryError::NotSupported { backend } => {
                anyhow::anyhow!("SQL not supported on {} backend", backend)
            }
            SqlQueryError::SyntaxError { message } => {
                anyhow::anyhow!("SQL syntax error: {}", message)
            }
            SqlQueryError::ExecutionFailed { reason } => {
                anyhow::anyhow!("SQL execution failed: {}", reason)
            }
            SqlQueryError::Timeout { duration_ms } => {
                anyhow::anyhow!("SQL query timed out after {}ms", duration_ms)
            }
            SqlQueryError::QueryNotAllowed { reason } => {
                anyhow::anyhow!("SQL query not allowed: {}", reason)
            }
            SqlQueryError::QueryTooLarge { size, max } => {
                anyhow::anyhow!("SQL query too large: {} bytes (max {})", size, max)
            }
            SqlQueryError::TooManyParams { count, max } => {
                anyhow::anyhow!("Too many SQL params: {} (max {})", count, max)
            }
        })?;

        self.artifact =
            std::mem::replace(&mut self.artifact, empty_artifact_builder()).add_event(format!(
                "sql: query='{}' returned {} rows",
                query.chars().take(50).collect::<String>(),
                result.row_count
            ));

        Ok(result)
    }

    // =========================================================================
    // Membership Change Operations (Phase 2.1)
    // =========================================================================

    /// Add a learner node to the cluster.
    ///
    /// Learners replicate data but don't participate in consensus votes.
    /// This is typically used before promoting a node to voter.
    ///
    /// # Arguments
    /// * `node_idx` - 0-based index of the node to add as learner
    ///
    /// # Example
    /// ```ignore
    /// // Add node 3 as a learner (it must already exist in the tester)
    /// t.add_learner(3).await?;
    /// ```
    pub async fn add_learner(&mut self, node_idx: usize) -> Result<()> {
        assert!(node_idx < self.nodes.len(), "Invalid node index");

        let leader_idx = self
            .check_one_leader()
            .await
            .ok_or_else(|| anyhow::anyhow!("No leader available for add_learner"))?;

        let node_id = NodeId::from(node_idx as u64 + 1);
        let member_info = create_test_raft_member_info(node_id);

        self.nodes[leader_idx]
            .raft()
            .add_learner(node_id, member_info, true)
            .await
            .map_err(|e| anyhow::anyhow!("add_learner failed: {}", e))?;

        self.metrics.membership_changes += 1;
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("membership: added node {} as learner", node_idx));

        Ok(())
    }

    /// Change cluster membership to a new set of voters.
    ///
    /// This reconfigures the Raft cluster to use a new set of voting members.
    /// The change is applied through joint consensus for safety.
    ///
    /// # Arguments
    /// * `voter_indices` - 0-based indices of nodes that should become voters
    ///
    /// # Example
    /// ```ignore
    /// // Change membership to nodes 0, 1, 2, 4 (removing node 3)
    /// t.change_membership(&[0, 1, 2, 4]).await?;
    /// ```
    pub async fn change_membership(&mut self, voter_indices: &[usize]) -> Result<()> {
        assert!(!voter_indices.is_empty(), "Must have at least one voter");
        for &idx in voter_indices {
            assert!(idx < self.nodes.len(), "Invalid node index: {}", idx);
        }

        let leader_idx = self
            .check_one_leader()
            .await
            .ok_or_else(|| anyhow::anyhow!("No leader available for change_membership"))?;

        let members: BTreeSet<NodeId> = voter_indices
            .iter()
            .map(|&i| NodeId::from(i as u64 + 1))
            .collect();

        self.nodes[leader_idx]
            .raft()
            .change_membership(members.clone(), false)
            .await
            .map_err(|e| anyhow::anyhow!("change_membership failed: {}", e))?;

        self.metrics.membership_changes += 1;
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("membership: changed voters to {:?}", voter_indices));

        Ok(())
    }

    /// Get the current cluster membership state.
    ///
    /// Returns a tuple of (voters, learners) as 0-based node indices.
    pub fn get_membership(&self) -> (Vec<usize>, Vec<usize>) {
        // Find a connected node to query
        for node in &self.nodes {
            if node.connected().load(Ordering::Relaxed) {
                let metrics = node.raft().metrics().borrow().clone();
                let membership = metrics.membership_config.membership();
                let voters: Vec<usize> = membership
                    .voter_ids()
                    .map(|id| (id.0 - 1) as usize)
                    .collect();
                let learners: Vec<usize> = membership
                    .learner_ids()
                    .map(|id| (id.0 - 1) as usize)
                    .collect();
                return (voters, learners);
            }
        }
        (vec![], vec![])
    }

    /// Wait for all connected nodes to reach the same log index.
    ///
    /// This is useful after membership changes to ensure replication.
    ///
    /// # Arguments
    /// * `timeout_secs` - Maximum time to wait for sync
    pub async fn wait_for_log_sync(&mut self, timeout_secs: u64) -> Result<()> {
        let deadline = Duration::from_secs(timeout_secs);
        let start = std::time::Instant::now();

        while start.elapsed() < deadline {
            let mut indices: Vec<u64> = Vec::new();

            for node in &self.nodes {
                if node.connected().load(Ordering::Relaxed) {
                    let metrics = node.raft().metrics().borrow().clone();
                    if let Some(applied) = metrics.last_applied {
                        indices.push(applied.index);
                    }
                }
            }

            // Check if all connected nodes have the same applied index
            if !indices.is_empty() && indices.iter().all(|&i| i == indices[0]) {
                self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
                    .add_event(format!("sync: all nodes at log index {}", indices[0]));
                return Ok(());
            }

            madsim::time::sleep(Duration::from_millis(100)).await;
        }

        anyhow::bail!(
            "Timeout waiting for log sync after {} seconds",
            timeout_secs
        )
    }

    // =========================================================================
    // BUGGIFY Fault Injection Methods (Phase 2.2)
    // =========================================================================

    /// Enable BUGGIFY fault injection for this test.
    ///
    /// # Arguments
    /// * `custom_probs` - Optional custom probabilities for each fault type
    ///
    /// # Example
    /// ```ignore
    /// t.enable_buggify(None);  // Use default probabilities
    ///
    /// // Or with custom probabilities
    /// let mut probs = HashMap::new();
    /// probs.insert(BuggifyFault::NetworkDelay, 0.10);  // 10% chance
    /// probs.insert(BuggifyFault::NodeCrash, 0.02);     // 2% chance
    /// t.enable_buggify(Some(probs));
    /// ```
    pub fn enable_buggify(&mut self, custom_probs: Option<HashMap<BuggifyFault, f64>>) {
        self.buggify.enable(custom_probs);
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event("buggify: enabled with fault injection");
    }

    /// Disable BUGGIFY fault injection.
    pub fn disable_buggify(&mut self) {
        self.buggify.disable();
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event("buggify: disabled");
    }

    /// Apply BUGGIFY faults if they should trigger.
    ///
    /// This method checks each fault type and applies them if triggered.
    /// Called periodically during test execution.
    pub async fn apply_buggify_faults(&mut self) {
        // Network delay
        if self.buggify.should_trigger(BuggifyFault::NetworkDelay) {
            let delay_ms = 50 + (self.seed % 200); // 50-250ms delay

            // Apply delay to all node pairs
            for i in 0..self.nodes.len() {
                for j in 0..self.nodes.len() {
                    if i != j {
                        self.injector.set_network_delay(
                            NodeId::from(i as u64 + 1),
                            NodeId::from(j as u64 + 1),
                            delay_ms,
                        );
                    }
                }
            }

            self.add_event(format!("buggify: injected {}ms network delay", delay_ms));
            self.metrics.buggify_triggers += 1;
        }

        // Network packet drops
        if self.buggify.should_trigger(BuggifyFault::NetworkDrop) {
            // Apply packet loss to all links
            for i in 0..self.nodes.len() {
                for j in 0..self.nodes.len() {
                    if i != j {
                        self.injector.set_packet_loss_rate(
                            NodeId::from(i as u64 + 1),
                            NodeId::from(j as u64 + 1),
                            0.1, // 10% loss rate
                        );
                    }
                }
            }

            self.add_event("buggify: enabled 10% packet drop");
            self.metrics.buggify_triggers += 1;

            // Restore after some time
            madsim::time::sleep(Duration::from_secs(2)).await;

            // Clear packet loss
            for i in 0..self.nodes.len() {
                for j in 0..self.nodes.len() {
                    if i != j {
                        self.injector.set_packet_loss_rate(
                            NodeId::from(i as u64 + 1),
                            NodeId::from(j as u64 + 1),
                            0.0,
                        );
                    }
                }
            }

            self.add_event("buggify: restored packet delivery");
        }

        // Random node crash
        if self.buggify.should_trigger(BuggifyFault::NodeCrash) {
            let connected_nodes: Vec<usize> = self
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.connected().load(Ordering::Relaxed))
                .map(|(i, _)| i)
                .collect();

            if connected_nodes.len() > 2 {
                // Keep at least 2 nodes alive
                let victim = connected_nodes[self.seed as usize % connected_nodes.len()];
                self.crash_node(victim).await;
                self.add_event(format!("buggify: crashed node {}", victim));
                self.metrics.buggify_triggers += 1;
            }
        }

        // Message corruption
        if self.buggify.should_trigger(BuggifyFault::MessageCorruption) {
            // Pick a random Byzantine corruption mode
            let modes = [
                ByzantineCorruptionMode::FlipVote,
                ByzantineCorruptionMode::IncrementTerm,
                ByzantineCorruptionMode::DuplicateMessage,
            ];
            let mode = modes[self.seed as usize % modes.len()];

            // Apply to a random node pair
            let src = self.seed as usize % self.nodes.len();
            let dst = (src + 1) % self.nodes.len();

            self.byzantine_injector.set_byzantine_mode(
                NodeId::from(src as u64 + 1),
                NodeId::from(dst as u64 + 1),
                mode,
                0.5,
            );

            self.add_event(format!(
                "buggify: enabled {:?} corruption on link {}->{}",
                mode, src, dst
            ));
            self.metrics.buggify_triggers += 1;
        }

        // Election timeout (force re-election)
        // Use has_leader_now() instead of check_one_leader() to avoid blocking delays
        if self.buggify.should_trigger(BuggifyFault::ElectionTimeout) && self.has_leader_now() {
            // Find current leader without blocking retries
            let leader_idx = self
                .nodes
                .iter()
                .enumerate()
                .find_map(|(i, node)| {
                    if node.connected().load(Ordering::Relaxed) {
                        let metrics = node.raft().metrics().borrow().clone();
                        metrics.current_leader.map(|_| i)
                    } else {
                        None
                    }
                })
                .unwrap_or(0);

            self.disconnect(leader_idx);
            self.add_event(format!(
                "buggify: partitioned leader {} to force re-election",
                leader_idx
            ));
            self.metrics.buggify_triggers += 1;

            // Restore after election timeout - reduced from 5s to 2s for faster tests
            // This is still longer than election_timeout_max (3s) so elections can complete
            madsim::time::sleep(Duration::from_secs(2)).await;
            self.connect(leader_idx);
            self.add_event(format!(
                "buggify: restored node {} connectivity",
                leader_idx
            ));
        }

        // Network partition
        if self.buggify.should_trigger(BuggifyFault::NetworkPartition) {
            let mid = self.nodes.len() / 2;
            for i in 0..mid {
                self.disconnect(i);
            }
            self.add_event(format!(
                "buggify: created network partition (nodes 0-{} isolated)",
                mid - 1
            ));
            self.metrics.buggify_triggers += 1;

            // Heal after some time - reduced from 10s to 3s for faster tests
            // while still being long enough to force leader re-election
            madsim::time::sleep(Duration::from_secs(3)).await;
            for i in 0..mid {
                self.connect(i);
            }
            self.add_event("buggify: healed network partition");
        }

        // Trigger snapshot
        if self.buggify.should_trigger(BuggifyFault::SnapshotTrigger)
            && let Some(leader_idx) = self.check_one_leader().await
        {
            // Use Raft::trigger().snapshot() to manually trigger snapshot on leader
            let raft = self.nodes[leader_idx].raft();
            if let Err(e) = raft.trigger().snapshot().await {
                self.add_event(format!("buggify: snapshot trigger failed: {:?}", e));
            } else {
                self.add_event("buggify: triggered snapshot on leader");
            }
            self.metrics.buggify_triggers += 1;
        }
    }

    /// Run a test loop with BUGGIFY enabled, periodically applying faults.
    ///
    /// This runs for the specified duration, applying BUGGIFY faults every second.
    ///
    /// # Example
    /// ```ignore
    /// t.enable_buggify(None);
    ///
    /// // Run with BUGGIFY for 30 seconds
    /// t.run_with_buggify_loop(Duration::from_secs(30), async {
    ///     // Check cluster health every 2 seconds
    ///     loop {
    ///         madsim::time::sleep(Duration::from_secs(2)).await;
    ///         t.check_one_leader().await;
    ///     }
    /// }).await;
    /// ```
    pub async fn run_with_buggify_loop(&mut self, duration: Duration) {
        let start = Instant::now();

        while start.elapsed() < duration {
            // Apply faults
            self.apply_buggify_faults().await;

            // Wait before next fault injection
            madsim::time::sleep(Duration::from_secs(1)).await;
        }

        self.add_event(format!(
            "buggify: completed {} seconds of fault injection",
            duration.as_secs()
        ));
    }

    // =========================================================================
    // Event Logging
    // =========================================================================

    /// Add a custom event to the artifact trace.
    pub fn add_event(&mut self, event: impl Into<String>) {
        self.artifact =
            std::mem::replace(&mut self.artifact, empty_artifact_builder()).add_event(event);
    }

    /// Delete a key from the key-value store.
    pub async fn delete(&mut self, key: String) -> Result<()> {
        let leader_idx = self
            .check_one_leader()
            .await
            .ok_or_else(|| anyhow::anyhow!("No leader available"))?;

        let req = AppRequest::Delete { key: key.clone() };

        let res = self.nodes[leader_idx].raft().client_write(req).await?;

        // Track event
        self.artifact =
            std::mem::replace(&mut self.artifact, empty_artifact_builder()).add_event(format!(
                "Delete key '{}' via node {} (response: {:?})",
                key, leader_idx, res.data
            ));

        Ok(())
    }

    /// Trigger an election on a specific node.
    pub async fn trigger_election(&mut self, node_id: u64) -> Result<()> {
        if node_id >= self.nodes.len() as u64 {
            return Err(anyhow::anyhow!("Invalid node ID: {}", node_id));
        }

        self.nodes[node_id as usize]
            .raft()
            .trigger()
            .elect()
            .await?;

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("Triggered election on node {}", node_id));

        Ok(())
    }

    /// Trigger a snapshot on a specific node.
    ///
    /// This manually triggers the state machine to build a snapshot.
    /// Typically called on the leader to force log compaction.
    pub async fn trigger_snapshot(&mut self, node_id: u64) -> Result<()> {
        if node_id >= self.nodes.len() as u64 {
            return Err(anyhow::anyhow!("Invalid node ID: {}", node_id));
        }

        self.nodes[node_id as usize]
            .raft()
            .trigger()
            .snapshot()
            .await?;

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("Triggered snapshot on node {}", node_id));

        Ok(())
    }

    /// Get metrics for a specific node.
    pub fn get_metrics(&self, node_id: u64) -> Option<openraft::RaftMetrics<AppTypeConfig>> {
        if node_id >= self.nodes.len() as u64 {
            return None;
        }
        Some(
            self.nodes[node_id as usize]
                .raft()
                .metrics()
                .borrow()
                .clone(),
        )
    }

    /// End the test and return the simulation artifact.
    ///
    /// This method persists the artifact to disk and returns it for
    /// optional additional processing.
    /// Apply a specific BUGGIFY fault with 100% probability.
    ///
    /// This is useful for property-based testing where you want to inject
    /// specific faults deterministically.
    pub async fn apply_single_fault(&mut self, fault: BuggifyFault) {
        if !self.buggify.enabled.load(Ordering::Relaxed) {
            // Enable BUGGIFY if not already enabled
            self.enable_buggify(None);
        }

        // Save current probabilities
        let saved = self.buggify.probabilities.lock().unwrap().clone();

        // Set 100% probability for this specific fault
        let mut probs = HashMap::new();
        probs.insert(fault, 1.0);
        *self.buggify.probabilities.lock().unwrap() = probs;

        // Apply the fault
        self.apply_buggify_faults().await;

        // Restore original probabilities
        *self.buggify.probabilities.lock().unwrap() = saved;
    }

    // =========================================================================
    // Liveness Mode Testing (TigerBeetle-Style Two-Phase Testing)
    // =========================================================================

    /// Enable liveness tracking at runtime.
    ///
    /// This allows enabling liveness mode after tester creation.
    /// Useful for two-phase testing where you first run safety tests,
    /// then re-run with liveness checking.
    pub fn enable_liveness(&mut self, mode: LivenessMode) {
        self.liveness_config.mode = mode;
        self.liveness_state.active = mode != LivenessMode::Disabled;
        self.add_event(format!("liveness: enabled mode {:?}", mode));
    }

    /// Disable liveness tracking.
    pub fn disable_liveness(&mut self) {
        self.liveness_config.mode = LivenessMode::Disabled;
        self.liveness_state.active = false;
        self.add_event("liveness: disabled");
    }

    /// Check if the cluster currently has a leader (quick check, no retries).
    ///
    /// Unlike `check_one_leader()` which waits with retries, this is a
    /// point-in-time snapshot used for liveness tracking.
    pub fn has_leader_now(&self) -> bool {
        for node in self.nodes.iter() {
            if node.connected().load(Ordering::Relaxed) {
                let metrics = node.raft().metrics().borrow().clone();
                if metrics.current_leader.is_some() {
                    return true;
                }
            }
        }
        false
    }

    /// Perform a liveness check and update internal state.
    ///
    /// This should be called periodically during tests to track liveness.
    /// Returns whether the cluster currently satisfies liveness requirements.
    pub fn check_liveness_tick(&mut self) -> bool {
        if !self.liveness_state.active {
            return true; // Not tracking, always passes
        }

        let now = Instant::now();
        let elapsed_from_start = now.duration_since(self.start_time).as_millis() as u64;
        let has_leader = self.has_leader_now();

        self.metrics.liveness.liveness_checks += 1;

        if has_leader {
            // Record first election time
            if self.liveness_state.first_election_time.is_none() {
                self.liveness_state.first_election_time = Some(now);
                self.metrics.liveness.first_election_ms = elapsed_from_start;
                self.add_event(format!(
                    "liveness: first leader elected at {}ms",
                    elapsed_from_start
                ));
            }

            // If we were leaderless, record recovery time
            if let Some(leaderless_start) = self.liveness_state.leaderless_since {
                let recovery_time = now.duration_since(leaderless_start).as_millis() as u64;
                self.metrics.liveness.leaderless_duration_ms += recovery_time;
                self.metrics.liveness.max_leader_recovery_ms = self
                    .metrics
                    .liveness
                    .max_leader_recovery_ms
                    .max(recovery_time);
                self.liveness_state.leaderless_since = None;
                self.add_event(format!(
                    "liveness: leader recovered after {}ms",
                    recovery_time
                ));
            }

            self.liveness_state.last_leader_time = Some(now);
            self.metrics.liveness.liveness_checks_passed += 1;
            true
        } else {
            // No leader currently
            if self.liveness_state.leaderless_since.is_none() {
                self.liveness_state.leaderless_since = Some(now);
                self.metrics.liveness.leaderless_periods += 1;
            }

            // Check for violations based on mode
            let leaderless_duration = self
                .liveness_state
                .leaderless_since
                .map(|t| now.duration_since(t).as_millis() as u64)
                .unwrap_or(0);

            let violation_threshold = match self.liveness_config.mode {
                LivenessMode::Disabled => u64::MAX, // Never violate
                LivenessMode::Strict => self.liveness_config.check_interval_ms * 2, // Very short
                LivenessMode::Eventual => self.liveness_config.recovery_timeout_ms,
                LivenessMode::CustomTimeout(ms) => ms,
            };

            if leaderless_duration > violation_threshold {
                self.liveness_state.violations.push(LivenessViolation {
                    started_at_ms: elapsed_from_start - leaderless_duration,
                    duration_ms: leaderless_duration,
                    violation_type: ViolationType::LeaderlessTimeout,
                    context: format!(
                        "Cluster leaderless for {}ms (threshold: {}ms)",
                        leaderless_duration, violation_threshold
                    ),
                });
                self.add_event(format!(
                    "liveness: VIOLATION - leaderless for {}ms",
                    leaderless_duration
                ));
                false
            } else {
                self.metrics.liveness.liveness_checks_passed += 1;
                true
            }
        }
    }

    /// Run a test with continuous liveness checking.
    ///
    /// This is the main entry point for TigerBeetle-style liveness testing.
    /// Runs the provided test function while periodically checking liveness.
    ///
    /// # Example
    /// ```ignore
    /// // Create tester with liveness enabled
    /// let config = TesterConfig::new(3, "liveness_test")
    ///     .with_liveness(LivenessConfig::eventual());
    /// let mut t = AspenRaftTester::with_config(config).await;
    ///
    /// // Run test with liveness checking
    /// let report = t.run_with_liveness(Duration::from_secs(30), |t| async {
    ///     // Inject faults
    ///     t.crash_node(0).await;
    ///     madsim::time::sleep(Duration::from_secs(5)).await;
    ///     t.restart_node(0).await;
    ///
    ///     // Verify operations still work
    ///     t.write("key".to_string(), "value".to_string()).await?;
    ///     Ok(())
    /// }).await;
    ///
    /// assert!(report.passed, "Liveness test failed: {}", report.summary);
    /// ```
    pub async fn run_with_liveness<F, Fut>(
        &mut self,
        duration: Duration,
        test_fn: F,
    ) -> LivenessReport
    where
        F: FnOnce(&mut Self) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let start = Instant::now();
        let check_interval = Duration::from_millis(self.liveness_config.check_interval_ms);

        // Initial liveness check
        self.check_liveness_tick();

        // Run the test function
        let test_result = test_fn(self).await;

        // Continue checking until duration expires
        while start.elapsed() < duration {
            self.check_liveness_tick();
            madsim::time::sleep(check_interval).await;
        }

        // Final check
        self.check_liveness_tick();

        // Generate report
        self.generate_liveness_report(test_result)
    }

    /// Run a liveness test loop with BUGGIFY fault injection.
    ///
    /// Combines BUGGIFY fault injection with liveness checking.
    /// This is the most comprehensive test mode.
    ///
    /// # Example
    /// ```ignore
    /// let config = TesterConfig::new(3, "chaos_liveness")
    ///     .with_liveness(LivenessConfig::eventual());
    /// let mut t = AspenRaftTester::with_config(config).await;
    ///
    /// t.enable_buggify(None);
    /// let report = t.run_with_liveness_and_buggify(Duration::from_secs(60)).await;
    ///
    /// assert!(report.passed, "Chaos liveness test failed");
    /// ```
    pub async fn run_with_liveness_and_buggify(&mut self, duration: Duration) -> LivenessReport {
        let start = Instant::now();
        let check_interval = Duration::from_millis(self.liveness_config.check_interval_ms);
        let fault_interval = Duration::from_secs(1);
        let mut last_fault_time = Instant::now();

        self.add_event("liveness: starting combined BUGGIFY + liveness test");

        while start.elapsed() < duration {
            // Check liveness
            self.check_liveness_tick();

            // Apply BUGGIFY faults periodically
            if last_fault_time.elapsed() >= fault_interval {
                self.apply_buggify_faults().await;
                last_fault_time = Instant::now();
            }

            madsim::time::sleep(check_interval).await;
        }

        // Final check
        self.check_liveness_tick();

        self.add_event("liveness: completed BUGGIFY + liveness test");

        self.generate_liveness_report(Ok(()))
    }

    /// Generate a liveness report from current state.
    fn generate_liveness_report(&mut self, test_result: Result<()>) -> LivenessReport {
        let passed = self.liveness_state.violations.is_empty() && test_result.is_ok();

        let summary = if passed {
            format!(
                "Liveness test PASSED: {} checks, {}ms leaderless total, {}ms max recovery",
                self.metrics.liveness.liveness_checks,
                self.metrics.liveness.leaderless_duration_ms,
                self.metrics.liveness.max_leader_recovery_ms
            )
        } else {
            let violation_count = self.liveness_state.violations.len();
            let test_error = test_result.as_ref().err().map(|e| e.to_string());
            format!(
                "Liveness test FAILED: {} violations, test error: {:?}",
                violation_count, test_error
            )
        };

        self.add_event(format!("liveness: {}", summary));

        // Copy metrics for report
        let report_metrics = self.metrics.liveness.clone();
        self.metrics.liveness = report_metrics.clone();

        LivenessReport {
            passed,
            mode: self.liveness_config.mode,
            metrics: report_metrics,
            violations: self.liveness_state.violations.clone(),
            summary,
        }
    }

    /// Get current liveness metrics.
    pub fn liveness_metrics(&self) -> &LivenessMetrics {
        &self.metrics.liveness
    }

    /// Check if any liveness violations have occurred.
    pub fn has_liveness_violations(&self) -> bool {
        !self.liveness_state.violations.is_empty()
    }

    /// Get all liveness violations.
    pub fn liveness_violations(&self) -> &[LivenessViolation] {
        &self.liveness_state.violations
    }

    /// Perform a write with liveness tracking.
    ///
    /// If the write fails due to no leader, this is tracked as blocked time.
    pub async fn write_with_liveness(&mut self, key: String, value: String) -> Result<()> {
        let start = Instant::now();
        let result = self.write(key.clone(), value).await;

        if result.is_ok() {
            self.metrics.liveness.writes_completed += 1;
        } else {
            let blocked_time = start.elapsed().as_millis() as u64;
            self.metrics.liveness.writes_blocked += 1;
            self.metrics.liveness.blocked_duration_ms += blocked_time;

            if self.liveness_state.active {
                let elapsed = Instant::now().duration_since(self.start_time).as_millis() as u64;
                self.liveness_state.violations.push(LivenessViolation {
                    started_at_ms: elapsed - blocked_time,
                    duration_ms: blocked_time,
                    violation_type: ViolationType::OperationBlocked,
                    context: format!("Write blocked for {}ms on key '{}'", blocked_time, key),
                });
            }
        }

        result
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
        let metrics_json =
            serde_json::to_string_pretty(&self.metrics).unwrap_or_else(|_| "{}".to_string());

        let artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .with_metrics(metrics_json)
            .build();

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

#[cfg(test)]
mod tests {
    // Note: These tests require madsim runtime, which is only available
    // with the madsim feature enabled. They are tested via the integration
    // test suite in tests/madsim_tester_test.rs
}
