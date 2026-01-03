//! High-level job management client for the Aspen job queue system.
//!
//! This module provides an ergonomic async API for interacting with the Aspen
//! distributed job queue system, including job submission, scheduling, monitoring,
//! and worker management.
//!
//! The job client is now fully integrated with the RPC layer, enabling
//! job submission and management through the Aspen distributed job queue system.

use crate::AspenClient;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

/// Priority levels for job execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum JobPriority {
    /// Lowest priority.
    Low = 0,
    /// Normal priority (default).
    Normal = 1,
    /// High priority.
    High = 2,
    /// Critical priority (highest).
    Critical = 3,
}

impl Default for JobPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Job status enumeration for filtering and querying.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is waiting to be scheduled.
    Pending,
    /// Job is scheduled and waiting in queue.
    Scheduled,
    /// Job is currently being executed.
    Running,
    /// Job completed successfully.
    Completed,
    /// Job failed after all retry attempts.
    Failed,
    /// Job was cancelled.
    Cancelled,
    /// Job is being retried.
    Retrying,
    /// Job is in the dead letter queue.
    DeadLetter,
}

impl JobStatus {
    /// Convert to string for RPC calls.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Scheduled => "scheduled",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Retrying => "retrying",
            Self::DeadLetter => "deadletter",
        }
    }
}

// Import job types from aspen-client-rpc
use aspen_client_rpc::{
    JobDetails, WorkerInfo as WorkerDetails,
    ClientRpcRequest, ClientRpcResponse,
    JobSubmitResultResponse, JobGetResultResponse, JobListResultResponse,
    JobCancelResultResponse, JobUpdateProgressResultResponse,
    JobQueueStatsResultResponse, WorkerStatusResultResponse,
};

/// Builder for submitting jobs with fluent API.
#[derive(Debug, Clone)]
pub struct JobSubmitBuilder {
    job_type: String,
    payload: Value,
    priority: JobPriority,
    max_retries: Option<u32>,
    timeout: Option<Duration>,
    dependencies: Vec<String>,
    schedule: Option<String>,
    tags: Vec<String>,
}

impl JobSubmitBuilder {
    /// Create a new job submission builder.
    pub fn new(job_type: impl Into<String>, payload: Value) -> Self {
        Self {
            job_type: job_type.into(),
            payload,
            priority: JobPriority::default(),
            max_retries: None,
            timeout: None,
            dependencies: Vec::new(),
            schedule: None,
            tags: Vec::new(),
        }
    }

    /// Set the job priority.
    pub fn with_priority(mut self, priority: JobPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set maximum retry attempts.
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = Some(retries);
        self
    }

    /// Set job execution timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Add job dependencies (job IDs that must complete first).
    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    /// Add a single dependency.
    pub fn add_dependency(mut self, dep: impl Into<String>) -> Self {
        self.dependencies.push(dep.into());
        self
    }

    /// Set a cron schedule for recurring execution.
    pub fn with_cron_schedule(mut self, schedule: impl Into<String>) -> Self {
        self.schedule = Some(schedule.into());
        self
    }

    /// Add tags for job categorization and filtering.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Add a single tag.
    pub fn add_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Options for listing jobs.
#[derive(Debug, Clone, Default)]
pub struct JobListOptions {
    /// Filter by job status.
    pub status: Option<JobStatus>,
    /// Filter by job type.
    pub job_type: Option<String>,
    /// Filter by tags (must have all specified tags).
    pub tags: Vec<String>,
    /// Maximum number of results to return.
    pub limit: Option<u32>,
    /// Continuation token for pagination.
    pub continuation_token: Option<String>,
}

/// High-level job management client.
pub struct JobClient<'a> {
    client: &'a AspenClient,
}

impl<'a> JobClient<'a> {
    /// Create a new job client wrapping an Aspen client.
    pub fn new(client: &'a AspenClient) -> Self {
        Self { client }
    }

    /// Submit a new job using the builder pattern.
    ///
    /// # Example
    /// ```rust,ignore
    /// let job_id = client.jobs()
    ///     .submit("data_processing", json!({ "input": "data.csv" }))
    ///     .with_priority(JobPriority::High)
    ///     .with_max_retries(3)
    ///     .with_timeout(Duration::from_secs(300))
    ///     .add_tag("batch")
    ///     .execute()
    ///     .await?;
    /// ```
    pub fn submit(&self, job_type: impl Into<String>, payload: Value) -> JobSubmitBuilder {
        JobSubmitBuilder::new(job_type, payload)
    }

    /// Execute a job submission.
    pub async fn submit_job(&self, builder: JobSubmitBuilder) -> Result<String> {
        // Serialize payload to JSON string for postcard compatibility
        let payload_str = serde_json::to_string(&builder.payload)
            .context("Failed to serialize job payload")?;

        let request = ClientRpcRequest::JobSubmit {
            job_type: builder.job_type,
            payload: payload_str,
            priority: Some(builder.priority as u8),
            timeout_ms: builder.timeout.map(|d| d.as_millis() as u64),
            max_retries: builder.max_retries,
            retry_delay_ms: Some(1000), // Default 1 second retry delay
            schedule: builder.schedule,
            tags: builder.tags,
        };

        let response = self.client.send(request).await?;

        match response {
            ClientRpcResponse::JobSubmitResult(result) => {
                if result.success {
                    result
                        .job_id
                        .context("Job submitted successfully but no ID returned")
                } else {
                    Err(anyhow::anyhow!(
                        "Failed to submit job: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    ))
                }
            }
            _ => Err(anyhow::anyhow!("Unexpected response type for job submit")),
        }
    }

    /// Get details for a specific job.
    pub async fn get(&self, job_id: impl Into<String>) -> Result<Option<JobDetails>> {

        let response = self
            .client
            .send(ClientRpcRequest::JobGet {
                job_id: job_id.into(),
            })
            .await?;

        match response {
            ClientRpcResponse::JobGetResult(result) => {
                if let Some(error) = result.error {
                    Err(anyhow::anyhow!("Failed to get job: {}", error))
                } else {
                    Ok(result.job)
                }
            }
            _ => Err(anyhow::anyhow!("Unexpected response type for job get")),
        }
    }

    /// List jobs with optional filtering.
    pub async fn list(&self, options: JobListOptions) -> Result<JobListResult> {

        let response = self
            .client
            .send(ClientRpcRequest::JobList {
                status: options.status.map(|s| s.as_str().to_string()),
                job_type: options.job_type,
                tags: options.tags,
                limit: options.limit,
                continuation_token: options.continuation_token,
            })
            .await?;

        match response {
            ClientRpcResponse::JobListResult(result) => {
                if let Some(error) = result.error {
                    Err(anyhow::anyhow!("Failed to list jobs: {}", error))
                } else {
                    Ok(JobListResult {
                        jobs: result.jobs,
                        total_count: result.total_count,
                        continuation_token: result.continuation_token,
                    })
                }
            }
            _ => Err(anyhow::anyhow!("Unexpected response type for job list")),
        }
    }

    /// Cancel a job with optional reason.
    pub async fn cancel(
        &self,
        job_id: impl Into<String>,
        reason: Option<String>,
    ) -> Result<String> {

        let response = self
            .client
            .send(ClientRpcRequest::JobCancel {
                job_id: job_id.into(),
                reason,
            })
            .await?;

        match response {
            ClientRpcResponse::JobCancelResult(result) => {
                if result.success {
                    Ok(result
                        .previous_status
                        .unwrap_or_else(|| "unknown".to_string()))
                } else {
                    Err(anyhow::anyhow!(
                        "Failed to cancel job: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    ))
                }
            }
            _ => Err(anyhow::anyhow!("Unexpected response type for job cancel")),
        }
    }

    /// Update job progress (for workers).
    pub async fn update_progress(
        &self,
        job_id: impl Into<String>,
        progress: u8,
        message: Option<String>,
    ) -> Result<()> {

        let response = self
            .client
            .send(ClientRpcRequest::JobUpdateProgress {
                job_id: job_id.into(),
                progress,
                message,
            })
            .await?;

        match response {
            ClientRpcResponse::JobUpdateProgressResult(result) => {
                if result.success {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Failed to update job progress: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    ))
                }
            }
            _ => Err(anyhow::anyhow!(
                "Unexpected response type for job progress update"
            )),
        }
    }

    /// Get job queue statistics.
    pub async fn get_queue_stats(&self) -> Result<JobQueueStats> {

        let response = self
            .client
            .send(ClientRpcRequest::JobQueueStats)
            .await?;

        match response {
            ClientRpcResponse::JobQueueStatsResult(result) => {
                if let Some(error) = result.error {
                    Err(anyhow::anyhow!("Failed to get queue stats: {}", error))
                } else {
                    Ok(JobQueueStats {
                        pending_count: result.pending_count,
                        scheduled_count: result.scheduled_count,
                        running_count: result.running_count,
                        completed_count: result.completed_count,
                        failed_count: result.failed_count,
                        cancelled_count: result.cancelled_count,
                        priority_counts: result
                            .priority_counts
                            .into_iter()
                            .map(|pc| (pc.priority, pc.count))
                            .collect(),
                        type_counts: result
                            .type_counts
                            .into_iter()
                            .map(|tc| (tc.job_type, tc.count))
                            .collect(),
                    })
                }
            }
            _ => Err(anyhow::anyhow!(
                "Unexpected response type for queue stats"
            )),
        }
    }

    /// Get worker pool status.
    pub async fn get_worker_status(&self) -> Result<Vec<WorkerDetails>> {

        let response = self.client.send(ClientRpcRequest::WorkerStatus).await?;

        match response {
            ClientRpcResponse::WorkerStatusResult(result) => {
                if let Some(error) = result.error {
                    Err(anyhow::anyhow!("Failed to get worker status: {}", error))
                } else {
                    Ok(result.workers)
                }
            }
            _ => Err(anyhow::anyhow!(
                "Unexpected response type for worker status"
            )),
        }
    }

    /// Wait for a job to reach a terminal state (completed, failed, cancelled).
    pub async fn wait_for_completion(
        &self,
        job_id: impl Into<String>,
        poll_interval: Duration,
        timeout: Option<Duration>,
    ) -> Result<JobDetails> {
        let job_id = job_id.into();
        let start = std::time::Instant::now();

        loop {
            if let Some(job) = self.get(&job_id).await? {
                let status = job.status.as_str();
                if status == "completed" || status == "failed" || status == "cancelled" {
                    return Ok(job);
                }
            }

            if let Some(timeout) = timeout {
                if start.elapsed() > timeout {
                    return Err(anyhow::anyhow!(
                        "Timeout waiting for job {} to complete",
                        job_id
                    ));
                }
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Submit and wait for a job to complete.
    pub async fn submit_and_wait(
        &self,
        builder: JobSubmitBuilder,
        poll_interval: Duration,
        timeout: Option<Duration>,
    ) -> Result<JobDetails> {
        let job_id = self.submit_job(builder).await?;
        self.wait_for_completion(job_id, poll_interval, timeout)
            .await
    }
}

/// Result of a job list operation.
#[derive(Debug, Clone)]
pub struct JobListResult {
    /// List of jobs.
    pub jobs: Vec<JobDetails>,
    /// Total count of matching jobs.
    pub total_count: u32,
    /// Continuation token for pagination.
    pub continuation_token: Option<String>,
}

/// Job queue statistics.
#[derive(Debug, Clone)]
pub struct JobQueueStats {
    /// Number of pending jobs.
    pub pending_count: u64,
    /// Number of scheduled jobs.
    pub scheduled_count: u64,
    /// Number of running jobs.
    pub running_count: u64,
    /// Number of completed jobs (recent).
    pub completed_count: u64,
    /// Number of failed jobs (recent).
    pub failed_count: u64,
    /// Number of cancelled jobs (recent).
    pub cancelled_count: u64,
    /// Jobs per priority level.
    pub priority_counts: HashMap<u8, u64>,
    /// Jobs per type.
    pub type_counts: HashMap<String, u64>,
}


/// Extension trait to add job management to AspenClient.
pub trait AspenClientJobExt {
    /// Get a job management client.
    fn jobs(&self) -> JobClient<'_>;
}

impl AspenClientJobExt for AspenClient {
    fn jobs(&self) -> JobClient<'_> {
        JobClient::new(self)
    }
}

/// Helper to execute a JobSubmitBuilder.
impl JobSubmitBuilder {
    /// Execute the job submission.
    pub async fn execute(self, client: &AspenClient) -> Result<String> {
        client.jobs().submit_job(self).await
    }
}