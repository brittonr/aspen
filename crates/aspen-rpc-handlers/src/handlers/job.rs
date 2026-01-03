//! Job queue RPC handlers.
//!
//! Handles job submission, management, and worker coordination through
//! the distributed job queue system.

use async_trait::async_trait;
use std::sync::Arc;
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
                &ctx.kv_store,
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

            ClientRpcRequest::WorkerStatus => handle_worker_status(ctx.worker_service.as_ref()).await,

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
    payload_str: String,
    priority: Option<u8>,
    timeout_ms: Option<u64>,
    max_retries: Option<u32>,
    retry_delay_ms: Option<u64>,
    _schedule: Option<String>,
    tags: Vec<String>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Submitting job: type={}, priority={:?}", job_type, priority);

    // Parse the JSON payload string
    let payload: serde_json::Value = serde_json::from_str(&payload_str)
        .map_err(|e| anyhow::anyhow!("invalid JSON payload: {}", e))?;

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
                payload: serde_json::to_string(&job.spec.payload).unwrap_or_default(),
                tags: job.spec.config.tags.iter().cloned().collect(),
                submitted_at: job.created_at.to_rfc3339(),
                started_at: job.started_at.map(|t| t.to_rfc3339()),
                completed_at: job.completed_at.map(|t| t.to_rfc3339()),
                worker_id: job.worker_id,
                attempts: job.attempts,
                result: job.result.as_ref().and_then(|r| {
                    if let JobResult::Success(output) = r {
                        serde_json::to_string(&output.data).ok()
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
    kv_store: &Arc<dyn KeyValueStore>,
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

    let limit = limit.unwrap_or(100).min(1000) as usize;
    let mut jobs: Vec<JobDetails> = Vec::new();

    // Scan job keys from the store
    let prefix = "__jobs:";

    // Use the KV store scan to find all job keys
    match kv_store.scan(aspen_core::ScanRequest {
        prefix: prefix.to_string(),
        limit: Some(limit as u32),
        continuation_token: None,
    }).await {
        Ok(scan_result) => {
            for entry in scan_result.entries.iter() {
                // Extract job ID from key (format: __jobs:<job_id>)
                if let Some(job_id_str) = entry.key.strip_prefix(prefix) {
                    let job_id = JobId::from_string(job_id_str.to_string());

                    // Fetch the full job details
                    if let Ok(Some(job)) = job_manager.get_job(&job_id).await {
                        // Apply filters
                        if let Some(ref status_filter) = status_filter {
                            if job.status != *status_filter {
                                continue;
                            }
                        }

                        if let Some(ref type_filter) = job_type {
                            if job.spec.job_type != *type_filter {
                                continue;
                            }
                        }

                        if !tags.is_empty() {
                            let has_all_tags = tags.iter().all(|tag|
                                job.spec.config.tags.contains(&tag.to_string())
                            );
                            if !has_all_tags {
                                continue;
                            }
                        }

                        // Convert to JobDetails for response
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
                            payload: serde_json::to_string(&job.spec.payload).unwrap_or_default(),
                            tags: job.spec.config.tags.iter().cloned().collect(),
                            submitted_at: job.created_at.to_rfc3339(),
                            started_at: job.started_at.map(|t| t.to_rfc3339()),
                            completed_at: job.completed_at.map(|t| t.to_rfc3339()),
                            worker_id: job.worker_id,
                            attempts: job.attempts,
                            result: job.result.as_ref().and_then(|r| {
                                if let JobResult::Success(output) = r {
                                    serde_json::to_string(&output.data).ok()
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

                        jobs.push(details);

                        if jobs.len() >= limit {
                            break;
                        }
                    }
                }
            }

            Ok(ClientRpcResponse::JobListResult(JobListResultResponse {
                total_count: jobs.len() as u32,
                jobs,
                continuation_token: scan_result.continuation_token,
                error: None,
            }))
        }
        Err(e) => {
            warn!("Failed to list jobs: {}", e);
            Ok(ClientRpcResponse::JobListResult(JobListResultResponse {
                jobs: vec![],
                total_count: 0,
                continuation_token: None,
                error: Some(e.to_string()),
            }))
        }
    }
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
    worker_service: Option<&std::sync::Arc<aspen_cluster::worker_service::WorkerService>>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Getting worker status");

    let Some(service) = worker_service else {
        return Ok(ClientRpcResponse::WorkerStatusResult(WorkerStatusResultResponse {
            workers: vec![],
            total_workers: 0,
            idle_workers: 0,
            busy_workers: 0,
            offline_workers: 0,
            total_capacity: 0,
            used_capacity: 0,
            error: Some("Worker service not available".to_string()),
        }));
    };

    // Get stats and worker info from the service
    let stats = service.get_stats().await;
    let worker_info = service.get_worker_info().await;

    // Convert aspen_jobs::WorkerInfo to aspen_client_rpc::WorkerInfo
    let workers: Vec<aspen_client_rpc::WorkerInfo> = worker_info
        .into_iter()
        .map(|w| {
            let status = match w.status {
                aspen_jobs::WorkerStatus::Starting => "starting",
                aspen_jobs::WorkerStatus::Idle => "idle",
                aspen_jobs::WorkerStatus::Processing => "busy",
                aspen_jobs::WorkerStatus::Stopping => "stopping",
                aspen_jobs::WorkerStatus::Stopped => "offline",
                aspen_jobs::WorkerStatus::Failed(_) => "failed",
            };
            aspen_client_rpc::WorkerInfo {
                worker_id: w.id,
                status: status.to_string(),
                capabilities: w.job_types,
                capacity: 1, // Each worker has capacity of 1 concurrent job by default
                active_jobs: if w.current_job.is_some() { 1 } else { 0 },
                active_job_ids: w.current_job.into_iter().collect(),
                last_heartbeat: w.last_heartbeat.to_rfc3339(),
                total_processed: w.jobs_processed,
                total_failed: w.jobs_failed,
            }
        })
        .collect();

    Ok(ClientRpcResponse::WorkerStatusResult(WorkerStatusResultResponse {
        workers,
        total_workers: stats.total_workers as u32,
        idle_workers: stats.idle_workers as u32,
        busy_workers: stats.processing_workers as u32,
        offline_workers: stats.failed_workers as u32,
        total_capacity: stats.total_workers as u32, // 1 job per worker
        used_capacity: stats.processing_workers as u32,
        error: None,
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
