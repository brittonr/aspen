//! Job service executor for typed RPC dispatch.
//!
//! Implements `ServiceExecutor` from aspen-rpc-core to handle job and
//! worker operations via the `ServiceHandler` wrapper.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_cluster::worker_service::WorkerService;
use aspen_coordination::DistributedWorkerCoordinator;
use aspen_coordination::WorkerInfo as CoordWorkerInfo;
use aspen_coordination::WorkerStats;
use aspen_core::KeyValueStore;
use aspen_jobs::JobConfig;
use aspen_jobs::JobId;
use aspen_jobs::JobManager;
use aspen_jobs::JobResult;
use aspen_jobs::JobSpec;
use aspen_jobs::JobStatus;
use aspen_jobs::Priority;
use aspen_jobs::RetryPolicy;
use aspen_jobs_protocol::*;
use aspen_rpc_core::ServiceExecutor;
use async_trait::async_trait;
use tracing::debug;
use tracing::warn;

/// Service executor for job queue operations.
pub struct JobServiceExecutor {
    job_manager: Arc<JobManager<dyn KeyValueStore>>,
    kv_store: Arc<dyn KeyValueStore>,
    worker_service: Option<Arc<WorkerService>>,
    worker_coordinator: Option<Arc<DistributedWorkerCoordinator<dyn KeyValueStore>>>,
    node_id: u64,
}

impl JobServiceExecutor {
    /// Create a new job service executor.
    pub fn new(
        job_manager: Arc<JobManager<dyn KeyValueStore>>,
        kv_store: Arc<dyn KeyValueStore>,
        worker_service: Option<Arc<WorkerService>>,
        worker_coordinator: Option<Arc<DistributedWorkerCoordinator<dyn KeyValueStore>>>,
        node_id: u64,
    ) -> Self {
        Self {
            job_manager,
            kv_store,
            worker_service,
            worker_coordinator,
            node_id,
        }
    }
}

#[async_trait]
impl ServiceExecutor for JobServiceExecutor {
    fn service_name(&self) -> &'static str {
        "jobs"
    }

    fn handles(&self) -> &'static [&'static str] {
        &[
            "JobSubmit",
            "JobGet",
            "JobList",
            "JobCancel",
            "JobUpdateProgress",
            "JobQueueStats",
            "WorkerStatus",
            "WorkerRegister",
            "WorkerHeartbeat",
            "WorkerDeregister",
        ]
    }

    fn priority(&self) -> u32 {
        560
    }

    fn app_id(&self) -> Option<&'static str> {
        Some("jobs")
    }

    async fn execute(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
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
            } => {
                self.handle_submit(job_type, payload, priority, timeout_ms, max_retries, retry_delay_ms, schedule, tags)
                    .await
            }
            ClientRpcRequest::JobGet { job_id } => self.handle_get(job_id).await,
            ClientRpcRequest::JobList {
                status,
                job_type,
                tags,
                limit,
                continuation_token: _,
            } => self.handle_list(status, job_type, tags, limit).await,
            ClientRpcRequest::JobCancel { job_id, reason: _ } => self.handle_cancel(job_id).await,
            ClientRpcRequest::JobUpdateProgress {
                job_id,
                progress,
                message,
            } => self.handle_update_progress(job_id, progress, message).await,
            ClientRpcRequest::JobQueueStats => self.handle_queue_stats().await,
            ClientRpcRequest::WorkerStatus => self.handle_worker_status().await,
            ClientRpcRequest::WorkerRegister {
                worker_id,
                capabilities,
                capacity_jobs,
            } => self.handle_worker_register(worker_id, capabilities, capacity_jobs).await,
            ClientRpcRequest::WorkerHeartbeat { worker_id, active_jobs } => {
                self.handle_worker_heartbeat(worker_id, active_jobs).await
            }
            ClientRpcRequest::WorkerDeregister { worker_id } => self.handle_worker_deregister(worker_id).await,
            _ => anyhow::bail!("unhandled request for jobs service"),
        }
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn job_status_to_string(status: &JobStatus) -> String {
    match status {
        JobStatus::Pending => "pending".to_string(),
        JobStatus::Scheduled => "scheduled".to_string(),
        JobStatus::Running => "running".to_string(),
        JobStatus::Completed => "completed".to_string(),
        JobStatus::Failed => "failed".to_string(),
        JobStatus::Cancelled => "cancelled".to_string(),
        JobStatus::Retrying => "retrying".to_string(),
        JobStatus::DeadLetter => "dead_letter".to_string(),
        JobStatus::Unknown => "unknown".to_string(),
    }
}

fn priority_to_u8(p: &Priority) -> u8 {
    match p {
        Priority::Low => 0,
        Priority::Normal => 1,
        Priority::High => 2,
        Priority::Critical => 3,
    }
}

fn job_to_details(job: &aspen_jobs::Job) -> JobDetails {
    JobDetails {
        job_id: job.id.to_string(),
        job_type: job.spec.job_type.clone(),
        status: job_status_to_string(&job.status),
        priority: priority_to_u8(&job.spec.config.priority),
        progress: job.progress.unwrap_or(0),
        progress_message: job.progress_message.clone(),
        payload: serde_json::to_string(&job.spec.payload).unwrap_or_default(),
        tags: job.spec.config.tags.to_vec(),
        submitted_at: job.created_at.to_rfc3339(),
        started_at: job.started_at.map(|t| t.to_rfc3339()),
        completed_at: job.completed_at.map(|t| t.to_rfc3339()),
        worker_id: job.worker_id.clone(),
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
    }
}

// =============================================================================
// Job operations
// =============================================================================

impl JobServiceExecutor {
    #[allow(clippy::too_many_arguments)]
    async fn handle_submit(
        &self,
        job_type: String,
        payload_str: String,
        priority_opt: Option<u8>,
        timeout_ms: Option<u64>,
        max_retries: Option<u32>,
        retry_delay_ms: Option<u64>,
        schedule: Option<String>,
        tags: Vec<String>,
    ) -> Result<ClientRpcResponse> {
        debug!("Submitting job: type={}", job_type);

        let payload: serde_json::Value =
            serde_json::from_str(&payload_str).map_err(|e| anyhow::anyhow!("invalid JSON payload: {e}"))?;

        let priority = match priority_opt.unwrap_or(1) {
            0 => Priority::Low,
            2 => Priority::High,
            3 => Priority::Critical,
            _ => Priority::Normal,
        };

        let retry_policy = match max_retries {
            Some(0) => RetryPolicy::none(),
            Some(n) => RetryPolicy::fixed(n, std::time::Duration::from_millis(retry_delay_ms.unwrap_or(1000))),
            None => RetryPolicy::default(),
        };

        let timeout = timeout_ms.map(std::time::Duration::from_millis);

        let parsed_schedule = match schedule {
            Some(s) => Some(aspen_jobs::parse_schedule(&s).map_err(|e| anyhow::anyhow!("invalid schedule: {e}"))?),
            None => None,
        };

        let spec = JobSpec {
            job_type,
            payload,
            config: JobConfig {
                priority,
                retry_policy,
                timeout,
                tags: tags.into_iter().collect(),
                dependencies: vec![],
                save_result: true,
                ttl_after_completion: None,
            },
            schedule: parsed_schedule,
            idempotency_key: None,
            metadata: HashMap::new(),
        };

        match self.job_manager.submit(spec).await {
            Ok(job_id) => Ok(ClientRpcResponse::JobSubmitResult(JobSubmitResultResponse {
                is_success: true,
                job_id: Some(job_id.to_string()),
                error: None,
            })),
            Err(e) => {
                warn!("Failed to submit job: {e}");
                Ok(ClientRpcResponse::JobSubmitResult(JobSubmitResultResponse {
                    is_success: false,
                    job_id: None,
                    error: Some(e.to_string()),
                }))
            }
        }
    }

    async fn handle_get(&self, job_id_str: String) -> Result<ClientRpcResponse> {
        let job_id = JobId::from_string(job_id_str);

        match self.job_manager.get_job(&job_id).await {
            Ok(Some(job)) => Ok(ClientRpcResponse::JobGetResult(JobGetResultResponse {
                was_found: true,
                job: Some(job_to_details(&job)),
                error: None,
            })),
            Ok(None) => Ok(ClientRpcResponse::JobGetResult(JobGetResultResponse {
                was_found: false,
                job: None,
                error: None,
            })),
            Err(e) => {
                warn!("Failed to get job: {e}");
                Ok(ClientRpcResponse::JobGetResult(JobGetResultResponse {
                    was_found: false,
                    job: None,
                    error: Some(e.to_string()),
                }))
            }
        }
    }

    async fn handle_list(
        &self,
        status_str: Option<String>,
        job_type: Option<String>,
        tags: Vec<String>,
        limit_opt: Option<u32>,
    ) -> Result<ClientRpcResponse> {
        let limit = limit_opt.unwrap_or(100).min(1000) as usize;

        let status_filter = status_str.as_deref().and_then(|s| match s {
            "pending" => Some(JobStatus::Pending),
            "scheduled" => Some(JobStatus::Scheduled),
            "running" => Some(JobStatus::Running),
            "completed" => Some(JobStatus::Completed),
            "failed" => Some(JobStatus::Failed),
            "cancelled" => Some(JobStatus::Cancelled),
            _ => None,
        });

        let prefix = "__jobs:";
        match self
            .kv_store
            .scan(aspen_core::ScanRequest {
                prefix: prefix.to_string(),
                limit_results: Some(limit as u32),
                continuation_token: None,
            })
            .await
        {
            Ok(scan_result) => {
                let mut jobs = Vec::new();

                for entry in scan_result.entries.iter() {
                    if let Some(job_id_str) = entry.key.strip_prefix(prefix) {
                        let job_id = JobId::from_string(job_id_str.to_string());

                        if let Ok(Some(job)) = self.job_manager.get_job(&job_id).await {
                            if let Some(ref sf) = status_filter
                                && job.status != *sf
                            {
                                continue;
                            }
                            if let Some(ref jt) = job_type
                                && job.spec.job_type != *jt
                            {
                                continue;
                            }
                            if !tags.is_empty() && !tags.iter().all(|t| job.spec.config.tags.contains(&t.to_string())) {
                                continue;
                            }

                            jobs.push(job_to_details(&job));

                            if jobs.len() >= limit {
                                break;
                            }
                        }
                    }
                }

                let total_count = jobs.len() as u32;
                Ok(ClientRpcResponse::JobListResult(JobListResultResponse {
                    total_count,
                    jobs,
                    continuation_token: scan_result.continuation_token,
                    error: None,
                }))
            }
            Err(e) => {
                warn!("Failed to list jobs: {e}");
                Ok(ClientRpcResponse::JobListResult(JobListResultResponse {
                    jobs: vec![],
                    total_count: 0,
                    continuation_token: None,
                    error: Some(e.to_string()),
                }))
            }
        }
    }

    async fn handle_cancel(&self, job_id_str: String) -> Result<ClientRpcResponse> {
        let job_id = JobId::from_string(job_id_str);

        let previous_status =
            self.job_manager.get_job(&job_id).await.ok().flatten().map(|j| job_status_to_string(&j.status));

        match self.job_manager.cancel_job(&job_id).await {
            Ok(()) => Ok(ClientRpcResponse::JobCancelResult(JobCancelResultResponse {
                is_success: true,
                previous_status,
                error: None,
            })),
            Err(e) => {
                warn!("Failed to cancel job: {e}");
                Ok(ClientRpcResponse::JobCancelResult(JobCancelResultResponse {
                    is_success: false,
                    previous_status,
                    error: Some(e.to_string()),
                }))
            }
        }
    }

    async fn handle_update_progress(
        &self,
        job_id_str: String,
        progress: u8,
        message: Option<String>,
    ) -> Result<ClientRpcResponse> {
        let job_id = JobId::from_string(job_id_str);

        match self.job_manager.update_progress(&job_id, progress, message).await {
            Ok(()) => Ok(ClientRpcResponse::JobUpdateProgressResult(JobUpdateProgressResultResponse {
                is_success: true,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::JobUpdateProgressResult(JobUpdateProgressResultResponse {
                is_success: false,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_queue_stats(&self) -> Result<ClientRpcResponse> {
        let prefix = "__jobs:";
        let mut counts: HashMap<String, u64> = HashMap::new();

        match self
            .kv_store
            .scan(aspen_core::ScanRequest {
                prefix: prefix.to_string(),
                limit_results: Some(10_000),
                continuation_token: None,
            })
            .await
        {
            Ok(scan_result) => {
                for entry in scan_result.entries.iter() {
                    if let Some(job_id_str) = entry.key.strip_prefix(prefix) {
                        let job_id = JobId::from_string(job_id_str.to_string());
                        if let Ok(Some(job)) = self.job_manager.get_job(&job_id).await {
                            let status = job_status_to_string(&job.status);
                            *counts.entry(status).or_insert(0) += 1;
                        }
                    }
                }

                Ok(ClientRpcResponse::JobQueueStatsResult(JobQueueStatsResultResponse {
                    pending_count: *counts.get("pending").unwrap_or(&0),
                    scheduled_count: *counts.get("scheduled").unwrap_or(&0),
                    running_count: *counts.get("running").unwrap_or(&0),
                    completed_count: *counts.get("completed").unwrap_or(&0),
                    failed_count: *counts.get("failed").unwrap_or(&0),
                    cancelled_count: *counts.get("cancelled").unwrap_or(&0),
                    priority_counts: vec![],
                    type_counts: vec![],
                    error: None,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::JobQueueStatsResult(JobQueueStatsResultResponse {
                pending_count: 0,
                scheduled_count: 0,
                running_count: 0,
                completed_count: 0,
                failed_count: 0,
                cancelled_count: 0,
                priority_counts: vec![],
                type_counts: vec![],
                error: Some(e.to_string()),
            })),
        }
    }

    // =========================================================================
    // Worker operations
    // =========================================================================

    async fn handle_worker_status(&self) -> Result<ClientRpcResponse> {
        let Some(ref ws) = self.worker_service else {
            return Ok(ClientRpcResponse::WorkerStatusResult(WorkerStatusResultResponse {
                workers: vec![],
                total_workers: 0,
                idle_workers: 0,
                busy_workers: 0,
                offline_workers: 0,
                total_capacity_jobs: 0,
                used_capacity_jobs: 0,
                error: Some("worker service not available".to_string()),
            }));
        };

        let stats = ws.get_stats().await;
        let workers: Vec<aspen_jobs::WorkerInfo> = ws.get_worker_info().await;

        let worker_infos: Vec<WorkerInfo> = workers
            .iter()
            .map(|w| {
                let is_busy = matches!(w.status, aspen_jobs::WorkerStatus::Processing);
                WorkerInfo {
                    worker_id: w.id.clone(),
                    status: format!("{:?}", w.status),
                    capabilities: w.job_types.clone(),
                    capacity_jobs: 1,
                    active_jobs: u32::from(is_busy),
                    active_job_ids: vec![],
                    last_heartbeat: String::new(),
                    total_processed: 0,
                    total_failed: 0,
                }
            })
            .collect();

        Ok(ClientRpcResponse::WorkerStatusResult(WorkerStatusResultResponse {
            workers: worker_infos,
            total_workers: stats.total_workers as u32,
            idle_workers: stats.idle_workers as u32,
            busy_workers: stats.processing_workers as u32,
            offline_workers: 0,
            total_capacity_jobs: stats.total_workers as u32,
            used_capacity_jobs: stats.processing_workers as u32,
            error: None,
        }))
    }

    async fn handle_worker_register(
        &self,
        worker_id: String,
        capabilities: Vec<String>,
        capacity_jobs: u32,
    ) -> Result<ClientRpcResponse> {
        let Some(ref wc) = self.worker_coordinator else {
            return Ok(ClientRpcResponse::WorkerRegisterResult(WorkerRegisterResultResponse {
                is_success: false,
                worker_token: None,
                error: Some("worker coordinator not available".to_string()),
            }));
        };

        let now_ms =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

        let info = CoordWorkerInfo {
            worker_id: worker_id.clone(),
            node_id: self.node_id.to_string(),
            peer_id: None,
            capabilities,
            load: 0.0,
            active_jobs: 0,
            max_concurrent: capacity_jobs,
            queue_depth: 0,
            health: aspen_coordination::HealthStatus::Healthy,
            tags: vec![],
            last_heartbeat_ms: now_ms,
            started_at_ms: now_ms,
            total_processed: 0,
            total_failed: 0,
            avg_processing_time_ms: 0,
            groups: Default::default(),
        };

        match wc.register_worker(info).await {
            Ok(()) => Ok(ClientRpcResponse::WorkerRegisterResult(WorkerRegisterResultResponse {
                is_success: true,
                worker_token: None,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::WorkerRegisterResult(WorkerRegisterResultResponse {
                is_success: false,
                worker_token: None,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_worker_heartbeat(&self, worker_id: String, active_jobs: Vec<String>) -> Result<ClientRpcResponse> {
        let Some(ref wc) = self.worker_coordinator else {
            return Ok(ClientRpcResponse::WorkerHeartbeatResult(WorkerHeartbeatResultResponse {
                is_success: false,
                jobs_to_process: vec![],
                error: Some("worker coordinator not available".to_string()),
            }));
        };

        let stats = WorkerStats {
            load: 0.0,
            active_jobs: active_jobs.len() as u32,
            queue_depth: 0,
            total_processed: 0,
            total_failed: 0,
            avg_processing_time_ms: 0,
            health: aspen_coordination::HealthStatus::Healthy,
        };

        match wc.heartbeat(&worker_id, stats).await {
            Ok(()) => Ok(ClientRpcResponse::WorkerHeartbeatResult(WorkerHeartbeatResultResponse {
                is_success: true,
                jobs_to_process: vec![],
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::WorkerHeartbeatResult(WorkerHeartbeatResultResponse {
                is_success: false,
                jobs_to_process: vec![],
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_worker_deregister(&self, worker_id: String) -> Result<ClientRpcResponse> {
        let Some(ref wc) = self.worker_coordinator else {
            return Ok(ClientRpcResponse::WorkerDeregisterResult(WorkerDeregisterResultResponse {
                is_success: false,
                error: Some("worker coordinator not available".to_string()),
            }));
        };

        match wc.deregister_worker(&worker_id).await {
            Ok(()) => Ok(ClientRpcResponse::WorkerDeregisterResult(WorkerDeregisterResultResponse {
                is_success: true,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::WorkerDeregisterResult(WorkerDeregisterResultResponse {
                is_success: false,
                error: Some(e.to_string()),
            })),
        }
    }
}
