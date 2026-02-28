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
use crate::commands::alert::AlertCommand;
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
use crate::commands::metric::MetricCommand;
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
use crate::commands::trace::TraceCommand;
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

    /// Distributed trace query operations.
    ///
    /// List, get, and search traces stored by the observability pipeline.
    #[command(subcommand)]
    Trace(TraceCommand),

    /// Server-side metric operations.
    ///
    /// Ingest, list, and query metrics stored by the observability pipeline.
    #[command(subcommand)]
    Metric(MetricCommand),

    /// Alert rule management.
    ///
    /// Create, delete, list, get, and evaluate alert rules.
    #[command(subcommand)]
    Alert(AlertCommand),

    /// Verify cluster replication (KV, docs, blobs).
    ///
    /// Run verification tests to ensure data is being replicated correctly.
    #[command(subcommand)]
    Verify(VerifyCommand),
}

impl Cli {
    /// Execute the CLI command.
    pub async fn run(self) -> Result<()> {
        // Handle index commands that don't require a cluster connection
        if let Commands::Index(ref cmd) = self.command {
            if cmd.is_local() {
                if let Commands::Index(cmd) = self.command {
                    return cmd.run_local(self.global.is_json);
                }
            }
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
            Commands::Index(cmd) => cmd.run(&client, self.global.is_json).await,
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
            Commands::Trace(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Metric(cmd) => cmd.run(&client, self.global.is_json).await,
            Commands::Alert(cmd) => cmd.run(&client, self.global.is_json).await,
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

    /// All WASM-dependent subcommands must parse so the CLI can connect
    /// and receive a clean CAPABILITY_UNAVAILABLE error from the server
    /// rather than failing at argument parsing.
    #[test]
    fn test_wasm_dependent_commands_parse_correctly() {
        // hooks
        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "hook", "list"]);
        assert!(r.is_ok(), "hook list must parse");

        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "hook", "metrics"]);
        assert!(r.is_ok(), "hook metrics must parse");

        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "hook", "trigger", "write_committed"]);
        assert!(r.is_ok(), "hook trigger must parse");

        // coordination primitives
        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "counter", "get", "c"]);
        assert!(r.is_ok(), "counter get must parse");

        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "sequence", "next", "s"]);
        assert!(r.is_ok(), "sequence next must parse");

        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "service", "list"]);
        assert!(r.is_ok(), "service list must parse");

        // federation
        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "federation", "status"]);
        assert!(r.is_ok(), "federation status must parse");
    }

    /// ratelimit commands must require --rate and --capacity so
    /// validate_rate can reject bad values before any RPC call.
    #[test]
    fn test_ratelimit_requires_rate_and_capacity() {
        // Missing --rate and --capacity
        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "ratelimit", "try-acquire", "rl-key"]);
        assert!(r.is_err(), "ratelimit try-acquire without --rate must fail");

        // With required args
        let r = Cli::try_parse_from([
            "aspen-cli",
            "--ticket",
            "fake",
            "ratelimit",
            "try-acquire",
            "rl-key",
            "--rate",
            "1.0",
            "--capacity",
            "10",
        ]);
        assert!(r.is_ok(), "ratelimit try-acquire with all args must parse");
    }

    // =========================================================================
    // Trace CLI parse tests
    // =========================================================================

    #[test]
    fn test_trace_list_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "trace", "list"]);
        assert!(r.is_ok(), "trace list must parse");
    }

    #[test]
    fn test_trace_list_with_filters_parses() {
        let r = Cli::try_parse_from([
            "aspen-cli",
            "--ticket",
            "fake",
            "trace",
            "list",
            "--start",
            "1000000",
            "--end",
            "2000000",
            "--limit",
            "50",
        ]);
        assert!(r.is_ok(), "trace list with time range must parse");
    }

    #[test]
    fn test_trace_get_requires_trace_id() {
        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "trace", "get"]);
        assert!(r.is_err(), "trace get without trace_id must fail");
    }

    #[test]
    fn test_trace_get_with_trace_id_parses() {
        let r = Cli::try_parse_from([
            "aspen-cli",
            "--ticket",
            "fake",
            "trace",
            "get",
            "aabbccdd11223344aabbccdd11223344",
        ]);
        assert!(r.is_ok(), "trace get with trace_id must parse");
    }

    #[test]
    fn test_trace_search_parses_with_filters() {
        let r = Cli::try_parse_from([
            "aspen-cli",
            "--ticket",
            "fake",
            "trace",
            "search",
            "--operation",
            "kv.read",
            "--min-duration-us",
            "1000",
            "--max-duration-us",
            "5000000",
            "--status",
            "error",
            "--limit",
            "50",
        ]);
        assert!(r.is_ok(), "trace search with all filters must parse");
    }

    #[test]
    fn test_trace_search_no_filters_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "trace", "search"]);
        assert!(r.is_ok(), "trace search with no filters must parse");
    }

    // =========================================================================
    // Metric CLI parse tests
    // =========================================================================

    #[test]
    fn test_metric_list_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "metric", "list"]);
        assert!(r.is_ok(), "metric list must parse");
    }

    #[test]
    fn test_metric_query_requires_name() {
        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "metric", "query"]);
        assert!(r.is_err(), "metric query without name must fail");
    }

    #[test]
    fn test_metric_query_with_all_args_parses() {
        let r = Cli::try_parse_from([
            "aspen-cli",
            "--ticket",
            "fake",
            "metric",
            "query",
            "cpu.usage",
            "--start-us",
            "1000000",
            "--end-us",
            "2000000",
            "--label",
            "node=1",
            "--aggregation",
            "avg",
            "--step-us",
            "60000000",
            "--limit",
            "100",
        ]);
        assert!(r.is_ok(), "metric query with all args must parse: {:?}", r.err());
    }

    #[test]
    fn test_metric_ingest_parses() {
        let r = Cli::try_parse_from([
            "aspen-cli",
            "--ticket",
            "fake",
            "metric",
            "ingest",
            "--ttl-seconds",
            "3600",
        ]);
        assert!(r.is_ok(), "metric ingest must parse");
    }

    // =========================================================================
    // Alert CLI parse tests
    // =========================================================================

    #[test]
    fn test_alert_list_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "alert", "list"]);
        assert!(r.is_ok(), "alert list must parse");
    }

    #[test]
    fn test_alert_create_requires_metric_and_threshold() {
        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "alert", "create", "my_rule"]);
        assert!(r.is_err(), "alert create without --metric must fail");
    }

    #[test]
    fn test_alert_create_with_all_args_parses() {
        let r = Cli::try_parse_from([
            "aspen-cli",
            "--ticket",
            "fake",
            "alert",
            "create",
            "high_cpu",
            "--metric",
            "cpu.usage",
            "--comparison",
            "gt",
            "--threshold",
            "90.0",
            "--aggregation",
            "avg",
            "--window-seconds",
            "300",
            "--for-seconds",
            "60",
            "--severity",
            "critical",
            "--label",
            "node=1",
        ]);
        assert!(r.is_ok(), "alert create with all args must parse: {:?}", r.err());
    }

    #[test]
    fn test_alert_delete_requires_name() {
        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "alert", "delete"]);
        assert!(r.is_err(), "alert delete without name must fail");
    }

    #[test]
    fn test_alert_get_requires_name() {
        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "alert", "get"]);
        assert!(r.is_err(), "alert get without name must fail");
    }

    #[test]
    fn test_alert_evaluate_requires_name() {
        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "alert", "evaluate"]);
        assert!(r.is_err(), "alert evaluate without name must fail");
    }

    #[test]
    fn test_alert_evaluate_with_name_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "--ticket", "fake", "alert", "evaluate", "my_rule"]);
        assert!(r.is_ok(), "alert evaluate with name must parse");
    }

    // =========================================================================
    // Global options
    // =========================================================================

    #[test]
    fn test_global_json_flag() {
        let cli = Cli::try_parse_from(["aspen-cli", "--json", "verify", "all"]).expect("parse");
        assert!(cli.global.is_json);
    }

    #[test]
    fn test_global_timeout_default() {
        let cli = Cli::try_parse_from(["aspen-cli", "verify", "all"]).expect("parse");
        assert_eq!(cli.global.timeout_ms, 30000);
    }

    #[test]
    fn test_global_timeout_custom() {
        let cli = Cli::try_parse_from(["aspen-cli", "--timeout", "5000", "verify", "all"]).expect("parse");
        assert_eq!(cli.global.timeout_ms, 5000);
    }

    #[test]
    fn test_global_verbose_flag() {
        let cli = Cli::try_parse_from(["aspen-cli", "-v", "verify", "all"]).expect("parse");
        assert!(cli.global.is_verbose);
    }

    #[test]
    fn test_global_quiet_flag() {
        let cli = Cli::try_parse_from(["aspen-cli", "-q", "verify", "all"]).expect("parse");
        assert!(cli.global.is_quiet);
    }

    #[test]
    fn test_global_ticket_option() {
        let cli = Cli::try_parse_from(["aspen-cli", "--ticket", "aspen-ticket-123", "verify", "all"]).expect("parse");
        assert_eq!(cli.global.ticket.as_deref(), Some("aspen-ticket-123"));
    }

    #[test]
    fn test_global_token_option() {
        let cli = Cli::try_parse_from(["aspen-cli", "--token", "base64token", "verify", "all"]).expect("parse");
        assert_eq!(cli.global.token.as_deref(), Some("base64token"));
    }

    #[test]
    fn test_no_command_shows_help() {
        let r = Cli::try_parse_from(["aspen-cli"]);
        assert!(r.is_err(), "no subcommand should fail with help");
    }

    #[test]
    fn test_invalid_command_fails() {
        let r = Cli::try_parse_from(["aspen-cli", "nonexistent-command"]);
        assert!(r.is_err());
    }

    // =========================================================================
    // KV command parsing (without ticket - tests ticket-optional parsing)
    // =========================================================================

    #[test]
    fn test_kv_get_no_ticket_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "kv", "get", "mykey"]);
        assert!(r.is_ok(), "kv get without ticket must parse");
    }

    #[test]
    fn test_kv_delete_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "kv", "delete", "mykey"]);
        assert!(r.is_ok(), "kv delete with key must parse");
    }

    #[test]
    fn test_kv_scan_parses_with_default_prefix() {
        // scan has prefix as positional with default ""
        let r = Cli::try_parse_from(["aspen-cli", "kv", "scan"]);
        assert!(r.is_ok(), "kv scan without prefix must parse: {:?}", r.err());
    }

    #[test]
    fn test_kv_scan_with_limit_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "kv", "scan", "app/", "--limit", "50"]);
        assert!(r.is_ok(), "kv scan with prefix and limit must parse: {:?}", r.err());
    }

    // =========================================================================
    // Cluster command parsing
    // =========================================================================

    #[test]
    fn test_cluster_status_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "cluster", "status"]);
        assert!(r.is_ok(), "cluster status must parse: {:?}", r.err());
    }

    #[test]
    fn test_cluster_health_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "cluster", "health"]);
        assert!(r.is_ok(), "cluster health must parse: {:?}", r.err());
    }

    #[test]
    fn test_cluster_metrics_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "cluster", "metrics"]);
        assert!(r.is_ok(), "cluster metrics must parse: {:?}", r.err());
    }

    #[test]
    fn test_cluster_ticket_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "cluster", "ticket"]);
        assert!(r.is_ok(), "cluster ticket must parse: {:?}", r.err());
    }

    // =========================================================================
    // Blob command parsing
    // =========================================================================

    #[test]
    fn test_blob_add_with_data_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "blob", "add", "--data", "hello-world"]);
        assert!(r.is_ok(), "blob add --data must parse: {:?}", r.err());
    }

    #[test]
    fn test_blob_add_with_file_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "blob", "add", "/tmp/test.txt"]);
        assert!(r.is_ok(), "blob add with file path must parse: {:?}", r.err());
    }

    #[test]
    fn test_blob_add_no_args_parses() {
        // Both file and data are optional (reads stdin)
        let r = Cli::try_parse_from(["aspen-cli", "blob", "add"]);
        assert!(r.is_ok(), "blob add with no args must parse: {:?}", r.err());
    }

    #[test]
    fn test_blob_get_requires_hash() {
        let r = Cli::try_parse_from(["aspen-cli", "blob", "get"]);
        assert!(r.is_err(), "blob get without hash must fail");
    }

    #[test]
    fn test_blob_list_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "blob", "list"]);
        assert!(r.is_ok(), "blob list must parse");
    }

    // =========================================================================
    // Docs command parsing
    // =========================================================================

    #[test]
    fn test_docs_status_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "docs", "status"]);
        assert!(r.is_ok(), "docs status must parse");
    }

    #[test]
    fn test_docs_set_requires_key_and_value() {
        let r = Cli::try_parse_from(["aspen-cli", "docs", "set"]);
        assert!(r.is_err(), "docs set without key must fail");
    }

    #[test]
    fn test_docs_get_requires_key() {
        let r = Cli::try_parse_from(["aspen-cli", "docs", "get"]);
        assert!(r.is_err(), "docs get without key must fail");
    }

    // =========================================================================
    // Queue command parsing
    // =========================================================================

    #[test]
    fn test_queue_create_requires_name() {
        let r = Cli::try_parse_from(["aspen-cli", "queue", "create"]);
        assert!(r.is_err(), "queue create without name must fail");
    }

    #[test]
    fn test_queue_create_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "queue", "create", "myqueue"]);
        assert!(r.is_ok(), "queue create must parse: {:?}", r.err());
    }

    #[test]
    fn test_queue_enqueue_requires_name() {
        let r = Cli::try_parse_from(["aspen-cli", "queue", "enqueue"]);
        assert!(r.is_err(), "queue enqueue without name must fail");
    }

    #[test]
    fn test_queue_status_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "queue", "status", "myqueue"]);
        assert!(r.is_ok(), "queue status must parse: {:?}", r.err());
    }

    // =========================================================================
    // Lease command parsing
    // =========================================================================

    #[test]
    fn test_lease_grant_requires_ttl() {
        let r = Cli::try_parse_from(["aspen-cli", "lease", "grant"]);
        assert!(r.is_err(), "lease grant without ttl must fail");
    }

    #[test]
    fn test_lease_grant_parses() {
        // ttl_secs is a positional arg
        let r = Cli::try_parse_from(["aspen-cli", "lease", "grant", "60"]);
        assert!(r.is_ok(), "lease grant with ttl must parse: {:?}", r.err());
    }

    #[test]
    fn test_lease_revoke_requires_id() {
        let r = Cli::try_parse_from(["aspen-cli", "lease", "revoke"]);
        assert!(r.is_err(), "lease revoke without id must fail");
    }

    // =========================================================================
    // Job command parsing
    // =========================================================================

    #[test]
    fn test_job_list_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "job", "list"]);
        assert!(r.is_ok(), "job list must parse");
    }

    #[test]
    fn test_job_status_requires_id() {
        let r = Cli::try_parse_from(["aspen-cli", "job", "status"]);
        assert!(r.is_err(), "job status without id must fail");
    }

    // =========================================================================
    // Barrier command parsing
    // =========================================================================

    #[test]
    fn test_barrier_enter_requires_key() {
        let r = Cli::try_parse_from(["aspen-cli", "barrier", "enter"]);
        assert!(r.is_err(), "barrier enter without key must fail");
    }

    #[test]
    fn test_barrier_status_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "barrier", "status", "mybarrier"]);
        assert!(r.is_ok(), "barrier status must parse: {:?}", r.err());
    }

    // =========================================================================
    // Semaphore command parsing
    // =========================================================================

    #[test]
    fn test_semaphore_acquire_requires_key() {
        let r = Cli::try_parse_from(["aspen-cli", "semaphore", "acquire"]);
        assert!(r.is_err(), "semaphore acquire without key must fail");
    }

    #[test]
    fn test_semaphore_status_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "semaphore", "status", "mysem"]);
        assert!(r.is_ok(), "semaphore status must parse: {:?}", r.err());
    }

    // =========================================================================
    // Sequence command parsing
    // =========================================================================

    #[test]
    fn test_sequence_next_requires_key() {
        let r = Cli::try_parse_from(["aspen-cli", "sequence", "next"]);
        assert!(r.is_err(), "sequence next without key must fail");
    }

    // =========================================================================
    // Index command parsing
    // =========================================================================

    #[test]
    fn test_index_list_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "index", "list"]);
        assert!(r.is_ok(), "index list must parse");
    }

    #[test]
    fn test_index_scan_requires_name() {
        let r = Cli::try_parse_from(["aspen-cli", "index", "scan"]);
        assert!(r.is_err(), "index scan without name must fail");
    }

    // =========================================================================
    // Git command parsing
    // =========================================================================

    #[test]
    fn test_git_log_requires_repo() {
        // --repo is required
        let r = Cli::try_parse_from(["aspen-cli", "git", "log"]);
        assert!(r.is_err(), "git log without --repo must fail");
    }

    #[test]
    fn test_git_log_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "git", "log", "--repo", "myrepo"]);
        assert!(r.is_ok(), "git log with --repo must parse: {:?}", r.err());
    }

    // =========================================================================
    // Issue command parsing
    // =========================================================================

    #[test]
    fn test_issue_list_requires_repo() {
        // --repo is a required flag
        let r = Cli::try_parse_from(["aspen-cli", "issue", "list"]);
        assert!(r.is_err(), "issue list without --repo must fail");
    }

    #[test]
    fn test_issue_list_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "issue", "list", "--repo", "myrepo"]);
        assert!(r.is_ok(), "issue list must parse: {:?}", r.err());
    }

    // =========================================================================
    // Patch command parsing
    // =========================================================================

    #[test]
    fn test_patch_list_requires_repo() {
        let r = Cli::try_parse_from(["aspen-cli", "patch", "list"]);
        assert!(r.is_err(), "patch list without --repo must fail");
    }

    #[test]
    fn test_patch_list_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "patch", "list", "--repo", "myrepo"]);
        assert!(r.is_ok(), "patch list must parse: {:?}", r.err());
    }

    // =========================================================================
    // Federation command parsing
    // =========================================================================

    #[test]
    fn test_federation_status_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "federation", "status"]);
        assert!(r.is_ok(), "federation status must parse");
    }

    #[test]
    fn test_federation_trust_requires_peer() {
        let r = Cli::try_parse_from(["aspen-cli", "federation", "trust"]);
        assert!(r.is_err(), "federation trust without peer must fail");
    }

    // =========================================================================
    // Verify command parsing
    // =========================================================================

    #[test]
    fn test_verify_all_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "verify", "all"]);
        assert!(r.is_ok(), "verify all must parse");
    }

    #[test]
    fn test_verify_kv_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "verify", "kv"]);
        assert!(r.is_ok(), "verify kv must parse");
    }

    #[test]
    fn test_verify_blob_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "verify", "blob"]);
        assert!(r.is_ok(), "verify blob must parse: {:?}", r.err());
    }

    // =========================================================================
    // RWLock command parsing
    // =========================================================================

    #[test]
    fn test_rwlock_read_requires_key_and_holder() {
        let r = Cli::try_parse_from(["aspen-cli", "rwlock", "read", "myrwlock"]);
        assert!(r.is_err(), "rwlock read without --holder must fail");
    }

    #[test]
    fn test_rwlock_write_requires_key_and_holder() {
        let r = Cli::try_parse_from(["aspen-cli", "rwlock", "write", "myrwlock"]);
        assert!(r.is_err(), "rwlock write without --holder must fail");
    }

    // =========================================================================
    // Service command parsing
    // =========================================================================

    #[test]
    fn test_service_list_parses() {
        let r = Cli::try_parse_from(["aspen-cli", "service", "list"]);
        assert!(r.is_ok(), "service list must parse");
    }

    #[test]
    fn test_service_register_requires_name() {
        let r = Cli::try_parse_from(["aspen-cli", "service", "register"]);
        assert!(r.is_err(), "service register without name must fail");
    }

    // =========================================================================
    // Combined flags
    // =========================================================================

    #[test]
    fn test_all_global_flags_together() {
        let cli = Cli::try_parse_from([
            "aspen-cli",
            "--json",
            "-v",
            "--ticket",
            "my-ticket",
            "--token",
            "my-token",
            "--timeout",
            "10000",
            "verify",
            "all",
        ])
        .expect("all global flags must parse together");
        assert!(cli.global.is_json);
        assert!(cli.global.is_verbose);
        assert_eq!(cli.global.ticket.as_deref(), Some("my-ticket"));
        assert_eq!(cli.global.token.as_deref(), Some("my-token"));
        assert_eq!(cli.global.timeout_ms, 10000);
    }

    #[test]
    fn test_global_flags_after_subcommand() {
        // clap propagated globals work after the subcommand too
        let cli = Cli::try_parse_from(["aspen-cli", "verify", "all", "--json"]).expect("json after subcommand");
        assert!(cli.global.is_json);
    }
}
