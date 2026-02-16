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
                capacity_jobs,
            } => {
                handle_worker_register(
                    ctx.worker_coordinator.as_ref(),
                    ctx.node_id,
                    worker_id,
                    capabilities,
                    capacity_jobs,
                )
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

#[cfg(test)]
mod tests {
    use aspen_rpc_core::RequestHandler;

    use super::*;

    fn handler() -> JobHandler {
        JobHandler
    }

    #[test]
    fn test_handler_name() {
        assert_eq!(handler().name(), "JobHandler");
    }

    #[test]
    fn test_can_handle_job_submit() {
        let req = ClientRpcRequest::JobSubmit {
            job_type: "build".to_string(),
            payload: "{}".to_string(),
            priority: Some(2),
            timeout_ms: Some(60_000),
            max_retries: Some(3),
            retry_delay_ms: Some(1_000),
            schedule: None,
            tags: vec!["ci".to_string()],
        };
        assert!(handler().can_handle(&req));
    }

    #[test]
    fn test_can_handle_job_get() {
        let req = ClientRpcRequest::JobGet {
            job_id: "job-123".to_string(),
        };
        assert!(handler().can_handle(&req));
    }

    #[test]
    fn test_can_handle_job_list() {
        let req = ClientRpcRequest::JobList {
            status: Some("pending".to_string()),
            job_type: Some("build".to_string()),
            tags: vec![],
            limit: Some(50),
            continuation_token: None,
        };
        assert!(handler().can_handle(&req));
    }

    #[test]
    fn test_can_handle_job_cancel() {
        let req = ClientRpcRequest::JobCancel {
            job_id: "job-456".to_string(),
            reason: Some("no longer needed".to_string()),
        };
        assert!(handler().can_handle(&req));
    }

    #[test]
    fn test_can_handle_job_update_progress() {
        let req = ClientRpcRequest::JobUpdateProgress {
            job_id: "job-789".to_string(),
            progress: 75,
            message: Some("compiling".to_string()),
        };
        assert!(handler().can_handle(&req));
    }

    #[test]
    fn test_can_handle_job_queue_stats() {
        let req = ClientRpcRequest::JobQueueStats;
        assert!(handler().can_handle(&req));
    }

    #[test]
    fn test_can_handle_worker_status() {
        let req = ClientRpcRequest::WorkerStatus;
        assert!(handler().can_handle(&req));
    }

    #[test]
    fn test_can_handle_worker_register() {
        let req = ClientRpcRequest::WorkerRegister {
            worker_id: "w-1".to_string(),
            capabilities: vec!["build".to_string(), "test".to_string()],
            capacity_jobs: 4,
        };
        assert!(handler().can_handle(&req));
    }

    #[test]
    fn test_can_handle_worker_heartbeat() {
        let req = ClientRpcRequest::WorkerHeartbeat {
            worker_id: "w-1".to_string(),
            active_jobs: vec!["job-1".to_string()],
        };
        assert!(handler().can_handle(&req));
    }

    #[test]
    fn test_can_handle_worker_deregister() {
        let req = ClientRpcRequest::WorkerDeregister {
            worker_id: "w-1".to_string(),
        };
        assert!(handler().can_handle(&req));
    }

    #[test]
    fn test_rejects_unrelated_request() {
        let req = ClientRpcRequest::Ping;
        assert!(!handler().can_handle(&req));
    }
}
