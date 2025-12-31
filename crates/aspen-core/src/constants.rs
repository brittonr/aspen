//! Public API constants for Aspen operations.
//!
//! This module contains constants that are part of the public API and are used
//! by external consumers of Aspen. Internal Raft-specific constants remain in
//! the main aspen crate's `src/raft/constants.rs`.
//!
//! Tiger Style: Constants are fixed and immutable, enforced at compile time.
//! Each constant has explicit bounds to prevent unbounded resource allocation.

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

/// Maximum number of keys that can be returned in a single scan.
///
/// Tiger Style: Fixed limit prevents unbounded memory allocation.
pub const MAX_SCAN_RESULTS: u32 = 10_000;

/// Default number of keys returned in a scan if limit is not specified.
pub const DEFAULT_SCAN_LIMIT: u32 = 1_000;

// ============================================================================
// SQL Query Constants (requires 'sql' feature)
// ============================================================================

/// Maximum SQL query string length (64 KB).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from oversized queries.
/// Most practical queries are < 10 KB; 64 KB allows for complex CTEs.
#[cfg(feature = "sql")]
pub const MAX_SQL_QUERY_SIZE: u32 = 64 * 1024;

/// Maximum number of query parameters (100).
///
/// Tiger Style: Bounded parameter count prevents pathological cases.
/// Most queries use < 10 parameters; 100 allows for bulk IN clauses.
#[cfg(feature = "sql")]
pub const MAX_SQL_PARAMS: u32 = 100;

/// Maximum rows returned from SQL query (10,000).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from large result sets.
/// Clients can use pagination for larger result sets.
#[cfg(feature = "sql")]
pub const MAX_SQL_RESULT_ROWS: u32 = 10_000;

/// Default rows returned from SQL query (1,000).
///
/// Tiger Style: Reasonable default that balances utility against resource use.
/// Applied when client doesn't specify a limit.
#[cfg(feature = "sql")]
pub const DEFAULT_SQL_RESULT_ROWS: u32 = 1_000;

/// Maximum SQL query timeout in milliseconds (30 seconds).
///
/// Tiger Style: Upper bound on query execution time.
/// Prevents clients from requesting indefinite timeouts.
#[cfg(feature = "sql")]
pub const MAX_SQL_TIMEOUT_MS: u32 = 30_000;

/// Default SQL query timeout in milliseconds (5 seconds).
///
/// Tiger Style: Explicit timeout prevents runaway queries.
/// 5 seconds is sufficient for most indexed queries.
#[cfg(feature = "sql")]
pub const DEFAULT_SQL_TIMEOUT_MS: u32 = 5_000;

// ============================================================================
// CAS Retry Constants (used by coordination primitives)
// ============================================================================

/// Maximum CAS retry attempts before failing (100).
///
/// Tiger Style: Bounded retries prevent indefinite spinning under contention.
/// 100 retries with exponential backoff allows for transient contention
/// while preventing infinite loops. Total max wait ~13 seconds.
pub const MAX_CAS_RETRIES: u32 = 100;

/// Initial backoff delay for CAS retries (1 millisecond).
///
/// Tiger Style: Short initial delay minimizes latency under light contention.
pub const CAS_RETRY_INITIAL_BACKOFF_MS: u64 = 1;

/// Maximum backoff delay for CAS retries (128 milliseconds).
///
/// Tiger Style: Upper bound prevents excessive wait times.
pub const CAS_RETRY_MAX_BACKOFF_MS: u64 = 128;

// ============================================================================
// Distributed Queue Constants
// ============================================================================

/// Maximum queue item payload size (1 MB, same as MAX_VALUE_SIZE).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from oversized payloads.
pub const MAX_QUEUE_ITEM_SIZE: u32 = 1024 * 1024;

/// Maximum items in a batch enqueue/dequeue operation (100).
///
/// Tiger Style: Bounded batch size prevents pathological memory use.
pub const MAX_QUEUE_BATCH_SIZE: u32 = 100;

/// Maximum visibility timeout (1 hour = 3,600,000 ms).
///
/// Tiger Style: Upper bound prevents indefinite item locking.
pub const MAX_QUEUE_VISIBILITY_TIMEOUT_MS: u64 = 3_600_000;

/// Default visibility timeout (30 seconds = 30,000 ms).
///
/// Tiger Style: Reasonable default for most processing tasks.
pub const DEFAULT_QUEUE_VISIBILITY_TIMEOUT_MS: u64 = 30_000;

/// Maximum queue item TTL (7 days = 604,800,000 ms).
///
/// Tiger Style: Upper bound on item lifetime prevents indefinite storage.
pub const MAX_QUEUE_ITEM_TTL_MS: u64 = 7 * 24 * 60 * 60 * 1000;

/// Default deduplication window TTL (5 minutes = 300,000 ms).
///
/// Tiger Style: Reasonable window for catching duplicate submissions.
pub const DEFAULT_QUEUE_DEDUP_TTL_MS: u64 = 300_000;

/// Maximum items to scan during cleanup operations (1000).
///
/// Tiger Style: Bounded cleanup prevents pathological scan times.
pub const MAX_QUEUE_CLEANUP_BATCH: u32 = 1000;

/// Default polling interval for blocking dequeue (100 ms).
///
/// Tiger Style: Bounded polling frequency prevents CPU spin.
pub const DEFAULT_QUEUE_POLL_INTERVAL_MS: u64 = 100;

/// Maximum polling interval for blocking dequeue (1 second).
///
/// Tiger Style: Upper bound on backoff prevents long waits.
pub const MAX_QUEUE_POLL_INTERVAL_MS: u64 = 1000;

// ============================================================================
// Service Registry Constants
// ============================================================================

/// Maximum instances to return in a single discovery (1,000).
///
/// Tiger Style: Bounded result set prevents memory exhaustion.
pub const MAX_SERVICE_DISCOVERY_RESULTS: u32 = 1_000;

/// Default service instance TTL (30 seconds = 30,000 ms).
///
/// Tiger Style: Reasonable default for service health.
pub const DEFAULT_SERVICE_TTL_MS: u64 = 30_000;

/// Maximum service instance TTL (24 hours = 86,400,000 ms).
///
/// Tiger Style: Upper bound prevents indefinite registration.
pub const MAX_SERVICE_TTL_MS: u64 = 24 * 60 * 60 * 1000;

/// Cleanup batch size for expired instance removal (100).
///
/// Tiger Style: Bounded cleanup prevents pathological scan times.
pub const SERVICE_CLEANUP_BATCH: u32 = 100;
