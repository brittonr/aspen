//! Configuration types and constants for the madsim tester.

use aspen_raft::storage::StorageBackend;

use super::liveness::LivenessConfig;

// Tiger Style: Fixed limits for simulation testing
pub(crate) const MAX_TESTER_NODES: usize = 64;
pub(crate) const DEFAULT_HEARTBEAT_INTERVAL_MS: u64 = 500;
pub(crate) const DEFAULT_ELECTION_TIMEOUT_MIN_MS: u64 = 1500;
pub(crate) const DEFAULT_ELECTION_TIMEOUT_MAX_MS: u64 = 3000;
// Leader check timing - reduced from 10 retries x 450-550ms (5.5s worst case)
// to 5 retries x 200-300ms (1.5s worst case) to avoid test timeouts
pub(crate) const LEADER_CHECK_RETRIES: u32 = 5;
pub(crate) const LEADER_CHECK_BACKOFF_MIN_MS: u64 = 200;
pub(crate) const LEADER_CHECK_BACKOFF_MAX_MS: u64 = 300;

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
    /// Storage backend to use (InMemory or Redb for persistence).
    pub storage_backend: StorageBackend,
    /// Base directory for persistent storage (only used with Redb backend).
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
    pub fn with_timeouts(mut self, heartbeat_ms: u64, election_min_ms: u64, election_max_ms: u64) -> Self {
        self.heartbeat_interval_ms = heartbeat_ms;
        self.election_timeout_min_ms = election_min_ms;
        self.election_timeout_max_ms = election_max_ms;
        self
    }

    /// Use persistent storage (Redb backend) for testing crash recovery.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = TesterConfig::new(3, "crash_recovery")
    ///     .with_persistent_storage("/tmp/aspen-test");
    /// ```
    pub fn with_persistent_storage(mut self, storage_dir: impl Into<std::path::PathBuf>) -> Self {
        self.storage_backend = StorageBackend::Redb;
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
