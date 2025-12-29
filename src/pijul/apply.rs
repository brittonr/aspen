//! Change application for Pijul repositories.
//!
//! This module provides utilities for applying changes to Pijul repositories
//! using libpijul's change application machinery combined with Aspen's
//! distributed storage.
//!
//! ## Architecture
//!
//! Change application involves three components:
//!
//! 1. **Pristine** (sanakirja): The local file graph database
//! 2. **Change Storage** (iroh-blobs): Distributed change storage
//! 3. **Channel Heads** (Raft KV): Consensus on branch state
//!
//! The workflow for applying a change is:
//! 1. Load change bytes from iroh-blobs
//! 2. Parse into libpijul's Change type
//! 3. Apply to the pristine using libpijul::apply_change
//! 4. Update channel head in Raft KV

use std::path::PathBuf;
use std::sync::Arc;

use libpijul::changestore::filesystem::FileSystem as LibpijulFileSystem;
use libpijul::pristine::{Hash, Merkle};
use libpijul::MutTxnTExt;
use tracing::{debug, info, instrument};

use crate::blob::BlobStore;
use crate::forge::identity::RepoId;

use super::change_store::AspenChangeStore;
use super::error::{PijulError, PijulResult};
use super::pristine::PristineHandle;
use super::types::ChangeHash;

// ============================================================================
// ChangeDirectory
// ============================================================================

/// A directory-based change store that bridges iroh-blobs with libpijul.
///
/// This struct manages a local directory where changes are cached from
/// iroh-blobs before being applied to the pristine. It uses libpijul's
/// built-in FileSystem ChangeStore for the actual change loading.
///
/// ## Directory Structure
///
/// ```text
/// {data_dir}/pijul/{repo_id}/changes/
/// ├── AB/
/// │   └── CDEF...hash.change
/// └── XY/
///     └── Z123...hash.change
/// ```
pub struct ChangeDirectory<B: BlobStore> {
    /// Base directory for changes.
    base_dir: PathBuf,

    /// The iroh-blobs backed change store for fetching.
    blobs: Arc<AspenChangeStore<B>>,

    /// Repository ID (reserved for future use in logging/debugging).
    #[allow(dead_code)]
    repo_id: RepoId,
}

impl<B: BlobStore> ChangeDirectory<B> {
    /// Create a new change directory for a repository.
    pub fn new(data_dir: &PathBuf, repo_id: RepoId, blobs: Arc<AspenChangeStore<B>>) -> Self {
        let base_dir = data_dir.join("pijul").join(repo_id.to_string()).join("changes");
        Self {
            base_dir,
            blobs,
            repo_id,
        }
    }

    /// Get the changes directory path.
    pub fn changes_dir(&self) -> &PathBuf {
        &self.base_dir
    }

    /// Ensure the changes directory exists.
    pub fn ensure_dir(&self) -> PijulResult<()> {
        std::fs::create_dir_all(&self.base_dir).map_err(|e| PijulError::Io {
            message: format!("failed to create changes directory: {}", e),
        })
    }

    /// Get the path where a change would be stored.
    ///
    /// Uses the same layout as libpijul's FileSystem ChangeStore:
    /// `{changes_dir}/{first_2_chars}/{rest}.change`
    fn change_path(&self, hash: &ChangeHash) -> PathBuf {
        let hex = hash.to_hex();
        let (prefix, suffix) = hex.split_at(2.min(hex.len()));
        self.base_dir.join(prefix).join(format!("{}.change", suffix))
    }

    /// Check if a change exists locally.
    pub fn has_change(&self, hash: &ChangeHash) -> bool {
        self.change_path(hash).exists()
    }

    /// Fetch a change from iroh-blobs and cache it locally.
    #[instrument(skip(self))]
    pub async fn fetch_change(&self, hash: &ChangeHash) -> PijulResult<PathBuf> {
        let path = self.change_path(hash);

        // Check if already cached
        if path.exists() {
            debug!(hash = %hash, path = %path.display(), "change already cached");
            return Ok(path);
        }

        // Fetch from blob storage
        let bytes = self
            .blobs
            .get_change(hash)
            .await?
            .ok_or_else(|| PijulError::ChangeNotFound {
                hash: hash.to_hex(),
            })?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| PijulError::Io {
                message: format!("failed to create change directory: {}", e),
            })?;
        }

        // Write to disk
        std::fs::write(&path, &bytes).map_err(|e| PijulError::Io {
            message: format!("failed to write change file: {}", e),
        })?;

        info!(hash = %hash, path = %path.display(), size = bytes.len(), "cached change from blobs");
        Ok(path)
    }

    /// Store a change locally and upload to iroh-blobs.
    #[instrument(skip(self, bytes))]
    pub async fn store_change(&self, bytes: &[u8]) -> PijulResult<ChangeHash> {
        // Store in blob storage first to get the hash
        let hash = self.blobs.store_change(bytes).await?;

        // Cache locally
        let path = self.change_path(&hash);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| PijulError::Io {
                message: format!("failed to create change directory: {}", e),
            })?;
        }

        std::fs::write(&path, bytes).map_err(|e| PijulError::Io {
            message: format!("failed to write change file: {}", e),
        })?;

        info!(hash = %hash, path = %path.display(), size = bytes.len(), "stored change");
        Ok(hash)
    }

    /// Get the libpijul FileSystem ChangeStore for this repository.
    ///
    /// Note: This should only be called after ensure_dir() has been called.
    pub fn libpijul_store(&self) -> LibpijulFileSystem {
        // from_changes takes the path and a cache capacity
        LibpijulFileSystem::from_changes(self.base_dir.clone(), 100)
    }

    /// Convert an Aspen ChangeHash to a libpijul Hash.
    pub fn to_pijul_hash(hash: &ChangeHash) -> Hash {
        Hash::Blake3(*hash.as_bytes())
    }

    /// Convert a libpijul Hash to an Aspen ChangeHash.
    pub fn from_pijul_hash(hash: &Hash) -> Option<ChangeHash> {
        match hash {
            Hash::Blake3(bytes) => Some(ChangeHash(*bytes)),
            _ => None,
        }
    }
}

// ============================================================================
// ChangeApplicator
// ============================================================================

/// Applies changes to a Pijul repository.
///
/// This struct combines the pristine, change directory, and transaction
/// management to provide a high-level API for change application.
///
/// # Example
///
/// ```ignore
/// let applicator = ChangeApplicator::new(pristine_handle, change_dir);
///
/// // Fetch and apply a change
/// applicator.fetch_and_apply("main", &change_hash).await?;
/// ```
pub struct ChangeApplicator<B: BlobStore> {
    /// Handle to the pristine database.
    pristine: PristineHandle,

    /// Change directory for caching.
    changes: ChangeDirectory<B>,
}

impl<B: BlobStore> ChangeApplicator<B> {
    /// Create a new change applicator.
    pub fn new(pristine: PristineHandle, changes: ChangeDirectory<B>) -> Self {
        Self { pristine, changes }
    }

    /// Apply a change that's already cached locally.
    ///
    /// The change must exist in the change directory before calling this.
    #[instrument(skip(self))]
    pub fn apply_local(&self, channel: &str, hash: &ChangeHash) -> PijulResult<ApplyResult> {
        // Get the libpijul change store
        let store = self.changes.libpijul_store();

        // Convert hash
        let pijul_hash = ChangeDirectory::<B>::to_pijul_hash(hash);

        // Start a mutable transaction
        let mut txn = self.pristine.mut_txn_begin()?;

        // Open or create the channel
        let channel_ref = txn.open_or_create_channel(channel)?;

        // Apply the change using MutTxnTExt trait method
        // ChannelRef uses interior mutability, so we need to get a write guard
        let (n, merkle) = {
            let txn_inner = txn.txn_mut();
            let mut channel_guard = channel_ref.write();
            txn_inner
                .apply_change(&store, &mut channel_guard, &pijul_hash)
                .map_err(|e| PijulError::ApplyFailed {
                    message: format!("{:?}", e),
                })?
        };

        // Commit the transaction
        txn.commit()?;

        info!(channel = channel, hash = %hash, n = n, "applied change");

        Ok(ApplyResult {
            changes_applied: n,
            merkle,
        })
    }

    /// Fetch a change from iroh-blobs and apply it.
    #[instrument(skip(self))]
    pub async fn fetch_and_apply(&self, channel: &str, hash: &ChangeHash) -> PijulResult<ApplyResult> {
        // Ensure changes directory exists
        self.changes.ensure_dir()?;

        // Fetch the change
        self.changes.fetch_change(hash).await?;

        // Apply it
        self.apply_local(channel, hash)
    }

    /// Apply multiple changes in order.
    ///
    /// Changes are applied in the order provided. If any change fails to apply,
    /// the operation stops and returns an error.
    #[instrument(skip(self, hashes))]
    pub async fn apply_changes(
        &self,
        channel: &str,
        hashes: &[ChangeHash],
    ) -> PijulResult<Vec<ApplyResult>> {
        let mut results = Vec::with_capacity(hashes.len());

        for hash in hashes {
            let result = self.fetch_and_apply(channel, hash).await?;
            results.push(result);
        }

        info!(channel = channel, count = results.len(), "applied changes");
        Ok(results)
    }
}

// ============================================================================
// Result Types
// ============================================================================

/// Result of applying a change.
#[derive(Debug, Clone)]
pub struct ApplyResult {
    /// Number of operations applied.
    pub changes_applied: u64,

    /// Merkle hash after application.
    pub merkle: Merkle,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blob::InMemoryBlobStore;
    use tempfile::TempDir;

    fn test_repo_id() -> RepoId {
        RepoId([1u8; 32])
    }

    #[test]
    fn test_change_path() {
        let tmp = TempDir::new().unwrap();
        let blobs = Arc::new(InMemoryBlobStore::new());
        let change_store = Arc::new(AspenChangeStore::new(blobs));
        let dir = ChangeDirectory::new(&tmp.path().to_path_buf(), test_repo_id(), change_store);

        let hash = ChangeHash([0xAB, 0xCD, 0xEF] .iter().cloned().chain(std::iter::repeat(0)).take(32).collect::<Vec<_>>().try_into().unwrap());
        let path = dir.change_path(&hash);

        // Should have the prefix directory structure
        assert!(path.to_string_lossy().contains("ab"));
        assert!(path.to_string_lossy().ends_with(".change"));
    }

    #[test]
    fn test_hash_conversion() {
        let aspen_hash = ChangeHash([42u8; 32]);
        let pijul_hash = ChangeDirectory::<InMemoryBlobStore>::to_pijul_hash(&aspen_hash);

        match pijul_hash {
            Hash::Blake3(bytes) => assert_eq!(bytes, [42u8; 32]),
            _ => panic!("expected Blake3 hash"),
        }

        let back = ChangeDirectory::<InMemoryBlobStore>::from_pijul_hash(&pijul_hash);
        assert_eq!(back, Some(aspen_hash));
    }

    #[tokio::test]
    async fn test_store_and_fetch_change() {
        let tmp = TempDir::new().unwrap();
        let blobs = Arc::new(InMemoryBlobStore::new());
        let change_store = Arc::new(AspenChangeStore::new(blobs));
        let dir = ChangeDirectory::new(&tmp.path().to_path_buf(), test_repo_id(), change_store);

        dir.ensure_dir().unwrap();

        // Store a fake change
        let change_bytes = b"fake pijul change content";
        let hash = dir.store_change(change_bytes).await.unwrap();

        // Should be cached locally
        assert!(dir.has_change(&hash));

        // Fetch should return immediately (cached)
        let path = dir.fetch_change(&hash).await.unwrap();
        assert!(path.exists());

        // Verify content
        let content = std::fs::read(&path).unwrap();
        assert_eq!(content, change_bytes);
    }
}
