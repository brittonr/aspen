//! CI/CD pipeline commands.
//!
//! Commands for triggering and monitoring CI pipelines with
//! Nickel-based configuration and distributed job execution.

use anyhow::Result;
use aspen_ci_core::log_writer::CiLogChunk;
use aspen_client::watch::WatchEvent;
use aspen_client::watch::WatchSession;
use aspen_client_api::CI_STATUS_SUCCESS;
use aspen_client_api::CI_TERMINAL_STATUS_LABELS;
use aspen_client_api::CiRunReceipt;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_core::CI_LOG_COMPLETE_MARKER;
use aspen_core::CI_LOG_KV_PREFIX;
use clap::Args;
use clap::Subcommand;
use serde_json::json;
use tracing::debug;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// CI/CD pipeline operations.
#[derive(Subcommand)]
pub enum CiCommand {
    /// Trigger a pipeline run for a repository.
    Run(RunArgs),

    /// Get the status of a pipeline run.
    Status(StatusArgs),

    /// Get a schema-versioned native CI run receipt.
    Receipt(ReceiptArgs),

    /// List pipeline runs.
    List(ListArgs),

    /// Cancel a running pipeline.
    Cancel(CancelArgs),

    /// Watch a repository for automatic CI triggering.
    Watch(WatchArgs),

    /// Stop watching a repository.
    Unwatch(UnwatchArgs),

    /// Stream build logs for a running or completed job.
    Logs(LogsArgs),

    /// Get the full output (stdout/stderr) for a completed job.
    Output(OutputArgs),

    /// Get the latest pipeline status for a repository ref.
    RefStatus(RefStatusArgs),
}

#[derive(Args)]
pub struct RunArgs {
    /// Repository ID to run CI for.
    pub repo_id: String,

    /// Git reference (branch or tag) to build.
    #[arg(long, default_value = "refs/heads/main")]
    pub ref_name: String,

    /// Specific commit hash (optional, uses ref head if not specified).
    #[arg(long)]
    pub commit: Option<String>,
}

#[derive(Args)]
pub struct StatusArgs {
    /// Pipeline run ID.
    pub run_id: String,

    /// Follow status updates (poll every second until completion).
    #[arg(short, long)]
    pub follow: bool,
}

#[derive(Args)]
pub struct ReceiptArgs {
    /// Pipeline run ID.
    pub run_id: String,
}

#[derive(Args)]
pub struct ListArgs {
    /// Filter by repository ID.
    #[arg(long)]
    pub repo_id: Option<String>,

    /// Filter by status; stable labels include initializing, checking_out,
    /// checkout_failed, pending, running, success, failed, cancelled.
    #[arg(long)]
    pub status: Option<String>,

    /// Maximum results (default 50, max 500).
    #[arg(long, default_value = "50")]
    pub limit: u32,
}

#[derive(Args)]
pub struct CancelArgs {
    /// Pipeline run ID to cancel.
    pub run_id: String,

    /// Cancellation reason.
    #[arg(long)]
    pub reason: Option<String>,
}

#[derive(Args)]
pub struct WatchArgs {
    /// Repository ID to watch for CI triggers.
    pub repo_id: String,
}

#[derive(Args)]
pub struct UnwatchArgs {
    /// Repository ID to stop watching.
    pub repo_id: String,
}

#[derive(Args)]
pub struct LogsArgs {
    /// Pipeline run ID.
    pub run_id: String,

    /// Job ID within the pipeline.
    pub job_id: String,

    /// Follow logs in real-time (tail -f style). Polls until the job completes.
    #[arg(short, long)]
    pub follow: bool,
}

#[derive(Args)]
pub struct OutputArgs {
    /// Pipeline run ID.
    pub run_id: String,

    /// Job ID within the pipeline.
    pub job_id: String,

    /// Show only stdout.
    #[arg(long, conflicts_with = "stderr")]
    pub stdout: bool,

    /// Show only stderr.
    #[arg(long, conflicts_with = "stdout")]
    pub stderr: bool,
}

#[derive(Args)]
pub struct RefStatusArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Ref name (e.g., "refs/heads/main"). Defaults to "refs/heads/main".
    #[arg(default_value = "refs/heads/main")]
    pub ref_name: String,
}

/// Pipeline trigger output.
pub struct CiTriggerOutput {
    pub is_success: bool,
    pub run_id: Option<String>,
    pub error: Option<String>,
}

impl Outputable for CiTriggerOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "is_success": self.is_success,
            "run_id": self.run_id,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.is_success {
            format!("Pipeline triggered: {}", self.run_id.as_deref().unwrap_or("unknown"))
        } else {
            format!("Trigger failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Pipeline status output.
pub struct CiStatusOutput {
    pub was_found: bool,
    pub run_id: Option<String>,
    pub repo_id: Option<String>,
    pub ref_name: Option<String>,
    pub status: Option<String>,
    pub stages: Vec<StageInfo>,
    pub created_at_ms: Option<u64>,
    pub completed_at_ms: Option<u64>,
    pub error: Option<String>,
}

pub struct StageInfo {
    pub name: String,
    pub status: String,
    pub jobs: Vec<JobInfo>,
}

pub struct JobInfo {
    pub id: String,
    pub name: String,
    pub status: String,
}

impl Outputable for CiStatusOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "was_found": self.was_found,
            "run_id": self.run_id,
            "repo_id": self.repo_id,
            "ref_name": self.ref_name,
            "status": self.status,
            "stages": self.stages.iter().map(|s| json!({
                "name": s.name,
                "status": s.status,
                "jobs": s.jobs.iter().map(|j| json!({
                    "id": j.id,
                    "name": j.name,
                    "status": j.status
                })).collect::<Vec<_>>()
            })).collect::<Vec<_>>(),
            "created_at_ms": self.created_at_ms,
            "completed_at_ms": self.completed_at_ms,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if !self.was_found {
            return format!(
                "Pipeline not found{}",
                self.error.as_ref().map(|e| format!(": {}", e)).unwrap_or_default()
            );
        }

        let mut output = format!(
            "Pipeline: {}\n\
             Repository: {}\n\
             Ref: {}\n\
             Status: {}\n",
            self.run_id.as_deref().unwrap_or("unknown"),
            self.repo_id.as_deref().unwrap_or("unknown"),
            self.ref_name.as_deref().unwrap_or("unknown"),
            self.status.as_deref().unwrap_or("unknown"),
        );

        if !self.stages.is_empty() {
            output.push_str("\nStages:\n");
            for stage in &self.stages {
                output.push_str(&format!("  {} [{}]\n", stage.name, stage.status));
                for job in &stage.jobs {
                    output.push_str(&format!("    - {} [{}]\n", job.name, job.status));
                }
            }
        }

        output
    }
}

/// Pipeline list output.
pub struct CiListOutput {
    pub runs: Vec<RunInfo>,
}

pub struct RunInfo {
    pub run_id: String,
    pub repo_id: String,
    pub ref_name: String,
    pub status: String,
    pub created_at_ms: u64,
}

impl Outputable for CiListOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "runs": self.runs.iter().map(|r| json!({
                "run_id": r.run_id,
                "repo_id": r.repo_id,
                "ref_name": r.ref_name,
                "status": r.status,
                "created_at_ms": r.created_at_ms
            })).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.runs.is_empty() {
            return "No pipeline runs found".to_string();
        }

        let mut output = format!("Pipeline Runs ({}):\n\n", self.runs.len());
        output.push_str("RUN ID                                REPO           REF              STATUS\n");
        output.push_str("────────────────────────────────────  ─────────────  ───────────────  ────────\n");

        for run in &self.runs {
            let id_short = if run.run_id.len() > 36 {
                &run.run_id[..36]
            } else {
                &run.run_id
            };
            let repo_short = if run.repo_id.len() > 13 {
                format!("{}...", &run.repo_id[..10])
            } else {
                run.repo_id.clone()
            };
            let ref_short = if run.ref_name.len() > 15 {
                format!("{}...", &run.ref_name[..12])
            } else {
                run.ref_name.clone()
            };

            output.push_str(&format!("{:<36}  {:<13}  {:<15}  {}\n", id_short, repo_short, ref_short, run.status));
        }

        output
    }
}

/// Pipeline cancel output.
pub struct CiCancelOutput {
    pub is_success: bool,
    pub error: Option<String>,
}

impl Outputable for CiCancelOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "is_success": self.is_success,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.is_success {
            "Pipeline cancelled".to_string()
        } else {
            format!("Cancel failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Watch repo output.
pub struct CiWatchOutput {
    pub is_success: bool,
    pub error: Option<String>,
}

impl Outputable for CiWatchOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "is_success": self.is_success,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.is_success {
            "Repository watching enabled".to_string()
        } else {
            format!("Watch failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// CI job output response.
pub struct CiOutputOutput {
    pub was_found: bool,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub stdout_was_blob: bool,
    pub stderr_was_blob: bool,
    pub stdout_size: u64,
    pub stderr_size: u64,
    pub error: Option<String>,
}

impl Outputable for CiOutputOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "was_found": self.was_found,
            "stdout": self.stdout,
            "stderr": self.stderr,
            "stdout_was_blob": self.stdout_was_blob,
            "stderr_was_blob": self.stderr_was_blob,
            "stdout_size": self.stdout_size,
            "stderr_size": self.stderr_size,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if !self.was_found {
            return format!("Job not found{}", self.error.as_ref().map(|e| format!(": {}", e)).unwrap_or_default());
        }

        let mut output = String::new();

        // Show metadata
        if self.stdout_was_blob || self.stderr_was_blob {
            output.push_str(&format!(
                "# Output retrieved from blob storage (stdout: {} bytes, stderr: {} bytes)\n\n",
                self.stdout_size, self.stderr_size
            ));
        }

        if let Some(stdout) = &self.stdout {
            if !stdout.is_empty() {
                output.push_str("=== STDOUT ===\n");
                output.push_str(stdout);
                if !stdout.ends_with('\n') {
                    output.push('\n');
                }
            }
        }

        if let Some(stderr) = &self.stderr {
            if !stderr.is_empty() {
                if !output.is_empty() {
                    output.push('\n');
                }
                output.push_str("=== STDERR ===\n");
                output.push_str(stderr);
                if !stderr.ends_with('\n') {
                    output.push('\n');
                }
            }
        }

        if output.is_empty() {
            output = "(no output)".to_string();
        }

        output
    }
}

/// Native CI run receipt output.
pub struct CiRunReceiptOutput {
    pub was_found: bool,
    pub receipt: Option<CiRunReceipt>,
    pub error: Option<String>,
}

impl Outputable for CiRunReceiptOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "was_found": self.was_found,
            "receipt": self.receipt,
            "error": self.error,
        })
    }

    fn to_human(&self) -> String {
        let Some(receipt) = &self.receipt else {
            return format!("CI receipt not found: {}", self.error.as_deref().unwrap_or("unknown error"));
        };
        let succeeded = receipt.stages.iter().filter(|stage| stage.status == CI_STATUS_SUCCESS).count();
        let total_jobs: usize = receipt.stages.iter().map(|stage| stage.jobs.len()).sum();
        let jobs_with_ids =
            receipt.stages.iter().flat_map(|stage| stage.jobs.iter()).filter(|job| job.job_id.is_some()).count();
        let mut output = format!(
            "CI receipt: {}\nSchema: {}\nPipeline: {}\nRepository: {}\nRef: {}\nCommit: {}\nStatus: {}\nStages: {}/{} succeeded\nJobs: {} ({} with log handles)",
            receipt.run_id,
            receipt.schema,
            receipt.pipeline_name,
            receipt.repo_id,
            receipt.ref_name,
            receipt.commit_hash,
            receipt.status,
            succeeded,
            receipt.stages.len(),
            total_jobs,
            jobs_with_ids,
        );
        if let Some(error) = &receipt.error {
            output.push_str(&format!("\nError: {error}"));
        }
        output
    }
}

impl CiCommand {
    /// Execute the CI command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            CiCommand::Run(args) => ci_run(client, args, json).await,
            CiCommand::Status(args) => ci_status(client, args, json).await,
            CiCommand::Receipt(args) => ci_receipt(client, args, json).await,
            CiCommand::List(args) => ci_list(client, args, json).await,
            CiCommand::Cancel(args) => ci_cancel(client, args, json).await,
            CiCommand::Watch(args) => ci_watch(client, args, json).await,
            CiCommand::Unwatch(args) => ci_unwatch(client, args, json).await,
            CiCommand::Logs(args) => ci_logs(client, args, json).await,
            CiCommand::Output(args) => ci_output(client, args, json).await,
            CiCommand::RefStatus(args) => ci_ref_status(client, args, json).await,
        }
    }
}

async fn ci_run(client: &AspenClient, args: RunArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::CiTriggerPipeline {
            repo_id: args.repo_id,
            ref_name: args.ref_name,
            commit_hash: args.commit,
        })
        .await?;

    match response {
        ClientRpcResponse::CiTriggerPipelineResult(result) => {
            let output = CiTriggerOutput {
                is_success: result.is_success,
                run_id: result.run_id,
                error: result.error,
            };
            print_output(&output, json);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn ci_receipt(client: &AspenClient, args: ReceiptArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::CiGetRunReceipt { run_id: args.run_id }).await?;

    match response {
        ClientRpcResponse::CiGetRunReceiptResult(result) => {
            let was_found = result.was_found;
            let output = CiRunReceiptOutput {
                was_found,
                receipt: result.receipt,
                error: result.error,
            };
            print_output(&output, json);
            if !was_found {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn ci_status(client: &AspenClient, args: StatusArgs, json: bool) -> Result<()> {
    if args.follow {
        ci_status_follow(client, &args.run_id, json).await
    } else {
        ci_status_single(client, &args.run_id, json).await
    }
}

/// Follow mode: poll status every second until a terminal state is reached.
async fn ci_status_follow(client: &AspenClient, run_id: &str, json: bool) -> Result<()> {
    loop {
        let response = client
            .send(ClientRpcRequest::CiGetStatus {
                run_id: run_id.to_string(),
            })
            .await?;

        match response {
            ClientRpcResponse::CiGetStatusResult(result) => {
                let output = ci_status_build_output(&result);

                let is_terminal = ci_status_is_terminal(&result.status);

                if !json {
                    print!("\x1B[2J\x1B[1;1H");
                    println!("{}", output.to_human());
                } else {
                    println!("{}", serde_json::to_string(&output.to_json())?);
                }

                if is_terminal {
                    return Ok(());
                }
            }
            ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
            _ => anyhow::bail!("unexpected response type"),
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

/// Single status check (non-follow mode).
async fn ci_status_single(client: &AspenClient, run_id: &str, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::CiGetStatus {
            run_id: run_id.to_string(),
        })
        .await?;

    match response {
        ClientRpcResponse::CiGetStatusResult(result) => {
            let was_found = result.was_found;
            let output = ci_status_build_output(&result);
            print_output(&output, json);
            if !was_found {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

/// Build a CiStatusOutput from a CiGetStatusResponse.
fn ci_status_build_output(result: &aspen_client_api::CiGetStatusResponse) -> CiStatusOutput {
    let stages: Vec<StageInfo> = result
        .stages
        .iter()
        .map(|s| StageInfo {
            name: s.name.clone(),
            status: s.status.clone(),
            jobs: s
                .jobs
                .iter()
                .map(|j| JobInfo {
                    id: j.id.clone(),
                    name: j.name.clone(),
                    status: j.status.clone(),
                })
                .collect(),
        })
        .collect();

    CiStatusOutput {
        was_found: result.was_found,
        run_id: result.run_id.clone(),
        repo_id: result.repo_id.clone(),
        ref_name: result.ref_name.clone(),
        status: result.status.clone(),
        stages,
        created_at_ms: result.created_at_ms,
        completed_at_ms: result.completed_at_ms,
        error: result.error.clone(),
    }
}

/// Check if a pipeline status is terminal (no more polling needed).
fn ci_status_is_terminal(status: &Option<String>) -> bool {
    status.as_ref().map(|s| CI_TERMINAL_STATUS_LABELS.contains(&s.as_str())).unwrap_or(false)
}

/// Build the canonical KV watch prefix for a CI job log stream.
fn ci_log_watch_prefix(run_id: &str, job_id: &str) -> String {
    format!("{CI_LOG_KV_PREFIX}{run_id}:{job_id}:")
}

/// Detect the source-of-truth CI log stream completion marker.
fn ci_log_is_completion_marker_key(key: &str) -> bool {
    key.ends_with(CI_LOG_COMPLETE_MARKER)
}

async fn ci_list(client: &AspenClient, args: ListArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::CiListRuns {
            repo_id: args.repo_id,
            status: args.status,
            limit: Some(args.limit),
        })
        .await?;

    match response {
        ClientRpcResponse::CiListRunsResult(result) => {
            let runs: Vec<RunInfo> = result
                .runs
                .iter()
                .map(|r| RunInfo {
                    run_id: r.run_id.clone(),
                    repo_id: r.repo_id.clone(),
                    ref_name: r.ref_name.clone(),
                    status: r.status.clone(),
                    created_at_ms: r.created_at_ms,
                })
                .collect();

            let output = CiListOutput { runs };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn ci_cancel(client: &AspenClient, args: CancelArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::CiCancelRun {
            run_id: args.run_id,
            reason: args.reason,
        })
        .await?;

    match response {
        ClientRpcResponse::CiCancelRunResult(result) => {
            let output = CiCancelOutput {
                is_success: result.is_success,
                error: result.error,
            };
            print_output(&output, json);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn ci_watch(client: &AspenClient, args: WatchArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::CiWatchRepo { repo_id: args.repo_id }).await?;

    match response {
        ClientRpcResponse::CiWatchRepoResult(result) => {
            let output = CiWatchOutput {
                is_success: result.is_success,
                error: result.error,
            };
            print_output(&output, json);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn ci_unwatch(client: &AspenClient, args: UnwatchArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::CiUnwatchRepo { repo_id: args.repo_id }).await?;

    match response {
        ClientRpcResponse::CiUnwatchRepoResult(result) => {
            let output = CiWatchOutput {
                is_success: result.is_success,
                error: result.error,
            };
            print_output(&output, json);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn ci_logs(client: &AspenClient, args: LogsArgs, json: bool) -> Result<()> {
    let mut start_index: u32 = 0;
    let chunk_limit: u32 = 100;

    // Phase 1: Historical catch-up via CiGetJobLogs RPC.
    // Fetch all existing chunks before switching to watch-based streaming.
    let mut is_complete = false;

    loop {
        let response = client
            .send(ClientRpcRequest::CiGetJobLogs {
                run_id: args.run_id.clone(),
                job_id: args.job_id.clone(),
                start_index,
                limit: Some(chunk_limit),
            })
            .await?;

        match response {
            ClientRpcResponse::CiGetJobLogsResult(result) => {
                if !result.was_found && start_index == 0 {
                    if args.follow {
                        // Job hasn't produced logs yet — wait and retry.
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        continue;
                    }
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string(&json!({
                                "error": result.error.as_deref().unwrap_or("Job logs not found"),
                                "was_found": false
                            }))?
                        );
                    } else {
                        eprintln!(
                            "No logs found for job {}{}",
                            args.job_id,
                            result.error.as_ref().map(|e| format!(": {}", e)).unwrap_or_default()
                        );
                    }
                    std::process::exit(1);
                }

                print_log_chunks(&result.chunks, json)?;

                if result.last_index >= start_index {
                    start_index = result.last_index + 1;
                }

                if result.is_complete {
                    is_complete = true;
                    break;
                }

                if !args.follow {
                    if result.has_more {
                        continue;
                    }
                    return Ok(());
                }

                // All historical chunks fetched, break to watch phase.
                if !result.has_more {
                    break;
                }
            }
            ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
            _ => anyhow::bail!("unexpected response type"),
        }
    }

    // Job already complete — no need for watch.
    if is_complete || !args.follow {
        return Ok(());
    }

    // Phase 2: Real-time streaming via WatchSession.
    // Falls back to polling if the watch connection fails.
    let watch_prefix = ci_log_watch_prefix(&args.run_id, &args.job_id);

    match try_watch_logs(client, &watch_prefix, start_index, json).await {
        Ok(()) => Ok(()),
        Err(e) => {
            debug!(error = %e, "watch connection failed, falling back to polling");
            ci_logs_poll_loop(client, &args.run_id, &args.job_id, start_index, json).await
        }
    }
}

/// Stream log chunks via WatchSession KV prefix subscription.
///
/// Connects to the cluster's log subscriber endpoint and subscribes to
/// the job's log key prefix. Prints chunks as they arrive and exits
/// when the completion marker key is received.
async fn try_watch_logs(client: &AspenClient, prefix: &str, last_chunk_index: u32, json: bool) -> Result<()> {
    let peer_addr = client.first_peer_addr().ok_or_else(|| anyhow::anyhow!("no peers available"))?;

    let session = WatchSession::connect(client.endpoint(), peer_addr.clone(), client.cluster_id()).await?;

    // Subscribe from start_index 0 to get all events from the beginning.
    // We use last_chunk_index to skip chunks we already printed during catch-up.
    let mut subscription = session.subscribe(prefix.as_bytes().to_vec(), 0).await?;

    let mut last_seen_index = last_chunk_index;

    while let Some(event) = subscription.next().await {
        match event {
            WatchEvent::Set { key, value, .. } | WatchEvent::SetWithTTL { key, value, .. } => {
                let key_str = String::from_utf8_lossy(&key);

                // Check for completion marker.
                if ci_log_is_completion_marker_key(&key_str) {
                    return Ok(());
                }

                // Parse the chunk from the watch event value.
                let chunk: CiLogChunk = match serde_json::from_slice(&value) {
                    Ok(c) => c,
                    Err(_) => continue, // Skip unparseable values
                };

                // Skip chunks already printed during historical catch-up.
                if chunk.index < last_seen_index {
                    continue;
                }

                print_single_chunk(&chunk, json)?;
                last_seen_index = chunk.index + 1;
            }
            WatchEvent::EndOfStream { reason } => {
                debug!(reason = %reason, "watch stream ended");
                // Return error so caller falls back to polling from last_seen_index.
                return Err(anyhow::anyhow!("watch stream ended: {}", reason));
            }
            // Ignore keepalives, deletes, membership changes, etc.
            _ => {}
        }
    }

    // Channel closed without EndOfStream — treat as disconnect.
    Err(anyhow::anyhow!("watch subscription channel closed"))
}

/// Polling fallback for CI log streaming.
///
/// Used when the WatchSession connection fails or drops mid-stream.
/// Polls CiGetJobLogs every second until the job completes.
async fn ci_logs_poll_loop(
    client: &AspenClient,
    run_id: &str,
    job_id: &str,
    mut start_index: u32,
    json: bool,
) -> Result<()> {
    let chunk_limit: u32 = 100;

    loop {
        let response = client
            .send(ClientRpcRequest::CiGetJobLogs {
                run_id: run_id.to_string(),
                job_id: job_id.to_string(),
                start_index,
                limit: Some(chunk_limit),
            })
            .await?;

        match response {
            ClientRpcResponse::CiGetJobLogsResult(result) => {
                print_log_chunks(&result.chunks, json)?;

                if result.last_index >= start_index {
                    start_index = result.last_index + 1;
                }

                if result.is_complete {
                    return Ok(());
                }

                if result.chunks.is_empty() {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
            ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
            _ => anyhow::bail!("unexpected response type"),
        }
    }
}

/// Print a batch of log chunks to stdout.
fn print_log_chunks(chunks: &[aspen_client_api::ci::CiLogChunkInfo], json: bool) -> Result<()> {
    for chunk in chunks {
        if json {
            println!(
                "{}",
                serde_json::to_string(&json!({
                    "index": chunk.index,
                    "content": chunk.content,
                    "timestamp_ms": chunk.timestamp_ms
                }))?
            );
        } else {
            print!("{}", chunk.content);
        }
    }
    Ok(())
}

/// Print a single CiLogChunk from a watch event.
fn print_single_chunk(chunk: &CiLogChunk, json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string(&json!({
                "index": chunk.index,
                "content": chunk.content,
                "timestamp_ms": chunk.timestamp_ms
            }))?
        );
    } else {
        print!("{}", chunk.content);
    }
    Ok(())
}

async fn ci_output(client: &AspenClient, args: OutputArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::CiGetJobOutput {
            run_id: args.run_id,
            job_id: args.job_id,
        })
        .await?;

    match response {
        ClientRpcResponse::CiGetJobOutputResult(result) => {
            // Filter based on --stdout or --stderr flags
            let (stdout, stderr) = if args.stdout {
                (result.stdout, None)
            } else if args.stderr {
                (None, result.stderr)
            } else {
                (result.stdout, result.stderr)
            };

            let output = CiOutputOutput {
                was_found: result.was_found,
                stdout,
                stderr,
                stdout_was_blob: result.stdout_was_blob,
                stderr_was_blob: result.stderr_was_blob,
                stdout_size: result.stdout_size,
                stderr_size: result.stderr_size,
                error: result.error,
            };
            print_output(&output, json);
            if !result.was_found {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn ci_ref_status(client: &AspenClient, args: RefStatusArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::CiGetRefStatus {
            repo_id: args.repo_id.clone(),
            ref_name: args.ref_name.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::CiGetRefStatusResult(result) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else if result.was_found {
                println!("Run:       {}", result.run_id.as_deref().unwrap_or("-"));
                println!("Repo:      {}", result.repo_id.as_deref().unwrap_or("-"));
                println!("Ref:       {}", result.ref_name.as_deref().unwrap_or("-"));
                println!("Commit:    {}", result.commit_hash.as_deref().unwrap_or("-"));
                println!("Status:    {}", result.status.as_deref().unwrap_or("unknown"));
                if !result.stages.is_empty() {
                    println!("Stages:");
                    for stage in &result.stages {
                        println!("  {} — {}", stage.name, stage.status);
                    }
                }
                if let Some(ref err) = result.error {
                    println!("Error:     {}", err);
                }
            } else {
                eprintln!("No pipeline run found for {} on {}", args.repo_id, args.ref_name);
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
#[cfg(test)]
mod tests {
    use aspen_client_api::CI_STATUS_CANCELLED;
    use aspen_client_api::CI_STATUS_CHECKOUT_FAILED;
    use aspen_client_api::CI_STATUS_FAILED;
    use aspen_client_api::CI_STATUS_RUNNING;
    use aspen_client_api::CI_STATUS_SUCCESS;

    use super::*;

    #[test]
    fn ci_status_follow_terminal_contract_includes_all_terminal_labels() {
        for label in [
            CI_STATUS_CHECKOUT_FAILED,
            CI_STATUS_SUCCESS,
            CI_STATUS_FAILED,
            CI_STATUS_CANCELLED,
        ] {
            assert!(ci_status_is_terminal(&Some(label.to_string())), "{label} must stop follow mode");
        }

        assert!(!ci_status_is_terminal(&Some(CI_STATUS_RUNNING.to_string())));
        assert!(!ci_status_is_terminal(&None));
    }

    #[test]
    fn ci_log_follow_uses_shared_completion_marker_contract() {
        let prefix = ci_log_watch_prefix("run-1", "job-1");
        assert_eq!(prefix, format!("{}run-1:job-1:", CI_LOG_KV_PREFIX));

        let completion_key = format!("{prefix}{CI_LOG_COMPLETE_MARKER}");
        assert!(ci_log_is_completion_marker_key(&completion_key));
        assert!(!ci_log_is_completion_marker_key(&format!("{prefix}0000000001")));
    }
}
