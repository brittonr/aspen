//! Change recording for Pijul repositories.
//!
//! This module provides utilities for recording changes from a working
//! directory into Pijul patches. It wraps libpijul's record module and
//! integrates with Aspen's storage layer.
//!
//! ## Workflow
//!
//! 1. Scan the working directory for modifications
//! 2. Diff against the pristine state
//! 3. Create hunks representing the changes
//! 4. Package into a LocalChange
//! 5. Serialize and store in iroh-blobs
//!
//! ## Example
//!
//! ```ignore
//! let recorder = ChangeRecorder::new(pristine, change_dir, working_dir);
//!
//! // Record all changes in the working directory
//! let result = recorder.record(
//!     "main",
//!     "Fix bug in parser",
//!     "Alice <alice@example.com>",
//! ).await?;
//!
//! println!("Created change: {}", result.hash);
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use libpijul::change::{Author, ChangeHeader};
use libpijul::pristine::MutTxnT;
use libpijul::record::{Algorithm, Builder as RecordBuilder};
use libpijul::working_copy::filesystem::FileSystem as WorkingCopyFs;
use libpijul::MutTxnTExt;
use tracing::{debug, info, instrument};

use aspen_blob::BlobStore;

use super::apply::ChangeDirectory;
use super::error::{PijulError, PijulResult};
use super::pristine::PristineHandle;
use super::types::ChangeHash;

// ============================================================================
// ChangeRecorder
// ============================================================================

/// Records changes from a working directory into Pijul patches.
///
/// The recorder diffs the working directory against the pristine state
/// and creates patches representing the modifications.
///
/// ## Performance Options
///
/// For large repositories, use:
/// - `with_threads()` to enable parallel file scanning
/// - `with_prefix()` to record only specific directories
/// - `with_algorithm(Algorithm::Patience)` for large files
pub struct ChangeRecorder<B: BlobStore> {
    /// Handle to the pristine database.
    pristine: PristineHandle,

    /// Change directory for storing/fetching changes.
    changes: ChangeDirectory<B>,

    /// Path to the working directory.
    working_dir: PathBuf,

    /// Diff algorithm to use.
    algorithm: Algorithm,

    /// Number of threads for parallel file scanning (1 = single-threaded).
    threads: usize,

    /// Prefix to limit recording scope (empty = all files).
    prefix: String,
}

impl<B: BlobStore> ChangeRecorder<B> {
    /// Create a new change recorder.
    ///
    /// # Arguments
    ///
    /// - `pristine`: Handle to the pristine database
    /// - `changes`: Change directory for storage
    /// - `working_dir`: Path to the working directory to record from
    pub fn new(
        pristine: PristineHandle,
        changes: ChangeDirectory<B>,
        working_dir: PathBuf,
    ) -> Self {
        Self {
            pristine,
            changes,
            working_dir,
            algorithm: Algorithm::default(), // Myers
            threads: 1,
            prefix: String::new(),
        }
    }

    /// Set the diff algorithm to use.
    ///
    /// - `Myers` (default): Good for small to medium files
    /// - `Patience`: Better for large files with structural changes
    pub fn with_algorithm(mut self, algorithm: Algorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// Set the number of threads for parallel file scanning.
    ///
    /// Using multiple threads can significantly speed up recording for
    /// large repositories with many files. Recommended value is the number
    /// of CPU cores (e.g., 4-8 threads).
    ///
    /// Default is 1 (single-threaded).
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.threads = threads.max(1);
        self
    }

    /// Set a prefix to limit the scope of recording.
    ///
    /// Only files under the specified prefix will be scanned and recorded.
    /// This is useful for large repositories where you only want to record
    /// changes in a specific directory (e.g., "src/pijul").
    ///
    /// Default is empty (record all files).
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Record changes from the working directory.
    ///
    /// This scans the working directory, diffs against the pristine state,
    /// and creates a new change (patch) containing all modifications.
    ///
    /// # Arguments
    ///
    /// - `channel`: The channel to record against
    /// - `message`: Description of the change
    /// - `author`: Author string (e.g., "Name <email>")
    ///
    /// # Returns
    ///
    /// Returns `RecordResult` containing the change hash and metadata,
    /// or `None` if there are no changes to record.
    #[instrument(skip(self, message, author))]
    pub async fn record(
        &self,
        channel: &str,
        message: &str,
        author: &str,
    ) -> PijulResult<Option<RecordResult>> {
        // Ensure changes directory exists
        self.changes.ensure_dir()?;

        // Create the working copy wrapper
        let working_copy = WorkingCopyFs::from_root(&self.working_dir);

        // Get the libpijul change store
        let change_store = self.changes.libpijul_store();

        // Start an arc-wrapped mutable transaction (needed for record API)
        let arc_txn = self.pristine.arc_txn_begin()?;

        // Open or create the channel
        let channel_ref = {
            let mut txn = arc_txn.write();
            txn.open_or_create_channel(channel).map_err(|e| {
                PijulError::PristineStorage {
                    message: format!("failed to open channel: {:?}", e),
                }
            })?
        };

        // Add files in the working directory to tracking
        // If prefix is set, only scan files under that prefix
        {
            use canonical_path::CanonicalPathBuf;

            let repo_path = CanonicalPathBuf::canonicalize(&self.working_dir)
                .map_err(|e| PijulError::RecordFailed {
                    message: format!("failed to canonicalize working dir: {:?}", e),
                })?;

            // If prefix is set, scan only that subdirectory
            let full = if self.prefix.is_empty() {
                repo_path.clone()
            } else {
                CanonicalPathBuf::canonicalize(self.working_dir.join(&self.prefix))
                    .unwrap_or_else(|_| repo_path.clone())
            };

            working_copy
                .add_prefix_rec(
                    &arc_txn,
                    repo_path,
                    full,
                    false,        // force: don't force-add ignored files
                    self.threads, // threads: configurable parallelism
                    0,            // salt: for conflict naming
                )
                .map_err(|e| PijulError::RecordFailed {
                    message: format!("failed to add files: {:?}", e),
                })?;
        }

        // Create the record builder
        let mut builder = RecordBuilder::new();

        // Create a default diff separator regex (empty pattern for line-based diffing)
        let separator = regex::bytes::Regex::new("").unwrap();

        // Record changes using the configured settings
        builder
            .record_single_thread(
                arc_txn.clone(),
                self.algorithm,
                false, // stop_early
                &separator,
                channel_ref.clone(),
                &working_copy,
                &change_store,
                &self.prefix, // use prefix to limit scope
            )
            .map_err(|e| PijulError::RecordFailed {
                message: format!("{:?}", e),
            })?;

        // Finish recording
        let recorded = builder.finish();

        // Check if there are any changes
        if recorded.actions.is_empty() {
            debug!(channel = channel, "no changes to record");
            return Ok(None);
        }

        // Create the author
        let mut author_map = BTreeMap::new();
        author_map.insert("name".to_string(), author.to_string());

        // Create the change header
        let header = ChangeHeader {
            message: message.to_string(),
            description: None,
            timestamp: chrono::Utc::now(),
            authors: vec![Author(author_map)],
        };

        // Convert to a LocalChange
        let mut local_change = {
            let txn = arc_txn.read();

            recorded
                .into_change(&*txn, &channel_ref, header)
                .map_err(|e| PijulError::RecordFailed {
                    message: format!("failed to create change: {:?}", e),
                })?
        };

        // Serialize the change to bytes
        // The closure handles content hashing - we ignore it since we use BLAKE3
        let mut change_bytes = Vec::new();
        let pijul_hash = local_change
            .serialize(
                &mut change_bytes,
                |_hash, _writer| -> Result<(), libpijul::change::ChangeError> { Ok(()) },
            )
            .map_err(|e| PijulError::Serialization {
                message: format!("failed to serialize change: {:?}", e),
            })?;

        // Store in blob storage (for P2P distribution)
        let aspen_hash = self.changes.store_change(&change_bytes).await?;

        // Also save the change file using libpijul's path format so apply_change can find it
        change_store
            .save_from_buf_unchecked(&change_bytes, &pijul_hash, None)
            .map_err(|e| PijulError::Io {
                message: format!("failed to save change file: {}", e),
            })?;

        // Apply the change to the pristine
        {
            let mut txn = arc_txn.write();
            let mut channel_guard = channel_ref.write();

            txn.apply_change(&change_store, &mut channel_guard, &pijul_hash)
                .map_err(|e| PijulError::ApplyFailed {
                    message: format!("{:?}", e),
                })?;
        }

        // Commit the arc transaction
        arc_txn.commit().map_err(|e| PijulError::PristineStorage {
            message: format!("failed to commit transaction: {:?}", e),
        })?;

        let num_hunks = local_change.changes.len();

        // Extract dependencies from the local change
        // These are the pijul hashes of changes this change depends on
        let dependencies: Vec<ChangeHash> = local_change
            .dependencies
            .iter()
            .filter_map(|pijul_hash| ChangeDirectory::<B>::from_pijul_hash(pijul_hash))
            .collect();

        info!(
            channel = channel,
            hash = %aspen_hash,
            hunks = num_hunks,
            deps = dependencies.len(),
            "recorded change"
        );

        Ok(Some(RecordResult {
            hash: aspen_hash,
            pijul_hash,
            message: message.to_string(),
            author: author.to_string(),
            num_hunks,
            size_bytes: change_bytes.len(),
            dependencies,
        }))
    }

    /// Get a reference to the working directory.
    pub fn working_dir(&self) -> &PathBuf {
        &self.working_dir
    }

    /// Get a reference to the pristine handle.
    pub fn pristine(&self) -> &PristineHandle {
        &self.pristine
    }
}

// ============================================================================
// DiffResult
// ============================================================================

/// Result of computing a diff between working directory and pristine.
#[derive(Debug, Clone)]
pub struct DiffResult {
    /// List of hunks (file changes).
    pub hunks: Vec<DiffHunkInfo>,
    /// Number of hunks in the diff.
    pub num_hunks: usize,
}

/// Information about a single hunk in a diff.
#[derive(Debug, Clone)]
pub struct DiffHunkInfo {
    /// Path of the file being changed.
    pub path: String,
    /// Type of change: "add", "delete", "modify", etc.
    pub kind: String,
    /// Number of lines added.
    pub additions: u32,
    /// Number of lines deleted.
    pub deletions: u32,
}

impl<B: BlobStore + Clone + 'static> ChangeRecorder<B> {
    /// Compute diff between working directory and channel without recording.
    ///
    /// This is similar to `record` but doesn't create or apply a change.
    /// It returns information about what changes would be recorded.
    #[instrument(skip(self))]
    pub async fn diff(&self, channel: &str) -> PijulResult<DiffResult> {
        // Create the working copy wrapper
        let working_copy = WorkingCopyFs::from_root(&self.working_dir);

        // Get the libpijul change store
        let change_store = self.changes.libpijul_store();

        // Start an arc-wrapped mutable transaction (needed for record API)
        let arc_txn = self.pristine.arc_txn_begin()?;

        // Open or create the channel
        let channel_ref = {
            let mut txn = arc_txn.write();
            txn.open_or_create_channel(channel).map_err(|e| {
                PijulError::PristineStorage {
                    message: format!("failed to open channel: {:?}", e),
                }
            })?
        };

        // Add files in the working directory to tracking
        // If prefix is set, only scan files under that prefix
        {
            use canonical_path::CanonicalPathBuf;

            let repo_path = CanonicalPathBuf::canonicalize(&self.working_dir)
                .map_err(|e| PijulError::RecordFailed {
                    message: format!("failed to canonicalize working dir: {:?}", e),
                })?;

            // If prefix is set, scan only that subdirectory
            let full = if self.prefix.is_empty() {
                repo_path.clone()
            } else {
                CanonicalPathBuf::canonicalize(self.working_dir.join(&self.prefix))
                    .unwrap_or_else(|_| repo_path.clone())
            };

            working_copy
                .add_prefix_rec(
                    &arc_txn,
                    repo_path,
                    full,
                    false,        // force: don't force-add ignored files
                    self.threads, // threads: configurable parallelism
                    0,            // salt: for conflict naming
                )
                .map_err(|e| PijulError::RecordFailed {
                    message: format!("failed to add files: {:?}", e),
                })?;
        }

        // Create the record builder
        let mut builder = RecordBuilder::new();

        // Create a default diff separator regex (empty pattern for line-based diffing)
        let separator = regex::bytes::Regex::new("").unwrap();

        // Record changes using the configured settings
        builder
            .record_single_thread(
                arc_txn.clone(),
                self.algorithm,
                false, // stop_early
                &separator,
                channel_ref.clone(),
                &working_copy,
                &change_store,
                &self.prefix, // use prefix to limit scope
            )
            .map_err(|e| PijulError::RecordFailed {
                message: format!("{:?}", e),
            })?;

        // Finish recording (but don't commit or apply)
        let recorded = builder.finish();

        // Convert actions to hunk info
        // The actions field contains libpijul::change::Hunk<Option<ChangeId>, LocalByte>
        let mut hunks = Vec::new();
        for action in &recorded.actions {
            use libpijul::change::Hunk;
            let (path, kind) = match action {
                Hunk::FileAdd { path, .. } => {
                    (path.clone(), "add".to_string())
                }
                Hunk::FileDel { path, .. } => {
                    (path.clone(), "delete".to_string())
                }
                Hunk::FileMove { path, .. } => {
                    (path.clone(), "rename".to_string())
                }
                Hunk::FileUndel { path, .. } => {
                    (path.clone(), "undelete".to_string())
                }
                Hunk::Edit { local, .. } => {
                    // Edit hunks don't have a path, use a placeholder
                    (format!("(line {})", local.line), "modify".to_string())
                }
                Hunk::Replacement { local, .. } => {
                    (format!("(line {})", local.line), "modify".to_string())
                }
                Hunk::SolveOrderConflict { local, .. } => {
                    (format!("(line {})", local.line), "solve".to_string())
                }
                Hunk::UnsolveOrderConflict { local, .. } => {
                    (format!("(line {})", local.line), "unsolve".to_string())
                }
                Hunk::ResurrectZombies { local, .. } => {
                    (format!("(line {})", local.line), "resurrect".to_string())
                }
                Hunk::SolveNameConflict { path, .. } => {
                    (path.clone(), "solve".to_string())
                }
                Hunk::UnsolveNameConflict { path, .. } => {
                    (path.clone(), "unsolve".to_string())
                }
                // Root operations (rare, usually for initial/empty repos)
                Hunk::AddRoot { .. } | Hunk::DelRoot { .. } => {
                    continue; // Skip root operations in diff output
                }
            };

            // Count line additions and deletions based on hunk content
            let (additions, deletions) = count_line_changes(action);

            hunks.push(DiffHunkInfo {
                path,
                kind,
                additions,
                deletions,
            });
        }

        // Don't commit - we're just computing the diff
        // arc_txn will be dropped and rolled back

        let num_hunks = hunks.len();
        debug!(channel = channel, num_hunks = num_hunks, "computed diff");

        Ok(DiffResult { hunks, num_hunks })
    }
}

/// Count line additions and deletions from a pijul hunk.
///
/// This analyzes the hunk content to estimate the number of lines
/// added and removed. The counts are approximate since pijul's internal
/// representation doesn't map directly to line-based changes.
fn count_line_changes(action: &libpijul::change::Hunk<Option<libpijul::pristine::ChangeId>, libpijul::change::LocalByte>) -> (u32, u32) {
    use libpijul::change::Hunk;

    match action {
        // File operations generally count as single-line changes
        Hunk::FileAdd { .. } => (1, 0),        // Adding a file
        Hunk::FileDel { .. } => (0, 1),        // Deleting a file
        Hunk::FileMove { .. } => (0, 0),       // Moving/renaming doesn't change content
        Hunk::FileUndel { .. } => (1, 0),      // Undeleting a file

        // Content edits - estimate based on hunk size
        Hunk::Edit { .. } => {
            // Edit hunks represent content changes
            // For now, estimate 1 addition (conservative)
            (1, 0)
        }

        Hunk::Replacement { .. } => {
            // Replacement typically involves both addition and deletion
            // Estimate as 1 line changed
            (1, 1)
        }

        // Conflict resolution operations
        Hunk::SolveOrderConflict { .. } => (0, 0),    // Solving conflicts doesn't change line count
        Hunk::UnsolveOrderConflict { .. } => (0, 0),
        Hunk::ResurrectZombies { .. } => (0, 0),
        Hunk::SolveNameConflict { .. } => (0, 0),
        Hunk::UnsolveNameConflict { .. } => (0, 0),

        // Root operations (rare)
        Hunk::AddRoot { .. } => (0, 0),
        Hunk::DelRoot { .. } => (0, 0),
    }
}

// ============================================================================
// RecordResult
// ============================================================================

/// Result of recording a change.
#[derive(Debug, Clone)]
pub struct RecordResult {
    /// The Aspen change hash (BLAKE3).
    pub hash: ChangeHash,

    /// The libpijul hash.
    pub pijul_hash: libpijul::pristine::Hash,

    /// The change message.
    pub message: String,

    /// The change author.
    pub author: String,

    /// Number of hunks in the change.
    pub num_hunks: usize,

    /// Size of the serialized change in bytes.
    pub size_bytes: usize,

    /// Dependencies of this change (parent changes it builds on).
    ///
    /// These are the changes that were already applied to the channel
    /// when this change was recorded. Remote nodes need these to properly
    /// order and apply changes.
    pub dependencies: Vec<ChangeHash>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use aspen_blob::InMemoryBlobStore;
    use aspen_forge::identity::RepoId;
    use crate::change_store::AspenChangeStore;
    use crate::pristine::PristineManager;
    use tempfile::TempDir;

    fn test_repo_id() -> RepoId {
        RepoId([1u8; 32])
    }

    #[tokio::test]
    async fn test_record_no_changes() {
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path().join("data");
        let work_dir = tmp.path().join("work");
        std::fs::create_dir_all(&work_dir).unwrap();

        let repo_id = test_repo_id();

        // Create stores
        let blobs = Arc::new(InMemoryBlobStore::new());
        let change_store = Arc::new(AspenChangeStore::new(blobs));
        let change_dir = ChangeDirectory::new(&data_dir, repo_id, change_store);

        // Create pristine
        let pristine_mgr = PristineManager::new(&data_dir);
        let pristine = pristine_mgr.open_or_create(&repo_id).unwrap();

        // Create recorder
        let recorder = ChangeRecorder::new(pristine, change_dir, work_dir);

        // Recording with no files should return None
        let result = recorder.record("main", "Empty change", "Test <test@test.com>").await;

        // Should succeed but return None (no changes)
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_recorder_creation() {
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path().join("data");
        let work_dir = tmp.path().join("work");
        std::fs::create_dir_all(&work_dir).unwrap();

        let repo_id = test_repo_id();

        let blobs = Arc::new(InMemoryBlobStore::new());
        let change_store = Arc::new(AspenChangeStore::new(blobs));
        let change_dir = ChangeDirectory::new(&data_dir, repo_id, change_store);

        let pristine_mgr = PristineManager::new(&data_dir);
        let pristine = pristine_mgr.open_or_create(&repo_id).unwrap();

        let recorder = ChangeRecorder::new(pristine, change_dir, work_dir.clone());

        assert_eq!(recorder.working_dir(), &work_dir);
    }
}
