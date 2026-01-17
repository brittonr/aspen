//! CLI argument parsing and command dispatch.
//!
//! Uses clap derive macros for declarative argument definition with
//! support for environment variables and global options.

use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use clap::Args;
use clap::Parser;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::commands::barrier::BarrierCommand;
use crate::commands::blob::BlobCommand;
use crate::commands::branch::BranchCommand;
use crate::commands::cluster::ClusterCommand;
use crate::commands::counter::CounterCommand;
#[cfg(feature = "dns")]
use crate::commands::dns::DnsCommand;
use crate::commands::docs::DocsCommand;
use crate::commands::federation::FederationCommand;
use crate::commands::git::GitCommand;
use crate::commands::hooks::HookCommand;
use crate::commands::index::IndexCommand;
use crate::commands::issue::IssueCommand;
use crate::commands::job::JobCommand;
use crate::commands::kv::KvCommand;
use crate::commands::lease::LeaseCommand;
use crate::commands::lock::LockCommand;
use crate::commands::patch::PatchCommand;
use crate::commands::peer::PeerCommand;
#[cfg(feature = "pijul")]
use crate::commands::pijul::PijulCommand;
use crate::commands::queue::QueueCommand;
use crate::commands::ratelimit::RateLimitCommand;
use crate::commands::rwlock::RWLockCommand;
#[cfg(feature = "secrets")]
use crate::commands::secrets::SecretsCommand;
use crate::commands::semaphore::SemaphoreCommand;
use crate::commands::sequence::SequenceCommand;
use crate::commands::service::ServiceCommand;
#[cfg(feature = "sql")]
use crate::commands::sql::SqlCommand;
use crate::commands::tag::TagCommand;
use crate::commands::verify::VerifyCommand;

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
    ///
    /// Default is 30 seconds to accommodate complex operations like
    /// git log/get-blob/get-tree that may traverse large object graphs.
    #[arg(long, default_value = "30000", global = true)]
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

    /// Branch management for Git repositories.
    #[command(subcommand)]
    Branch(BranchCommand),

    /// Cluster management commands.
    #[command(subcommand)]
    Cluster(ClusterCommand),

    /// Atomic counter operations.
    #[command(subcommand)]
    Counter(CounterCommand),

    /// DNS record and zone management.
    ///
    /// Manage DNS records stored with Raft consensus and synchronized
    /// via iroh-docs to clients for local DNS resolution.
    #[cfg(feature = "dns")]
    #[command(subcommand)]
    Dns(DnsCommand),

    /// CRDT-replicated docs namespace operations.
    #[command(subcommand)]
    Docs(DocsCommand),

    /// Cross-cluster federation.
    ///
    /// Discover and sync with other Aspen clusters using DHT and gossip.
    #[command(subcommand)]
    Federation(FederationCommand),

    /// Git repository operations.
    ///
    /// Decentralized Git with BLAKE3 content-addressing and Raft-consistent refs.
    #[command(subcommand)]
    Git(GitCommand),

    /// Event-driven hook system operations.
    ///
    /// List handlers, view execution metrics, and manually trigger events.
    #[command(subcommand)]
    Hook(HookCommand),

    /// Secondary index operations.
    ///
    /// List and inspect secondary indexes used by the SQL query engine.
    #[command(subcommand)]
    Index(IndexCommand),

    /// Issue tracking (collaborative objects).
    ///
    /// Create and manage issues as immutable, content-addressed DAGs.
    #[command(subcommand)]
    Issue(IssueCommand),

    /// Job queue operations.
    ///
    /// Submit and manage distributed jobs with priority scheduling and worker pools.
    #[command(subcommand)]
    Job(JobCommand),

    /// Key-value store operations.
    #[command(subcommand)]
    Kv(KvCommand),

    /// Time-based lease operations.
    #[command(subcommand)]
    Lease(LeaseCommand),

    /// Distributed lock operations.
    #[command(subcommand)]
    Lock(LockCommand),

    /// Patch management (collaborative objects).
    ///
    /// Create and manage patches (pull requests) as immutable, content-addressed DAGs.
    #[command(subcommand)]
    Patch(PatchCommand),

    /// Peer cluster federation.
    #[command(subcommand)]
    Peer(PeerCommand),

    /// Pijul patch-based version control.
    ///
    /// Manage repositories, channels, and changes with P2P distribution
    /// via iroh-blobs and Raft-consistent channel refs.
    #[cfg(feature = "pijul")]
    #[command(subcommand)]
    Pijul(PijulCommand),

    /// Distributed queue operations.
    #[command(subcommand)]
    Queue(QueueCommand),

    /// Rate limiter operations.
    #[command(subcommand)]
    Ratelimit(RateLimitCommand),

    /// Read-write lock operations.
    #[command(subcommand)]
    Rwlock(RWLockCommand),

    /// Secrets engine operations.
    ///
    /// Vault-compatible secrets management including KV v2, Transit, and PKI.
    #[cfg(feature = "secrets")]
    #[command(subcommand)]
    Secrets(SecretsCommand),

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
    #[cfg(feature = "sql")]
    #[command(subcommand)]
    Sql(SqlCommand),

    /// Tag management for Git repositories.
    #[command(subcommand)]
    Tag(TagCommand),

    /// Verify cluster replication (KV, docs, blobs).
    ///
    /// Run verification tests to ensure data is being replicated correctly.
    #[command(subcommand)]
    Verify(VerifyCommand),
}

impl Cli {
    /// Execute the CLI command.
    pub async fn run(self) -> Result<()> {
        // Handle commands that don't require a cluster connection first
        if let Commands::Index(cmd) = self.command {
            return cmd.run(self.global.json);
        }

        // Validate ticket is provided for cluster commands
        let ticket = self
            .global
            .ticket
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("--ticket is required (or set ASPEN_TICKET)"))?;

        // Parse capability token if provided
        let cap_token = if let Some(ref token_b64) = self.global.token {
            let token =
                aspen_auth::CapabilityToken::from_base64(token_b64).context("failed to parse capability token")?;
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
            Commands::Branch(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Cluster(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Counter(cmd) => cmd.run(&client, self.global.json).await,
            #[cfg(feature = "dns")]
            Commands::Dns(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Docs(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Federation(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Git(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Hook(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Index(_) => unreachable!("handled above"),
            Commands::Issue(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Job(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Kv(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Lease(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Lock(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Patch(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Peer(cmd) => cmd.run(&client, self.global.json).await,
            #[cfg(feature = "pijul")]
            Commands::Pijul(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Queue(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Ratelimit(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Rwlock(cmd) => cmd.run(&client, self.global.json).await,
            #[cfg(feature = "secrets")]
            Commands::Secrets(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Semaphore(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Sequence(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Service(cmd) => cmd.run(&client, self.global.json).await,
            #[cfg(feature = "sql")]
            Commands::Sql(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Tag(cmd) => cmd.run(&client, self.global.json).await,
            Commands::Verify(cmd) => cmd.run(&client, self.global.json).await,
        }
    }
}
