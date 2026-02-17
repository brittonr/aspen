//! FUSE filesystem implementation backed by Aspen KV store.
//!
//! Maps filesystem paths to KV keys:
//! - `/myapp/config/db` -> KV key `myapp/config/db`
//! - Directories are virtual (derived from key prefixes)
//! - File contents are the KV values

mod operations;

#[cfg(test)]
mod tests;

use std::path::Component;
use std::path::Path;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use fuse_backend_rs::abi::fuse_abi::stat64;
use fuse_backend_rs::api::filesystem::Context;
use fuse_backend_rs::api::filesystem::Entry;
use tracing::debug;

use crate::client::SharedClient;
use crate::constants::ATTR_TTL;
use crate::constants::BLOCK_SIZE;
use crate::constants::DEFAULT_DIR_MODE;
use crate::constants::DEFAULT_FILE_MODE;
use crate::constants::DEFAULT_SYMLINK_MODE;
use crate::constants::ENTRY_TTL;
use crate::constants::MAX_KEY_SIZE;
use crate::constants::MAX_READDIR_ENTRIES;
use crate::constants::MAX_VALUE_SIZE;
use crate::constants::ROOT_INODE;
use crate::inode::EntryType;
use crate::inode::InodeManager;

// POSIX access mode bits
const R_OK: u32 = 4;
const W_OK: u32 = 2;
const X_OK: u32 = 1;
const F_OK: u32 = 0;

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
    /// Key prefix for namespace isolation (e.g., `ci/workspaces/{vm_id}/`).
    /// Prepended to all KV keys. Empty string means no prefix.
    key_prefix: String,
}

impl AspenFs {
    /// Create a new Aspen filesystem with a connected client.
    pub fn new(uid: u32, gid: u32, client: SharedClient) -> Self {
        Self {
            inodes: InodeManager::new(),
            uid,
            gid,
            client: Some(client),
            key_prefix: String::new(),
        }
    }

    /// Create a new Aspen filesystem with a key prefix for namespace isolation.
    ///
    /// All KV operations will be scoped under the given prefix.
    /// For example, with prefix `ci/workspaces/vm-1/`, a file at `/foo/bar`
    /// maps to KV key `ci/workspaces/vm-1/foo/bar`.
    pub fn with_prefix(uid: u32, gid: u32, client: SharedClient, prefix: String) -> Self {
        Self {
            inodes: InodeManager::new(),
            uid,
            gid,
            client: Some(client),
            key_prefix: prefix,
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
            key_prefix: String::new(),
        }
    }

    /// Convert a filesystem path to a KV key.
    ///
    /// Strips leading slash, normalizes the path, and rejects path traversal attempts.
    ///
    /// # Security
    ///
    /// This function explicitly rejects:
    /// - Parent directory traversal (`..` components)
    /// - Current directory components (`.`) are skipped
    /// - Only `Normal` path components are allowed
    ///
    /// # Errors
    ///
    /// Returns `EINVAL` if the path contains traversal attempts or invalid components.
    fn path_to_key(path: &str) -> std::io::Result<String> {
        let path_obj = Path::new(path);
        let mut normalized = String::new();

        for component in path_obj.components() {
            match component {
                Component::Normal(name) => {
                    let name_str = name.to_str().ok_or_else(|| {
                        std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid UTF-8 in path")
                    })?;
                    if !normalized.is_empty() {
                        normalized.push('/');
                    }
                    normalized.push_str(name_str);
                }
                Component::ParentDir => {
                    // Security: Reject parent directory traversal
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "path traversal with '..' not allowed",
                    ));
                }
                Component::CurDir => {
                    // Skip current directory components (.)
                }
                Component::RootDir | Component::Prefix(_) => {
                    // Skip root and prefix components (leading / or Windows drive letters)
                }
            }
        }

        Ok(normalized)
    }

    /// Convert a KV key to a filesystem path.
    #[allow(dead_code)]
    fn key_to_path(key: &str) -> String {
        if key.is_empty() { String::new() } else { key.to_string() }
    }

    /// Build a stat64 structure for a file or directory.
    fn make_attr(&self, inode: u64, entry_type: EntryType, size: u64) -> stat64 {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        let sec = now.as_secs() as i64;
        let nsec = now.subsec_nanos() as i64;

        let mode = match entry_type {
            EntryType::File => DEFAULT_FILE_MODE,
            EntryType::Directory => DEFAULT_DIR_MODE,
            EntryType::Symlink => DEFAULT_SYMLINK_MODE,
        };

        let nlink = match entry_type {
            EntryType::File | EntryType::Symlink => 1,
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

    /// Prepend the key prefix to a key for namespace isolation.
    fn prefixed_key(&self, key: &str) -> String {
        if self.key_prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}{}", self.key_prefix, key)
        }
    }

    /// Strip the key prefix from a key returned by scan.
    fn strip_prefix<'a>(&self, key: &'a str) -> &'a str {
        if self.key_prefix.is_empty() {
            key
        } else {
            key.strip_prefix(&self.key_prefix).unwrap_or(key)
        }
    }

    /// Read a key from the KV store.
    fn kv_read(&self, key: &str) -> std::io::Result<Option<Vec<u8>>> {
        let Some(ref client) = self.client else {
            // No client connected (mock mode)
            return Ok(None);
        };

        let prefixed = self.prefixed_key(key);
        debug!(key = %prefixed, "reading key from Aspen");

        client.read_key(&prefixed).map_err(|e| std::io::Error::other(e.to_string()))
    }

    /// Write a key to the KV store.
    fn kv_write(&self, key: &str, value: &[u8]) -> std::io::Result<()> {
        let prefixed = self.prefixed_key(key);
        if prefixed.len() > MAX_KEY_SIZE {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "key too large"));
        }
        if value.len() > MAX_VALUE_SIZE {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "value too large"));
        }

        let Some(ref client) = self.client else {
            // No client connected (mock mode)
            return Ok(());
        };

        debug!(key = %prefixed, value_len = value.len(), "writing key to Aspen");

        client.write_key(&prefixed, value).map_err(|e| std::io::Error::other(e.to_string()))?;

        Ok(())
    }

    /// Delete a key from the KV store.
    fn kv_delete(&self, key: &str) -> std::io::Result<()> {
        let Some(ref client) = self.client else {
            // No client connected (mock mode)
            return Ok(());
        };

        let prefixed = self.prefixed_key(key);
        debug!(key = %prefixed, "deleting key from Aspen");

        client.delete_key(&prefixed).map_err(|e| std::io::Error::other(e.to_string()))?;

        Ok(())
    }

    /// Scan keys with a prefix from the KV store.
    fn kv_scan(&self, prefix: &str) -> std::io::Result<Vec<String>> {
        let Some(ref client) = self.client else {
            // No client connected (mock mode)
            return Ok(Vec::new());
        };

        let prefixed = self.prefixed_key(prefix);
        debug!(prefix = %prefixed, "scanning keys from Aspen");

        let entries =
            client.scan_keys(&prefixed, MAX_READDIR_ENTRIES).map_err(|e| std::io::Error::other(e.to_string()))?;

        Ok(entries.into_iter().map(|(k, _)| self.strip_prefix(&k).to_string()).collect())
    }

    /// Check if a key exists (file) or has children (directory).
    fn exists(&self, path: &str) -> std::io::Result<Option<EntryType>> {
        let key = Self::path_to_key(path)?;

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

    /// Check access permissions following POSIX semantics.
    ///
    /// # Arguments
    /// - `ctx`: The FUSE context containing requesting user's uid/gid
    /// - `file_mode`: The file's permission mode (e.g., 0o644)
    /// - `mask`: The requested access mask (R_OK, W_OK, X_OK, or combinations)
    ///
    /// # Permission Model
    ///
    /// This implementation uses a simplified ownership model where all files
    /// are owned by the mounting user (self.uid, self.gid). Permission checks
    /// follow standard Unix semantics:
    ///
    /// - Root (uid=0): Can read/write anything, can execute only if any exec bit is set
    /// - Owner (ctx.uid == self.uid): Uses owner permission bits (mode >> 6) & 7
    /// - Group (ctx.gid == self.gid): Uses group permission bits (mode >> 3) & 7
    /// - Other: Uses other permission bits (mode & 7)
    fn check_access(&self, ctx: &Context, file_mode: u32, mask: u32) -> std::io::Result<()> {
        // F_OK just checks existence, which is handled by the caller
        if mask == F_OK {
            return Ok(());
        }

        // Root can read/write anything
        if ctx.uid == 0 {
            // For X_OK, root can only execute if any execute bit is set
            if mask & X_OK != 0 && file_mode & 0o111 == 0 {
                return Err(std::io::Error::from_raw_os_error(libc::EACCES));
            }
            return Ok(());
        }

        // Determine which permission bits to check based on ownership
        let perm_bits = if ctx.uid == self.uid {
            // Owner permissions (bits 6-8)
            (file_mode >> 6) & 7
        } else if ctx.gid == self.gid {
            // Group permissions (bits 3-5)
            (file_mode >> 3) & 7
        } else {
            // Other permissions (bits 0-2)
            file_mode & 7
        };

        // Check if all requested permissions are granted
        // R_OK=4, W_OK=2, X_OK=1 map directly to permission bits
        if (mask & perm_bits) != mask {
            return Err(std::io::Error::from_raw_os_error(libc::EACCES));
        }

        Ok(())
    }

    /// Get the default mode for an entry type.
    fn mode_for_entry_type(&self, entry_type: EntryType) -> u32 {
        match entry_type {
            EntryType::File => DEFAULT_FILE_MODE,
            EntryType::Directory => DEFAULT_DIR_MODE,
            EntryType::Symlink => DEFAULT_SYMLINK_MODE,
        }
    }

    /// Check read access for an inode.
    fn check_read_access(&self, ctx: &Context, inode: u64) -> std::io::Result<()> {
        if inode == ROOT_INODE {
            return self.check_access(ctx, DEFAULT_DIR_MODE, R_OK);
        }

        let entry = self
            .inodes
            .get_path(inode)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        let mode = self.mode_for_entry_type(entry.entry_type);
        self.check_access(ctx, mode, R_OK)
    }

    /// Check write access for an inode.
    fn check_write_access(&self, ctx: &Context, inode: u64) -> std::io::Result<()> {
        if inode == ROOT_INODE {
            return self.check_access(ctx, DEFAULT_DIR_MODE, W_OK);
        }

        let entry = self
            .inodes
            .get_path(inode)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        let mode = self.mode_for_entry_type(entry.entry_type);
        self.check_access(ctx, mode, W_OK)
    }

    /// Check execute access for an inode (for directories, this is needed for traversal).
    fn check_exec_access(&self, ctx: &Context, inode: u64) -> std::io::Result<()> {
        if inode == ROOT_INODE {
            return self.check_access(ctx, DEFAULT_DIR_MODE, X_OK);
        }

        let entry = self
            .inodes
            .get_path(inode)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        let mode = self.mode_for_entry_type(entry.entry_type);
        self.check_access(ctx, mode, X_OK)
    }

    /// Convert open flags to permission mask.
    fn flags_to_permission_mask(flags: u32) -> u32 {
        let accmode = flags & libc::O_ACCMODE as u32;
        match accmode {
            x if x == libc::O_RDONLY as u32 => R_OK,
            x if x == libc::O_WRONLY as u32 => W_OK,
            x if x == libc::O_RDWR as u32 => R_OK | W_OK,
            _ => R_OK, // Default to read-only for unknown modes
        }
    }
}
