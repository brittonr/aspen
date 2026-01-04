//! Working directory output for Pijul repositories.
//!
//! This module provides utilities for outputting files from the pristine
//! state to the working directory. It wraps libpijul's output module and
//! integrates with Aspen's storage layer.
//!
//! ## Workflow
//!
//! 1. Open the pristine database with an arc-wrapped transaction
//! 2. Get the channel to output from
//! 3. Output files to the working directory using libpijul's output_repository_no_pending
//!
//! ## Example
//!
//! ```ignore
//! let outputter = WorkingDirOutput::new(pristine, change_dir, working_dir);
//!
//! // Output all files to the working directory
//! let conflicts = outputter.output("main").await?;
//!
//! if !conflicts.is_empty() {
//!     println!("Warning: {} conflicts found", conflicts.len());
//! }
//! ```

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use aspen_blob::BlobStore;
use libpijul::output::Conflict;
use libpijul::output::output_repository_no_pending;
use libpijul::pristine::MutTxnT;
use libpijul::working_copy::filesystem::FileSystem as WorkingCopyFs;
use tracing::debug;
use tracing::info;
use tracing::instrument;

use super::apply::ChangeDirectory;
use super::error::PijulError;
use super::error::PijulResult;
use super::pristine::PristineHandle;

// ============================================================================
// WorkingDirOutput
// ============================================================================

/// Outputs files from the pristine state to the working directory.
///
/// This struct wraps libpijul's output functionality, providing a high-level
/// API for materializing the repository state as files on disk.
///
/// # Warning
///
/// Output operations will **overwrite** the working directory contents,
/// cancelling any unrecorded changes. Use with caution.
pub struct WorkingDirOutput<B: BlobStore> {
    /// Handle to the pristine database.
    pristine: PristineHandle,

    /// Change directory for accessing changes.
    changes: ChangeDirectory<B>,

    /// Path to the working directory.
    working_dir: PathBuf,

    /// Number of worker threads for parallel output.
    n_workers: usize,
}

impl<B: BlobStore> WorkingDirOutput<B> {
    /// Create a new working directory outputter.
    ///
    /// # Arguments
    ///
    /// - `pristine`: Handle to the pristine database
    /// - `changes`: Change directory for accessing changes
    /// - `working_dir`: Path to the working directory to output to
    pub fn new(pristine: PristineHandle, changes: ChangeDirectory<B>, working_dir: PathBuf) -> Self {
        Self {
            pristine,
            changes,
            working_dir,
            n_workers: 1, // Single-threaded by default for simplicity
        }
    }

    /// Set the number of worker threads for parallel output.
    ///
    /// More workers can speed up output for repositories with many files,
    /// but also increase resource usage.
    pub fn with_workers(mut self, n_workers: usize) -> Self {
        self.n_workers = n_workers.max(1);
        self
    }

    /// Output the channel's current state to the working directory.
    ///
    /// This materializes all files in the channel on disk. Any existing
    /// files in the working directory will be overwritten.
    ///
    /// # Arguments
    ///
    /// - `channel`: The channel name to output
    ///
    /// # Returns
    ///
    /// Returns a set of conflicts if any files contain merge conflicts.
    /// An empty set indicates a clean output with no conflicts.
    #[instrument(skip(self))]
    pub fn output(&self, channel: &str) -> PijulResult<OutputResult> {
        // Ensure changes directory exists
        self.changes.ensure_dir()?;

        // Ensure working directory exists
        std::fs::create_dir_all(&self.working_dir).map_err(|e| PijulError::Io {
            message: format!("failed to create working directory: {}", e),
        })?;

        // Create the working copy wrapper
        let working_copy = WorkingCopyFs::from_root(&self.working_dir);

        // Get the libpijul change store
        let change_store = self.changes.libpijul_store();

        // Start an arc-wrapped transaction (needed for output API)
        let arc_txn = self.pristine.arc_txn_begin()?;

        // Open the channel
        let channel_ref = {
            let mut txn = arc_txn.write();
            txn.open_or_create_channel(channel).map_err(|e| PijulError::PristineStorage {
                message: format!("failed to open channel: {:?}", e),
            })?
        };

        // Output to the working directory
        debug!(channel = channel, working_dir = %self.working_dir.display(), "starting output");

        let conflicts = output_repository_no_pending(
            &working_copy,
            &change_store,
            &arc_txn,
            &channel_ref,
            "",   // prefix: empty = all files
            true, // output_name_conflicts: show conflicting file names
            None, // if_modified_since: always output all files
            self.n_workers,
            0, // salt: used for conflict naming
        )
        .map_err(|e| PijulError::OutputFailed {
            message: format!("{:?}", e),
        })?;

        // Commit the arc transaction to save any inode updates
        arc_txn.commit().map_err(|e| PijulError::PristineStorage {
            message: format!("failed to commit transaction: {:?}", e),
        })?;

        let conflict_count = conflicts.len();
        if conflict_count > 0 {
            info!(channel = channel, conflicts = conflict_count, "output complete with conflicts");
        } else {
            info!(channel = channel, "output complete");
        }

        Ok(OutputResult {
            conflicts,
            working_dir: self.working_dir.clone(),
        })
    }

    /// Output a single file or directory prefix.
    ///
    /// This outputs only the specified path prefix from the channel.
    ///
    /// # Arguments
    ///
    /// - `channel`: The channel name to output from
    /// - `prefix`: The path prefix to output (e.g., "src/" or "README.md")
    #[instrument(skip(self))]
    pub fn output_prefix(&self, channel: &str, prefix: &str) -> PijulResult<OutputResult> {
        // Ensure changes directory exists
        self.changes.ensure_dir()?;

        // Ensure working directory exists
        std::fs::create_dir_all(&self.working_dir).map_err(|e| PijulError::Io {
            message: format!("failed to create working directory: {}", e),
        })?;

        // Create the working copy wrapper
        let working_copy = WorkingCopyFs::from_root(&self.working_dir);

        // Get the libpijul change store
        let change_store = self.changes.libpijul_store();

        // Start an arc-wrapped transaction
        let arc_txn = self.pristine.arc_txn_begin()?;

        // Open the channel
        let channel_ref = {
            let mut txn = arc_txn.write();
            txn.open_or_create_channel(channel).map_err(|e| PijulError::PristineStorage {
                message: format!("failed to open channel: {:?}", e),
            })?
        };

        // Output with prefix
        debug!(
            channel = channel,
            prefix = prefix,
            working_dir = %self.working_dir.display(),
            "starting output with prefix"
        );

        let conflicts = output_repository_no_pending(
            &working_copy,
            &change_store,
            &arc_txn,
            &channel_ref,
            prefix,
            true,
            None,
            self.n_workers,
            0,
        )
        .map_err(|e| PijulError::OutputFailed {
            message: format!("{:?}", e),
        })?;

        // Commit the transaction
        arc_txn.commit().map_err(|e| PijulError::PristineStorage {
            message: format!("failed to commit transaction: {:?}", e),
        })?;

        let conflict_count = conflicts.len();
        if conflict_count > 0 {
            info!(channel = channel, prefix = prefix, conflicts = conflict_count, "output complete with conflicts");
        } else {
            info!(channel = channel, prefix = prefix, "output complete");
        }

        Ok(OutputResult {
            conflicts,
            working_dir: self.working_dir.clone(),
        })
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
// OutputResult
// ============================================================================

/// Result of outputting files to the working directory.
#[derive(Debug)]
pub struct OutputResult {
    /// Set of conflicts found during output.
    ///
    /// Each conflict represents a file that has conflicting content
    /// due to un-merged patches.
    pub conflicts: BTreeSet<Conflict>,

    /// Path to the working directory that was output to.
    pub working_dir: PathBuf,
}

impl OutputResult {
    /// Check if the output was clean (no conflicts).
    pub fn is_clean(&self) -> bool {
        self.conflicts.is_empty()
    }

    /// Get the number of conflicts.
    pub fn conflict_count(&self) -> usize {
        self.conflicts.len()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use aspen_blob::InMemoryBlobStore;
    use aspen_forge::identity::RepoId;
    use tempfile::TempDir;

    use super::*;
    use crate::change_store::AspenChangeStore;
    use crate::pristine::PristineManager;

    fn test_repo_id() -> RepoId {
        RepoId([1u8; 32])
    }

    #[test]
    fn test_outputter_creation() {
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

        let outputter = WorkingDirOutput::new(pristine, change_dir, work_dir.clone());

        assert_eq!(outputter.working_dir(), &work_dir);
    }

    #[test]
    fn test_output_empty_channel() {
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path().join("data");
        let work_dir = tmp.path().join("work");

        let repo_id = test_repo_id();

        let blobs = Arc::new(InMemoryBlobStore::new());
        let change_store = Arc::new(AspenChangeStore::new(blobs));
        let change_dir = ChangeDirectory::new(&data_dir, repo_id, change_store);

        let pristine_mgr = PristineManager::new(&data_dir);
        let pristine = pristine_mgr.open_or_create(&repo_id).unwrap();

        let outputter = WorkingDirOutput::new(pristine, change_dir, work_dir.clone());

        // Output an empty channel should succeed with no conflicts
        let result = outputter.output("main");
        assert!(result.is_ok());

        let output_result = result.unwrap();
        assert!(output_result.is_clean());
        assert_eq!(output_result.conflict_count(), 0);

        // Working directory should exist but be empty
        assert!(work_dir.exists());
    }

    #[test]
    fn test_output_with_workers() {
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path().join("data");
        let work_dir = tmp.path().join("work");

        let repo_id = test_repo_id();

        let blobs = Arc::new(InMemoryBlobStore::new());
        let change_store = Arc::new(AspenChangeStore::new(blobs));
        let change_dir = ChangeDirectory::new(&data_dir, repo_id, change_store);

        let pristine_mgr = PristineManager::new(&data_dir);
        let pristine = pristine_mgr.open_or_create(&repo_id).unwrap();

        let outputter = WorkingDirOutput::new(pristine, change_dir, work_dir).with_workers(4);

        // with_workers should set the worker count
        assert_eq!(outputter.n_workers, 4);
    }
}
