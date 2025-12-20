//! FUSE filesystem implementation backed by Aspen KV store.
//!
//! Maps filesystem paths to KV keys:
//! - `/myapp/config/db` -> KV key `myapp/config/db`
//! - Directories are virtual (derived from key prefixes)
//! - File contents are the KV values

use std::collections::BTreeSet;
use std::ffi::CStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fuse_backend_rs::abi::fuse_abi::{CreateIn, stat64};
use fuse_backend_rs::api::filesystem::{
    Context, DirEntry, Entry, FileSystem, FsOptions, OpenOptions, SetattrValid, ZeroCopyReader,
    ZeroCopyWriter,
};
use tracing::debug;

use crate::client::SharedClient;
use crate::constants::{
    ATTR_TTL, BLOCK_SIZE, DEFAULT_DIR_MODE, DEFAULT_FILE_MODE, ENTRY_TTL, MAX_KEY_SIZE,
    MAX_READDIR_ENTRIES, MAX_VALUE_SIZE, ROOT_INODE,
};
use crate::inode::{EntryType, InodeManager};

/// Aspen FUSE filesystem.
///
/// Provides a filesystem view of an Aspen KV cluster.
/// All operations are backed by the cluster via Iroh Client RPC.
pub struct AspenFs {
    /// Inode manager for path <-> inode mappings.
    inodes: InodeManager,
    /// UID for file ownership.
    uid: u32,
    /// GID for file ownership.
    gid: u32,
    /// Aspen client for KV operations (optional for testing).
    client: Option<SharedClient>,
}

impl AspenFs {
    /// Create a new Aspen filesystem with a connected client.
    pub fn new(uid: u32, gid: u32, client: SharedClient) -> Self {
        Self {
            inodes: InodeManager::new(),
            uid,
            gid,
            client: Some(client),
        }
    }

    /// Create a new Aspen filesystem without a client (for testing).
    #[cfg(test)]
    pub fn new_mock(uid: u32, gid: u32) -> Self {
        Self {
            inodes: InodeManager::new(),
            uid,
            gid,
            client: None,
        }
    }

    /// Convert a filesystem path to a KV key.
    ///
    /// Strips leading slash and normalizes the path.
    fn path_to_key(path: &str) -> String {
        path.trim_start_matches('/').to_string()
    }

    /// Convert a KV key to a filesystem path.
    #[allow(dead_code)]
    fn key_to_path(key: &str) -> String {
        if key.is_empty() {
            String::new()
        } else {
            key.to_string()
        }
    }

    /// Build a stat64 structure for a file or directory.
    fn make_attr(&self, inode: u64, entry_type: EntryType, size: u64) -> stat64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);
        let sec = now.as_secs() as i64;
        let nsec = now.subsec_nanos() as i64;

        let mode = match entry_type {
            EntryType::File => DEFAULT_FILE_MODE,
            EntryType::Directory => DEFAULT_DIR_MODE,
        };

        let nlink = match entry_type {
            EntryType::File => 1,
            EntryType::Directory => 2,
        };

        let blocks = size.div_ceil(u64::from(BLOCK_SIZE));

        // SAFETY: stat64 is a C struct that can be safely zero-initialized.
        // All fields are primitive types (integers) with no invariants.
        let mut attr: stat64 = unsafe { std::mem::zeroed() };
        attr.st_ino = inode;
        attr.st_mode = mode;
        attr.st_nlink = nlink;
        attr.st_uid = self.uid;
        attr.st_gid = self.gid;
        attr.st_size = size as i64;
        attr.st_blocks = blocks as i64;
        attr.st_blksize = i64::from(BLOCK_SIZE);
        attr.st_atime = sec;
        attr.st_atime_nsec = nsec;
        attr.st_mtime = sec;
        attr.st_mtime_nsec = nsec;
        attr.st_ctime = sec;
        attr.st_ctime_nsec = nsec;
        attr
    }

    /// Build an Entry for FUSE lookup responses.
    fn make_entry(&self, inode: u64, entry_type: EntryType, size: u64) -> Entry {
        Entry {
            inode,
            generation: 0,
            attr: self.make_attr(inode, entry_type, size),
            attr_flags: 0,
            attr_timeout: ATTR_TTL,
            entry_timeout: ENTRY_TTL,
        }
    }

    /// Read a key from the KV store.
    fn kv_read(&self, key: &str) -> std::io::Result<Option<Vec<u8>>> {
        let Some(ref client) = self.client else {
            // No client connected (mock mode)
            return Ok(None);
        };

        debug!(key, "reading key from Aspen");

        client
            .read_key(key)
            .map_err(|e| std::io::Error::other(e.to_string()))
    }

    /// Write a key to the KV store.
    fn kv_write(&self, key: &str, value: &[u8]) -> std::io::Result<()> {
        if key.len() > MAX_KEY_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "key too large",
            ));
        }
        if value.len() > MAX_VALUE_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "value too large",
            ));
        }

        let Some(ref client) = self.client else {
            // No client connected (mock mode)
            return Ok(());
        };

        debug!(key, value_len = value.len(), "writing key to Aspen");

        client
            .write_key(key, value)
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        Ok(())
    }

    /// Delete a key from the KV store.
    fn kv_delete(&self, key: &str) -> std::io::Result<()> {
        let Some(ref client) = self.client else {
            // No client connected (mock mode)
            return Ok(());
        };

        debug!(key, "deleting key from Aspen");

        client
            .delete_key(key)
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        Ok(())
    }

    /// Scan keys with a prefix from the KV store.
    fn kv_scan(&self, prefix: &str) -> std::io::Result<Vec<String>> {
        let Some(ref client) = self.client else {
            // No client connected (mock mode)
            return Ok(Vec::new());
        };

        debug!(prefix, "scanning keys from Aspen");

        let entries = client
            .scan_keys(prefix, MAX_READDIR_ENTRIES)
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        Ok(entries.into_iter().map(|(k, _)| k).collect())
    }

    /// Check if a key exists (file) or has children (directory).
    fn exists(&self, path: &str) -> std::io::Result<Option<EntryType>> {
        let key = Self::path_to_key(path);

        // Check if it's a file (exact key match)
        if self.kv_read(&key)?.is_some() {
            return Ok(Some(EntryType::File));
        }

        // Check if it's a directory (has children with this prefix)
        let prefix = if key.is_empty() {
            String::new()
        } else {
            format!("{}/", key)
        };
        let children = self.kv_scan(&prefix)?;
        if !children.is_empty() {
            return Ok(Some(EntryType::Directory));
        }

        Ok(None)
    }
}

impl FileSystem for AspenFs {
    type Inode = u64;
    type Handle = u64;

    fn init(&self, _capable: FsOptions) -> std::io::Result<FsOptions> {
        // Return capabilities we support
        Ok(FsOptions::empty())
    }

    fn destroy(&self) {
        // Cleanup (close connections, etc.)
    }

    fn lookup(&self, _ctx: &Context, parent: u64, name: &CStr) -> std::io::Result<Entry> {
        let name_str = name
            .to_str()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid name"))?;

        // Get parent path
        let parent_entry = self
            .inodes
            .get_path(parent)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "parent not found"))?;

        // Build child path
        let child_path = if parent_entry.path.is_empty() {
            name_str.to_string()
        } else {
            format!("{}/{}", parent_entry.path, name_str)
        };

        // Check if child exists
        let entry_type = self
            .exists(&child_path)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "not found"))?;

        // Get or create inode
        let inode = self.inodes.get_or_create(&child_path, entry_type);

        // Get size (0 for directories)
        let size = match entry_type {
            EntryType::File => {
                let key = Self::path_to_key(&child_path);
                self.kv_read(&key)?.map(|v| v.len() as u64).unwrap_or(0)
            }
            EntryType::Directory => 0,
        };

        Ok(self.make_entry(inode, entry_type, size))
    }

    fn getattr(
        &self,
        _ctx: &Context,
        inode: u64,
        _handle: Option<u64>,
    ) -> std::io::Result<(stat64, Duration)> {
        if inode == ROOT_INODE {
            return Ok((
                self.make_attr(ROOT_INODE, EntryType::Directory, 0),
                ATTR_TTL,
            ));
        }

        let entry = self
            .inodes
            .get_path(inode)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        let size = match entry.entry_type {
            EntryType::File => {
                let key = Self::path_to_key(&entry.path);
                self.kv_read(&key)?.map(|v| v.len() as u64).unwrap_or(0)
            }
            EntryType::Directory => 0,
        };

        Ok((self.make_attr(inode, entry.entry_type, size), ATTR_TTL))
    }

    fn setattr(
        &self,
        _ctx: &Context,
        inode: u64,
        _attr: stat64,
        _handle: Option<u64>,
        _valid: SetattrValid,
    ) -> std::io::Result<(stat64, Duration)> {
        // We don't support changing attributes (mode, owner, etc.)
        // Just return current attributes
        self.getattr(_ctx, inode, None)
    }

    fn opendir(
        &self,
        _ctx: &Context,
        inode: u64,
        _flags: u32,
    ) -> std::io::Result<(Option<u64>, OpenOptions)> {
        // Verify it's a directory
        if inode == ROOT_INODE {
            return Ok((None, OpenOptions::empty()));
        }

        let entry = self
            .inodes
            .get_path(inode)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        if entry.entry_type != EntryType::Directory {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                "not a directory",
            ));
        }

        Ok((None, OpenOptions::empty()))
    }

    fn readdir(
        &self,
        _ctx: &Context,
        inode: u64,
        _handle: u64,
        size: u32,
        offset: u64,
        add_entry: &mut dyn FnMut(DirEntry) -> std::io::Result<usize>,
    ) -> std::io::Result<()> {
        // Get directory path
        let dir_path = if inode == ROOT_INODE {
            String::new()
        } else {
            let entry = self.inodes.get_path(inode).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found")
            })?;
            entry.path.clone()
        };

        // Build prefix for scanning
        let prefix = if dir_path.is_empty() {
            String::new()
        } else {
            format!("{}/", dir_path)
        };

        // Scan for children
        let keys = self.kv_scan(&prefix)?;

        // Extract direct children (not nested)
        let mut children: BTreeSet<String> = BTreeSet::new();
        let prefix_len = prefix.len();

        for key in keys {
            if key.len() > prefix_len {
                let suffix = &key[prefix_len..];
                // Get just the first component
                let child = match suffix.find('/') {
                    Some(pos) => &suffix[..pos],
                    None => suffix,
                };
                children.insert(child.to_string());
            }
        }

        // Build directory entries (skip to offset)
        let mut current_offset = 0u64;
        let mut bytes_written = 0u32;

        // Add . and .. entries
        if offset == 0 {
            let entry = DirEntry {
                ino: inode,
                offset: 1,
                type_: libc::DT_DIR as u32,
                name: b".",
            };
            bytes_written += add_entry(entry)? as u32;
            current_offset = 1;
        }

        if offset <= 1 && bytes_written < size {
            let parent_inode = if inode == ROOT_INODE {
                ROOT_INODE
            } else {
                // Find parent inode
                let parent_path = if dir_path.contains('/') {
                    dir_path.rsplit_once('/').map(|(p, _)| p).unwrap_or("")
                } else {
                    ""
                };
                self.inodes.get_inode(parent_path).unwrap_or(ROOT_INODE)
            };
            let entry = DirEntry {
                ino: parent_inode,
                offset: 2,
                type_: libc::DT_DIR as u32,
                name: b"..",
            };
            bytes_written += add_entry(entry)? as u32;
            current_offset = 2;
        }

        // Add child entries
        let mut entry_count = 0u32;
        for child_name in children {
            current_offset += 1;

            if current_offset <= offset {
                continue;
            }

            if bytes_written >= size || entry_count >= MAX_READDIR_ENTRIES {
                break;
            }

            // Build child path
            let child_path = if dir_path.is_empty() {
                child_name.clone()
            } else {
                format!("{}/{}", dir_path, child_name)
            };

            // Determine type and get inode
            let entry_type = self.exists(&child_path)?.unwrap_or(EntryType::File);
            let child_inode = self.inodes.get_or_create(&child_path, entry_type);

            let dtype = match entry_type {
                EntryType::File => libc::DT_REG as u32,
                EntryType::Directory => libc::DT_DIR as u32,
            };

            let entry = DirEntry {
                ino: child_inode,
                offset: current_offset,
                type_: dtype,
                name: child_name.as_bytes(),
            };
            bytes_written += add_entry(entry)? as u32;
            entry_count += 1;
        }

        Ok(())
    }

    fn open(
        &self,
        _ctx: &Context,
        inode: u64,
        _flags: u32,
        _fuse_flags: u32,
    ) -> std::io::Result<(Option<u64>, OpenOptions, Option<u32>)> {
        // Verify file exists
        if inode == ROOT_INODE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::IsADirectory,
                "is a directory",
            ));
        }

        let entry = self
            .inodes
            .get_path(inode)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        if entry.entry_type == EntryType::Directory {
            return Err(std::io::Error::new(
                std::io::ErrorKind::IsADirectory,
                "is a directory",
            ));
        }

        // No handle needed for stateless operations
        Ok((None, OpenOptions::empty(), None))
    }

    fn read(
        &self,
        _ctx: &Context,
        inode: u64,
        _handle: u64,
        w: &mut dyn ZeroCopyWriter,
        size: u32,
        offset: u64,
        _lock_owner: Option<u64>,
        _flags: u32,
    ) -> std::io::Result<usize> {
        let entry = self
            .inodes
            .get_path(inode)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        let key = Self::path_to_key(&entry.path);
        let value = self.kv_read(&key)?.unwrap_or_default();

        // Calculate read range
        let start = offset as usize;
        if start >= value.len() {
            return Ok(0);
        }
        let end = std::cmp::min(start + size as usize, value.len());
        let data = &value[start..end];

        // Write to buffer
        w.write_all(data)?;
        Ok(data.len())
    }

    fn write(
        &self,
        _ctx: &Context,
        inode: u64,
        _handle: u64,
        r: &mut dyn ZeroCopyReader,
        size: u32,
        offset: u64,
        _lock_owner: Option<u64>,
        _delayed_write: bool,
        _flags: u32,
        _fuse_flags: u32,
    ) -> std::io::Result<usize> {
        let entry = self
            .inodes
            .get_path(inode)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        let key = Self::path_to_key(&entry.path);

        // Read existing value
        let mut value = self.kv_read(&key)?.unwrap_or_default();

        // Extend if needed
        let end = offset as usize + size as usize;
        if value.len() < end {
            value.resize(end, 0);
        }

        // Read new data into buffer
        let mut buf = vec![0u8; size as usize];
        r.read_exact(&mut buf)?;

        // Copy to value
        value[offset as usize..end].copy_from_slice(&buf);

        // Write back
        self.kv_write(&key, &value)?;

        Ok(size as usize)
    }

    fn release(
        &self,
        _ctx: &Context,
        _inode: u64,
        _flags: u32,
        _handle: u64,
        _flush: bool,
        _flock_release: bool,
        _lock_owner: Option<u64>,
    ) -> std::io::Result<()> {
        // Nothing to release for stateless operations
        Ok(())
    }

    fn create(
        &self,
        _ctx: &Context,
        parent: u64,
        name: &CStr,
        _args: CreateIn,
    ) -> std::io::Result<(Entry, Option<u64>, OpenOptions, Option<u32>)> {
        let name_str = name
            .to_str()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid name"))?;

        // Get parent path
        let parent_entry = self
            .inodes
            .get_path(parent)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "parent not found"))?;

        // Build child path
        let child_path = if parent_entry.path.is_empty() {
            name_str.to_string()
        } else {
            format!("{}/{}", parent_entry.path, name_str)
        };

        let key = Self::path_to_key(&child_path);

        // Create empty file
        self.kv_write(&key, &[])?;

        // Allocate inode
        let inode = self.inodes.get_or_create(&child_path, EntryType::File);
        let entry = self.make_entry(inode, EntryType::File, 0);

        Ok((entry, None, OpenOptions::empty(), None))
    }

    fn unlink(&self, _ctx: &Context, parent: u64, name: &CStr) -> std::io::Result<()> {
        let name_str = name
            .to_str()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid name"))?;

        // Get parent path
        let parent_entry = self
            .inodes
            .get_path(parent)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "parent not found"))?;

        // Build child path
        let child_path = if parent_entry.path.is_empty() {
            name_str.to_string()
        } else {
            format!("{}/{}", parent_entry.path, name_str)
        };

        let key = Self::path_to_key(&child_path);

        // Delete from KV
        self.kv_delete(&key)?;

        // Remove from inode cache
        self.inodes.remove_path(&child_path);

        Ok(())
    }

    fn mkdir(
        &self,
        _ctx: &Context,
        parent: u64,
        name: &CStr,
        _mode: u32,
        _umask: u32,
    ) -> std::io::Result<Entry> {
        let name_str = name
            .to_str()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid name"))?;

        // Get parent path
        let parent_entry = self
            .inodes
            .get_path(parent)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "parent not found"))?;

        // Build directory path
        let dir_path = if parent_entry.path.is_empty() {
            name_str.to_string()
        } else {
            format!("{}/{}", parent_entry.path, name_str)
        };

        // Create directory marker (empty key ending with /)
        // Or just allocate the inode - directories are virtual
        let inode = self.inodes.get_or_create(&dir_path, EntryType::Directory);

        Ok(self.make_entry(inode, EntryType::Directory, 0))
    }

    fn rmdir(&self, _ctx: &Context, parent: u64, name: &CStr) -> std::io::Result<()> {
        let name_str = name
            .to_str()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid name"))?;

        // Get parent path
        let parent_entry = self
            .inodes
            .get_path(parent)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "parent not found"))?;

        // Build directory path
        let dir_path = if parent_entry.path.is_empty() {
            name_str.to_string()
        } else {
            format!("{}/{}", parent_entry.path, name_str)
        };

        // Check if directory is empty
        let prefix = format!("{}/", dir_path);
        let children = self.kv_scan(&prefix)?;
        if !children.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::DirectoryNotEmpty,
                "directory not empty",
            ));
        }

        // Remove from inode cache
        self.inodes.remove_path(&dir_path);

        Ok(())
    }

    fn fsync(
        &self,
        _ctx: &Context,
        _inode: u64,
        _datasync: bool,
        _handle: u64,
    ) -> std::io::Result<()> {
        // Writes are already synchronous through Raft consensus
        Ok(())
    }

    fn flush(
        &self,
        _ctx: &Context,
        _inode: u64,
        _handle: u64,
        _lock_owner: u64,
    ) -> std::io::Result<()> {
        // No buffering, nothing to flush
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    /// Create a mock Context for testing.
    fn mock_context() -> Context {
        Context::default()
    }

    // === Path/Key Conversion Tests ===

    #[test]
    fn test_path_to_key_strips_leading_slash() {
        assert_eq!(AspenFs::path_to_key("/myapp/config"), "myapp/config");
        assert_eq!(AspenFs::path_to_key("/simple"), "simple");
        assert_eq!(AspenFs::path_to_key("/a/b/c"), "a/b/c");
    }

    #[test]
    fn test_path_to_key_empty_path() {
        assert_eq!(AspenFs::path_to_key("/"), "");
        assert_eq!(AspenFs::path_to_key(""), "");
    }

    #[test]
    fn test_path_to_key_no_leading_slash() {
        // Paths without leading slash pass through
        assert_eq!(AspenFs::path_to_key("already/no/slash"), "already/no/slash");
    }

    #[test]
    fn test_key_to_path_non_empty() {
        assert_eq!(AspenFs::key_to_path("myapp/config"), "myapp/config");
        assert_eq!(AspenFs::key_to_path("simple"), "simple");
    }

    #[test]
    fn test_key_to_path_empty() {
        assert_eq!(AspenFs::key_to_path(""), "");
    }

    // === Mock Filesystem Tests ===

    #[test]
    fn test_new_mock_creates_filesystem() {
        let fs = AspenFs::new_mock(1000, 1000);
        assert!(fs.client.is_none());
    }

    #[test]
    fn test_mock_kv_read_returns_none() {
        let fs = AspenFs::new_mock(1000, 1000);
        let result = fs.kv_read("any_key").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_mock_kv_write_succeeds() {
        let fs = AspenFs::new_mock(1000, 1000);
        let result = fs.kv_write("key", b"value");
        assert!(result.is_ok());
    }

    #[test]
    fn test_mock_kv_delete_succeeds() {
        let fs = AspenFs::new_mock(1000, 1000);
        let result = fs.kv_delete("key");
        assert!(result.is_ok());
    }

    #[test]
    fn test_mock_kv_scan_returns_empty() {
        let fs = AspenFs::new_mock(1000, 1000);
        let result = fs.kv_scan("prefix/").unwrap();
        assert!(result.is_empty());
    }

    // === Size Limit Tests ===

    #[test]
    fn test_kv_write_rejects_oversized_key() {
        let fs = AspenFs::new_mock(1000, 1000);
        let oversized_key = "k".repeat(MAX_KEY_SIZE + 1);
        let result = fs.kv_write(&oversized_key, b"value");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("key too large"));
    }

    #[test]
    fn test_kv_write_rejects_oversized_value() {
        let fs = AspenFs::new_mock(1000, 1000);
        let oversized_value = vec![0u8; MAX_VALUE_SIZE + 1];
        let result = fs.kv_write("key", &oversized_value);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("value too large"));
    }

    #[test]
    fn test_kv_write_accepts_max_key_size() {
        let fs = AspenFs::new_mock(1000, 1000);
        let max_key = "k".repeat(MAX_KEY_SIZE);
        let result = fs.kv_write(&max_key, b"value");
        assert!(result.is_ok());
    }

    #[test]
    fn test_kv_write_accepts_max_value_size() {
        let fs = AspenFs::new_mock(1000, 1000);
        let max_value = vec![0u8; MAX_VALUE_SIZE];
        let result = fs.kv_write("key", &max_value);
        assert!(result.is_ok());
    }

    // === Attribute Generation Tests ===

    #[test]
    fn test_make_attr_file() {
        let fs = AspenFs::new_mock(1000, 1000);
        let attr = fs.make_attr(42, EntryType::File, 1024);

        assert_eq!(attr.st_ino, 42);
        assert_eq!(attr.st_mode, DEFAULT_FILE_MODE);
        assert_eq!(attr.st_nlink, 1);
        assert_eq!(attr.st_uid, 1000);
        assert_eq!(attr.st_gid, 1000);
        assert_eq!(attr.st_size, 1024);
        assert_eq!(attr.st_blksize, i64::from(BLOCK_SIZE));
    }

    #[test]
    fn test_make_attr_directory() {
        let fs = AspenFs::new_mock(1000, 1000);
        let attr = fs.make_attr(1, EntryType::Directory, 0);

        assert_eq!(attr.st_ino, 1);
        assert_eq!(attr.st_mode, DEFAULT_DIR_MODE);
        assert_eq!(attr.st_nlink, 2); // Directories have nlink=2
        assert_eq!(attr.st_size, 0);
    }

    #[test]
    fn test_make_attr_block_count() {
        let fs = AspenFs::new_mock(1000, 1000);

        // Empty file
        let attr = fs.make_attr(1, EntryType::File, 0);
        assert_eq!(attr.st_blocks, 0);

        // 1 byte file (1 block)
        let attr = fs.make_attr(1, EntryType::File, 1);
        assert_eq!(attr.st_blocks, 1);

        // Exactly 1 block
        let attr = fs.make_attr(1, EntryType::File, u64::from(BLOCK_SIZE));
        assert_eq!(attr.st_blocks, 1);

        // Just over 1 block
        let attr = fs.make_attr(1, EntryType::File, u64::from(BLOCK_SIZE) + 1);
        assert_eq!(attr.st_blocks, 2);
    }

    // === Entry Generation Tests ===

    #[test]
    fn test_make_entry_file() {
        let fs = AspenFs::new_mock(1000, 1000);
        let entry = fs.make_entry(42, EntryType::File, 512);

        assert_eq!(entry.inode, 42);
        assert_eq!(entry.generation, 0);
        assert_eq!(entry.attr.st_ino, 42);
        assert_eq!(entry.attr.st_size, 512);
        assert_eq!(entry.attr_timeout, ATTR_TTL);
        assert_eq!(entry.entry_timeout, ENTRY_TTL);
    }

    #[test]
    fn test_make_entry_directory() {
        let fs = AspenFs::new_mock(1000, 1000);
        let entry = fs.make_entry(1, EntryType::Directory, 0);

        assert_eq!(entry.inode, 1);
        assert_eq!(entry.attr.st_mode, DEFAULT_DIR_MODE);
    }

    // === Existence Checks ===

    #[test]
    fn test_exists_returns_none_in_mock_mode() {
        let fs = AspenFs::new_mock(1000, 1000);
        // In mock mode, no keys exist
        let result = fs.exists("any/path").unwrap();
        assert!(result.is_none());
    }

    // === FileSystem Trait Method Tests ===

    #[test]
    fn test_init_returns_empty_options() {
        let fs = AspenFs::new_mock(1000, 1000);
        let options = fs.init(FsOptions::empty()).unwrap();
        assert!(options.is_empty());
    }

    #[test]
    fn test_getattr_root() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let (attr, ttl) = fs.getattr(&ctx, ROOT_INODE, None).unwrap();

        assert_eq!(attr.st_ino, ROOT_INODE);
        assert_eq!(attr.st_mode, DEFAULT_DIR_MODE);
        assert_eq!(ttl, ATTR_TTL);
    }

    #[test]
    fn test_getattr_unknown_inode() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let result = fs.getattr(&ctx, 99999, None);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn test_opendir_root() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let (handle, options) = fs.opendir(&ctx, ROOT_INODE, 0).unwrap();

        assert!(handle.is_none()); // Stateless
        assert!(options.is_empty());
    }

    #[test]
    fn test_opendir_unknown_inode() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let result = fs.opendir(&ctx, 99999, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_open_root_fails() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        // Root is a directory, open should fail
        let result = fs.open(&ctx, ROOT_INODE, 0, 0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::IsADirectory);
    }

    #[test]
    fn test_mkdir_and_rmdir() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        // Create directory under root
        let name = CString::new("testdir").unwrap();
        let entry = fs.mkdir(&ctx, ROOT_INODE, &name, 0o755, 0o022).unwrap();

        assert_ne!(entry.inode, ROOT_INODE);
        assert_eq!(entry.attr.st_mode, DEFAULT_DIR_MODE);

        // Remove directory
        let result = fs.rmdir(&ctx, ROOT_INODE, &name);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fsync_succeeds() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let result = fs.fsync(&ctx, ROOT_INODE, false, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_flush_succeeds() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let result = fs.flush(&ctx, ROOT_INODE, 0, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_release_succeeds() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let result = fs.release(&ctx, ROOT_INODE, 0, 0, false, false, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_destroy_is_noop() {
        let fs = AspenFs::new_mock(1000, 1000);
        // Should not panic
        fs.destroy();
    }

    // === Inode Manager Integration Tests ===

    #[test]
    fn test_inode_manager_integration() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        // Create a directory
        let name = CString::new("mydir").unwrap();
        let entry = fs.mkdir(&ctx, ROOT_INODE, &name, 0o755, 0o022).unwrap();

        // Inode should be retrievable via getattr
        let (attr, _) = fs.getattr(&ctx, entry.inode, None).unwrap();
        assert_eq!(attr.st_ino, entry.inode);
        assert_eq!(attr.st_mode, DEFAULT_DIR_MODE);
    }

    #[test]
    fn test_lookup_root_child() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        // First create a directory so there's something to look up
        let name = CString::new("subdir").unwrap();
        let _created = fs.mkdir(&ctx, ROOT_INODE, &name, 0o755, 0o022).unwrap();

        // Lookup should fail in mock mode (no actual KV entries)
        // because exists() returns None
        let result = fs.lookup(&ctx, ROOT_INODE, &name);

        // In mock mode, lookup fails because kv_read returns None
        // and directory check (kv_scan) returns empty
        assert!(result.is_err());
    }
}
