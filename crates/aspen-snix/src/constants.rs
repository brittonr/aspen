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
