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
