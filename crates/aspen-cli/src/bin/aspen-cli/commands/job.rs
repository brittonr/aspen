//! Job queue commands.
//!
//! Commands for submitting and managing distributed jobs with
//! priority-based scheduling, retries, and worker pools.

use anyhow::Result;
use clap::Args;
use clap::Subcommand;
use serde_json::json;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_client_rpc::JobDetails;
use aspen_client_rpc::WorkerInfo;

/// Job queue operations.
#[derive(Subcommand)]
pub enum JobCommand {
    /// Submit a new job.
    Submit(SubmitArgs),

    /// Get job details.
    Get(GetArgs),

    /// List jobs with optional filtering.
    List(ListArgs),

    /// Cancel a job.
    Cancel(CancelArgs),

    /// Get job queue statistics.
    Stats,

    /// Get worker pool status.
    Workers,
}

#[derive(Args)]
pub struct SubmitArgs {
    /// Job type identifier.
    pub job_type: String,

    /// Job payload (JSON string).
    pub payload: String,

    /// Priority level (0=Low, 1=Normal, 2=High, 3=Critical).
    #[arg(short, long, default_value = "1")]
    pub priority: u8,

    /// Timeout in seconds.
    #[arg(long, default_value = "300")]
    pub timeout_secs: u64,

    /// Maximum retry attempts.
    #[arg(long, default_value = "3")]
    pub max_retries: u32,

    /// Retry delay in seconds.
    #[arg(long, default_value = "1")]
    pub retry_delay_secs: u64,

    /// Schedule expression (cron format or interval).
    #[arg(long)]
    pub schedule: Option<String>,

    /// Tags for job filtering (comma-separated).
    #[arg(long)]
    pub tags: Option<String>,
}

#[derive(Args)]
pub struct GetArgs {
    /// Job ID.
    pub job_id: String,
}

#[derive(Args)]
pub struct ListArgs {
    /// Filter by status: pending, scheduled, running, completed, failed, cancelled.
    #[arg(long)]
    pub status: Option<String>,

    /// Filter by job type.
    #[arg(long)]
    pub job_type: Option<String>,

    /// Filter by tags (comma-separated, must have all).
    #[arg(long)]
    pub tags: Option<String>,

    /// Maximum results (default 100, max 1000).
    #[arg(long, default_value = "100")]
    pub limit: u32,
}

#[derive(Args)]
pub struct CancelArgs {
    /// Job ID to cancel.
    pub job_id: String,

    /// Optional cancellation reason.
    #[arg(long)]
    pub reason: Option<String>,
}

/// Job submit output.
pub struct JobSubmitOutput {
    pub success: bool,
    pub job_id: Option<String>,
    pub error: Option<String>,
}

impl Outputable for JobSubmitOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "job_id": self.job_id,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            format!("Job submitted: {}", self.job_id.as_deref().unwrap_or("unknown"))
        } else {
            format!("Submit failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Job get output.
pub struct JobGetOutput {
    pub found: bool,
    pub job: Option<JobDetails>,
    pub error: Option<String>,
}

impl Outputable for JobGetOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "found": self.found,
            "job": self.job,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(job) = &self.job {
            format!(
                "Job: {}\n\
                 Type: {}\n\
                 Status: {}\n\
                 Priority: {}\n\
                 Progress: {}%{}\n\
                 Worker: {}\n\
                 Submitted: {}\n\
                 Started: {}\n\
                 Completed: {}{}",
                job.job_id,
                job.job_type,
                job.status,
                job.priority,
                job.progress,
                job.progress_message
                    .as_ref()
                    .map(|m| format!(" ({})", m))
                    .unwrap_or_default(),
                job.worker_id.as_deref().unwrap_or("none"),
                job.submitted_at,
                job.started_at.as_deref().unwrap_or("not started"),
                job.completed_at.as_deref().unwrap_or("not completed"),
                if let Some(err) = &job.error_message {
                    format!("\nError: {}", err)
                } else if let Some(result) = &job.result {
                    format!("\nResult: {}", serde_json::to_string_pretty(result).unwrap_or_default())
                } else {
                    String::new()
                }
            )
        } else {
            format!("Job not found{}",
                self.error.as_ref()
                    .map(|e| format!(": {}", e))
                    .unwrap_or_default())
        }
    }
}

/// Job list output.
pub struct JobListOutput {
    pub jobs: Vec<JobDetails>,
    pub total_count: u32,
    pub error: Option<String>,
}

impl Outputable for JobListOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "jobs": self.jobs,
            "total_count": self.total_count,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.jobs.is_empty() {
            return "No jobs found".to_string();
        }

        let mut output = format!("Jobs ({} total):\n\n", self.total_count);

        // Table header
        output.push_str("ID                                    TYPE            STATUS      PRIORITY  PROGRESS\n");
        output.push_str("────────────────────────────────────  ──────────────  ──────────  ────────  ────────\n");

        for job in &self.jobs {
            let id_short = if job.job_id.len() > 36 {
                &job.job_id[..36]
            } else {
                &job.job_id
            };

            let type_short = if job.job_type.len() > 14 {
                format!("{}...", &job.job_type[..11])
            } else {
                job.job_type.clone()
            };

            let priority_str = match job.priority {
                0 => "Low",
                1 => "Normal",
                2 => "High",
                3 => "Critical",
                _ => "Unknown",
            };

            output.push_str(&format!(
                "{:<36}  {:<14}  {:<10}  {:<8}  {:>3}%\n",
                id_short,
                type_short,
                job.status,
                priority_str,
                job.progress
            ));
        }

        output
    }
}

/// Job cancel output.
pub struct JobCancelOutput {
    pub success: bool,
    pub previous_status: Option<String>,
    pub error: Option<String>,
}

impl Outputable for JobCancelOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "previous_status": self.previous_status,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            format!("Job cancelled (was: {})",
                self.previous_status.as_deref().unwrap_or("unknown"))
        } else {
            format!("Cancel failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Job stats output.
pub struct JobStatsOutput {
    pub pending: u64,
    pub scheduled: u64,
    pub running: u64,
    pub completed: u64,
    pub failed: u64,
    pub cancelled: u64,
    pub error: Option<String>,
}

impl Outputable for JobStatsOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "pending": self.pending,
            "scheduled": self.scheduled,
            "running": self.running,
            "completed": self.completed,
            "failed": self.failed,
            "cancelled": self.cancelled,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        format!(
            "Job Queue Statistics\n\
             ──────────────────────\n\
             Pending:    {:>8}\n\
             Scheduled:  {:>8}\n\
             Running:    {:>8}\n\
             Completed:  {:>8}\n\
             Failed:     {:>8}\n\
             Cancelled:  {:>8}\n\
             ──────────────────────\n\
             Total:      {:>8}",
            self.pending,
            self.scheduled,
            self.running,
            self.completed,
            self.failed,
            self.cancelled,
            self.pending + self.scheduled + self.running + self.completed + self.failed + self.cancelled
        )
    }
}

/// Worker status output.
pub struct WorkerStatusOutput {
    pub workers: Vec<WorkerInfo>,
    pub total_workers: u32,
    pub idle: u32,
    pub busy: u32,
    pub offline: u32,
    pub total_capacity: u32,
    pub used_capacity: u32,
    pub error: Option<String>,
}

impl Outputable for WorkerStatusOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "workers": self.workers,
            "total_workers": self.total_workers,
            "idle_workers": self.idle,
            "busy_workers": self.busy,
            "offline_workers": self.offline,
            "total_capacity": self.total_capacity,
            "used_capacity": self.used_capacity,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        let mut output = format!(
            "Worker Pool Status\n\
             ────────────────────\n\
             Total Workers:  {}\n\
             Idle:           {}\n\
             Busy:           {}\n\
             Offline:        {}\n\
             Capacity:       {}/{}\n\n",
            self.total_workers,
            self.idle,
            self.busy,
            self.offline,
            self.used_capacity,
            self.total_capacity
        );

        if !self.workers.is_empty() {
            output.push_str("Workers:\n");
            output.push_str("ID                    STATUS    ACTIVE  CAPACITY  PROCESSED  FAILED\n");
            output.push_str("────────────────────  ────────  ──────  ────────  ─────────  ──────\n");

            for worker in &self.workers {
                let id_short = if worker.worker_id.len() > 20 {
                    format!("{}...", &worker.worker_id[..17])
                } else {
                    worker.worker_id.clone()
                };

                output.push_str(&format!(
                    "{:<20}  {:<8}  {:>6}  {:>8}  {:>9}  {:>6}\n",
                    id_short,
                    worker.status,
                    worker.active_jobs,
                    worker.capacity,
                    worker.total_processed,
                    worker.total_failed
                ));
            }
        }

        output
    }
}

impl JobCommand {
    /// Execute the job command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            JobCommand::Submit(args) => job_submit(client, args, json).await,
            JobCommand::Get(args) => job_get(client, args, json).await,
            JobCommand::List(args) => job_list(client, args, json).await,
            JobCommand::Cancel(args) => job_cancel(client, args, json).await,
            JobCommand::Stats => job_stats(client, json).await,
            JobCommand::Workers => worker_status(client, json).await,
        }
    }
}

async fn job_submit(client: &AspenClient, args: SubmitArgs, json: bool) -> Result<()> {
    // Parse payload JSON
    let payload: serde_json::Value = serde_json::from_str(&args.payload)
        .map_err(|e| anyhow::anyhow!("Invalid payload JSON: {}", e))?;

    // Parse tags
    let tags = args.tags
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let response = client
        .send(ClientRpcRequest::JobSubmit {
            job_type: args.job_type,
            payload,
            priority: Some(args.priority),
            timeout_ms: Some(args.timeout_secs * 1000),
            max_retries: Some(args.max_retries),
            retry_delay_ms: Some(args.retry_delay_secs * 1000),
            schedule: args.schedule,
            tags,
        })
        .await?;

    match response {
        ClientRpcResponse::JobSubmitResult(result) => {
            let output = JobSubmitOutput {
                success: result.success,
                job_id: result.job_id,
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

async fn job_get(client: &AspenClient, args: GetArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::JobGet {
            job_id: args.job_id,
        })
        .await?;

    match response {
        ClientRpcResponse::JobGetResult(result) => {
            let output = JobGetOutput {
                found: result.found,
                job: result.job,
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

async fn job_list(client: &AspenClient, args: ListArgs, json: bool) -> Result<()> {
    // Parse tags
    let tags = args.tags
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let response = client
        .send(ClientRpcRequest::JobList {
            status: args.status,
            job_type: args.job_type,
            tags,
            limit: Some(args.limit),
            continuation_token: None,
        })
        .await?;

    match response {
        ClientRpcResponse::JobListResult(result) => {
            let output = JobListOutput {
                jobs: result.jobs,
                total_count: result.total_count,
                error: result.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn job_cancel(client: &AspenClient, args: CancelArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::JobCancel {
            job_id: args.job_id,
            reason: args.reason,
        })
        .await?;

    match response {
        ClientRpcResponse::JobCancelResult(result) => {
            let output = JobCancelOutput {
                success: result.success,
                previous_status: result.previous_status,
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

async fn job_stats(client: &AspenClient, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::JobQueueStats)
        .await?;

    match response {
        ClientRpcResponse::JobQueueStatsResult(result) => {
            let output = JobStatsOutput {
                pending: result.pending_count,
                scheduled: result.scheduled_count,
                running: result.running_count,
                completed: result.completed_count,
                failed: result.failed_count,
                cancelled: result.cancelled_count,
                error: result.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn worker_status(client: &AspenClient, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::WorkerStatus)
        .await?;

    match response {
        ClientRpcResponse::WorkerStatusResult(result) => {
            let output = WorkerStatusOutput {
                workers: result.workers,
                total_workers: result.total_workers,
                idle: result.idle_workers,
                busy: result.busy_workers,
                offline: result.offline_workers,
                total_capacity: result.total_capacity,
                used_capacity: result.used_capacity,
                error: result.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}