//! Pristine management for Pijul repositories.
//!
//! This module wraps libpijul's sanakirja-backed pristine storage,
//! providing a safe interface for managing the internal file graph
//! representation of Pijul repositories.
//!
//! ## Architecture
//!
//! Each Pijul repository has its own sanakirja database stored at:
//! ```text
//! {data_dir}/pijul/{repo_id}/pristine/
//! ```
//!
//! The pristine stores:
//! - The file graph (vertices and edges representing file contents)
//! - Channel metadata (which patches are applied to each channel)
//! - Inode mappings (filesystem paths to internal identifiers)
//!
//! ## Thread Safety
//!
//! The `Pristine` type from libpijul is `Send + Sync`, but transactions
//! are not. This module provides a `PristineHandle` that manages
//! transaction lifecycle safely.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use libpijul::pristine::sanakirja::{MutTxn, Pristine, Txn};
use libpijul::pristine::{Base32, ChannelRef, ChannelTxnT, Hash as PijulHash, MutTxnT, TxnT};
use libpijul::{ArcTxn, TxnTExt};
use parking_lot::RwLock;
use tracing::{debug, info, instrument};

use crate::forge::identity::RepoId;

use super::constants::{MAX_PRISTINE_SIZE_BYTES, PRISTINE_CACHE_SIZE};
use super::error::{PijulError, PijulResult};

// ============================================================================
// PristineManager
// ============================================================================

/// Manages pristine databases for multiple Pijul repositories.
///
/// The PristineManager maintains a cache of open pristine databases
/// to avoid repeated open/close overhead. It provides safe access
/// to repository state through transaction-based APIs.
///
/// # Example
///
/// ```ignore
/// let manager = PristineManager::new("/var/lib/aspen");
///
/// // Open or create a pristine for a repository
/// let handle = manager.open_or_create(&repo_id)?;
///
/// // Start a mutable transaction
/// let txn = handle.mut_txn_begin()?;
/// let channel = txn.open_or_create_channel("main")?;
/// // ... perform operations ...
/// txn.commit()?;
/// ```
pub struct PristineManager {
    /// Base directory for all pristine databases.
    data_dir: PathBuf,

    /// Cache of open pristine databases (keyed by repo_id hex).
    cache: RwLock<HashMap<String, Arc<Pristine>>>,

    /// Maximum number of cached pristines.
    max_cached: usize,
}

impl PristineManager {
    /// Create a new PristineManager.
    ///
    /// # Arguments
    ///
    /// - `data_dir`: Base directory for storing pristine databases.
    ///   Each repository's pristine will be stored at `{data_dir}/pijul/{repo_id}/pristine/`.
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            cache: RwLock::new(HashMap::new()),
            max_cached: PRISTINE_CACHE_SIZE,
        }
    }

    /// Get the path to a repository's pristine directory.
    pub fn pristine_dir(&self, repo_id: &RepoId) -> PathBuf {
        self.data_dir
            .join("pijul")
            .join(repo_id.to_string())
            .join("pristine")
    }

    /// Get the path to a repository's pristine database file.
    ///
    /// Sanakirja stores the database as a single file called "db"
    /// inside the pristine directory.
    pub fn pristine_path(&self, repo_id: &RepoId) -> PathBuf {
        self.pristine_dir(repo_id).join("db")
    }

    /// Check if a pristine exists for a repository.
    pub fn exists(&self, repo_id: &RepoId) -> bool {
        self.pristine_path(repo_id).exists()
    }

    /// Open or create a pristine database for a repository.
    ///
    /// If the pristine already exists, it is opened. Otherwise, a new
    /// empty pristine is created.
    ///
    /// # Errors
    ///
    /// Returns an error if the pristine cannot be opened or created.
    #[instrument(skip(self))]
    pub fn open_or_create(&self, repo_id: &RepoId) -> PijulResult<PristineHandle> {
        let key = repo_id.to_string();

        // Check cache first
        {
            let cache = self.cache.read();
            if let Some(pristine) = cache.get(&key) {
                debug!(repo_id = %repo_id, "pristine cache hit");
                return Ok(PristineHandle {
                    pristine: Arc::clone(pristine),
                    repo_id: *repo_id,
                });
            }
        }

        // Create directory if needed
        let dir = self.pristine_dir(repo_id);
        std::fs::create_dir_all(&dir).map_err(|e| PijulError::PristineStorage {
            message: format!("failed to create pristine directory: {}", e),
        })?;

        // Open or create the pristine database file with a reasonable size
        let path = self.pristine_path(repo_id);
        let pristine = Pristine::new_with_size(&path, MAX_PRISTINE_SIZE_BYTES).map_err(|e| {
            PijulError::PristineStorage {
                message: format!("failed to open pristine: {}", e),
            }
        })?;

        let pristine = Arc::new(pristine);

        // Add to cache (evict oldest if needed)
        {
            let mut cache = self.cache.write();
            if cache.len() >= self.max_cached {
                // Simple eviction: remove an arbitrary entry
                // A more sophisticated LRU could be used here
                if let Some(old_key) = cache.keys().next().cloned() {
                    cache.remove(&old_key);
                    debug!(evicted = %old_key, "evicted pristine from cache");
                }
            }
            cache.insert(key.clone(), Arc::clone(&pristine));
        }

        info!(repo_id = %repo_id, path = %path.display(), "opened pristine");

        Ok(PristineHandle {
            pristine,
            repo_id: *repo_id,
        })
    }

    /// Open an existing pristine database.
    ///
    /// Unlike `open_or_create`, this returns an error if the pristine
    /// does not exist.
    #[instrument(skip(self))]
    pub fn open(&self, repo_id: &RepoId) -> PijulResult<PristineHandle> {
        if !self.exists(repo_id) {
            return Err(PijulError::PristineNotInitialized {
                repo_id: repo_id.to_string(),
            });
        }
        self.open_or_create(repo_id)
    }

    /// Remove a pristine from the cache.
    ///
    /// This does not delete the on-disk data, just removes it from
    /// the in-memory cache.
    pub fn evict(&self, repo_id: &RepoId) {
        let key = repo_id.to_string();
        let mut cache = self.cache.write();
        cache.remove(&key);
        debug!(repo_id = %repo_id, "evicted pristine from cache");
    }

    /// Delete a repository's pristine database entirely.
    ///
    /// This removes both the in-memory cache entry and the on-disk data.
    /// Use with caution as this is irreversible.
    #[instrument(skip(self))]
    pub fn delete(&self, repo_id: &RepoId) -> PijulResult<()> {
        // Remove from cache
        self.evict(repo_id);

        // Delete on-disk data (delete the entire pristine directory)
        let dir = self.pristine_dir(repo_id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|e| PijulError::PristineStorage {
                message: format!("failed to delete pristine: {}", e),
            })?;
            info!(repo_id = %repo_id, path = %dir.display(), "deleted pristine");
        }

        Ok(())
    }

    /// Get the number of cached pristines.
    pub fn cache_size(&self) -> usize {
        self.cache.read().len()
    }

    /// Clear all cached pristines.
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write();
        cache.clear();
        debug!("cleared pristine cache");
    }
}

// ============================================================================
// PristineHandle
// ============================================================================

/// A handle to an open pristine database.
///
/// The handle provides access to the underlying sanakirja database
/// and can be used to start read or write transactions.
///
/// # Thread Safety
///
/// While the `Pristine` is thread-safe, transactions are not.
/// Each transaction should be used from a single thread.
#[derive(Clone)]
pub struct PristineHandle {
    pristine: Arc<Pristine>,
    repo_id: RepoId,
}

impl PristineHandle {
    /// Get the repository ID this handle is for.
    pub fn repo_id(&self) -> &RepoId {
        &self.repo_id
    }

    /// Start a read-only transaction.
    ///
    /// Read transactions can be used to query the pristine state
    /// without modifying it.
    pub fn txn_begin(&self) -> PijulResult<ReadTxn> {
        let txn = self.pristine.txn_begin().map_err(|e| PijulError::PristineStorage {
            message: format!("failed to begin transaction: {}", e),
        })?;

        Ok(ReadTxn {
            txn,
            repo_id: self.repo_id,
        })
    }

    /// Start a mutable transaction.
    ///
    /// Mutable transactions allow modifying the pristine state.
    /// Changes are only persisted when `commit()` is called.
    pub fn mut_txn_begin(&self) -> PijulResult<WriteTxn> {
        let txn = self.pristine.mut_txn_begin().map_err(|e| PijulError::PristineStorage {
            message: format!("failed to begin mutable transaction: {}", e),
        })?;

        Ok(WriteTxn {
            txn,
            repo_id: self.repo_id,
        })
    }

    /// Start an arc-wrapped mutable transaction.
    ///
    /// Arc-wrapped transactions are needed for libpijul's record API
    /// which requires shared ownership of the transaction across
    /// multiple operations.
    pub fn arc_txn_begin(&self) -> PijulResult<ArcTxn<MutTxn<()>>> {
        self.pristine.arc_txn_begin().map_err(|e| PijulError::PristineStorage {
            message: format!("failed to begin arc transaction: {}", e),
        })
    }

    /// Get a reference to the underlying pristine.
    ///
    /// This is exposed for advanced use cases where direct access
    /// to the libpijul Pristine is needed.
    pub fn pristine(&self) -> &Arc<Pristine> {
        &self.pristine
    }

    /// Compute the difference between two channels.
    ///
    /// Returns information about changes that are in channel2 but not in channel1.
    /// This is useful for comparing branches to see what would be merged.
    pub fn diff_channels(
        &self,
        channel1: &str,
        channel2: &str,
    ) -> PijulResult<crate::pijul::record::DiffResult> {
        use crate::pijul::record::{DiffHunkInfo, DiffResult};

        let txn = self.txn_begin()?;

        // Load both channels
        let ch1 = txn.load_channel(channel1)?
            .ok_or_else(|| PijulError::ChannelNotFound {
                channel: channel1.to_string(),
            })?;

        let ch2 = txn.load_channel(channel2)?
            .ok_or_else(|| PijulError::ChannelNotFound {
                channel: channel2.to_string(),
            })?;

        // Get the changes in each channel
        let ch1_guard = ch1.read();
        let ch2_guard = ch2.read();

        // Collect change hashes from channel1
        let mut ch1_changes = std::collections::HashSet::new();
        let log1 = txn.txn().log(&ch1_guard, 0u64).map_err(|e| PijulError::PristineStorage {
            message: format!("failed to read log for channel1: {:?}", e),
        })?;
        for change_result in log1 {
            if let Ok((_, (h, _))) = change_result {
                ch1_changes.insert(h);
            }
        }

        // Find changes in channel2 that are not in channel1
        let mut hunks = Vec::new();
        let log2 = txn.txn().log(&ch2_guard, 0u64).map_err(|e| PijulError::PristineStorage {
            message: format!("failed to read log for channel2: {:?}", e),
        })?;
        for change_result in log2 {
            if let Ok((_, (h, _))) = change_result {
                if !ch1_changes.contains(&h) {
                    // This change is in channel2 but not channel1
                    // Convert SerializedHash to Hash for base32 display
                    let hash: PijulHash = h.into();
                    hunks.push(DiffHunkInfo {
                        path: format!("change:{}", hash.to_base32()),
                        kind: "add".to_string(),
                        additions: 0,
                        deletions: 0,
                    });
                }
            }
        }

        let num_hunks = hunks.len();
        Ok(DiffResult { hunks, num_hunks })
    }
}

// ============================================================================
// ReadTxn
// ============================================================================

/// A read-only transaction on a pristine database.
///
/// Provides read access to channels, file state, and other
/// repository data.
pub struct ReadTxn {
    txn: Txn,
    repo_id: RepoId,
}

impl ReadTxn {
    /// Get the repository ID.
    pub fn repo_id(&self) -> &RepoId {
        &self.repo_id
    }

    /// Load a channel by name.
    ///
    /// Returns `None` if the channel does not exist.
    pub fn load_channel(&self, name: &str) -> PijulResult<Option<ChannelRef<Txn>>> {
        self.txn.load_channel(name).map_err(|e| PijulError::PristineStorage {
            message: format!("failed to load channel '{}': {}", name, e),
        })
    }

    /// List all channels in the repository.
    pub fn list_channels(&self) -> PijulResult<Vec<String>> {
        let channel_refs = self.txn.channels("").map_err(|e| PijulError::PristineStorage {
            message: format!("failed to list channels: {:?}", e),
        })?;

        let mut channels = Vec::with_capacity(channel_refs.len());
        for channel_ref in &channel_refs {
            let channel = channel_ref.read();
            let name = self.txn.name(&channel);
            channels.push(name.to_string());
        }
        Ok(channels)
    }

    /// Check if a channel exists.
    pub fn channel_exists(&self, name: &str) -> PijulResult<bool> {
        Ok(self.load_channel(name)?.is_some())
    }

    /// Get a reference to the underlying transaction.
    ///
    /// For advanced use cases requiring direct libpijul access.
    pub fn txn(&self) -> &Txn {
        &self.txn
    }
}

// ============================================================================
// WriteTxn
// ============================================================================

/// A mutable transaction on a pristine database.
///
/// Allows modifying the pristine state. Changes must be committed
/// with `commit()` to be persisted.
pub struct WriteTxn {
    txn: MutTxn<()>,
    repo_id: RepoId,
}

impl WriteTxn {
    /// Get the repository ID.
    pub fn repo_id(&self) -> &RepoId {
        &self.repo_id
    }

    /// Open or create a channel.
    ///
    /// If the channel exists, it is returned. Otherwise, a new empty
    /// channel is created.
    pub fn open_or_create_channel(
        &mut self,
        name: &str,
    ) -> PijulResult<ChannelRef<MutTxn<()>>> {
        self.txn.open_or_create_channel(name).map_err(|e| {
            PijulError::PristineStorage {
                message: format!("failed to open/create channel '{}': {}", name, e),
            }
        })
    }

    /// Load a channel by name.
    ///
    /// Returns `None` if the channel does not exist.
    pub fn load_channel(&self, name: &str) -> PijulResult<Option<ChannelRef<MutTxn<()>>>> {
        self.txn.load_channel(name).map_err(|e| PijulError::PristineStorage {
            message: format!("failed to load channel '{}': {}", name, e),
        })
    }

    /// Fork a channel.
    ///
    /// Creates a new channel with the same state as the source channel.
    pub fn fork_channel(&mut self, source: &str, dest: &str) -> PijulResult<ChannelRef<MutTxn<()>>> {
        let source_channel = self.load_channel(source)?.ok_or_else(|| PijulError::ChannelNotFound {
            channel: source.to_string(),
        })?;

        self.txn.fork(&source_channel, dest).map_err(|e| {
            PijulError::PristineStorage {
                message: format!("failed to fork channel '{}' to '{}': {}", source, dest, e),
            }
        })
    }

    /// Rename a channel.
    pub fn rename_channel(&mut self, old_name: &str, new_name: &str) -> PijulResult<()> {
        let mut channel = self.load_channel(old_name)?.ok_or_else(|| PijulError::ChannelNotFound {
            channel: old_name.to_string(),
        })?;

        self.txn.rename_channel(&mut channel, new_name).map_err(|e| {
            PijulError::PristineStorage {
                message: format!("failed to rename channel '{}' to '{}': {}", old_name, new_name, e),
            }
        })
    }

    /// Delete a channel.
    pub fn drop_channel(&mut self, name: &str) -> PijulResult<bool> {
        self.txn.drop_channel(name).map_err(|e| PijulError::PristineStorage {
            message: format!("failed to drop channel '{}': {}", name, e),
        })
    }

    /// List all channels in the repository.
    pub fn list_channels(&self) -> PijulResult<Vec<String>> {
        let channel_refs = self.txn.channels("").map_err(|e| PijulError::PristineStorage {
            message: format!("failed to list channels: {:?}", e),
        })?;

        let mut channels = Vec::with_capacity(channel_refs.len());
        for channel_ref in &channel_refs {
            let channel = channel_ref.read();
            let name = self.txn.name(&channel);
            channels.push(name.to_string());
        }
        Ok(channels)
    }

    /// Commit the transaction.
    ///
    /// This persists all changes made in this transaction.
    /// After commit, the transaction is consumed.
    pub fn commit(self) -> PijulResult<()> {
        self.txn.commit().map_err(|e| PijulError::PristineStorage {
            message: format!("failed to commit transaction: {}", e),
        })
    }

    /// Get a mutable reference to the underlying transaction.
    ///
    /// For advanced use cases requiring direct libpijul access.
    pub fn txn_mut(&mut self) -> &mut MutTxn<()> {
        &mut self.txn
    }

    /// Get a reference to the underlying transaction.
    pub fn txn(&self) -> &MutTxn<()> {
        &self.txn
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_repo_id() -> RepoId {
        RepoId([1u8; 32])
    }

    #[test]
    fn test_pristine_manager_new() {
        let tmp = TempDir::new().unwrap();
        let manager = PristineManager::new(tmp.path());
        assert_eq!(manager.cache_size(), 0);
    }

    #[test]
    fn test_open_or_create_pristine() {
        let tmp = TempDir::new().unwrap();
        let manager = PristineManager::new(tmp.path());
        let repo_id = test_repo_id();

        // Should not exist initially
        assert!(!manager.exists(&repo_id));

        // Open or create should succeed
        let handle = manager.open_or_create(&repo_id).unwrap();
        assert_eq!(handle.repo_id(), &repo_id);

        // Now it should exist
        assert!(manager.exists(&repo_id));

        // Should be cached
        assert_eq!(manager.cache_size(), 1);
    }

    #[test]
    fn test_open_nonexistent_fails() {
        let tmp = TempDir::new().unwrap();
        let manager = PristineManager::new(tmp.path());
        let repo_id = test_repo_id();

        let result = manager.open(&repo_id);
        assert!(matches!(result, Err(PijulError::PristineNotInitialized { .. })));
    }

    #[test]
    fn test_create_and_list_channels() {
        let tmp = TempDir::new().unwrap();
        let manager = PristineManager::new(tmp.path());
        let repo_id = test_repo_id();

        let handle = manager.open_or_create(&repo_id).unwrap();

        // Create some channels
        {
            let mut txn = handle.mut_txn_begin().unwrap();
            let _main = txn.open_or_create_channel("main").unwrap();
            let _dev = txn.open_or_create_channel("develop").unwrap();
            txn.commit().unwrap();
        }

        // List channels
        {
            let txn = handle.txn_begin().unwrap();
            let channels = txn.list_channels().unwrap();
            assert!(channels.contains(&"main".to_string()));
            assert!(channels.contains(&"develop".to_string()));
        }
    }

    #[test]
    fn test_fork_channel() {
        let tmp = TempDir::new().unwrap();
        let manager = PristineManager::new(tmp.path());
        let repo_id = test_repo_id();

        let handle = manager.open_or_create(&repo_id).unwrap();

        // Create main and fork to feature
        {
            let mut txn = handle.mut_txn_begin().unwrap();
            let _main = txn.open_or_create_channel("main").unwrap();
            let _feature = txn.fork_channel("main", "feature/test").unwrap();
            txn.commit().unwrap();
        }

        // Verify both exist
        {
            let txn = handle.txn_begin().unwrap();
            assert!(txn.channel_exists("main").unwrap());
            assert!(txn.channel_exists("feature/test").unwrap());
        }
    }

    #[test]
    fn test_delete_channel() {
        let tmp = TempDir::new().unwrap();
        let manager = PristineManager::new(tmp.path());
        let repo_id = test_repo_id();

        let handle = manager.open_or_create(&repo_id).unwrap();

        // Create and delete a channel
        {
            let mut txn = handle.mut_txn_begin().unwrap();
            let _ch = txn.open_or_create_channel("temp").unwrap();
            txn.commit().unwrap();
        }

        {
            let mut txn = handle.mut_txn_begin().unwrap();
            let deleted = txn.drop_channel("temp").unwrap();
            assert!(deleted);
            txn.commit().unwrap();
        }

        // Verify gone
        {
            let txn = handle.txn_begin().unwrap();
            assert!(!txn.channel_exists("temp").unwrap());
        }
    }

    #[test]
    fn test_delete_pristine() {
        let tmp = TempDir::new().unwrap();
        let manager = PristineManager::new(tmp.path());
        let repo_id = test_repo_id();

        // Create
        let _ = manager.open_or_create(&repo_id).unwrap();
        assert!(manager.exists(&repo_id));

        // Delete
        manager.delete(&repo_id).unwrap();
        assert!(!manager.exists(&repo_id));
        assert_eq!(manager.cache_size(), 0);
    }

    #[test]
    fn test_cache_eviction() {
        let tmp = TempDir::new().unwrap();
        let mut manager = PristineManager::new(tmp.path());
        manager.max_cached = 2; // Small cache for testing

        // Create 3 repos, should evict one
        for i in 0u8..3 {
            let repo_id = RepoId([i; 32]);
            let _ = manager.open_or_create(&repo_id).unwrap();
        }

        // Cache should only have 2 entries
        assert_eq!(manager.cache_size(), 2);
    }
}
