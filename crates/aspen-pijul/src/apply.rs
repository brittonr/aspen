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

use aspen_blob::prelude::*;
use aspen_forge::identity::RepoId;
use libpijul::MutTxnTExt;
use libpijul::change::Change;
use libpijul::changestore::filesystem::FileSystem as LibpijulFileSystem;
use libpijul::pristine::Hash;
use libpijul::pristine::Merkle;
use tracing::debug;
use tracing::info;
use tracing::instrument;

use super::change_store::AspenChangeStore;
use super::error::PijulError;
use super::error::PijulResult;
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

impl<B: BlobStore> Clone for ChangeDirectory<B> {
    fn clone(&self) -> Self {
        Self {
            base_dir: self.base_dir.clone(),
            blobs: Arc::clone(&self.blobs),
            repo_id: self.repo_id,
        }
    }
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
    pub fn change_path(&self, hash: &ChangeHash) -> PathBuf {
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
            .ok_or_else(|| PijulError::ChangeNotFound { hash: hash.to_hex() })?;

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
    ///
    /// This stores the change in both our BLAKE3-based path format (for P2P lookup)
    /// and libpijul's base32 path format (for apply_change to find).
    #[instrument(skip(self, bytes))]
    pub async fn store_change(&self, bytes: &[u8]) -> PijulResult<ChangeHash> {
        // Store in blob storage first to get our BLAKE3 hash
        let hash = self.blobs.store_change(bytes).await?;

        // Cache locally using our path format
        let path = self.change_path(&hash);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| PijulError::Io {
                message: format!("failed to create change directory: {}", e),
            })?;
        }

        std::fs::write(&path, bytes).map_err(|e| PijulError::Io {
            message: format!("failed to write change file: {}", e),
        })?;

        // Also save using libpijul's path format so apply_change can find it
        // We need to deserialize to get the pijul hash, then save with that hash
        // We already wrote the bytes to `path`, so use that for deserialization
        let change = Change::deserialize(
            path.to_str().ok_or_else(|| PijulError::Io {
                message: "change file path is not valid UTF-8".to_string(),
            })?,
            None,
        )
        .map_err(|e| PijulError::Deserialization {
            message: format!("failed to deserialize change for hash extraction: {:?}", e),
        })?;
        let pijul_hash = change.hash().map_err(|e| PijulError::Deserialization {
            message: format!("failed to compute pijul hash: {:?}", e),
        })?;

        let store = self.libpijul_store();
        store.save_from_buf_unchecked(bytes, &pijul_hash, None).map_err(|e| PijulError::Io {
            message: format!("failed to save change in libpijul format: {}", e),
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

        // Read the change from our storage to get the actual pijul hash
        let our_path = self.changes.change_path(hash);
        let bytes = std::fs::read(&our_path).map_err(|e| PijulError::Io {
            message: format!("failed to read change file: {}", e),
        })?;
        let change = Change::deserialize(
            our_path.to_str().ok_or_else(|| PijulError::Io {
                message: "change file path is not valid UTF-8".to_string(),
            })?,
            None,
        )
        .map_err(|e| PijulError::Deserialization {
            message: format!("failed to deserialize change: {:?}", e),
        })?;
        let pijul_hash = change.hash().map_err(|e| PijulError::Deserialization {
            message: format!("failed to compute pijul hash: {:?}", e),
        })?;

        // Save in libpijul format so apply_change can find it
        store.save_from_buf_unchecked(&bytes, &pijul_hash, None).map_err(|e| PijulError::Io {
            message: format!("failed to save change in libpijul format: {}", e),
        })?;

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
    pub async fn apply_changes(&self, channel: &str, hashes: &[ChangeHash]) -> PijulResult<Vec<ApplyResult>> {
        let mut results = Vec::with_capacity(hashes.len());

        for hash in hashes {
            let result = self.fetch_and_apply(channel, hash).await?;
            results.push(result);
        }

        info!(channel = channel, count = results.len(), "applied changes");
        Ok(results)
    }

    /// List all change hashes applied to a channel.
    ///
    /// Returns the hashes in chronological order (oldest first).
    #[instrument(skip(self))]
    pub fn list_channel_changes(&self, channel: &str) -> PijulResult<Vec<ChangeHash>> {
        use libpijul::TxnTExt;
        use libpijul::pristine::Hash as PijulHash;

        let txn = self.pristine.txn_begin()?;

        // Load the channel
        let channel_ref = txn.load_channel(channel)?.ok_or_else(|| PijulError::ChannelNotFound {
            channel: channel.to_string(),
        })?;

        let channel_guard = channel_ref.read();

        // Get the log iterator
        let log = txn.txn().log(&channel_guard, 0u64).map_err(|e| PijulError::PristineStorage {
            message: format!("failed to read channel log: {:?}", e),
        })?;

        // Collect all change hashes
        let mut hashes = Vec::new();
        for entry in log {
            if let Ok((_, (h, _))) = entry {
                // Convert SerializedHash to Hash, then to our ChangeHash
                let pijul_hash: PijulHash = h.into();
                // Extract the blake3 bytes if it's a Blake3 hash
                if let PijulHash::Blake3(bytes) = pijul_hash {
                    hashes.push(ChangeHash(bytes));
                }
            }
        }

        debug!(channel = channel, count = hashes.len(), "listed channel changes");
        Ok(hashes)
    }

    /// Check for conflicts on a channel after applying changes.
    ///
    /// This outputs the channel's state to a temporary directory and counts
    /// any conflicts that libpijul detects. This is useful for detecting
    /// merge conflicts after syncing changes from multiple sources.
    ///
    /// # Arguments
    ///
    /// - `channel`: Channel name to check
    ///
    /// # Returns
    ///
    /// The number of conflicts detected.
    #[instrument(skip(self))]
    pub fn check_conflicts(&self, channel: &str) -> PijulResult<u32> {
        use super::output::WorkingDirOutput;

        // Create a temp directory for output
        let temp_dir = std::env::temp_dir().join(format!(
            "aspen-conflict-check-{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).map_err(|e| PijulError::Io {
            message: format!("failed to create temp directory: {}", e),
        })?;

        // Create outputter using our pristine and change directory
        let outputter = WorkingDirOutput::new(self.pristine.clone(), self.changes.clone(), temp_dir.clone());

        // Output and check for conflicts
        let result = outputter.output(channel)?;
        let conflict_count = result.conflict_count() as u32;

        // Clean up temp directory
        if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
            debug!(path = %temp_dir.display(), error = %e, "failed to clean up temp directory");
        }

        if conflict_count > 0 {
            info!(channel = channel, conflicts = conflict_count, "conflicts detected");
        } else {
            debug!(channel = channel, "no conflicts");
        }

        Ok(conflict_count)
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
    use aspen_blob::InMemoryBlobStore;
    use tempfile::TempDir;

    use super::*;

    fn test_repo_id() -> RepoId {
        RepoId([1u8; 32])
    }

    #[test]
    fn test_change_path() {
        let tmp = TempDir::new().unwrap();
        let blobs = Arc::new(InMemoryBlobStore::new());
        let change_store = Arc::new(AspenChangeStore::new(blobs));
        let dir = ChangeDirectory::new(&tmp.path().to_path_buf(), test_repo_id(), change_store);

        let hash = ChangeHash(
            [0xAB, 0xCD, 0xEF]
                .iter()
                .cloned()
                .chain(std::iter::repeat(0))
                .take(32)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        );
        let path = dir.change_path(&hash);

        // Should have the prefix directory structure
        assert!(path.to_string_lossy().contains("ab"));
        assert!(path.to_string_lossy().ends_with(".change"));
    }

    #[test]
    fn test_hash_conversion() {
        let aspen_hash = ChangeHash([42u8; 32]);
        let pijul_hash = ChangeDirectory::<InMemoryBlobStore>::to_pijul_hash(&aspen_hash);

        assert!(
            matches!(pijul_hash, Hash::Blake3(bytes) if bytes == [42u8; 32]),
            "expected Blake3 hash with [42u8; 32], got {:?}",
            pijul_hash
        );

        let back = ChangeDirectory::<InMemoryBlobStore>::from_pijul_hash(&pijul_hash);
        assert_eq!(back, Some(aspen_hash));
    }

    #[tokio::test]
    async fn test_store_invalid_change_fails() {
        let tmp = TempDir::new().unwrap();
        let blobs = Arc::new(InMemoryBlobStore::new());
        let change_store = Arc::new(AspenChangeStore::new(blobs));
        let dir = ChangeDirectory::new(&tmp.path().to_path_buf(), test_repo_id(), change_store);

        dir.ensure_dir().unwrap();

        // Storing invalid change content should fail (can't deserialize)
        let fake_bytes = b"fake pijul change content";
        let result = dir.store_change(fake_bytes).await;

        // Should fail with deserialization error
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, PijulError::Deserialization { .. }), "expected Deserialization error, got: {:?}", err);
    }
}
