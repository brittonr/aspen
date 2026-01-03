//! Centralized constants for Aspen distributed system.
//!
//! This crate contains all configuration constants used throughout the Aspen
//! implementation, organized by category for easy discovery and maintenance.
//!
//! Tiger Style: Constants are fixed and immutable, enforced at compile time.
//! Each constant has explicit bounds to prevent unbounded resource allocation.

use std::time::Duration;

// ============================================================================
// Key-Value Size Limits
// ============================================================================

/// Maximum size of a single key in bytes (1 KB).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from oversized keys.
/// Applied to all write operations before they reach the Raft log.
pub const MAX_KEY_SIZE: u32 = 1024;

/// Maximum size of a single value in bytes (1 MB).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from oversized values.
/// Applied to all write operations before they reach the Raft log.
pub const MAX_VALUE_SIZE: u32 = 1024 * 1024;

/// Maximum number of keys in a SetMulti operation (100 keys).
///
/// Tiger Style: Fixed limit on multi-key operations prevents pathological
/// cases with unbounded key counts.
pub const MAX_SETMULTI_KEYS: u32 = 100;

// ============================================================================
// SQL Query Constants
// ============================================================================

/// Maximum SQL query string length (64 KB).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from oversized queries.
/// Most practical queries are < 10 KB; 64 KB allows for complex CTEs.
pub const MAX_SQL_QUERY_SIZE: u32 = 64 * 1024;

/// Maximum number of query parameters (100).
///
/// Tiger Style: Bounded parameter count prevents pathological cases.
/// Most queries use < 10 parameters; 100 allows for bulk IN clauses.
pub const MAX_SQL_PARAMS: u32 = 100;

/// Maximum rows returned from SQL query (10,000).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from large result sets.
/// Clients can use pagination for larger result sets.
pub const MAX_SQL_RESULT_ROWS: u32 = 10_000;

/// Default rows returned from SQL query (1,000).
///
/// Tiger Style: Reasonable default that balances utility against resource use.
/// Applied when client doesn't specify a limit.
pub const DEFAULT_SQL_RESULT_ROWS: u32 = 1_000;

/// Maximum SQL query timeout in milliseconds (30 seconds).
///
/// Tiger Style: Upper bound on query execution time.
/// Prevents clients from requesting indefinite timeouts.
pub const MAX_SQL_TIMEOUT_MS: u32 = 30_000;

/// Default SQL query timeout in milliseconds (5 seconds).
///
/// Tiger Style: Explicit timeout prevents runaway queries.
/// 5 seconds is sufficient for most indexed queries.
pub const DEFAULT_SQL_TIMEOUT_MS: u32 = 5_000;

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

// ============================================================================
// Client RPC Rate Limiting Constants
// ============================================================================

/// Per-client request rate limit (requests per second).
///
/// Default: Effectively unlimited (1M requests/sec).
/// Set to a lower value (e.g., 100.0) to enable rate limiting for DoS protection.
///
/// Used in:
/// - `protocol_handlers.rs`: Per-client rate limiter
pub const CLIENT_RPC_RATE_PER_SECOND: f64 = 1_000_000.0;

/// Per-client request burst capacity.
///
/// Default: Effectively unlimited (1M tokens).
/// Set to a lower value (e.g., 50) to enable rate limiting for DoS protection.
///
/// Used in:
/// - `protocol_handlers.rs`: Per-client rate limiter burst
pub const CLIENT_RPC_BURST: u64 = 1_000_000;

/// System key prefix for internal rate limiter state.
///
/// Tiger Style: Isolates internal state from user keyspace.
///
/// Used in:
/// - `protocol_handlers.rs`: Rate limiter key construction
pub const CLIENT_RPC_RATE_LIMIT_PREFIX: &str = "_system:ratelimit:client:";

/// System key for cluster-wide request counter.
///
/// Tiger Style: Single counter tracks total requests across cluster.
///
/// Used in:
/// - `protocol_handlers.rs`: Request counting for metrics
pub const CLIENT_RPC_REQUEST_COUNTER: &str = "_system:metrics:client_requests_total";

/// System key for request ID sequence generator.
///
/// Tiger Style: Monotonic sequence ensures unique, cluster-wide request IDs.
///
/// Used in:
/// - `protocol_handlers.rs`: Request ID generation for tracing
pub const CLIENT_RPC_REQUEST_ID_SEQUENCE: &str = "_system:sequence:request_id";

// ============================================================================
// Distributed Queue Constants
// ============================================================================

/// Maximum queue item payload size (1 MB, same as MAX_VALUE_SIZE).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from oversized payloads.
/// Queue items should be bounded like all other values.
///
/// Used in:
/// - `coordination/queue.rs`: Enqueue payload validation
pub const MAX_QUEUE_ITEM_SIZE: u32 = 1024 * 1024;

/// Maximum items in a batch enqueue/dequeue operation (100).
///
/// Tiger Style: Bounded batch size prevents pathological memory use.
/// Balances throughput against memory consumption.
///
/// Used in:
/// - `coordination/queue.rs`: Batch operation limits
pub const MAX_QUEUE_BATCH_SIZE: u32 = 100;

/// Maximum visibility timeout (1 hour = 3,600,000 ms).
///
/// Tiger Style: Upper bound prevents indefinite item locking.
/// Most processing should complete within 1 hour.
///
/// Used in:
/// - `coordination/queue.rs`: Visibility timeout validation
pub const MAX_QUEUE_VISIBILITY_TIMEOUT_MS: u64 = 3_600_000;

/// Default visibility timeout (30 seconds = 30,000 ms).
///
/// Tiger Style: Reasonable default for most processing tasks.
/// Short enough to detect failures quickly, long enough for typical work.
///
/// Used in:
/// - `coordination/queue.rs`: Default config value
pub const DEFAULT_QUEUE_VISIBILITY_TIMEOUT_MS: u64 = 30_000;

/// Maximum queue item TTL (7 days = 604,800,000 ms).
///
/// Tiger Style: Upper bound on item lifetime prevents indefinite storage.
/// Items older than 7 days are likely stale.
///
/// Used in:
/// - `coordination/queue.rs`: TTL validation
pub const MAX_QUEUE_ITEM_TTL_MS: u64 = 7 * 24 * 60 * 60 * 1000;

/// Default deduplication window TTL (5 minutes = 300,000 ms).
///
/// Tiger Style: Reasonable window for catching duplicate submissions.
/// Long enough for retry scenarios, short enough to not bloat storage.
///
/// Used in:
/// - `coordination/queue.rs`: Deduplication entry TTL
pub const DEFAULT_QUEUE_DEDUP_TTL_MS: u64 = 300_000;

/// Maximum items to scan during cleanup operations (1000).
///
/// Tiger Style: Bounded cleanup prevents pathological scan times.
/// Multiple cleanup cycles can handle larger backlogs.
///
/// Used in:
/// - `coordination/queue.rs`: Expired item cleanup
pub const MAX_QUEUE_CLEANUP_BATCH: u32 = 1000;

/// Default polling interval for blocking dequeue (100 ms).
///
/// Tiger Style: Bounded polling frequency prevents CPU spin.
/// Short enough for responsive dequeue, long enough to avoid overhead.
///
/// Used in:
/// - `coordination/queue.rs`: dequeue_wait polling
pub const DEFAULT_QUEUE_POLL_INTERVAL_MS: u64 = 100;

/// Maximum polling interval for blocking dequeue (1 second).
///
/// Tiger Style: Upper bound on backoff prevents long waits.
/// Used when queue is empty for extended periods.
///
/// Used in:
/// - `coordination/queue.rs`: dequeue_wait backoff cap
pub const MAX_QUEUE_POLL_INTERVAL_MS: u64 = 1000;

// ============================================================================
// Service Registry Constants
// ============================================================================

/// Maximum service name length (256 bytes).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from oversized names.
/// Allows descriptive names like "com.example.user-service.api.v2".
///
/// Used in:
/// - `coordination/registry.rs`: Service name validation
pub const MAX_SERVICE_NAME_SIZE: u32 = 256;

/// Maximum instance ID length (256 bytes).
///
/// Tiger Style: Fixed limit for instance identifiers.
/// Allows UUIDs, hostnames, or composite IDs.
///
/// Used in:
/// - `coordination/registry.rs`: Instance ID validation
pub const MAX_INSTANCE_ID_SIZE: u32 = 256;

/// Maximum service address length (512 bytes).
///
/// Tiger Style: Fixed limit for network addresses.
/// Allows for long DNS names with ports (e.g., "very-long-hostname.example.com:8080").
///
/// Used in:
/// - `coordination/registry.rs`: Address validation
pub const MAX_SERVICE_ADDRESS_SIZE: u32 = 512;

/// Maximum number of tags per service instance (32).
///
/// Tiger Style: Bounded tag count prevents pathological cases.
/// Sufficient for environment, region, version, and custom labels.
///
/// Used in:
/// - `coordination/registry.rs`: Tag array validation
pub const MAX_SERVICE_TAGS: u32 = 32;

/// Maximum tag length (128 bytes).
///
/// Tiger Style: Fixed limit for individual tag strings.
/// Allows descriptive tags like "region:us-east-1" or "version:1.2.3-beta".
///
/// Used in:
/// - `coordination/registry.rs`: Tag string validation
pub const MAX_SERVICE_TAG_SIZE: u32 = 128;

/// Maximum custom metadata entries per instance (32).
///
/// Tiger Style: Bounded metadata prevents unbounded growth.
/// Sufficient for common use cases (protocol, capabilities, etc.).
///
/// Used in:
/// - `coordination/registry.rs`: Custom metadata validation
pub const MAX_SERVICE_CUSTOM_METADATA: u32 = 32;

/// Maximum instances per service (10,000).
///
/// Tiger Style: Bounded to prevent unbounded growth.
/// Sufficient for large-scale services with many replicas.
///
/// Used in:
/// - `coordination/registry.rs`: Instance count limits
pub const MAX_SERVICE_INSTANCES: u32 = 10_000;

/// Maximum instances to return in a single discovery (1,000).
///
/// Tiger Style: Bounded result set prevents memory exhaustion.
/// Clients can filter or paginate for larger sets.
///
/// Used in:
/// - `coordination/registry.rs`: Discovery result limits
pub const MAX_SERVICE_DISCOVERY_RESULTS: u32 = 1_000;

/// Default service instance TTL (30 seconds = 30,000 ms).
///
/// Tiger Style: Reasonable default for service health.
/// Short enough to detect failures quickly, long enough for heartbeats.
///
/// Used in:
/// - `coordination/registry.rs`: Default registration TTL
pub const DEFAULT_SERVICE_TTL_MS: u64 = 30_000;

/// Maximum service instance TTL (24 hours = 86,400,000 ms).
///
/// Tiger Style: Upper bound prevents indefinite registration.
/// Long-lived services should renew via heartbeat.
///
/// Used in:
/// - `coordination/registry.rs`: TTL validation
pub const MAX_SERVICE_TTL_MS: u64 = 24 * 60 * 60 * 1000;

/// Cleanup batch size for expired instance removal (100).
///
/// Tiger Style: Bounded cleanup prevents pathological scan times.
/// Multiple cleanup cycles can handle larger backlogs.
///
/// Used in:
/// - `coordination/registry.rs`: Expired instance cleanup
pub const SERVICE_CLEANUP_BATCH: u32 = 100;

// ============================================================================
// Capability Token Constants
// ============================================================================

/// Maximum number of capabilities per token (32).
///
/// Tiger Style: Bounded to prevent token bloat and DoS.
pub const MAX_CAPABILITIES_PER_TOKEN: u32 = 32;

/// Maximum delegation chain depth (8 levels).
///
/// Tiger Style: Bounded to prevent unbounded proof chains.
/// Root -> Service -> User -> ... max 8 levels
pub const MAX_DELEGATION_DEPTH: u8 = 8;

/// Maximum token size in bytes (8 KB).
///
/// Tiger Style: Bounded to prevent oversized tokens.
/// Typical token with 10 capabilities is ~500 bytes.
pub const MAX_TOKEN_SIZE: u32 = 8 * 1024;

/// Maximum revocation list size (10,000 entries).
///
/// Tiger Style: Bounded to prevent unbounded memory growth.
/// Old revocations can be pruned after token expiry.
pub const MAX_REVOCATION_LIST_SIZE: u32 = 10_000;

/// Token clock skew tolerance (60 seconds).
///
/// Tiger Style: Fixed tolerance for clock drift between nodes.
pub const TOKEN_CLOCK_SKEW_SECS: u64 = 60;

// ============================================================================
// Hanging Prevention Timeout Constants
// ============================================================================
// These timeouts prevent indefinite hangs during network partitions, high
// concurrency, or cluster topology changes. All critical async operations
// must have explicit timeouts.

/// Timeout for ReadIndex linearizability check (5 seconds).
///
/// Tiger Style: Explicit timeout prevents indefinite hangs when leader is unavailable.
/// Applied to all ReadIndex `await_ready()` calls to ensure bounded wait times.
///
/// Used in:
/// - `node.rs`: KeyValueStore::read(), KeyValueStore::scan(), SqlQueryExecutor::execute_sql()
pub const READ_INDEX_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout for cluster membership operations (30 seconds).
///
/// Tiger Style: Explicit timeout prevents hangs during partition events.
/// Applied to init(), add_learner(), and change_membership() operations.
/// Membership operations may require multiple round trips and quorum confirmation.
///
/// Used in:
/// - `node.rs`: ClusterController::init(), add_learner(), change_membership()
pub const MEMBERSHIP_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

/// Timeout for gossip subscription (10 seconds).
///
/// Tiger Style: Explicit timeout prevents indefinite blocking during subscription.
/// If gossip is unavailable, the node should continue without it (non-fatal).
///
/// Used in:
/// - `gossip_discovery.rs`: GossipPeerDiscovery::spawn()
pub const GOSSIP_SUBSCRIBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Timeout for snapshot installation per segment (5000 milliseconds).
///
/// Tiger Style: Explicit timeout prevents hangs during large snapshot transfers.
/// Default OpenRaft value of 200ms is too short for production snapshots.
/// 5 seconds allows for 100MB snapshots at ~20MB/s transfer rate.
///
/// Used in:
/// - `bootstrap.rs`: RaftConfig::install_snapshot_timeout
pub const SNAPSHOT_INSTALL_TIMEOUT_MS: u64 = 5000;

/// Capacity of failure detector update channel.
///
/// Tiger Style: Bounded channel prevents unbounded task spawning.
/// Used to batch failure detector updates from multiple concurrent RPC failures
/// through a single consumer task instead of spawning unbounded tasks.
///
/// Used in:
/// - `network.rs`: IrpcRaftNetworkFactory failure update channel
pub const FAILURE_DETECTOR_CHANNEL_CAPACITY: usize = 100;

// ============================================================================
// CAS Retry Constants
// ============================================================================

/// Maximum CAS retry attempts before failing (100).
///
/// Tiger Style: Bounded retries prevent indefinite spinning under contention.
/// 100 retries with exponential backoff allows for transient contention
/// while preventing infinite loops. Total max wait ~13 seconds.
///
/// Used in:
/// - `coordination/sequence.rs`: SequenceGenerator::reserve()
/// - `coordination/rate_limiter.rs`: try_acquire_n(), reset()
/// - `coordination/queue.rs`: create(), extend_visibility(), update_queue_stats()
pub const MAX_CAS_RETRIES: u32 = 100;

/// Initial backoff delay for CAS retries (1 millisecond).
///
/// Tiger Style: Short initial delay minimizes latency under light contention.
pub const CAS_RETRY_INITIAL_BACKOFF_MS: u64 = 1;

/// Maximum backoff delay for CAS retries (128 milliseconds).
///
/// Tiger Style: Upper bound prevents excessive wait times.
pub const CAS_RETRY_MAX_BACKOFF_MS: u64 = 128;
