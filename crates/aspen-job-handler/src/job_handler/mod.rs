//! Job queue RPC handlers.
//!
//! Handles job submission, management, and worker coordination through
//! the distributed job queue system.
//!
//! Each domain is handled by a dedicated sub-module:
//! - `submit`: Job submission with priority, retry, scheduling
//! - `query`: Job retrieval and listing with filters
//! - `lifecycle`: Job cancellation and progress updates
//! - `stats`: Job queue statistics
//! - `worker`: Worker registration, heartbeats, status

mod lifecycle;
mod query;
mod stats;
mod submit;
mod worker;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_jobs::JobStatus;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use async_trait::async_trait;

use self::lifecycle::handle_job_cancel;
use self::lifecycle::handle_job_update_progress;
use self::query::handle_job_get;
use self::query::handle_job_list;
use self::stats::handle_job_queue_stats;
use self::submit::JobSubmitConfig;
use self::submit::handle_job_submit;
use self::worker::handle_worker_deregister;
use self::worker::handle_worker_heartbeat;
use self::worker::handle_worker_register;
use self::worker::handle_worker_status;

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

/// Convert JobStatus to lowercase string for API responses.
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
        let job_manager = ctx.job_manager.as_ref().ok_or_else(|| anyhow::anyhow!("job manager not available"))?;

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
                handle_job_submit(job_manager, job_type, payload, JobSubmitConfig {
                    priority,
                    timeout_ms,
                    max_retries,
                    retry_delay_ms,
                    schedule,
                    tags,
                })
                .await
            }

            ClientRpcRequest::JobGet { job_id } => handle_job_get(job_manager, job_id).await,

            ClientRpcRequest::JobList {
                status,
                job_type,
                tags,
                limit,
                continuation_token,
            } => handle_job_list(job_manager, &ctx.kv_store, status, job_type, tags, limit, continuation_token).await,

            ClientRpcRequest::JobCancel { job_id, reason } => handle_job_cancel(job_manager, job_id, reason).await,

            ClientRpcRequest::JobUpdateProgress {
                job_id,
                progress,
                message,
            } => handle_job_update_progress(job_manager, job_id, progress, message).await,

            ClientRpcRequest::JobQueueStats => handle_job_queue_stats(job_manager).await,

            ClientRpcRequest::WorkerStatus => handle_worker_status(ctx.worker_service.as_ref()).await,

            ClientRpcRequest::WorkerRegister {
                worker_id,
                capabilities,
                capacity,
            } => {
                handle_worker_register(ctx.worker_coordinator.as_ref(), ctx.node_id, worker_id, capabilities, capacity)
                    .await
            }

            ClientRpcRequest::WorkerHeartbeat { worker_id, active_jobs } => {
                handle_worker_heartbeat(ctx.worker_coordinator.as_ref(), worker_id, active_jobs).await
            }

            ClientRpcRequest::WorkerDeregister { worker_id } => {
                handle_worker_deregister(ctx.worker_coordinator.as_ref(), worker_id).await
            }

            _ => Err(anyhow::anyhow!("request not handled by JobHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "JobHandler"
    }
}
