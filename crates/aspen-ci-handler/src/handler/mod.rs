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
                    is_success: false,
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
                    is_success: false,
                    error: Some("CI watch requires forge feature".to_string()),
                }))
            }
            #[cfg(feature = "forge")]
            ClientRpcRequest::CiUnwatchRepo { repo_id } => handle_unwatch_repo(ctx, repo_id).await,
            #[cfg(not(feature = "forge"))]
            ClientRpcRequest::CiUnwatchRepo { .. } => {
                Ok(ClientRpcResponse::CiUnwatchRepoResult(aspen_client_api::CiUnwatchRepoResponse {
                    is_success: false,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_handle_trigger_pipeline() {
        let handler = CiHandler;
        assert!(handler.can_handle(&ClientRpcRequest::CiTriggerPipeline {
            repo_id: "my-repo".to_string(),
            ref_name: "refs/heads/main".to_string(),
            commit_hash: None,
        }));
    }

    #[test]
    fn test_can_handle_get_status() {
        let handler = CiHandler;
        assert!(handler.can_handle(&ClientRpcRequest::CiGetStatus {
            run_id: "run-123".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_list_runs() {
        let handler = CiHandler;
        assert!(handler.can_handle(&ClientRpcRequest::CiListRuns {
            repo_id: Some("my-repo".to_string()),
            status: Some("running".to_string()),
            limit: Some(50),
        }));
    }

    #[test]
    fn test_can_handle_cancel_run() {
        let handler = CiHandler;
        assert!(handler.can_handle(&ClientRpcRequest::CiCancelRun {
            run_id: "run-456".to_string(),
            reason: Some("no longer needed".to_string()),
        }));
    }

    #[test]
    fn test_can_handle_watch_repo() {
        let handler = CiHandler;
        assert!(handler.can_handle(&ClientRpcRequest::CiWatchRepo {
            repo_id: "my-repo".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_unwatch_repo() {
        let handler = CiHandler;
        assert!(handler.can_handle(&ClientRpcRequest::CiUnwatchRepo {
            repo_id: "my-repo".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_list_artifacts() {
        let handler = CiHandler;
        assert!(handler.can_handle(&ClientRpcRequest::CiListArtifacts {
            job_id: "job-789".to_string(),
            run_id: Some("run-123".to_string()),
        }));
    }

    #[test]
    fn test_can_handle_get_artifact() {
        let handler = CiHandler;
        assert!(handler.can_handle(&ClientRpcRequest::CiGetArtifact {
            blob_hash: "abc123def456".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_get_job_logs() {
        let handler = CiHandler;
        assert!(handler.can_handle(&ClientRpcRequest::CiGetJobLogs {
            run_id: "run-123".to_string(),
            job_id: "job-456".to_string(),
            start_index: 0,
            limit: Some(100),
        }));
    }

    #[test]
    fn test_can_handle_subscribe_logs() {
        let handler = CiHandler;
        assert!(handler.can_handle(&ClientRpcRequest::CiSubscribeLogs {
            run_id: "run-123".to_string(),
            job_id: "job-456".to_string(),
            from_index: Some(42),
        }));
    }

    #[test]
    fn test_can_handle_get_job_output() {
        let handler = CiHandler;
        assert!(handler.can_handle(&ClientRpcRequest::CiGetJobOutput {
            run_id: "run-123".to_string(),
            job_id: "job-456".to_string(),
        }));
    }

    #[test]
    fn test_rejects_unrelated_requests() {
        let handler = CiHandler;

        // Core requests
        assert!(!handler.can_handle(&ClientRpcRequest::Ping));
        assert!(!handler.can_handle(&ClientRpcRequest::GetHealth));

        // Cluster requests
        assert!(!handler.can_handle(&ClientRpcRequest::InitCluster));
        assert!(!handler.can_handle(&ClientRpcRequest::GetClusterState));

        // KV requests
        assert!(!handler.can_handle(&ClientRpcRequest::ReadKey {
            key: "test".to_string(),
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::WriteKey {
            key: "test".to_string(),
            value: vec![1, 2, 3],
        }));
    }

    #[test]
    fn test_handler_name() {
        let handler = CiHandler;
        assert_eq!(handler.name(), "CiHandler");
    }
}
