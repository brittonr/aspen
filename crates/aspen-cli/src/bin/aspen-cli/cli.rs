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
#[cfg(feature = "automerge")]
use crate::commands::automerge::AutomergeCommand;
use crate::commands::barrier::BarrierCommand;
use crate::commands::blob::BlobCommand;
use crate::commands::branch::BranchCommand;
#[cfg(feature = "ci")]
use crate::commands::cache::CacheCommand;
#[cfg(feature = "ci")]
use crate::commands::ci::CiCommand;
use crate::commands::cluster::ClusterCommand;
use crate::commands::counter::CounterCommand;
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
#[cfg(feature = "plugins-rpc")]
use crate::commands::plugin::PluginCommand;
#[cfg(feature = "proxy")]
use crate::commands::proxy::ProxyCommand;
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
    #[arg(long = "timeout", default_value = "30000", global = true)]
    pub timeout_ms: u64,

    /// Output JSON instead of human-readable format.
    #[arg(long = "json", global = true)]
    pub is_json: bool,

    /// Enable verbose logging.
    #[arg(short = 'v', long = "verbose", global = true)]
    pub is_verbose: bool,

    /// Suppress all logging output (warnings, debug messages).
    ///
    /// Useful for scripting and when parsing JSON output.
    #[arg(short = 'q', long = "quiet", global = true)]
    pub is_quiet: bool,
}

/// Top-level command categories.
#[derive(Subcommand)]
pub enum Commands {
    /// Automerge CRDT document operations.
    ///
    /// Manage collaborative documents with automatic merge support.
    /// Documents are stored with Raft consensus for durability.
    #[cfg(feature = "automerge")]
    #[command(subcommand)]
    Automerge(AutomergeCommand),

    /// Distributed barrier operations.
    #[command(subcommand)]
    Barrier(BarrierCommand),

    /// Content-addressed blob storage.
    #[command(subcommand)]
    Blob(BlobCommand),

    /// Branch management for Git repositories.
    #[command(subcommand)]
    Branch(BranchCommand),

    /// Nix binary cache operations.
    ///
    /// Query and download from the distributed Nix binary cache.
    #[cfg(feature = "ci")]
    #[command(subcommand)]
    Cache(CacheCommand),

    /// CI/CD pipeline operations.
    ///
    /// Trigger, monitor, and manage CI pipelines with Nickel-based configuration.
    #[cfg(feature = "ci")]
    #[command(subcommand)]
    Ci(CiCommand),

    /// Cluster management commands.
    #[command(subcommand)]
    Cluster(ClusterCommand),

    /// Atomic counter operations.
    #[command(subcommand)]
    Counter(CounterCommand),

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

    /// WASM plugin management.
    ///
    /// Install, list, enable, disable, and remove WASM handler plugins.
    #[cfg(feature = "plugins-rpc")]
    #[command(subcommand)]
    Plugin(PluginCommand),

    /// HTTP proxy for TCP tunneling over iroh.
    ///
    /// Start local proxies that tunnel TCP traffic through remote Aspen nodes.
    #[cfg(feature = "proxy")]
    #[command(subcommand)]
    Proxy(ProxyCommand),

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
    /// Vault-compatible secrets management including KV, Transit, and PKI.
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
            return cmd.run(self.global.is_json);
        }

        // Handle proxy commands — they create their own iroh endpoint
        #[cfg(feature = "proxy")]
        if let Commands::Proxy(cmd) = self.command {
            let ticket = self
                .global
                .ticket
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("--ticket is required (or set ASPEN_TICKET)"))?;
            return cmd.run(ticket, self.global.is_json).await;
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
        let client = AspenClient::connect(ticket, Duration::from_millis(self.global.timeout_ms), cap_token)
            .await
            .context("failed to connect to cluster")?;

        // Dispatch to appropriate command handler
        match self.command {
            #[cfg(feature = "automerge")]
            Commands::Automerge(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Barrier(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Blob(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Branch(cmd) => cmd.run(&client, self.global.is_json).await,
            #[cfg(feature = "ci")]
            Commands::Cache(cmd) => cmd.run(&client, self.global.is_json).await,
            #[cfg(feature = "ci")]
            Commands::Ci(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Cluster(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Counter(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Docs(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Federation(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Git(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Hook(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Index(_) => unreachable!("handled above"),
            Commands::Issue(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Job(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Kv(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Lease(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Lock(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Patch(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Peer(cmd) => cmd.run(&client, self.global.is_json).await,
            #[cfg(feature = "plugins-rpc")]
            Commands::Plugin(cmd) => cmd.run(&client, self.global.is_json).await,
            #[cfg(feature = "proxy")]
            Commands::Proxy(_) => unreachable!("handled above"),
            Commands::Queue(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Ratelimit(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Rwlock(cmd) => cmd.run(&client, self.global.is_json).await,
            #[cfg(feature = "secrets")]
            Commands::Secrets(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Semaphore(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Sequence(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Service(cmd) => cmd.run(&client, self.global.is_json).await,
            #[cfg(feature = "sql")]
            Commands::Sql(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Tag(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Verify(cmd) => cmd.run(&client, self.global.is_json).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_help_does_not_panic() {
        // Verify CLI structure is valid by attempting to parse --help
        // --help causes clap to return an error with exit code 0
        let result = Cli::try_parse_from(["aspen-cli", "--help"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_parses_without_ticket() {
        // Ticket is optional at parse time (validated at runtime)
        let result = Cli::try_parse_from(["aspen-cli", "kv", "get", "mykey"]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        assert!(cli.global.ticket.is_none());
    }

    #[test]
    fn test_cli_parses_with_ticket() {
        let result = Cli::try_parse_from(["aspen-cli", "--ticket", "aspen1abc123", "kv", "get", "mykey"]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        assert_eq!(cli.global.ticket, Some("aspen1abc123".to_string()));
    }

    #[test]
    fn test_kv_get_requires_key() {
        // kv get without a key should fail
        let result = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "kv", "get"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_kv_set_requires_key_and_value_or_file() {
        // kv set with only key should fail (no value or --file)
        let result = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "kv", "set", "key"]);
        assert!(result.is_err());

        // kv set with key and value should succeed
        let result = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "kv", "set", "key", "value"]);
        assert!(result.is_ok());

        // kv set with key and --file should succeed (no value needed)
        let result = Cli::try_parse_from([
            "aspen-cli",
            "--ticket",
            "fake",
            "kv",
            "set",
            "key",
            "--file",
            "/tmp/test",
        ]);
        assert!(result.is_ok());

        // kv set with both value and --file should fail (conflicts)
        let result = Cli::try_parse_from([
            "aspen-cli",
            "--ticket",
            "fake",
            "kv",
            "set",
            "key",
            "value",
            "--file",
            "/tmp/test",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn test_timeout_default_value() {
        let result = Cli::try_parse_from(["aspen-cli", "kv", "get", "key"]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        // Default timeout is 30000ms (30 seconds)
        assert_eq!(cli.global.timeout_ms, 30000);
    }

    #[test]
    fn test_timeout_custom_value() {
        let result = Cli::try_parse_from(["aspen-cli", "--timeout", "60000", "kv", "get", "key"]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        assert_eq!(cli.global.timeout_ms, 60000);
    }

    #[test]
    fn test_json_flag_parsed() {
        let result = Cli::try_parse_from(["aspen-cli", "--json", "kv", "get", "key"]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        assert!(cli.global.is_json);
    }

    #[test]
    fn test_verbose_flag_parsed() {
        let result = Cli::try_parse_from(["aspen-cli", "-v", "kv", "get", "key"]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        assert!(cli.global.is_verbose);
    }

    #[test]
    fn test_quiet_flag_parsed() {
        let result = Cli::try_parse_from(["aspen-cli", "-q", "kv", "get", "key"]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        assert!(cli.global.is_quiet);
    }

    #[test]
    fn test_cluster_init_parses() {
        // cluster init has no arguments
        let result = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "cluster", "init"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cluster_add_learner_requires_args() {
        // add-learner requires --node-id and --addr
        let result = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "cluster", "add-learner"]);
        assert!(result.is_err());

        // With required args it should succeed
        let result = Cli::try_parse_from([
            "aspen-cli",
            "--ticket",
            "fake",
            "cluster",
            "add-learner",
            "--node-id",
            "2",
            "--addr",
            "node2:8080",
        ]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lock_acquire_requires_holder_and_ttl() {
        // lock acquire requires key, --holder, and --ttl
        let result = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "lock", "acquire", "mylock"]);
        assert!(result.is_err()); // Missing --holder and --ttl

        // With all required args it should succeed
        let result = Cli::try_parse_from([
            "aspen-cli",
            "--ticket",
            "fake",
            "lock",
            "acquire",
            "mylock",
            "--holder",
            "client1",
            "--ttl",
            "30000",
        ]);
        assert!(result.is_ok());
    }
}
