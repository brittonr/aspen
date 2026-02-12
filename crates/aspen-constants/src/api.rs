//! Public API constants for Aspen operations.
//!
//! These constants define the bounds for the public API and are used by external
//! consumers of Aspen. They include key-value size limits, SQL query limits,
//! and scan result bounds.
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
// VM Execution Constants (for job isolation)
// ============================================================================

/// Maximum size for a built binary (50MB).
///
/// Tiger Style: Fixed limit prevents DoS via huge build artifacts.
/// Most binaries are < 10MB; 50MB allows for statically linked binaries.
pub const MAX_BINARY_SIZE: usize = 50 * 1024 * 1024;

/// Maximum build time for Nix derivations (5 minutes).
///
/// Tiger Style: Upper bound on build time prevents indefinite compilation.
pub const MAX_BUILD_TIME_MS: u64 = 5 * 60 * 1000;

/// Maximum concurrent VMs per worker (100).
///
/// Tiger Style: Bounded VM count prevents resource exhaustion.
pub const MAX_CONCURRENT_VMS: usize = 100;

/// VM startup timeout (10ms).
///
/// Tiger Style: Fast fail if VM doesn't start quickly.
/// Hyperlight targets 1-2ms startup, 10ms allows for variance.
pub const VM_STARTUP_TIMEOUT_MS: u64 = 10;

/// Default VM execution timeout (5 seconds).
///
/// Tiger Style: Explicit timeout prevents runaway execution.
pub const DEFAULT_VM_TIMEOUT_MS: u64 = 5_000;

/// Maximum VM execution timeout (60 seconds).
///
/// Tiger Style: Upper bound on execution time.
pub const MAX_VM_TIMEOUT_MS: u64 = 60_000;
