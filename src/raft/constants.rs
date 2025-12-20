//! Centralized constants for the Raft module.
//!
//! This module contains all configuration constants used throughout the Raft
//! implementation, organized by category for easy discovery and maintenance.
//!
//! Tiger Style: Constants are fixed and immutable, enforced at compile time.
//! Each constant has explicit bounds to prevent unbounded resource allocation.

use std::time::Duration;

// ============================================================================
// Network Constants
// ============================================================================

/// Maximum size for RPC messages (10 MB).
///
/// Tiger Style: Fixed limit to prevent unbounded memory use during RPC serialization
/// and deserialization. Applied to both request and response payloads.
///
/// Used in:
/// - `network.rs`: Message reading with `read_to_end(MAX_RPC_MESSAGE_SIZE)`
/// - `server.rs`: RPC message deserialization from streams
pub const MAX_RPC_MESSAGE_SIZE: u32 = 10 * 1024 * 1024;

/// Timeout for Iroh connection establishment (5 seconds).
///
/// Tiger Style: Explicit timeout prevents indefinite hangs on unreachable peers.
/// Applied when initiating peer connections.
///
/// Used in:
/// - `network.rs`: `endpoint.connect()` with timeout wrapper
pub const IROH_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout for bidirectional stream open (2 seconds).
///
/// Tiger Style: Bounded wait for stream establishment after connection succeeds.
/// Prevents indefinite blocking during stream initialization.
///
/// Used in:
/// - `network.rs`: `connection.open_bi()` with timeout wrapper
pub const IROH_STREAM_OPEN_TIMEOUT: Duration = Duration::from_secs(2);

/// Timeout for RPC response read (10 seconds).
///
/// Accounts for slow snapshot transfers and disk I/O from the peer.
/// Tiger Style: Prevents indefinite blocking on slow or stalled peers.
/// Much higher than connect/stream timeouts due to variable snapshot sizes.
///
/// Used in:
/// - `network.rs`: `recv_stream.read_to_end()` with timeout wrapper
pub const IROH_READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum snapshot size (100 MB).
///
/// Tiger Style: Fixed limit prevents unbounded memory allocation from malicious
/// or corrupt snapshots. Prevents DoS attacks via large snapshot payloads.
///
/// Used in:
/// - `network.rs`: Chunked snapshot reading with size validation
pub const MAX_SNAPSHOT_SIZE: u64 = 100 * 1024 * 1024;

// ============================================================================
// Failure Detection Constants
// ============================================================================

/// Maximum number of tracked unreachable nodes (1000).
///
/// Tiger Style: Bounded storage for node failure tracking.
/// Prevents unbounded growth of failure detector state even if many nodes crash.
///
/// Used in:
/// - `node_failure_detection.rs`: NodeFailureDetector state management
pub const MAX_UNREACHABLE_NODES: u32 = 1000;

/// Maximum number of peers to track in gossip discovery (1000).
///
/// Tiger Style: Bounded storage for peer discovery state.
/// Prevents unbounded growth of peer list even in large clusters.
///
/// Used in:
/// - `gossip_actor.rs`: GossipActor peer management
pub const MAX_PEER_COUNT: u32 = 1000;

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

/// Maximum number of keys in a SetMulti operation (100 keys).
///
/// Tiger Style: Fixed limit on multi-key operations prevents pathological
/// cases with unbounded key counts.
///
/// Used in:
/// - `storage_sqlite.rs`: SetMulti validation
/// - `mod.rs`: Write handler validation
pub const MAX_SETMULTI_KEYS: u32 = 100;

/// Maximum size of a single key in bytes (1 KB).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from oversized keys.
/// Applied to all write operations before they reach the Raft log.
///
/// Used in:
/// - `mod.rs`: Write handler validation
pub const MAX_KEY_SIZE: u32 = 1024;

/// Maximum size of a single value in bytes (1 MB).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from oversized values.
/// Applied to all write operations before they reach the Raft log.
///
/// Used in:
/// - `mod.rs`: Write handler validation
pub const MAX_VALUE_SIZE: u32 = 1024 * 1024;

/// Maximum number of concurrent streams per connection (100).
///
/// Tiger Style: Fixed limit prevents DoS attacks via unbounded stream creation.
/// Applied in the RPC server to limit streams from any single peer.
///
/// Used in:
/// - `server.rs`: Stream acceptance limits
pub const MAX_STREAMS_PER_CONNECTION: u32 = 100;

/// Maximum number of concurrent connections (500).
///
/// Tiger Style: Fixed limit prevents connection exhaustion attacks.
/// Applied in the RPC server to limit total concurrent connections.
///
/// Used in:
/// - `server.rs`: Connection acceptance limits
pub const MAX_CONCURRENT_CONNECTIONS: u32 = 500;

/// Maximum number of peers in the peer map (1000).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from peer map growth.
/// Applied in the network factory to prevent Sybil attacks.
///
/// Used in:
/// - `network.rs`: Peer map bounds
pub const MAX_PEERS: u32 = 1000;

/// Maximum snapshot entries during build (1,000,000 entries).
///
/// Tiger Style: Fixed limit prevents OOM during snapshot construction.
/// This is a conservative estimate before MAX_SNAPSHOT_SIZE applies.
///
/// Used in:
/// - `storage_sqlite.rs`: Snapshot builder validation
pub const MAX_SNAPSHOT_ENTRIES: u32 = 1_000_000;

/// Default size for the SQLite read connection pool (10 connections).
///
/// Tiger Style: Fixed pool size prevents unbounded connection creation.
/// Balances concurrency against resource usage.
///
/// Used in:
/// - `storage_sqlite.rs`: Connection pool initialization
pub const DEFAULT_READ_POOL_SIZE: u32 = 10;

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
// Gossip Rate Limiting Constants (HIGH-6 Security Enhancement)
// ============================================================================

/// Maximum number of peers to track for rate limiting (256).
///
/// Tiger Style: Bounded LRU-style cache prevents unbounded memory growth.
/// Sufficient for tracking active gossipers while limiting memory to ~8KB.
/// Oldest entries are evicted when limit is reached.
///
/// Used in:
/// - `gossip_discovery.rs`: Per-peer rate limiter cache size
pub const GOSSIP_MAX_TRACKED_PEERS: usize = 256;

/// Per-peer gossip announcement rate limit (messages per minute).
///
/// Tiger Style: Fixed limit prevents individual peer abuse.
/// Allows normal operation (6/minute at 10s interval) with margin for retries.
/// Value of 12 allows 1 message every 5 seconds average.
///
/// Used in:
/// - `gossip_discovery.rs`: Per-peer rate limiter threshold
pub const GOSSIP_PER_PEER_RATE_PER_MINUTE: u32 = 12;

/// Per-peer gossip announcement burst capacity (messages).
///
/// Tiger Style: Allows brief bursts (e.g., reconnection) without blocking.
/// A peer can send up to this many messages before rate limiting kicks in.
///
/// Used in:
/// - `gossip_discovery.rs`: Per-peer burst allowance
pub const GOSSIP_PER_PEER_BURST: u32 = 3;

/// Global gossip announcement rate limit (messages per minute).
///
/// Tiger Style: Cluster-wide bandwidth protection for 1000+ peer scenarios.
/// With 1000 peers each sending 6/minute, worst case is 6000/minute.
/// Setting to 10,000 provides margin for bursts and network recovery.
///
/// Used in:
/// - `gossip_discovery.rs`: Global rate limiter threshold
pub const GOSSIP_GLOBAL_RATE_PER_MINUTE: u32 = 10_000;

/// Global gossip announcement burst capacity (messages).
///
/// Tiger Style: Handles cluster-wide startup bursts gracefully.
/// Allows brief spikes without immediate rejection.
///
/// Used in:
/// - `gossip_discovery.rs`: Global burst allowance
pub const GOSSIP_GLOBAL_BURST: u32 = 100;

// ============================================================================
// Gossip Error Recovery Constants
// ============================================================================

/// Maximum gossip stream error retries before giving up (5 retries).
///
/// Tiger Style: Bounded retry count prevents infinite retry loops.
/// After this many consecutive errors, the receiver task exits.
///
/// Used in:
/// - `gossip_discovery.rs`: Receiver task error handling
pub const GOSSIP_MAX_STREAM_RETRIES: u32 = 5;

/// Gossip stream error backoff durations in seconds.
///
/// Tiger Style: Fixed backoff progression: 1s, 2s, 4s, 8s, 16s (capped).
/// Exponential backoff prevents overwhelming a recovering network.
///
/// Used in:
/// - `gossip_discovery.rs`: Receiver task backoff calculation
pub const GOSSIP_STREAM_BACKOFF_SECS: [u64; 5] = [1, 2, 4, 8, 16];

// ============================================================================
// Gossip Announcer Rate Limiting Constants
// ============================================================================

/// Minimum interval between peer announcements (10 seconds).
///
/// Tiger Style: Fixed floor prevents announcement flooding.
/// This is the normal announcement interval when network is healthy.
///
/// Used in:
/// - `gossip_discovery.rs`: Announcer task interval
pub const GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS: u64 = 10;

/// Maximum interval between peer announcements (60 seconds).
///
/// Tiger Style: Upper bound on backoff prevents stale discovery.
/// Used when announcements are failing to avoid network flooding.
///
/// Used in:
/// - `gossip_discovery.rs`: Announcer task adaptive interval
pub const GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS: u64 = 60;

/// Consecutive announcement failures before increasing interval (3).
///
/// Tiger Style: Bounded failures before adaptive backoff.
/// Prevents flooding the network when broadcast consistently fails.
///
/// Used in:
/// - `gossip_discovery.rs`: Announcer task adaptive logic
pub const GOSSIP_ANNOUNCE_FAILURE_THRESHOLD: u32 = 3;
