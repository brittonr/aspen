//! CI/CD pipeline RPC handlers.
//!
//! Handles pipeline triggering, status queries, and repository watching
//! through the aspen-ci orchestrator and trigger service.

use aspen_client_rpc::CiCancelRunResponse;
use aspen_client_rpc::CiGetStatusResponse;
use aspen_client_rpc::CiListRunsResponse;
use aspen_client_rpc::CiTriggerPipelineResponse;
use aspen_client_rpc::CiUnwatchRepoResponse;
use aspen_client_rpc::CiWatchRepoResponse;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use async_trait::async_trait;
use tracing::debug;
use tracing::info;

use crate::context::ClientProtocolContext;
use crate::registry::RequestHandler;

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
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::CiTriggerPipeline {
                repo_id,
                ref_name,
                commit_hash,
            } => handle_trigger_pipeline(ctx, repo_id, ref_name, commit_hash).await,
            ClientRpcRequest::CiGetStatus { run_id } => handle_get_status(ctx, run_id).await,
            ClientRpcRequest::CiListRuns { repo_id, status, limit } => {
                handle_list_runs(ctx, repo_id, status, limit).await
            }
            ClientRpcRequest::CiCancelRun { run_id, reason } => handle_cancel_run(ctx, run_id, reason).await,
            ClientRpcRequest::CiWatchRepo { repo_id } => handle_watch_repo(ctx, repo_id).await,
            ClientRpcRequest::CiUnwatchRepo { repo_id } => handle_unwatch_repo(ctx, repo_id).await,
            _ => Err(anyhow::anyhow!("request not handled by CiHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "CiHandler"
    }
}

/// Handle CiTriggerPipeline request.
///
/// Triggers a new pipeline run for the given repository and ref.
async fn handle_trigger_pipeline(
    ctx: &ClientProtocolContext,
    repo_id: String,
    ref_name: String,
    _commit_hash: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(_orchestrator) = &ctx.ci_orchestrator else {
        return Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
            success: false,
            run_id: None,
            error: Some("CI orchestrator not available".to_string()),
        }));
    };

    info!(repo_id = %repo_id, ref_name = %ref_name, "triggering CI pipeline");

    // TODO: Load CI config from repository's .aspen/ci.ncl
    // TODO: Create pipeline context with repo_id, ref_name, commit_hash
    // TODO: Execute pipeline via orchestrator

    // For now, return a placeholder response indicating the feature is not fully implemented
    Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
        success: false,
        run_id: None,
        error: Some("CI pipeline triggering not yet fully implemented - config loading pending".to_string()),
    }))
}

/// Handle CiGetStatus request.
///
/// Returns the current status of a pipeline run.
async fn handle_get_status(ctx: &ClientProtocolContext, run_id: String) -> anyhow::Result<ClientRpcResponse> {
    let Some(_orchestrator) = &ctx.ci_orchestrator else {
        return Ok(ClientRpcResponse::CiGetStatusResult(CiGetStatusResponse {
            found: false,
            run_id: None,
            repo_id: None,
            ref_name: None,
            commit_hash: None,
            status: None,
            stages: vec![],
            created_at_ms: None,
            completed_at_ms: None,
            error: Some("CI orchestrator not available".to_string()),
        }));
    };

    debug!(run_id = %run_id, "getting CI pipeline status");

    // TODO: Query pipeline run status from orchestrator
    // For now, return not found
    Ok(ClientRpcResponse::CiGetStatusResult(CiGetStatusResponse {
        found: false,
        run_id: Some(run_id),
        repo_id: None,
        ref_name: None,
        commit_hash: None,
        status: None,
        stages: vec![],
        created_at_ms: None,
        completed_at_ms: None,
        error: Some("Pipeline run not found".to_string()),
    }))
}

/// Handle CiListRuns request.
///
/// Lists pipeline runs with optional filtering.
async fn handle_list_runs(
    ctx: &ClientProtocolContext,
    repo_id: Option<String>,
    status: Option<String>,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(_orchestrator) = &ctx.ci_orchestrator else {
        return Ok(ClientRpcResponse::CiListRunsResult(CiListRunsResponse { runs: vec![] }));
    };

    let limit = limit.unwrap_or(50).min(500) as usize;
    debug!(?repo_id, ?status, limit, "listing CI pipeline runs");

    // TODO: Query pipeline runs from orchestrator with filters
    // For now, return empty list
    Ok(ClientRpcResponse::CiListRunsResult(CiListRunsResponse { runs: vec![] }))
}

/// Handle CiCancelRun request.
///
/// Cancels a running pipeline.
async fn handle_cancel_run(
    ctx: &ClientProtocolContext,
    run_id: String,
    reason: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(_orchestrator) = &ctx.ci_orchestrator else {
        return Ok(ClientRpcResponse::CiCancelRunResult(CiCancelRunResponse {
            success: false,
            error: Some("CI orchestrator not available".to_string()),
        }));
    };

    info!(%run_id, ?reason, "cancelling CI pipeline");

    // TODO: Cancel pipeline via orchestrator
    Ok(ClientRpcResponse::CiCancelRunResult(CiCancelRunResponse {
        success: false,
        error: Some("Pipeline cancellation not yet implemented".to_string()),
    }))
}

/// Handle CiWatchRepo request.
///
/// Subscribes to forge gossip events for automatic CI triggering.
async fn handle_watch_repo(ctx: &ClientProtocolContext, repo_id: String) -> anyhow::Result<ClientRpcResponse> {
    let Some(_trigger_service) = &ctx.ci_trigger_service else {
        return Ok(ClientRpcResponse::CiWatchRepoResult(CiWatchRepoResponse {
            success: false,
            error: Some("CI trigger service not available".to_string()),
        }));
    };

    info!(%repo_id, "watching repository for CI triggers");

    // TODO: Register repo for watching via trigger service
    Ok(ClientRpcResponse::CiWatchRepoResult(CiWatchRepoResponse {
        success: false,
        error: Some("Repository watching not yet implemented".to_string()),
    }))
}

/// Handle CiUnwatchRepo request.
///
/// Removes CI trigger subscription for a repository.
async fn handle_unwatch_repo(ctx: &ClientProtocolContext, repo_id: String) -> anyhow::Result<ClientRpcResponse> {
    let Some(_trigger_service) = &ctx.ci_trigger_service else {
        return Ok(ClientRpcResponse::CiUnwatchRepoResult(CiUnwatchRepoResponse {
            success: false,
            error: Some("CI trigger service not available".to_string()),
        }));
    };

    info!(%repo_id, "unwatching repository");

    // TODO: Unregister repo from trigger service
    Ok(ClientRpcResponse::CiUnwatchRepoResult(CiUnwatchRepoResponse {
        success: false,
        error: Some("Repository unwatching not yet implemented".to_string()),
    }))
}
