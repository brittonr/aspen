//! Worker coordination RPC handler for distributed job management.
//!
//! Handles worker coordination RPC requests from ephemeral CI workers:
//! - Job polling and assignment
//! - Job completion reporting
//!
//! VM workers use these RPCs to participate in distributed job execution.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::WorkerCompleteJobResultResponse;
use aspen_client_api::WorkerJobInfo;
use aspen_client_api::WorkerPollJobsResultResponse;
use aspen_core::KeyValueStore;
use aspen_jobs::JobId;
use aspen_jobs::JobManager;
use aspen_jobs::JobOutput;
use aspen_jobs::JobResult;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use async_trait::async_trait;
use base64::Engine;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::warn;

/// Handler for worker coordination RPC requests.
///
/// Implements job polling and completion reporting using the job manager.
/// Heartbeats are handled separately by JobHandler.
pub struct WorkerHandler<S: KeyValueStore + ?Sized> {
    job_manager: Arc<JobManager<S>>,
}

impl<S: KeyValueStore + ?Sized> WorkerHandler<S> {
    /// Create a new worker coordination handler.
    pub fn new(job_manager: Arc<JobManager<S>>) -> Self {
        Self { job_manager }
    }
}

#[async_trait]
impl<S: KeyValueStore + ?Sized + Send + Sync + 'static> RequestHandler for WorkerHandler<S> {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(request, ClientRpcRequest::WorkerPollJobs { .. } | ClientRpcRequest::WorkerCompleteJob { .. })
    }

    #[instrument(skip(self, _context))]
    async fn handle(&self, request: ClientRpcRequest, _context: &ClientProtocolContext) -> Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::WorkerPollJobs {
                worker_id,
                job_types,
                max_jobs,
                visibility_timeout_secs,
            } => self.handle_poll_jobs(worker_id, job_types, max_jobs, visibility_timeout_secs).await,

            ClientRpcRequest::WorkerCompleteJob {
                worker_id,
                job_id,
                receipt_handle,
                execution_token,
                is_success,
                error_message,
                output_data,
                processing_time_ms,
            } => {
                self.handle_complete_job(
                    worker_id,
                    job_id,
                    receipt_handle,
                    execution_token,
                    is_success,
                    error_message,
                    output_data,
                    processing_time_ms,
                )
                .await
            }

            _ => unreachable!("can_handle should have rejected this request"),
        }
    }

    fn name(&self) -> &'static str {
        "WorkerHandler"
    }
}

impl<S: KeyValueStore + ?Sized + Send + Sync + 'static> WorkerHandler<S> {
    /// Handle job polling request from a remote worker.
    ///
    /// Dequeues jobs matching the requested types, marks them as started,
    /// and returns job info with execution credentials.
    async fn handle_poll_jobs(
        &self,
        worker_id: String,
        job_types: Vec<String>,
        max_jobs: usize,
        visibility_timeout_secs: u64,
    ) -> Result<ClientRpcResponse> {
        info!(
            worker_id = %worker_id,
            job_types = ?job_types,
            max_jobs,
            visibility_timeout_secs,
            "external worker polling for jobs"
        );

        let visibility_timeout = Duration::from_secs(visibility_timeout_secs);

        // Dequeue jobs from the job manager
        let dequeued = match self.job_manager.dequeue_jobs(&worker_id, max_jobs as u32, visibility_timeout).await {
            Ok(jobs) => jobs,
            Err(e) => {
                error!(worker_id = %worker_id, error = %e, "failed to dequeue jobs");
                return Ok(ClientRpcResponse::WorkerPollJobsResult(WorkerPollJobsResultResponse {
                    is_success: false,
                    worker_id,
                    jobs: vec![],
                    error: Some(format!("Failed to dequeue jobs: {}", e)),
                }));
            }
        };

        info!(
            worker_id = %worker_id,
            dequeued_count = dequeued.len(),
            "dequeued jobs for external worker"
        );

        // Filter by job types if specified, releasing non-matching jobs back to the queue
        let filtered: Vec<_> = if job_types.is_empty() {
            dequeued
        } else {
            let (matching, non_matching): (Vec<_>, Vec<_>) =
                dequeued.into_iter().partition(|(_, job)| job_types.contains(&job.spec.job_type));

            // Release non-matching jobs back to the queue IMMEDIATELY so other workers can claim them.
            // This is critical for FIFO message group ordering - if we leave them pending, jobs of
            // the same type will be blocked until visibility timeout expires.
            for (item, job) in non_matching {
                info!(
                    worker_id = %worker_id,
                    job_id = %job.id,
                    job_type = %job.spec.job_type,
                    requested_types = ?job_types,
                    "releasing non-matching job back to queue"
                );
                if let Err(e) = self
                    .job_manager
                    .release_unhandled_job(
                        &job.id,
                        &item.receipt_handle,
                        format!("job type {} not in requested types {:?}", job.spec.job_type, job_types),
                    )
                    .await
                {
                    warn!(
                        worker_id = %worker_id,
                        job_id = %job.id,
                        error = %e,
                        "failed to release non-matching job - it will become visible after timeout"
                    );
                }
            }

            matching
        };

        info!(
            worker_id = %worker_id,
            filtered_count = filtered.len(),
            job_types = ?job_types,
            "filtered jobs by type for external worker"
        );

        // Convert dequeued jobs to WorkerJobInfo, marking each as started
        let mut job_infos = Vec::with_capacity(filtered.len());

        for (dequeued_item, job) in filtered {
            // Mark job as started to get execution token
            let execution_token = match self.job_manager.mark_started(&job.id, worker_id.clone()).await {
                Ok(token) => token,
                Err(e) => {
                    warn!(
                        worker_id = %worker_id,
                        job_id = %job.id,
                        error = %e,
                        "failed to mark job as started, skipping"
                    );
                    // Nack the job so it can be picked up by another worker
                    // We don't have an execution token, so we can't use nack_job directly
                    // The visibility timeout will handle this - job becomes visible again
                    continue;
                }
            };

            // Serialize job spec to JSON
            let job_spec_json = match serde_json::to_string(&job.spec) {
                Ok(json) => json,
                Err(e) => {
                    error!(job_id = %job.id, error = %e, "failed to serialize job spec");
                    continue;
                }
            };

            let job_info = WorkerJobInfo {
                job_id: job.id.to_string(),
                job_type: job.spec.job_type.clone(),
                job_spec_json,
                priority: format!("{:?}", job.spec.config.priority),
                created_at_ms: job.created_at.timestamp_millis() as u64,
                visibility_timeout_ms: dequeued_item.visibility_deadline_ms,
                receipt_handle: dequeued_item.receipt_handle,
                execution_token,
            };

            info!(
                worker_id = %worker_id,
                job_id = %job_info.job_id,
                job_type = %job_info.job_type,
                "job assigned to remote worker"
            );

            job_infos.push(job_info);
        }

        debug!(
            worker_id = %worker_id,
            jobs_assigned = job_infos.len(),
            "job polling complete"
        );

        Ok(ClientRpcResponse::WorkerPollJobsResult(WorkerPollJobsResultResponse {
            is_success: true,
            worker_id,
            jobs: job_infos,
            error: None,
        }))
    }

    /// Handle job completion report from a remote worker.
    ///
    /// Acknowledges or negative-acknowledges the job based on success status.
    #[allow(clippy::too_many_arguments)]
    async fn handle_complete_job(
        &self,
        worker_id: String,
        job_id: String,
        receipt_handle: String,
        execution_token: String,
        is_success: bool,
        error_message: Option<String>,
        output_data: Option<Vec<u8>>,
        processing_time_ms: u64,
    ) -> Result<ClientRpcResponse> {
        debug!(
            worker_id = %worker_id,
            job_id = %job_id,
            is_success,
            processing_time_ms,
            "worker completing job"
        );

        let job_id_parsed = JobId::from_string(job_id.clone());

        if is_success {
            // Build job output from provided data
            let data = match output_data {
                Some(d) => serde_json::Value::String(base64::engine::general_purpose::STANDARD.encode(&d)),
                None => serde_json::Value::Null,
            };

            let mut metadata = std::collections::HashMap::new();
            metadata.insert("worker_id".to_string(), worker_id.clone());
            metadata.insert("processing_time_ms".to_string(), processing_time_ms.to_string());

            let output = JobOutput { data, metadata };

            // Acknowledge successful completion
            match self
                .job_manager
                .ack_job(&job_id_parsed, &receipt_handle, &execution_token, JobResult::Success(output))
                .await
            {
                Ok(()) => {
                    info!(
                        worker_id = %worker_id,
                        job_id = %job_id,
                        processing_time_ms,
                        "job completed successfully"
                    );
                    Ok(ClientRpcResponse::WorkerCompleteJobResult(WorkerCompleteJobResultResponse {
                        is_success: true,
                        worker_id,
                        job_id,
                        error: None,
                    }))
                }
                Err(e) => {
                    error!(
                        worker_id = %worker_id,
                        job_id = %job_id,
                        error = %e,
                        "failed to acknowledge job completion"
                    );
                    Ok(ClientRpcResponse::WorkerCompleteJobResult(WorkerCompleteJobResultResponse {
                        is_success: false,
                        worker_id,
                        job_id,
                        error: Some(format!("Failed to acknowledge job: {}", e)),
                    }))
                }
            }
        } else {
            // Negative acknowledge - job failed
            let error_msg = error_message.unwrap_or_else(|| "Unknown error".to_string());

            match self
                .job_manager
                .nack_job(&job_id_parsed, &receipt_handle, &execution_token, error_msg.clone())
                .await
            {
                Ok(()) => {
                    warn!(
                        worker_id = %worker_id,
                        job_id = %job_id,
                        error = %error_msg,
                        "job failed, negative acknowledged"
                    );
                    Ok(ClientRpcResponse::WorkerCompleteJobResult(WorkerCompleteJobResultResponse {
                        is_success: true,
                        worker_id,
                        job_id,
                        error: None,
                    }))
                }
                Err(e) => {
                    error!(
                        worker_id = %worker_id,
                        job_id = %job_id,
                        error = %e,
                        "failed to negative acknowledge job"
                    );
                    Ok(ClientRpcResponse::WorkerCompleteJobResult(WorkerCompleteJobResultResponse {
                        is_success: false,
                        worker_id,
                        job_id,
                        error: Some(format!("Failed to nack job: {}", e)),
                    }))
                }
            }
        }
    }
}
