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
