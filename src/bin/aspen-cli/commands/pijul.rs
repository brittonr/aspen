//! Pijul version control commands.
//!
//! Commands for managing Pijul repositories, channels, and changes
//! using Aspen's distributed storage with iroh-blobs and Raft consensus.

use anyhow::Result;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::print_output;
use crate::output::print_success;
use crate::output::Outputable;
use aspen::client_rpc::ClientRpcRequest;
use aspen::client_rpc::ClientRpcResponse;

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
    let response = client
        .send(ClientRpcRequest::PijulRecord {
            repo_id: args.repo_id,
            channel: args.channel,
            working_dir: args.working_dir,
            message: args.message,
            author_name: args.author,
            author_email: args.email,
        })
        .await?;

    match response {
        ClientRpcResponse::PijulRecordResult(result) => {
            if let Some(change) = result.change {
                let output = PijulRecordOutput {
                    change_hash: change.hash,
                    message: change.message,
                    hunks: change.hunks,
                    size_bytes: change.size_bytes,
                };
                print_output(&output, json);
            } else {
                print_success("No changes to record", json);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
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
