//! CLI argument parsing and command dispatch.
//!
//! Uses clap derive macros for declarative argument definition with
//! support for environment variables and global options.

use anyhow::Context;
use anyhow::Result;
use clap::Args;
use clap::Parser;
use clap::Subcommand;
use std::time::Duration;

use crate::client::AspenClient;
use crate::commands::barrier::BarrierCommand;
use crate::commands::blob::BlobCommand;
use crate::commands::cluster::ClusterCommand;
use crate::commands::counter::CounterCommand;
use crate::commands::kv::KvCommand;
use crate::commands::lease::LeaseCommand;
use crate::commands::lock::LockCommand;
use crate::commands::peer::PeerCommand;
use crate::commands::queue::QueueCommand;
use crate::commands::ratelimit::RateLimitCommand;
use crate::commands::rwlock::RWLockCommand;
use crate::commands::semaphore::SemaphoreCommand;
use crate::commands::sequence::SequenceCommand;
use crate::commands::service::ServiceCommand;
use crate::commands::sql::SqlCommand;

/// Command-line interface for Aspen distributed system.
#[derive(Parser)]
#[command(name = "aspen-cli")]
#[command(version)]
#[command(about = "Command-line interface for Aspen distributed system")]
#[command(long_about = "Non-interactive CLI for managing Aspen clusters, key-value stores, \
    distributed coordination primitives, and more.")]
#[command(propagate_version = true)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOptions,

    #[command(subcommand)]
    pub command: Commands,
}

/// Global options available to all commands.
#[derive(Args, Clone)]
pub struct GlobalOptions {
    /// Cluster connection ticket (format: aspen...).
    ///
    /// Required for all commands. Can also be set via ASPEN_TICKET environment variable.
    #[arg(long, env = "ASPEN_TICKET", global = true)]
    pub ticket: Option<String>,

    /// Capability token for authentication (base64-encoded).
    ///
    /// Can also be set via ASPEN_TOKEN environment variable.
    #[arg(long, env = "ASPEN_TOKEN", global = true)]
    pub token: Option<String>,

    /// RPC timeout in milliseconds.
    #[arg(long, default_value = "5000", global = true)]
    pub timeout: u64,

    /// Output JSON instead of human-readable format.
    #[arg(long, global = true)]
    pub json: bool,

    /// Enable verbose logging.
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress all logging output (warnings, debug messages).
    ///
    /// Useful for scripting and when parsing JSON output.
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

/// Top-level command categories.
#[derive(Subcommand)]
pub enum Commands {
    /// Distributed barrier operations.
    #[command(subcommand)]
    Barrier(BarrierCommand),

    /// Content-addressed blob storage.
    #[command(subcommand)]
    Blob(BlobCommand),

    /// Cluster management commands.
    #[command(subcommand)]
    Cluster(ClusterCommand),

    /// Atomic counter operations.
    #[command(subcommand)]
    Counter(CounterCommand),

    /// Key-value store operations.
    #[command(subcommand)]
    Kv(KvCommand),

    /// Time-based lease operations.
    #[command(subcommand)]
    Lease(LeaseCommand),

    /// Distributed lock operations.
    #[command(subcommand)]
    Lock(LockCommand),

    /// Peer cluster federation.
    #[command(subcommand)]
    Peer(PeerCommand),

    /// Distributed queue operations.
    #[command(subcommand)]
    Queue(QueueCommand),

    /// Rate limiter operations.
    #[command(subcommand)]
    Ratelimit(RateLimitCommand),

    /// Read-write lock operations.
    #[command(subcommand)]
    Rwlock(RWLockCommand),

    /// Distributed semaphore operations.
    #[command(subcommand)]
    Semaphore(SemaphoreCommand),

    /// Sequence generator operations.
    #[command(subcommand)]
    Sequence(SequenceCommand),

    /// Service registry operations.
    #[command(subcommand)]
    Service(ServiceCommand),

    /// SQL query operations.
    ///
    /// Execute read-only SQL queries against the distributed state machine.
    #[command(subcommand)]
    Sql(SqlCommand),
}

impl Cli {
    /// Execute the CLI command.
    pub async fn run(self) -> Result<()> {
        // Validate ticket is provided
        let ticket = self
            .global
            .ticket
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("--ticket is required (or set ASPEN_TICKET)"))?;

        // Parse capability token if provided
        let cap_token = if let Some(ref token_b64) = self.global.token {
            let token =
                aspen::auth::CapabilityToken::from_base64(token_b64).context("failed to parse capability token")?;
            Some(token)
        } else {
            None
        };

        // Connect to the cluster
        let client = AspenClient::connect(ticket, Duration::from_millis(self.global.timeout), cap_token)
            .await
            .context("failed to connect to cluster")?;

        // Dispatch to appropriate command handler
        match self.command {
            Commands::Barrier(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Blob(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Cluster(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Counter(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Kv(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Lease(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Lock(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Peer(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Queue(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Ratelimit(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Rwlock(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Semaphore(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Sequence(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Service(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Sql(cmd) => cmd.run(&client, self.global.json).await,
        }
    }
}
