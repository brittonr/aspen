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

// ============================================================================
// Observability Constants
// ============================================================================

/// Maximum spans in a single trace ingest batch (100).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from oversized batches.
pub const MAX_TRACE_BATCH_SIZE: u32 = 100;

/// Maximum attributes per span (32).
///
/// Tiger Style: Bounded attribute count prevents pathological cases.
pub const MAX_SPAN_ATTRIBUTES: u32 = 32;

/// Maximum events per span (16).
///
/// Tiger Style: Bounded event count prevents unbounded memory allocation.
pub const MAX_SPAN_EVENTS: u32 = 16;

/// Default limit for trace list/search query results (100).
///
/// Tiger Style: Explicit default prevents unbounded queries.
pub const DEFAULT_TRACE_QUERY_LIMIT: u32 = 100;

/// Maximum trace list/search query results (1,000).
///
/// Tiger Style: Fixed upper bound on trace query result sets.
pub const MAX_TRACE_QUERY_RESULTS: u32 = 1_000;

// ============================================================================
// Index Constants
// ============================================================================

/// Maximum length of an index name in bytes (128).
///
/// Tiger Style: Fixed limit prevents oversized index names.
pub const MAX_INDEX_NAME_SIZE: u32 = 128;

/// Maximum number of custom indexes (64).
///
/// Tiger Style: Bounded index count prevents excessive storage overhead.
pub const MAX_CUSTOM_INDEXES: u32 = 64;

// ============================================================================
// Metrics Constants
// ============================================================================

/// Maximum data points in a single metric ingest batch (200).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from oversized batches.
pub const MAX_METRIC_BATCH_SIZE: u32 = 200;

/// Maximum labels per metric data point (16).
///
/// Tiger Style: Bounded label count prevents pathological cases.
pub const MAX_METRIC_LABELS: u32 = 16;

/// Maximum length of a metric name in bytes (128).
///
/// Tiger Style: Fixed limit prevents oversized metric names.
pub const MAX_METRIC_NAME_SIZE: u32 = 128;

/// Maximum length of a label key or value in bytes (64).
///
/// Tiger Style: Fixed limit prevents oversized label strings.
pub const MAX_METRIC_LABEL_SIZE: u32 = 64;

/// Default metric data point TTL in seconds (24 hours).
///
/// Tiger Style: Automatic expiration prevents unbounded storage growth.
pub const METRIC_DEFAULT_TTL_SECONDS: u32 = 86_400;

/// Maximum metric data point TTL in seconds (7 days).
///
/// Tiger Style: Upper bound on retention prevents indefinite storage.
pub const METRIC_MAX_TTL_SECONDS: u32 = 604_800;

/// Default limit for metric query results (500).
///
/// Tiger Style: Explicit default prevents unbounded queries.
pub const DEFAULT_METRIC_QUERY_LIMIT: u32 = 500;

/// Maximum metric query results (5,000).
///
/// Tiger Style: Fixed upper bound on metric query result sets.
pub const MAX_METRIC_QUERY_RESULTS: u32 = 5_000;

/// Maximum distinct metric names returned by MetricList (1,000).
///
/// Tiger Style: Bounded list prevents unbounded memory allocation.
pub const MAX_METRIC_LIST_RESULTS: u32 = 1_000;

// ============================================================================
// Alert Constants
// ============================================================================

/// Maximum alert rules per cluster (256).
///
/// Tiger Style: Fixed limit prevents unbounded rule storage.
pub const MAX_ALERT_RULES: u32 = 256;

/// Maximum alert rule name length in bytes (128).
///
/// Tiger Style: Fixed limit prevents oversized rule names.
pub const MAX_ALERT_RULE_NAME_SIZE: u32 = 128;

/// Maximum alert history entries retained per rule (100).
///
/// Tiger Style: Bounded history prevents unbounded storage growth.
pub const MAX_ALERT_HISTORY_PER_RULE: u32 = 100;

/// Alert history TTL in seconds (7 days).
///
/// Tiger Style: Automatic expiration prevents indefinite retention.
pub const ALERT_HISTORY_TTL_SECONDS: u32 = 604_800;

/// Maximum notification targets per alert rule (8).
///
/// Tiger Style: Bounded notification fanout prevents amplification.
pub const MAX_ALERT_NOTIFICATION_TARGETS: u32 = 8;

/// Default periodic alert evaluation interval in seconds (60).
///
/// Tiger Style: Fixed interval prevents both too-frequent evaluations
/// (wasting Raft bandwidth) and too-infrequent checks (missing alerts).
/// 60 seconds balances responsiveness with resource efficiency.
pub const ALERT_EVALUATION_INTERVAL_SECONDS: u64 = 60;

/// Minimum periodic alert evaluation interval in seconds (10).
///
/// Tiger Style: Lower bound prevents runaway evaluation loops.
pub const ALERT_EVALUATION_MIN_INTERVAL_SECONDS: u64 = 10;

/// Maximum periodic alert evaluation interval in seconds (3600 = 1 hour).
///
/// Tiger Style: Upper bound ensures alerts are eventually evaluated.
pub const ALERT_EVALUATION_MAX_INTERVAL_SECONDS: u64 = 3_600;

// ============================================================================
// Deployment Constants
// ============================================================================

/// Graceful drain timeout in seconds before node upgrade (30s).
///
/// Tiger Style: Bounded drain prevents indefinite wait for in-flight operations.
/// After this timeout, remaining operations are cancelled and upgrade proceeds.
pub const DRAIN_TIMEOUT_SECS: u64 = 30;

/// Health check timeout after node upgrade in seconds (120s).
///
/// Tiger Style: Bounded health wait prevents deployment from stalling indefinitely.
/// Node must pass health checks (Raft membership, log gap, GetHealth) within this window.
pub const DEPLOY_HEALTH_TIMEOUT_SECS: u64 = 120;

/// Maximum deployment history entries (50).
///
/// Tiger Style: Bounded history prevents unbounded KV growth from past deployments.
/// Oldest entries are pruned when this limit is exceeded.
pub const MAX_DEPLOY_HISTORY: u32 = 50;

/// Deployment status poll interval in seconds (5s).
///
/// Tiger Style: Fixed interval balances responsiveness with Raft read overhead.
/// Used by CLI `cluster deploy` to poll deployment progress.
pub const DEPLOY_STATUS_POLL_INTERVAL_SECS: u64 = 5;

/// Raft log gap threshold for post-upgrade health (100 entries).
///
/// Tiger Style: Bounded gap ensures upgraded node has caught up with cluster.
/// Node is not considered healthy until its log gap is below this threshold.
pub const DEPLOY_LOG_GAP_THRESHOLD: u64 = 100;
