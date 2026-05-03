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

use std::fmt::Write as _;
use std::future::Future;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use aspen_client::AspenClient;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
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
    /// Inspect durable dogfood run receipts.
    Receipts {
        #[command(subcommand)]
        command: ReceiptsCommand,
    },
}

#[derive(Subcommand)]
enum ReceiptsCommand {
    /// List receipts in the configured receipts directory.
    List,
    /// Show one receipt by run id or explicit path.
    Show(ShowReceiptArgs),
    /// Diagnose one receipt and print first-response triage guidance.
    Diagnose(DiagnoseReceiptArgs),
    /// Publish a validated local receipt into the running Aspen cluster KV store.
    Publish(PublishReceiptArgs),
    /// Show one receipt that was published into the running Aspen cluster KV store.
    ClusterShow(ClusterShowReceiptArgs),
}

#[derive(Args)]
struct ShowReceiptArgs {
    /// Run id in the configured receipts directory, or an explicit receipt JSON path.
    run_id_or_path: String,

    /// Emit validated canonical JSON instead of a text summary.
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct DiagnoseReceiptArgs {
    /// Run id in the configured receipts directory, or an explicit receipt JSON path.
    run_id_or_path: String,
}

#[derive(Args)]
struct PublishReceiptArgs {
    /// Run id in the configured receipts directory, or an explicit receipt JSON path.
    run_id_or_path: String,
}

#[derive(Args)]
struct ClusterShowReceiptArgs {
    /// Run id published under dogfood/receipts/<run-id>.json in cluster KV.
    run_id: String,

    /// Emit validated canonical JSON instead of a text summary.
    #[arg(long)]
    json: bool,
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
    tracing_subscriber::fmt().with_env_filter(build_env_filter()).with_writer(std::io::stderr).init();
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
    let ci_timeout_secs = std::env::var("ASPEN_DOGFOOD_CI_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(7200);

    RunConfig {
        cluster_dir: cli.cluster_dir.clone(),
        federation: cli.federation,
        vm_ci: cli.vm_ci,
        aspen_node_bin: std::env::var("ASPEN_NODE_BIN").unwrap_or_else(|_| "aspen-node".to_string()),
        git_remote_aspen_bin: std::env::var("GIT_REMOTE_ASPEN_BIN").unwrap_or_else(|_| "git-remote-aspen".to_string()),
        project_dir: std::env::var("PROJECT_DIR").unwrap_or_else(|_| ".".to_string()),
        nix_cache_gateway_bin: std::env::var("ASPEN_NIX_CACHE_GATEWAY_BIN").ok(),
        ci_timeout_secs,
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
        Command::Receipts { command } => cmd_receipts(config, command).await,
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
        self.receipt_dir_path().join(format!("{run_id}.json"))
    }

    fn receipt_dir_path(&self) -> PathBuf {
        PathBuf::from(format!("{}-receipts", self.cluster_dir.trim_end_matches('/')))
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
    cmd_deploy_run(config, None).await
}

async fn cmd_deploy_run(config: &RunConfig, run_id: Option<&str>) -> DogfoodResult<()> {
    info!("🚢 Deploying artifact...");

    let state = state::read_state(&config.state_file_path())?;
    let ticket = state.primary_ticket();

    deploy::trigger_and_wait(ticket, run_id).await?;

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

async fn cmd_receipts(config: &RunConfig, command: ReceiptsCommand) -> DogfoodResult<()> {
    match command {
        ReceiptsCommand::List => cmd_receipts_list(config),
        ReceiptsCommand::Show(args) => cmd_receipts_show(config, &args),
        ReceiptsCommand::Diagnose(args) => cmd_receipts_diagnose(config, &args),
        ReceiptsCommand::Publish(args) => cmd_receipts_publish(config, &args).await,
        ReceiptsCommand::ClusterShow(args) => cmd_receipts_cluster_show(config, &args).await,
    }
}

fn cmd_receipts_list(config: &RunConfig) -> DogfoodResult<()> {
    let receipt_dir = config.receipt_dir_path();
    let summaries = list_receipt_summaries(&receipt_dir)?;
    if summaries.is_empty() {
        println!("No dogfood receipts found in {}", receipt_dir.display());
        return Ok(());
    }

    println!("RUN ID                         CREATED AT            COMMAND  FINAL      STAGES  PATH");
    for summary in summaries {
        println!(
            "{:<30} {:<21} {:<8} {:<10} {:>2}/{}    {}",
            summary.run_id,
            summary.created_at,
            summary.command,
            summary.final_status,
            summary.succeeded_stages,
            summary.total_stages,
            summary.path.display()
        );
    }
    Ok(())
}

fn cmd_receipts_show(config: &RunConfig, args: &ShowReceiptArgs) -> DogfoodResult<()> {
    let path = resolve_receipt_selector(config, &args.run_id_or_path);
    let receipt = load_receipt_file(&path)?;
    if args.json {
        let bytes = receipt.canonical_json_bytes().map_err(|error| DogfoodError::Receipt {
            operation: "serialize".to_string(),
            reason: error.to_string(),
        })?;
        println!("{}", String::from_utf8_lossy(&bytes));
    } else {
        print_receipt_summary(&receipt, &path);
    }
    Ok(())
}

fn cmd_receipts_diagnose(config: &RunConfig, args: &DiagnoseReceiptArgs) -> DogfoodResult<()> {
    let path = resolve_receipt_selector(config, &args.run_id_or_path);
    let receipt = load_receipt_file(&path)?;
    print!("{}", diagnose_receipt(&receipt, &path));
    Ok(())
}

async fn cmd_receipts_publish(config: &RunConfig, args: &PublishReceiptArgs) -> DogfoodResult<()> {
    let path = resolve_receipt_selector(config, &args.run_id_or_path);
    let receipt = load_receipt_file(&path)?;
    let bytes = receipt.canonical_json_bytes().map_err(|error| DogfoodError::Receipt {
        operation: "serialize".to_string(),
        reason: error.to_string(),
    })?;
    let key = receipt_cluster_key(&receipt.run_id);
    let state = state::read_state(&config.state_file_path())?;
    publish_receipt_to_cluster(state.primary_ticket(), &key, bytes).await?;
    println!("Published dogfood receipt {} to cluster key {}", receipt.run_id, key);
    Ok(())
}

async fn cmd_receipts_cluster_show(config: &RunConfig, args: &ClusterShowReceiptArgs) -> DogfoodResult<()> {
    if args.run_id.contains('/') || args.run_id.contains('\\') || args.run_id.ends_with(".json") {
        return Err(DogfoodError::Receipt {
            operation: "cluster-show".to_string(),
            reason: "cluster-show expects a run id, not a path".to_string(),
        });
    }

    let key = receipt_cluster_key(&args.run_id);
    let state = state::read_state(&config.state_file_path())?;
    let bytes = read_receipt_from_cluster(state.primary_ticket(), &key).await?;
    let receipt =
        receipt::DogfoodRunReceipt::from_canonical_json_bytes(&bytes).map_err(|error| DogfoodError::Receipt {
            operation: "cluster-show".to_string(),
            reason: format!("cluster key {key}: {error}"),
        })?;
    if args.json {
        let bytes = receipt.canonical_json_bytes().map_err(|error| DogfoodError::Receipt {
            operation: "serialize".to_string(),
            reason: error.to_string(),
        })?;
        println!("{}", String::from_utf8_lossy(&bytes));
    } else {
        print_receipt_summary(&receipt, Path::new(&key));
    }
    Ok(())
}

fn receipt_cluster_key(run_id: &str) -> String {
    format!("dogfood/receipts/{run_id}.json")
}

async fn connect_receipt_cluster(ticket: &str) -> DogfoodResult<AspenClient> {
    AspenClient::connect(ticket, Duration::from_secs(30), None)
        .await
        .map_err(|source| DogfoodError::ClientRpc {
            operation: "connect receipt cluster".to_string(),
            target: cluster::ticket_preview(ticket),
            source,
        })
}

async fn publish_receipt_to_cluster(ticket: &str, key: &str, value: Vec<u8>) -> DogfoodResult<()> {
    let client = connect_receipt_cluster(ticket).await?;
    let response = client
        .send(ClientRpcRequest::WriteKey {
            key: key.to_string(),
            value,
        })
        .await;
    client.shutdown().await;
    let response = response.map_err(|source| DogfoodError::ClientRpc {
        operation: "WriteKey receipt".to_string(),
        target: cluster::ticket_preview(ticket),
        source,
    })?;
    interpret_receipt_write_response(response, key)
}

fn interpret_receipt_write_response(response: ClientRpcResponse, key: &str) -> DogfoodResult<()> {
    match response {
        ClientRpcResponse::WriteResult(result) if result.is_success => Ok(()),
        ClientRpcResponse::WriteResult(result) => Err(DogfoodError::Receipt {
            operation: "publish".to_string(),
            reason: format!("cluster key {key}: {}", result.error.unwrap_or_else(|| "write failed".to_string())),
        }),
        ClientRpcResponse::Error(error) => Err(DogfoodError::Receipt {
            operation: "publish".to_string(),
            reason: format!("cluster key {key}: {}: {}", error.code, error.message),
        }),
        other => Err(DogfoodError::Receipt {
            operation: "publish".to_string(),
            reason: format!("cluster key {key}: unexpected response {other:?}"),
        }),
    }
}

async fn read_receipt_from_cluster(ticket: &str, key: &str) -> DogfoodResult<Vec<u8>> {
    let client = connect_receipt_cluster(ticket).await?;
    let response = client.send(ClientRpcRequest::ReadKey { key: key.to_string() }).await;
    client.shutdown().await;
    let response = response.map_err(|source| DogfoodError::ClientRpc {
        operation: "ReadKey receipt".to_string(),
        target: cluster::ticket_preview(ticket),
        source,
    })?;
    interpret_receipt_read_response(response, key)
}

fn interpret_receipt_read_response(response: ClientRpcResponse, key: &str) -> DogfoodResult<Vec<u8>> {
    match response {
        ClientRpcResponse::ReadResult(result) if result.was_found => {
            result.value.ok_or_else(|| DogfoodError::Receipt {
                operation: "cluster-show".to_string(),
                reason: format!("cluster key {key}: read response had no value"),
            })
        }
        ClientRpcResponse::ReadResult(result) => Err(DogfoodError::Receipt {
            operation: "cluster-show".to_string(),
            reason: format!("cluster key {key}: {}", result.error.unwrap_or_else(|| "not found".to_string())),
        }),
        ClientRpcResponse::Error(error) => Err(DogfoodError::Receipt {
            operation: "cluster-show".to_string(),
            reason: format!("cluster key {key}: {}: {}", error.code, error.message),
        }),
        other => Err(DogfoodError::Receipt {
            operation: "cluster-show".to_string(),
            reason: format!("cluster key {key}: unexpected response {other:?}"),
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReceiptSummary {
    run_id: String,
    created_at: String,
    command: String,
    final_status: String,
    succeeded_stages: usize,
    total_stages: usize,
    path: PathBuf,
}

fn list_receipt_summaries(receipt_dir: &Path) -> DogfoodResult<Vec<ReceiptSummary>> {
    if !receipt_dir.exists() {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(receipt_dir).map_err(|source| DogfoodError::Receipt {
        operation: "list".to_string(),
        reason: format!("reading {}: {source}", receipt_dir.display()),
    })?;

    let mut summaries = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| DogfoodError::Receipt {
            operation: "list".to_string(),
            reason: format!("reading {} entry: {source}", receipt_dir.display()),
        })?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        match load_receipt_file(&path).map(|receipt| summarize_receipt(receipt, path.clone())) {
            Ok(summary) => summaries.push(summary),
            Err(error) => eprintln!("warning: skipping invalid dogfood receipt {}: {error}", path.display()),
        }
    }
    summaries
        .sort_by(|left, right| left.created_at.cmp(&right.created_at).then_with(|| left.run_id.cmp(&right.run_id)));
    Ok(summaries)
}

fn summarize_receipt(receipt: receipt::DogfoodRunReceipt, path: PathBuf) -> ReceiptSummary {
    let total_stages = receipt.stages.len();
    let succeeded_stages =
        receipt.stages.iter().filter(|stage| stage.status == receipt::DogfoodStageStatus::Succeeded).count();
    let final_status = aggregate_receipt_status(&receipt.stages).to_string();

    ReceiptSummary {
        run_id: receipt.run_id,
        created_at: receipt.created_at,
        command: receipt.command,
        final_status,
        succeeded_stages,
        total_stages,
        path,
    }
}

fn aggregate_receipt_status(stages: &[receipt::DogfoodStageReceipt]) -> &'static str {
    if stages.is_empty() {
        return "empty";
    }
    if stages.iter().any(|stage| stage.status == receipt::DogfoodStageStatus::Failed) {
        return "failed";
    }
    if stages.iter().any(|stage| stage.status == receipt::DogfoodStageStatus::Running) {
        return "running";
    }
    if stages.iter().any(|stage| stage.status == receipt::DogfoodStageStatus::Pending) {
        return "pending";
    }
    if stages.iter().any(|stage| stage.status == receipt::DogfoodStageStatus::Skipped) {
        return "skipped";
    }
    "succeeded"
}

fn resolve_receipt_selector(config: &RunConfig, selector: &str) -> PathBuf {
    let path = PathBuf::from(selector);
    if path.is_absolute() || path.components().count() > 1 || path.extension().is_some() {
        path
    } else {
        config.receipt_file_path(selector)
    }
}

fn load_receipt_file(path: &Path) -> DogfoodResult<receipt::DogfoodRunReceipt> {
    let bytes = std::fs::read(path).map_err(|source| DogfoodError::Receipt {
        operation: "read".to_string(),
        reason: format!("{}: {source}", path.display()),
    })?;
    receipt::DogfoodRunReceipt::from_canonical_json_bytes(&bytes).map_err(|error| DogfoodError::Receipt {
        operation: "parse".to_string(),
        reason: format!("{}: {error}", path.display()),
    })
}

fn print_receipt_summary(receipt: &receipt::DogfoodRunReceipt, path: &Path) {
    println!("Dogfood receipt: {}", receipt.run_id);
    println!("  schema: {}", receipt.schema);
    println!("  command: {}", receipt.command);
    println!("  created_at: {}", receipt.created_at);
    println!("  mode: federation={}, vm_ci={}", receipt.mode.federation, receipt.mode.vm_ci);
    println!("  project_dir: {}", receipt.project_dir);
    println!("  cluster_dir: {}", receipt.cluster_dir);
    println!("  path: {}", path.display());
    println!("  stages:");
    for stage in &receipt.stages {
        let finished_at = stage.finished_at.as_deref().unwrap_or("-");
        println!("    - {}: {} ({} → {})", stage.stage.as_str(), stage.status.as_str(), stage.started_at, finished_at);
        for artifact in &stage.artifacts {
            println!(
                "        artifact {} [{}] store_id={} blob_id={} digest={} size={} path={}",
                artifact.name,
                artifact.kind.as_str(),
                artifact.store_id.as_deref().unwrap_or("-"),
                artifact.blob_id.as_deref().unwrap_or("-"),
                artifact.digest.as_deref().unwrap_or("-"),
                artifact.size_bytes.map(|size| size.to_string()).unwrap_or_else(|| "-".to_string()),
                artifact.relative_path.as_deref().unwrap_or("-")
            );
        }
        if let Some(failure) = &stage.failure {
            println!("        failure {} [{}]: {}", failure.operation, failure.category, failure.message);
        }
    }
}

fn diagnose_receipt(receipt: &receipt::DogfoodRunReceipt, path: &Path) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "Dogfood receipt diagnosis: {}", receipt.run_id);
    let _ = writeln!(output, "  path: {}", path.display());
    let _ = writeln!(output, "  command: {}", receipt.command);
    let _ = writeln!(output, "  created_at: {}", receipt.created_at);

    let Some(stage) = receipt.stages.iter().find(|stage| stage.status == receipt::DogfoodStageStatus::Failed) else {
        let succeeded =
            receipt.stages.iter().filter(|stage| stage.status == receipt::DogfoodStageStatus::Succeeded).count();
        let _ = writeln!(
            output,
            "  status: no failed stage found ({}/{}) stages succeeded",
            succeeded,
            receipt.stages.len()
        );
        let _ = writeln!(
            output,
            "  next: use `receipts show {}` for archival details or rerun `full` for fresh acceptance evidence",
            receipt.run_id
        );
        return output;
    };

    let _ = writeln!(output, "  status: failed");
    let _ = writeln!(output, "  failed_stage: {}", stage.stage.as_str());
    if let Some(failure) = &stage.failure {
        let _ = writeln!(output, "  failure_category: {}", failure.category);
        let _ = writeln!(output, "  failure_operation: {}", failure.operation);
        let _ = writeln!(output, "  failure_message: {}", failure.message);
        let _ = writeln!(output, "  first_checks:");
        for check in dogfood_diagnosis_checks(stage.stage, &failure.category) {
            let _ = writeln!(output, "    - {check}");
        }
    } else {
        let _ = writeln!(output, "  failure_category: missing_failure_summary");
        let _ = writeln!(output, "  first_checks:");
        let _ = writeln!(
            output,
            "    - Treat the receipt as structurally suspicious and inspect it with `receipts show {}`",
            receipt.run_id
        );
    }
    output
}

fn dogfood_diagnosis_checks(stage: receipt::DogfoodStageKind, category: &str) -> &'static [&'static str] {
    match (stage, category) {
        (receipt::DogfoodStageKind::Start, "process_spawn") => &[
            "Check that the dogfood aspen-node binary path exists and is executable.",
            "Inspect node logs under the cluster directory; do not copy or preserve cluster-ticket.txt.",
        ],
        (receipt::DogfoodStageKind::Start, "node_crash") => &[
            "Inspect node stderr/logs under the cluster directory for the crash cause.",
            "Verify local ports, data directory ownership, and feature flags before rerunning.",
        ],
        (receipt::DogfoodStageKind::Start, "health_check") => &[
            "Inspect node logs and confirm the cluster can elect a healthy node.",
            "Check for startup retries before changing source code.",
        ],
        (receipt::DogfoodStageKind::Push, "git_push") => &[
            "Read the captured git stdout/stderr in the receipt failure message.",
            "Check Forge reachability and whether the pushed tree exceeded protocol message limits.",
        ],
        (receipt::DogfoodStageKind::Push, "forge" | "client_rpc") => &[
            "Check Forge RPC reachability through the cluster ticket without preserving ticket contents.",
            "Inspect git-remote-aspen diagnostics for protocol or batching errors.",
        ],
        (receipt::DogfoodStageKind::Build, "ci_pipeline") => &[
            "Use the CI run artifact from the receipt, then inspect CI status and logs for that exact run id.",
            "Check failed job output before rerunning or changing source.",
        ],
        (receipt::DogfoodStageKind::Build, "timeout") => &[
            "Use CI status/logs to distinguish a still-running local Nix build from a stuck worker.",
            "If the build is still making progress, consider increasing ASPEN_DOGFOOD_CI_TIMEOUT_SECS before changing source.",
        ],
        (receipt::DogfoodStageKind::Deploy, "deploy_failed" | "client_rpc" | "timeout") => &[
            "Check deployment status and confirm the receipt's CI run artifact is the one being deployed.",
            "Confirm quorum is intact and the built store path or artifact is available to the target node.",
        ],
        (receipt::DogfoodStageKind::Verify, "health_check" | "client_rpc") => &[
            "Treat this as a post-deploy health failure and inspect cluster health plus recent node logs.",
            "Check node uptime and deployment status before rerunning full dogfood.",
        ],
        (receipt::DogfoodStageKind::Stop, "stop_node" | "state_file") => &[
            "Inspect local process state and cluster directory ownership.",
            "Do not manually delete cluster files until process cleanup state is understood.",
        ],
        _ => &[
            "Use the failure message as the primary clue and preserve the JSON receipt as evidence.",
            "Run `receipts show <run-id>` for full stage and artifact context.",
        ],
    }
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
    let build_run_id = recorder
        .run_stage_with_artifacts(receipt::DogfoodStageKind::Build, || async {
            let run_id = build_ci_pipeline(config).await?;
            Ok((run_id.clone(), vec![ci_run_artifact(run_id)]))
        })
        .await?;
    recorder
        .run_stage(receipt::DogfoodStageKind::Deploy, || cmd_deploy_run(config, Some(&build_run_id)))
        .await?;
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
            Ok(((), Vec::new()))
        })
        .await
    }

    async fn run_stage_with_artifacts<F, Fut, T>(
        &mut self,
        stage: receipt::DogfoodStageKind,
        operation: F,
    ) -> DogfoodResult<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = DogfoodResult<(T, Vec<receipt::DogfoodArtifactReceipt>)>>,
    {
        let started_at = current_timestamp_utc();
        match operation().await {
            Ok((value, artifacts)) => {
                self.receipt.stages.push(receipt::DogfoodStageReceipt {
                    stage,
                    status: receipt::DogfoodStageStatus::Succeeded,
                    started_at,
                    finished_at: Some(current_timestamp_utc()),
                    failure: None,
                    artifacts,
                });
                self.write()?;
                Ok(value)
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

    fn sample_receipt(
        run_id: &str,
        created_at: &str,
        status: receipt::DogfoodStageStatus,
    ) -> receipt::DogfoodRunReceipt {
        receipt::DogfoodRunReceipt::new(receipt::DogfoodRunReceiptInit {
            run_id: run_id.to_string(),
            command: "full".to_string(),
            created_at: created_at.to_string(),
            mode: receipt::DogfoodRunMode {
                federation: false,
                vm_ci: false,
            },
            project_dir: "/repo".to_string(),
            cluster_dir: "/tmp/test-dogfood".to_string(),
            stages: vec![receipt::DogfoodStageReceipt {
                stage: receipt::DogfoodStageKind::Build,
                status,
                started_at: "2026-05-03T01:00:00Z".to_string(),
                finished_at: Some("2026-05-03T01:01:00Z".to_string()),
                failure: if status == receipt::DogfoodStageStatus::Failed {
                    Some(receipt::DogfoodFailureSummary {
                        operation: "Build".to_string(),
                        category: "ci_pipeline".to_string(),
                        message: "failed".to_string(),
                    })
                } else {
                    None
                },
                artifacts: vec![ci_run_artifact("run-123".to_string())],
            }],
        })
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
        assert_eq!(config.receipt_dir_path(), PathBuf::from("/tmp/test-dogfood-receipts"));
    }

    #[test]
    fn list_receipt_summaries_reads_valid_receipts_sorted_by_created_at() {
        let tempdir = tempfile::tempdir().unwrap();
        let first = sample_receipt("dogfood-2", "2026-05-03T02:00:00Z", receipt::DogfoodStageStatus::Succeeded);
        let second = sample_receipt("dogfood-1", "2026-05-03T01:00:00Z", receipt::DogfoodStageStatus::Failed);
        first.write_canonical_json_file(&tempdir.path().join("dogfood-2.json")).unwrap();
        second.write_canonical_json_file(&tempdir.path().join("dogfood-1.json")).unwrap();
        std::fs::write(tempdir.path().join("not-a-receipt.json"), b"{}").unwrap();
        std::fs::write(tempdir.path().join("notes.txt"), b"ignore me").unwrap();

        let summaries = list_receipt_summaries(tempdir.path()).unwrap();

        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].run_id, "dogfood-1");
        assert_eq!(summaries[0].final_status, "failed");
        assert_eq!(summaries[0].succeeded_stages, 0);
        assert_eq!(summaries[1].run_id, "dogfood-2");
        assert_eq!(summaries[1].final_status, "succeeded");
        assert_eq!(summaries[1].succeeded_stages, 1);
    }

    #[test]
    fn list_receipt_summary_failed_stage_overrides_successful_stop() {
        let mut receipt =
            sample_receipt("dogfood-failed-clean", "2026-05-03T03:00:00Z", receipt::DogfoodStageStatus::Failed);
        receipt.stages.push(receipt::DogfoodStageReceipt {
            stage: receipt::DogfoodStageKind::Stop,
            status: receipt::DogfoodStageStatus::Succeeded,
            started_at: "2026-05-03T01:02:00Z".to_string(),
            finished_at: Some("2026-05-03T01:03:00Z".to_string()),
            failure: None,
            artifacts: Vec::new(),
        });

        let summary = summarize_receipt(receipt, PathBuf::from("/tmp/dogfood-failed-clean.json"));

        assert_eq!(summary.final_status, "failed");
        assert_eq!(summary.succeeded_stages, 1);
        assert_eq!(summary.total_stages, 2);
    }

    #[test]
    fn aggregate_receipt_status_uses_operator_precedence() {
        assert_eq!(aggregate_receipt_status(&[]), "empty");
        let mut receipt =
            sample_receipt("dogfood-running", "2026-05-03T04:00:00Z", receipt::DogfoodStageStatus::Succeeded);
        receipt.stages.push(receipt::DogfoodStageReceipt {
            stage: receipt::DogfoodStageKind::Deploy,
            status: receipt::DogfoodStageStatus::Running,
            started_at: "2026-05-03T01:02:00Z".to_string(),
            finished_at: None,
            failure: None,
            artifacts: Vec::new(),
        });

        assert_eq!(aggregate_receipt_status(&receipt.stages), "running");
    }

    #[test]
    fn receipt_cluster_key_is_deterministic() {
        assert_eq!(receipt_cluster_key("dogfood-20260503T180335Z"), "dogfood/receipts/dogfood-20260503T180335Z.json");
    }

    #[test]
    fn interpret_receipt_write_response_accepts_success_only() {
        let ok = ClientRpcResponse::WriteResult(aspen_client_api::WriteResultResponse {
            is_success: true,
            error: None,
        });
        assert!(interpret_receipt_write_response(ok, "dogfood/receipts/run.json").is_ok());

        let failed = ClientRpcResponse::WriteResult(aspen_client_api::WriteResultResponse {
            is_success: false,
            error: Some("raft unavailable".to_string()),
        });
        let error = interpret_receipt_write_response(failed, "dogfood/receipts/run.json").unwrap_err();
        assert!(error.to_string().contains("raft unavailable"));
    }

    #[test]
    fn interpret_receipt_read_response_returns_value_or_not_found() {
        let found = ClientRpcResponse::ReadResult(aspen_client_api::ReadResultResponse {
            value: Some(b"receipt".to_vec()),
            was_found: true,
            error: None,
        });
        assert_eq!(interpret_receipt_read_response(found, "dogfood/receipts/run.json").unwrap(), b"receipt".to_vec());

        let missing = ClientRpcResponse::ReadResult(aspen_client_api::ReadResultResponse {
            value: None,
            was_found: false,
            error: None,
        });
        let error = interpret_receipt_read_response(missing, "dogfood/receipts/run.json").unwrap_err();
        assert!(error.to_string().contains("not found"));
    }

    #[test]
    fn list_receipt_summaries_missing_directory_is_empty() {
        let tempdir = tempfile::tempdir().unwrap();
        let missing = tempdir.path().join("missing");

        let summaries = list_receipt_summaries(&missing).unwrap();

        assert!(summaries.is_empty());
    }

    #[test]
    fn resolve_receipt_selector_uses_run_id_or_explicit_path() {
        let config = test_config(false);

        assert_eq!(
            resolve_receipt_selector(&config, "dogfood-run"),
            PathBuf::from("/tmp/test-dogfood-receipts/dogfood-run.json")
        );
        assert_eq!(resolve_receipt_selector(&config, "./receipt.json"), PathBuf::from("./receipt.json"));
        assert_eq!(resolve_receipt_selector(&config, "/tmp/receipt.json"), PathBuf::from("/tmp/receipt.json"));
    }

    #[test]
    fn load_receipt_file_rejects_invalid_json() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("bad.json");
        std::fs::write(&path, b"{}").unwrap();

        let error = load_receipt_file(&path).unwrap_err();

        assert!(error.to_string().contains("receipt parse"));
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
    fn diagnose_receipt_reports_success_without_live_cluster() {
        let receipt = sample_receipt("dogfood-ok", "2026-05-03T02:00:00Z", receipt::DogfoodStageStatus::Succeeded);

        let output = diagnose_receipt(&receipt, Path::new("/tmp/dogfood-ok.json"));

        assert!(output.contains("Dogfood receipt diagnosis: dogfood-ok"));
        assert!(output.contains("status: no failed stage found (1/1) stages succeeded"));
        assert!(output.contains("receipts show dogfood-ok"));
    }

    #[test]
    fn diagnose_receipt_reports_failed_stage_and_checks() {
        let receipt = sample_receipt("dogfood-failed", "2026-05-03T01:00:00Z", receipt::DogfoodStageStatus::Failed);

        let output = diagnose_receipt(&receipt, Path::new("/tmp/dogfood-failed.json"));

        assert!(output.contains("status: failed"));
        assert!(output.contains("failed_stage: build"));
        assert!(output.contains("failure_category: ci_pipeline"));
        assert!(output.contains("Use the CI run artifact from the receipt"));
    }

    #[test]
    fn diagnosis_checks_fall_back_for_unknown_category() {
        let checks = dogfood_diagnosis_checks(receipt::DogfoodStageKind::Build, "unexpected");

        assert!(checks[0].contains("failure message"));
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
