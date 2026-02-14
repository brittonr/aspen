//! CI pipeline RPC methods for IrohClient.

use anyhow::Result;
use aspen_client::ClientRpcRequest;
use aspen_client::ClientRpcResponse;

use super::IrohClient;
use crate::types::CiJobInfo;
use crate::types::CiPipelineDetail;
use crate::types::CiPipelineRunInfo;
use crate::types::CiStageInfo;

impl IrohClient {
    /// List CI pipeline runs.
    pub async fn ci_list_runs(
        &self,
        repo_id: Option<String>,
        status: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<CiPipelineRunInfo>> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::CiListRuns { repo_id, status, limit }).await?;

        match response {
            ClientRpcResponse::CiListRunsResult(result) => Ok(result
                .runs
                .into_iter()
                .map(|r| CiPipelineRunInfo {
                    run_id: r.run_id,
                    repo_id: r.repo_id,
                    ref_name: r.ref_name,
                    status: r.status,
                    created_at_ms: r.created_at_ms,
                })
                .collect()),
            _ => anyhow::bail!("unexpected response type for CiListRuns"),
        }
    }

    /// Get CI pipeline run status and details.
    pub async fn ci_get_status(&self, run_id: &str) -> Result<CiPipelineDetail> {
        let response = self
            .send_rpc_with_retry(ClientRpcRequest::CiGetStatus {
                run_id: run_id.to_string(),
            })
            .await?;

        match response {
            ClientRpcResponse::CiGetStatusResult(result) => {
                if !result.found {
                    anyhow::bail!("Pipeline run not found: {}", run_id);
                }
                Ok(CiPipelineDetail {
                    run_id: result.run_id.unwrap_or_default(),
                    repo_id: result.repo_id.unwrap_or_default(),
                    ref_name: result.ref_name.unwrap_or_default(),
                    commit_hash: result.commit_hash.unwrap_or_default(),
                    status: result.status.unwrap_or_default(),
                    stages: result
                        .stages
                        .into_iter()
                        .map(|s| CiStageInfo {
                            name: s.name,
                            status: s.status,
                            jobs: s
                                .jobs
                                .into_iter()
                                .map(|j| CiJobInfo {
                                    id: j.id,
                                    name: j.name,
                                    status: j.status,
                                    started_at_ms: j.started_at_ms,
                                    ended_at_ms: j.ended_at_ms,
                                    error: j.error,
                                })
                                .collect(),
                        })
                        .collect(),
                    created_at_ms: result.created_at_ms.unwrap_or(0),
                    completed_at_ms: result.completed_at_ms,
                    error: result.error,
                })
            }
            _ => anyhow::bail!("unexpected response type for CiGetStatus"),
        }
    }

    /// Trigger a CI pipeline.
    pub async fn ci_trigger_pipeline(
        &self,
        repo_id: &str,
        ref_name: &str,
        commit_hash: Option<&str>,
    ) -> Result<String> {
        let response = self
            .send_rpc_with_retry(ClientRpcRequest::CiTriggerPipeline {
                repo_id: repo_id.to_string(),
                ref_name: ref_name.to_string(),
                commit_hash: commit_hash.map(|s| s.to_string()),
            })
            .await?;

        match response {
            ClientRpcResponse::CiTriggerPipelineResult(result) => {
                if result.success {
                    Ok(result.run_id.unwrap_or_default())
                } else {
                    anyhow::bail!(
                        "Failed to trigger pipeline: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    )
                }
            }
            _ => anyhow::bail!("unexpected response type for CiTriggerPipeline"),
        }
    }

    /// Cancel a CI pipeline run.
    pub async fn ci_cancel_run(&self, run_id: &str, reason: Option<String>) -> Result<()> {
        let response = self
            .send_rpc_with_retry(ClientRpcRequest::CiCancelRun {
                run_id: run_id.to_string(),
                reason,
            })
            .await?;

        match response {
            ClientRpcResponse::CiCancelRunResult(result) => {
                if result.success {
                    Ok(())
                } else {
                    anyhow::bail!(
                        "Failed to cancel pipeline: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    )
                }
            }
            _ => anyhow::bail!("unexpected response type for CiCancelRun"),
        }
    }

    /// Get historical logs for a CI job.
    ///
    /// Fetches log chunks starting from a specific index.
    pub async fn ci_get_job_logs(
        &self,
        run_id: &str,
        job_id: &str,
        start_index: u32,
        limit: Option<u32>,
    ) -> Result<crate::client_trait::CiJobLogsResult> {
        let response = self
            .send_rpc_with_retry(ClientRpcRequest::CiGetJobLogs {
                run_id: run_id.to_string(),
                job_id: job_id.to_string(),
                start_index,
                limit,
            })
            .await?;

        match response {
            ClientRpcResponse::CiGetJobLogsResult(result) => {
                if let Some(error) = result.error {
                    anyhow::bail!("Failed to get job logs: {}", error);
                }
                Ok(crate::client_trait::CiJobLogsResult {
                    found: result.found,
                    chunks: result
                        .chunks
                        .into_iter()
                        .map(|c| crate::client_trait::CiLogChunkResult {
                            index: c.index,
                            content: c.content,
                            timestamp_ms: c.timestamp_ms,
                        })
                        .collect(),
                    last_index: result.last_index,
                    has_more: result.has_more,
                    is_complete: result.is_complete,
                })
            }
            _ => anyhow::bail!("unexpected response type for CiGetJobLogs"),
        }
    }
}
