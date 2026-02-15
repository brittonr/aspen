//! Constants for distributed coordination primitives.
//!
//! This module contains constants for queues, service registries, rate limiters,
//! and other coordination primitives.
//!
//! Tiger Style: Constants are fixed and immutable, enforced at compile time.

// ============================================================================
// CAS Retry Constants (used by coordination primitives)
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

/// Initial backoff delay for CAS retries in milliseconds (1 millisecond).
///
/// Tiger Style: Short initial delay minimizes latency under light contention.
pub const CAS_RETRY_INITIAL_BACKOFF_MS: u64 = 1;

/// Maximum backoff delay for CAS retries in milliseconds (128 milliseconds).
///
/// Tiger Style: Upper bound prevents excessive wait times.
pub const CAS_RETRY_MAX_BACKOFF_MS: u64 = 128;

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

/// Maximum visibility timeout in milliseconds (1 hour = 3,600,000 ms).
///
/// Tiger Style: Upper bound prevents indefinite item locking.
/// Most processing should complete within 1 hour.
///
/// Used in:
/// - `coordination/queue.rs`: Visibility timeout validation
pub const MAX_QUEUE_VISIBILITY_TIMEOUT_MS: u64 = 3_600_000;

/// Default visibility timeout in milliseconds (30 seconds = 30,000 ms).
///
/// Tiger Style: Reasonable default for most processing tasks.
/// Short enough to detect failures quickly, long enough for typical work.
///
/// Used in:
/// - `coordination/queue.rs`: Default config value
pub const DEFAULT_QUEUE_VISIBILITY_TIMEOUT_MS: u64 = 30_000;

/// Maximum queue item TTL in milliseconds (7 days = 604,800,000 ms).
///
/// Tiger Style: Upper bound on item lifetime prevents indefinite storage.
/// Items older than 7 days are likely stale.
///
/// Used in:
/// - `coordination/queue.rs`: TTL validation
pub const MAX_QUEUE_ITEM_TTL_MS: u64 = 7 * 24 * 60 * 60 * 1000;

/// Default deduplication window TTL in milliseconds (5 minutes = 300,000 ms).
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

/// Default polling interval for blocking dequeue in milliseconds (100 ms).
///
/// Tiger Style: Bounded polling frequency prevents CPU spin.
/// Short enough for responsive dequeue, long enough to avoid overhead.
///
/// Used in:
/// - `coordination/queue.rs`: dequeue_wait polling
pub const DEFAULT_QUEUE_POLL_INTERVAL_MS: u64 = 100;

/// Maximum polling interval for blocking dequeue in milliseconds (1 second).
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

/// Default service instance TTL in milliseconds (30 seconds = 30,000 ms).
///
/// Tiger Style: Reasonable default for service health.
/// Short enough to detect failures quickly, long enough for heartbeats.
///
/// Used in:
/// - `coordination/registry.rs`: Default registration TTL
pub const DEFAULT_SERVICE_TTL_MS: u64 = 30_000;

/// Maximum service instance TTL in milliseconds (24 hours = 86,400,000 ms).
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

/// Token clock skew tolerance in seconds (60 seconds).
///
/// Tiger Style: Fixed tolerance for clock drift between nodes.
pub const TOKEN_CLOCK_SKEW_SECS: u64 = 60;

// ============================================================================
// Dependency Tracker Constants
// ============================================================================

/// Maximum items in dependency tracker worklist queue (10,000).
///
/// Tiger Style: Bounded worklist prevents unbounded memory growth
/// during dependency graph traversal and failure cascade processing.
///
/// Used in:
/// - `aspen-jobs/dependency_tracker.rs`: Worklist and topological sort queues
pub const MAX_DEPENDENCY_QUEUE_SIZE: u32 = 10_000;

/// Maximum changes in a Pijul topological sort operation (10,000).
///
/// Tiger Style: Bounded to prevent memory exhaustion during dependency
/// ordering for large changelogs.
///
/// Used in:
/// - `aspen-pijul/store/helpers.rs`: order_changes_by_dependencies()
pub const MAX_PIJUL_CHANGES_PER_SORT: u32 = 10_000;

// ============================================================================
// Workflow State Bounds
// ============================================================================

/// Maximum state transitions in workflow history (1000).
///
/// Tiger Style: Bounded history prevents unbounded memory growth.
/// Older entries are pruned when limit is reached.
///
/// Used in:
/// - `aspen-jobs/workflow.rs`: WorkflowState history management
pub const MAX_WORKFLOW_HISTORY_SIZE: u32 = 1000;

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
// RWLock Constants
// ============================================================================

/// Maximum concurrent readers for a distributed RWLock (128).
///
/// Tiger Style: Bounded to prevent memory exhaustion from reader accumulation.
/// 128 readers is generous for most use cases while preventing abuse.
///
/// Used in:
/// - `coordination/rwlock.rs`: Reader acquisition validation
pub const MAX_RWLOCK_READERS: u32 = 128;

/// Maximum pending writers for a distributed RWLock (64).
///
/// Tiger Style: Bounded to prevent unbounded writer queue growth.
/// Writers waiting for lock should be bounded to prevent memory exhaustion.
///
/// Used in:
/// - `coordination/rwlock.rs`: Writer queue validation
pub const MAX_RWLOCK_PENDING_WRITERS: u32 = 64;

// ============================================================================
// Semaphore Constants
// ============================================================================

/// Maximum concurrent holders for a distributed semaphore (256).
///
/// Tiger Style: Bounded to prevent memory exhaustion from holder accumulation.
/// 256 holders is sufficient for most use cases while preventing abuse.
///
/// Used in:
/// - `coordination/semaphore.rs`: Holder acquisition validation
pub const MAX_SEMAPHORE_HOLDERS: u32 = 256;

// ============================================================================
// Lock Constants
// ============================================================================

/// Maximum lock wait timeout in milliseconds (5 minutes = 300,000 ms).
///
/// Tiger Style: Upper bound prevents indefinite lock waiting.
/// Clients should use shorter timeouts for responsiveness.
pub const MAX_LOCK_WAIT_TIMEOUT_MS: u64 = 300_000;

/// Default lock wait timeout in milliseconds (30 seconds).
///
/// Tiger Style: Reasonable default for most lock acquisition scenarios.
pub const DEFAULT_LOCK_WAIT_TIMEOUT_MS: u64 = 30_000;

/// Maximum lock TTL in milliseconds (1 hour = 3,600,000 ms).
///
/// Tiger Style: Upper bound prevents indefinite lock holding.
/// Long-running processes should periodically extend TTL.
pub const MAX_LOCK_TTL_MS: u64 = 3_600_000;

/// Default lock TTL in milliseconds (60 seconds).
///
/// Tiger Style: Reasonable default for most critical sections.
pub const DEFAULT_LOCK_TTL_MS: u64 = 60_000;

// ============================================================================
// Bridge Event Dispatch Constants
// ============================================================================

/// Maximum in-flight TTL expiration event dispatch tasks (100).
///
/// Tiger Style: Bounded to prevent unbounded task spawning when many keys
/// expire simultaneously. When at capacity, completed tasks are drained
/// before spawning new ones.
///
/// Used in:
/// - `aspen-cluster-bridges/ttl_events_bridge.rs`: TTL event dispatch
pub const MAX_TTL_EVENT_DISPATCHES: usize = 100;

/// Maximum in-flight docs event dispatch tasks (100).
///
/// Tiger Style: Bounded to prevent unbounded task spawning under high
/// docs sync throughput. When at capacity, completed tasks are drained
/// before spawning new ones.
///
/// Used in:
/// - `aspen-cluster-bridges/docs_bridge.rs`: Docs event dispatch
pub const MAX_DOCS_EVENT_DISPATCHES: usize = 100;
