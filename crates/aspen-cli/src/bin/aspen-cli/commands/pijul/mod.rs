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

mod config;
mod operations;
pub mod output;
mod working_dir;

use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use aspen_forge::identity::RepoId;
use clap::Args;
use clap::Subcommand;
// Re-export all public output types so that existing `commands::pijul::*` paths work.
pub use output::*;

use crate::client::AspenClient;

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
// Command Implementation
// =============================================================================

impl PijulCommand {
    /// Execute the pijul command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            PijulCommand::Repo(cmd) => cmd.run(client, json).await,
            PijulCommand::Channel(cmd) => cmd.run(client, json).await,
            PijulCommand::Wd(cmd) => cmd.run(client, json).await,
            PijulCommand::Record(args) => operations::pijul_record(client, args, json).await,
            PijulCommand::Apply(args) => operations::pijul_apply(client, args, json).await,
            PijulCommand::Unrecord(args) => operations::pijul_unrecord(client, args, json).await,
            PijulCommand::Log(args) => operations::pijul_log(client, args, json).await,
            PijulCommand::Checkout(args) => operations::pijul_checkout(client, args, json).await,
            PijulCommand::Sync(args) => operations::pijul_sync(client, args, json).await,
            PijulCommand::Pull(args) => operations::pijul_pull(client, args, json).await,
            PijulCommand::Push(args) => operations::pijul_push(client, args, json).await,
            PijulCommand::Diff(args) => operations::pijul_diff(args, json).await,
            PijulCommand::Archive(args) => operations::pijul_archive(args, json).await,
            PijulCommand::Show(args) => operations::pijul_show(client, args, json).await,
            PijulCommand::Blame(args) => operations::pijul_blame(client, args, json).await,
            PijulCommand::Config(cmd) => cmd.run(json),
            PijulCommand::Remote(cmd) => cmd.run(json),
        }
    }
}

impl RepoCommand {
    /// Execute the repo subcommand.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            RepoCommand::Init(args) => operations::repo_init(client, args, json).await,
            RepoCommand::List(args) => operations::repo_list(client, args, json).await,
            RepoCommand::Info(args) => operations::repo_info(client, args, json).await,
        }
    }
}

impl ChannelCommand {
    /// Execute the channel subcommand.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            ChannelCommand::List(args) => operations::channel_list(client, args, json).await,
            ChannelCommand::Create(args) => operations::channel_create(client, args, json).await,
            ChannelCommand::Delete(args) => operations::channel_delete(client, args, json).await,
            ChannelCommand::Fork(args) => operations::channel_fork(client, args, json).await,
            ChannelCommand::Info(args) => operations::channel_info(client, args, json).await,
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
            WdCommand::Init(args) => working_dir::wd_init(args, json),
            WdCommand::Add(args) => working_dir::wd_add(args, json),
            WdCommand::Reset(args) => working_dir::wd_reset(args, json),
            WdCommand::Status(args) => working_dir::wd_status(args, json),
            WdCommand::Record(args) => working_dir::wd_record(client, args, json).await,
            WdCommand::Checkout(args) => working_dir::wd_checkout(client, args, json).await,
            WdCommand::Diff(args) => working_dir::wd_diff(args, json).await,
            WdCommand::Conflicts(args) => working_dir::wd_conflicts(args, json),
            WdCommand::Solve(args) => working_dir::wd_solve(args, json),
        }
    }
}

impl ConfigCommand {
    /// Execute the config subcommand.
    ///
    /// Config commands are local-only and don't require cluster connection.
    pub fn run(self, json: bool) -> Result<()> {
        match self {
            ConfigCommand::Get { key, global } => config::config_get(&key, global, json),
            ConfigCommand::Set { key, value, global } => config::config_set(&key, &value, global, json),
            ConfigCommand::List { global } => config::config_list(global, json),
            ConfigCommand::Unset { key, global } => config::config_unset(&key, global, json),
        }
    }
}

impl RemoteCommand {
    /// Execute the remote subcommand.
    ///
    /// Remote commands are local-only and don't require cluster connection.
    pub fn run(self, json: bool) -> Result<()> {
        match self {
            RemoteCommand::List => config::remote_list(json),
            RemoteCommand::Add { name, url } => config::remote_add(&name, &url, json),
            RemoteCommand::Remove { name } => config::remote_remove(&name, json),
            RemoteCommand::Show { name } => config::remote_show(&name, json),
        }
    }
}
