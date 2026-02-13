//! CI/CD pipeline RPC handlers.
//!
//! Handles pipeline triggering, status queries, and repository watching
//! through the aspen-ci orchestrator and trigger service.

mod artifacts;
mod helpers;
mod logs;
mod pipeline;
#[cfg(feature = "forge")]
mod watch;

use artifacts::*;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use async_trait::async_trait;
use logs::*;
use pipeline::*;
#[cfg(feature = "forge")]
use watch::*;

/// Handler for CI/CD pipeline operations.
///
/// Processes CI RPC requests for pipeline triggering, status queries,
/// and repository watching.
///
/// # Tiger Style
///
/// - Bounded pipeline execution via aspen-jobs
/// - Clear error reporting for configuration and execution issues
/// - Read-only status queries for monitoring
pub struct CiHandler;

#[async_trait]
impl RequestHandler for CiHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::CiTriggerPipeline { .. }
                | ClientRpcRequest::CiGetStatus { .. }
                | ClientRpcRequest::CiListRuns { .. }
                | ClientRpcRequest::CiCancelRun { .. }
                | ClientRpcRequest::CiWatchRepo { .. }
                | ClientRpcRequest::CiUnwatchRepo { .. }
                | ClientRpcRequest::CiListArtifacts { .. }
                | ClientRpcRequest::CiGetArtifact { .. }
                | ClientRpcRequest::CiGetJobLogs { .. }
                | ClientRpcRequest::CiSubscribeLogs { .. }
                | ClientRpcRequest::CiGetJobOutput { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            // Pipeline operations
            #[cfg(all(feature = "forge", feature = "blob"))]
            ClientRpcRequest::CiTriggerPipeline {
                repo_id,
                ref_name,
                commit_hash,
            } => handle_trigger_pipeline(ctx, repo_id, ref_name, commit_hash).await,
            #[cfg(not(all(feature = "forge", feature = "blob")))]
            ClientRpcRequest::CiTriggerPipeline { .. } => {
                Ok(ClientRpcResponse::CiTriggerPipelineResult(aspen_client_api::CiTriggerPipelineResponse {
                    success: false,
                    run_id: None,
                    error: Some("CI trigger requires forge and blob features".to_string()),
                }))
            }
            ClientRpcRequest::CiGetStatus { run_id } => handle_get_status(ctx, run_id).await,
            #[cfg(feature = "forge")]
            ClientRpcRequest::CiListRuns { repo_id, status, limit } => {
                handle_list_runs(ctx, repo_id, status, limit).await
            }
            #[cfg(not(feature = "forge"))]
            ClientRpcRequest::CiListRuns { .. } => {
                Ok(ClientRpcResponse::CiListRunsResult(aspen_client_api::CiListRunsResponse { runs: vec![] }))
            }
            ClientRpcRequest::CiCancelRun { run_id, reason } => handle_cancel_run(ctx, run_id, reason).await,

            // Watch operations
            #[cfg(feature = "forge")]
            ClientRpcRequest::CiWatchRepo { repo_id } => handle_watch_repo(ctx, repo_id).await,
            #[cfg(not(feature = "forge"))]
            ClientRpcRequest::CiWatchRepo { .. } => {
                Ok(ClientRpcResponse::CiWatchRepoResult(aspen_client_api::CiWatchRepoResponse {
                    success: false,
                    error: Some("CI watch requires forge feature".to_string()),
                }))
            }
            #[cfg(feature = "forge")]
            ClientRpcRequest::CiUnwatchRepo { repo_id } => handle_unwatch_repo(ctx, repo_id).await,
            #[cfg(not(feature = "forge"))]
            ClientRpcRequest::CiUnwatchRepo { .. } => {
                Ok(ClientRpcResponse::CiUnwatchRepoResult(aspen_client_api::CiUnwatchRepoResponse {
                    success: false,
                    error: Some("CI unwatch requires forge feature".to_string()),
                }))
            }

            // Artifact operations
            ClientRpcRequest::CiListArtifacts { job_id, run_id } => handle_list_artifacts(ctx, job_id, run_id).await,
            ClientRpcRequest::CiGetArtifact { blob_hash } => handle_get_artifact(ctx, blob_hash).await,

            // Log operations
            ClientRpcRequest::CiGetJobLogs {
                run_id,
                job_id,
                start_index,
                limit,
            } => handle_get_job_logs(ctx, run_id, job_id, start_index, limit).await,
            ClientRpcRequest::CiSubscribeLogs {
                run_id,
                job_id,
                from_index,
            } => handle_subscribe_logs(ctx, run_id, job_id, from_index).await,
            ClientRpcRequest::CiGetJobOutput { run_id, job_id } => handle_get_job_output(ctx, run_id, job_id).await,

            _ => Err(anyhow::anyhow!("request not handled by CiHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "CiHandler"
    }
}
