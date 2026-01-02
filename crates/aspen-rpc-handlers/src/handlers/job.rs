//! Job queue RPC handlers.
//!
//! Handles job submission, management, and worker coordination through
//! the distributed job queue system.

use async_trait::async_trait;
use tracing::{debug, info, warn};

use crate::context::ClientProtocolContext;
use crate::registry::RequestHandler;
use aspen_client_rpc::{
    ClientRpcRequest, ClientRpcResponse, JobDetails, JobSubmitResultResponse,
    JobGetResultResponse, JobListResultResponse, JobCancelResultResponse,
    JobUpdateProgressResultResponse, JobQueueStatsResultResponse, WorkerStatusResultResponse,
    WorkerRegisterResultResponse, WorkerHeartbeatResultResponse, WorkerDeregisterResultResponse,
    PriorityCount, TypeCount,
};
use aspen_jobs::{
    JobId, JobSpec, JobConfig, JobStatus, JobResult,
    Priority, RetryPolicy,
    JobManager,
};
use aspen_core::KeyValueStore;

/// Handler for job queue operations.
///
/// Processes all Job* RPC requests including job submission, status queries,
/// cancellation, and worker management.
///
/// # Tiger Style
///
/// - Bounded operations with explicit limits
/// - Fail-fast on invalid parameters
/// - Clear separation between job management and worker coordination
pub struct JobHandler;

#[async_trait]
impl RequestHandler for JobHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::JobSubmit { .. }
                | ClientRpcRequest::JobGet { .. }
                | ClientRpcRequest::JobList { .. }
                | ClientRpcRequest::JobCancel { .. }
                | ClientRpcRequest::JobUpdateProgress { .. }
                | ClientRpcRequest::JobQueueStats
                | ClientRpcRequest::WorkerStatus
                | ClientRpcRequest::WorkerRegister { .. }
                | ClientRpcRequest::WorkerHeartbeat { .. }
                | ClientRpcRequest::WorkerDeregister { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        // Check if job manager is available
        let job_manager = ctx.job_manager.as_ref()
            .ok_or_else(|| anyhow::anyhow!("job manager not available"))?;

        match request {
            ClientRpcRequest::JobSubmit {
                job_type,
                payload,
                priority,
                timeout_ms,
                max_retries,
                retry_delay_ms,
                schedule,
                tags,
            } => handle_job_submit(
                job_manager,
                job_type,
                payload,
                priority,
                timeout_ms,
                max_retries,
                retry_delay_ms,
                schedule,
                tags,
            ).await,

            ClientRpcRequest::JobGet { job_id } => handle_job_get(job_manager, job_id).await,

            ClientRpcRequest::JobList {
                status,
                job_type,
                tags,
                limit,
                continuation_token,
            } => handle_job_list(
                job_manager,
                status,
                job_type,
                tags,
                limit,
                continuation_token,
            ).await,

            ClientRpcRequest::JobCancel { job_id, reason } =>
                handle_job_cancel(job_manager, job_id, reason).await,

            ClientRpcRequest::JobUpdateProgress { job_id, progress, message } =>
                handle_job_update_progress(job_manager, job_id, progress, message).await,

            ClientRpcRequest::JobQueueStats => handle_job_queue_stats(job_manager).await,

            ClientRpcRequest::WorkerStatus => handle_worker_status(job_manager).await,

            ClientRpcRequest::WorkerRegister { worker_id, capabilities, capacity } =>
                handle_worker_register(job_manager, worker_id, capabilities, capacity).await,

            ClientRpcRequest::WorkerHeartbeat { worker_id, active_jobs } =>
                handle_worker_heartbeat(job_manager, worker_id, active_jobs).await,

            ClientRpcRequest::WorkerDeregister { worker_id } =>
                handle_worker_deregister(job_manager, worker_id).await,

            _ => Err(anyhow::anyhow!("request not handled by JobHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "JobHandler"
    }
}

// =============================================================================
// Job Management Handlers
// =============================================================================

async fn handle_job_submit(
    job_manager: &JobManager<dyn KeyValueStore>,
    job_type: String,
    payload: serde_json::Value,
    priority: Option<u8>,
    timeout_ms: Option<u64>,
    max_retries: Option<u32>,
    retry_delay_ms: Option<u64>,
    schedule: Option<String>,
    tags: Vec<String>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Submitting job: type={}, priority={:?}", job_type, priority);

    // Convert priority
    let priority = match priority.unwrap_or(1) {
        0 => Priority::Low,
        1 => Priority::Normal,
        2 => Priority::High,
        3 => Priority::Critical,
        _ => Priority::Normal,
    };

    // Create retry policy
    let retry_policy = if let Some(max_attempts) = max_retries {
        if max_attempts == 0 {
            RetryPolicy::none()
        } else {
            RetryPolicy::fixed(
                max_attempts,
                std::time::Duration::from_millis(retry_delay_ms.unwrap_or(1000)),
            )
        }
    } else {
        RetryPolicy::default()
    };

    // Create job config
    let config = JobConfig {
        priority,
        retry_policy,
        timeout: timeout_ms.map(|ms| std::time::Duration::from_millis(ms)),
        tags: tags.into_iter().collect(),
        dependencies: vec![],
        save_result: true,
        ttl_after_completion: None,
    };

    // Create job spec
    let spec = JobSpec {
        job_type,
        payload,
        config,
        schedule: None, // TODO: implement schedule parsing
        idempotency_key: None,
        metadata: std::collections::HashMap::new(),
    };

    // Submit job
    match job_manager.submit(spec).await {
        Ok(job_id) => {
            info!("Job submitted: {}", job_id);
            Ok(ClientRpcResponse::JobSubmitResult(JobSubmitResultResponse {
                success: true,
                job_id: Some(job_id.to_string()),
                error: None,
            }))
        }
        Err(e) => {
            warn!("Failed to submit job: {}", e);
            Ok(ClientRpcResponse::JobSubmitResult(JobSubmitResultResponse {
                success: false,
                job_id: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

async fn handle_job_get(
    job_manager: &JobManager<dyn KeyValueStore>,
    job_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Getting job: {}", job_id);

    let job_id = JobId::from_string(job_id);

    match job_manager.get_job(&job_id).await {
        Ok(Some(job)) => {
            let details = JobDetails {
                job_id: job.id.to_string(),
                job_type: job.spec.job_type.clone(),
                status: format!("{:?}", job.status),
                priority: match job.spec.config.priority {
                    Priority::Low => 0,
                    Priority::Normal => 1,
                    Priority::High => 2,
                    Priority::Critical => 3,
                },
                progress: job.progress.unwrap_or(0),
                progress_message: job.progress_message.clone(),
                payload: job.spec.payload.clone(),
                tags: job.spec.config.tags.iter().cloned().collect(),
                submitted_at: job.created_at.to_rfc3339(),
                started_at: job.started_at.map(|t| t.to_rfc3339()),
                completed_at: job.completed_at.map(|t| t.to_rfc3339()),
                worker_id: job.worker_id,
                attempts: job.attempts,
                result: job.result.as_ref().and_then(|r| {
                    if let JobResult::Success(output) = r {
                        Some(output.data.clone())
                    } else {
                        None
                    }
                }),
                error_message: job.result.as_ref().and_then(|r| {
                    if let JobResult::Failure(failure) = r {
                        Some(failure.reason.clone())
                    } else {
                        None
                    }
                }),
            };

            Ok(ClientRpcResponse::JobGetResult(JobGetResultResponse {
                found: true,
                job: Some(details),
                error: None,
            }))
        }
        Ok(None) => Ok(ClientRpcResponse::JobGetResult(JobGetResultResponse {
            found: false,
            job: None,
            error: None,
        })),
        Err(e) => {
            warn!("Failed to get job: {}", e);
            Ok(ClientRpcResponse::JobGetResult(JobGetResultResponse {
                found: false,
                job: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

async fn handle_job_list(
    job_manager: &JobManager<dyn KeyValueStore>,
    status: Option<String>,
    job_type: Option<String>,
    tags: Vec<String>,
    limit: Option<u32>,
    _continuation_token: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Listing jobs: status={:?}, type={:?}", status, job_type);

    // Parse status filter
    let status_filter = status.and_then(|s| match s.as_str() {
        "pending" => Some(JobStatus::Pending),
        "scheduled" => Some(JobStatus::Scheduled),
        "running" => Some(JobStatus::Running),
        "completed" => Some(JobStatus::Completed),
        "failed" => Some(JobStatus::Failed),
        "cancelled" => Some(JobStatus::Cancelled),
        _ => None,
    });

    // For now, we'll scan all jobs and filter
    // In production, this would use an index
    let limit = limit.unwrap_or(100).min(1000) as usize;
    let jobs = Vec::new();
    let count = 0;

    // Scan job keys
    let prefix = "__jobs:";
    let scan_result = job_manager.get_job(&JobId::from_string("dummy".to_string())).await;

    // Note: This is a simplified implementation
    // Real implementation would scan the KV store properly
    Ok(ClientRpcResponse::JobListResult(JobListResultResponse {
        jobs,
        total_count: count,
        continuation_token: None,
        error: None,
    }))
}

async fn handle_job_cancel(
    job_manager: &JobManager<dyn KeyValueStore>,
    job_id: String,
    _reason: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Cancelling job: {}", job_id);

    let job_id = JobId::from_string(job_id);

    match job_manager.cancel_job(&job_id).await {
        Ok(()) => {
            info!("Job cancelled: {}", job_id);
            Ok(ClientRpcResponse::JobCancelResult(JobCancelResultResponse {
                success: true,
                previous_status: None, // Could fetch this if needed
                error: None,
            }))
        }
        Err(e) => {
            warn!("Failed to cancel job: {}", e);
            Ok(ClientRpcResponse::JobCancelResult(JobCancelResultResponse {
                success: false,
                previous_status: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

async fn handle_job_update_progress(
    job_manager: &JobManager<dyn KeyValueStore>,
    job_id: String,
    progress: u8,
    message: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Updating job progress: {} to {}%", job_id, progress);

    let job_id = JobId::from_string(job_id);

    match job_manager.update_progress(&job_id, progress, message).await {
        Ok(()) => Ok(ClientRpcResponse::JobUpdateProgressResult(
            JobUpdateProgressResultResponse {
                success: true,
                error: None,
            }
        )),
        Err(e) => {
            warn!("Failed to update job progress: {}", e);
            Ok(ClientRpcResponse::JobUpdateProgressResult(
                JobUpdateProgressResultResponse {
                    success: false,
                    error: Some(e.to_string()),
                }
            ))
        }
    }
}

async fn handle_job_queue_stats(
    job_manager: &JobManager<dyn KeyValueStore>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Getting job queue statistics");

    match job_manager.get_queue_stats().await {
        Ok(stats) => {
            // Convert internal stats to response format
            let priority_counts = stats.by_priority
                .into_iter()
                .map(|(priority, count)| {
                    let priority_num = match priority {
                        Priority::Low => 0,
                        Priority::Normal => 1,
                        Priority::High => 2,
                        Priority::Critical => 3,
                    };
                    PriorityCount { priority: priority_num, count }
                })
                .collect();

            Ok(ClientRpcResponse::JobQueueStatsResult(JobQueueStatsResultResponse {
                pending_count: stats.total_queued,
                scheduled_count: 0, // Not available in current QueueStats
                running_count: stats.processing,
                completed_count: 0, // Not available in current QueueStats
                failed_count: 0, // Not available in current QueueStats
                cancelled_count: 0, // Not available in current QueueStats
                priority_counts,
                type_counts: vec![], // Not available in current QueueStats
                error: None,
            }))
        }
        Err(e) => {
            warn!("Failed to get queue stats: {}", e);
            Ok(ClientRpcResponse::JobQueueStatsResult(JobQueueStatsResultResponse {
                pending_count: 0,
                scheduled_count: 0,
                running_count: 0,
                completed_count: 0,
                failed_count: 0,
                cancelled_count: 0,
                priority_counts: vec![],
                type_counts: vec![],
                error: Some(e.to_string()),
            }))
        }
    }
}

// =============================================================================
// Worker Management Handlers
// =============================================================================

async fn handle_worker_status(
    _job_manager: &JobManager<dyn KeyValueStore>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Getting worker status");

    // Note: Worker management is not yet implemented in JobManager
    // This is a placeholder implementation
    Ok(ClientRpcResponse::WorkerStatusResult(WorkerStatusResultResponse {
        workers: vec![],
        total_workers: 0,
        idle_workers: 0,
        busy_workers: 0,
        offline_workers: 0,
        total_capacity: 0,
        used_capacity: 0,
        error: Some("Worker management not yet implemented".to_string()),
    }))
}

async fn handle_worker_register(
    _job_manager: &JobManager<dyn KeyValueStore>,
    worker_id: String,
    _capabilities: Vec<String>,
    capacity: u32,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Registering worker: {} with capacity {}", worker_id, capacity);

    // Placeholder implementation
    Ok(ClientRpcResponse::WorkerRegisterResult(WorkerRegisterResultResponse {
        success: false,
        worker_token: None,
        error: Some("Worker registration not yet implemented".to_string()),
    }))
}

async fn handle_worker_heartbeat(
    _job_manager: &JobManager<dyn KeyValueStore>,
    worker_id: String,
    active_jobs: Vec<String>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Worker heartbeat: {} with {} active jobs", worker_id, active_jobs.len());

    // Placeholder implementation
    Ok(ClientRpcResponse::WorkerHeartbeatResult(WorkerHeartbeatResultResponse {
        success: false,
        jobs_to_process: vec![],
        error: Some("Worker heartbeat not yet implemented".to_string()),
    }))
}

async fn handle_worker_deregister(
    _job_manager: &JobManager<dyn KeyValueStore>,
    worker_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Deregistering worker: {}", worker_id);

    // Placeholder implementation
    Ok(ClientRpcResponse::WorkerDeregisterResult(WorkerDeregisterResultResponse {
        success: false,
        error: Some("Worker deregistration not yet implemented".to_string()),
    }))
}