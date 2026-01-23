//! CI/CD pipeline RPC handlers.
//!
//! Handles pipeline triggering, status queries, and repository watching
//! through the aspen-ci orchestrator and trigger service.

use std::collections::HashMap;

use aspen_ci::checkout::checkout_dir_for_run;
use aspen_ci::checkout::checkout_repository;
use aspen_ci::checkout::prepare_for_ci_build;
use aspen_ci::config::load_pipeline_config_str_async;
use aspen_ci::orchestrator::PipelineContext;
use aspen_client_rpc::CiCancelRunResponse;
use aspen_client_rpc::CiGetStatusResponse;
use aspen_client_rpc::CiJobInfo;
use aspen_client_rpc::CiListRunsResponse;
use aspen_client_rpc::CiRunInfo;
use aspen_client_rpc::CiStageInfo;
use aspen_client_rpc::CiTriggerPipelineResponse;
use aspen_client_rpc::CiUnwatchRepoResponse;
use aspen_client_rpc::CiWatchRepoResponse;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_forge::identity::RepoId;
use async_trait::async_trait;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::context::ClientProtocolContext;
use crate::registry::RequestHandler;

/// CI config file path within a repository.
const CI_CONFIG_PATH: &[&str] = &[".aspen", "ci.ncl"];

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
///
/// # Flow
///
/// 1. Parse repo_id from hex string
/// 2. Resolve ref to commit hash (if commit_hash not provided)
/// 3. Fetch `.aspen/ci.ncl` from the commit's tree
/// 4. Parse the CI config
/// 5. Create PipelineContext
/// 6. Execute pipeline via orchestrator
async fn handle_trigger_pipeline(
    ctx: &ClientProtocolContext,
    repo_id: String,
    ref_name: String,
    commit_hash_opt: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(orchestrator) = &ctx.ci_orchestrator else {
        return Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
            success: false,
            run_id: None,
            error: Some("CI orchestrator not available".to_string()),
        }));
    };

    let Some(forge_node) = &ctx.forge_node else {
        return Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
            success: false,
            run_id: None,
            error: Some("Forge not available - required for CI config".to_string()),
        }));
    };

    info!(repo_id = %repo_id, ref_name = %ref_name, "triggering CI pipeline");

    // Parse repo_id from hex string
    let repo_id_parsed = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
                success: false,
                run_id: None,
                error: Some(format!("Invalid repo_id: {}", e)),
            }));
        }
    };

    // Resolve commit hash - either use provided or resolve from ref
    let commit_hash = match commit_hash_opt {
        Some(hash_str) => {
            // Parse provided hash
            match parse_commit_hash(&hash_str) {
                Ok(hash) => hash,
                Err(e) => {
                    return Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
                        success: false,
                        run_id: None,
                        error: Some(format!("Invalid commit hash: {}", e)),
                    }));
                }
            }
        }
        None => {
            // Resolve from ref
            let ref_path = if ref_name.starts_with("refs/") {
                ref_name.strip_prefix("refs/").unwrap_or(&ref_name).to_string()
            } else {
                format!("heads/{}", ref_name)
            };

            match forge_node.refs.get(&repo_id_parsed, &ref_path).await {
                Ok(Some(hash)) => *hash.as_bytes(),
                Ok(None) => {
                    return Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
                        success: false,
                        run_id: None,
                        error: Some(format!("Ref '{}' not found in repository", ref_name)),
                    }));
                }
                Err(e) => {
                    return Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
                        success: false,
                        run_id: None,
                        error: Some(format!("Failed to resolve ref '{}': {}", ref_name, e)),
                    }));
                }
            }
        }
    };

    // Get the commit to find its tree
    let commit_hash_blake3 = blake3::Hash::from_bytes(commit_hash);
    let commit = match forge_node.git.get_commit(&commit_hash_blake3).await {
        Ok(c) => c,
        Err(e) => {
            return Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
                success: false,
                run_id: None,
                error: Some(format!("Failed to get commit: {}", e)),
            }));
        }
    };

    // Walk the tree to find .aspen/ci.ncl
    let ci_config_content = match walk_tree_for_file(&forge_node.git, &commit.tree(), CI_CONFIG_PATH).await {
        Ok(Some(content)) => content,
        Ok(None) => {
            return Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
                success: false,
                run_id: None,
                error: Some("CI config file (.aspen/ci.ncl) not found in repository".to_string()),
            }));
        }
        Err(e) => {
            return Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
                success: false,
                run_id: None,
                error: Some(format!("Failed to read CI config: {}", e)),
            }));
        }
    };

    // Parse the CI config
    let config_str = match String::from_utf8(ci_config_content) {
        Ok(s) => s,
        Err(e) => {
            return Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
                success: false,
                run_id: None,
                error: Some(format!("CI config is not valid UTF-8: {}", e)),
            }));
        }
    };

    // Use async version to run Nickel evaluation on a thread with large stack
    let pipeline_config = match load_pipeline_config_str_async(config_str, ".aspen/ci.ncl".to_string()).await {
        Ok(c) => c,
        Err(e) => {
            return Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
                success: false,
                run_id: None,
                error: Some(format!("Failed to parse CI config: {}", e)),
            }));
        }
    };

    // Generate checkout directory and perform checkout
    let run_id = uuid::Uuid::new_v4().to_string();
    let checkout_dir = checkout_dir_for_run(&run_id);

    info!(
        repo_id = %repo_id,
        commit = %hex::encode(commit_hash),
        checkout_dir = %checkout_dir.display(),
        "Checking out repository for CI"
    );

    if let Err(e) = checkout_repository(forge_node, &commit_hash, &checkout_dir).await {
        return Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
            success: false,
            run_id: None,
            error: Some(format!("Failed to checkout repository: {}", e)),
        }));
    }

    // Prepare checkout for CI build (removes path patches from .cargo/config.toml)
    if let Err(e) = prepare_for_ci_build(&checkout_dir).await {
        return Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
            success: false,
            run_id: None,
            error: Some(format!("Failed to prepare checkout for CI build: {}", e)),
        }));
    }

    // Build environment variables
    let mut env = HashMap::new();
    env.insert("CI_CHECKOUT_DIR".to_string(), checkout_dir.to_string_lossy().to_string());

    // Create pipeline context with checkout directory
    let context = PipelineContext {
        repo_id: repo_id_parsed,
        commit_hash,
        ref_name: ref_name.clone(),
        triggered_by: "rpc".to_string(), // Could be enhanced with auth info
        env,
        checkout_dir: Some(checkout_dir),
    };

    // Execute the pipeline
    let run = match orchestrator.execute(pipeline_config, context).await {
        Ok(r) => r,
        Err(e) => {
            return Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
                success: false,
                run_id: None,
                error: Some(format!("Failed to start pipeline: {}", e)),
            }));
        }
    };

    info!(run_id = %run.id, "CI pipeline started successfully");

    Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
        success: true,
        run_id: Some(run.id),
        error: None,
    }))
}

/// Handle CiGetStatus request.
///
/// Returns the current status of a pipeline run.
async fn handle_get_status(ctx: &ClientProtocolContext, run_id: String) -> anyhow::Result<ClientRpcResponse> {
    let Some(orchestrator) = &ctx.ci_orchestrator else {
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

    // Query the orchestrator
    let run = match orchestrator.get_run(&run_id).await {
        Some(r) => r,
        None => {
            return Ok(ClientRpcResponse::CiGetStatusResult(CiGetStatusResponse {
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
            }));
        }
    };

    // Convert internal types to RPC response types
    let status_str = pipeline_status_to_string(&run.status);

    let stages: Vec<CiStageInfo> = run
        .stages
        .iter()
        .map(|s| CiStageInfo {
            name: s.name.clone(),
            status: pipeline_status_to_string(&s.status),
            jobs: s
                .jobs
                .iter()
                .map(|(name, job)| CiJobInfo {
                    id: job.job_id.as_ref().map(|id| id.to_string()).unwrap_or_default(),
                    name: name.clone(),
                    status: pipeline_status_to_string(&job.status),
                    started_at_ms: job.started_at.map(|t| t.timestamp_millis() as u64),
                    ended_at_ms: job.completed_at.map(|t| t.timestamp_millis() as u64),
                    error: job.error.clone(),
                })
                .collect(),
        })
        .collect();

    Ok(ClientRpcResponse::CiGetStatusResult(CiGetStatusResponse {
        found: true,
        run_id: Some(run.id),
        repo_id: Some(run.context.repo_id.to_hex()),
        ref_name: Some(run.context.ref_name),
        commit_hash: Some(hex::encode(run.context.commit_hash)),
        status: Some(status_str),
        stages,
        created_at_ms: Some(run.created_at.timestamp_millis() as u64),
        completed_at_ms: run.completed_at.map(|t| t.timestamp_millis() as u64),
        error: None,
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
    let Some(orchestrator) = &ctx.ci_orchestrator else {
        return Ok(ClientRpcResponse::CiListRunsResult(CiListRunsResponse { runs: vec![] }));
    };

    let limit = limit.unwrap_or(50).min(500);
    debug!(?repo_id, ?status, limit, "listing CI pipeline runs");

    // Parse repo_id if provided
    let repo_id_parsed = if let Some(ref id_str) = repo_id {
        match RepoId::from_hex(id_str) {
            Ok(id) => Some(id),
            Err(e) => {
                warn!(repo_id = %id_str, error = %e, "Invalid repo_id in list request");
                return Ok(ClientRpcResponse::CiListRunsResult(CiListRunsResponse { runs: vec![] }));
            }
        }
    } else {
        None
    };

    // Query the orchestrator
    let runs = orchestrator.list_all_runs(repo_id_parsed.as_ref(), status.as_deref(), limit).await;

    // Convert to RPC response format
    let run_infos: Vec<CiRunInfo> = runs
        .into_iter()
        .map(|run| CiRunInfo {
            run_id: run.id,
            repo_id: run.context.repo_id.to_hex(),
            ref_name: run.context.ref_name,
            status: pipeline_status_to_string(&run.status),
            created_at_ms: run.created_at.timestamp_millis() as u64,
        })
        .collect();

    Ok(ClientRpcResponse::CiListRunsResult(CiListRunsResponse { runs: run_infos }))
}

/// Handle CiCancelRun request.
///
/// Cancels a running pipeline.
async fn handle_cancel_run(
    ctx: &ClientProtocolContext,
    run_id: String,
    reason: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(orchestrator) = &ctx.ci_orchestrator else {
        return Ok(ClientRpcResponse::CiCancelRunResult(CiCancelRunResponse {
            success: false,
            error: Some("CI orchestrator not available".to_string()),
        }));
    };

    info!(%run_id, ?reason, "cancelling CI pipeline");

    match orchestrator.cancel(&run_id).await {
        Ok(()) => Ok(ClientRpcResponse::CiCancelRunResult(CiCancelRunResponse {
            success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::CiCancelRunResult(CiCancelRunResponse {
            success: false,
            error: Some(format!("Failed to cancel pipeline: {}", e)),
        })),
    }
}

/// Handle CiWatchRepo request.
///
/// Subscribes to forge gossip events for automatic CI triggering.
async fn handle_watch_repo(ctx: &ClientProtocolContext, repo_id: String) -> anyhow::Result<ClientRpcResponse> {
    let Some(trigger_service) = &ctx.ci_trigger_service else {
        return Ok(ClientRpcResponse::CiWatchRepoResult(CiWatchRepoResponse {
            success: false,
            error: Some("CI trigger service not available".to_string()),
        }));
    };

    info!(%repo_id, "watching repository for CI triggers");

    // Parse repo_id from hex string
    let repo_id_parsed = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::CiWatchRepoResult(CiWatchRepoResponse {
                success: false,
                error: Some(format!("Invalid repo_id: {}", e)),
            }));
        }
    };

    // Register repository for CI auto-triggering
    match trigger_service.watch_repo(repo_id_parsed).await {
        Ok(()) => Ok(ClientRpcResponse::CiWatchRepoResult(CiWatchRepoResponse {
            success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::CiWatchRepoResult(CiWatchRepoResponse {
            success: false,
            error: Some(format!("Failed to watch repository: {}", e)),
        })),
    }
}

/// Handle CiUnwatchRepo request.
///
/// Removes CI trigger subscription for a repository.
async fn handle_unwatch_repo(ctx: &ClientProtocolContext, repo_id: String) -> anyhow::Result<ClientRpcResponse> {
    let Some(trigger_service) = &ctx.ci_trigger_service else {
        return Ok(ClientRpcResponse::CiUnwatchRepoResult(CiUnwatchRepoResponse {
            success: false,
            error: Some("CI trigger service not available".to_string()),
        }));
    };

    info!(%repo_id, "unwatching repository");

    // Parse repo_id from hex string
    let repo_id_parsed = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::CiUnwatchRepoResult(CiUnwatchRepoResponse {
                success: false,
                error: Some(format!("Invalid repo_id: {}", e)),
            }));
        }
    };

    // Unregister repository - this operation always succeeds
    trigger_service.unwatch_repo(&repo_id_parsed).await;

    Ok(ClientRpcResponse::CiUnwatchRepoResult(CiUnwatchRepoResponse {
        success: true,
        error: None,
    }))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Walk a tree recursively to find a file by path components.
///
/// Returns the file content as bytes if found, None if not found.
async fn walk_tree_for_file<B: aspen_blob::BlobStore>(
    git: &aspen_forge::git::GitBlobStore<B>,
    root_tree_hash: &blake3::Hash,
    path: &[&str],
) -> Result<Option<Vec<u8>>, anyhow::Error> {
    if path.is_empty() {
        return Ok(None);
    }

    let mut current_hash = *root_tree_hash;

    // Walk through each path component
    for (i, part) in path.iter().enumerate() {
        let tree = git.get_tree(&current_hash).await?;

        // Find entry with matching name
        let entry = match tree.entries.iter().find(|e| e.name == *part) {
            Some(e) => e,
            None => return Ok(None), // Path component not found
        };

        if i == path.len() - 1 {
            // Last component - should be a file
            if entry.is_file() {
                let content = git.get_blob(&entry.hash()).await?;
                return Ok(Some(content));
            } else {
                // Expected file but found directory
                return Ok(None);
            }
        } else {
            // Intermediate component - should be a directory
            if entry.is_directory() {
                current_hash = entry.hash();
            } else {
                // Expected directory but found file
                return Ok(None);
            }
        }
    }

    Ok(None)
}

/// Parse a commit hash from hex string to [u8; 32].
fn parse_commit_hash(hex_str: &str) -> Result<[u8; 32], anyhow::Error> {
    let bytes = hex::decode(hex_str)?;
    if bytes.len() != 32 {
        anyhow::bail!("commit hash must be 32 bytes (64 hex chars), got {}", bytes.len());
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

/// Convert PipelineStatus to string representation.
fn pipeline_status_to_string(status: &aspen_ci::orchestrator::PipelineStatus) -> String {
    match status {
        aspen_ci::orchestrator::PipelineStatus::Pending => "pending".to_string(),
        aspen_ci::orchestrator::PipelineStatus::Running => "running".to_string(),
        aspen_ci::orchestrator::PipelineStatus::Success => "success".to_string(),
        aspen_ci::orchestrator::PipelineStatus::Failed => "failed".to_string(),
        aspen_ci::orchestrator::PipelineStatus::Cancelled => "cancelled".to_string(),
    }
}
