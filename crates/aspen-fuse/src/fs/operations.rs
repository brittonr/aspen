//! FUSE `FileSystem` trait implementation for `AspenFs`.
//!
//! All FUSE operations (lookup, read, write, mkdir, etc.) are implemented here.

use std::collections::BTreeSet;
use std::ffi::CStr;
use std::time::Duration;

use fuse_backend_rs::abi::fuse_abi::CreateIn;
use fuse_backend_rs::abi::fuse_abi::stat64;
use fuse_backend_rs::api::filesystem::Context;
use fuse_backend_rs::api::filesystem::DirEntry;
use fuse_backend_rs::api::filesystem::Entry;
use fuse_backend_rs::api::filesystem::FileSystem;
use fuse_backend_rs::api::filesystem::FsOptions;
use fuse_backend_rs::api::filesystem::OpenOptions;
use fuse_backend_rs::api::filesystem::SetattrValid;
use fuse_backend_rs::api::filesystem::ZeroCopyReader;
use fuse_backend_rs::api::filesystem::ZeroCopyWriter;
use tracing::debug;

use super::AspenFs;
use crate::constants::ATTR_TTL;
use crate::constants::BLOCK_SIZE;
use crate::constants::DEFAULT_DIR_MODE;
use crate::constants::MAX_KEY_SIZE;
use crate::constants::MAX_READDIR_ENTRIES;
use crate::constants::MAX_XATTR_NAME_SIZE;
use crate::constants::MAX_XATTR_VALUE_SIZE;
use crate::constants::MAX_XATTRS_PER_FILE;
use crate::constants::ROOT_INODE;
use crate::constants::SYMLINK_SUFFIX;
use crate::constants::XATTR_PREFIX;
use crate::inode::EntryType;

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

    fn lookup(&self, ctx: &Context, parent: u64, name: &CStr) -> std::io::Result<Entry> {
        // Check execute permission on parent directory (required for traversal)
        self.check_exec_access(ctx, parent)?;

        let name_str =
            name.to_str().map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid name"))?;

        // Get parent path
        let parent_entry = self
            .inodes
            .get_path(parent)?
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
        let inode = self.inodes.get_or_create(&child_path, entry_type)?;

        // Get size (0 for directories, target length for symlinks)
        let size = match entry_type {
            EntryType::File => {
                let key = Self::path_to_key(&child_path)?;
                self.kv_read(&key)?.map(|v| v.len() as u64).unwrap_or(0)
            }
            EntryType::Directory => 0,
            EntryType::Symlink => {
                let key = Self::path_to_key(&child_path)?;
                let symlink_key = format!("{}{}", key, SYMLINK_SUFFIX);
                self.kv_read(&symlink_key)?.map(|v| v.len() as u64).unwrap_or(0)
            }
        };

        Ok(self.make_entry(inode, entry_type, size))
    }

    fn getattr(&self, _ctx: &Context, inode: u64, _handle: Option<u64>) -> std::io::Result<(stat64, Duration)> {
        if inode == ROOT_INODE {
            return Ok((self.make_attr(ROOT_INODE, EntryType::Directory, 0), ATTR_TTL));
        }

        let entry = self
            .inodes
            .get_path(inode)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        let size = match entry.entry_type {
            EntryType::File => {
                let key = Self::path_to_key(&entry.path)?;
                self.kv_read(&key)?.map(|v| v.len() as u64).unwrap_or(0)
            }
            EntryType::Directory => 0,
            EntryType::Symlink => {
                let key = Self::path_to_key(&entry.path)?;
                let symlink_key = format!("{}{}", key, SYMLINK_SUFFIX);
                self.kv_read(&symlink_key)?.map(|v| v.len() as u64).unwrap_or(0)
            }
        };

        Ok((self.make_attr(inode, entry.entry_type, size), ATTR_TTL))
    }

    fn setattr(
        &self,
        ctx: &Context,
        inode: u64,
        attr: stat64,
        _handle: Option<u64>,
        valid: SetattrValid,
    ) -> std::io::Result<(stat64, Duration)> {
        // Handle truncation / extension (e.g., O_TRUNC sends setattr with SIZE)
        if valid.contains(SetattrValid::SIZE) && inode != ROOT_INODE {
            let entry = self
                .inodes
                .get_path(inode)?
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

            if entry.entry_type == EntryType::File {
                let key = Self::path_to_key(&entry.path)?;
                let mut value = self.kv_read(&key)?.unwrap_or_default();
                value.resize(attr.st_size as usize, 0);
                self.kv_write(&key, &value)?;
            }
        }

        // Return updated attributes (mode, owner, etc. are not mutable)
        self.getattr(ctx, inode, None)
    }

    fn opendir(&self, ctx: &Context, inode: u64, _flags: u32) -> std::io::Result<(Option<u64>, OpenOptions)> {
        // Check read and execute permissions (needed to list and traverse directory)
        self.check_read_access(ctx, inode)?;
        self.check_exec_access(ctx, inode)?;

        // Verify it's a directory
        if inode == ROOT_INODE {
            return Ok((None, OpenOptions::empty()));
        }

        let entry = self
            .inodes
            .get_path(inode)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        if entry.entry_type != EntryType::Directory {
            return Err(std::io::Error::new(std::io::ErrorKind::NotADirectory, "not a directory"));
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
            let entry = self
                .inodes
                .get_path(inode)?
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;
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
                self.inodes.get_inode(parent_path)?.unwrap_or(ROOT_INODE)
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
            let child_inode = self.inodes.get_or_create(&child_path, entry_type)?;

            let dtype = match entry_type {
                EntryType::File => libc::DT_REG as u32,
                EntryType::Directory => libc::DT_DIR as u32,
                EntryType::Symlink => libc::DT_LNK as u32,
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
        ctx: &Context,
        inode: u64,
        flags: u32,
        _fuse_flags: u32,
    ) -> std::io::Result<(Option<u64>, OpenOptions, Option<u32>)> {
        // Verify file exists
        if inode == ROOT_INODE {
            return Err(std::io::Error::new(std::io::ErrorKind::IsADirectory, "is a directory"));
        }

        let entry = self
            .inodes
            .get_path(inode)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        if entry.entry_type == EntryType::Directory {
            return Err(std::io::Error::new(std::io::ErrorKind::IsADirectory, "is a directory"));
        }

        // Check permissions based on open flags
        let perm_mask = Self::flags_to_permission_mask(flags);
        let mode = self.mode_for_entry_type(entry.entry_type);
        self.check_access(ctx, mode, perm_mask)?;

        // No handle needed for stateless operations
        Ok((None, OpenOptions::empty(), None))
    }

    fn read(
        &self,
        ctx: &Context,
        inode: u64,
        _handle: u64,
        w: &mut dyn ZeroCopyWriter,
        size: u32,
        offset: u64,
        _lock_owner: Option<u64>,
        _flags: u32,
    ) -> std::io::Result<usize> {
        // Check read permission
        self.check_read_access(ctx, inode)?;

        let entry = self
            .inodes
            .get_path(inode)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        let key = Self::path_to_key(&entry.path)?;
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
        ctx: &Context,
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
        // Check write permission
        self.check_write_access(ctx, inode)?;

        let entry = self
            .inodes
            .get_path(inode)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        let key = Self::path_to_key(&entry.path)?;

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
        ctx: &Context,
        parent: u64,
        name: &CStr,
        _args: CreateIn,
    ) -> std::io::Result<(Entry, Option<u64>, OpenOptions, Option<u32>)> {
        // Check write permission on parent directory
        self.check_write_access(ctx, parent)?;

        let name_str =
            name.to_str().map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid name"))?;

        // Get parent path
        let parent_entry = self
            .inodes
            .get_path(parent)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "parent not found"))?;

        // Build child path
        let child_path = if parent_entry.path.is_empty() {
            name_str.to_string()
        } else {
            format!("{}/{}", parent_entry.path, name_str)
        };

        let key = Self::path_to_key(&child_path)?;

        // Create empty file
        self.kv_write(&key, &[])?;

        // Allocate inode
        let inode = self.inodes.get_or_create(&child_path, EntryType::File)?;
        let entry = self.make_entry(inode, EntryType::File, 0);

        Ok((entry, None, OpenOptions::empty(), None))
    }

    fn unlink(&self, ctx: &Context, parent: u64, name: &CStr) -> std::io::Result<()> {
        // Check write permission on parent directory
        self.check_write_access(ctx, parent)?;

        let name_str =
            name.to_str().map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid name"))?;

        // Get parent path
        let parent_entry = self
            .inodes
            .get_path(parent)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "parent not found"))?;

        // Build child path
        let child_path = if parent_entry.path.is_empty() {
            name_str.to_string()
        } else {
            format!("{}/{}", parent_entry.path, name_str)
        };

        let key = Self::path_to_key(&child_path)?;

        // Delete from KV
        self.kv_delete(&key)?;

        // Remove from inode cache
        self.inodes.remove_path(&child_path)?;

        Ok(())
    }

    fn mkdir(&self, ctx: &Context, parent: u64, name: &CStr, _mode: u32, _umask: u32) -> std::io::Result<Entry> {
        // Check write permission on parent directory
        self.check_write_access(ctx, parent)?;

        let name_str =
            name.to_str().map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid name"))?;

        // Get parent path
        let parent_entry = self
            .inodes
            .get_path(parent)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "parent not found"))?;

        // Build directory path
        let dir_path = if parent_entry.path.is_empty() {
            name_str.to_string()
        } else {
            format!("{}/{}", parent_entry.path, name_str)
        };

        // Create directory marker (empty key ending with /)
        // Or just allocate the inode - directories are virtual
        let inode = self.inodes.get_or_create(&dir_path, EntryType::Directory)?;

        Ok(self.make_entry(inode, EntryType::Directory, 0))
    }

    fn rmdir(&self, ctx: &Context, parent: u64, name: &CStr) -> std::io::Result<()> {
        // Check write permission on parent directory
        self.check_write_access(ctx, parent)?;

        let name_str =
            name.to_str().map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid name"))?;

        // Get parent path
        let parent_entry = self
            .inodes
            .get_path(parent)?
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
            return Err(std::io::Error::new(std::io::ErrorKind::DirectoryNotEmpty, "directory not empty"));
        }

        // Remove from inode cache
        self.inodes.remove_path(&dir_path)?;

        Ok(())
    }

    fn fsync(&self, _ctx: &Context, _inode: u64, _datasync: bool, _handle: u64) -> std::io::Result<()> {
        // Writes are already synchronous through Raft consensus
        Ok(())
    }

    fn flush(&self, _ctx: &Context, _inode: u64, _handle: u64, _lock_owner: u64) -> std::io::Result<()> {
        // No buffering, nothing to flush
        Ok(())
    }

    fn rename(
        &self,
        ctx: &Context,
        olddir: u64,
        oldname: &CStr,
        newdir: u64,
        newname: &CStr,
        _flags: u32,
    ) -> std::io::Result<()> {
        // Check write permission on both parent directories
        self.check_write_access(ctx, olddir)?;
        if olddir != newdir {
            self.check_write_access(ctx, newdir)?;
        }

        let old_name_str = oldname
            .to_str()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid name"))?;
        let new_name_str = newname
            .to_str()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid name"))?;

        // Get old parent path
        let old_parent = self
            .inodes
            .get_path(olddir)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "old parent not found"))?;

        // Get new parent path
        let new_parent = self
            .inodes
            .get_path(newdir)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "new parent not found"))?;

        // Build old path
        let old_path = if old_parent.path.is_empty() {
            old_name_str.to_string()
        } else {
            format!("{}/{}", old_parent.path, old_name_str)
        };

        // Build new path
        let new_path = if new_parent.path.is_empty() {
            new_name_str.to_string()
        } else {
            format!("{}/{}", new_parent.path, new_name_str)
        };

        let old_key = Self::path_to_key(&old_path)?;
        let new_key = Self::path_to_key(&new_path)?;

        debug!(old_key, new_key, "rename");

        // Check source exists and get its type
        let entry_type = self
            .exists(&old_path)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "source not found"))?;

        match entry_type {
            EntryType::File | EntryType::Symlink => {
                self.rename_file_or_symlink(&old_key, &new_key)?;
            }
            EntryType::Directory => {
                self.rename_directory(&old_key, &new_key)?;
            }
        }

        // Update inode cache
        self.inodes.remove_path(&old_path)?;
        self.inodes.get_or_create(&new_path, entry_type)?;

        Ok(())
    }

    fn symlink(&self, ctx: &Context, linkname: &CStr, parent: u64, name: &CStr) -> std::io::Result<Entry> {
        // Check write permission on parent directory
        self.check_write_access(ctx, parent)?;

        let link_target = linkname
            .to_str()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid target"))?;
        let name_str =
            name.to_str().map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid name"))?;

        // Get parent path
        let parent_entry = self
            .inodes
            .get_path(parent)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "parent not found"))?;

        // Build symlink path
        let link_path = if parent_entry.path.is_empty() {
            name_str.to_string()
        } else {
            format!("{}/{}", parent_entry.path, name_str)
        };

        let key = Self::path_to_key(&link_path)?;
        let symlink_key = format!("{}{}", key, SYMLINK_SUFFIX);

        debug!(key, link_target, "symlink");

        // Store empty file marker and symlink target
        self.kv_write(&key, &[])?;
        self.kv_write(&symlink_key, link_target.as_bytes())?;

        // Allocate inode
        let inode = self.inodes.get_or_create(&link_path, EntryType::Symlink)?;
        let entry = self.make_entry(inode, EntryType::Symlink, link_target.len() as u64);

        Ok(entry)
    }

    fn readlink(&self, _ctx: &Context, inode: u64) -> std::io::Result<Vec<u8>> {
        let entry = self
            .inodes
            .get_path(inode)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        if entry.entry_type != EntryType::Symlink {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a symlink"));
        }

        let key = Self::path_to_key(&entry.path)?;
        let symlink_key = format!("{}{}", key, SYMLINK_SUFFIX);

        debug!(key, "readlink");

        self.kv_read(&symlink_key)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "symlink target not found"))
    }

    fn statfs(&self, _ctx: &Context, _inode: u64) -> std::io::Result<libc::statvfs64> {
        // Return reasonable defaults for a network filesystem
        // SAFETY: statvfs64 is a C struct that can be safely zero-initialized.
        // All fields are primitive types (integers) with no invariants.
        let mut st: libc::statvfs64 = unsafe { std::mem::zeroed() };

        st.f_bsize = u64::from(BLOCK_SIZE); // Filesystem block size
        st.f_frsize = u64::from(BLOCK_SIZE); // Fragment size
        st.f_blocks = 1024 * 1024 * 1024; // Total blocks (1 TB at 4K blocks)
        st.f_bfree = 1024 * 1024 * 512; // Free blocks (512 GB)
        st.f_bavail = 1024 * 1024 * 512; // Available blocks
        st.f_files = 1_000_000; // Total inodes
        st.f_ffree = 900_000; // Free inodes
        st.f_favail = 900_000; // Available inodes
        st.f_namemax = MAX_KEY_SIZE as u64; // Max filename length

        Ok(st)
    }

    fn access(&self, ctx: &Context, inode: u64, mask: u32) -> std::io::Result<()> {
        // Check if the file exists and get its type
        if inode == ROOT_INODE {
            // Root directory always exists, check permissions
            return self.check_access(ctx, DEFAULT_DIR_MODE, mask);
        }

        let entry = self
            .inodes
            .get_path(inode)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        // Get the mode for this entry type and check permissions
        let mode = self.mode_for_entry_type(entry.entry_type);
        self.check_access(ctx, mode, mask)
    }

    fn setxattr(&self, ctx: &Context, inode: u64, name: &CStr, value: &[u8], flags: u32) -> std::io::Result<()> {
        // Check write permission
        self.check_write_access(ctx, inode)?;

        let name_str =
            name.to_str().map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid name"))?;

        // Validate name length
        if name_str.len() > MAX_XATTR_NAME_SIZE {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "xattr name too long"));
        }

        // Validate value size
        if value.len() > MAX_XATTR_VALUE_SIZE {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "xattr value too large"));
        }

        let entry = self
            .inodes
            .get_path(inode)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        let key = Self::path_to_key(&entry.path)?;
        let xattr_key = format!("{}{}{}", key, XATTR_PREFIX, name_str);

        debug!(key, name_str, value_len = value.len(), "setxattr");

        // Check flags: XATTR_CREATE (1) = fail if exists, XATTR_REPLACE (2) = fail if missing
        let exists = self.kv_read(&xattr_key)?.is_some();

        if flags & libc::XATTR_CREATE as u32 != 0 && exists {
            return Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "xattr already exists"));
        }

        if flags & libc::XATTR_REPLACE as u32 != 0 && !exists {
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "xattr not found"));
        }

        // Check xattr count limit
        if !exists {
            let prefix = format!("{}{}", key, XATTR_PREFIX);
            let xattrs = self.kv_scan(&prefix)?;
            if xattrs.len() >= MAX_XATTRS_PER_FILE {
                return Err(std::io::Error::new(std::io::ErrorKind::OutOfMemory, "too many xattrs"));
            }
        }

        self.kv_write(&xattr_key, value)?;
        Ok(())
    }

    fn getxattr(
        &self,
        ctx: &Context,
        inode: u64,
        name: &CStr,
        size: u32,
    ) -> std::io::Result<fuse_backend_rs::api::filesystem::GetxattrReply> {
        // Check read permission
        self.check_read_access(ctx, inode)?;

        use fuse_backend_rs::api::filesystem::GetxattrReply;

        let name_str =
            name.to_str().map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid name"))?;

        let entry = self
            .inodes
            .get_path(inode)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        let key = Self::path_to_key(&entry.path)?;
        let xattr_key = format!("{}{}{}", key, XATTR_PREFIX, name_str);

        debug!(key, name_str, size, "getxattr");

        let value = self
            .kv_read(&xattr_key)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "xattr not found"))?;

        if size == 0 {
            // Size query only
            Ok(GetxattrReply::Count(value.len() as u32))
        } else if size < value.len() as u32 {
            Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "buffer too small"))
        } else {
            Ok(GetxattrReply::Value(value))
        }
    }

    fn listxattr(
        &self,
        ctx: &Context,
        inode: u64,
        size: u32,
    ) -> std::io::Result<fuse_backend_rs::api::filesystem::ListxattrReply> {
        // Check read permission
        self.check_read_access(ctx, inode)?;

        use fuse_backend_rs::api::filesystem::ListxattrReply;

        let entry = self
            .inodes
            .get_path(inode)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        let key = Self::path_to_key(&entry.path)?;
        let prefix = format!("{}{}", key, XATTR_PREFIX);

        debug!(key, size, "listxattr");

        let xattr_keys = self.kv_scan(&prefix)?;

        // Build null-terminated list of names
        let mut result = Vec::new();
        for xattr_key in xattr_keys {
            // Extract name part after prefix
            if xattr_key.len() > prefix.len() {
                let name = &xattr_key[prefix.len()..];
                result.extend_from_slice(name.as_bytes());
                result.push(0); // null terminator
            }
        }

        if size == 0 {
            // Size query only
            Ok(ListxattrReply::Count(result.len() as u32))
        } else if size < result.len() as u32 {
            Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "buffer too small"))
        } else {
            Ok(ListxattrReply::Names(result))
        }
    }

    fn removexattr(&self, ctx: &Context, inode: u64, name: &CStr) -> std::io::Result<()> {
        // Check write permission
        self.check_write_access(ctx, inode)?;

        let name_str =
            name.to_str().map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid name"))?;

        let entry = self
            .inodes
            .get_path(inode)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        let key = Self::path_to_key(&entry.path)?;
        let xattr_key = format!("{}{}{}", key, XATTR_PREFIX, name_str);

        debug!(key, name_str, "removexattr");

        // Check if exists
        if self.kv_read(&xattr_key)?.is_none() {
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "xattr not found"));
        }

        self.kv_delete(&xattr_key)?;
        Ok(())
    }

    fn releasedir(&self, _ctx: &Context, _inode: u64, _flags: u32, _handle: u64) -> std::io::Result<()> {
        // Nothing to release for stateless directory operations
        Ok(())
    }

    fn fsyncdir(&self, _ctx: &Context, _inode: u64, _datasync: bool, _handle: u64) -> std::io::Result<()> {
        // Writes are already synchronous through Raft consensus
        Ok(())
    }
}

/// Helper methods for rename operations, extracted to keep `rename` under 70 lines.
impl AspenFs {
    /// Rename a file or symlink by copying data and xattrs to the new key.
    fn rename_file_or_symlink(&self, old_key: &str, new_key: &str) -> std::io::Result<()> {
        // Read old value
        let value = self.kv_read(old_key)?.unwrap_or_default();

        // For symlinks, also copy the target
        let symlink_old = format!("{}{}", old_key, SYMLINK_SUFFIX);
        let symlink_target = self.kv_read(&symlink_old)?;

        // Write to new location
        self.kv_write(new_key, &value)?;

        // If symlink, also write target
        if let Some(target) = symlink_target {
            let symlink_new = format!("{}{}", new_key, SYMLINK_SUFFIX);
            self.kv_write(&symlink_new, &target)?;
            self.kv_delete(&symlink_old)?;
        }

        // Delete old
        self.kv_delete(old_key)?;

        // Copy xattrs
        let xattr_prefix = format!("{}{}", old_key, XATTR_PREFIX);
        let xattrs = self.kv_scan(&xattr_prefix)?;
        for xattr_key in xattrs {
            if let Some(value) = self.kv_read(&xattr_key)? {
                let attr_name = &xattr_key[old_key.len()..];
                let new_xattr_key = format!("{}{}", new_key, attr_name);
                self.kv_write(&new_xattr_key, &value)?;
                self.kv_delete(&xattr_key)?;
            }
        }

        Ok(())
    }

    /// Rename a directory by moving all children to the new prefix.
    fn rename_directory(&self, old_key: &str, new_key: &str) -> std::io::Result<()> {
        let old_prefix = format!("{}/", old_key);
        let new_prefix = format!("{}/", new_key);

        let children = self.kv_scan(&old_prefix)?;
        for child_key in children {
            if let Some(value) = self.kv_read(&child_key)? {
                let suffix = &child_key[old_prefix.len()..];
                let new_child_key = format!("{}{}", new_prefix, suffix);
                self.kv_write(&new_child_key, &value)?;
                self.kv_delete(&child_key)?;
            }
        }

        Ok(())
    }
}
