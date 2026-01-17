//! Pijul version control commands.
//!
//! Commands for managing Pijul repositories, channels, and changes
//! using Aspen's distributed storage with iroh-blobs and Raft consensus.
//!
//! ## Local Mode
//!
//! The `record` and `checkout` commands support a `--data-dir` option for
//! local operation. When specified, these commands use a local pristine
//! database while still syncing changes to the remote cluster.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use aspen_blob::InMemoryBlobStore;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_forge::identity::RepoId;
use aspen_pijul::AspenChangeStore;
use aspen_pijul::ChangeDirectory;
use aspen_pijul::ChangeMetadata;
use aspen_pijul::ChangeRecorder;
use aspen_pijul::PijulAuthor;
use aspen_pijul::PristineManager;
use aspen_pijul::ResolutionStrategy;
use aspen_pijul::WorkingDirectory;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;
use crate::output::print_success;

/// Get the default cache directory for Pijul pristines.
///
/// Uses `~/.cache/aspen/pijul/` on Unix and `%LOCALAPPDATA%/aspen/pijul/` on Windows.
fn default_cache_dir() -> Result<PathBuf> {
    // Try XDG_CACHE_HOME first (Unix standard)
    if let Ok(cache) = std::env::var("XDG_CACHE_HOME") {
        return Ok(PathBuf::from(cache).join("aspen").join("pijul"));
    }

    // Fall back to platform-specific defaults
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            return Ok(PathBuf::from(local_app_data).join("aspen").join("pijul"));
        }
    }

    // Fall back to $HOME/.cache on Unix or %USERPROFILE% on Windows
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .context("could not determine home directory")?;

    Ok(PathBuf::from(home).join(".cache").join("aspen").join("pijul"))
}

/// Get the cache directory for a specific repository.
fn repo_cache_dir(repo_id: &RepoId) -> Result<PathBuf> {
    Ok(default_cache_dir()?.join(repo_id.to_hex()))
}

/// Pijul version control operations.
///
/// Manage patch-based version control with P2P distribution via iroh-blobs
/// and strongly consistent channel refs via Raft consensus.
#[derive(Subcommand)]
pub enum PijulCommand {
    /// Repository management.
    #[command(subcommand)]
    Repo(RepoCommand),

    /// Channel (branch) management.
    #[command(subcommand)]
    Channel(ChannelCommand),

    /// Working directory management.
    #[command(subcommand)]
    Wd(WdCommand),

    /// Record changes from working directory.
    Record(RecordArgs),

    /// Apply a change to a channel.
    Apply(ApplyArgs),

    /// Unrecord (remove) a change from a channel.
    Unrecord(UnrecordArgs),

    /// Show change log for a channel.
    Log(LogArgs),

    /// Output pristine state to working directory.
    Checkout(CheckoutArgs),

    /// Sync local pristine with cluster state.
    ///
    /// Downloads missing changes from the cluster and applies them to the
    /// local pristine cache. This is required before recording if your
    /// local cache is out of date.
    Sync(SyncArgs),

    /// Pull changes from the cluster to local pristine.
    ///
    /// Downloads and applies changes from the cluster that are not yet
    /// in the local pristine. This is the read-only direction of sync.
    Pull(PullArgs),

    /// Push local changes to the cluster.
    ///
    /// Uploads changes from the local pristine that are not yet in the
    /// cluster. This is the write direction of sync - it uploads change
    /// blobs and updates channel heads via Raft consensus.
    Push(PushArgs),

    /// Show differences between working directory and pristine.
    ///
    /// Shows what changes would be recorded, without actually recording them.
    /// Can also compare differences between two channels.
    Diff(DiffArgs),

    /// Export repository state as archive (directory or tarball).
    ///
    /// Creates an archive of the repository state at the current channel head.
    /// The output can be a directory or a tarball (.tar or .tar.gz).
    Archive(ArchiveArgs),

    /// Show details of a specific change.
    ///
    /// Displays metadata about a change including author, message, timestamp,
    /// dependencies, and size.
    Show(ShowArgs),

    /// Show change attribution for a file.
    ///
    /// Lists the changes that have contributed to the current state of a file.
    /// Currently shows change-level attribution; per-line blame is planned.
    Blame(BlameArgs),

    /// Manage Pijul configuration.
    ///
    /// Get, set, or list configuration values. Supports both local (working directory)
    /// and global (~/.config/aspen-pijul/) configuration.
    #[command(subcommand)]
    Config(ConfigCommand),

    /// Manage remote repositories.
    ///
    /// Add, remove, or list remote repositories for sync operations.
    #[command(subcommand)]
    Remote(RemoteCommand),
}

// =============================================================================
// Repository Commands
// =============================================================================

/// Repository management commands.
#[derive(Subcommand)]
pub enum RepoCommand {
    /// Initialize a new Pijul repository.
    Init(RepoInitArgs),

    /// List all repositories.
    List(RepoListArgs),

    /// Get repository information.
    Info(RepoInfoArgs),
}

#[derive(Args)]
pub struct RepoInitArgs {
    /// Repository name.
    pub name: String,

    /// Repository description.
    #[arg(short, long)]
    pub description: Option<String>,

    /// Default channel name (default: "main").
    #[arg(long, default_value = "main")]
    pub default_channel: String,
}

#[derive(Args)]
pub struct RepoListArgs {
    /// Maximum number of repositories to return.
    #[arg(long, default_value = "100")]
    pub limit: u32,
}

#[derive(Args)]
pub struct RepoInfoArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,
}

// =============================================================================
// Channel Commands
// =============================================================================

/// Channel (branch) management commands.
#[derive(Subcommand)]
pub enum ChannelCommand {
    /// List channels in a repository.
    List(ChannelListArgs),

    /// Create a new channel.
    Create(ChannelCreateArgs),

    /// Delete a channel.
    Delete(ChannelDeleteArgs),

    /// Fork a channel.
    Fork(ChannelForkArgs),

    /// Get channel information.
    Info(ChannelInfoArgs),
}

#[derive(Args)]
pub struct ChannelListArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,
}

#[derive(Args)]
pub struct ChannelCreateArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Channel name.
    pub name: String,
}

#[derive(Args)]
pub struct ChannelDeleteArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Channel name.
    pub name: String,
}

#[derive(Args)]
pub struct ChannelForkArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Source channel name.
    pub source: String,

    /// New channel name.
    pub target: String,
}

#[derive(Args)]
pub struct ChannelInfoArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Channel name.
    pub name: String,
}

// =============================================================================
// Working Directory Commands
// =============================================================================

/// Working directory management commands.
#[derive(Subcommand)]
pub enum WdCommand {
    /// Initialize a working directory linked to a repository.
    ///
    /// Creates a `.aspen/pijul/` metadata directory that tracks the remote
    /// repository, current channel, and staged files.
    Init(WdInitArgs),

    /// Add files to the staging area.
    ///
    /// Staged files will be included in the next `pijul record` command.
    /// Supports both files and directories (recursively adds all files).
    Add(WdAddArgs),

    /// Remove files from the staging area.
    ///
    /// Unstages files but does not modify them in the working directory.
    Reset(WdResetArgs),

    /// Show working directory status.
    ///
    /// Displays staged files, modified files, and untracked files.
    Status(WdStatusArgs),

    /// Record staged changes to the current channel.
    ///
    /// Records changes from the working directory and pushes them to the cluster.
    /// Auto-detects repo_id and channel from the working directory config.
    Record(WdRecordArgs),

    /// Checkout the current channel to working directory.
    ///
    /// Syncs with the cluster and outputs the channel state to the working directory.
    /// Auto-detects repo_id and channel from the working directory config.
    Checkout(WdCheckoutArgs),

    /// Show diff between working directory and channel.
    ///
    /// Shows what changes would be recorded without actually recording them.
    /// Auto-detects repo_id and channel from the working directory config.
    Diff(WdDiffArgs),

    /// List conflicts in the working directory.
    ///
    /// Shows files with conflicts that need to be resolved.
    /// Conflicts can occur when applying changes that modify the same files.
    Conflicts(WdConflictsArgs),

    /// Resolve conflicts using a strategy.
    ///
    /// Applies a resolution strategy to conflicting files.
    /// Use --list to see available strategies.
    Solve(WdSolveArgs),
}

#[derive(Args)]
pub struct WdInitArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Working directory path (default: current directory).
    #[arg(short, long)]
    pub path: Option<String>,

    /// Initial channel to track (default: main).
    #[arg(short, long, default_value = "main")]
    pub channel: String,

    /// Remote node address for syncing.
    ///
    /// Format: node_id@host:port or ticket string.
    #[arg(long)]
    pub remote: Option<String>,
}

#[derive(Args)]
pub struct WdAddArgs {
    /// Files or directories to stage.
    ///
    /// Relative to the working directory root. Use "." for all files.
    #[arg(required = true)]
    pub paths: Vec<String>,

    /// Working directory path (default: find from current directory).
    #[arg(long)]
    pub path: Option<String>,
}

#[derive(Args)]
pub struct WdResetArgs {
    /// Files to unstage.
    ///
    /// If not specified, unstages all files.
    pub paths: Vec<String>,

    /// Unstage all files.
    #[arg(long)]
    pub all: bool,

    /// Working directory path (default: find from current directory).
    #[arg(long)]
    pub path: Option<String>,
}

#[derive(Args)]
pub struct WdStatusArgs {
    /// Working directory path (default: find from current directory).
    #[arg(long)]
    pub path: Option<String>,

    /// Show only staged files.
    #[arg(long)]
    pub staged: bool,

    /// Output in machine-readable format.
    ///
    /// Format: XY PATH
    /// Where X is staging status and Y is working tree status:
    ///   A = Added (staged)
    ///   M = Modified
    ///   D = Deleted
    ///   ? = Untracked
    ///   ' ' = Unmodified
    #[arg(long)]
    pub porcelain: bool,
}

#[derive(Args)]
pub struct WdRecordArgs {
    /// Change message (required).
    #[arg(short, long)]
    pub message: String,

    /// Author name (default: from git config or $USER).
    #[arg(long)]
    pub author: Option<String>,

    /// Author email.
    #[arg(long)]
    pub email: Option<String>,

    /// Working directory path (default: find from current directory).
    #[arg(long)]
    pub path: Option<String>,

    /// Push to cluster after recording (default: true).
    #[arg(long, default_value = "true")]
    pub push: bool,

    /// Number of threads for parallel file scanning.
    #[arg(long, default_value = "1")]
    pub threads: usize,
}

#[derive(Args)]
pub struct WdCheckoutArgs {
    /// Channel to checkout (default: current from config).
    #[arg(short, long)]
    pub channel: Option<String>,

    /// Working directory path (default: find from current directory).
    #[arg(long)]
    pub path: Option<String>,

    /// Pull from cluster before checkout (default: true).
    #[arg(long, default_value = "true")]
    pub pull: bool,
}

#[derive(Args)]
pub struct WdDiffArgs {
    /// Working directory path (default: find from current directory).
    #[arg(long)]
    pub path: Option<String>,

    /// Show summary only.
    #[arg(long)]
    pub summary: bool,
}

#[derive(Args)]
pub struct WdConflictsArgs {
    /// Working directory path (default: find from current directory).
    #[arg(long)]
    pub path: Option<String>,

    /// Show detailed conflict information including markers.
    #[arg(long)]
    pub detailed: bool,
}

#[derive(Args)]
pub struct WdSolveArgs {
    /// File path(s) to resolve (relative to working directory).
    ///
    /// If not specified with --all, you must provide at least one path.
    pub paths: Vec<String>,

    /// Working directory path (default: find from current directory).
    #[arg(long)]
    pub path: Option<String>,

    /// Resolution strategy to use.
    ///
    /// Available strategies: ours, theirs, newest, oldest, manual
    #[arg(long, short, default_value = "manual")]
    pub strategy: String,

    /// Resolve all conflicts using the specified strategy.
    #[arg(long)]
    pub all: bool,

    /// List available resolution strategies and exit.
    #[arg(long)]
    pub list: bool,
}

// =============================================================================
// Change Commands
// =============================================================================

#[derive(Args)]
pub struct RecordArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Channel to record changes to.
    #[arg(short, long, default_value = "main")]
    pub channel: String,

    /// Working directory path.
    #[arg(short, long)]
    pub working_dir: String,

    /// Change message.
    #[arg(short, long)]
    pub message: String,

    /// Author name.
    #[arg(long)]
    pub author: Option<String>,

    /// Author email.
    #[arg(long)]
    pub email: Option<String>,

    /// Local data directory for pristine storage.
    ///
    /// The CLI uses a local pristine database for recording changes,
    /// then uploads the change to the cluster's blob store and updates
    /// the channel head via Raft.
    ///
    /// Defaults to ~/.cache/aspen/pijul/<repo_id>/ if not specified.
    /// Use `pijul sync` to sync the local pristine before recording.
    #[arg(long)]
    pub data_dir: Option<PathBuf>,

    /// Number of threads for parallel file scanning.
    ///
    /// Using multiple threads can significantly speed up recording for
    /// large repositories. Recommended value is the number of CPU cores.
    #[arg(long, default_value = "1")]
    pub threads: usize,

    /// Prefix to limit recording scope.
    ///
    /// Only files under this path prefix will be scanned and recorded.
    /// Useful for large repositories where you only want to record
    /// changes in a specific directory (e.g., "src/pijul").
    #[arg(long)]
    pub prefix: Option<String>,
}

#[derive(Args)]
pub struct ArchiveArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Channel to archive (default: main).
    #[arg(short, long, default_value = "main")]
    pub channel: String,

    /// Output path (directory or .tar/.tar.gz file).
    ///
    /// If the path ends in .tar or .tar.gz, creates a tarball.
    /// Otherwise, exports to a directory.
    #[arg(short, long)]
    pub output_path: String,

    /// Local data directory (default: ~/.cache/aspen/pijul/<repo_id>).
    #[arg(long)]
    pub data_dir: Option<PathBuf>,

    /// Prefix to limit archive scope.
    ///
    /// Only files under this path prefix will be included in the archive.
    #[arg(long)]
    pub prefix: Option<String>,
}

#[derive(Args)]
pub struct ApplyArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Channel to apply change to.
    #[arg(short, long, default_value = "main")]
    pub channel: String,

    /// Change hash (hex-encoded BLAKE3).
    pub change_hash: String,
}

#[derive(Args)]
pub struct UnrecordArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Channel to unrecord change from.
    #[arg(short, long, default_value = "main")]
    pub channel: String,

    /// Change hash (hex-encoded BLAKE3).
    pub change_hash: String,
}

#[derive(Args)]
pub struct LogArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Channel name.
    #[arg(short, long, default_value = "main")]
    pub channel: String,

    /// Maximum number of changes to show.
    #[arg(short = 'n', long, default_value = "20")]
    pub limit: u32,
}

#[derive(Args)]
pub struct CheckoutArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Channel to checkout.
    #[arg(short, long, default_value = "main")]
    pub channel: String,

    /// Output directory path.
    #[arg(short, long)]
    pub output_dir: String,

    /// Use local pristine for checkout (faster, works offline).
    ///
    /// When specified, checkout uses the local pristine cache instead
    /// of requesting files from the remote server. Requires running
    /// `pijul sync` first to populate the local cache.
    #[arg(long)]
    pub local: bool,

    /// Local data directory for pristine storage.
    ///
    /// Defaults to ~/.cache/aspen/pijul/<repo_id>/ if not specified.
    #[arg(long)]
    pub data_dir: Option<PathBuf>,
}

#[derive(Args)]
pub struct SyncArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Channel to sync (default: all channels).
    #[arg(short, long)]
    pub channel: Option<String>,

    /// Force rebuild of local pristine from scratch.
    ///
    /// This discards any local-only state and rebuilds the pristine
    /// entirely from the cluster's change log.
    #[arg(long)]
    pub rebuild: bool,

    /// Local data directory (default: ~/.cache/aspen/pijul/<repo_id>).
    #[arg(long)]
    pub data_dir: Option<PathBuf>,
}

#[derive(Args)]
pub struct PullArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Channel to pull (default: all channels).
    #[arg(short, long)]
    pub channel: Option<String>,

    /// Local data directory (default: ~/.cache/aspen/pijul/<repo_id>).
    #[arg(long)]
    pub data_dir: Option<PathBuf>,
}

#[derive(Args)]
pub struct PushArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Channel to push (default: main).
    #[arg(short, long, default_value = "main")]
    pub channel: String,

    /// Local data directory (default: ~/.cache/aspen/pijul/<repo_id>).
    #[arg(long)]
    pub data_dir: Option<PathBuf>,
}

#[derive(Args)]
pub struct DiffArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Channel to compare against (default: main).
    ///
    /// When comparing working directory changes, this is the channel
    /// to diff against. When comparing two channels, this is the first channel.
    #[arg(short, long, default_value = "main")]
    pub channel: String,

    /// Second channel to compare (for channel-to-channel diff).
    ///
    /// If specified, shows changes that are in channel2 but not in channel1.
    /// If not specified, shows working directory changes against channel1.
    #[arg(long)]
    pub channel2: Option<String>,

    /// Working directory path.
    ///
    /// Required when diffing working directory against a channel.
    /// Not used when comparing two channels.
    #[arg(short, long)]
    pub working_dir: Option<String>,

    /// Local data directory (default: ~/.cache/aspen/pijul/<repo_id>).
    #[arg(long)]
    pub data_dir: Option<PathBuf>,

    /// Show summary only (file counts, not content).
    #[arg(long)]
    pub summary: bool,

    /// Number of threads for parallel file scanning.
    #[arg(long, default_value = "1")]
    pub threads: usize,

    /// Prefix to limit diff scope.
    #[arg(long)]
    pub prefix: Option<String>,
}

#[derive(Args)]
pub struct ShowArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Change hash (full or partial, hex-encoded BLAKE3).
    pub change_hash: String,
}

#[derive(Args)]
pub struct BlameArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Channel name.
    #[arg(default_value = "main")]
    pub channel: String,

    /// File path to get blame for.
    pub path: String,
}

/// Configuration management commands.
#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Get a configuration value.
    Get {
        /// Configuration key (e.g., "user.name", "user.email").
        key: String,
        /// Use global config instead of local.
        #[arg(long)]
        global: bool,
    },

    /// Set a configuration value.
    Set {
        /// Configuration key (e.g., "user.name", "user.email").
        key: String,
        /// Value to set.
        value: String,
        /// Use global config instead of local.
        #[arg(long)]
        global: bool,
    },

    /// List all configuration values.
    List {
        /// Use global config instead of local.
        #[arg(long)]
        global: bool,
    },

    /// Unset (remove) a configuration value.
    Unset {
        /// Configuration key to remove.
        key: String,
        /// Use global config instead of local.
        #[arg(long)]
        global: bool,
    },
}

/// Remote repository management commands.
#[derive(Subcommand)]
pub enum RemoteCommand {
    /// List configured remotes.
    List,

    /// Add a remote.
    Add {
        /// Remote name (e.g., "origin", "upstream").
        name: String,
        /// Remote URL (ticket or Iroh node ID).
        url: String,
    },

    /// Remove a remote.
    Remove {
        /// Remote name to remove.
        name: String,
    },

    /// Show remote details.
    Show {
        /// Remote name.
        name: String,
    },
}

// =============================================================================

/// Pijul repository output.
pub struct PijulRepoOutput {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub default_channel: String,
    pub channel_count: u32,
    pub created_at_ms: u64,
}

impl Outputable for PijulRepoOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "name": self.name,
            "description": self.description,
            "default_channel": self.default_channel,
            "channel_count": self.channel_count,
            "created_at_ms": self.created_at_ms
        })
    }

    fn to_human(&self) -> String {
        let desc = self.description.as_deref().unwrap_or("-");
        format!(
            "Repository: {}\n\
             ID:              {}\n\
             Default Channel: {}\n\
             Channels:        {}\n\
             Description:     {}",
            self.name, self.id, self.default_channel, self.channel_count, desc
        )
    }
}

/// Pijul repository list output.
pub struct PijulRepoListOutput {
    pub repos: Vec<PijulRepoOutput>,
    pub count: u32,
}

impl Outputable for PijulRepoListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.count,
            "repos": self.repos.iter().map(|r| r.to_json()).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.repos.is_empty() {
            return "No repositories found".to_string();
        }

        let mut output = format!("Repositories ({}):\n\n", self.count);
        for repo in &self.repos {
            let desc = repo.description.as_deref().unwrap_or("");
            let desc_preview = if desc.len() > 40 {
                format!("{}...", &desc[..37])
            } else {
                desc.to_string()
            };
            output.push_str(&format!(
                "  {} ({}) - {}\n",
                repo.name,
                &repo.id[..16.min(repo.id.len())],
                if desc_preview.is_empty() { "-" } else { &desc_preview }
            ));
        }
        output
    }
}

/// Pijul channel output.
pub struct PijulChannelOutput {
    pub name: String,
    pub head: Option<String>,
    pub updated_at_ms: u64,
}

impl Outputable for PijulChannelOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "head": self.head,
            "updated_at_ms": self.updated_at_ms
        })
    }

    fn to_human(&self) -> String {
        let head = self.head.as_deref().unwrap_or("(empty)");
        let head_short = if head.len() > 16 { &head[..16] } else { head };
        format!("{:<20} -> {}", self.name, head_short)
    }
}

/// Pijul channel list output.
pub struct PijulChannelListOutput {
    pub channels: Vec<PijulChannelOutput>,
    pub count: u32,
}

impl Outputable for PijulChannelListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.count,
            "channels": self.channels.iter().map(|c| c.to_json()).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.channels.is_empty() {
            return "No channels found".to_string();
        }

        let mut output = format!("Channels ({}):\n\n", self.count);
        for channel in &self.channels {
            output.push_str(&format!("  {}\n", channel.to_human()));
        }
        output
    }
}

/// Pijul record result output.
pub struct PijulRecordOutput {
    pub change_hash: String,
    pub message: String,
    pub hunks: u32,
    pub size_bytes: u64,
}

impl Outputable for PijulRecordOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "change_hash": self.change_hash,
            "message": self.message,
            "hunks": self.hunks,
            "size_bytes": self.size_bytes
        })
    }

    fn to_human(&self) -> String {
        format!(
            "Recorded change {}\n\
             Message: {}\n\
             Hunks:   {}\n\
             Size:    {} bytes",
            &self.change_hash[..16.min(self.change_hash.len())],
            self.message,
            self.hunks,
            self.size_bytes
        )
    }
}

/// Pijul apply result output.
pub struct PijulApplyOutput {
    pub change_hash: String,
    pub channel: String,
    pub operations: u64,
}

impl Outputable for PijulApplyOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "change_hash": self.change_hash,
            "channel": self.channel,
            "operations": self.operations
        })
    }

    fn to_human(&self) -> String {
        format!(
            "Applied {} to channel '{}' ({} operations)",
            &self.change_hash[..16.min(self.change_hash.len())],
            self.channel,
            self.operations
        )
    }
}

/// Pijul unrecord result output.
pub struct PijulUnrecordOutput {
    pub change_hash: String,
    pub channel: String,
    pub unrecorded: bool,
}

impl Outputable for PijulUnrecordOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "change_hash": self.change_hash,
            "channel": self.channel,
            "unrecorded": self.unrecorded
        })
    }

    fn to_human(&self) -> String {
        if self.unrecorded {
            format!(
                "Unrecorded {} from channel '{}'",
                &self.change_hash[..16.min(self.change_hash.len())],
                self.channel
            )
        } else {
            format!(
                "Change {} was not in channel '{}'",
                &self.change_hash[..16.min(self.change_hash.len())],
                self.channel
            )
        }
    }
}

/// Pijul change log entry.
pub struct PijulLogEntry {
    pub change_hash: String,
    pub message: String,
    pub author: Option<String>,
    pub timestamp_ms: u64,
}

impl Outputable for PijulLogEntry {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "change_hash": self.change_hash,
            "message": self.message,
            "author": self.author,
            "timestamp_ms": self.timestamp_ms
        })
    }

    fn to_human(&self) -> String {
        let author = self.author.as_deref().unwrap_or("unknown");
        format!(
            "change {}\n\
             Author: {}\n\n\
             {}",
            &self.change_hash[..16.min(self.change_hash.len())],
            author,
            self.message.lines().map(|l| format!("    {}", l)).collect::<Vec<_>>().join("\n")
        )
    }
}

/// Pijul log output.
pub struct PijulLogOutput {
    pub entries: Vec<PijulLogEntry>,
    pub count: u32,
}

impl Outputable for PijulLogOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.count,
            "entries": self.entries.iter().map(|e| e.to_json()).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.entries.is_empty() {
            return "No changes found".to_string();
        }

        self.entries.iter().map(|e| e.to_human()).collect::<Vec<_>>().join("\n\n")
    }
}

/// Pijul checkout result output.
pub struct PijulCheckoutOutput {
    pub channel: String,
    pub output_dir: String,
    pub files_written: u32,
    pub conflicts: u32,
}

impl Outputable for PijulCheckoutOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "channel": self.channel,
            "output_dir": self.output_dir,
            "files_written": self.files_written,
            "conflicts": self.conflicts
        })
    }

    fn to_human(&self) -> String {
        if self.conflicts > 0 {
            format!(
                "Checked out '{}' to {}\n\
                 Files written: {}\n\
                 Conflicts:     {} (review manually)",
                self.channel, self.output_dir, self.files_written, self.conflicts
            )
        } else {
            format!(
                "Checked out '{}' to {}\n\
                 Files written: {}",
                self.channel, self.output_dir, self.files_written
            )
        }
    }
}

/// Pijul sync result output.
pub struct PijulSyncOutput {
    pub repo_id: String,
    pub channel: Option<String>,
    pub changes_fetched: u32,
    pub changes_applied: u32,
    pub already_synced: bool,
    pub conflicts: u32,
    pub cache_dir: String,
}

impl Outputable for PijulSyncOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "repo_id": self.repo_id,
            "channel": self.channel,
            "changes_fetched": self.changes_fetched,
            "changes_applied": self.changes_applied,
            "already_synced": self.already_synced,
            "conflicts": self.conflicts,
            "cache_dir": self.cache_dir
        })
    }

    fn to_human(&self) -> String {
        let channel_str = self.channel.as_deref().unwrap_or("all channels");
        if self.already_synced {
            format!(
                "Sync complete for {}\n\
                 Status:    Already up to date\n\
                 Cache:     {}",
                channel_str, self.cache_dir
            )
        } else if self.conflicts > 0 {
            format!(
                "Sync complete for {}\n\
                 Fetched:   {} changes\n\
                 Applied:   {} changes\n\
                 Conflicts: {} (review after checkout)\n\
                 Cache:     {}",
                channel_str, self.changes_fetched, self.changes_applied, self.conflicts, self.cache_dir
            )
        } else {
            format!(
                "Sync complete for {}\n\
                 Fetched:   {} changes\n\
                 Applied:   {} changes\n\
                 Cache:     {}",
                channel_str, self.changes_fetched, self.changes_applied, self.cache_dir
            )
        }
    }
}

/// A single hunk in a diff showing changes to a file.
#[derive(Debug, Clone)]
pub struct DiffHunk {
    /// Path of the file being changed.
    pub path: String,
    /// Type of change: "add", "delete", "modify", "rename", "permission".
    pub change_type: String,
    /// Number of lines added.
    pub additions: u32,
    /// Number of lines deleted.
    pub deletions: u32,
}

impl Outputable for DiffHunk {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "path": self.path,
            "change_type": self.change_type,
            "additions": self.additions,
            "deletions": self.deletions
        })
    }

    fn to_human(&self) -> String {
        let symbol = match self.change_type.as_str() {
            "add" => "+",
            "delete" => "-",
            "modify" => "M",
            "rename" => "R",
            "permission" => "P",
            _ => "?",
        };
        if self.additions > 0 || self.deletions > 0 {
            format!("{} {} (+{}, -{})", symbol, self.path, self.additions, self.deletions)
        } else {
            format!("{} {}", symbol, self.path)
        }
    }
}

/// Pijul diff result output.
pub struct PijulDiffOutput {
    /// Repository ID.
    pub repo_id: String,
    /// First channel (base).
    pub channel: String,
    /// Second channel if comparing channels, None if comparing working dir.
    pub channel2: Option<String>,
    /// Working directory path if comparing working dir.
    pub working_dir: Option<String>,
    /// List of changes (hunks).
    pub hunks: Vec<DiffHunk>,
    /// Total files added.
    pub files_added: u32,
    /// Total files deleted.
    pub files_deleted: u32,
    /// Total files modified.
    pub files_modified: u32,
    /// Total lines added.
    pub lines_added: u32,
    /// Total lines deleted.
    pub lines_deleted: u32,
    /// Whether there are no changes.
    pub no_changes: bool,
}

impl Outputable for PijulDiffOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "repo_id": self.repo_id,
            "channel": self.channel,
            "channel2": self.channel2,
            "working_dir": self.working_dir,
            "hunks": self.hunks.iter().map(|h| h.to_json()).collect::<Vec<_>>(),
            "files_added": self.files_added,
            "files_deleted": self.files_deleted,
            "files_modified": self.files_modified,
            "lines_added": self.lines_added,
            "lines_deleted": self.lines_deleted,
            "no_changes": self.no_changes
        })
    }

    fn to_human(&self) -> String {
        if self.no_changes {
            return "No changes".to_string();
        }

        let mut output = String::new();

        // Header
        if let Some(ref channel2) = self.channel2 {
            output.push_str(&format!("Diff between channels '{}' and '{}'\n\n", self.channel, channel2));
        } else if let Some(ref working_dir) = self.working_dir {
            output.push_str(&format!("Diff of '{}' against channel '{}'\n\n", working_dir, self.channel));
        }

        // List of changes
        for hunk in &self.hunks {
            output.push_str(&format!("  {}\n", hunk.to_human()));
        }

        // Summary
        output.push_str(&format!(
            "\n{} files changed: {} added, {} deleted, {} modified\n",
            self.files_added + self.files_deleted + self.files_modified,
            self.files_added,
            self.files_deleted,
            self.files_modified
        ));
        output.push_str(&format!("{} insertions(+), {} deletions(-)", self.lines_added, self.lines_deleted));

        output
    }
}

/// Pijul archive result output.
pub struct PijulArchiveOutput {
    /// Repository ID.
    pub repo_id: String,
    /// Channel archived.
    pub channel: String,
    /// Output path.
    pub output_path: String,
    /// Archive format (directory, tar, tar.gz).
    pub format: String,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Number of conflicts.
    pub conflicts: u32,
}

impl Outputable for PijulArchiveOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "repo_id": self.repo_id,
            "channel": self.channel,
            "output_path": self.output_path,
            "format": self.format,
            "size_bytes": self.size_bytes,
            "conflicts": self.conflicts
        })
    }

    fn to_human(&self) -> String {
        if self.conflicts > 0 {
            format!(
                "Archive created for '{}'\n\
                 Output:    {}\n\
                 Format:    {}\n\
                 Size:      {} bytes\n\
                 Conflicts: {} (review manually)",
                self.channel, self.output_path, self.format, self.size_bytes, self.conflicts
            )
        } else {
            format!(
                "Archive created for '{}'\n\
                 Output:    {}\n\
                 Format:    {}\n\
                 Size:      {} bytes",
                self.channel, self.output_path, self.format, self.size_bytes
            )
        }
    }
}

/// Pijul show result output - details of a specific change.
pub struct PijulShowOutput {
    /// Full change hash (hex-encoded BLAKE3).
    pub change_hash: String,
    /// Repository ID.
    pub repo_id: String,
    /// Channel this change was recorded on.
    pub channel: String,
    /// Change message/description.
    pub message: String,
    /// Authors of the change.
    pub authors: Vec<(String, Option<String>)>, // (name, email)
    /// Hashes of changes this change depends on.
    pub dependencies: Vec<String>,
    /// Size of the change in bytes.
    pub size_bytes: u64,
    /// Timestamp when the change was recorded (milliseconds since Unix epoch).
    pub recorded_at_ms: u64,
}

impl Outputable for PijulShowOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "change_hash": self.change_hash,
            "repo_id": self.repo_id,
            "channel": self.channel,
            "message": self.message,
            "authors": self.authors.iter().map(|(name, email)| {
                serde_json::json!({ "name": name, "email": email })
            }).collect::<Vec<_>>(),
            "dependencies": self.dependencies,
            "size_bytes": self.size_bytes,
            "recorded_at_ms": self.recorded_at_ms
        })
    }

    fn to_human(&self) -> String {
        use std::time::Duration;
        use std::time::UNIX_EPOCH;

        // Format authors
        let authors_str = if self.authors.is_empty() {
            "unknown".to_string()
        } else {
            self.authors
                .iter()
                .map(|(name, email)| {
                    if let Some(e) = email {
                        format!("{} <{}>", name, e)
                    } else {
                        name.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        };

        // Format timestamp
        let timestamp = UNIX_EPOCH + Duration::from_millis(self.recorded_at_ms);
        let datetime = chrono::DateTime::<chrono::Utc>::from(timestamp);
        let time_str = datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string();

        // Format dependencies
        let deps_str = if self.dependencies.is_empty() {
            "(none)".to_string()
        } else {
            self.dependencies
                .iter()
                .map(|d| format!("  {}", &d[..16.min(d.len())]))
                .collect::<Vec<_>>()
                .join("\n")
        };

        // Format message with indentation
        let message_str = self.message.lines().map(|l| format!("    {}", l)).collect::<Vec<_>>().join("\n");

        format!(
            "change {}\n\
             Author:       {}\n\
             Channel:      {}\n\
             Date:         {}\n\
             Size:         {} bytes\n\
             Dependencies:\n{}\n\n\
             {}",
            &self.change_hash[..16.min(self.change_hash.len())],
            authors_str,
            self.channel,
            time_str,
            self.size_bytes,
            deps_str,
            message_str
        )
    }
}

/// Pijul blame output - change attribution for a file.
pub struct PijulBlameOutput {
    /// File path being blamed.
    pub path: String,
    /// Channel the blame was performed on.
    pub channel: String,
    /// Repository ID.
    pub repo_id: String,
    /// List of changes that contributed to the file.
    pub attributions: Vec<BlameEntry>,
    /// Whether the file currently exists.
    pub file_exists: bool,
}

/// A single entry in blame output.
pub struct BlameEntry {
    /// Change hash (hex-encoded).
    pub change_hash: String,
    /// Author name.
    pub author: Option<String>,
    /// Author email.
    pub author_email: Option<String>,
    /// Change message (first line).
    pub message: String,
    /// Timestamp when recorded.
    pub recorded_at_ms: u64,
    /// Type of change.
    pub change_type: String,
}

impl Outputable for PijulBlameOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "path": self.path,
            "channel": self.channel,
            "repo_id": self.repo_id,
            "file_exists": self.file_exists,
            "attributions": self.attributions.iter().map(|a| {
                serde_json::json!({
                    "change_hash": a.change_hash,
                    "author": a.author,
                    "author_email": a.author_email,
                    "message": a.message,
                    "recorded_at_ms": a.recorded_at_ms,
                    "change_type": a.change_type
                })
            }).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        use std::time::Duration;
        use std::time::UNIX_EPOCH;

        if self.attributions.is_empty() {
            return format!("No changes found for '{}' on channel '{}'", self.path, self.channel);
        }

        let mut output = format!(
            "Blame for '{}' on channel '{}'\n\
             {} change(s) found:\n\n",
            self.path,
            self.channel,
            self.attributions.len()
        );

        for attr in &self.attributions {
            let author = attr.author.as_deref().unwrap_or("unknown");
            let timestamp = UNIX_EPOCH + Duration::from_millis(attr.recorded_at_ms);
            let datetime = chrono::DateTime::<chrono::Utc>::from(timestamp);
            let time_str = datetime.format("%Y-%m-%d").to_string();

            output.push_str(&format!(
                "  {} {} {} {}\n",
                &attr.change_hash[..8.min(attr.change_hash.len())],
                author,
                time_str,
                if attr.message.is_empty() {
                    "(no message)"
                } else {
                    &attr.message
                }
            ));
        }

        output
    }
}

/// Pijul config get/set output.
pub struct PijulConfigOutput {
    /// Configuration key.
    pub key: Option<String>,
    /// Configuration value.
    pub value: Option<String>,
    /// All configuration entries (for list command).
    pub entries: Vec<(String, String)>,
    /// Whether it's a global config.
    pub global: bool,
}

impl Outputable for PijulConfigOutput {
    fn to_json(&self) -> serde_json::Value {
        if !self.entries.is_empty() {
            serde_json::json!({
                "global": self.global,
                "entries": self.entries.iter().map(|(k, v)| {
                    serde_json::json!({ "key": k, "value": v })
                }).collect::<Vec<_>>()
            })
        } else {
            serde_json::json!({
                "key": self.key,
                "value": self.value,
                "global": self.global
            })
        }
    }

    fn to_human(&self) -> String {
        if !self.entries.is_empty() {
            let scope = if self.global { "global" } else { "local" };
            let mut output = format!("Pijul configuration ({}):\n", scope);
            if self.entries.is_empty() {
                output.push_str("  (no configuration set)\n");
            } else {
                for (key, value) in &self.entries {
                    output.push_str(&format!("  {} = {}\n", key, value));
                }
            }
            output
        } else if let Some(value) = &self.value {
            value.clone()
        } else if let Some(key) = &self.key {
            format!("(not set: {})", key)
        } else {
            "(no output)".to_string()
        }
    }
}

/// Pijul remote output.
pub struct PijulRemoteOutput {
    /// Remote name (for single remote display).
    pub name: Option<String>,
    /// Remote URL (for single remote display).
    pub url: Option<String>,
    /// All remotes (for list display).
    pub remotes: Vec<(String, String)>,
}

impl Outputable for PijulRemoteOutput {
    fn to_json(&self) -> serde_json::Value {
        if !self.remotes.is_empty() {
            serde_json::json!({
                "remotes": self.remotes.iter().map(|(name, url)| {
                    serde_json::json!({ "name": name, "url": url })
                }).collect::<Vec<_>>()
            })
        } else {
            serde_json::json!({
                "name": self.name,
                "url": self.url
            })
        }
    }

    fn to_human(&self) -> String {
        if !self.remotes.is_empty() {
            let mut output = String::from("Remotes:\n");
            if self.remotes.is_empty() {
                output.push_str("  (no remotes configured)\n");
            } else {
                for (name, url) in &self.remotes {
                    output.push_str(&format!("  {} -> {}\n", name, url));
                }
            }
            output
        } else if let (Some(name), Some(url)) = (&self.name, &self.url) {
            format!("{} -> {}", name, url)
        } else if let Some(name) = &self.name {
            format!("Remote '{}' not found", name)
        } else {
            "(no output)".to_string()
        }
    }
}

// =============================================================================
// Working Directory Output Types
// =============================================================================

/// Pijul working directory init result.
pub struct PijulWdInitOutput {
    pub path: String,
    pub repo_id: String,
    pub channel: String,
    pub remote: Option<String>,
}

impl Outputable for PijulWdInitOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "path": self.path,
            "repo_id": self.repo_id,
            "channel": self.channel,
            "remote": self.remote
        })
    }

    fn to_human(&self) -> String {
        let remote_str = self.remote.as_deref().unwrap_or("(none)");
        format!(
            "Initialized Pijul working directory\n\
             Path:    {}\n\
             Repo:    {}\n\
             Channel: {}\n\
             Remote:  {}",
            self.path,
            &self.repo_id[..16.min(self.repo_id.len())],
            self.channel,
            remote_str
        )
    }
}

/// Pijul working directory add result.
pub struct PijulWdAddOutput {
    pub added: Vec<String>,
    pub already_staged: Vec<String>,
    pub not_found: Vec<String>,
}

impl Outputable for PijulWdAddOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "added": self.added,
            "already_staged": self.already_staged,
            "not_found": self.not_found
        })
    }

    fn to_human(&self) -> String {
        let mut output = String::new();

        if !self.added.is_empty() {
            output.push_str(&format!("Staged {} file(s):\n", self.added.len()));
            for path in &self.added {
                output.push_str(&format!("  + {}\n", path));
            }
        }

        if !self.already_staged.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&format!("Already staged {} file(s):\n", self.already_staged.len()));
            for path in &self.already_staged {
                output.push_str(&format!("  = {}\n", path));
            }
        }

        if !self.not_found.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&format!("Not found {} file(s):\n", self.not_found.len()));
            for path in &self.not_found {
                output.push_str(&format!("  ? {}\n", path));
            }
        }

        if output.is_empty() {
            output = "No files to stage".to_string();
        }

        output.trim_end().to_string()
    }
}

/// Pijul working directory reset result.
pub struct PijulWdResetOutput {
    pub removed: Vec<String>,
    pub not_staged: Vec<String>,
}

impl Outputable for PijulWdResetOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "removed": self.removed,
            "not_staged": self.not_staged
        })
    }

    fn to_human(&self) -> String {
        let mut output = String::new();

        if !self.removed.is_empty() {
            output.push_str(&format!("Unstaged {} file(s):\n", self.removed.len()));
            for path in &self.removed {
                output.push_str(&format!("  - {}\n", path));
            }
        }

        if !self.not_staged.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&format!("Not staged {} file(s):\n", self.not_staged.len()));
            for path in &self.not_staged {
                output.push_str(&format!("  ? {}\n", path));
            }
        }

        if output.is_empty() {
            output = "No files to unstage".to_string();
        }

        output.trim_end().to_string()
    }
}

/// Pijul working directory status output.
pub struct PijulWdStatusOutput {
    pub path: String,
    pub repo_id: String,
    pub channel: String,
    pub staged: Vec<String>,
    pub remote: Option<String>,
    pub last_synced_head: Option<String>,
}

impl Outputable for PijulWdStatusOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "path": self.path,
            "repo_id": self.repo_id,
            "channel": self.channel,
            "staged": self.staged,
            "remote": self.remote,
            "last_synced_head": self.last_synced_head
        })
    }

    fn to_human(&self) -> String {
        let mut output = format!(
            "Working directory: {}\n\
             Repository: {}\n\
             Channel:    {}\n",
            self.path,
            &self.repo_id[..16.min(self.repo_id.len())],
            self.channel
        );

        if let Some(ref remote) = self.remote {
            output.push_str(&format!("Remote:     {}\n", remote));
        }

        if let Some(ref head) = self.last_synced_head {
            output.push_str(&format!("Last sync:  {}\n", &head[..16.min(head.len())]));
        }

        output.push('\n');

        if self.staged.is_empty() {
            output.push_str("No staged files\n");
        } else {
            output.push_str(&format!("Staged files ({}):\n", self.staged.len()));
            for path in &self.staged {
                output.push_str(&format!("  + {}\n", path));
            }
        }

        output.trim_end().to_string()
    }
}

/// Pijul working directory conflicts output.
pub struct PijulWdConflictsOutput {
    pub path: String,
    pub channel: String,
    pub conflicts: Vec<ConflictInfo>,
    pub total: usize,
}

/// Information about a single conflict.
pub struct ConflictInfo {
    pub file_path: String,
    pub kind: String,
    pub status: String,
    pub markers: Option<MarkerInfo>,
}

/// Conflict marker information.
pub struct MarkerInfo {
    pub start_line: u32,
    pub end_line: u32,
}

impl Outputable for PijulWdConflictsOutput {
    fn to_json(&self) -> serde_json::Value {
        let conflicts: Vec<serde_json::Value> = self
            .conflicts
            .iter()
            .map(|c| {
                let mut obj = serde_json::json!({
                    "path": c.file_path,
                    "kind": c.kind,
                    "status": c.status,
                });
                if let Some(ref m) = c.markers {
                    obj["markers"] = serde_json::json!({
                        "start_line": m.start_line,
                        "end_line": m.end_line
                    });
                }
                obj
            })
            .collect();

        serde_json::json!({
            "path": self.path,
            "channel": self.channel,
            "conflicts": conflicts,
            "total": self.total
        })
    }

    fn to_human(&self) -> String {
        let mut output = format!(
            "Working directory: {}\n\
             Channel:           {}\n\n",
            self.path, self.channel
        );

        if self.conflicts.is_empty() {
            output.push_str("No conflicts detected\n");
        } else {
            output.push_str(&format!("Conflicts ({}):\n", self.total));
            for c in &self.conflicts {
                let status_indicator = match c.status.as_str() {
                    "unresolved" => "!",
                    "in_progress" => "~",
                    "pending" => "?",
                    "resolved" => "+",
                    _ => " ",
                };
                output.push_str(&format!("  {} {} [{}] {}\n", status_indicator, c.file_path, c.kind, c.status));
                if let Some(ref m) = c.markers {
                    output.push_str(&format!("      lines {}-{}\n", m.start_line, m.end_line));
                }
            }
        }

        output.trim_end().to_string()
    }
}

/// Pijul working directory solve output.
pub struct PijulWdSolveOutput {
    pub path: String,
    pub resolved: Vec<String>,
    pub strategy: String,
    pub remaining: usize,
}

impl Outputable for PijulWdSolveOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "path": self.path,
            "resolved": self.resolved,
            "strategy": self.strategy,
            "remaining": self.remaining
        })
    }

    fn to_human(&self) -> String {
        let mut output = format!("Working directory: {}\n\n", self.path);

        if self.resolved.is_empty() {
            output.push_str("No conflicts resolved\n");
        } else {
            output.push_str(&format!(
                "Resolved {} conflict(s) using '{}' strategy:\n",
                self.resolved.len(),
                self.strategy
            ));
            for path in &self.resolved {
                output.push_str(&format!("  + {}\n", path));
            }
        }

        if self.remaining > 0 {
            output.push_str(&format!("\n{} conflict(s) remaining\n", self.remaining));
        }

        output.trim_end().to_string()
    }
}

/// Pijul resolution strategies list output.
pub struct PijulStrategiesOutput {
    pub strategies: Vec<StrategyInfo>,
}

/// Information about a resolution strategy.
pub struct StrategyInfo {
    pub name: String,
    pub description: String,
}

impl Outputable for PijulStrategiesOutput {
    fn to_json(&self) -> serde_json::Value {
        let strategies: Vec<serde_json::Value> = self
            .strategies
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "description": s.description
                })
            })
            .collect();

        serde_json::json!({
            "strategies": strategies
        })
    }

    fn to_human(&self) -> String {
        let mut output = String::from("Available resolution strategies:\n\n");

        for s in &self.strategies {
            output.push_str(&format!("  {:8} - {}\n", s.name, s.description));
        }

        output.trim_end().to_string()
    }
}

// =============================================================================
// Command Implementation
// =============================================================================

impl PijulCommand {
    /// Execute the pijul command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            PijulCommand::Repo(cmd) => cmd.run(client, json).await,
            PijulCommand::Channel(cmd) => cmd.run(client, json).await,
            PijulCommand::Wd(cmd) => cmd.run(client, json).await,
            PijulCommand::Record(args) => pijul_record(client, args, json).await,
            PijulCommand::Apply(args) => pijul_apply(client, args, json).await,
            PijulCommand::Unrecord(args) => pijul_unrecord(client, args, json).await,
            PijulCommand::Log(args) => pijul_log(client, args, json).await,
            PijulCommand::Checkout(args) => pijul_checkout(client, args, json).await,
            PijulCommand::Sync(args) => pijul_sync(client, args, json).await,
            PijulCommand::Pull(args) => pijul_pull(client, args, json).await,
            PijulCommand::Push(args) => pijul_push(client, args, json).await,
            PijulCommand::Diff(args) => pijul_diff(args, json).await,
            PijulCommand::Archive(args) => pijul_archive(args, json).await,
            PijulCommand::Show(args) => pijul_show(client, args, json).await,
            PijulCommand::Blame(args) => pijul_blame(client, args, json).await,
            PijulCommand::Config(cmd) => cmd.run(json),
            PijulCommand::Remote(cmd) => cmd.run(json),
        }
    }
}

impl RepoCommand {
    /// Execute the repo subcommand.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            RepoCommand::Init(args) => repo_init(client, args, json).await,
            RepoCommand::List(args) => repo_list(client, args, json).await,
            RepoCommand::Info(args) => repo_info(client, args, json).await,
        }
    }
}

impl ChannelCommand {
    /// Execute the channel subcommand.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            ChannelCommand::List(args) => channel_list(client, args, json).await,
            ChannelCommand::Create(args) => channel_create(client, args, json).await,
            ChannelCommand::Delete(args) => channel_delete(client, args, json).await,
            ChannelCommand::Fork(args) => channel_fork(client, args, json).await,
            ChannelCommand::Info(args) => channel_info(client, args, json).await,
        }
    }
}

impl WdCommand {
    /// Execute the working directory subcommand.
    ///
    /// Note: Some working directory commands (init, add, reset, status, diff) are
    /// local-only while others (record, checkout) may sync with the cluster.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            WdCommand::Init(args) => wd_init(args, json),
            WdCommand::Add(args) => wd_add(args, json),
            WdCommand::Reset(args) => wd_reset(args, json),
            WdCommand::Status(args) => wd_status(args, json),
            WdCommand::Record(args) => wd_record(client, args, json).await,
            WdCommand::Checkout(args) => wd_checkout(client, args, json).await,
            WdCommand::Diff(args) => wd_diff(args, json).await,
            WdCommand::Conflicts(args) => wd_conflicts(args, json),
            WdCommand::Solve(args) => wd_solve(args, json),
        }
    }
}

impl ConfigCommand {
    /// Execute the config subcommand.
    ///
    /// Config commands are local-only and don't require cluster connection.
    pub fn run(self, json: bool) -> Result<()> {
        match self {
            ConfigCommand::Get { key, global } => config_get(&key, global, json),
            ConfigCommand::Set { key, value, global } => config_set(&key, &value, global, json),
            ConfigCommand::List { global } => config_list(global, json),
            ConfigCommand::Unset { key, global } => config_unset(&key, global, json),
        }
    }
}

impl RemoteCommand {
    /// Execute the remote subcommand.
    ///
    /// Remote commands are local-only and don't require cluster connection.
    pub fn run(self, json: bool) -> Result<()> {
        match self {
            RemoteCommand::List => remote_list(json),
            RemoteCommand::Add { name, url } => remote_add(&name, &url, json),
            RemoteCommand::Remove { name } => remote_remove(&name, json),
            RemoteCommand::Show { name } => remote_show(&name, json),
        }
    }
}

// =============================================================================
// Config Handlers
// =============================================================================

/// Get the global config path.
fn global_config_path() -> Result<PathBuf> {
    // Try XDG_CONFIG_HOME first
    if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(config_home).join("aspen-pijul").join("config.toml"));
    }

    // Fall back to $HOME/.config
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .context("could not determine home directory")?;

    Ok(PathBuf::from(home).join(".config").join("aspen-pijul").join("config.toml"))
}

/// Get the local config path (current working directory).
fn local_config_path() -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    Ok(cwd.join(".aspen").join("pijul").join("config.toml"))
}

/// Load configuration from TOML file.
fn load_config(path: &PathBuf) -> Result<toml::Table> {
    if !path.exists() {
        return Ok(toml::Table::new());
    }

    let content =
        std::fs::read_to_string(path).with_context(|| format!("failed to read config file: {}", path.display()))?;

    content.parse().with_context(|| format!("failed to parse config file: {}", path.display()))
}

/// Save configuration to TOML file.
fn save_config(path: &PathBuf, table: &toml::Table) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory: {}", parent.display()))?;
    }

    let content = toml::to_string_pretty(table).context("failed to serialize config")?;

    std::fs::write(path, content).with_context(|| format!("failed to write config file: {}", path.display()))
}

/// Get a value from config using dot notation (e.g., "user.name").
fn get_value(table: &toml::Table, key: &str) -> Option<String> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current: &toml::Value = &toml::Value::Table(table.clone());

    for part in parts {
        match current {
            toml::Value::Table(t) => {
                current = t.get(part)?;
            }
            _ => return None,
        }
    }

    match current {
        toml::Value::String(s) => Some(s.clone()),
        toml::Value::Integer(i) => Some(i.to_string()),
        toml::Value::Float(f) => Some(f.to_string()),
        toml::Value::Boolean(b) => Some(b.to_string()),
        _ => Some(current.to_string()),
    }
}

/// Set a value in config using dot notation.
fn set_value(table: &mut toml::Table, key: &str, value: &str) {
    let parts: Vec<&str> = key.split('.').collect();

    if parts.len() == 1 {
        table.insert(parts[0].to_string(), toml::Value::String(value.to_string()));
        return;
    }

    // Navigate/create nested tables
    let mut current = table;
    for part in &parts[..parts.len() - 1] {
        current = current
            .entry(part.to_string())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .expect("expected table");
    }

    current.insert(parts[parts.len() - 1].to_string(), toml::Value::String(value.to_string()));
}

/// Unset a value in config using dot notation.
fn unset_value(table: &mut toml::Table, key: &str) -> bool {
    let parts: Vec<&str> = key.split('.').collect();

    if parts.len() == 1 {
        return table.remove(parts[0]).is_some();
    }

    // Navigate to parent table
    let mut current = table;
    for part in &parts[..parts.len() - 1] {
        match current.get_mut(*part) {
            Some(toml::Value::Table(t)) => current = t,
            _ => return false,
        }
    }

    current.remove(parts[parts.len() - 1]).is_some()
}

/// Flatten config table to key-value pairs.
fn flatten_config(table: &toml::Table, prefix: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();

    for (key, value) in table {
        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", prefix, key)
        };

        match value {
            toml::Value::Table(t) => {
                result.extend(flatten_config(t, &full_key));
            }
            toml::Value::String(s) => {
                result.push((full_key, s.clone()));
            }
            _ => {
                result.push((full_key, value.to_string()));
            }
        }
    }

    result
}

fn config_get(key: &str, global: bool, json: bool) -> Result<()> {
    let path = if global {
        global_config_path()?
    } else {
        local_config_path()?
    };

    let table = load_config(&path)?;
    let value = get_value(&table, key);

    let output = PijulConfigOutput {
        key: Some(key.to_string()),
        value,
        entries: Vec::new(),
        global,
    };
    print_output(&output, json);
    Ok(())
}

fn config_set(key: &str, value: &str, global: bool, json: bool) -> Result<()> {
    let path = if global {
        global_config_path()?
    } else {
        local_config_path()?
    };

    let mut table = load_config(&path)?;
    set_value(&mut table, key, value);
    save_config(&path, &table)?;

    let output = PijulConfigOutput {
        key: Some(key.to_string()),
        value: Some(value.to_string()),
        entries: Vec::new(),
        global,
    };
    print_output(&output, json);
    print_success(&format!("Set {} = {}", key, value), json);
    Ok(())
}

fn config_list(global: bool, json: bool) -> Result<()> {
    let path = if global {
        global_config_path()?
    } else {
        local_config_path()?
    };

    let table = load_config(&path)?;
    let entries = flatten_config(&table, "");

    let output = PijulConfigOutput {
        key: None,
        value: None,
        entries,
        global,
    };
    print_output(&output, json);
    Ok(())
}

fn config_unset(key: &str, global: bool, json: bool) -> Result<()> {
    let path = if global {
        global_config_path()?
    } else {
        local_config_path()?
    };

    let mut table = load_config(&path)?;
    let removed = unset_value(&mut table, key);
    save_config(&path, &table)?;

    if removed {
        print_success(&format!("Unset {}", key), json);
    } else {
        print_success(&format!("Key '{}' was not set", key), json);
    }

    let output = PijulConfigOutput {
        key: Some(key.to_string()),
        value: None,
        entries: Vec::new(),
        global,
    };
    if json {
        print_output(&output, json);
    }
    Ok(())
}

// =============================================================================
// Remote Handlers
// =============================================================================

/// Load remotes from the local config file.
fn load_remotes() -> Result<std::collections::HashMap<String, String>> {
    let path = local_config_path()?;
    let table = load_config(&path)?;

    let mut remotes = std::collections::HashMap::new();
    if let Some(toml::Value::Table(remotes_table)) = table.get("remotes") {
        for (name, value) in remotes_table {
            if let toml::Value::String(url) = value {
                remotes.insert(name.clone(), url.clone());
            }
        }
    }
    Ok(remotes)
}

/// Save remotes to the local config file.
fn save_remotes(remotes: &std::collections::HashMap<String, String>) -> Result<()> {
    let path = local_config_path()?;
    let mut table = load_config(&path)?;

    // Build remotes table
    let mut remotes_table = toml::Table::new();
    for (name, url) in remotes {
        remotes_table.insert(name.clone(), toml::Value::String(url.clone()));
    }
    table.insert("remotes".to_string(), toml::Value::Table(remotes_table));

    save_config(&path, &table)
}

fn remote_list(json: bool) -> Result<()> {
    let remotes = load_remotes()?;

    let output = PijulRemoteOutput {
        name: None,
        url: None,
        remotes: remotes.into_iter().collect(),
    };
    print_output(&output, json);
    Ok(())
}

fn remote_add(name: &str, url: &str, json: bool) -> Result<()> {
    let mut remotes = load_remotes()?;

    if remotes.contains_key(name) {
        anyhow::bail!("Remote '{}' already exists. Use 'remote remove' first.", name);
    }

    remotes.insert(name.to_string(), url.to_string());
    save_remotes(&remotes)?;

    let output = PijulRemoteOutput {
        name: Some(name.to_string()),
        url: Some(url.to_string()),
        remotes: Vec::new(),
    };
    print_output(&output, json);
    print_success(&format!("Added remote '{}' -> {}", name, url), json);
    Ok(())
}

fn remote_remove(name: &str, json: bool) -> Result<()> {
    let mut remotes = load_remotes()?;

    if remotes.remove(name).is_none() {
        anyhow::bail!("Remote '{}' not found", name);
    }

    save_remotes(&remotes)?;

    print_success(&format!("Removed remote '{}'", name), json);
    Ok(())
}

fn remote_show(name: &str, json: bool) -> Result<()> {
    let remotes = load_remotes()?;

    let output = if let Some(url) = remotes.get(name) {
        PijulRemoteOutput {
            name: Some(name.to_string()),
            url: Some(url.clone()),
            remotes: Vec::new(),
        }
    } else {
        PijulRemoteOutput {
            name: Some(name.to_string()),
            url: None,
            remotes: Vec::new(),
        }
    };
    print_output(&output, json);
    Ok(())
}

// =============================================================================
// Working Directory Handlers
// =============================================================================

fn wd_init(args: WdInitArgs, json: bool) -> Result<()> {
    // Parse repo ID
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Determine working directory path
    let path = args
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    // Initialize working directory
    let wd = WorkingDirectory::init(&path, &repo_id, &args.channel, args.remote.clone())
        .context("failed to initialize working directory")?;

    let output = PijulWdInitOutput {
        path: wd.root().display().to_string(),
        repo_id: args.repo_id,
        channel: args.channel,
        remote: args.remote,
    };
    print_output(&output, json);

    Ok(())
}

fn wd_add(args: WdAddArgs, json: bool) -> Result<()> {
    // Find or open working directory
    let start_path = args
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let wd =
        WorkingDirectory::find(&start_path).context("not in a Pijul working directory (run 'pijul wd init' first)")?;

    // Add files
    let result = wd.add(&args.paths).context("failed to add files")?;

    let output = PijulWdAddOutput {
        added: result.added,
        already_staged: result.already_staged,
        not_found: result.not_found,
    };
    print_output(&output, json);

    Ok(())
}

fn wd_reset(args: WdResetArgs, json: bool) -> Result<()> {
    // Find or open working directory
    let start_path = args
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let wd =
        WorkingDirectory::find(&start_path).context("not in a Pijul working directory (run 'pijul wd init' first)")?;

    // Reset files
    let result = if args.all || args.paths.is_empty() {
        wd.reset_all().context("failed to reset all files")?
    } else {
        wd.reset(&args.paths).context("failed to reset files")?
    };

    let output = PijulWdResetOutput {
        removed: result.removed,
        not_staged: result.not_staged,
    };
    print_output(&output, json);

    Ok(())
}

fn wd_status(args: WdStatusArgs, json: bool) -> Result<()> {
    // Find or open working directory
    let start_path = args
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let wd =
        WorkingDirectory::find(&start_path).context("not in a Pijul working directory (run 'pijul wd init' first)")?;

    // Get staged files
    let staged = wd.staged_files().context("failed to read staged files")?;

    // Porcelain output: machine-readable format
    if args.porcelain {
        // Output format: XY PATH
        // X = index status, Y = working tree status
        // A = added/staged, ? = untracked
        // Currently we only track staged files, so all staged files are "A "
        for path in &staged {
            println!("A  {}", path);
        }
        return Ok(());
    }

    let config = wd.config();

    let output = PijulWdStatusOutput {
        path: wd.root().display().to_string(),
        repo_id: config.repo_id.clone(),
        channel: config.channel.clone(),
        staged,
        remote: config.remote.clone(),
        last_synced_head: config.last_synced_head.clone(),
    };
    print_output(&output, json);

    Ok(())
}

/// Record changes from working directory with auto-detected repo and channel.
async fn wd_record(client: &AspenClient, args: WdRecordArgs, json: bool) -> Result<()> {
    use tracing::info;

    // Find working directory
    let start_path = args
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let wd =
        WorkingDirectory::find(&start_path).context("not in a Pijul working directory (run 'pijul wd init' first)")?;

    // Check staged files
    let staged = wd.staged_files().context("failed to read staged files")?;
    if staged.is_empty() {
        anyhow::bail!("No staged files. Use 'pijul wd add <files>' first.");
    }

    // Get repo_id and channel from working directory config
    let config = wd.config();
    let repo_id = RepoId::from_hex(&config.repo_id).context("invalid repository ID in working directory config")?;
    let channel = config.channel.clone();

    info!(
        repo_id = %config.repo_id,
        channel = %channel,
        staged_files = staged.len(),
        "recording changes from working directory"
    );

    // Determine cache directory for local pristine
    let cache_dir = repo_cache_dir(&repo_id).context("failed to get cache directory")?;

    // Create cache directory if needed
    std::fs::create_dir_all(&cache_dir).context("failed to create cache directory")?;

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open_or_create(&repo_id).context("failed to open/create local pristine")?;

    // Create temporary in-memory blob store for recording
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs.clone()));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store.clone());

    // Create author string
    let author_str = match (args.author, args.email) {
        (Some(name), Some(email)) => format!("{} <{}>", name, email),
        (Some(name), None) => name,
        (None, Some(email)) => format!("<{}>", email),
        (None, None) => {
            // Try to get from git config or environment
            let name = std::env::var("USER").unwrap_or_else(|_| "Unknown".to_string());
            format!("{} <{}@local>", name, name)
        }
    };

    // Record changes
    let recorder =
        ChangeRecorder::new(pristine.clone(), change_dir, wd.root().to_path_buf()).with_threads(args.threads);

    let result = recorder.record(&channel, &args.message, &author_str).await.context("failed to record changes")?;

    match result {
        Some(record_result) => {
            info!(
                hash = %record_result.hash,
                hunks = record_result.num_hunks,
                bytes = record_result.size_bytes,
                "recorded change locally"
            );

            // Push to cluster if requested
            if args.push {
                // Get the change bytes from local store
                let change_bytes = change_store
                    .get_change(&record_result.hash)
                    .await
                    .context("failed to get change from local store")?
                    .context("change not found in local store after recording")?;

                // Upload change to cluster blob store
                let blob_response = client
                    .send(ClientRpcRequest::AddBlob {
                        data: change_bytes.clone(),
                        tag: Some(format!("pijul:{}:{}", config.repo_id, record_result.hash)),
                    })
                    .await
                    .context("failed to upload change to cluster")?;

                match blob_response {
                    ClientRpcResponse::AddBlobResult(_) => {
                        info!(hash = %record_result.hash, "uploaded change to cluster blob store");
                    }
                    ClientRpcResponse::Error(e) => {
                        anyhow::bail!("failed to upload change: {}: {}", e.code, e.message);
                    }
                    _ => anyhow::bail!("unexpected response from blob add"),
                }

                // Apply the change to update channel head
                let apply_response = client
                    .send(ClientRpcRequest::PijulApply {
                        repo_id: config.repo_id.clone(),
                        channel: channel.clone(),
                        change_hash: record_result.hash.to_string(),
                    })
                    .await
                    .context("failed to apply change to cluster")?;

                match apply_response {
                    ClientRpcResponse::PijulApplyResult(_) => {
                        info!(channel = %channel, "updated channel head via Raft");
                    }
                    ClientRpcResponse::Error(e) => {
                        anyhow::bail!("failed to apply change: {}: {}", e.code, e.message);
                    }
                    _ => anyhow::bail!("unexpected response from apply"),
                }

                // Store change metadata
                let (author_name, author_email) = if let Some(start) = author_str.find('<') {
                    if let Some(end) = author_str.find('>') {
                        let name = author_str[..start].trim().to_string();
                        let email = author_str[start + 1..end].to_string();
                        (name, email)
                    } else {
                        (author_str.clone(), String::new())
                    }
                } else {
                    (author_str.clone(), String::new())
                };

                let metadata = ChangeMetadata {
                    hash: record_result.hash,
                    repo_id: RepoId::from_hex(&config.repo_id).unwrap(),
                    channel: channel.clone(),
                    message: args.message.clone(),
                    authors: vec![PijulAuthor::from_name_email(author_name, author_email)],
                    dependencies: record_result.dependencies.clone(),
                    size_bytes: record_result.size_bytes as u64,
                    recorded_at_ms: chrono::Utc::now().timestamp_millis() as u64,
                };

                let meta_bytes = postcard::to_allocvec(&metadata).context("failed to serialize change metadata")?;
                let meta_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &meta_bytes);
                let meta_key = format!("pijul:change:meta:{}:{}", config.repo_id, record_result.hash);

                let _ = client
                    .send(ClientRpcRequest::WriteKey {
                        key: meta_key,
                        value: meta_b64.into_bytes(),
                    })
                    .await;
            }

            // Clear staged files on success
            wd.reset_all().context("failed to clear staged files")?;

            let output = PijulRecordOutput {
                change_hash: record_result.hash.to_string(),
                message: args.message,
                hunks: record_result.num_hunks as u32,
                size_bytes: record_result.size_bytes as u64,
            };
            print_output(&output, json);
            Ok(())
        }
        None => {
            print_success("No changes to record", json);
            Ok(())
        }
    }
}

/// Checkout channel to working directory with auto-detected repo.
async fn wd_checkout(client: &AspenClient, args: WdCheckoutArgs, json: bool) -> Result<()> {
    use aspen_pijul::WorkingDirOutput;
    use tracing::info;

    // Find working directory
    let start_path = args
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let wd =
        WorkingDirectory::find(&start_path).context("not in a Pijul working directory (run 'pijul wd init' first)")?;

    // Get config
    let config = wd.config();
    let repo_id = RepoId::from_hex(&config.repo_id).context("invalid repository ID in working directory config")?;
    let channel = args.channel.unwrap_or_else(|| config.channel.clone());

    info!(
        repo_id = %config.repo_id,
        channel = %channel,
        "checking out to working directory"
    );

    // Determine cache directory
    let cache_dir = repo_cache_dir(&repo_id).context("failed to get cache directory")?;

    // Pull from cluster if requested
    if args.pull {
        // Create cache directory if needed
        std::fs::create_dir_all(&cache_dir).context("failed to create cache directory")?;

        // Sync the channel from cluster
        let response = client
            .send(ClientRpcRequest::PijulChannelInfo {
                repo_id: config.repo_id.clone(),
                name: channel.clone(),
            })
            .await?;

        match response {
            ClientRpcResponse::PijulChannelResult(_channel_info) => {
                info!(channel = %channel, "synced channel info from cluster");
            }
            ClientRpcResponse::Error(e) => {
                anyhow::bail!("failed to get channel info: {}: {}", e.code, e.message);
            }
            _ => anyhow::bail!("unexpected response type"),
        }
    }

    // Check if cache exists
    if !cache_dir.exists() {
        anyhow::bail!("Local cache not found at {}. Run 'pijul sync {}' first.", cache_dir.display(), config.repo_id);
    }

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open(&repo_id).context("failed to open local pristine - run 'pijul sync' first")?;

    // Create change directory
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store);

    // Output to working directory
    let outputter = WorkingDirOutput::new(pristine, change_dir, wd.root().to_path_buf());
    let result = outputter.output(&channel).context("failed to output to working directory")?;

    let conflicts = result.conflict_count() as u32;

    let output = PijulCheckoutOutput {
        channel,
        output_dir: wd.root().display().to_string(),
        files_written: 0, // WorkingDirOutput doesn't track this currently
        conflicts,
    };
    print_output(&output, json);

    if conflicts > 0 {
        std::process::exit(1);
    }
    Ok(())
}

/// Show diff between working directory and channel with auto-detected repo.
async fn wd_diff(args: WdDiffArgs, json: bool) -> Result<()> {
    use tracing::info;

    // Find working directory
    let start_path = args
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let wd =
        WorkingDirectory::find(&start_path).context("not in a Pijul working directory (run 'pijul wd init' first)")?;

    // Get config
    let config = wd.config();
    let repo_id = RepoId::from_hex(&config.repo_id).context("invalid repository ID in working directory config")?;
    let channel = config.channel.clone();

    info!(
        repo_id = %config.repo_id,
        channel = %channel,
        "computing diff for working directory"
    );

    // Determine cache directory
    let cache_dir = repo_cache_dir(&repo_id).context("failed to get cache directory")?;

    // Check if cache exists
    if !cache_dir.exists() {
        anyhow::bail!("Local cache not found at {}. Run 'pijul sync {}' first.", cache_dir.display(), config.repo_id);
    }

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open(&repo_id).context("failed to open local pristine - run 'pijul sync' first")?;

    // Create change directory
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store);

    // Create recorder to get diff info
    let recorder = ChangeRecorder::new(pristine, change_dir, wd.root().to_path_buf());

    // Get the diff
    let diff_result = recorder.diff(&channel).await.context("failed to compute diff")?;

    // Convert to output format
    let mut hunks = Vec::new();
    let mut files_added = 0u32;
    let mut files_deleted = 0u32;
    let mut files_modified = 0u32;
    let mut lines_added = 0u32;
    let mut lines_deleted = 0u32;

    for hunk_info in &diff_result.hunks {
        let change_type = match hunk_info.kind.as_str() {
            "add" | "new" => {
                files_added += 1;
                "add"
            }
            "delete" | "remove" => {
                files_deleted += 1;
                "delete"
            }
            "modify" | "edit" => {
                files_modified += 1;
                "modify"
            }
            "rename" => {
                files_modified += 1;
                "rename"
            }
            "permission" | "perm" => {
                files_modified += 1;
                "permission"
            }
            _ => {
                files_modified += 1;
                "modify"
            }
        };

        lines_added += hunk_info.additions;
        lines_deleted += hunk_info.deletions;

        hunks.push(DiffHunk {
            path: hunk_info.path.clone(),
            change_type: change_type.to_string(),
            additions: hunk_info.additions,
            deletions: hunk_info.deletions,
        });
    }

    let no_changes = hunks.is_empty();

    // Show summary only if requested
    if args.summary {
        if no_changes {
            print_success("No changes", json);
        } else {
            let summary = format!(
                "{} files changed: +{} added, -{} deleted, ~{} modified ({} insertions, {} deletions)",
                files_added + files_deleted + files_modified,
                files_added,
                files_deleted,
                files_modified,
                lines_added,
                lines_deleted
            );
            print_success(&summary, json);
        }
        return Ok(());
    }

    let output = PijulDiffOutput {
        repo_id: config.repo_id.clone(),
        channel: channel.clone(),
        channel2: None,
        working_dir: Some(wd.root().display().to_string()),
        hunks,
        files_added,
        files_deleted,
        files_modified,
        lines_added,
        lines_deleted,
        no_changes,
    };

    print_output(&output, json);
    Ok(())
}

/// List conflicts in the working directory.
fn wd_conflicts(args: WdConflictsArgs, json: bool) -> Result<()> {
    // Find working directory
    let start_path = args
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let wd =
        WorkingDirectory::find(&start_path).context("not in a Pijul working directory (run 'pijul wd init' first)")?;

    let config = wd.config();

    // For now, we scan the working directory for conflict markers
    // In a full implementation, this would integrate with the pristine database
    let conflicts = scan_for_conflict_markers(wd.root(), args.detailed)?;

    let output = PijulWdConflictsOutput {
        path: wd.root().display().to_string(),
        channel: config.channel.clone(),
        total: conflicts.len(),
        conflicts,
    };

    print_output(&output, json);
    Ok(())
}

/// Scan a directory for files containing Pijul conflict markers.
fn scan_for_conflict_markers(root: &std::path::Path, detailed: bool) -> Result<Vec<ConflictInfo>> {
    use std::fs;

    let mut conflicts = Vec::new();

    // Walk the directory tree, skipping .pijul directory
    fn visit_dir(
        dir: &std::path::Path,
        root: &std::path::Path,
        detailed: bool,
        conflicts: &mut Vec<ConflictInfo>,
    ) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(dir).context("failed to read directory")? {
            let entry = entry.context("failed to read directory entry")?;
            let path = entry.path();

            // Skip .pijul directory
            if path.file_name().map(|n| n == ".pijul").unwrap_or(false) {
                continue;
            }

            if path.is_dir() {
                visit_dir(&path, root, detailed, conflicts)?;
            } else if path.is_file() {
                // Check for conflict markers in text files
                if let Some(conflict) = check_file_for_conflicts(&path, root, detailed) {
                    conflicts.push(conflict);
                }
            }
        }

        Ok(())
    }

    visit_dir(root, root, detailed, &mut conflicts)?;
    Ok(conflicts)
}

/// Check a single file for Pijul conflict markers.
fn check_file_for_conflicts(path: &std::path::Path, root: &std::path::Path, detailed: bool) -> Option<ConflictInfo> {
    use std::fs::File;
    use std::io::BufRead;
    use std::io::BufReader;

    // Try to open and read the file
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut start_line: Option<u32> = None;
    let mut end_line: Option<u32> = None;
    let mut has_conflict = false;

    // Pijul conflict markers
    const CONFLICT_START: &str = ">>>>>";
    const CONFLICT_END: &str = "<<<<<";

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = line_result.ok()?;

        if line.starts_with(CONFLICT_START) {
            has_conflict = true;
            if start_line.is_none() {
                start_line = Some((line_num + 1) as u32);
            }
        } else if line.starts_with(CONFLICT_END) {
            end_line = Some((line_num + 1) as u32);
        }
    }

    if !has_conflict {
        return None;
    }

    let relative_path = path.strip_prefix(root).ok()?.to_string_lossy().to_string();

    let markers = if detailed {
        start_line.map(|start| MarkerInfo {
            start_line: start,
            end_line: end_line.unwrap_or(start),
        })
    } else {
        None
    };

    Some(ConflictInfo {
        file_path: relative_path,
        kind: "order".to_string(), // Most common type
        status: "unresolved".to_string(),
        markers,
    })
}

/// Resolve conflicts in the working directory.
fn wd_solve(args: WdSolveArgs, json: bool) -> Result<()> {
    // Handle --list flag
    if args.list {
        let strategies = vec![
            StrategyInfo {
                name: "ours".to_string(),
                description: "Keep our version, discard theirs".to_string(),
            },
            StrategyInfo {
                name: "theirs".to_string(),
                description: "Keep their version, discard ours".to_string(),
            },
            StrategyInfo {
                name: "newest".to_string(),
                description: "Keep the most recently authored change".to_string(),
            },
            StrategyInfo {
                name: "oldest".to_string(),
                description: "Keep the oldest change".to_string(),
            },
            StrategyInfo {
                name: "manual".to_string(),
                description: "User manually resolves in the working directory".to_string(),
            },
        ];

        let output = PijulStrategiesOutput { strategies };
        print_output(&output, json);
        return Ok(());
    }

    // Parse strategy
    let strategy: ResolutionStrategy = args.strategy.parse().map_err(|e: String| anyhow::anyhow!(e))?;

    // Find working directory
    let start_path = args
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let wd =
        WorkingDirectory::find(&start_path).context("not in a Pijul working directory (run 'pijul wd init' first)")?;

    // Collect files to resolve
    let files_to_resolve: Vec<String> = if args.all {
        // Scan for all conflicts
        let conflicts = scan_for_conflict_markers(wd.root(), false)?;
        conflicts.into_iter().map(|c| c.file_path).collect()
    } else if args.paths.is_empty() {
        anyhow::bail!("specify file path(s) or use --all to resolve all conflicts");
    } else {
        args.paths.clone()
    };

    if files_to_resolve.is_empty() {
        let output = PijulWdSolveOutput {
            path: wd.root().display().to_string(),
            resolved: vec![],
            strategy: strategy.to_string(),
            remaining: 0,
        };
        print_output(&output, json);
        return Ok(());
    }

    // Resolve conflicts based on strategy
    let mut resolved = Vec::new();
    let mut errors = Vec::new();

    for file_path in &files_to_resolve {
        let full_path = wd.root().join(file_path);
        if !full_path.exists() {
            errors.push(format!("{}: file not found", file_path));
            continue;
        }

        match resolve_file_conflict(&full_path, strategy) {
            Ok(true) => resolved.push(file_path.clone()),
            Ok(false) => {} // No conflict in file
            Err(e) => errors.push(format!("{}: {}", file_path, e)),
        }
    }

    // Report errors
    if !errors.is_empty() && !json {
        for error in &errors {
            eprintln!("Warning: {}", error);
        }
    }

    // Get remaining conflicts
    let remaining = scan_for_conflict_markers(wd.root(), false)?;

    let output = PijulWdSolveOutput {
        path: wd.root().display().to_string(),
        resolved,
        strategy: strategy.to_string(),
        remaining: remaining.len(),
    };

    print_output(&output, json);
    Ok(())
}

/// Resolve conflicts in a single file based on the specified strategy.
fn resolve_file_conflict(path: &std::path::Path, strategy: ResolutionStrategy) -> Result<bool> {
    use std::fs;

    let content = fs::read_to_string(path).context("failed to read file")?;

    // Pijul conflict markers
    const CONFLICT_START: &str = ">>>>>";
    const CONFLICT_SEPARATOR: &str = "=====";
    const CONFLICT_END: &str = "<<<<<";

    if !content.contains(CONFLICT_START) {
        return Ok(false);
    }

    let mut result = String::new();
    let mut in_conflict = false;
    let mut in_ours = false;
    let mut in_theirs = false;
    let mut ours_content = String::new();
    let mut theirs_content = String::new();

    for line in content.lines() {
        if line.starts_with(CONFLICT_START) {
            in_conflict = true;
            in_ours = true;
            in_theirs = false;
            ours_content.clear();
            theirs_content.clear();
        } else if line.starts_with(CONFLICT_SEPARATOR) && in_conflict {
            in_ours = false;
            in_theirs = true;
        } else if line.starts_with(CONFLICT_END) && in_conflict {
            // Resolve based on strategy
            let resolved_content = match strategy {
                ResolutionStrategy::Ours => &ours_content,
                ResolutionStrategy::Theirs => &theirs_content,
                ResolutionStrategy::Newest | ResolutionStrategy::Oldest => {
                    // For now, treat newest/oldest like theirs (would need metadata)
                    &theirs_content
                }
                ResolutionStrategy::Manual => {
                    // Keep the conflict markers for manual resolution
                    result.push_str(CONFLICT_START);
                    result.push('\n');
                    result.push_str(&ours_content);
                    result.push_str(CONFLICT_SEPARATOR);
                    result.push('\n');
                    result.push_str(&theirs_content);
                    result.push_str(CONFLICT_END);
                    result.push('\n');
                    in_conflict = false;
                    in_ours = false;
                    in_theirs = false;
                    continue;
                }
            };
            result.push_str(resolved_content);
            in_conflict = false;
            in_ours = false;
            in_theirs = false;
        } else if in_ours {
            ours_content.push_str(line);
            ours_content.push('\n');
        } else if in_theirs {
            theirs_content.push_str(line);
            theirs_content.push('\n');
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    // Write back the resolved content
    fs::write(path, result).context("failed to write resolved file")?;

    Ok(true)
}

// =============================================================================
// Repository Handlers
// =============================================================================

async fn repo_init(client: &AspenClient, args: RepoInitArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulRepoInit {
            name: args.name.clone(),
            description: args.description,
            default_channel: args.default_channel,
        })
        .await?;

    match response {
        ClientRpcResponse::PijulRepoResult(result) => {
            let output = PijulRepoOutput {
                id: result.id,
                name: result.name,
                description: result.description,
                default_channel: result.default_channel,
                channel_count: result.channel_count,
                created_at_ms: result.created_at_ms,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn repo_list(client: &AspenClient, args: RepoListArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::PijulRepoList { limit: args.limit }).await?;

    match response {
        ClientRpcResponse::PijulRepoListResult(result) => {
            let output = PijulRepoListOutput {
                repos: result
                    .repos
                    .into_iter()
                    .map(|r| PijulRepoOutput {
                        id: r.id,
                        name: r.name,
                        description: r.description,
                        default_channel: r.default_channel,
                        channel_count: r.channel_count,
                        created_at_ms: r.created_at_ms,
                    })
                    .collect(),
                count: result.count,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn repo_info(client: &AspenClient, args: RepoInfoArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::PijulRepoInfo { repo_id: args.repo_id }).await?;

    match response {
        ClientRpcResponse::PijulRepoResult(result) => {
            let output = PijulRepoOutput {
                id: result.id,
                name: result.name,
                description: result.description,
                default_channel: result.default_channel,
                channel_count: result.channel_count,
                created_at_ms: result.created_at_ms,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

// =============================================================================
// Channel Handlers
// =============================================================================

async fn channel_list(client: &AspenClient, args: ChannelListArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::PijulChannelList { repo_id: args.repo_id }).await?;

    match response {
        ClientRpcResponse::PijulChannelListResult(result) => {
            let output = PijulChannelListOutput {
                channels: result
                    .channels
                    .into_iter()
                    .map(|c| PijulChannelOutput {
                        name: c.name,
                        head: c.head,
                        updated_at_ms: c.updated_at_ms,
                    })
                    .collect(),
                count: result.count,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn channel_create(client: &AspenClient, args: ChannelCreateArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulChannelCreate {
            repo_id: args.repo_id,
            name: args.name.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulChannelResult(result) => {
            let output = PijulChannelOutput {
                name: result.name,
                head: result.head,
                updated_at_ms: result.updated_at_ms,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn channel_delete(client: &AspenClient, args: ChannelDeleteArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulChannelDelete {
            repo_id: args.repo_id,
            name: args.name.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulSuccess => {
            print_success(&format!("Deleted channel '{}'", args.name), json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn channel_fork(client: &AspenClient, args: ChannelForkArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulChannelFork {
            repo_id: args.repo_id,
            source: args.source,
            target: args.target.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulChannelResult(result) => {
            let output = PijulChannelOutput {
                name: result.name,
                head: result.head,
                updated_at_ms: result.updated_at_ms,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn channel_info(client: &AspenClient, args: ChannelInfoArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulChannelInfo {
            repo_id: args.repo_id,
            name: args.name,
        })
        .await?;

    match response {
        ClientRpcResponse::PijulChannelResult(result) => {
            let output = PijulChannelOutput {
                name: result.name,
                head: result.head,
                updated_at_ms: result.updated_at_ms,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

// =============================================================================
// Change Handlers
// =============================================================================

async fn pijul_record(client: &AspenClient, args: RecordArgs, json: bool) -> Result<()> {
    // Parse repo ID first to determine cache directory
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Use provided data-dir or auto-use the cache directory
    let data_dir = args.data_dir.unwrap_or_else(|| repo_cache_dir(&repo_id).expect("failed to get cache dir"));

    // Always use local mode with the cache directory
    // Recording requires a local pristine - changes are then uploaded to the cluster
    pijul_record_local(
        client,
        args.repo_id,
        args.channel,
        args.working_dir,
        args.message,
        args.author,
        args.email,
        data_dir,
        args.threads,
        args.prefix,
        json,
    )
    .await
}

/// Record changes in local mode.
///
/// This uses a local pristine database while syncing changes to the cluster.
async fn pijul_record_local(
    client: &AspenClient,
    repo_id_str: String,
    channel: String,
    working_dir: String,
    message: String,
    author: Option<String>,
    email: Option<String>,
    data_dir: PathBuf,
    threads: usize,
    prefix: Option<String>,
    json: bool,
) -> Result<()> {
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(&repo_id_str).context("invalid repository ID format")?;

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&data_dir);
    let pristine = pristine_mgr.open_or_create(&repo_id).context("failed to open/create local pristine")?;

    // Create temporary in-memory blob store for recording
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs.clone()));
    let change_dir = ChangeDirectory::new(&data_dir, repo_id, change_store.clone());

    // Create author string
    let author_str = match (author, email) {
        (Some(name), Some(email)) => format!("{} <{}>", name, email),
        (Some(name), None) => name,
        (None, Some(email)) => format!("<{}>", email),
        (None, None) => "Unknown Author <unknown@local>".to_string(),
    };

    // Record changes with performance options
    let mut recorder =
        ChangeRecorder::new(pristine.clone(), change_dir, PathBuf::from(&working_dir)).with_threads(threads);

    if let Some(ref p) = prefix {
        recorder = recorder.with_prefix(p);
    }

    let result = recorder.record(&channel, &message, &author_str).await.context("failed to record changes")?;

    match result {
        Some(record_result) => {
            info!(
                hash = %record_result.hash,
                hunks = record_result.num_hunks,
                bytes = record_result.size_bytes,
                "recorded change locally"
            );

            // Get the change bytes from local store
            let change_bytes = change_store
                .get_change(&record_result.hash)
                .await
                .context("failed to get change from local store")?
                .context("change not found in local store after recording")?;

            // Upload change to cluster blob store
            let blob_response = client
                .send(ClientRpcRequest::AddBlob {
                    data: change_bytes.clone(),
                    tag: Some(format!("pijul:{}:{}", repo_id_str, record_result.hash)),
                })
                .await
                .context("failed to upload change to cluster")?;

            match blob_response {
                ClientRpcResponse::AddBlobResult(_) => {
                    info!(hash = %record_result.hash, "uploaded change to cluster blob store");
                }
                ClientRpcResponse::Error(e) => {
                    anyhow::bail!("failed to upload change: {}: {}", e.code, e.message);
                }
                _ => anyhow::bail!("unexpected response from blob add"),
            }

            // Apply the change to update channel head
            let apply_response = client
                .send(ClientRpcRequest::PijulApply {
                    repo_id: repo_id_str.clone(),
                    channel: channel.clone(),
                    change_hash: record_result.hash.to_string(),
                })
                .await
                .context("failed to apply change to cluster")?;

            match apply_response {
                ClientRpcResponse::PijulApplyResult(_) => {
                    info!(channel = %channel, "updated channel head via Raft");
                }
                ClientRpcResponse::Error(e) => {
                    anyhow::bail!("failed to apply change: {}: {}", e.code, e.message);
                }
                _ => anyhow::bail!("unexpected response from apply"),
            }

            // Store change metadata so pijul log works
            // Parse author string - expected format "Name <email>" or just "Name"
            let (author_name, author_email) = if let Some(start) = author_str.find('<') {
                if let Some(end) = author_str.find('>') {
                    let name = author_str[..start].trim().to_string();
                    let email = author_str[start + 1..end].to_string();
                    (name, email)
                } else {
                    (author_str.clone(), String::new())
                }
            } else {
                (author_str.clone(), String::new())
            };

            let metadata = ChangeMetadata {
                hash: record_result.hash,
                repo_id,
                channel: channel.clone(),
                message: message.clone(),
                authors: vec![PijulAuthor::from_name_email(author_name, author_email)],
                dependencies: record_result.dependencies.clone(),
                size_bytes: record_result.size_bytes as u64,
                recorded_at_ms: chrono::Utc::now().timestamp_millis() as u64,
            };

            let meta_bytes = postcard::to_allocvec(&metadata).context("failed to serialize change metadata")?;
            // Base64 encode for storage - get_change_metadata expects base64
            let meta_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &meta_bytes);

            let meta_key = format!("pijul:change:meta:{}:{}", repo_id_str, record_result.hash);

            let meta_response = client
                .send(ClientRpcRequest::WriteKey {
                    key: meta_key,
                    value: meta_b64.into_bytes(),
                })
                .await
                .context("failed to store change metadata")?;

            match meta_response {
                ClientRpcResponse::WriteResult(_) => {
                    info!(hash = %record_result.hash, "stored change metadata");
                }
                ClientRpcResponse::Error(e) => {
                    // Warn but don't fail - the change was applied successfully
                    tracing::warn!("failed to store metadata: {}: {}", e.code, e.message);
                }
                _ => {
                    tracing::warn!("unexpected response from metadata store");
                }
            }

            // Output result
            let output = PijulRecordOutput {
                change_hash: record_result.hash.to_string(),
                message,
                hunks: record_result.num_hunks as u32,
                size_bytes: record_result.size_bytes as u64,
            };
            print_output(&output, json);
            Ok(())
        }
        None => {
            print_success("No changes to record", json);
            Ok(())
        }
    }
}

async fn pijul_apply(client: &AspenClient, args: ApplyArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulApply {
            repo_id: args.repo_id,
            channel: args.channel.clone(),
            change_hash: args.change_hash.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulApplyResult(result) => {
            let output = PijulApplyOutput {
                change_hash: args.change_hash,
                channel: args.channel,
                operations: result.operations,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn pijul_unrecord(client: &AspenClient, args: UnrecordArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulUnrecord {
            repo_id: args.repo_id,
            channel: args.channel.clone(),
            change_hash: args.change_hash.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulUnrecordResult(result) => {
            let output = PijulUnrecordOutput {
                change_hash: args.change_hash,
                channel: args.channel,
                unrecorded: result.unrecorded,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn pijul_log(client: &AspenClient, args: LogArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulLog {
            repo_id: args.repo_id,
            channel: args.channel,
            limit: args.limit,
        })
        .await?;

    match response {
        ClientRpcResponse::PijulLogResult(result) => {
            let output = PijulLogOutput {
                entries: result
                    .entries
                    .into_iter()
                    .map(|e| PijulLogEntry {
                        change_hash: e.change_hash,
                        message: e.message,
                        author: e.author,
                        timestamp_ms: e.timestamp_ms,
                    })
                    .collect(),
                count: result.count,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn pijul_checkout(client: &AspenClient, args: CheckoutArgs, json: bool) -> Result<()> {
    // Check if we're using local mode
    if args.local {
        return pijul_checkout_local(args, json).await;
    }

    // Remote mode - request from server
    let response = client
        .send(ClientRpcRequest::PijulCheckout {
            repo_id: args.repo_id,
            channel: args.channel.clone(),
            output_dir: args.output_dir.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulCheckoutResult(result) => {
            let output = PijulCheckoutOutput {
                channel: args.channel,
                output_dir: args.output_dir,
                files_written: result.files_written,
                conflicts: result.conflicts,
            };
            print_output(&output, json);
            if result.conflicts > 0 {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

/// Checkout using local pristine cache.
async fn pijul_checkout_local(args: CheckoutArgs, json: bool) -> Result<()> {
    use aspen_pijul::WorkingDirOutput;
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Determine cache directory
    let cache_dir = args.data_dir.unwrap_or_else(|| repo_cache_dir(&repo_id).expect("failed to get cache dir"));

    // Check if cache exists
    if !cache_dir.exists() {
        anyhow::bail!("Local cache not found at {}. Run 'pijul sync {}' first.", cache_dir.display(), args.repo_id);
    }

    info!(cache_dir = %cache_dir.display(), "using local cache");

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open(&repo_id).context("failed to open local pristine - run 'pijul sync' first")?;

    // Create change directory
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store);

    // Create output directory
    let output_path = PathBuf::from(&args.output_dir);
    std::fs::create_dir_all(&output_path).context("failed to create output directory")?;

    // Output to working directory
    let outputter = WorkingDirOutput::new(pristine, change_dir, output_path);
    let result = outputter.output(&args.channel).context("failed to output to working directory")?;

    let conflicts = result.conflict_count() as u32;

    let output = PijulCheckoutOutput {
        channel: args.channel,
        output_dir: args.output_dir,
        files_written: 0, // WorkingDirOutput doesn't track this currently
        conflicts,
    };
    print_output(&output, json);

    if conflicts > 0 {
        std::process::exit(1);
    }
    Ok(())
}

// =============================================================================
// Sync Handler
// =============================================================================

/// Sync local pristine with cluster state.
async fn pijul_sync(client: &AspenClient, args: SyncArgs, json: bool) -> Result<()> {
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Determine cache directory
    let cache_dir = args.data_dir.unwrap_or_else(|| repo_cache_dir(&repo_id).expect("failed to get cache dir"));

    // Create cache directory if it doesn't exist
    std::fs::create_dir_all(&cache_dir).context("failed to create cache directory")?;

    info!(cache_dir = %cache_dir.display(), "using cache directory");

    // Get channels to sync
    let channels = if let Some(ref channel) = args.channel {
        vec![channel.clone()]
    } else {
        // Fetch all channels from the cluster
        let response = client
            .send(ClientRpcRequest::PijulChannelList {
                repo_id: args.repo_id.clone(),
            })
            .await?;

        match response {
            ClientRpcResponse::PijulChannelListResult(result) => result.channels.into_iter().map(|c| c.name).collect(),
            ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
            _ => anyhow::bail!("unexpected response type"),
        }
    };

    if channels.is_empty() {
        print_success("No channels to sync", json);
        return Ok(());
    }

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open_or_create(&repo_id).context("failed to open/create local pristine")?;

    // Create temporary blob store for fetching changes
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs.clone()));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store.clone());

    // Ensure change directory exists
    change_dir.ensure_dir().context("failed to create change directory")?;

    let mut total_fetched = 0u32;
    let mut total_applied = 0u32;
    let mut total_conflicts = 0u32;
    let mut all_synced = true;

    // Create the applicator once for all channels
    use aspen_pijul::ChangeApplicator;
    let applicator = ChangeApplicator::new(pristine.clone(), change_dir.clone());

    // Sync each channel
    for channel in &channels {
        info!(channel = %channel, "syncing channel");

        // Get the cluster's change log
        let log_response = client
            .send(ClientRpcRequest::PijulLog {
                repo_id: args.repo_id.clone(),
                channel: channel.clone(),
                limit: 10_000,
            })
            .await?;

        let cluster_log = match log_response {
            ClientRpcResponse::PijulLogResult(result) => result.entries,
            ClientRpcResponse::Error(e) => {
                tracing::warn!(channel = %channel, error = %e.message, "failed to fetch log");
                continue;
            }
            _ => continue,
        };

        if cluster_log.is_empty() {
            info!(channel = %channel, "channel is empty");
            continue;
        }

        // For each change in the log, fetch and apply if missing
        for entry in &cluster_log {
            let change_hash = aspen_pijul::ChangeHash::from_hex(&entry.change_hash).context("invalid change hash")?;

            // Check if we already have this change locally
            let change_path = change_dir.change_path(&change_hash);
            if change_path.exists() {
                continue;
            }

            // Fetch change from cluster blob store
            let blob_response = client
                .send(ClientRpcRequest::GetBlob {
                    hash: entry.change_hash.clone(),
                })
                .await;

            match blob_response {
                Ok(ClientRpcResponse::GetBlobResult(blob_result)) => {
                    if let Some(data) = blob_result.data {
                        // Store locally
                        change_store.store_change(&data).await.context("failed to store change locally")?;

                        // Write to change directory for libpijul
                        change_dir.ensure_dir()?;
                        std::fs::write(&change_path, &data).context("failed to write change file")?;

                        total_fetched += 1;
                        all_synced = false;
                        info!(hash = %change_hash, "fetched change");
                    }
                }
                Ok(ClientRpcResponse::Error(e)) => {
                    tracing::warn!(hash = %change_hash, error = %e.message, "failed to fetch change");
                }
                _ => {}
            }
        }

        // Apply changes to pristine
        for entry in &cluster_log {
            let change_hash = aspen_pijul::ChangeHash::from_hex(&entry.change_hash)?;
            let change_path = change_dir.change_path(&change_hash);

            if !change_path.exists() {
                continue;
            }

            match applicator.apply_local(channel, &change_hash) {
                Ok(_) => {
                    total_applied += 1;
                    all_synced = false;
                }
                Err(e) => {
                    // Change might already be applied, or there's a conflict
                    tracing::debug!(hash = %change_hash, error = %e, "apply result");
                }
            }
        }

        // Check for conflicts after applying all changes to this channel
        match applicator.check_conflicts(channel) {
            Ok(conflicts) => {
                total_conflicts += conflicts;
                if conflicts > 0 {
                    tracing::warn!(channel = %channel, conflicts = conflicts, "conflicts detected");
                }
            }
            Err(e) => {
                tracing::debug!(channel = %channel, error = %e, "failed to check conflicts");
            }
        }

        info!(channel = %channel, "channel sync complete");
    }

    // Output result
    let output = PijulSyncOutput {
        repo_id: args.repo_id,
        channel: args.channel,
        changes_fetched: total_fetched,
        changes_applied: total_applied,
        already_synced: all_synced && total_fetched == 0,
        conflicts: total_conflicts,
        cache_dir: cache_dir.display().to_string(),
    };
    print_output(&output, json);

    Ok(())
}

// =============================================================================
// Archive Handler
// =============================================================================

/// Export repository state as archive (directory or tarball).
///
/// This command outputs the pristine state to a directory, then optionally
/// packages it as a tarball. It uses the local pristine cache, so you must
/// run `pijul sync` first if you want the latest cluster state.
async fn pijul_archive(args: ArchiveArgs, json: bool) -> Result<()> {
    use std::fs::File;

    use aspen_pijul::WorkingDirOutput;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Determine cache directory
    let cache_dir = args.data_dir.unwrap_or_else(|| repo_cache_dir(&repo_id).expect("failed to get cache dir"));

    // Check if cache exists
    if !cache_dir.exists() {
        anyhow::bail!("Local cache not found at {}. Run 'pijul sync {}' first.", cache_dir.display(), args.repo_id);
    }

    info!(cache_dir = %cache_dir.display(), "using local cache");

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open(&repo_id).context("failed to open local pristine - run 'pijul sync' first")?;

    // Create change directory
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store);

    // Determine output format
    let output_path = PathBuf::from(&args.output_path);
    let is_tar_gz = args.output_path.ends_with(".tar.gz") || args.output_path.ends_with(".tgz");
    let is_tar = args.output_path.ends_with(".tar") || is_tar_gz;
    let format = if is_tar_gz {
        "tar.gz"
    } else if is_tar {
        "tar"
    } else {
        "directory"
    };

    // For tarballs, we need a temp directory first
    let (work_dir, cleanup_temp) = if is_tar {
        let temp_dir = std::env::temp_dir().join(format!(
            "aspen-archive-{}-{}",
            repo_id.to_hex(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).context("failed to create temp directory")?;
        (temp_dir, true)
    } else {
        std::fs::create_dir_all(&output_path).context("failed to create output directory")?;
        (output_path.clone(), false)
    };

    // Output to working directory (or temp dir for tarball)
    let outputter = WorkingDirOutput::new(pristine, change_dir, work_dir.clone());

    let result = if let Some(ref prefix) = args.prefix {
        outputter.output_prefix(&args.channel, prefix).context("failed to output repository state")?
    } else {
        outputter.output(&args.channel).context("failed to output repository state")?
    };

    let conflicts = result.conflict_count() as u32;

    // If tarball requested, create it
    let size_bytes = if is_tar {
        // Create the tarball
        let tar_file = File::create(&output_path).context("failed to create archive file")?;

        let size = if is_tar_gz {
            let encoder = GzEncoder::new(tar_file, Compression::default());
            let mut tar_builder = tar::Builder::new(encoder);
            tar_builder.append_dir_all(".", &work_dir).context("failed to add files to archive")?;
            let encoder = tar_builder.into_inner().context("failed to finish tar archive")?;
            let tar_file = encoder.finish().context("failed to finish gzip compression")?;
            tar_file.metadata().map(|m| m.len()).unwrap_or(0)
        } else {
            let mut tar_builder = tar::Builder::new(tar_file);
            tar_builder.append_dir_all(".", &work_dir).context("failed to add files to archive")?;
            let tar_file = tar_builder.into_inner().context("failed to finish tar archive")?;
            tar_file.metadata().map(|m| m.len()).unwrap_or(0)
        };

        // Clean up temp directory
        if cleanup_temp && let Err(e) = std::fs::remove_dir_all(&work_dir) {
            tracing::warn!(path = %work_dir.display(), error = %e, "failed to clean up temp dir");
        }

        size
    } else {
        // Calculate directory size
        calculate_dir_size(&output_path).unwrap_or(0)
    };

    info!(
        output = %args.output_path,
        format = format,
        size = size_bytes,
        conflicts = conflicts,
        "archive complete"
    );

    let output = PijulArchiveOutput {
        repo_id: args.repo_id,
        channel: args.channel,
        output_path: args.output_path,
        format: format.to_string(),
        size_bytes,
        conflicts,
    };
    print_output(&output, json);

    if conflicts > 0 {
        std::process::exit(1);
    }

    Ok(())
}

// =============================================================================
// Show Handler
// =============================================================================

/// Show details of a specific change.
///
/// Queries the cluster for change metadata and displays it in a human-readable
/// or JSON format.
async fn pijul_show(client: &AspenClient, args: ShowArgs, json: bool) -> Result<()> {
    // Send request to cluster
    let response = client
        .send(ClientRpcRequest::PijulShow {
            repo_id: args.repo_id.clone(),
            change_hash: args.change_hash.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulShowResult(result) => {
            let output = PijulShowOutput {
                change_hash: result.change_hash,
                repo_id: result.repo_id,
                channel: result.channel,
                message: result.message,
                authors: result.authors.into_iter().map(|a| (a.name, a.email)).collect(),
                dependencies: result.dependencies,
                size_bytes: result.size_bytes,
                recorded_at_ms: result.recorded_at_ms,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message);
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

// =============================================================================
// Blame Handler
// =============================================================================

/// Show change attribution for a file.
///
/// Queries the cluster for blame information and displays it in a human-readable
/// or JSON format.
async fn pijul_blame(client: &AspenClient, args: BlameArgs, json: bool) -> Result<()> {
    // Send request to cluster
    let response = client
        .send(ClientRpcRequest::PijulBlame {
            repo_id: args.repo_id.clone(),
            channel: args.channel.clone(),
            path: args.path.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulBlameResult(result) => {
            let output = PijulBlameOutput {
                path: result.path,
                channel: result.channel,
                repo_id: result.repo_id,
                attributions: result
                    .attributions
                    .into_iter()
                    .map(|a| BlameEntry {
                        change_hash: a.change_hash,
                        author: a.author,
                        author_email: a.author_email,
                        message: a.message,
                        recorded_at_ms: a.recorded_at_ms,
                        change_type: a.change_type,
                    })
                    .collect(),
                file_exists: result.file_exists,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message);
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

/// Calculate the total size of a directory recursively.
fn calculate_dir_size(path: &PathBuf) -> Result<u64> {
    let mut total = 0u64;

    if path.is_file() {
        return Ok(std::fs::metadata(path)?.len());
    }

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            total += calculate_dir_size(&path)?;
        } else {
            total += std::fs::metadata(&path)?.len();
        }
    }

    Ok(total)
}

// =============================================================================
// Pull Handler
// =============================================================================

/// Pull changes from the cluster to local pristine.
async fn pijul_pull(client: &AspenClient, args: PullArgs, json: bool) -> Result<()> {
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Determine cache directory
    let cache_dir = args.data_dir.unwrap_or_else(|| repo_cache_dir(&repo_id).expect("failed to get cache dir"));

    // Create cache directory if it doesn't exist
    std::fs::create_dir_all(&cache_dir).context("failed to create cache directory")?;

    info!(cache_dir = %cache_dir.display(), "using cache directory");

    // Get channels to pull
    let channels = if let Some(ref channel) = args.channel {
        vec![channel.clone()]
    } else {
        // Fetch all channels from the cluster
        let response = client
            .send(ClientRpcRequest::PijulChannelList {
                repo_id: args.repo_id.clone(),
            })
            .await?;

        match response {
            ClientRpcResponse::PijulChannelListResult(result) => result.channels.into_iter().map(|c| c.name).collect(),
            ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
            _ => anyhow::bail!("unexpected response type"),
        }
    };

    if channels.is_empty() {
        print_success("No channels to pull", json);
        return Ok(());
    }

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open_or_create(&repo_id).context("failed to open/create local pristine")?;

    // Create temporary blob store for fetching changes
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs.clone()));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store.clone());

    // Ensure change directory exists
    change_dir.ensure_dir().context("failed to create change directory")?;

    let mut total_fetched = 0u32;
    let mut total_applied = 0u32;
    let mut total_conflicts = 0u32;
    let mut all_up_to_date = true;

    // Create the applicator once for all channels
    use aspen_pijul::ChangeApplicator;
    let applicator = ChangeApplicator::new(pristine.clone(), change_dir.clone());

    // Pull each channel
    for channel in &channels {
        info!(channel = %channel, "pulling channel");

        // Get the cluster's change log
        let log_response = client
            .send(ClientRpcRequest::PijulLog {
                repo_id: args.repo_id.clone(),
                channel: channel.clone(),
                limit: 10_000,
            })
            .await?;

        let cluster_log = match log_response {
            ClientRpcResponse::PijulLogResult(result) => result.entries,
            ClientRpcResponse::Error(e) => {
                tracing::warn!(channel = %channel, error = %e.message, "failed to fetch log");
                continue;
            }
            _ => continue,
        };

        if cluster_log.is_empty() {
            info!(channel = %channel, "channel is empty");
            continue;
        }

        // For each change in the log, fetch if missing
        for entry in &cluster_log {
            let change_hash = aspen_pijul::ChangeHash::from_hex(&entry.change_hash).context("invalid change hash")?;

            // Check if we already have this change locally
            let change_path = change_dir.change_path(&change_hash);
            if change_path.exists() {
                continue;
            }

            // Fetch change from cluster blob store
            let blob_response = client
                .send(ClientRpcRequest::GetBlob {
                    hash: entry.change_hash.clone(),
                })
                .await;

            match blob_response {
                Ok(ClientRpcResponse::GetBlobResult(blob_result)) => {
                    if let Some(data) = blob_result.data {
                        // Store locally
                        change_store.store_change(&data).await.context("failed to store change locally")?;

                        // Write to change directory for libpijul
                        change_dir.ensure_dir()?;
                        std::fs::write(&change_path, &data).context("failed to write change file")?;

                        total_fetched += 1;
                        all_up_to_date = false;
                        info!(hash = %change_hash, "fetched change");
                    }
                }
                Ok(ClientRpcResponse::Error(e)) => {
                    tracing::warn!(hash = %change_hash, error = %e.message, "failed to fetch change");
                }
                _ => {}
            }
        }

        // Apply changes to pristine
        for entry in &cluster_log {
            let change_hash = aspen_pijul::ChangeHash::from_hex(&entry.change_hash)?;
            let change_path = change_dir.change_path(&change_hash);

            if !change_path.exists() {
                continue;
            }

            match applicator.apply_local(channel, &change_hash) {
                Ok(_) => {
                    total_applied += 1;
                    all_up_to_date = false;
                }
                Err(e) => {
                    // Change might already be applied, or there's a conflict
                    tracing::debug!(hash = %change_hash, error = %e, "apply result");
                }
            }
        }

        // Check for conflicts after applying all changes to this channel
        match applicator.check_conflicts(channel) {
            Ok(conflicts) => {
                total_conflicts += conflicts;
                if conflicts > 0 {
                    tracing::warn!(channel = %channel, conflicts = conflicts, "conflicts detected");
                }
            }
            Err(e) => {
                tracing::debug!(channel = %channel, error = %e, "failed to check conflicts");
            }
        }

        info!(channel = %channel, "channel pull complete");
    }

    // Output result
    let output = PijulPullOutput {
        repo_id: args.repo_id,
        channel: args.channel,
        changes_fetched: total_fetched,
        changes_applied: total_applied,
        already_up_to_date: all_up_to_date && total_fetched == 0,
        conflicts: total_conflicts,
        cache_dir: cache_dir.display().to_string(),
    };
    print_output(&output, json);

    Ok(())
}

// =============================================================================
// Push Handler
// =============================================================================

/// Push local changes to the cluster.
async fn pijul_push(client: &AspenClient, args: PushArgs, json: bool) -> Result<()> {
    use aspen_pijul::ChangeApplicator;
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Determine cache directory
    let cache_dir = args.data_dir.unwrap_or_else(|| repo_cache_dir(&repo_id).expect("failed to get cache dir"));

    // Check if cache exists
    if !cache_dir.exists() {
        anyhow::bail!(
            "Local cache not found at {}. Run 'pijul sync {}' or 'pijul record' first.",
            cache_dir.display(),
            args.repo_id
        );
    }

    info!(cache_dir = %cache_dir.display(), channel = %args.channel, "pushing to cluster");

    // Open local pristine
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open(&repo_id).context("failed to open local pristine - run 'pijul sync' first")?;

    // Create change directory
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs.clone()));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store.clone());

    // Get local changes from pristine for this channel
    let applicator = ChangeApplicator::new(pristine.clone(), change_dir.clone());
    let local_changes = applicator.list_channel_changes(&args.channel).context("failed to list local changes")?;

    if local_changes.is_empty() {
        let output = PijulPushOutput {
            repo_id: args.repo_id,
            channel: args.channel,
            changes_pushed: 0,
            already_up_to_date: true,
            cache_dir: cache_dir.display().to_string(),
        };
        print_output(&output, json);
        return Ok(());
    }

    // Get the cluster's change log to find what we need to push
    let log_response = client
        .send(ClientRpcRequest::PijulLog {
            repo_id: args.repo_id.clone(),
            channel: args.channel.clone(),
            limit: 10_000,
        })
        .await?;

    let cluster_hashes: std::collections::HashSet<String> = match log_response {
        ClientRpcResponse::PijulLogResult(result) => result.entries.into_iter().map(|e| e.change_hash).collect(),
        ClientRpcResponse::Error(e) => {
            // Channel might not exist on cluster yet, that's fine
            tracing::debug!(error = %e.message, "cluster log fetch failed, assuming empty");
            std::collections::HashSet::new()
        }
        _ => std::collections::HashSet::new(),
    };

    // Find changes in local that are not in cluster
    let mut changes_to_push = Vec::new();
    for hash in &local_changes {
        let hash_str = hash.to_string();
        if !cluster_hashes.contains(&hash_str) {
            changes_to_push.push(*hash);
        }
    }

    if changes_to_push.is_empty() {
        let output = PijulPushOutput {
            repo_id: args.repo_id,
            channel: args.channel,
            changes_pushed: 0,
            already_up_to_date: true,
            cache_dir: cache_dir.display().to_string(),
        };
        print_output(&output, json);
        return Ok(());
    }

    info!(count = changes_to_push.len(), "pushing changes to cluster");

    let mut changes_pushed = 0u32;

    for hash in &changes_to_push {
        // Read the change from local storage
        let change_path = change_dir.change_path(hash);
        if !change_path.exists() {
            tracing::warn!(hash = %hash, "change file not found locally, skipping");
            continue;
        }

        let change_bytes = std::fs::read(&change_path).context("failed to read local change file")?;

        // Upload change to cluster blob store
        let blob_response = client
            .send(ClientRpcRequest::AddBlob {
                data: change_bytes.clone(),
                tag: Some(format!("pijul:{}:{}", args.repo_id, hash)),
            })
            .await
            .context("failed to upload change to cluster")?;

        match blob_response {
            ClientRpcResponse::AddBlobResult(_) => {
                info!(hash = %hash, "uploaded change to cluster blob store");
            }
            ClientRpcResponse::Error(e) => {
                anyhow::bail!("failed to upload change: {}: {}", e.code, e.message);
            }
            _ => anyhow::bail!("unexpected response from blob add"),
        }

        // Apply the change to update channel head
        let apply_response = client
            .send(ClientRpcRequest::PijulApply {
                repo_id: args.repo_id.clone(),
                channel: args.channel.clone(),
                change_hash: hash.to_string(),
            })
            .await
            .context("failed to apply change to cluster")?;

        match apply_response {
            ClientRpcResponse::PijulApplyResult(_) => {
                info!(hash = %hash, channel = %args.channel, "applied change via Raft");
                changes_pushed += 1;
            }
            ClientRpcResponse::Error(e) => {
                anyhow::bail!("failed to apply change: {}: {}", e.code, e.message);
            }
            _ => anyhow::bail!("unexpected response from apply"),
        }
    }

    // Output result
    let output = PijulPushOutput {
        repo_id: args.repo_id,
        channel: args.channel,
        changes_pushed,
        already_up_to_date: false,
        cache_dir: cache_dir.display().to_string(),
    };
    print_output(&output, json);

    Ok(())
}

/// Pijul pull result output.
pub struct PijulPullOutput {
    pub repo_id: String,
    pub channel: Option<String>,
    pub changes_fetched: u32,
    pub changes_applied: u32,
    pub already_up_to_date: bool,
    pub conflicts: u32,
    pub cache_dir: String,
}

impl Outputable for PijulPullOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "repo_id": self.repo_id,
            "channel": self.channel,
            "changes_fetched": self.changes_fetched,
            "changes_applied": self.changes_applied,
            "already_up_to_date": self.already_up_to_date,
            "conflicts": self.conflicts,
            "cache_dir": self.cache_dir
        })
    }

    fn to_human(&self) -> String {
        let channel_str = self.channel.as_deref().unwrap_or("all channels");
        if self.already_up_to_date {
            format!(
                "Pull complete for {}\n\
                 Status:    Already up to date\n\
                 Cache:     {}",
                channel_str, self.cache_dir
            )
        } else if self.conflicts > 0 {
            format!(
                "Pull complete for {}\n\
                 Fetched:   {} changes\n\
                 Applied:   {} changes\n\
                 Conflicts: {} (review after checkout)\n\
                 Cache:     {}",
                channel_str, self.changes_fetched, self.changes_applied, self.conflicts, self.cache_dir
            )
        } else {
            format!(
                "Pull complete for {}\n\
                 Fetched:   {} changes\n\
                 Applied:   {} changes\n\
                 Cache:     {}",
                channel_str, self.changes_fetched, self.changes_applied, self.cache_dir
            )
        }
    }
}

/// Pijul push result output.
pub struct PijulPushOutput {
    pub repo_id: String,
    pub channel: String,
    pub changes_pushed: u32,
    pub already_up_to_date: bool,
    pub cache_dir: String,
}

impl Outputable for PijulPushOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "repo_id": self.repo_id,
            "channel": self.channel,
            "changes_pushed": self.changes_pushed,
            "already_up_to_date": self.already_up_to_date,
            "cache_dir": self.cache_dir
        })
    }

    fn to_human(&self) -> String {
        if self.already_up_to_date {
            format!(
                "Push complete for '{}'\n\
                 Status:    Already up to date\n\
                 Cache:     {}",
                self.channel, self.cache_dir
            )
        } else {
            format!(
                "Push complete for '{}'\n\
                 Pushed:    {} changes\n\
                 Cache:     {}",
                self.channel, self.changes_pushed, self.cache_dir
            )
        }
    }
}

// =============================================================================
// Diff Handler
// =============================================================================

/// Show differences between working directory and pristine state.
///
/// This uses the same recording mechanism as `pijul record` but only shows
/// what would be recorded without actually creating a change.
async fn pijul_diff(args: DiffArgs, json: bool) -> Result<()> {
    use aspen_pijul::ChangeRecorder;
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Determine cache directory
    let cache_dir = args.data_dir.unwrap_or_else(|| repo_cache_dir(&repo_id).expect("failed to get cache dir"));

    // Check for channel-to-channel diff
    if let Some(ref channel2) = args.channel2 {
        // Channel-to-channel diff: show changes in channel2 not in channel1
        return pijul_diff_channels(&args.repo_id, &args.channel, channel2, &cache_dir, json).await;
    }

    // Working directory diff - require working_dir
    let working_dir = args
        .working_dir
        .context("--working-dir is required when diffing working directory against a channel")?;

    // Check if cache exists
    if !cache_dir.exists() {
        anyhow::bail!("Local cache not found at {}. Run 'pijul sync {}' first.", cache_dir.display(), args.repo_id);
    }

    info!(cache_dir = %cache_dir.display(), "using local cache for diff");

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open(&repo_id).context("failed to open local pristine - run 'pijul sync' first")?;

    // Create temporary in-memory blob store
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store);

    // Create recorder with performance options to get diff info
    let mut recorder =
        ChangeRecorder::new(pristine, change_dir, PathBuf::from(&working_dir)).with_threads(args.threads);

    if let Some(ref prefix) = args.prefix {
        recorder = recorder.with_prefix(prefix);
    }

    // Get the diff (record with dry-run equivalent - we use the hunks info)
    let diff_result = recorder.diff(&args.channel).await.context("failed to compute diff")?;

    // Convert to output format
    let mut hunks = Vec::new();
    let mut files_added = 0u32;
    let mut files_deleted = 0u32;
    let mut files_modified = 0u32;
    let mut lines_added = 0u32;
    let mut lines_deleted = 0u32;

    for hunk_info in &diff_result.hunks {
        let change_type = match hunk_info.kind.as_str() {
            "add" | "new" => {
                files_added += 1;
                "add"
            }
            "delete" | "remove" => {
                files_deleted += 1;
                "delete"
            }
            "modify" | "edit" => {
                files_modified += 1;
                "modify"
            }
            "rename" => {
                files_modified += 1;
                "rename"
            }
            "permission" | "perm" => {
                files_modified += 1;
                "permission"
            }
            _ => {
                files_modified += 1;
                "modify"
            }
        };

        lines_added += hunk_info.additions;
        lines_deleted += hunk_info.deletions;

        hunks.push(DiffHunk {
            path: hunk_info.path.clone(),
            change_type: change_type.to_string(),
            additions: hunk_info.additions,
            deletions: hunk_info.deletions,
        });
    }

    let no_changes = hunks.is_empty();

    let output = PijulDiffOutput {
        repo_id: args.repo_id,
        channel: args.channel,
        channel2: None,
        working_dir: Some(working_dir),
        hunks,
        files_added,
        files_deleted,
        files_modified,
        lines_added,
        lines_deleted,
        no_changes,
    };

    print_output(&output, json);
    Ok(())
}

/// Diff between two channels.
///
/// Shows changes that are in channel2 but not in channel1.
async fn pijul_diff_channels(
    repo_id_str: &str,
    channel1: &str,
    channel2: &str,
    cache_dir: &PathBuf,
    json: bool,
) -> Result<()> {
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(repo_id_str).context("invalid repository ID format")?;

    // Check if cache exists
    if !cache_dir.exists() {
        anyhow::bail!("Local cache not found at {}. Run 'pijul sync {}' first.", cache_dir.display(), repo_id_str);
    }

    info!(
        cache_dir = %cache_dir.display(),
        channel1 = %channel1,
        channel2 = %channel2,
        "comparing channels"
    );

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(cache_dir);
    let pristine = pristine_mgr.open(&repo_id).context("failed to open local pristine - run 'pijul sync' first")?;

    // Get changes from both channels and compute the difference
    // This shows changes in channel2 that are not in channel1
    let diff_result = pristine.diff_channels(channel1, channel2).context("failed to compute channel diff")?;

    // Convert to output format
    let mut hunks = Vec::new();
    let mut files_added = 0u32;
    let mut files_deleted = 0u32;
    let mut files_modified = 0u32;
    let mut lines_added = 0u32;
    let mut lines_deleted = 0u32;

    for hunk_info in &diff_result.hunks {
        let change_type = match hunk_info.kind.as_str() {
            "add" | "new" => {
                files_added += 1;
                "add"
            }
            "delete" | "remove" => {
                files_deleted += 1;
                "delete"
            }
            _ => {
                files_modified += 1;
                "modify"
            }
        };

        lines_added += hunk_info.additions;
        lines_deleted += hunk_info.deletions;

        hunks.push(DiffHunk {
            path: hunk_info.path.clone(),
            change_type: change_type.to_string(),
            additions: hunk_info.additions,
            deletions: hunk_info.deletions,
        });
    }

    let no_changes = hunks.is_empty();

    let output = PijulDiffOutput {
        repo_id: repo_id_str.to_string(),
        channel: channel1.to_string(),
        channel2: Some(channel2.to_string()),
        working_dir: None,
        hunks,
        files_added,
        files_deleted,
        files_modified,
        lines_added,
        lines_deleted,
        no_changes,
    };

    print_output(&output, json);
    Ok(())
}
