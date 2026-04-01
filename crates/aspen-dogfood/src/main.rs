//! Dogfood orchestrator for self-hosted Aspen build pipeline.
//!
//! Replaces the shell scripts (`dogfood-local.sh`, `dogfood-local-vmci.sh`,
//! `dogfood-federation.sh`) with a typed Rust binary that uses `aspen-client`
//! directly over Iroh instead of shelling out to `aspen-cli`.

mod ci;
mod cluster;
mod deploy;
mod error;
mod forge;
mod node;
mod state;

use clap::Parser;
use clap::Subcommand;
use tracing::info;

use crate::error::DogfoodResult;
use crate::node::NodeManager;
use crate::state::DogfoodState;

/// Dogfood orchestrator — self-hosted Aspen build pipeline.
#[derive(Parser)]
#[command(name = "aspen-dogfood", about = "Self-hosted Aspen build pipeline")]
struct Cli {
    /// Enable federation mode (two independent clusters).
    #[arg(long, global = true)]
    federation: bool,

    /// Enable VM-isolated CI execution.
    #[arg(long, global = true)]
    vm_ci: bool,

    /// Cluster data directory.
    #[arg(long, global = true, default_value = "/tmp/aspen-dogfood")]
    cluster_dir: String,

    /// Number of nodes per cluster.
    #[arg(long, global = true, default_value_t = 1)]
    node_count: u32,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the dogfood cluster.
    Start,
    /// Stop the dogfood cluster.
    Stop,
    /// Show cluster status.
    Status,
    /// Push source code to Forge.
    Push,
    /// Trigger or wait for CI build.
    Build,
    /// Deploy CI-built artifact to the cluster.
    Deploy,
    /// Verify the deployed artifact.
    Verify,
    /// Run build → deploy → verify in sequence.
    FullLoop,
    /// Run the full pipeline: start → push → build → deploy → verify → stop.
    Full,
}

#[tokio::main]
async fn main() {
    // Note: ASPEN_RELAY_DISABLED is set on spawned node processes only (in spawn_node),
    // NOT globally. The client endpoint needs relay enabled for QUIC signaling even
    // when connecting to local nodes.

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        tracing::error!("❌ {e}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> DogfoodResult<()> {
    let config = RunConfig {
        cluster_dir: cli.cluster_dir.clone(),
        federation: cli.federation,
        vm_ci: cli.vm_ci,
        node_count: cli.node_count,
        aspen_node_bin: std::env::var("ASPEN_NODE_BIN").unwrap_or_else(|_| "aspen-node".to_string()),
        git_remote_aspen_bin: std::env::var("GIT_REMOTE_ASPEN_BIN").unwrap_or_else(|_| "git-remote-aspen".to_string()),
        project_dir: std::env::var("PROJECT_DIR").unwrap_or_else(|_| ".".to_string()),
        nix_cache_gateway_bin: std::env::var("ASPEN_NIX_CACHE_GATEWAY_BIN").ok(),
        ci_timeout_secs: 600,
    };

    match cli.command {
        Command::Start => cmd_start(&config).await,
        Command::Stop => cmd_stop(&config).await,
        Command::Status => cmd_status(&config).await,
        Command::Push => cmd_push(&config).await,
        Command::Build => cmd_build(&config).await,
        Command::Deploy => cmd_deploy(&config).await,
        Command::Verify => cmd_verify(&config).await,
        Command::FullLoop => cmd_full_loop(&config).await,
        Command::Full => cmd_full(&config).await,
    }
}

/// Runtime configuration derived from CLI args and environment.
pub struct RunConfig {
    pub cluster_dir: String,
    pub federation: bool,
    pub vm_ci: bool,
    pub node_count: u32,
    pub aspen_node_bin: String,
    pub git_remote_aspen_bin: String,
    pub project_dir: String,
    pub nix_cache_gateway_bin: Option<String>,
    pub ci_timeout_secs: u64,
}

impl RunConfig {
    fn state_file_path(&self) -> String {
        format!("{}/dogfood-state.json", self.cluster_dir)
    }

    fn cookie(&self) -> String {
        let date = chrono::Local::now().format("%Y%m%d");
        format!("dogfood-{date}")
    }
}

// ── Subcommand implementations ───────────────────────────────────────

async fn cmd_start(config: &RunConfig) -> DogfoodResult<()> {
    info!("🚀 Starting dogfood cluster...");

    if config.federation {
        cmd_start_federation(config).await
    } else {
        cmd_start_single(config).await
    }
}

async fn cmd_start_single(config: &RunConfig) -> DogfoodResult<()> {
    let mut manager = NodeManager::new();

    let node_info = cluster::start_single_node(&mut manager, config).await?;

    let state = DogfoodState::new_single(
        node_info.pid,
        node_info.ticket.clone(),
        node_info.endpoint_addr.clone(),
        config.vm_ci,
    );
    state::write_state(&config.state_file_path(), &state)?;

    info!(
        "✅ Single-node cluster running (ticket: {}...)",
        &node_info.ticket[..20.min(node_info.ticket.len())]
    );
    Ok(())
}

async fn cmd_start_federation(config: &RunConfig) -> DogfoodResult<()> {
    let mut manager = NodeManager::new();

    let (alice, bob) = cluster::start_federation(&mut manager, config).await?;

    let state = DogfoodState::new_federation(
        alice.pid,
        alice.ticket.clone(),
        alice.endpoint_addr.clone(),
        bob.pid,
        bob.ticket.clone(),
        bob.endpoint_addr.clone(),
        config.vm_ci,
    );
    state::write_state(&config.state_file_path(), &state)?;

    info!("✅ Federation clusters running (alice + bob)");
    Ok(())
}

async fn cmd_stop(config: &RunConfig) -> DogfoodResult<()> {
    info!("🛑 Stopping dogfood cluster...");

    let state = state::read_state(&config.state_file_path())?;
    node::stop_nodes_by_pids(&state.node_pids()).await?;
    state::delete_state(&config.state_file_path())?;

    // Clean up cluster directory
    let _ = tokio::fs::remove_dir_all(&config.cluster_dir).await;

    info!("✅ Cluster stopped and cleaned up");
    Ok(())
}

async fn cmd_status(config: &RunConfig) -> DogfoodResult<()> {
    let state = state::read_state(&config.state_file_path())?;

    for (i, ticket) in state.tickets().iter().enumerate() {
        let label = if state.is_federation {
            if i == 0 { "alice" } else { "bob" }
        } else {
            "node"
        };

        match cluster::check_health(ticket).await {
            Ok(health) => {
                info!(
                    "✅ {label}: status={}, node_id={}, uptime={}s",
                    health.status, health.node_id, health.uptime_seconds
                );
            }
            Err(e) => {
                tracing::warn!("⚠️  {label}: unreachable — {e}");
            }
        }
    }

    Ok(())
}

async fn cmd_push(config: &RunConfig) -> DogfoodResult<()> {
    info!("📦 Pushing source to Forge...");

    let state = state::read_state(&config.state_file_path())?;
    let ticket = state.primary_ticket();

    forge::ensure_repo_exists(ticket, "aspen").await?;
    forge::git_push(config, ticket, "aspen").await?;

    info!("✅ Source pushed to Forge");
    Ok(())
}

async fn cmd_build(config: &RunConfig) -> DogfoodResult<()> {
    info!("🔨 Waiting for CI build...");

    let state = state::read_state(&config.state_file_path())?;
    let ticket = if state.is_federation {
        // In federation mode, build runs on bob
        state.tickets().get(1).unwrap_or(&state.primary_ticket().to_string()).clone()
    } else {
        state.primary_ticket().to_string()
    };

    let run_id = ci::wait_for_pipeline(&ticket, "aspen", config.ci_timeout_secs).await?;

    info!("✅ CI build completed (run_id: {run_id})");
    Ok(())
}

async fn cmd_deploy(config: &RunConfig) -> DogfoodResult<()> {
    info!("🚢 Deploying artifact...");

    let state = state::read_state(&config.state_file_path())?;
    let ticket = state.primary_ticket();

    deploy::trigger_and_wait(ticket).await?;

    info!("✅ Deployment complete");
    Ok(())
}

async fn cmd_verify(config: &RunConfig) -> DogfoodResult<()> {
    info!("🔍 Verifying deployment...");

    let state = state::read_state(&config.state_file_path())?;
    let ticket = state.primary_ticket();

    deploy::verify_deployment(ticket).await?;

    info!("✅ Verification passed");
    Ok(())
}

async fn cmd_full_loop(config: &RunConfig) -> DogfoodResult<()> {
    cmd_build(config).await?;
    cmd_deploy(config).await?;
    cmd_verify(config).await?;
    Ok(())
}

async fn cmd_full(config: &RunConfig) -> DogfoodResult<()> {
    cmd_start(config).await?;

    // Install signal handler so we stop the cluster on ctrl-c even during the pipeline.
    let config_dir = config.cluster_dir.clone();
    let state_path = config.state_file_path();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::warn!("⚠️  Caught SIGINT — stopping cluster...");
        if let Ok(state) = state::read_state(&state_path) {
            let _ = node::stop_nodes_by_pids(&state.node_pids()).await;
        }
        let _ = tokio::fs::remove_dir_all(&config_dir).await;
        std::process::exit(130);
    });

    let result = async {
        cmd_push(config).await?;
        cmd_build(config).await?;
        cmd_deploy(config).await?;
        cmd_verify(config).await?;
        Ok(())
    }
    .await;

    // Always stop the cluster, even on failure
    let stop_result = cmd_stop(config).await;

    // Return the pipeline error if there was one, otherwise the stop error
    result?;
    stop_result
}
