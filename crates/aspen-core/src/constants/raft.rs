//! Raft consensus constants for Aspen distributed system.
//!
//! This module contains constants for Raft consensus internals including
//! storage, membership, backoff, and integrity verification.
//!
//! Tiger Style: Constants are fixed and immutable, enforced at compile time.

use std::time::Duration;

// ============================================================================
// Storage Constants
// ============================================================================

/// Maximum batch size for log append operations (1000 entries).
///
/// Tiger Style: Bounded batch processing to prevent excessive memory use
/// during bulk write operations. Applied to both redb and SQLite storage.
///
/// Used in:
/// - `storage.rs`: Redb log append pre-allocation
/// - `storage_sqlite.rs`: SQLite write batching logic
pub const MAX_BATCH_SIZE: u32 = 1000;

/// Maximum snapshot entries during build (1,000,000 entries).
///
/// Tiger Style: Fixed limit prevents OOM during snapshot construction.
/// This is a conservative estimate before MAX_SNAPSHOT_SIZE applies.
///
/// Used in:
/// - `storage_sqlite.rs`: Snapshot builder validation
pub const MAX_SNAPSHOT_ENTRIES: u32 = 1_000_000;

/// Default size for the SQLite read connection pool (50 connections).
///
/// Tiger Style: Fixed pool size prevents unbounded connection creation.
/// Sized at 5% of MAX_CONCURRENT_OPS (1000) to balance concurrency against
/// resource usage. SQLite WAL mode handles many concurrent readers efficiently.
///
/// Used in:
/// - `storage_sqlite.rs`: Connection pool initialization
/// - `cluster/config.rs`: Default for NodeConfig.sqlite_read_pool_size
pub const DEFAULT_READ_POOL_SIZE: u32 = 50;

// ============================================================================
// Actor and Concurrency Constants
// ============================================================================

/// Default capacity for bounded proxy queues (1000 items).
///
/// Tiger Style: Initial queue capacity prevents pathological growth.
/// Used for elastic sizing between min and max capacity.
///
/// Used in:
/// - `bounded_proxy.rs`: Queue initialization
pub const DEFAULT_CAPACITY: u32 = 1000;

/// Maximum capacity for bounded proxy queues (10,000 items).
///
/// Tiger Style: Upper bound on queue growth to prevent unbounded allocation.
/// Hard limit regardless of demand.
///
/// Used in:
/// - `bounded_proxy.rs`: Queue maximum size
pub const MAX_CAPACITY: u32 = 10_000;

/// Maximum number of concurrent connections per node in madsim (100).
///
/// Tiger Style: Fixed limit to prevent connection exhaustion during simulation.
/// Applies only to madsim deterministic network, not to Iroh.
///
/// Used in:
/// - `madsim_network.rs`: Connection management
pub const MAX_CONNECTIONS_PER_NODE: u32 = 100;

/// Maximum number of voters in the cluster (100 nodes).
///
/// Tiger Style: Bounded voter count prevents consensus complexity explosion.
/// Maintains predictable Raft quorum sizes.
///
/// Used in:
/// - `learner_promotion.rs`: Membership change validation
pub const MAX_VOTERS: u32 = 100;

// ============================================================================
// Learning and Replication Constants
// ============================================================================

/// Maximum replication lag threshold for learner promotion (100 log entries).
///
/// Tiger Style: Fixed threshold ensures learners are sufficiently caught up
/// before promotion to voters. Prevents cascade failures.
///
/// Used in:
/// - `learner_promotion.rs`: Learner readiness validation
pub const LEARNER_LAG_THRESHOLD: u64 = 100;

/// Cooldown period for membership changes (300 seconds / 5 minutes).
///
/// Tiger Style: Fixed cooldown prevents rapid membership changes that could
/// destabilize consensus. Allows time for quorum stabilization.
///
/// Used in:
/// - `learner_promotion.rs`: Membership change rate limiting
pub const MEMBERSHIP_COOLDOWN: Duration = Duration::from_secs(300);

// ============================================================================
// Supervision and Fault Tolerance Constants
// ============================================================================

/// Maximum size of restart history for circuit breaker (100 entries).
///
/// Tiger Style: Bounded history prevents unbounded memory growth in restart
/// tracking. Sufficient for failure pattern analysis.
///
/// Used in:
/// - `supervision.rs`: RestartCircuitBreaker state management
pub const MAX_RESTART_HISTORY_SIZE: u32 = 100;

/// Maximum backoff duration for exponential backoff (16 seconds).
///
/// Tiger Style: Upper bound on retry delays prevents stale-client-discovery bugs.
/// Allows faster recovery when faults are transient.
///
/// Used in:
/// - `supervision.rs`: Exponential backoff calculation (2^MAX_BACKOFF_SECONDS)
pub const MAX_BACKOFF_SECONDS: u64 = 16;

// ============================================================================
// Clock Drift Detection Constants
// ============================================================================
// Note: Clock synchronization is NOT required for Raft consensus correctness.
// Raft uses logical ordering (term + index) and monotonic clocks for timeouts.
// These constants are for observational monitoring to help operators detect
// NTP misconfiguration that could affect TLS certificates and debugging.

/// Maximum number of nodes to track for clock drift observation (100).
///
/// Tiger Style: Bounded storage for clock drift tracking.
/// Prevents unbounded growth of drift observation state.
///
/// Used in:
/// - `clock_drift_detection.rs`: ClockDriftDetector storage bounds
pub const MAX_DRIFT_OBSERVATIONS: u32 = 100;

/// Clock drift warning threshold (100 milliseconds).
///
/// When estimated clock offset between nodes exceeds this value,
/// a warning is logged. This is purely observational and does not
/// affect Raft consensus (which uses monotonic clocks).
///
/// Typical NTP-synchronized machines have < 100ms drift.
///
/// Used in:
/// - `clock_drift_detection.rs`: DriftSeverity classification
pub const CLOCK_DRIFT_WARNING_THRESHOLD_MS: u64 = 100;

/// Clock drift alert threshold (500 milliseconds).
///
/// When estimated clock offset between nodes exceeds this value,
/// an alert-level log is emitted. This indicates potential NTP
/// misconfiguration that should be investigated for operational health.
///
/// Used in:
/// - `clock_drift_detection.rs`: DriftSeverity classification
pub const CLOCK_DRIFT_ALERT_THRESHOLD_MS: u64 = 500;

/// Exponential weighted moving average alpha for drift smoothing (0.1).
///
/// Controls how quickly drift estimates adapt to new observations.
/// Lower values (like 0.1) provide more smoothing, reducing noise
/// but slower adaptation. Value of 0.1 means ~10 samples for full weight.
///
/// Used in:
/// - `clock_drift_detection.rs`: EWMA calculation
pub const DRIFT_EWMA_ALPHA: f64 = 0.1;

/// Minimum observations before reporting clock drift (3).
///
/// Prevents false positives from single outlier measurements.
/// Drift is only reported after this many consistent observations.
///
/// Tiger Style: Explicit threshold prevents noisy alerts.
///
/// Used in:
/// - `clock_drift_detection.rs`: Observation count validation
pub const MIN_DRIFT_OBSERVATIONS: u32 = 3;

// ============================================================================
// Chain Hashing / Integrity Verification Constants
// ============================================================================
// Chain hashing provides cryptographic integrity verification for Raft log
// entries, detecting hardware corruption (bit flips, disk errors) and
// Byzantine modification attempts.

/// Background chain verification interval (300 seconds / 5 minutes).
///
/// Controls how often the background verification task runs to check chain
/// integrity. Set to 5 minutes to balance corruption detection timeliness
/// against CPU overhead.
///
/// Tiger Style: Fixed interval prevents runaway CPU usage while ensuring
/// timely corruption detection.
///
/// Used in:
/// - `integrity.rs`: Background chain verifier task scheduling
pub const CHAIN_VERIFY_INTERVAL_SECS: u64 = 300;

/// Number of entries to verify per background batch (1000).
///
/// Controls how many log entries are verified in each background verification
/// cycle. Larger batches reduce verification overhead but increase per-cycle
/// latency.
///
/// Tiger Style: Fixed batch size bounds memory and CPU per iteration.
///
/// Used in:
/// - `integrity.rs`: Background chain verifier batch sizing
pub const CHAIN_VERIFY_BATCH_SIZE: u32 = 1000;

/// Integrity schema version for migration tracking (1).
///
/// Incremented when the chain hashing schema changes. Used to detect
/// databases that need migration to add chain hashing support.
///
/// Used in:
/// - `storage.rs`: Database migration for chain hashing
pub const INTEGRITY_VERSION: u32 = 1;

// ============================================================================
// Pipeline Recovery Constants
// ============================================================================
// Constants for detecting and recovering orphaned pipelines after leader crash.

/// Job orphan detection threshold (5 minutes).
///
/// Tiger Style: Fixed threshold for detecting jobs that may have been orphaned.
/// If a job's heartbeat is older than this, consider it potentially orphaned.
///
/// Used in:
/// - `aspen-ci/orchestrator/recovery.rs`: Orphan detection logic
pub const JOB_ORPHAN_DETECTION_THRESHOLD_MS: u64 = 300_000;

/// Maximum pipelines to recover per scan (50).
///
/// Tiger Style: Bounded recovery to prevent recovery storm after leader election.
/// Multiple scans can handle larger backlogs.
///
/// Used in:
/// - `aspen-ci/orchestrator/recovery.rs`: Recovery batch size
pub const MAX_PIPELINE_RECOVERY_BATCH: u32 = 50;

/// Job heartbeat interval (10 seconds).
///
/// Tiger Style: Fixed interval for job liveness updates.
/// Workers update job heartbeat at this interval during execution.
///
/// Used in:
/// - `aspen-jobs/worker.rs`: Job heartbeat update loop
pub const JOB_HEARTBEAT_INTERVAL_MS: u64 = 10_000;

// ============================================================================
// Memory Pressure Detection
// ============================================================================
// Constants for detecting low memory conditions and taking protective action.

/// Memory usage warning threshold (80%).
///
/// Tiger Style: Fixed threshold for early warning of memory pressure.
/// At 80% usage, new CI jobs should be paused.
///
/// Used in:
/// - `aspen-cluster/memory_watcher.rs`: MemoryPressureLevel classification
pub const MEMORY_PRESSURE_WARNING_PERCENT: u64 = 80;

/// Memory usage critical threshold (90%).
///
/// Tiger Style: Fixed threshold for critical memory pressure.
/// At 90% usage, running CI jobs should be cancelled.
///
/// Used in:
/// - `aspen-cluster/memory_watcher.rs`: MemoryPressureLevel classification
pub const MEMORY_PRESSURE_CRITICAL_PERCENT: u64 = 90;

/// Memory watcher polling interval (1 second).
///
/// Tiger Style: Fixed interval for memory pressure checks.
/// 1 second balances responsiveness against overhead.
///
/// Used in:
/// - `aspen-cluster/memory_watcher.rs`: MemoryWatcher polling loop
pub const MEMORY_WATCHER_INTERVAL_MS: u64 = 1000;

/// Memory reserved for Raft consensus operations (512 MB).
///
/// Tiger Style: Ensures Raft always has memory for heartbeats and log replication.
/// System should maintain at least this much free memory.
///
/// Used in:
/// - `aspen-cluster/memory_watcher.rs`: Available memory threshold
pub const MEMORY_RESERVED_FOR_RAFT_BYTES: u64 = 512 * 1024 * 1024;
