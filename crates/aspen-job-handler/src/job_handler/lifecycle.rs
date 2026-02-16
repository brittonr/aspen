//! Job lifecycle handlers.
//!
//! Handles job cancellation and progress updates.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::JobCancelResultResponse;
use aspen_client_api::JobUpdateProgressResultResponse;
use aspen_core::KeyValueStore;
use aspen_jobs::JobId;
use aspen_jobs::JobManager;
use tracing::debug;
use tracing::info;
use tracing::warn;

pub(crate) async fn handle_job_cancel(
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
                is_success: true,
                previous_status: None, // Could fetch this if needed
                error: None,
            }))
        }
        Err(e) => {
            warn!("Failed to cancel job: {}", e);
            Ok(ClientRpcResponse::JobCancelResult(JobCancelResultResponse {
                is_success: false,
                previous_status: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

pub(crate) async fn handle_job_update_progress(
    job_manager: &JobManager<dyn KeyValueStore>,
    job_id: String,
    progress: u8,
    message: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Updating job progress: {} to {}%", job_id, progress);

    let job_id = JobId::from_string(job_id);

    match job_manager.update_progress(&job_id, progress, message).await {
        Ok(()) => Ok(ClientRpcResponse::JobUpdateProgressResult(JobUpdateProgressResultResponse {
            is_success: true,
            error: None,
        })),
        Err(e) => {
            warn!("Failed to update job progress: {}", e);
            Ok(ClientRpcResponse::JobUpdateProgressResult(JobUpdateProgressResultResponse {
                is_success: false,
                error: Some(e.to_string()),
            }))
        }
    }
}
