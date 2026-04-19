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
use fuse_backend_rs::api::filesystem::FileLock;
use fuse_backend_rs::api::filesystem::FileSystem;
use fuse_backend_rs::api::filesystem::FsOptions;
use fuse_backend_rs::api::filesystem::OpenOptions;
use fuse_backend_rs::api::filesystem::SetattrValid;
use fuse_backend_rs::api::filesystem::ZeroCopyReader;
use fuse_backend_rs::api::filesystem::ZeroCopyWriter;
use tracing::debug;
use tracing::warn;

use super::AspenFs;
use crate::constants::ATTR_TTL;
use crate::constants::BLOCK_SIZE;
use crate::constants::DEFAULT_DIR_MODE;
use crate::constants::MAX_KEY_SIZE;
use crate::constants::MAX_READDIR_ENTRIES;
use crate::constants::MAX_XATTR_NAME_SIZE;
use crate::constants::MAX_XATTR_VALUE_SIZE;
use crate::constants::MAX_XATTRS_PER_FILE;
use crate::constants::META_SUFFIX;
use crate::constants::ROOT_INODE;
use crate::constants::SYMLINK_SUFFIX;
use crate::constants::XATTR_PREFIX;
use crate::inode::EntryType;
use crate::metadata::FileMetadata;

const STATFS_TOTAL_BLOCKS: u64 = 1024u64.saturating_mul(1024).saturating_mul(1024);
const STATFS_FREE_BLOCKS: u64 = 1024u64.saturating_mul(1024).saturating_mul(512);

fn is_internal_child_name(child_name: &str) -> bool {
    child_name.ends_with(META_SUFFIX)
        || child_name.ends_with(SYMLINK_SUFFIX)
        || child_name.contains(crate::constants::XATTR_PREFIX)
        || child_name.contains(crate::constants::CHUNK_KEY_SUFFIX)
}

impl FileSystem for AspenFs {
    type Inode = u64;
    type Handle = u64;

    fn init(&self, _capable: FsOptions) -> std::io::Result<FsOptions> {
        // Start the background flush timer.
        // The timer periodically flushes expired write buffers even when
        // there is no FUSE activity, ensuring durability.
        let timer = crate::writeback::FlushTimer::start(self.write_buffer.clone(), self.clone_for_kv_access());
        if let Ok(mut guard) = self.flush_timer.lock() {
            *guard = Some(timer);
        }

        // Return capabilities we support
        Ok(FsOptions::empty())
    }

    fn destroy(&self) {
        // Log access statistics summary on unmount
        let stats = self.access_stats.snapshot();
        tracing::info!(
            files_opened = stats.files_opened,
            files_read = stats.files_read,
            read_ratio = format!("{:.1}%", stats.read_ratio_percent()),
            bytes_fetched = stats.bytes_fetched_from_cluster,
            bytes_cached = stats.bytes_served_from_cache,
            prefetch_bytes = stats.prefetch_bytes_fetched,
            prefetch_savings = stats.prefetch_savings_bytes(),
            "FUSE access stats on unmount"
        );

        // Drop all active branches (uncommitted state is discarded).
        #[cfg(feature = "kv-branch")]
        self.branch_manager.drop_all();

        // Stop the background flush timer
        if let Ok(mut guard) = self.flush_timer.lock()
            && let Some(mut timer) = guard.take()
        {
            timer.stop();
        }

        // Flush all remaining buffered writes before shutdown.
        // Uses a lightweight KV-access clone since we can't pass &self
        // to our own write_buffer (it's already borrowed as &self.write_buffer).
        let kv_fs = self.clone_for_kv_access();
        if let Err(e) = self.write_buffer.flush_all(&kv_fs) {
            tracing::error!(error = %e, "failed to flush write buffer on destroy");
        }
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

        // Get size and metadata (cached)
        let (size, meta) = self.get_size_and_meta(&child_path, entry_type)?;

        match meta {
            Some(ref m) => Ok(self.make_entry_with_meta(inode, entry_type, size, m)),
            None => Ok(self.make_entry(inode, entry_type, size)),
        }
    }

    fn getattr(&self, _ctx: &Context, inode: u64, _handle: Option<u64>) -> std::io::Result<(stat64, Duration)> {
        if inode == ROOT_INODE {
            return Ok((self.make_attr(ROOT_INODE, EntryType::Directory, 0), ATTR_TTL));
        }

        let entry = self
            .inodes
            .get_path(inode)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        let (size, meta) = self.get_size_and_meta(&entry.path, entry.entry_type)?;

        let attr = match meta {
            Some(ref m) => self.make_attr_with_meta(inode, entry.entry_type, size, m),
            None => self.make_attr(inode, entry.entry_type, size),
        };

        Ok((attr, ATTR_TTL))
    }

    fn setattr(
        &self,
        ctx: &Context,
        inode: u64,
        attr: stat64,
        _handle: Option<u64>,
        valid: SetattrValid,
    ) -> std::io::Result<(stat64, Duration)> {
        if inode == ROOT_INODE {
            return self.getattr(ctx, inode, None);
        }

        let entry = self
            .inodes
            .get_path(inode)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "inode not found"))?;

        let key = Self::path_to_key(&entry.path)?;

        // Handle truncation / extension (e.g., O_TRUNC sends setattr with SIZE)
        if valid.contains(SetattrValid::SIZE) && entry.entry_type == EntryType::File {
            let mut value = self.kv_read(&key)?.unwrap_or_default();
            let file_size_bytes = usize::try_from(attr.st_size)
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "file size must fit in usize"))?;
            value.resize(file_size_bytes, 0);
            self.kv_write(&key, &value)?;
        }

        // Handle timestamp changes (touch, utimensat)
        let needs_meta_update = valid.contains(SetattrValid::MTIME)
            || valid.contains(SetattrValid::MTIME_NOW)
            || valid.contains(SetattrValid::SIZE);

        if needs_meta_update {
            let existing = self.read_metadata(&key)?.unwrap_or_else(FileMetadata::now);

            let updated = if valid.contains(SetattrValid::MTIME_NOW) {
                // `touch` without explicit time — set mtime to now
                existing.touch()
            } else if valid.contains(SetattrValid::MTIME) {
                // Explicit mtime from utimensat
                FileMetadata::with_mtime(attr.st_mtime, attr.st_mtime_nsec).touch_ctime()
            } else {
                // SIZE change — update both mtime and ctime
                existing.touch()
            };

            self.write_metadata(&key, &updated)?;
            // Invalidate meta cache so getattr picks up new timestamps
            self.cache.invalidate_meta(&entry.path);
        }

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

        // Extract direct children (not nested), filtering out internal keys
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

                // Filter out internal keys: .meta, .symlink, .xattr., .chunk.
                if is_internal_child_name(child) {
                    continue;
                }

                children.insert(child.to_string());
            }
        }

        // Build directory entries (skip to offset)
        let mut dir_entry_index = 0u64;
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
            dir_entry_index = 1;
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
            dir_entry_index = 2;
        }

        // Snapshot names for prefetch before the loop consumes children
        let prefetch_names: Vec<String> = children.iter().cloned().collect();

        // Add child entries
        let mut entry_count = 0u32;
        for child_name in children {
            dir_entry_index += 1;

            if dir_entry_index <= offset {
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
                offset: dir_entry_index,
                type_: dtype,
                name: child_name.as_bytes(),
            };
            bytes_written += add_entry(entry)? as u32;
            entry_count += 1;
        }

        // Queue prefetch for directory entries (metadata + small file data)
        self.prefetcher.queue_dir_prefetch(&prefix, &prefetch_names);

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

        // Track file open for access stats
        self.access_stats.record_open();

        // Exec cache: create child tracking session for new PIDs
        #[cfg(feature = "exec-cache")]
        if self.read_tracker.is_enabled() {
            let pid = ctx.pid as u32;
            if !self.read_tracker.has_session(pid) {
                if let Some(ppid) = aspen_exec_cache::read_tracker::read_ppid(pid) {
                    if self.read_tracker.has_session(ppid) {
                        self.read_tracker.start_child_session(pid, ppid);
                    }
                }
            }
        }

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

        // Track first read per inode for access stats
        self.access_stats.record_read(inode);

        // Execute any pending prefetch requests (populates cache)
        self.prefetcher.execute_pending(self);

        // Use chunked read — only fetches the chunks that overlap with the range
        let mut data = crate::chunking::chunked_read_range(self, &key, offset, size)?;

        // Apply buffered writes on top for read-your-writes consistency
        let buffered = self.write_buffer.read_buffered(&key, offset, size);
        if let Some(overlaps) = buffered {
            for (buf_offset, buf_data) in overlaps {
                let start_in_data = usize::try_from(buf_offset.saturating_sub(offset)).map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "buffered read offset must fit in usize")
                })?;
                let end_in_data = start_in_data.saturating_add(buf_data.len()).min(data.len());
                if start_in_data < data.len() {
                    let copy_len = end_in_data.saturating_sub(start_in_data).min(buf_data.len());
                    let slice_end = start_in_data.saturating_add(copy_len).min(data.len());
                    data[start_in_data..slice_end].copy_from_slice(&buf_data[..copy_len]);
                }
            }
        }

        if data.is_empty() {
            return Ok(0);
        }

        // Track access pattern for sequential readahead
        if let Some(req) = self.prefetcher.record_read(&key, offset, size) {
            // Execute readahead prefetch inline
            match req {
                crate::prefetch::PrefetchRequest::FileData {
                    key: k,
                    offset: o,
                    size: s,
                } => {
                    if let Err(error) = crate::chunking::chunked_read_range(self, &k, o, s) {
                        debug!(%error, prefetch_key = %k, prefetch_offset = o, prefetch_size = s, "readahead prefetch failed");
                    }
                }
                crate::prefetch::PrefetchRequest::DirMeta { .. } => {
                    // Dir prefetch handled by execute_pending
                }
            }
        }

        // Exec cache: record this read in the PID's tracking session
        #[cfg(feature = "exec-cache")]
        if self.read_tracker.is_enabled() {
            let pid = ctx.pid as u32;
            if self.read_tracker.has_session(pid) {
                let content_hash = blake3::hash(&data);
                self.read_tracker.record_read(pid, key.clone(), *content_hash.as_bytes());
            }
        }

        w.write_all(&data)?;
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

        // Read new data from the FUSE client
        let write_size_bytes = usize::try_from(size)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "write size must fit in usize"))?;
        let mut buf = vec![0u8; write_size_bytes];
        r.read_exact(&mut buf)?;

        // Buffer the write — may trigger auto-flush
        let should_flush = self.write_buffer.buffer_write(&key, offset, &buf).unwrap_or(false);

        if should_flush {
            // Auto-flush: per-file or global limit exceeded
            self.write_buffer.flush_file(self, &key)?;
        }

        // Update timestamps: both mtime and ctime change on write
        let meta = self.read_metadata(&key)?.unwrap_or_else(FileMetadata::now).touch();
        self.write_metadata(&key, &meta)?;
        self.cache.invalidate_meta(&entry.path);

        Ok(write_size_bytes)
    }

    fn release(
        &self,
        _ctx: &Context,
        inode: u64,
        _flags: u32,
        _handle: u64,
        _flush: bool,
        flock_release: bool,
        lock_owner: Option<u64>,
    ) -> std::io::Result<()> {
        // Flush any buffered writes for this file
        if let Ok(Some(entry)) = self.inodes.get_path(inode)
            && let Ok(key) = Self::path_to_key(&entry.path)
            && let Err(error) = self.write_buffer.flush_file(self, &key)
        {
            debug!(%error, file_key = %key, "failed to flush buffered writes on release");
        }

        // Release locks held by this owner
        if flock_release && let Some(owner) = lock_owner {
            self.lock_manager.release_owner(inode, owner);
        }

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

        // Store initial timestamps
        let meta = FileMetadata::now();
        self.write_metadata(&key, &meta)?;

        // Allocate inode
        let inode = self.inodes.get_or_create(&child_path, EntryType::File)?;
        let entry = self.make_entry_with_meta(inode, EntryType::File, 0, &meta);

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

        // Discard any buffered writes (file is being deleted)
        if let Err(error) = self.write_buffer.discard_file(&key) {
            debug!(%error, file_key = %key, "failed to discard buffered file writes during unlink");
        }

        // Release all locks on this inode before removing it
        if let Ok(Some(child_inode)) = self.inodes.get_inode(&child_path) {
            self.lock_manager.release_inode(child_inode);
        }

        // Delete from KV (data + all chunks + metadata)
        crate::chunking::chunked_delete(self, &key)?;
        if let Err(error) = self.delete_metadata(&key) {
            debug!(%error, file_key = %key, "failed to delete file metadata during unlink");
        }

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

        // Store directory timestamps
        let key = Self::path_to_key(&dir_path)?;
        let meta = FileMetadata::now();
        self.write_metadata(&key, &meta)?;

        // Allocate the inode — directories are virtual (derived from key prefixes)
        let inode = self.inodes.get_or_create(&dir_path, EntryType::Directory)?;

        Ok(self.make_entry_with_meta(inode, EntryType::Directory, 0, &meta))
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

        // Check if directory is empty (exclude .meta keys from the check)
        let prefix = format!("{}/", dir_path);
        let children = self.kv_scan(&prefix)?;
        let real_children: Vec<_> = children.iter().filter(|k| !k.ends_with(META_SUFFIX)).collect();
        if !real_children.is_empty() {
            return Err(std::io::Error::new(std::io::ErrorKind::DirectoryNotEmpty, "directory not empty"));
        }

        // Clean up directory metadata
        let key = Self::path_to_key(&dir_path)?;
        if let Err(error) = self.delete_metadata(&key) {
            debug!(%error, dir_key = %key, "failed to delete directory metadata during rmdir");
        }

        // Remove from inode cache
        self.inodes.remove_path(&dir_path)?;

        Ok(())
    }

    fn fsync(&self, _ctx: &Context, inode: u64, _datasync: bool, _handle: u64) -> std::io::Result<()> {
        // Flush buffered writes to KV store (then Raft consensus makes them durable)
        if let Ok(Some(entry)) = self.inodes.get_path(inode)
            && let Ok(key) = Self::path_to_key(&entry.path)
        {
            self.write_buffer.flush_file(self, &key)?;
        }
        Ok(())
    }

    fn flush(&self, _ctx: &Context, inode: u64, _handle: u64, _lock_owner: u64) -> std::io::Result<()> {
        // Flush buffered writes on close
        if let Ok(Some(entry)) = self.inodes.get_path(inode)
            && let Ok(key) = Self::path_to_key(&entry.path)
        {
            self.write_buffer.flush_file(self, &key)?;
        }
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

        // Store initial timestamps
        let meta = FileMetadata::now();
        self.write_metadata(&key, &meta)?;

        // Allocate inode
        let inode = self.inodes.get_or_create(&link_path, EntryType::Symlink)?;
        let entry = self.make_entry_with_meta(inode, EntryType::Symlink, link_target.len() as u64, &meta);

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
        st.f_blocks = STATFS_TOTAL_BLOCKS; // Total blocks (1 TB at 4K blocks)
        st.f_bfree = STATFS_FREE_BLOCKS; // Free blocks (512 GB)
        st.f_bavail = STATFS_FREE_BLOCKS; // Available blocks
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
        let has_xattr = self.kv_read(&xattr_key)?.is_some();

        if flags & libc::XATTR_CREATE as u32 != 0 && has_xattr {
            return Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "xattr already exists"));
        }

        if flags & libc::XATTR_REPLACE as u32 != 0 && !has_xattr {
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "xattr not found"));
        }

        // Check xattr count limit
        if !has_xattr {
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

        let value = self.kv_read(&xattr_key)?.ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENODATA))?;

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
        let estimated_result_len = xattr_keys.iter().fold(0usize, |acc, xattr_key| {
            let name_len = xattr_key.len().saturating_sub(prefix.len()).saturating_add(1);
            acc.saturating_add(name_len)
        });
        let mut result = Vec::with_capacity(estimated_result_len);
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
        // Flush expired write buffers opportunistically
        if let Err(error) = self.write_buffer.flush_expired(self) {
            warn!(%error, "failed to flush expired write buffers during fsyncdir");
        }
        // Clean up expired prefetch trackers
        self.prefetcher.cleanup_expired();
        Ok(())
    }

    fn getlk(
        &self,
        _ctx: &Context,
        inode: u64,
        _handle: u64,
        owner: u64,
        lock: FileLock,
        _flags: u32,
    ) -> std::io::Result<FileLock> {
        let result = self.lock_manager.get_lock(inode, owner, lock.lock_type, lock.start, lock.end);
        Ok(FileLock {
            start: result.start,
            end: result.end,
            lock_type: result.lock_type,
            pid: result.pid,
        })
    }

    fn setlk(
        &self,
        _ctx: &Context,
        inode: u64,
        _handle: u64,
        owner: u64,
        lock: FileLock,
        _flags: u32,
    ) -> std::io::Result<()> {
        self.lock_manager.set_lock(inode, owner, lock.pid, lock.lock_type, lock.start, lock.end)
    }

    fn setlkw(
        &self,
        _ctx: &Context,
        inode: u64,
        _handle: u64,
        owner: u64,
        lock: FileLock,
        _flags: u32,
    ) -> std::io::Result<()> {
        self.lock_manager.set_lock_wait(inode, owner, lock.pid, lock.lock_type, lock.start, lock.end)
    }
}

/// Helper methods for rename operations, extracted to keep `rename` under 70 lines.
impl AspenFs {
    /// Rename a file or symlink by copying data, metadata, and xattrs to the new key.
    fn rename_file_or_symlink(&self, old_key: &str, new_key: &str) -> std::io::Result<()> {
        // Use chunked rename — handles both small files and chunked large files
        crate::chunking::chunked_rename(self, crate::chunking::RenameKeys { old_key, new_key })?;

        // For symlinks, also move the target
        let symlink_old = format!("{}{}", old_key, SYMLINK_SUFFIX);
        if let Some(target) = self.kv_read(&symlink_old)? {
            let symlink_new = format!("{}{}", new_key, SYMLINK_SUFFIX);
            self.kv_write(&symlink_new, &target)?;
            self.kv_delete(&symlink_old)?;
        }

        // Move metadata (update ctime for the rename)
        if let Ok(Some(meta)) = self.read_metadata(old_key) {
            let updated = meta.touch_ctime();
            self.write_metadata(new_key, &updated)?;
            if let Err(error) = self.delete_metadata(old_key) {
                debug!(%error, old_key, "failed to delete old metadata during rename");
            }
        }

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

    /// Rename a directory by moving all children (including metadata) to the new prefix.
    fn rename_directory(&self, old_key: &str, new_key: &str) -> std::io::Result<()> {
        let old_prefix = format!("{}/", old_key);
        let new_prefix = format!("{}/", new_key);

        // Move directory's own metadata
        if let Ok(Some(meta)) = self.read_metadata(old_key) {
            let updated = meta.touch_ctime();
            self.write_metadata(new_key, &updated)?;
            if let Err(error) = self.delete_metadata(old_key) {
                debug!(%error, old_key, "failed to delete old directory metadata during rename");
            }
        }

        let children = self.kv_scan(&old_prefix)?;
        for child_key in children {
            if let Some(value) = self.kv_read(&child_key)? {
                let suffix = &child_key[old_prefix.len()..];
                let new_child_key = format!("{}{}", new_prefix, suffix);
                self.kv_write(&new_child_key, &value)?;
                self.kv_delete(&child_key)?;
            }
        }

        // Invalidate cache for old prefix
        self.cache.invalidate_prefix(&old_prefix);

        Ok(())
    }
}
