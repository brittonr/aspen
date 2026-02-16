//! Working directory support for Pijul repositories.
//!
//! This module provides client-side working directory management for Pijul
//! repositories, enabling users to work with files directly rather than just
//! pristine operations.
//!
//! ## Architecture
//!
//! A working directory is linked to a remote Aspen repository via the
//! `.aspen/pijul` metadata directory:
//!
//! ```text
//! my-project/
//! ├── .aspen/
//! │   └── pijul/
//! │       ├── config.toml      # Repo ID, remote, channel
//! │       ├── staged           # Staged file paths (one per line)
//! │       └── pristine/        # Local sanakirja database
//! ├── src/
//! │   └── main.rs
//! └── README.md
//! ```
//!
//! ## Workflow
//!
//! 1. `pijul init` - Initialize a working directory linked to a repo
//! 2. `pijul add <path>` - Stage files for the next change
//! 3. `pijul reset <path>` - Unstage files
//! 4. `pijul record` - Create a change from staged files
//! 5. `pijul sync` - Sync with remote repository
//!
//! ## Usage
//!
//! ```ignore
//! // Initialize a working directory
//! let wd = WorkingDirectory::init(
//!     path,
//!     repo_id,
//!     remote,
//!     "main",
//! )?;
//!
//! // Stage files
//! wd.add(&["src/main.rs", "Cargo.toml"])?;
//!
//! // Check status
//! let staged = wd.staged_files()?;
//! println!("Staged: {:?}", staged);
//!
//! // Unstage files
//! wd.reset(&["Cargo.toml"])?;
//! ```

use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use aspen_forge::identity::RepoId;
use serde::Deserialize;
use serde::Serialize;
use snafu::ResultExt;
use tracing::debug;
use tracing::info;
use tracing::instrument;

use super::constants::MAX_CHANNEL_NAME_LENGTH_BYTES;
use super::constants::MAX_PATH_LENGTH_BYTES;
use super::constants::MAX_STAGED_FILES;
use super::constants::MAX_WORKING_DIR_FILE_SIZE;
use super::constants::WORKING_DIR_CONFIG_FILE;
use super::constants::WORKING_DIR_METADATA_DIR;
use super::constants::WORKING_DIR_PRISTINE_DIR;
use super::constants::WORKING_DIR_STAGED_FILE;
use super::error::CreateDirSnafu;
use super::error::PijulError;
use super::error::PijulResult;
use super::error::ReadDirEntrySnafu;
use super::error::ReadDirSnafu;
use super::error::ReadFileToStringSnafu;
use super::error::StatFileSnafu;
use super::error::WriteFileSnafu;
use super::pristine::PristineHandle;
use super::pristine::PristineManager;

// ============================================================================
// Working Directory Configuration
// ============================================================================

/// Configuration stored in `.aspen/pijul/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingDirectoryConfig {
    /// Repository ID this working directory is linked to.
    pub repo_id: String,

    /// Current channel (branch) being tracked.
    pub channel: String,

    /// Remote node address for syncing (if any).
    pub remote: Option<String>,

    /// Last synced channel head hash.
    pub last_synced_head: Option<String>,

    /// Timestamp of last sync (milliseconds since epoch).
    pub last_synced_at_ms: Option<u64>,
}

impl WorkingDirectoryConfig {
    /// Create a new configuration.
    pub fn new(repo_id: &RepoId, channel: &str, remote: Option<String>) -> Self {
        Self {
            repo_id: repo_id.to_hex(),
            channel: channel.to_string(),
            remote,
            last_synced_head: None,
            last_synced_at_ms: None,
        }
    }

    /// Parse the repository ID.
    pub fn parse_repo_id(&self) -> PijulResult<RepoId> {
        RepoId::from_hex(&self.repo_id).map_err(|_| PijulError::InvalidRepoIdentity {
            message: format!("invalid repo_id in config: {}", self.repo_id),
        })
    }
}

// ============================================================================
// Working Directory Status
// ============================================================================

/// Status of a file in the working directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    /// File is tracked and unchanged.
    Unmodified,
    /// File has been modified since last change.
    Modified,
    /// File is new (not in pristine).
    Added,
    /// File was deleted from working directory.
    Deleted,
    /// File is staged for the next change.
    Staged,
    /// File is untracked (not in pristine, not staged).
    Untracked,
}

/// Status of a single file.
#[derive(Debug, Clone)]
pub struct FileStatusEntry {
    /// Path relative to working directory root.
    pub path: String,
    /// Status of the file.
    pub status: FileStatus,
}

/// Overall status of the working directory.
#[derive(Debug, Clone)]
pub struct WorkingDirectoryStatus {
    /// Current channel.
    pub channel: String,
    /// Last synced head (if any).
    pub last_synced_head: Option<String>,
    /// Number of staged files.
    pub staged_count: u32,
    /// Number of modified files.
    pub modified_count: u32,
    /// Number of untracked files.
    pub untracked_count: u32,
    /// Individual file statuses (limited to first 1000).
    pub files: Vec<FileStatusEntry>,
}

// ============================================================================
// Working Directory
// ============================================================================

/// A working directory linked to a Pijul repository.
///
/// Provides client-side file tracking and staging for Pijul operations.
pub struct WorkingDirectory {
    /// Root path of the working directory.
    root: PathBuf,

    /// Path to the metadata directory (`.aspen/pijul`).
    metadata_dir: PathBuf,

    /// Configuration.
    config: WorkingDirectoryConfig,

    /// Pristine manager for the local pristine.
    pristine_mgr: PristineManager,
}

impl WorkingDirectory {
    /// Initialize a new working directory.
    ///
    /// Creates the `.aspen/pijul` metadata directory and configuration file.
    /// If the directory already exists, returns an error.
    ///
    /// # Arguments
    ///
    /// - `root`: Path to the working directory root
    /// - `repo_id`: Repository ID to link to
    /// - `channel`: Initial channel to track
    /// - `remote`: Optional remote node address
    #[instrument(skip(root, repo_id))]
    pub fn init(root: impl AsRef<Path>, repo_id: &RepoId, channel: &str, remote: Option<String>) -> PijulResult<Self> {
        let root = root.as_ref().to_path_buf();
        let metadata_dir = root.join(WORKING_DIR_METADATA_DIR);

        // Check channel name length
        if channel.len() > MAX_CHANNEL_NAME_LENGTH_BYTES as usize {
            return Err(PijulError::InvalidChannelName {
                channel: channel.to_string(),
            });
        }

        // Check if already initialized
        if metadata_dir.exists() {
            return Err(PijulError::WorkingDirAlreadyInitialized {
                path: root.display().to_string(),
            });
        }

        // Create metadata directory
        std::fs::create_dir_all(&metadata_dir).context(CreateDirSnafu {
            path: metadata_dir.clone(),
        })?;

        // Create pristine subdirectory
        let pristine_dir = metadata_dir.join(WORKING_DIR_PRISTINE_DIR);
        std::fs::create_dir_all(&pristine_dir).context(CreateDirSnafu {
            path: pristine_dir.clone(),
        })?;

        // Create empty staged file
        let staged_path = metadata_dir.join(WORKING_DIR_STAGED_FILE);
        std::fs::write(&staged_path, "").context(WriteFileSnafu {
            path: staged_path.clone(),
        })?;

        // Create configuration
        let config = WorkingDirectoryConfig::new(repo_id, channel, remote);
        let config_path = metadata_dir.join(WORKING_DIR_CONFIG_FILE);
        let config_toml =
            toml::to_string_pretty(&config).map_err(|e| PijulError::SerializeConfig { message: e.to_string() })?;
        std::fs::write(&config_path, config_toml).context(WriteFileSnafu {
            path: config_path.clone(),
        })?;

        info!(path = %root.display(), repo_id = %repo_id, channel = channel, "initialized working directory");

        // Create pristine manager using the metadata dir as base
        let pristine_mgr = PristineManager::new(&metadata_dir);

        Ok(Self {
            root,
            metadata_dir,
            config,
            pristine_mgr,
        })
    }

    /// Open an existing working directory.
    ///
    /// Reads the configuration from `.aspen/pijul/config.toml`.
    #[instrument(skip(root))]
    pub fn open(root: impl AsRef<Path>) -> PijulResult<Self> {
        let root = root.as_ref().to_path_buf();
        let metadata_dir = root.join(WORKING_DIR_METADATA_DIR);

        // Check if initialized
        if !metadata_dir.exists() {
            return Err(PijulError::WorkingDirNotInitialized {
                path: root.display().to_string(),
            });
        }

        // Read configuration with size limit (Tiger Style)
        let config_path = metadata_dir.join(WORKING_DIR_CONFIG_FILE);
        let metadata = std::fs::metadata(&config_path).context(StatFileSnafu {
            path: config_path.clone(),
        })?;
        if metadata.len() > MAX_WORKING_DIR_FILE_SIZE {
            return Err(PijulError::Io {
                message: format!("config file too large: {} bytes (max {})", metadata.len(), MAX_WORKING_DIR_FILE_SIZE),
            });
        }
        let config_toml = std::fs::read_to_string(&config_path).context(ReadFileToStringSnafu {
            path: config_path.clone(),
        })?;
        let config: WorkingDirectoryConfig = toml::from_str(&config_toml).map_err(|e| PijulError::ParseConfig {
            path: config_path.clone(),
            message: e.to_string(),
        })?;

        debug!(path = %root.display(), repo_id = %config.repo_id, "opened working directory");

        // Create pristine manager using the metadata dir as base
        let pristine_mgr = PristineManager::new(&metadata_dir);

        Ok(Self {
            root,
            metadata_dir,
            config,
            pristine_mgr,
        })
    }

    /// Find the working directory root by searching parent directories.
    ///
    /// Starts from the given path and walks up until `.aspen/pijul` is found.
    pub fn find(start: impl AsRef<Path>) -> PijulResult<Self> {
        let mut current = start.as_ref().to_path_buf();

        loop {
            let metadata_dir = current.join(WORKING_DIR_METADATA_DIR);
            if metadata_dir.exists() {
                return Self::open(&current);
            }

            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => {
                    return Err(PijulError::WorkingDirNotInitialized {
                        path: start.as_ref().display().to_string(),
                    });
                }
            }
        }
    }

    /// Get the working directory root path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get the metadata directory path.
    pub fn metadata_dir(&self) -> &Path {
        &self.metadata_dir
    }

    /// Get the current configuration.
    pub fn config(&self) -> &WorkingDirectoryConfig {
        &self.config
    }

    /// Get the repository ID.
    pub fn repo_id(&self) -> PijulResult<RepoId> {
        self.config.parse_repo_id()
    }

    /// Get the current channel.
    pub fn channel(&self) -> &str {
        &self.config.channel
    }

    /// Get the remote address.
    pub fn remote(&self) -> Option<&str> {
        self.config.remote.as_deref()
    }

    /// Get or create the local pristine handle.
    pub fn pristine(&self) -> PijulResult<PristineHandle> {
        let repo_id = self.repo_id()?;
        self.pristine_mgr.open_or_create(&repo_id)
    }

    // ========================================================================
    // Staged Files Management
    // ========================================================================

    /// Get the path to the staged files list.
    fn staged_path(&self) -> PathBuf {
        self.metadata_dir.join(WORKING_DIR_STAGED_FILE)
    }

    /// Read the list of staged files.
    ///
    /// Tiger Style: Validates file size before reading to prevent memory exhaustion.
    pub fn staged_files(&self) -> PijulResult<Vec<String>> {
        let path = self.staged_path();

        if !path.exists() {
            return Ok(Vec::new());
        }

        // Tiger Style: Check file size before reading
        let metadata = std::fs::metadata(&path).context(StatFileSnafu { path: path.clone() })?;
        if metadata.len() > MAX_WORKING_DIR_FILE_SIZE {
            return Err(PijulError::Io {
                message: format!("staged file too large: {} bytes (max {})", metadata.len(), MAX_WORKING_DIR_FILE_SIZE),
            });
        }

        let content = std::fs::read_to_string(&path).context(ReadFileToStringSnafu { path: path.clone() })?;

        Ok(content.lines().filter(|line| !line.is_empty()).map(|s| s.to_string()).collect())
    }

    /// Write the list of staged files.
    fn write_staged_files(&self, files: &[String]) -> PijulResult<()> {
        let path = self.staged_path();
        let content = files.join("\n");

        std::fs::write(&path, content).context(WriteFileSnafu { path })?;

        Ok(())
    }

    /// Add files to the staging area.
    ///
    /// Validates that paths exist and are within the working directory.
    /// Supports both files and directories (recursively adds all files).
    #[instrument(skip(self, paths))]
    pub fn add(&self, paths: &[impl AsRef<Path>]) -> PijulResult<AddResult> {
        let mut staged = self.staged_files()?;
        let staged_set: HashSet<_> = staged.iter().cloned().collect();
        let mut added = Vec::new();
        let mut already_staged = Vec::new();
        let mut not_found = Vec::new();

        for path in paths {
            let path = path.as_ref();

            // Resolve to absolute path
            let abs_path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                self.root.join(path)
            };

            // Check if path exists
            if !abs_path.exists() {
                not_found.push(path.display().to_string());
                continue;
            }

            // Get relative path from working directory root
            let rel_path = match abs_path.strip_prefix(&self.root) {
                Ok(p) => p.to_string_lossy().to_string(),
                Err(_) => {
                    return Err(PijulError::WorkingDirPathOutside {
                        path: path.display().to_string(),
                    });
                }
            };

            // Validate path length
            if rel_path.len() > MAX_PATH_LENGTH_BYTES as usize {
                return Err(PijulError::InvalidChange {
                    message: format!("path too long: {}", rel_path),
                });
            }

            // Skip metadata directory
            if rel_path.starts_with(".aspen") {
                continue;
            }

            // Handle directories by walking them
            if abs_path.is_dir() {
                self.add_directory_recursive(&abs_path, &staged_set, &mut staged, &mut added)?;
            } else {
                // Single file
                if staged_set.contains(&rel_path) {
                    already_staged.push(rel_path);
                } else {
                    staged.push(rel_path.clone());
                    added.push(rel_path);
                }
            }
        }

        // Check staged limit
        if staged.len() > MAX_STAGED_FILES as usize {
            return Err(PijulError::TooManyStagedFiles {
                count: staged.len() as u32,
                max: MAX_STAGED_FILES,
            });
        }

        // Write updated staged list
        self.write_staged_files(&staged)?;

        info!(added = added.len(), already = already_staged.len(), "staged files");

        Ok(AddResult {
            added,
            already_staged,
            not_found,
        })
    }

    /// Recursively add files from a directory.
    fn add_directory_recursive(
        &self,
        dir: &Path,
        staged_set: &HashSet<String>,
        staged: &mut Vec<String>,
        added: &mut Vec<String>,
    ) -> PijulResult<()> {
        let entries = std::fs::read_dir(dir).context(ReadDirSnafu {
            path: dir.to_path_buf(),
        })?;

        for entry in entries {
            let entry = entry.context(ReadDirEntrySnafu {
                path: dir.to_path_buf(),
            })?;

            let path = entry.path();
            let rel_path = match path.strip_prefix(&self.root) {
                Ok(p) => p.to_string_lossy().to_string(),
                Err(_) => continue,
            };

            // Skip metadata directory
            if rel_path.starts_with(".aspen") {
                continue;
            }

            // Check limit early
            if staged.len() >= MAX_STAGED_FILES as usize {
                return Err(PijulError::TooManyStagedFiles {
                    count: staged.len() as u32,
                    max: MAX_STAGED_FILES,
                });
            }

            if path.is_dir() {
                self.add_directory_recursive(&path, staged_set, staged, added)?;
            } else if !staged_set.contains(&rel_path) {
                staged.push(rel_path.clone());
                added.push(rel_path);
            }
        }

        Ok(())
    }

    /// Remove files from the staging area.
    ///
    /// Files are unstaged but not modified in the working directory.
    #[instrument(skip(self, paths))]
    pub fn reset(&self, paths: &[impl AsRef<Path>]) -> PijulResult<ResetResult> {
        let staged = self.staged_files()?;
        let paths_set: HashSet<_> = paths
            .iter()
            .map(|p| {
                let p = p.as_ref();
                if p.is_absolute() {
                    p.strip_prefix(&self.root)
                        .map(|r| r.to_string_lossy().to_string())
                        .unwrap_or_else(|_| p.display().to_string())
                } else {
                    p.display().to_string()
                }
            })
            .collect();

        let mut removed = Vec::new();
        let mut not_staged = Vec::new();
        let mut remaining = Vec::new();

        for file in &staged {
            if paths_set.contains(file) {
                removed.push(file.clone());
            } else {
                remaining.push(file.clone());
            }
        }

        // Check which requested paths were not staged
        for path in paths_set {
            if !staged.contains(&path) {
                not_staged.push(path);
            }
        }

        // Write updated staged list
        self.write_staged_files(&remaining)?;

        info!(removed = removed.len(), not_staged = not_staged.len(), "reset files");

        Ok(ResetResult { removed, not_staged })
    }

    /// Reset all staged files.
    pub fn reset_all(&self) -> PijulResult<ResetResult> {
        let staged = self.staged_files()?;
        self.write_staged_files(&[])?;

        info!(removed = staged.len(), "reset all staged files");

        Ok(ResetResult {
            removed: staged,
            not_staged: Vec::new(),
        })
    }

    // ========================================================================
    // Configuration Updates
    // ========================================================================

    /// Update the current channel.
    pub fn set_channel(&mut self, channel: &str) -> PijulResult<()> {
        if channel.len() > MAX_CHANNEL_NAME_LENGTH_BYTES as usize {
            return Err(PijulError::InvalidChannelName {
                channel: channel.to_string(),
            });
        }

        self.config.channel = channel.to_string();
        self.save_config()
    }

    /// Update the remote address.
    pub fn set_remote(&mut self, remote: Option<String>) -> PijulResult<()> {
        self.config.remote = remote;
        self.save_config()
    }

    /// Update the last synced head.
    pub fn set_last_synced(&mut self, head: Option<String>) -> PijulResult<()> {
        self.config.last_synced_head = head;
        self.config.last_synced_at_ms = Some(chrono::Utc::now().timestamp_millis() as u64);
        self.save_config()
    }

    /// Save configuration to disk.
    fn save_config(&self) -> PijulResult<()> {
        let config_path = self.metadata_dir.join(WORKING_DIR_CONFIG_FILE);
        let config_toml =
            toml::to_string_pretty(&self.config).map_err(|e| PijulError::SerializeConfig { message: e.to_string() })?;
        std::fs::write(&config_path, config_toml).context(WriteFileSnafu { path: config_path })?;
        Ok(())
    }
}

// ============================================================================
// Result Types
// ============================================================================

/// Result of adding files to the staging area.
#[derive(Debug, Clone)]
pub struct AddResult {
    /// Files that were added to staging.
    pub added: Vec<String>,
    /// Files that were already staged.
    pub already_staged: Vec<String>,
    /// Files that were not found.
    pub not_found: Vec<String>,
}

/// Result of resetting (unstaging) files.
#[derive(Debug, Clone)]
pub struct ResetResult {
    /// Files that were removed from staging.
    pub removed: Vec<String>,
    /// Files that were not staged.
    pub not_staged: Vec<String>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn test_repo_id() -> RepoId {
        RepoId([1u8; 32])
    }

    #[test]
    fn test_init_working_directory() {
        let tmp = TempDir::new().unwrap();
        let repo_id = test_repo_id();

        let wd = WorkingDirectory::init(tmp.path(), &repo_id, "main", None).unwrap();

        assert_eq!(wd.channel(), "main");
        assert!(wd.remote().is_none());
        assert!(wd.metadata_dir().exists());
        assert!(wd.staged_files().unwrap().is_empty());
    }

    #[test]
    fn test_init_already_initialized() {
        let tmp = TempDir::new().unwrap();
        let repo_id = test_repo_id();

        WorkingDirectory::init(tmp.path(), &repo_id, "main", None).unwrap();

        // Second init should fail
        let result = WorkingDirectory::init(tmp.path(), &repo_id, "main", None);
        assert!(matches!(result, Err(PijulError::WorkingDirAlreadyInitialized { .. })));
    }

    #[test]
    fn test_open_working_directory() {
        let tmp = TempDir::new().unwrap();
        let repo_id = test_repo_id();

        WorkingDirectory::init(tmp.path(), &repo_id, "develop", Some("node:abc".to_string())).unwrap();

        let wd = WorkingDirectory::open(tmp.path()).unwrap();
        assert_eq!(wd.channel(), "develop");
        assert_eq!(wd.remote(), Some("node:abc"));
    }

    #[test]
    fn test_open_not_initialized() {
        let tmp = TempDir::new().unwrap();

        let result = WorkingDirectory::open(tmp.path());
        assert!(matches!(result, Err(PijulError::WorkingDirNotInitialized { .. })));
    }

    #[test]
    fn test_add_files() {
        let tmp = TempDir::new().unwrap();
        let repo_id = test_repo_id();

        // Create some test files
        std::fs::write(tmp.path().join("file1.txt"), "content1").unwrap();
        std::fs::write(tmp.path().join("file2.txt"), "content2").unwrap();

        let wd = WorkingDirectory::init(tmp.path(), &repo_id, "main", None).unwrap();

        let result = wd.add(&["file1.txt", "file2.txt"]).unwrap();
        assert_eq!(result.added.len(), 2);
        assert!(result.already_staged.is_empty());
        assert!(result.not_found.is_empty());

        let staged = wd.staged_files().unwrap();
        assert!(staged.contains(&"file1.txt".to_string()));
        assert!(staged.contains(&"file2.txt".to_string()));
    }

    #[test]
    fn test_add_already_staged() {
        let tmp = TempDir::new().unwrap();
        let repo_id = test_repo_id();

        std::fs::write(tmp.path().join("file.txt"), "content").unwrap();

        let wd = WorkingDirectory::init(tmp.path(), &repo_id, "main", None).unwrap();

        wd.add(&["file.txt"]).unwrap();

        // Add again
        let result = wd.add(&["file.txt"]).unwrap();
        assert!(result.added.is_empty());
        assert_eq!(result.already_staged.len(), 1);
    }

    #[test]
    fn test_add_directory() {
        let tmp = TempDir::new().unwrap();
        let repo_id = test_repo_id();

        // Create a directory with files
        let subdir = tmp.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("file1.txt"), "content1").unwrap();
        std::fs::write(subdir.join("file2.txt"), "content2").unwrap();

        let wd = WorkingDirectory::init(tmp.path(), &repo_id, "main", None).unwrap();

        let result = wd.add(&["subdir"]).unwrap();
        assert_eq!(result.added.len(), 2);

        let staged = wd.staged_files().unwrap();
        assert!(staged.iter().any(|s| s.contains("file1.txt")));
        assert!(staged.iter().any(|s| s.contains("file2.txt")));
    }

    #[test]
    fn test_reset_files() {
        let tmp = TempDir::new().unwrap();
        let repo_id = test_repo_id();

        std::fs::write(tmp.path().join("file1.txt"), "content1").unwrap();
        std::fs::write(tmp.path().join("file2.txt"), "content2").unwrap();

        let wd = WorkingDirectory::init(tmp.path(), &repo_id, "main", None).unwrap();

        wd.add(&["file1.txt", "file2.txt"]).unwrap();

        let result = wd.reset(&["file1.txt"]).unwrap();
        assert_eq!(result.removed.len(), 1);
        assert!(result.not_staged.is_empty());

        let staged = wd.staged_files().unwrap();
        assert!(!staged.contains(&"file1.txt".to_string()));
        assert!(staged.contains(&"file2.txt".to_string()));
    }

    #[test]
    fn test_reset_all() {
        let tmp = TempDir::new().unwrap();
        let repo_id = test_repo_id();

        std::fs::write(tmp.path().join("file1.txt"), "content1").unwrap();
        std::fs::write(tmp.path().join("file2.txt"), "content2").unwrap();

        let wd = WorkingDirectory::init(tmp.path(), &repo_id, "main", None).unwrap();

        wd.add(&["file1.txt", "file2.txt"]).unwrap();

        let result = wd.reset_all().unwrap();
        assert_eq!(result.removed.len(), 2);

        assert!(wd.staged_files().unwrap().is_empty());
    }

    #[test]
    fn test_find_working_directory() {
        let tmp = TempDir::new().unwrap();
        let repo_id = test_repo_id();

        // Create nested directory
        let nested = tmp.path().join("a").join("b").join("c");
        std::fs::create_dir_all(&nested).unwrap();

        WorkingDirectory::init(tmp.path(), &repo_id, "main", None).unwrap();

        // Find from nested directory should work
        let wd = WorkingDirectory::find(&nested).unwrap();
        assert_eq!(wd.root(), tmp.path());
    }

    #[test]
    fn test_set_channel() {
        let tmp = TempDir::new().unwrap();
        let repo_id = test_repo_id();

        let mut wd = WorkingDirectory::init(tmp.path(), &repo_id, "main", None).unwrap();
        assert_eq!(wd.channel(), "main");

        wd.set_channel("feature").unwrap();
        assert_eq!(wd.channel(), "feature");

        // Reload and verify
        let wd2 = WorkingDirectory::open(tmp.path()).unwrap();
        assert_eq!(wd2.channel(), "feature");
    }

    #[test]
    fn test_skips_aspen_metadata() {
        let tmp = TempDir::new().unwrap();
        let repo_id = test_repo_id();

        let wd = WorkingDirectory::init(tmp.path(), &repo_id, "main", None).unwrap();

        // Try to add the metadata directory (should be skipped)
        let result = wd.add(&[".aspen"]).unwrap();
        assert!(result.added.is_empty());
    }
}
