//! Tiger Style resource bounds for SNIX storage.
//!
//! All limits are explicit and documented to prevent resource exhaustion.

/// Maximum size of a single blob in bytes (1 GB).
///
/// This matches the maximum NAR size we're willing to handle in memory
/// for streaming operations.
pub const MAX_BLOB_SIZE_BYTES: u64 = 1024 * 1024 * 1024;

/// Maximum number of entries in a single directory.
///
/// Prevents excessive memory usage when loading directories.
pub const MAX_DIRECTORY_ENTRIES: u32 = 100_000;

/// Maximum depth for recursive directory traversal.
///
/// Prevents stack overflow and excessive recursion in deeply nested paths.
pub const MAX_DIRECTORY_DEPTH: u32 = 256;

/// Maximum number of directories to buffer in get_recursive stream.
///
/// Limits memory usage during recursive directory enumeration.
pub const MAX_RECURSIVE_BUFFER: u32 = 10_000;

/// Maximum number of references a store path can have.
///
/// Matches Nix's practical limits for package dependencies.
pub const MAX_PATH_REFERENCES: u32 = 10_000;

/// Maximum length of a deriver path in bytes.
pub const MAX_DERIVER_LENGTH: u32 = 1024;

/// Maximum number of signatures on a path info.
pub const MAX_SIGNATURES: u32 = 100;

/// Chunk size for streaming blob reads (256 KB).
///
/// Balance between memory usage and I/O efficiency.
pub const BLOB_CHUNK_SIZE_BYTES: u32 = 256 * 1024;

/// Timeout for blob operations in milliseconds.
pub const BLOB_TIMEOUT_MS: u64 = 30_000;

/// Timeout for directory operations in milliseconds.
pub const DIRECTORY_TIMEOUT_MS: u64 = 10_000;

/// Timeout for path info operations in milliseconds.
pub const PATHINFO_TIMEOUT_MS: u64 = 10_000;

/// Key prefix for directory entries in KV store.
pub const DIRECTORY_KEY_PREFIX: &str = "snix:dir:";

/// Key prefix for path info entries in KV store.
pub const PATHINFO_KEY_PREFIX: &str = "snix:pathinfo:";

/// Length of Nix store path digest in bytes (20 bytes = 160 bits).
///
/// This is the truncated SHA-256 hash used by Nix for store paths.
pub const STORE_PATH_DIGEST_LENGTH: usize = 20;

/// Length of BLAKE3 digest in bytes (32 bytes = 256 bits).
pub const B3_DIGEST_LENGTH: usize = 32;

// Migration-specific constants

/// Batch size for migration operations.
///
/// Number of entries to process before checkpointing progress.
pub const MIGRATION_BATCH_SIZE: u32 = 50;

/// Timeout for migrating a single entry in seconds.
pub const MIGRATION_ENTRY_TIMEOUT_SECS: u64 = 60;

/// Delay between migration batches in milliseconds.
///
/// Rate limiting to avoid impacting normal operations.
pub const MIGRATION_BATCH_DELAY_MS: u64 = 100;

/// Key prefix for migration progress tracking.
pub const MIGRATION_PROGRESS_KEY_PREFIX: &str = "snix:migration:";

// ============================================================================
// Compile-Time Constant Assertions
// ============================================================================

// Blob limits must be positive
const _: () = assert!(MAX_BLOB_SIZE_BYTES > 0);
const _: () = assert!(BLOB_CHUNK_SIZE_BYTES > 0);
const _: () = assert!((BLOB_CHUNK_SIZE_BYTES as u64) < MAX_BLOB_SIZE_BYTES);

// Directory limits must be positive
const _: () = assert!(MAX_DIRECTORY_ENTRIES > 0);
const _: () = assert!(MAX_DIRECTORY_DEPTH > 0);
const _: () = assert!(MAX_RECURSIVE_BUFFER > 0);

// Path limits must be positive
const _: () = assert!(MAX_PATH_REFERENCES > 0);
const _: () = assert!(MAX_DERIVER_LENGTH > 0);
const _: () = assert!(MAX_SIGNATURES > 0);

// Timeouts must be positive
const _: () = assert!(BLOB_TIMEOUT_MS > 0);
const _: () = assert!(DIRECTORY_TIMEOUT_MS > 0);
const _: () = assert!(PATHINFO_TIMEOUT_MS > 0);

// Digest sizes must match expected values
const _: () = assert!(STORE_PATH_DIGEST_LENGTH == 20); // Nix uses truncated SHA-256
const _: () = assert!(B3_DIGEST_LENGTH == 32); // BLAKE3 is 256 bits

// Migration limits must be positive
const _: () = assert!(MIGRATION_BATCH_SIZE > 0);
const _: () = assert!(MIGRATION_ENTRY_TIMEOUT_SECS > 0);
const _: () = assert!(MIGRATION_BATCH_DELAY_MS > 0);
