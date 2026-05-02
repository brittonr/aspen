//! Job-related RPC methods for IrohClient.

use anyhow::Result;
use aspen_client::ClientRpcRequest;
use aspen_client::ClientRpcResponse;

use super::IrohClient;
use crate::types::JobInfo;
use crate::types::QueueStats;
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
                Ok(result.jobs.into_iter().map(JobInfo::from).collect())
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
                Ok(QueueStats::from(result))
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
                Ok(WorkerPoolInfo::from(result))
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
