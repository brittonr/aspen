//! Constants for the Git bridge module.
//!
//! Tiger Style: All constants are explicitly typed with fixed limits
//! to prevent unbounded resource usage.

use std::time::Duration;

// ============================================================================
// Concurrency Limits
// ============================================================================

/// Maximum concurrent object imports during batch processing.
///
/// Tiger Style: Limits parallelism to prevent resource exhaustion.
/// Tuned for typical server workloads: high enough for throughput,
/// low enough to avoid overwhelming Raft consensus.
pub const MAX_IMPORT_CONCURRENCY: usize = 32;

// ============================================================================
// Hash Mapping Limits
// ============================================================================

/// Maximum number of entries in the in-memory hash mapping LRU cache.
///
/// Tiger Style: Bounded cache prevents memory exhaustion.
/// At ~64 bytes per entry (32 BLAKE3 + 20 SHA1 + overhead), 10K entries ≈ 640KB.
pub const MAX_HASH_CACHE_SIZE: usize = 10_000;

/// Maximum mappings to store in a single batch write.
///
/// Tiger Style: Limits KV write size for Raft log entries.
pub const MAX_HASH_MAPPING_BATCH_SIZE: usize = 1_000;

// ============================================================================
// Import/Export Limits
// ============================================================================

/// Maximum objects to process in a single import batch.
///
/// Tiger Style: Prevents memory exhaustion during large imports.
/// Note: Increased from 10K to 50K for testing self-hosting workflow.
/// TODO: Implement chunked import to restore lower limit for security.
pub const MAX_IMPORT_BATCH_SIZE: usize = 50_000;

/// Maximum objects to include in a single push/export.
///
/// Tiger Style: Limits packfile size and memory usage.
/// Note: Increased from 10K to 50K for testing self-hosting workflow.
pub const MAX_PUSH_OBJECTS: usize = 50_000;

/// Maximum depth for commit DAG traversal during export.
///
/// Tiger Style: Prevents runaway traversal on pathological histories.
pub const MAX_DAG_TRAVERSAL_DEPTH: usize = 100_000;

/// Maximum objects to collect before forcing incremental processing.
///
/// Tiger Style: Provides backpressure during large operations.
pub const MAX_PENDING_OBJECTS: usize = 50_000;

// ============================================================================
// Network Timeouts
// ============================================================================

/// Timeout for fetching objects from a git remote.
pub const GIT_FETCH_TIMEOUT: Duration = Duration::from_secs(300);

/// Timeout for pushing objects to a git remote.
pub const GIT_PUSH_TIMEOUT: Duration = Duration::from_secs(300);

/// Timeout for listing refs from a git remote.
pub const GIT_LIST_TIMEOUT: Duration = Duration::from_secs(60);

/// Timeout for authentication handshake.
pub const GIT_AUTH_TIMEOUT: Duration = Duration::from_secs(30);

// ============================================================================
// KV Key Prefixes
// ============================================================================

/// KV prefix for BLAKE3 -> SHA-1 hash mappings.
///
/// Key format: `{KV_PREFIX_B3_TO_SHA1}{repo_id_hex}:{blake3_hex}`
/// Value: SHA-1 hex string (40 chars)
pub const KV_PREFIX_B3_TO_SHA1: &str = "forge:hashmap:b3:";

/// KV prefix for SHA-1 -> BLAKE3 hash mappings.
///
/// Key format: `{KV_PREFIX_SHA1_TO_B3}{repo_id_hex}:{sha1_hex}`
/// Value: BLAKE3 hex string (64 chars)
pub const KV_PREFIX_SHA1_TO_B3: &str = "forge:hashmap:sha1:";

/// KV prefix for stored credentials (encrypted).
///
/// Key format: `{KV_PREFIX_CREDENTIALS}{repo_id_hex}:{url_hash}`
/// Value: Encrypted CredentialEntry (postcard serialized)
pub const KV_PREFIX_CREDENTIALS: &str = "forge:credentials:";

/// KV prefix for remote configuration.
///
/// Key format: `{KV_PREFIX_REMOTES}{repo_id_hex}:{remote_name}`
/// Value: RemoteConfig (postcard serialized)
pub const KV_PREFIX_REMOTES: &str = "forge:remotes:";

// ============================================================================
// Protocol Constants
// ============================================================================

/// ALPN protocol identifier for git bridge operations.
pub const GIT_BRIDGE_ALPN: &[u8] = b"/aspen/git-bridge/1";

/// Maximum size of a git remote helper command line.
///
/// Tiger Style: Prevents buffer overflow from malicious input.
pub const MAX_HELPER_LINE_SIZE: usize = 64 * 1024; // 64 KB

/// Maximum number of refs to list in a single response.
///
/// Tiger Style: Limits memory usage during ref listing.
pub const MAX_LISTED_REFS: usize = 10_000;

// ============================================================================
// Compile-Time Constant Assertions
// ============================================================================

// Concurrency limits must be positive
const _: () = assert!(MAX_IMPORT_CONCURRENCY > 0);
const _: () = assert!(MAX_IMPORT_CONCURRENCY <= 256); // sanity check

// Hash cache limits must be positive
const _: () = assert!(MAX_HASH_CACHE_SIZE > 0);
const _: () = assert!(MAX_HASH_MAPPING_BATCH_SIZE > 0);

// Import/export limits must be positive
const _: () = assert!(MAX_IMPORT_BATCH_SIZE > 0);
const _: () = assert!(MAX_PUSH_OBJECTS > 0);
const _: () = assert!(MAX_DAG_TRAVERSAL_DEPTH > 0);
const _: () = assert!(MAX_PENDING_OBJECTS > 0);

// Protocol limits must be positive
const _: () = assert!(MAX_HELPER_LINE_SIZE > 0);
const _: () = assert!(MAX_LISTED_REFS > 0);
