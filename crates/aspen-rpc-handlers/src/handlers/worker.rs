//! Worker coordination RPC handler for distributed job management.
//!
//! Handles worker coordination RPC requests from ephemeral CI workers:
//! - Job polling and assignment
//! - Job completion reporting
//!
//! VM workers use these RPCs to participate in distributed job execution.

use std::sync::Arc;

use anyhow::Result;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_client_rpc::WorkerCompleteJobResultResponse;
use aspen_client_rpc::WorkerPollJobsResultResponse;
use aspen_coordination::DistributedWorkerCoordinator;
use aspen_core::KeyValueStore;
use aspen_jobs::JobManager;
use async_trait::async_trait;
use tracing::debug;
use tracing::instrument;
use tracing::warn;

use crate::RequestHandler;
use crate::context::ClientProtocolContext;

/// Handler for worker coordination RPC requests.
///
/// Implements job polling and completion reporting
/// using the distributed worker coordinator and job manager.
pub struct WorkerHandler<S: KeyValueStore + ?Sized> {
    _coordinator: Arc<DistributedWorkerCoordinator<S>>,
    _job_manager: Arc<JobManager<S>>,
}

impl<S: KeyValueStore + ?Sized> WorkerHandler<S> {
    /// Create a new worker coordination handler.
    pub fn new(coordinator: Arc<DistributedWorkerCoordinator<S>>, job_manager: Arc<JobManager<S>>) -> Self {
        Self {
            _coordinator: coordinator,
            _job_manager: job_manager,
        }
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
            } => {
                debug!(worker_id, ?job_types, max_jobs, visibility_timeout_secs, "worker polling for jobs");

                // TODO: Implement job polling logic
                // For now, return empty job list to prevent blocking
                warn!(worker_id, "job polling not fully implemented yet");

                Ok(ClientRpcResponse::WorkerPollJobsResult(WorkerPollJobsResultResponse {
                    success: true,
                    worker_id,
                    jobs: vec![], // TODO: Fetch actual jobs from job manager
                    error: None,
                }))
            }

            ClientRpcRequest::WorkerCompleteJob {
                worker_id,
                job_id,
                success,
                error_message,
                output_data: _,
                processing_time_ms,
            } => {
                debug!(worker_id, job_id, success, processing_time_ms, "worker completing job");

                // TODO: Implement job completion logic
                // For now, just log the completion
                if success {
                    debug!(worker_id, job_id, "job completed successfully");
                } else {
                    warn!(
                        worker_id,
                        job_id,
                        error = ?error_message,
                        "job failed"
                    );
                }

                Ok(ClientRpcResponse::WorkerCompleteJobResult(WorkerCompleteJobResultResponse {
                    success: true,
                    worker_id,
                    job_id,
                    error: None,
                }))
            }

            _ => unreachable!("can_handle should have rejected this request"),
        }
    }

    fn name(&self) -> &'static str {
        "WorkerHandler"
    }
}
