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
/// The chunked push protocol splits objects into chunks, but tree objects
/// may arrive in a single batch due to interdependencies. 50K supports
/// repositories with ~12K+ trees (e.g., Aspen's 72-crate workspace).
pub const MAX_IMPORT_BATCH_SIZE: usize = 50_000;

/// Maximum objects to include in a single push/export.
///
/// Tiger Style: Limits packfile size and memory usage.
/// With chunked push, each chunk carries batches of objects. Tree objects
/// may all arrive in one chunk to preserve dependency ordering.
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

/// KV prefix for git object bytes (serialized SignedObject).
///
/// Stores the postcard-serialized `SignedObject<GitObject>` in redb KV
/// so the exporter can read without depending on iroh-blobs FsStore.
///
/// Key format: `{KV_PREFIX_OBJ}{repo_id_hex}:{blake3_hex}`
/// Value: base64-encoded serialized bytes (for objects ≤ CHUNK_THRESHOLD),
///        or `chunks:N` for large objects (chunks stored at `...:{hash}:c:{0..N}`)
pub const KV_PREFIX_OBJ: &str = "forge:obj:";

/// Maximum base64-encoded size to store in a single KV value.
///
/// Objects whose base64 encoding exceeds this are split into chunks.
/// Raft MAX_VALUE_SIZE is 1MB; keep 256KB headroom for key/overhead.
pub const OBJ_CHUNK_THRESHOLD: usize = 768 * 1024;

/// Size of each chunk for large git objects.
pub const OBJ_CHUNK_SIZE: usize = 768 * 1024;

/// Maximum number of chunks for a single object (768KB × 200 ≈ 150MB).
pub const OBJ_MAX_CHUNKS: usize = 200;

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
