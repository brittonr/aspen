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

use anyhow::{Context, Result};
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::print_output;
use crate::output::print_success;
use crate::output::Outputable;
use aspen::blob::InMemoryBlobStore;
use aspen::client_rpc::ClientRpcRequest;
use aspen::client_rpc::ClientRpcResponse;
use aspen::forge::identity::RepoId;
use aspen::pijul::{
    AspenChangeStore, ChangeDirectory, ChangeMetadata, ChangeRecorder, PijulAuthor,
    PristineManager,
};

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

    /// Record changes from working directory.
    Record(RecordArgs),

    /// Apply a change to a channel.
    Apply(ApplyArgs),

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

// =============================================================================
// Output Types
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
                channel_str, self.changes_fetched, self.changes_applied,
                self.conflicts, self.cache_dir
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

// =============================================================================
// Command Implementation
// =============================================================================

impl PijulCommand {
    /// Execute the pijul command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            PijulCommand::Repo(cmd) => cmd.run(client, json).await,
            PijulCommand::Channel(cmd) => cmd.run(client, json).await,
            PijulCommand::Record(args) => pijul_record(client, args, json).await,
            PijulCommand::Apply(args) => pijul_apply(client, args, json).await,
            PijulCommand::Log(args) => pijul_log(client, args, json).await,
            PijulCommand::Checkout(args) => pijul_checkout(client, args, json).await,
            PijulCommand::Sync(args) => pijul_sync(client, args, json).await,
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
    let response = client
        .send(ClientRpcRequest::PijulRepoList { limit: args.limit })
        .await?;

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
    let response = client
        .send(ClientRpcRequest::PijulRepoInfo {
            repo_id: args.repo_id,
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

// =============================================================================
// Channel Handlers
// =============================================================================

async fn channel_list(client: &AspenClient, args: ChannelListArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulChannelList {
            repo_id: args.repo_id,
        })
        .await?;

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
    let repo_id = RepoId::from_hex(&args.repo_id)
        .context("invalid repository ID format")?;

    // Use provided data-dir or auto-use the cache directory
    let data_dir = args.data_dir
        .unwrap_or_else(|| repo_cache_dir(&repo_id).expect("failed to get cache dir"));

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
    ).await
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
    let repo_id = RepoId::from_hex(&repo_id_str)
        .context("invalid repository ID format")?;

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&data_dir);
    let pristine = pristine_mgr
        .open_or_create(&repo_id)
        .context("failed to open/create local pristine")?;

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
    let mut recorder = ChangeRecorder::new(pristine.clone(), change_dir, PathBuf::from(&working_dir))
        .with_threads(threads);

    if let Some(ref p) = prefix {
        recorder = recorder.with_prefix(p);
    }

    let result = recorder
        .record(&channel, &message, &author_str)
        .await
        .context("failed to record changes")?;

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

            let meta_bytes = postcard::to_allocvec(&metadata)
                .context("failed to serialize change metadata")?;
            // Base64 encode for storage - get_change_metadata expects base64
            let meta_b64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &meta_bytes,
            );

            let meta_key = format!(
                "pijul:change:meta:{}:{}",
                repo_id_str, record_result.hash
            );

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
    use aspen::pijul::WorkingDirOutput;
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(&args.repo_id)
        .context("invalid repository ID format")?;

    // Determine cache directory
    let cache_dir = args.data_dir
        .unwrap_or_else(|| repo_cache_dir(&repo_id).expect("failed to get cache dir"));

    // Check if cache exists
    if !cache_dir.exists() {
        anyhow::bail!(
            "Local cache not found at {}. Run 'pijul sync {}' first.",
            cache_dir.display(),
            args.repo_id
        );
    }

    info!(cache_dir = %cache_dir.display(), "using local cache");

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr
        .open(&repo_id)
        .context("failed to open local pristine - run 'pijul sync' first")?;

    // Create change directory
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store);

    // Create output directory
    let output_path = PathBuf::from(&args.output_dir);
    std::fs::create_dir_all(&output_path)
        .context("failed to create output directory")?;

    // Output to working directory
    let outputter = WorkingDirOutput::new(pristine, change_dir, output_path);
    let result = outputter.output(&args.channel)
        .context("failed to output to working directory")?;

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
    let repo_id = RepoId::from_hex(&args.repo_id)
        .context("invalid repository ID format")?;

    // Determine cache directory
    let cache_dir = args.data_dir
        .unwrap_or_else(|| repo_cache_dir(&repo_id).expect("failed to get cache dir"));

    // Create cache directory if it doesn't exist
    std::fs::create_dir_all(&cache_dir)
        .context("failed to create cache directory")?;

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
            ClientRpcResponse::PijulChannelListResult(result) => {
                result.channels.into_iter().map(|c| c.name).collect()
            }
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
    let pristine = pristine_mgr
        .open_or_create(&repo_id)
        .context("failed to open/create local pristine")?;

    // Create temporary blob store for fetching changes
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs.clone()));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store.clone());

    // Ensure change directory exists
    change_dir.ensure_dir().context("failed to create change directory")?;

    let mut total_fetched = 0u32;
    let mut total_applied = 0u32;
    let total_conflicts = 0u32; // TODO: detect conflicts during apply
    let mut all_synced = true;

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
            let change_hash = aspen::pijul::ChangeHash::from_hex(&entry.change_hash)
                .context("invalid change hash")?;

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
                        change_store
                            .store_change(&data)
                            .await
                            .context("failed to store change locally")?;

                        // Write to change directory for libpijul
                        change_dir.ensure_dir()?;
                        std::fs::write(&change_path, &data)
                            .context("failed to write change file")?;

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
        use aspen::pijul::ChangeApplicator;
        let applicator = ChangeApplicator::new(pristine.clone(), change_dir.clone());

        for entry in &cluster_log {
            let change_hash = aspen::pijul::ChangeHash::from_hex(&entry.change_hash)?;
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
