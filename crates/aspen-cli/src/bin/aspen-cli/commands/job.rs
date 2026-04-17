//! Job queue commands.
//!
//! Commands for submitting and managing distributed jobs with
//! priority-based scheduling, retries, and worker pools.

use std::path::PathBuf;

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::JobDetails;
use aspen_client_api::WorkerInfo;
use clap::Args;
use clap::Subcommand;
use serde_json::json;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Job queue operations.
#[derive(Subcommand)]
pub enum JobCommand {
    /// Submit a new job.
    Submit(SubmitArgs),

    /// Submit a VM execution job with binary upload.
    SubmitVm(SubmitVmArgs),

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

    /// Get job status and track progress.
    Status(StatusArgs),

    /// Wait for a job to complete and get result.
    Result(ResultArgs),
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
    #[arg(long = "limit", default_value = "100")]
    pub max_results: u32,
}

#[derive(Args)]
pub struct SubmitVmArgs {
    /// Path to the binary file to execute in VM.
    pub binary: PathBuf,

    /// Optional input data for the VM job.
    #[arg(long)]
    pub input: Option<String>,

    /// Priority level (0=Low, 1=Normal, 2=High, 3=Critical).
    #[arg(short, long, default_value = "1")]
    pub priority: u8,

    /// Timeout in seconds.
    #[arg(long, default_value = "10")]
    pub timeout_secs: u64,

    /// Maximum retry attempts.
    #[arg(long, default_value = "0")]
    pub max_retries: u32,

    /// Tags for job filtering (comma-separated).
    #[arg(long)]
    pub tags: Option<String>,

    /// Protection tag for the uploaded binary.
    #[arg(long, default_value = "vm-binary")]
    pub blob_tag: String,
}

#[derive(Args)]
pub struct CancelArgs {
    /// Job ID to cancel.
    pub job_id: String,

    /// Optional cancellation reason.
    #[arg(long)]
    pub reason: Option<String>,
}

#[derive(Args)]
pub struct StatusArgs {
    /// Job ID to check status for.
    pub job_id: String,

    /// Follow job status changes (poll every second).
    #[arg(short, long)]
    pub follow: bool,
}

#[derive(Args)]
pub struct ResultArgs {
    /// Job ID to get result for.
    pub job_id: String,

    /// Maximum time to wait for completion (seconds).
    #[arg(long = "timeout", default_value = "300")]
    pub timeout_secs: u64,
}

/// Job submit output.
pub struct JobSubmitOutput {
    pub is_success: bool,
    pub job_id: Option<String>,
    pub error: Option<String>,
}

impl Outputable for JobSubmitOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "is_success": self.is_success,
            "job_id": self.job_id,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.is_success {
            format!("Job submitted: {}", self.job_id.as_deref().unwrap_or("unknown"))
        } else {
            format!("Submit failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Job get output.
pub struct JobGetOutput {
    pub was_found: bool,
    pub job: Option<JobDetails>,
    pub error: Option<String>,
}

impl Outputable for JobGetOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "was_found": self.was_found,
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
                job.progress_message.as_ref().map(|m| format!(" ({})", m)).unwrap_or_default(),
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
            format!("Job not found{}", self.error.as_ref().map(|e| format!(": {}", e)).unwrap_or_default())
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
                id_short, type_short, job.status, priority_str, job.progress
            ));
        }

        output
    }
}

/// Job cancel output.
pub struct JobCancelOutput {
    pub is_success: bool,
    pub previous_status: Option<String>,
    pub error: Option<String>,
}

impl Outputable for JobCancelOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "is_success": self.is_success,
            "previous_status": self.previous_status,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.is_success {
            format!("Job cancelled (was: {})", self.previous_status.as_deref().unwrap_or("unknown"))
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
        let total_jobs = self
            .pending
            .saturating_add(self.scheduled)
            .saturating_add(self.running)
            .saturating_add(self.completed)
            .saturating_add(self.failed)
            .saturating_add(self.cancelled);
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
            self.pending, self.scheduled, self.running, self.completed, self.failed, self.cancelled, total_jobs
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
    pub total_capacity_jobs: u32,
    pub used_capacity_jobs: u32,
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
            "total_capacity_jobs": self.total_capacity_jobs,
            "used_capacity_jobs": self.used_capacity_jobs,
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
            self.total_workers, self.idle, self.busy, self.offline, self.used_capacity_jobs, self.total_capacity_jobs
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
                    worker.capacity_jobs,
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
    pub async fn run(self, client: &AspenClient, is_json_output: bool) -> Result<()> {
        match self {
            JobCommand::Submit(args) => job_submit(client, args, is_json_output).await,
            JobCommand::SubmitVm(args) => job_submit_vm(client, args, is_json_output).await,
            JobCommand::Get(args) => job_get(client, args, is_json_output).await,
            JobCommand::List(args) => job_list(client, args, is_json_output).await,
            JobCommand::Cancel(args) => job_cancel(client, args, is_json_output).await,
            JobCommand::Stats => job_stats(client, is_json_output).await,
            JobCommand::Workers => worker_status(client, is_json_output).await,
            JobCommand::Status(args) => job_status(client, args, is_json_output).await,
            JobCommand::Result(args) => job_result(client, args, is_json_output).await,
        }
    }
}

fn duration_secs_to_millis(duration_secs: u64) -> u64 {
    duration_secs.saturating_mul(1_000)
}

const MAX_JOB_STATUS_FOLLOW_POLLS: u64 = 86_400;

#[allow(
    ambient_clock,
    reason = "CLI job result waits need a monotonic start instant while polling remote status"
)]
fn job_result_start() -> std::time::Instant {
    std::time::Instant::now()
}

#[allow(
    ambient_clock,
    reason = "CLI job result waits compare monotonic time against a timeout deadline"
)]
fn has_job_result_timed_out(deadline: std::time::Instant) -> bool {
    std::time::Instant::now() > deadline
}

async fn job_submit(client: &AspenClient, args: SubmitArgs, is_json_output: bool) -> Result<()> {
    // Validate payload is valid JSON
    let _: serde_json::Value =
        serde_json::from_str(&args.payload).map_err(|e| anyhow::anyhow!("Invalid payload JSON: {}", e))?;

    // Parse tags
    let tags = args.tags.map(|t| t.split(',').map(|s| s.trim().to_string()).collect()).unwrap_or_default();

    let response = client
        .send(ClientRpcRequest::JobSubmit {
            job_type: args.job_type,
            payload: args.payload,
            priority: Some(args.priority),
            timeout_ms: Some(duration_secs_to_millis(args.timeout_secs)),
            max_retries: Some(args.max_retries),
            retry_delay_ms: Some(duration_secs_to_millis(args.retry_delay_secs)),
            schedule: args.schedule,
            tags,
        })
        .await?;

    match response {
        ClientRpcResponse::JobSubmitResult(result) => {
            let output = JobSubmitOutput {
                is_success: result.is_success,
                job_id: result.job_id,
                error: result.error,
            };
            print_output(&output, is_json_output);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn job_get(client: &AspenClient, args: GetArgs, is_json_output: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::JobGet { job_id: args.job_id }).await?;

    match response {
        ClientRpcResponse::JobGetResult(result) => {
            let output = JobGetOutput {
                was_found: result.was_found,
                job: result.job,
                error: result.error,
            };
            print_output(&output, is_json_output);
            if !result.was_found {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn job_list(client: &AspenClient, args: ListArgs, is_json_output: bool) -> Result<()> {
    // Parse tags
    let tags = args.tags.map(|t| t.split(',').map(|s| s.trim().to_string()).collect()).unwrap_or_default();

    let response = client
        .send(ClientRpcRequest::JobList {
            status: args.status,
            job_type: args.job_type,
            tags,
            limit: Some(args.max_results),
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
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn job_cancel(client: &AspenClient, args: CancelArgs, is_json_output: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::JobCancel {
            job_id: args.job_id,
            reason: args.reason,
        })
        .await?;

    match response {
        ClientRpcResponse::JobCancelResult(result) => {
            let output = JobCancelOutput {
                is_success: result.is_success,
                previous_status: result.previous_status,
                error: result.error,
            };
            print_output(&output, is_json_output);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn job_stats(client: &AspenClient, is_json_output: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::JobQueueStats).await?;

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
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn worker_status(client: &AspenClient, is_json_output: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::WorkerStatus).await?;

    match response {
        ClientRpcResponse::WorkerStatusResult(result) => {
            let output = WorkerStatusOutput {
                workers: result.workers,
                total_workers: result.total_workers,
                idle: result.idle_workers,
                busy: result.busy_workers,
                offline: result.offline_workers,
                total_capacity_jobs: result.total_capacity_jobs,
                used_capacity_jobs: result.used_capacity_jobs,
                error: result.error,
            };
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn job_submit_vm(client: &AspenClient, args: SubmitVmArgs, is_json_output: bool) -> Result<()> {
    // Read the binary file
    let binary_data = std::fs::read(&args.binary).map_err(|e| anyhow::anyhow!("Failed to read binary file: {}", e))?;

    if !is_json_output {
        println!("Uploading binary ({} bytes)...", binary_data.len());
    }

    // Upload binary to blob storage
    let blob_hash = job_submit_vm_upload_binary(client, &binary_data, &args.blob_tag).await?;

    if !is_json_output {
        println!("Binary uploaded with hash: {}", blob_hash);
        println!("Submitting VM job...");
    }

    // Build payload and tags
    let (payload_str, tags) = job_submit_vm_build_payload(&blob_hash, binary_data.len(), args.input, args.tags)?;

    let response = client
        .send(ClientRpcRequest::JobSubmit {
            job_type: "vm_execute".to_string(),
            payload: payload_str,
            priority: Some(args.priority),
            timeout_ms: Some(duration_secs_to_millis(args.timeout_secs)),
            max_retries: Some(args.max_retries),
            retry_delay_ms: Some(1_000),
            schedule: None,
            tags,
        })
        .await?;

    match response {
        ClientRpcResponse::JobSubmitResult(result) => {
            let output = JobSubmitOutput {
                is_success: result.is_success,
                job_id: result.job_id,
                error: result.error,
            };
            print_output(&output, is_json_output);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

/// Upload a binary to blob storage and return the hash.
async fn job_submit_vm_upload_binary(client: &AspenClient, binary_data: &[u8], blob_tag: &str) -> Result<String> {
    let blob_response = client
        .send(ClientRpcRequest::AddBlob {
            data: binary_data.to_vec(),
            tag: Some(blob_tag.to_string()),
        })
        .await?;

    match blob_response {
        ClientRpcResponse::AddBlobResult(result) if result.is_success => {
            result.hash.ok_or_else(|| anyhow::anyhow!("Blob uploaded but no hash returned"))
        }
        ClientRpcResponse::AddBlobResult(result) => {
            anyhow::bail!("Failed to upload binary: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("Blob upload error: {}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type for blob upload"),
    }
}

/// Build the VM job payload JSON string and tags list.
fn job_submit_vm_build_payload(
    blob_hash: &str,
    binary_size: usize,
    input: Option<String>,
    tags_arg: Option<String>,
) -> Result<(String, Vec<String>)> {
    let mut payload = json!({
        "type": "BlobBinary",
        "hash": blob_hash,
        "size": binary_size,
        "format": "elf"
    });

    if let Some(input) = input {
        payload["input"] = json!(input.into_bytes());
    }

    let tags = tags_arg
        .map(|t| {
            let mut tags: Vec<String> = t.split(',').map(|s| s.trim().to_string()).collect();
            tags.push("vm-job".to_string());
            tags
        })
        .unwrap_or_else(|| vec!["vm-job".to_string()]);

    let payload_str =
        serde_json::to_string(&payload).map_err(|e| anyhow::anyhow!("Failed to serialize payload: {}", e))?;

    Ok((payload_str, tags))
}

async fn job_status(client: &AspenClient, args: StatusArgs, is_json_output: bool) -> Result<()> {
    if args.follow {
        // Follow mode: poll every second with an explicit long-lived bound.
        for _poll_attempt in 0..MAX_JOB_STATUS_FOLLOW_POLLS {
            let response = client
                .send(ClientRpcRequest::JobGet {
                    job_id: args.job_id.clone(),
                })
                .await?;

            match response {
                ClientRpcResponse::JobGetResult(result) => {
                    if let Some(job) = result.job {
                        if !is_json_output {
                            // Clear screen and show status
                            print!("\x1B[2J\x1B[1;1H");
                            println!("Job: {}", job.job_id);
                            println!("Status: {}", job.status);
                            println!("Progress: {}%", job.progress);
                            if let Some(msg) = &job.progress_message {
                                println!("Message: {}", msg);
                            }

                            // Check if job is in terminal state
                            let status = job.status.as_str();
                            let is_terminal = matches!(status, "completed" | "failed" | "cancelled");
                            if is_terminal {
                                if let Some(result) = &job.result {
                                    println!("\nResult: {}", serde_json::to_string_pretty(result)?);
                                }
                                if let Some(error) = &job.error_message {
                                    println!("\nError: {}", error);
                                }
                                return Ok(());
                            }
                        } else {
                            // JSON mode: just print the current state
                            println!("{}", serde_json::to_string(&job)?);

                            let status = job.status.as_str();
                            let is_terminal = matches!(status, "completed" | "failed" | "cancelled");
                            if is_terminal {
                                return Ok(());
                            }
                        }
                    } else {
                        if !is_json_output {
                            println!("Job not found: {}", args.job_id);
                        } else {
                            println!("{{\"error\": \"job not found\"}}");
                        }
                        std::process::exit(1);
                    }
                }
                ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
                _ => anyhow::bail!("unexpected response type"),
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        anyhow::bail!("job status follow exceeded {} polls", MAX_JOB_STATUS_FOLLOW_POLLS)
    } else {
        // Single status check
        job_get(client, GetArgs { job_id: args.job_id }, is_json_output).await
    }
}

async fn job_result(client: &AspenClient, args: ResultArgs, is_json_output: bool) -> Result<()> {
    let deadline = job_result_start() + std::time::Duration::from_secs(args.timeout_secs);
    let max_poll_attempts = args.timeout_secs.saturating_add(1);

    debug_assert!(max_poll_attempts >= 1);
    debug_assert!(args.timeout_secs <= max_poll_attempts);

    if !is_json_output {
        println!("Waiting for job {} to complete (timeout: {}s)...", args.job_id, args.timeout_secs);
    }

    // Poll until job completes or timeout.
    for _poll_attempt in 0..max_poll_attempts {
        if has_job_result_timed_out(deadline) {
            anyhow::bail!("Timeout waiting for job {} to complete", args.job_id);
        }

        let response = client
            .send(ClientRpcRequest::JobGet {
                job_id: args.job_id.clone(),
            })
            .await?;

        match response {
            ClientRpcResponse::JobGetResult(result) => {
                if let Some(job) = result.job {
                    match job_result_handle_terminal(&job, is_json_output)? {
                        Some(should_exit) => {
                            if should_exit {
                                std::process::exit(1);
                            }
                            return Ok(());
                        }
                        None => {
                            // Job still running, show progress
                            if !is_json_output && job.progress > 0 {
                                print!("\rProgress: {}%", job.progress);
                                use std::io::Write;
                                std::io::stdout().flush()?;
                            }
                        }
                    }
                } else {
                    anyhow::bail!("Job not found: {}", args.job_id);
                }
            }
            ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
            _ => anyhow::bail!("unexpected response type"),
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    anyhow::bail!("Timeout waiting for job {} to complete", args.job_id)
}

/// Handle a terminal job state (completed/failed/cancelled).
/// Returns Some(should_exit) if terminal, None if still running.
fn job_result_handle_terminal(job: &JobDetails, is_json_output: bool) -> Result<Option<bool>> {
    let status = job.status.as_str();

    if status == "completed" {
        if !is_json_output {
            println!("Job completed successfully!");
            if let Some(result) = &job.result {
                println!("Result: {}", serde_json::to_string_pretty(result)?);
            }
        } else {
            let output = json!({
                "status": "completed",
                "result": job.result,
                "duration": job.completed_at.as_ref()
                    .and_then(|_c| job.started_at.as_ref().map(|_s| "unknown"))
            });
            println!("{}", serde_json::to_string(&output)?);
        }
        return Ok(Some(false));
    }

    if status == "failed" {
        if !is_json_output {
            println!("Job failed!");
            if let Some(error) = &job.error_message {
                println!("Error: {}", error);
            }
        } else {
            let output = json!({
                "status": "failed",
                "error": job.error_message
            });
            println!("{}", serde_json::to_string(&output)?);
        }
        return Ok(Some(true));
    }

    if status == "cancelled" {
        if !is_json_output {
            println!("Job was cancelled");
        } else {
            println!("{{\"status\": \"cancelled\"}}");
        }
        return Ok(Some(true));
    }

    Ok(None)
}
