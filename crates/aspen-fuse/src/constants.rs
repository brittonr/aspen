//! Tiger Style resource bounds for FUSE filesystem.
//!
//! All limits are explicitly defined to prevent unbounded resource use.

use std::time::Duration;

/// Maximum number of concurrent file handles.
#[allow(dead_code)]
pub const MAX_FILE_HANDLES: u32 = 1000;

/// Maximum number of inodes in the cache.
pub const MAX_INODE_CACHE: usize = 10_000;

/// Maximum entries returned in a single readdir call.
pub const MAX_READDIR_ENTRIES: u32 = 1000;

/// TTL for cached file attributes (1 second).
pub const ATTR_TTL: Duration = Duration::from_secs(1);

/// TTL for cached directory entries (1 second).
pub const ENTRY_TTL: Duration = Duration::from_secs(1);

/// Maximum path depth (number of components).
#[allow(dead_code)]
pub const MAX_PATH_DEPTH: u32 = 32;

/// Maximum key length in bytes.
pub const MAX_KEY_SIZE: usize = 1024;

/// Maximum value size in bytes (1 MB).
pub const MAX_VALUE_SIZE: usize = 1024 * 1024;

// ============================================================================
// File Chunking
// ============================================================================

/// Chunk size for large files (512 KB). Files larger than this are split.
pub const CHUNK_SIZE: usize = 512 * 1024;

/// Maximum file size (4 GB). Theoretical limit with u32 chunk count.
pub const MAX_FILE_SIZE: u64 = 4 * 1024 * 1024 * 1024;

/// Magic bytes for chunk manifest entries (distinguishes from raw file data).
pub const CHUNK_MANIFEST_MAGIC: &[u8] = b"ASPEN_CHUNKED\x00";

/// Suffix for chunk keys.
pub const CHUNK_KEY_SUFFIX: &str = ".chunk.";

/// Connection timeout for Aspen cluster.
pub const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);

/// Read timeout for Aspen operations.
pub const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Write timeout for Aspen operations.
pub const WRITE_TIMEOUT: Duration = Duration::from_secs(30);

/// Root inode number (always 1 per FUSE convention).
pub const ROOT_INODE: u64 = 1;

/// Default file mode for regular files (0644).
pub const DEFAULT_FILE_MODE: u32 = 0o100644;

/// Default directory mode (0755).
pub const DEFAULT_DIR_MODE: u32 = 0o040755;

/// Block size for file operations.
pub const BLOCK_SIZE: u32 = 4096;

/// Default symlink mode (0777 with symlink bit).
pub const DEFAULT_SYMLINK_MODE: u32 = 0o120777;

/// Prefix for symlink target storage in KV.
/// Format: `<key>.symlink` -> target path
pub const SYMLINK_SUFFIX: &str = ".symlink";

/// Prefix for extended attribute storage in KV.
/// Format: `<key>.xattr.<name>` -> attribute value
pub const XATTR_PREFIX: &str = ".xattr.";

/// Maximum extended attribute name length.
pub const MAX_XATTR_NAME_SIZE: usize = 255;

/// Maximum extended attribute value size (64 KB).
pub const MAX_XATTR_VALUE_SIZE: usize = 64 * 1024;

/// Maximum number of extended attributes per file.
pub const MAX_XATTRS_PER_FILE: usize = 100;

// ============================================================================
// Read Cache
// ============================================================================

/// Maximum number of entries in the data read cache.
pub const CACHE_MAX_DATA_ENTRIES: usize = 1000;

/// Maximum total bytes stored in the data read cache (64 MB).
pub const CACHE_MAX_DATA_BYTES: usize = 64 * 1024 * 1024;

/// TTL for cached data entries (5 seconds).
pub const CACHE_DATA_TTL: Duration = Duration::from_secs(5);

/// Maximum number of entries in the metadata cache.
pub const CACHE_MAX_META_ENTRIES: usize = 5000;

/// TTL for cached metadata entries (2 seconds).
pub const CACHE_META_TTL: Duration = Duration::from_secs(2);

/// Maximum number of entries in the scan/readdir cache.
pub const CACHE_MAX_SCAN_ENTRIES: usize = 500;

/// TTL for cached scan results (1 second — directories change more often).
pub const CACHE_SCAN_TTL: Duration = Duration::from_secs(1);

// ============================================================================
// Persistent File Metadata
// ============================================================================

/// Suffix for file metadata storage in KV.
///
/// Format: `<key>.meta` -> serialized FileMetadata (mtime, ctime).
pub const META_SUFFIX: &str = ".meta";

// ============================================================================
// Connection Pool
// ============================================================================

/// Maximum number of connections in the pool.
pub const POOL_MAX_CONNECTIONS: usize = 8;

/// How long to wait for a connection before creating a new one.
pub const POOL_ACQUIRE_TIMEOUT: Duration = Duration::from_millis(500);

// ============================================================================
// Compile-Time Constant Assertions
// ============================================================================

// Inode and file handle limits must be positive
const _: () = assert!(MAX_INODE_CACHE > 0);
const _: () = assert!(MAX_READDIR_ENTRIES > 0);

// Key/value limits must be positive and ordered
const _: () = assert!(MAX_KEY_SIZE > 0);
const _: () = assert!(MAX_VALUE_SIZE > 0);
const _: () = assert!(MAX_KEY_SIZE < MAX_VALUE_SIZE);

// Extended attribute limits must be positive
const _: () = assert!(MAX_XATTR_NAME_SIZE > 0);
const _: () = assert!(MAX_XATTR_VALUE_SIZE > 0);
const _: () = assert!(MAX_XATTRS_PER_FILE > 0);

// Block size must be positive and power of 2
const _: () = assert!(BLOCK_SIZE > 0);
const _: () = assert!(BLOCK_SIZE.count_ones() == 1); // power of 2

// Root inode must be 1 per FUSE convention
const _: () = assert!(ROOT_INODE == 1);

// Timeout ordering: connection < read < write
const _: () = assert!(CONNECTION_TIMEOUT.as_secs() > 0);
const _: () = assert!(READ_TIMEOUT.as_secs() > 0);
const _: () = assert!(WRITE_TIMEOUT.as_secs() > 0);
const _: () = assert!(CONNECTION_TIMEOUT.as_secs() < READ_TIMEOUT.as_secs());
const _: () = assert!(READ_TIMEOUT.as_secs() < WRITE_TIMEOUT.as_secs());

// Xattr limits relationships
const _: () = assert!(MAX_XATTR_NAME_SIZE < MAX_XATTR_VALUE_SIZE);
const _: () = assert!(MAX_XATTR_VALUE_SIZE < MAX_VALUE_SIZE);

// Cache limits
const _: () = assert!(CACHE_MAX_DATA_ENTRIES > 0);
const _: () = assert!(CACHE_MAX_DATA_BYTES > 0);
const _: () = assert!(CACHE_MAX_META_ENTRIES > 0);
const _: () = assert!(CACHE_MAX_SCAN_ENTRIES > 0);

// Connection pool
const _: () = assert!(POOL_MAX_CONNECTIONS > 0);

// Chunking limits
const _: () = assert!(CHUNK_SIZE > 0);
const _: () = assert!(CHUNK_SIZE < MAX_VALUE_SIZE); // Chunks must fit in KV values
const _: () = assert!(MAX_FILE_SIZE > 0);
const _: () = assert!(MAX_FILE_SIZE > CHUNK_SIZE as u64); // Max must be larger than chunk
const _: () = assert!(!CHUNK_MANIFEST_MAGIC.is_empty());
const _: () = assert!(!CHUNK_KEY_SUFFIX.is_empty());

// ============================================================================
// File Locking
// ============================================================================

/// Maximum number of locks per inode.
pub const MAX_LOCKS_PER_INODE: usize = 256;

/// Maximum number of inodes with active locks.
pub const MAX_LOCKED_INODES: usize = 8192;

/// Timeout for blocking lock acquisition (setlkw).
pub const LOCK_WAIT_TIMEOUT: Duration = Duration::from_secs(10);

// File locking limits
const _: () = assert!(MAX_LOCKS_PER_INODE > 0);
const _: () = assert!(MAX_LOCKED_INODES > 0);
const _: () = assert!(LOCK_WAIT_TIMEOUT.as_secs() > 0);

// ============================================================================
// Write-Ahead Buffer
// ============================================================================

/// Maximum bytes buffered per file before auto-flush (256 KB).
pub const WRITEBACK_MAX_FILE_BYTES: usize = 256 * 1024;

/// Maximum total bytes buffered across all files (8 MB).
pub const WRITEBACK_MAX_TOTAL_BYTES: usize = 8 * 1024 * 1024;

/// Maximum number of files with buffered writes.
pub const WRITEBACK_MAX_FILES: usize = 1024;

/// Flush interval for dirty data.
pub const WRITEBACK_FLUSH_INTERVAL: Duration = Duration::from_millis(500);

// Writeback limits
const _: () = assert!(WRITEBACK_MAX_FILE_BYTES > 0);
const _: () = assert!(WRITEBACK_MAX_TOTAL_BYTES > 0);
const _: () = assert!(WRITEBACK_MAX_TOTAL_BYTES >= WRITEBACK_MAX_FILE_BYTES);
const _: () = assert!(WRITEBACK_MAX_FILES > 0);

// ============================================================================
// Prefetch / Readahead
// ============================================================================

/// Bytes to prefetch on file open (128 KB).
pub const PREFETCH_READAHEAD_BYTES: u32 = 128 * 1024;

/// Sequential reads needed before aggressive prefetch kicks in.
pub const PREFETCH_SEQUENTIAL_THRESHOLD: u32 = 3;

/// Maximum prefetch requests queued.
pub const PREFETCH_MAX_PENDING: usize = 64;

/// Maximum files tracked for access patterns.
pub const PREFETCH_MAX_TRACKED_FILES: usize = 4096;

/// How far ahead to prefetch on sequential access (512 KB).
pub const PREFETCH_LOOKAHEAD_BYTES: u32 = 512 * 1024;

/// Access tracker entry TTL.
pub const PREFETCH_TRACKER_TTL: Duration = Duration::from_secs(30);

// ============================================================================
// Content-Hash Cache Validation
// ============================================================================

/// TTL for cache entries revalidated by content-hash check (60 seconds).
///
/// Longer than the default data TTL (5s) because hash validation provides
/// a stronger freshness guarantee than time-based expiry.
pub const CACHE_REVALIDATED_TTL: Duration = Duration::from_secs(60);

/// Timeout for hash-check RPC (100ms). Falls through to full fetch on timeout.
pub const HASH_CHECK_TIMEOUT_MS: u64 = 100;

/// Minimum interval between offline-mode stale-data warnings (60 seconds).
pub const OFFLINE_STALE_WARN_INTERVAL: Duration = Duration::from_secs(60);

// Content-hash validation constants
const _: () = assert!(CACHE_REVALIDATED_TTL.as_secs() > CACHE_DATA_TTL.as_secs());
const _: () = assert!(HASH_CHECK_TIMEOUT_MS > 0);
const _: () = assert!(OFFLINE_STALE_WARN_INTERVAL.as_secs() > 0);

// Prefetch limits
const _: () = assert!(PREFETCH_READAHEAD_BYTES > 0);
const _: () = assert!(PREFETCH_SEQUENTIAL_THRESHOLD > 0);
const _: () = assert!(PREFETCH_MAX_PENDING > 0);
const _: () = assert!(PREFETCH_MAX_TRACKED_FILES > 0);
const _: () = assert!(PREFETCH_LOOKAHEAD_BYTES > 0);
const _: () = assert!(PREFETCH_LOOKAHEAD_BYTES >= PREFETCH_READAHEAD_BYTES);
