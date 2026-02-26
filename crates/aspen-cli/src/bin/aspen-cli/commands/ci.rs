//! CI/CD pipeline commands.
//!
//! Commands for triggering and monitoring CI pipelines with
//! Nickel-based configuration and distributed job execution.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;
use serde_json::json;

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

    /// List pipeline runs.
    List(ListArgs),

    /// Cancel a running pipeline.
    Cancel(CancelArgs),

    /// Watch a repository for automatic CI triggering.
    Watch(WatchArgs),

    /// Stop watching a repository.
    Unwatch(UnwatchArgs),

    /// Get the full output (stdout/stderr) for a completed job.
    Output(OutputArgs),
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
pub struct ListArgs {
    /// Filter by repository ID.
    #[arg(long)]
    pub repo_id: Option<String>,

    /// Filter by status: pending, running, completed, failed, cancelled.
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

impl CiCommand {
    /// Execute the CI command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            CiCommand::Run(args) => ci_run(client, args, json).await,
            CiCommand::Status(args) => ci_status(client, args, json).await,
            CiCommand::List(args) => ci_list(client, args, json).await,
            CiCommand::Cancel(args) => ci_cancel(client, args, json).await,
            CiCommand::Watch(args) => ci_watch(client, args, json).await,
            CiCommand::Unwatch(args) => ci_unwatch(client, args, json).await,
            CiCommand::Output(args) => ci_output(client, args, json).await,
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
    status.as_ref().map(|s| matches!(s.as_str(), "success" | "failed" | "cancelled")).unwrap_or(false)
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
