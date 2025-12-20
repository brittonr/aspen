//! Inode allocation and caching for FUSE filesystem.
//!
//! Maps filesystem paths to stable inode numbers using hashing.
//! Maintains a bounded LRU cache for inode <-> path mappings.

use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::constants::{MAX_INODE_CACHE, ROOT_INODE};

/// Entry type for filesystem nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    /// Regular file (leaf node).
    File,
    /// Directory (can have children).
    Directory,
    /// Symbolic link (pointer to another path).
    Symlink,
}

/// Cached inode entry with path and type information.
#[derive(Debug, Clone)]
pub struct InodeEntry {
    /// The KV key path (e.g., "myapp/config/db").
    pub path: String,
    /// Whether this is a file or directory.
    pub entry_type: EntryType,
    /// Last access timestamp for LRU eviction.
    pub last_access: u64,
}

/// Manages inode allocation and path <-> inode mappings.
///
/// Uses stable hashing to generate consistent inode numbers for paths.
/// The root directory always has inode 1.
pub struct InodeManager {
    /// Counter for tracking access order (for LRU eviction).
    access_counter: AtomicU64,
    /// Map from inode to entry (bounded by MAX_INODE_CACHE).
    inode_to_entry: RwLock<HashMap<u64, InodeEntry>>,
    /// Reverse map from path to inode for fast lookups.
    path_to_inode: RwLock<HashMap<String, u64>>,
}

impl InodeManager {
    /// Create a new inode manager with root directory initialized.
    pub fn new() -> Self {
        let mut inode_to_entry = HashMap::new();
        let mut path_to_inode = HashMap::new();

        // Root directory is always inode 1
        inode_to_entry.insert(
            ROOT_INODE,
            InodeEntry {
                path: String::new(),
                entry_type: EntryType::Directory,
                last_access: 0,
            },
        );
        path_to_inode.insert(String::new(), ROOT_INODE);

        Self {
            access_counter: AtomicU64::new(1),
            inode_to_entry: RwLock::new(inode_to_entry),
            path_to_inode: RwLock::new(path_to_inode),
        }
    }

    /// Get or allocate an inode for a given path.
    ///
    /// If the path is already cached, returns the existing inode.
    /// Otherwise, generates a stable inode from the path hash.
    ///
    /// # Panics
    ///
    /// Panics if internal RwLock is poisoned (indicates prior panic in critical section).
    /// Tiger Style: fail-fast on unrecoverable internal state corruption.
    pub fn get_or_create(&self, path: &str, entry_type: EntryType) -> u64 {
        // Fast path: check if already cached
        {
            let path_map = self
                .path_to_inode
                .read()
                .expect("inode manager path_to_inode lock poisoned");
            if let Some(&inode) = path_map.get(path) {
                // Update access time
                let access = self.access_counter.fetch_add(1, Ordering::Relaxed);
                if let Ok(mut entries) = self.inode_to_entry.write()
                    && let Some(entry) = entries.get_mut(&inode)
                {
                    entry.last_access = access;
                }
                return inode;
            }
        }

        // Slow path: allocate new inode
        let inode = self.hash_path(path);
        let access = self.access_counter.fetch_add(1, Ordering::Relaxed);

        let mut entries = self
            .inode_to_entry
            .write()
            .expect("inode manager inode_to_entry lock poisoned");
        let mut paths = self
            .path_to_inode
            .write()
            .expect("inode manager path_to_inode lock poisoned");

        // Evict old entries if at capacity
        if entries.len() >= MAX_INODE_CACHE {
            self.evict_lru(&mut entries, &mut paths);
        }

        // Insert new entry
        entries.insert(
            inode,
            InodeEntry {
                path: path.to_string(),
                entry_type,
                last_access: access,
            },
        );
        paths.insert(path.to_string(), inode);

        inode
    }

    /// Look up a path by its inode.
    ///
    /// # Panics
    ///
    /// Panics if internal RwLock is poisoned.
    pub fn get_path(&self, inode: u64) -> Option<InodeEntry> {
        let entries = self
            .inode_to_entry
            .read()
            .expect("inode manager inode_to_entry lock poisoned");
        entries.get(&inode).cloned()
    }

    /// Look up an inode by its path.
    ///
    /// # Panics
    ///
    /// Panics if internal RwLock is poisoned.
    pub fn get_inode(&self, path: &str) -> Option<u64> {
        let paths = self
            .path_to_inode
            .read()
            .expect("inode manager path_to_inode lock poisoned");
        paths.get(path).copied()
    }

    /// Remove an inode from the cache.
    ///
    /// # Panics
    ///
    /// Panics if internal RwLock is poisoned.
    #[allow(dead_code)]
    pub fn remove(&self, inode: u64) {
        let mut entries = self
            .inode_to_entry
            .write()
            .expect("inode manager inode_to_entry lock poisoned");
        let mut paths = self
            .path_to_inode
            .write()
            .expect("inode manager path_to_inode lock poisoned");

        if let Some(entry) = entries.remove(&inode) {
            paths.remove(&entry.path);
        }
    }

    /// Remove a path from the cache.
    ///
    /// # Panics
    ///
    /// Panics if internal RwLock is poisoned.
    pub fn remove_path(&self, path: &str) {
        let mut entries = self
            .inode_to_entry
            .write()
            .expect("inode manager inode_to_entry lock poisoned");
        let mut paths = self
            .path_to_inode
            .write()
            .expect("inode manager path_to_inode lock poisoned");

        if let Some(inode) = paths.remove(path) {
            entries.remove(&inode);
        }
    }

    /// Update the entry type for an inode (e.g., when a file becomes a directory).
    ///
    /// # Panics
    ///
    /// Panics if internal RwLock is poisoned.
    #[allow(dead_code)]
    pub fn update_type(&self, inode: u64, entry_type: EntryType) {
        let mut entries = self
            .inode_to_entry
            .write()
            .expect("inode manager inode_to_entry lock poisoned");
        if let Some(entry) = entries.get_mut(&inode) {
            entry.entry_type = entry_type;
        }
    }

    /// Generate a stable inode number from a path using blake3 hash.
    ///
    /// Uses the first 8 bytes of the hash, avoiding 0 and 1 (reserved).
    fn hash_path(&self, path: &str) -> u64 {
        let hash = blake3::hash(path.as_bytes());
        let bytes = hash.as_bytes();
        let mut inode = u64::from_le_bytes(bytes[0..8].try_into().unwrap());

        // Avoid reserved inodes (0 = invalid, 1 = root)
        if inode < 2 {
            inode = inode.wrapping_add(2);
        }

        inode
    }

    /// Evict the least recently used entry.
    fn evict_lru(&self, entries: &mut HashMap<u64, InodeEntry>, paths: &mut HashMap<String, u64>) {
        // Find entry with lowest access time (excluding root)
        let oldest = entries
            .iter()
            .filter(|&(&inode, _)| inode != ROOT_INODE)
            .min_by_key(|(_, entry)| entry.last_access)
            .map(|(&inode, _)| inode);

        if let Some(inode) = oldest
            && let Some(entry) = entries.remove(&inode)
        {
            paths.remove(&entry.path);
        }
    }

    /// Get the current cache size.
    ///
    /// # Panics
    ///
    /// Panics if internal RwLock is poisoned.
    #[allow(dead_code)]
    pub fn cache_size(&self) -> usize {
        self.inode_to_entry
            .read()
            .expect("inode manager inode_to_entry lock poisoned")
            .len()
    }
}

impl Default for InodeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_inode() {
        let mgr = InodeManager::new();
        let entry = mgr.get_path(ROOT_INODE).unwrap();
        assert_eq!(entry.path, "");
        assert_eq!(entry.entry_type, EntryType::Directory);
    }

    #[test]
    fn test_get_or_create() {
        let mgr = InodeManager::new();

        let inode1 = mgr.get_or_create("myapp/config", EntryType::Directory);
        let inode2 = mgr.get_or_create("myapp/config", EntryType::Directory);

        // Same path should return same inode
        assert_eq!(inode1, inode2);
        assert_ne!(inode1, ROOT_INODE);
    }

    #[test]
    fn test_different_paths() {
        let mgr = InodeManager::new();

        let inode1 = mgr.get_or_create("path/a", EntryType::File);
        let inode2 = mgr.get_or_create("path/b", EntryType::File);

        // Different paths should have different inodes
        assert_ne!(inode1, inode2);
    }

    #[test]
    fn test_remove() {
        let mgr = InodeManager::new();

        let inode = mgr.get_or_create("test/path", EntryType::File);
        assert!(mgr.get_path(inode).is_some());

        mgr.remove(inode);
        assert!(mgr.get_path(inode).is_none());
    }

    #[test]
    fn test_remove_path() {
        let mgr = InodeManager::new();

        let path = "test/path";
        let inode = mgr.get_or_create(path, EntryType::File);
        assert!(mgr.get_inode(path).is_some());

        mgr.remove_path(path);
        assert!(mgr.get_inode(path).is_none());
        assert!(mgr.get_path(inode).is_none());
    }

    // === Additional Tests ===

    #[test]
    fn test_default_creates_new() {
        let mgr = InodeManager::default();
        let entry = mgr.get_path(ROOT_INODE).unwrap();
        assert_eq!(entry.path, "");
    }

    #[test]
    fn test_get_inode_returns_none_for_unknown() {
        let mgr = InodeManager::new();
        assert!(mgr.get_inode("nonexistent/path").is_none());
    }

    #[test]
    fn test_get_path_returns_none_for_unknown() {
        let mgr = InodeManager::new();
        assert!(mgr.get_path(99999).is_none());
    }

    #[test]
    fn test_update_type() {
        let mgr = InodeManager::new();

        // Create as file
        let inode = mgr.get_or_create("test/path", EntryType::File);
        let entry = mgr.get_path(inode).unwrap();
        assert_eq!(entry.entry_type, EntryType::File);

        // Update to directory
        mgr.update_type(inode, EntryType::Directory);
        let entry = mgr.get_path(inode).unwrap();
        assert_eq!(entry.entry_type, EntryType::Directory);
    }

    #[test]
    fn test_update_type_unknown_inode() {
        let mgr = InodeManager::new();
        // Should not panic
        mgr.update_type(99999, EntryType::File);
    }

    #[test]
    fn test_cache_size() {
        let mgr = InodeManager::new();

        // Initially just root
        assert_eq!(mgr.cache_size(), 1);

        // Add some entries
        mgr.get_or_create("path/a", EntryType::File);
        mgr.get_or_create("path/b", EntryType::File);
        mgr.get_or_create("path/c", EntryType::File);

        assert_eq!(mgr.cache_size(), 4);
    }

    #[test]
    fn test_inode_never_zero_or_one() {
        let mgr = InodeManager::new();

        // Test many paths to ensure we never get reserved inodes
        for i in 0..1000 {
            let path = format!("test/path/{}", i);
            let inode = mgr.get_or_create(&path, EntryType::File);
            assert!(
                inode >= 2 || inode == ROOT_INODE,
                "Got reserved inode: {}",
                inode
            );
        }
    }

    #[test]
    fn test_access_time_updates() {
        let mgr = InodeManager::new();

        // Create entry
        let inode = mgr.get_or_create("test/path", EntryType::File);
        let entry1 = mgr.get_path(inode).unwrap();
        let access1 = entry1.last_access;

        // Access again (should update last_access)
        let _ = mgr.get_or_create("test/path", EntryType::File);
        let entry2 = mgr.get_path(inode).unwrap();
        let access2 = entry2.last_access;

        assert!(access2 > access1);
    }

    #[test]
    fn test_entry_type_equality() {
        assert_eq!(EntryType::File, EntryType::File);
        assert_eq!(EntryType::Directory, EntryType::Directory);
        assert_ne!(EntryType::File, EntryType::Directory);
    }

    #[test]
    fn test_entry_type_clone() {
        let entry_type = EntryType::File;
        let cloned = entry_type.clone();
        assert_eq!(entry_type, cloned);
    }

    #[test]
    fn test_entry_type_copy() {
        let entry_type = EntryType::Directory;
        let copied: EntryType = entry_type; // Copy trait
        assert_eq!(entry_type, copied);
    }

    #[test]
    fn test_inode_entry_clone() {
        let entry = InodeEntry {
            path: "test/path".to_string(),
            entry_type: EntryType::File,
            last_access: 42,
        };
        let cloned = entry.clone();
        assert_eq!(entry.path, cloned.path);
        assert_eq!(entry.entry_type, cloned.entry_type);
        assert_eq!(entry.last_access, cloned.last_access);
    }

    #[test]
    fn test_hash_deterministic() {
        let mgr = InodeManager::new();

        // Hash same path multiple times
        let inode1 = mgr.hash_path("consistent/path");
        let inode2 = mgr.hash_path("consistent/path");

        assert_eq!(inode1, inode2);
    }

    #[test]
    fn test_remove_root_inode_still_works() {
        let mgr = InodeManager::new();

        // Try to remove root (should work but re-lookup via path will fail)
        mgr.remove(ROOT_INODE);

        // Root should be gone
        assert!(mgr.get_path(ROOT_INODE).is_none());
    }

    #[test]
    fn test_remove_nonexistent_inode() {
        let mgr = InodeManager::new();
        // Should not panic
        mgr.remove(99999);
    }

    #[test]
    fn test_remove_nonexistent_path() {
        let mgr = InodeManager::new();
        // Should not panic
        mgr.remove_path("nonexistent/path");
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let mgr = Arc::new(InodeManager::new());
        let mut handles = vec![];

        // Spawn multiple threads accessing the manager
        for i in 0..10 {
            let mgr_clone = mgr.clone();
            let handle = thread::spawn(move || {
                for j in 0..100 {
                    let path = format!("thread{}/path{}", i, j);
                    mgr_clone.get_or_create(&path, EntryType::File);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Should have 1000 entries + root
        assert_eq!(mgr.cache_size(), 1001);
    }

    #[test]
    fn test_lru_eviction() {
        // Create a manager with many entries to trigger eviction
        let mgr = InodeManager::new();

        // Fill cache to capacity (minus root)
        for i in 0..(MAX_INODE_CACHE - 1) {
            let path = format!("eviction/path/{}", i);
            mgr.get_or_create(&path, EntryType::File);
        }

        // Cache should be at capacity
        assert_eq!(mgr.cache_size(), MAX_INODE_CACHE);

        // Add one more entry (should trigger eviction)
        mgr.get_or_create("new/path", EntryType::File);

        // Cache should still be at capacity
        assert_eq!(mgr.cache_size(), MAX_INODE_CACHE);
    }

    #[test]
    fn test_root_never_evicted() {
        let mgr = InodeManager::new();

        // Fill cache beyond capacity
        for i in 0..(MAX_INODE_CACHE + 10) {
            let path = format!("eviction/path/{}", i);
            mgr.get_or_create(&path, EntryType::File);
        }

        // Root should still exist
        assert!(mgr.get_path(ROOT_INODE).is_some());
    }
}
