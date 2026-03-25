//! Resource bound constants for the execution cache.
//!
//! Tiger Style: fixed limits to prevent unbounded resource allocation.

/// Maximum size of a single output blob (1 GB).
pub const MAX_OUTPUT_BLOB_SIZE: u64 = 1_073_741_824;

/// Maximum number of output files per cache entry.
pub const MAX_OUTPUT_FILES: u32 = 10_000;

/// Maximum number of input files tracked per session.
pub const MAX_INPUT_FILES: u32 = 100_000;

/// Maximum total cache storage per node (10 GB).
pub const MAX_CACHE_STORAGE_BYTES: u64 = 10_737_418_240;

/// Default TTL for cache entries (24 hours in milliseconds).
pub const DEFAULT_TTL_MS: u64 = 86_400_000;

/// Maximum concurrent tracking sessions.
pub const MAX_CONCURRENT_SESSIONS: u32 = 10_000;

/// Session timeout for stale cleanup (5 minutes in seconds).
pub const SESSION_TIMEOUT_SECS: u64 = 300;

/// Cache lookup timeout (100 milliseconds).
pub const CACHE_LOOKUP_TIMEOUT_MS: u64 = 100;

/// KV key prefix for execution cache entries.
pub const EXEC_CACHE_KEY_PREFIX: &str = "_exec_cache:";

/// Default curated environment variables included in cache key.
pub const DEFAULT_ENV_VARS: &[&str] = &[
    "PATH",
    "HOME",
    "CC",
    "CXX",
    "CFLAGS",
    "CXXFLAGS",
    "RUSTFLAGS",
    "NIX_STORE",
];

/// Environment variable that specifies additional vars to include in cache key.
pub const EXTRA_ENV_VAR: &str = "ASPEN_EXEC_CACHE_ENV";

/// Environment variable that enables cached execution.
pub const ENABLE_ENV_VAR: &str = "ASPEN_CACHED_EXEC";
