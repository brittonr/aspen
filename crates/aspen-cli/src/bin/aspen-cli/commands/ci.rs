//! CI/CD pipeline commands.
//!
//! Commands for triggering and monitoring CI pipelines with
//! Nickel-based configuration and distributed job execution.

use anyhow::Result;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
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

/// Pipeline trigger output.
pub struct CiTriggerOutput {
    pub success: bool,
    pub run_id: Option<String>,
    pub error: Option<String>,
}

impl Outputable for CiTriggerOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "run_id": self.run_id,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            format!("Pipeline triggered: {}", self.run_id.as_deref().unwrap_or("unknown"))
        } else {
            format!("Trigger failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Pipeline status output.
pub struct CiStatusOutput {
    pub found: bool,
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
            "found": self.found,
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
        if !self.found {
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
    pub success: bool,
    pub error: Option<String>,
}

impl Outputable for CiCancelOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            "Pipeline cancelled".to_string()
        } else {
            format!("Cancel failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Watch repo output.
pub struct CiWatchOutput {
    pub success: bool,
    pub error: Option<String>,
}

impl Outputable for CiWatchOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            "Repository watching enabled".to_string()
        } else {
            format!("Watch failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
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
                success: result.success,
                run_id: result.run_id,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
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
        // Follow mode: poll every second
        loop {
            let response = client
                .send(ClientRpcRequest::CiGetStatus {
                    run_id: args.run_id.clone(),
                })
                .await?;

            match response {
                ClientRpcResponse::CiGetStatusResult(result) => {
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

                    let output = CiStatusOutput {
                        found: result.found,
                        run_id: result.run_id,
                        repo_id: result.repo_id,
                        ref_name: result.ref_name,
                        status: result.status.clone(),
                        stages,
                        created_at_ms: result.created_at_ms,
                        completed_at_ms: result.completed_at_ms,
                        error: result.error,
                    };

                    if !json {
                        // Clear screen and show status
                        print!("\x1B[2J\x1B[1;1H");
                        println!("{}", output.to_human());

                        // Check terminal states
                        if let Some(status) = &result.status {
                            if status == "completed" || status == "failed" || status == "cancelled" {
                                return Ok(());
                            }
                        }
                    } else {
                        println!("{}", serde_json::to_string(&output.to_json())?);
                        if let Some(status) = &result.status {
                            if status == "completed" || status == "failed" || status == "cancelled" {
                                return Ok(());
                            }
                        }
                    }
                }
                ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
                _ => anyhow::bail!("unexpected response type"),
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    } else {
        // Single status check
        let response = client.send(ClientRpcRequest::CiGetStatus { run_id: args.run_id }).await?;

        match response {
            ClientRpcResponse::CiGetStatusResult(result) => {
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

                let output = CiStatusOutput {
                    found: result.found,
                    run_id: result.run_id,
                    repo_id: result.repo_id,
                    ref_name: result.ref_name,
                    status: result.status,
                    stages,
                    created_at_ms: result.created_at_ms,
                    completed_at_ms: result.completed_at_ms,
                    error: result.error,
                };
                print_output(&output, json);
                if !result.found {
                    std::process::exit(1);
                }
                Ok(())
            }
            ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
            _ => anyhow::bail!("unexpected response type"),
        }
    }
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
                success: result.success,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
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
                success: result.success,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
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
                success: result.success,
                error: result.error,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
