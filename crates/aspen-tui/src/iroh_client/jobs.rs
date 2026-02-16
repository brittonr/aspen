//! Job-related RPC methods for IrohClient.

use anyhow::Result;
use aspen_client::ClientRpcRequest;
use aspen_client::ClientRpcResponse;

use super::IrohClient;
use crate::types::JobInfo;
use crate::types::QueueStats;
use crate::types::WorkerInfo;
use crate::types::WorkerPoolInfo;

impl IrohClient {
    /// List jobs with optional status filter.
    pub async fn list_jobs(&self, status: Option<String>, limit: Option<u32>) -> Result<Vec<JobInfo>> {
        let response = self
            .send_rpc_with_retry(ClientRpcRequest::JobList {
                status,
                job_type: None,
                tags: vec![],
                limit,
                continuation_token: None,
            })
            .await?;

        match response {
            ClientRpcResponse::JobListResult(result) => {
                if let Some(error) = result.error {
                    anyhow::bail!("Failed to list jobs: {}", error);
                }
                Ok(result
                    .jobs
                    .into_iter()
                    .map(|j| JobInfo {
                        job_id: j.job_id,
                        job_type: j.job_type,
                        status: j.status,
                        priority: j.priority,
                        progress: j.progress,
                        progress_message: j.progress_message,
                        tags: j.tags,
                        submitted_at: j.submitted_at,
                        started_at: j.started_at,
                        completed_at: j.completed_at,
                        worker_id: j.worker_id,
                        attempts: j.attempts,
                        error_message: j.error_message,
                    })
                    .collect())
            }
            _ => anyhow::bail!("unexpected response type for JobList"),
        }
    }

    /// Get job queue statistics.
    pub async fn get_queue_stats(&self) -> Result<QueueStats> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::JobQueueStats).await?;

        match response {
            ClientRpcResponse::JobQueueStatsResult(result) => {
                if let Some(error) = result.error {
                    anyhow::bail!("Failed to get queue stats: {}", error);
                }
                Ok(QueueStats {
                    pending_count: result.pending_count,
                    scheduled_count: result.scheduled_count,
                    running_count: result.running_count,
                    completed_count: result.completed_count,
                    failed_count: result.failed_count,
                    cancelled_count: result.cancelled_count,
                    priority_counts: result.priority_counts.into_iter().map(|pc| (pc.priority, pc.count)).collect(),
                    type_counts: result.type_counts.into_iter().map(|tc| (tc.job_type, tc.count)).collect(),
                })
            }
            _ => anyhow::bail!("unexpected response type for JobQueueStats"),
        }
    }

    /// Get worker pool status.
    pub async fn get_worker_status(&self) -> Result<WorkerPoolInfo> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::WorkerStatus).await?;

        match response {
            ClientRpcResponse::WorkerStatusResult(result) => {
                if let Some(error) = result.error {
                    anyhow::bail!("Failed to get worker status: {}", error);
                }
                Ok(WorkerPoolInfo {
                    workers: result
                        .workers
                        .into_iter()
                        .map(|w| WorkerInfo {
                            worker_id: w.worker_id,
                            status: w.status,
                            capabilities: w.capabilities,
                            capacity: w.capacity,
                            active_jobs: w.active_jobs,
                            active_job_ids: w.active_job_ids,
                            last_heartbeat: w.last_heartbeat,
                            total_processed: w.total_processed,
                            total_failed: w.total_failed,
                        })
                        .collect(),
                    total_workers: result.total_workers,
                    idle_workers: result.idle_workers,
                    busy_workers: result.busy_workers,
                    offline_workers: result.offline_workers,
                    total_capacity: result.total_capacity,
                    used_capacity: result.used_capacity,
                })
            }
            _ => anyhow::bail!("unexpected response type for WorkerStatus"),
        }
    }

    /// Cancel a job.
    pub async fn cancel_job(&self, job_id: &str, reason: Option<String>) -> Result<()> {
        let response = self
            .send_rpc_with_retry(ClientRpcRequest::JobCancel {
                job_id: job_id.to_string(),
                reason,
            })
            .await?;

        match response {
            ClientRpcResponse::JobCancelResult(result) => {
                if result.is_success {
                    Ok(())
                } else {
                    anyhow::bail!(
                        "Failed to cancel job: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    )
                }
            }
            _ => anyhow::bail!("unexpected response type for JobCancel"),
        }
    }
}
