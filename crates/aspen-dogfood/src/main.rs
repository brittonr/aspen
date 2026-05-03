//! Dogfood orchestrator for self-hosted Aspen build pipeline.
//!
//! Replaces the shell scripts (`dogfood-local.sh`, `dogfood-local-vmci.sh`,
//! `dogfood-federation.sh`) with a typed Rust binary that uses `aspen-client`
//! directly over Iroh instead of shelling out to `aspen-cli`.

mod ci;
mod cluster;
mod deploy;
mod error;
mod federation;
mod forge;
mod node;
mod receipt;
mod state;

use std::future::Future;
use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use tracing::info;

use crate::error::DogfoodError;
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

fn main() {
    if let Err(error) = run_main() {
        tracing::error!("❌ {error}");
        std::process::exit(1);
    }
}

fn run_main() -> DogfoodResult<()> {
    // Note: ASPEN_RELAY_DISABLED is set on spawned node processes only (in spawn_node),
    // NOT globally. The client endpoint needs relay enabled for QUIC signaling even
    // when connecting to local nodes.
    tracing_subscriber::fmt().with_env_filter(build_env_filter()).init();
    let cli = Cli::parse();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| std::io::Error::new(error.kind(), format!("failed to build tokio runtime: {error}")))
        .map_err(|source| crate::error::DogfoodError::ProcessSpawn {
            binary: "tokio-runtime".to_string(),
            source,
        })?;
    runtime.block_on(run(cli))
}

fn build_env_filter() -> tracing_subscriber::EnvFilter {
    tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
}

fn build_run_config(cli: &Cli) -> RunConfig {
    RunConfig {
        cluster_dir: cli.cluster_dir.clone(),
        federation: cli.federation,
        vm_ci: cli.vm_ci,
        aspen_node_bin: std::env::var("ASPEN_NODE_BIN").unwrap_or_else(|_| "aspen-node".to_string()),
        git_remote_aspen_bin: std::env::var("GIT_REMOTE_ASPEN_BIN").unwrap_or_else(|_| "git-remote-aspen".to_string()),
        project_dir: std::env::var("PROJECT_DIR").unwrap_or_else(|_| ".".to_string()),
        nix_cache_gateway_bin: std::env::var("ASPEN_NIX_CACHE_GATEWAY_BIN").ok(),
        ci_timeout_secs: 600,
    }
}

async fn run(cli: Cli) -> DogfoodResult<()> {
    let config = build_run_config(&cli);
    dispatch_command(&config, cli.command).await
}

async fn dispatch_command(config: &RunConfig, command: Command) -> DogfoodResult<()> {
    match command {
        Command::Start => cmd_start(config).await,
        Command::Stop => cmd_stop(config).await,
        Command::Status => cmd_status(config).await,
        Command::Push => cmd_push(config).await,
        Command::Build => cmd_build(config).await,
        Command::Deploy => cmd_deploy(config).await,
        Command::Verify => cmd_verify(config).await,
        Command::FullLoop => cmd_full_loop(config).await,
        Command::Full => cmd_full(config).await,
    }
}

/// Runtime configuration derived from CLI args and environment.
pub struct RunConfig {
    pub cluster_dir: String,
    pub federation: bool,
    pub vm_ci: bool,
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

    fn receipt_file_path(&self, run_id: &str) -> PathBuf {
        PathBuf::from(format!("{}-receipts/{run_id}.json", self.cluster_dir.trim_end_matches('/')))
    }

    /// Cookie for the primary (or only) cluster.
    pub fn cookie(&self) -> String {
        format!("dogfood-{}", current_cookie_date())
    }

    /// Cookie for alice's cluster in federation mode.
    pub fn alice_cookie(&self) -> String {
        format!("fed-alice-{}", current_cookie_date())
    }

    /// Cookie for bob's cluster in federation mode.
    pub fn bob_cookie(&self) -> String {
        format!("fed-bob-{}", current_cookie_date())
    }
}

#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "dogfood cookie prefixes intentionally encode local run date"
)]
fn current_cookie_date() -> String {
    chrono::Local::now().format("%Y%m%d").to_string()
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

    if let Err(error) = tokio::fs::remove_dir_all(&config.cluster_dir).await
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(cluster_dir = %config.cluster_dir, "failed to clean up cluster directory: {error}");
    }

    info!("✅ Cluster stopped and cleaned up");
    Ok(())
}

async fn cmd_status(config: &RunConfig) -> DogfoodResult<()> {
    let state = state::read_state(&config.state_file_path())?;

    for (index, ticket) in state.tickets().iter().enumerate() {
        log_node_status(NodeStatusTarget {
            ticket,
            label: status_label(state.is_federation, index),
        })
        .await;
    }

    Ok(())
}

fn status_label(is_federation: bool, index: usize) -> &'static str {
    if !is_federation {
        return "node";
    }
    if index == 0 { "alice" } else { "bob" }
}

struct NodeStatusTarget<'a> {
    ticket: &'a str,
    label: &'a str,
}

async fn log_node_status(target: NodeStatusTarget<'_>) {
    match cluster::check_health(target.ticket).await {
        Ok(health) => {
            info!(
                "✅ {}: status={}, node_id={}, uptime={}s",
                target.label, health.status, health.node_id, health.uptime_seconds
            );
        }
        Err(error) => {
            tracing::warn!("⚠️  {}: unreachable — {error}", target.label);
        }
    }
}

async fn cmd_push(config: &RunConfig) -> DogfoodResult<()> {
    info!("📦 Pushing source to Forge...");

    let state = state::read_state(&config.state_file_path())?;
    let ticket = state.primary_ticket();

    let repo_id = forge::ensure_repo_exists(ticket, "aspen").await?;
    forge::watch_repo(ticket, &repo_id).await?;
    forge::git_push(config, ticket, &repo_id).await?;

    info!("✅ Source pushed to Forge");
    Ok(())
}

async fn cmd_build(config: &RunConfig) -> DogfoodResult<()> {
    build_ci_pipeline(config).await.map(|_run_id| ())
}

async fn build_ci_pipeline(config: &RunConfig) -> DogfoodResult<String> {
    info!("🔨 Waiting for CI build...");

    let state = state::read_state(&config.state_file_path())?;
    let (ticket, repo_name) = build_wait_pipeline_parts(config, &state).await?;
    let run_id = ci::wait_for_pipeline(
        ci::WaitPipelineTarget {
            ticket: &ticket,
            repo_name: &repo_name,
        },
        config.ci_timeout_secs,
    )
    .await?;

    info!("✅ CI build completed (run_id: {run_id})");
    Ok(run_id)
}

async fn build_wait_pipeline_parts(config: &RunConfig, state: &DogfoodState) -> DogfoodResult<(String, String)> {
    if state.is_federation {
        let repo_name = federation::prepare_build(config, state).await?;
        return Ok((state.bob_ticket().to_string(), repo_name));
    }
    Ok((state.primary_ticket().to_string(), "aspen".to_string()))
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
    let mut recorder = DogfoodReceiptRecorder::new(config, "full")?;
    if let Err(error) = recorder.run_stage(receipt::DogfoodStageKind::Start, || cmd_start(config)).await {
        recorder.log_path();
        return Err(error);
    }
    install_ctrl_c_shutdown(config);
    let result = run_full_pipeline_with_receipts(config, &mut recorder).await;
    let stop_result = recorder.run_stage(receipt::DogfoodStageKind::Stop, || cmd_stop(config)).await;
    recorder.log_path();
    result?;
    stop_result
}

fn install_ctrl_c_shutdown(config: &RunConfig) {
    let config_dir = config.cluster_dir.clone();
    let state_path = config.state_file_path();
    tokio::spawn(async move {
        handle_ctrl_c_shutdown(config_dir, state_path).await;
    });
}

async fn handle_ctrl_c_shutdown(config_dir: String, state_path: String) {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::warn!("failed to wait for ctrl-c: {error}");
        return;
    }

    tracing::warn!("⚠️  Caught SIGINT — stopping cluster...");
    if let Ok(state) = state::read_state(&state_path)
        && let Err(error) = node::stop_nodes_by_pids(&state.node_pids()).await
    {
        tracing::warn!("failed to stop dogfood nodes after ctrl-c: {error}");
    }
    if let Err(error) = tokio::fs::remove_dir_all(&config_dir).await
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!("failed to clean dogfood directory after ctrl-c: {error}");
    }
    std::process::exit(130);
}

async fn run_full_pipeline_with_receipts(
    config: &RunConfig,
    recorder: &mut DogfoodReceiptRecorder,
) -> DogfoodResult<()> {
    recorder.run_stage(receipt::DogfoodStageKind::Push, || cmd_push(config)).await?;
    recorder
        .run_stage_with_artifacts(receipt::DogfoodStageKind::Build, || async {
            let run_id = build_ci_pipeline(config).await?;
            Ok(vec![ci_run_artifact(run_id)])
        })
        .await?;
    recorder.run_stage(receipt::DogfoodStageKind::Deploy, || cmd_deploy(config)).await?;
    recorder.run_stage(receipt::DogfoodStageKind::Verify, || cmd_verify(config)).await?;
    Ok(())
}

struct DogfoodReceiptRecorder {
    receipt: receipt::DogfoodRunReceipt,
    path: PathBuf,
}

impl DogfoodReceiptRecorder {
    fn new(config: &RunConfig, command: &str) -> DogfoodResult<Self> {
        let run_id = dogfood_run_id();
        let path = config.receipt_file_path(&run_id);
        let receipt = receipt::DogfoodRunReceipt::new(receipt::DogfoodRunReceiptInit {
            run_id,
            command: command.to_string(),
            created_at: current_timestamp_utc(),
            mode: receipt::DogfoodRunMode {
                federation: config.federation,
                vm_ci: config.vm_ci,
            },
            project_dir: config.project_dir.clone(),
            cluster_dir: config.cluster_dir.clone(),
            stages: Vec::new(),
        });
        let recorder = Self { receipt, path };
        recorder.write()?;
        Ok(recorder)
    }

    async fn run_stage<F, Fut>(&mut self, stage: receipt::DogfoodStageKind, operation: F) -> DogfoodResult<()>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = DogfoodResult<()>>,
    {
        self.run_stage_with_artifacts(stage, || async {
            operation().await?;
            Ok(Vec::new())
        })
        .await
    }

    async fn run_stage_with_artifacts<F, Fut>(
        &mut self,
        stage: receipt::DogfoodStageKind,
        operation: F,
    ) -> DogfoodResult<()>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = DogfoodResult<Vec<receipt::DogfoodArtifactReceipt>>>,
    {
        let started_at = current_timestamp_utc();
        match operation().await {
            Ok(artifacts) => {
                self.receipt.stages.push(receipt::DogfoodStageReceipt {
                    stage,
                    status: receipt::DogfoodStageStatus::Succeeded,
                    started_at,
                    finished_at: Some(current_timestamp_utc()),
                    failure: None,
                    artifacts,
                });
                self.write()
            }
            Err(error) => {
                let failure = dogfood_failure_summary(stage, &error);
                self.receipt.stages.push(receipt::DogfoodStageReceipt {
                    stage,
                    status: receipt::DogfoodStageStatus::Failed,
                    started_at,
                    finished_at: Some(current_timestamp_utc()),
                    failure: Some(failure),
                    artifacts: Vec::new(),
                });
                if let Err(write_error) = self.write() {
                    tracing::warn!("failed to write dogfood receipt after stage failure: {write_error}");
                }
                Err(error)
            }
        }
    }

    fn write(&self) -> DogfoodResult<()> {
        self.receipt.write_canonical_json_file(&self.path).map_err(|error| DogfoodError::Receipt {
            operation: "write".to_string(),
            reason: error.to_string(),
        })
    }

    fn log_path(&self) {
        info!("🧾 Dogfood receipt: {}", self.path.display());
    }
}

fn dogfood_failure_summary(stage: receipt::DogfoodStageKind, error: &DogfoodError) -> receipt::DogfoodFailureSummary {
    receipt::DogfoodFailureSummary {
        operation: format!("{stage:?}"),
        category: dogfood_error_category(error).to_string(),
        message: error.to_string(),
    }
}

fn ci_run_artifact(run_id: String) -> receipt::DogfoodArtifactReceipt {
    receipt::DogfoodArtifactReceipt {
        name: "ci-run".to_string(),
        kind: receipt::DogfoodArtifactKind::CiRun,
        store_id: Some(run_id),
        blob_id: None,
        digest: None,
        size_bytes: None,
        relative_path: None,
    }
}

fn dogfood_error_category(error: &DogfoodError) -> &'static str {
    match error {
        DogfoodError::ClientRpc { .. } => "client_rpc",
        DogfoodError::ProcessSpawn { .. } => "process_spawn",
        DogfoodError::NodeCrash { .. } => "node_crash",
        DogfoodError::StateFile { .. } => "state_file",
        DogfoodError::StateDeserialize { .. } => "state_deserialize",
        DogfoodError::StateSerialize { .. } => "state_serialize",
        DogfoodError::Timeout { .. } => "timeout",
        DogfoodError::HealthCheck { .. } => "health_check",
        DogfoodError::CiPipeline { .. } => "ci_pipeline",
        DogfoodError::DeployFailed { .. } => "deploy_failed",
        DogfoodError::Forge { .. } => "forge",
        DogfoodError::Receipt { .. } => "receipt",
        DogfoodError::Federation { .. } => "federation",
        DogfoodError::GitPush { .. } => "git_push",
        DogfoodError::NoCluster => "no_cluster",
        DogfoodError::StopNode { .. } => "stop_node",
    }
}

fn dogfood_run_id() -> String {
    format!("dogfood-{}", current_timestamp_utc().replace([':', '-'], ""))
}

#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "dogfood receipts intentionally record wall-clock run timestamps"
)]
fn current_timestamp_utc() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(federation: bool) -> RunConfig {
        RunConfig {
            cluster_dir: "/tmp/test-dogfood".to_string(),
            federation,
            vm_ci: false,
            aspen_node_bin: "aspen-node".to_string(),
            git_remote_aspen_bin: "git-remote-aspen".to_string(),
            project_dir: ".".to_string(),
            nix_cache_gateway_bin: None,
            ci_timeout_secs: 60,
        }
    }

    #[test]
    fn cookies_are_distinct_across_modes() {
        let config = test_config(true);
        let single = config.cookie();
        let alice = config.alice_cookie();
        let bob = config.bob_cookie();

        // All three must be non-empty and mutually distinct.
        assert!(!single.is_empty());
        assert!(!alice.is_empty());
        assert!(!bob.is_empty());
        assert_ne!(alice, bob, "alice and bob must get different cookies");
        assert_ne!(single, alice);
        assert_ne!(single, bob);
    }

    #[test]
    fn cookie_contains_date() {
        let config = test_config(false);
        let date = chrono::Local::now().format("%Y%m%d").to_string();
        assert!(config.cookie().contains(&date));
        assert!(config.alice_cookie().contains(&date));
        assert!(config.bob_cookie().contains(&date));
    }

    #[test]
    fn state_file_path_uses_cluster_dir() {
        let config = test_config(false);
        assert_eq!(config.state_file_path(), "/tmp/test-dogfood/dogfood-state.json");
    }

    #[test]
    fn receipt_file_path_uses_sibling_receipts_dir() {
        let config = test_config(false);
        assert_eq!(config.receipt_file_path("run-1"), PathBuf::from("/tmp/test-dogfood-receipts/run-1.json"));
    }

    #[test]
    fn dogfood_error_category_is_stable_for_receipts() {
        let error = DogfoodError::Timeout {
            operation: "CI pipeline abc123".to_string(),
            timeout_secs: 600,
        };
        assert_eq!(dogfood_error_category(&error), "timeout");
    }

    #[test]
    fn ci_run_artifact_records_pipeline_run_id() {
        let artifact = ci_run_artifact("run-123".to_string());
        assert_eq!(artifact.name, "ci-run");
        assert_eq!(artifact.kind, receipt::DogfoodArtifactKind::CiRun);
        assert_eq!(artifact.store_id.as_deref(), Some("run-123"));
    }

    #[test]
    fn node_count_flag_rejected() {
        // --node-count was removed. clap must reject it as an unknown flag.
        use clap::Parser;
        let result = Cli::try_parse_from(["aspen-dogfood", "--node-count", "3", "start"]);
        assert!(result.is_err(), "--node-count should be rejected as unknown flag");
    }
}
