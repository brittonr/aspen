//! CI/CD pipeline RPC handlers.
//!
//! Handles pipeline triggering, status queries, and repository watching
//! through the aspen-ci orchestrator and trigger service.

use std::collections::HashMap;
#[cfg(feature = "blob")]
use std::str::FromStr;

#[cfg(all(feature = "forge", feature = "blob"))]
use aspen_ci::checkout::checkout_dir_for_run;
#[cfg(all(feature = "forge", feature = "blob"))]
use aspen_ci::checkout::checkout_repository;
#[cfg(all(feature = "forge", feature = "blob"))]
use aspen_ci::checkout::cleanup_checkout;
#[cfg(all(feature = "forge", feature = "blob"))]
use aspen_ci::checkout::prepare_for_ci_build;
#[cfg(all(feature = "forge", feature = "blob"))]
use aspen_ci::config::load_pipeline_config_str_async;
#[cfg(all(feature = "forge", feature = "blob"))]
use aspen_ci::orchestrator::PipelineContext;
use aspen_client_api::CiArtifactInfo;
use aspen_client_api::CiCancelRunResponse;
use aspen_client_api::CiGetArtifactResponse;
use aspen_client_api::CiGetJobLogsResponse;
use aspen_client_api::CiGetJobOutputResponse;
use aspen_client_api::CiGetStatusResponse;
use aspen_client_api::CiJobInfo;
use aspen_client_api::CiListArtifactsResponse;
use aspen_client_api::CiListRunsResponse;
use aspen_client_api::CiLogChunkInfo;
#[cfg(feature = "forge")]
use aspen_client_api::CiRunInfo;
use aspen_client_api::CiStageInfo;
use aspen_client_api::CiSubscribeLogsResponse;
use aspen_client_api::CiTriggerPipelineResponse;
use aspen_client_api::CiUnwatchRepoResponse;
use aspen_client_api::CiWatchRepoResponse;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_core::CI_LOG_COMPLETE_MARKER;
use aspen_core::CI_LOG_KV_PREFIX;
use aspen_core::DEFAULT_CI_LOG_FETCH_CHUNKS;
use aspen_core::MAX_CI_LOG_FETCH_CHUNKS;
#[cfg(feature = "forge")]
use aspen_forge::identity::RepoId;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use async_trait::async_trait;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// CI config file path within a repository.
#[cfg(all(feature = "forge", feature = "blob"))]
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
            #[cfg(all(feature = "forge", feature = "blob"))]
            ClientRpcRequest::CiTriggerPipeline {
                repo_id,
                ref_name,
                commit_hash,
            } => handle_trigger_pipeline(ctx, repo_id, ref_name, commit_hash).await,
            #[cfg(not(all(feature = "forge", feature = "blob")))]
            ClientRpcRequest::CiTriggerPipeline { .. } => {
                Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
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
                Ok(ClientRpcResponse::CiListRunsResult(CiListRunsResponse { runs: vec![] }))
            }
            ClientRpcRequest::CiCancelRun { run_id, reason } => handle_cancel_run(ctx, run_id, reason).await,
            #[cfg(feature = "forge")]
            ClientRpcRequest::CiWatchRepo { repo_id } => handle_watch_repo(ctx, repo_id).await,
            #[cfg(not(feature = "forge"))]
            ClientRpcRequest::CiWatchRepo { .. } => Ok(ClientRpcResponse::CiWatchRepoResult(CiWatchRepoResponse {
                success: false,
                error: Some("CI watch requires forge feature".to_string()),
            })),
            #[cfg(feature = "forge")]
            ClientRpcRequest::CiUnwatchRepo { repo_id } => handle_unwatch_repo(ctx, repo_id).await,
            #[cfg(not(feature = "forge"))]
            ClientRpcRequest::CiUnwatchRepo { .. } => {
                Ok(ClientRpcResponse::CiUnwatchRepoResult(CiUnwatchRepoResponse {
                    success: false,
                    error: Some("CI unwatch requires forge feature".to_string()),
                }))
            }
            ClientRpcRequest::CiListArtifacts { job_id, run_id } => handle_list_artifacts(ctx, job_id, run_id).await,
            ClientRpcRequest::CiGetArtifact { blob_hash } => handle_get_artifact(ctx, blob_hash).await,
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
#[cfg(all(feature = "forge", feature = "blob"))]
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
        // Clean up partial checkout directory
        let _ = cleanup_checkout(&checkout_dir).await;
        return Ok(ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
            success: false,
            run_id: None,
            error: Some(format!("Failed to checkout repository: {}", e)),
        }));
    }

    // Prepare checkout for CI build (removes path patches from .cargo/config.toml)
    if let Err(e) = prepare_for_ci_build(&checkout_dir).await {
        // Clean up failed checkout directory
        let _ = cleanup_checkout(&checkout_dir).await;
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
    // Note: source_hash is not set here because this is a direct RPC call without blob store.
    // VM jobs triggered via this path will need checkout_dir to be accessible or use
    // the OrchestratorPipelineStarter adapter which creates source archives.
    let context = PipelineContext {
        repo_id: repo_id_parsed,
        commit_hash,
        ref_name: ref_name.clone(),
        triggered_by: "rpc".to_string(), // Could be enhanced with auth info
        env,
        checkout_dir: Some(checkout_dir),
        source_hash: None, // VM jobs may fail without source_hash
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
#[cfg(feature = "forge")]
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
/// This performs two operations:
/// 1. Registers the repo with TriggerService (in-memory watch list)
/// 2. Subscribes to the repo's gossip topic for multi-node announcements
#[cfg(feature = "forge")]
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

    // Step 1: Register with TriggerService (in-memory watch list)
    if let Err(e) = trigger_service.watch_repo(repo_id_parsed).await {
        return Ok(ClientRpcResponse::CiWatchRepoResult(CiWatchRepoResponse {
            success: false,
            error: Some(format!("Failed to watch repository: {}", e)),
        }));
    }

    // Step 2: Subscribe to gossip topic for multi-node announcements
    #[cfg(feature = "forge")]
    if let Some(forge_node) = &ctx.forge_node {
        if let Err(e) = forge_node.subscribe_repo_gossip(&repo_id_parsed).await {
            warn!(
                repo_id = %repo_id,
                error = %e,
                "Failed to subscribe to repo gossip (multi-node triggers may not work)"
            );
            // Continue - local triggers will still work
        } else {
            info!(repo_id = %repo_id, "subscribed to repo gossip for CI triggers");
        }
    }

    // Verify and log the watch status
    let is_watching = trigger_service.is_watching(&repo_id_parsed).await;
    let watched_count = trigger_service.watched_count().await;
    info!(
        repo_id = %repo_id,
        is_watching = is_watching,
        watched_count = watched_count,
        "CI watch registered and verified"
    );

    Ok(ClientRpcResponse::CiWatchRepoResult(CiWatchRepoResponse {
        success: true,
        error: None,
    }))
}

/// Handle CiUnwatchRepo request.
///
/// Removes CI trigger subscription for a repository.
/// This performs two operations:
/// 1. Unregisters the repo from TriggerService
/// 2. Unsubscribes from the repo's gossip topic
#[cfg(feature = "forge")]
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

    // Step 1: Unregister from TriggerService
    trigger_service.unwatch_repo(&repo_id_parsed).await;

    // Step 2: Unsubscribe from gossip topic
    #[cfg(feature = "forge")]
    if let Some(forge_node) = &ctx.forge_node {
        if let Err(e) = forge_node.unsubscribe_repo_gossip(&repo_id_parsed).await {
            warn!(
                repo_id = %repo_id,
                error = %e,
                "Failed to unsubscribe from repo gossip"
            );
            // Continue - the watch is already removed
        } else {
            debug!(repo_id = %repo_id, "unsubscribed from repo gossip");
        }
    }

    info!(repo_id = %repo_id, "CI watch removed");

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
#[cfg(all(feature = "forge", feature = "blob"))]
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
#[cfg(all(feature = "forge", feature = "blob"))]
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
        aspen_ci::orchestrator::PipelineStatus::Initializing => "initializing".to_string(),
        aspen_ci::orchestrator::PipelineStatus::CheckingOut => "checking_out".to_string(),
        aspen_ci::orchestrator::PipelineStatus::CheckoutFailed => "checkout_failed".to_string(),
        aspen_ci::orchestrator::PipelineStatus::Pending => "pending".to_string(),
        aspen_ci::orchestrator::PipelineStatus::Running => "running".to_string(),
        aspen_ci::orchestrator::PipelineStatus::Success => "success".to_string(),
        aspen_ci::orchestrator::PipelineStatus::Failed => "failed".to_string(),
        aspen_ci::orchestrator::PipelineStatus::Cancelled => "cancelled".to_string(),
    }
}

// ============================================================================
// Artifact Handlers
// ============================================================================

/// Handle CiListArtifacts request.
///
/// Lists artifacts produced by a CI job. Artifacts are stored in the KV store
/// with metadata and blob hashes for the actual content in the blob store.
async fn handle_list_artifacts(
    ctx: &ClientProtocolContext,
    job_id: String,
    run_id: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_core::ScanRequest;

    info!(%job_id, ?run_id, "listing CI artifacts");

    // Scan for artifacts associated with this job
    // Artifact metadata is stored under: _ci:artifacts:{job_id}:{artifact_name}
    let prefix = format!("_ci:artifacts:{}:", job_id);

    let scan_result = ctx
        .kv_store
        .scan(ScanRequest {
            prefix,
            limit: Some(100), // Tiger Style: bounded results
            continuation_token: None,
        })
        .await;

    let entries = match scan_result {
        Ok(result) => result.entries,
        Err(e) => {
            warn!(job_id = %job_id, error = %e, "failed to scan artifacts");
            return Ok(ClientRpcResponse::CiListArtifactsResult(CiListArtifactsResponse {
                success: false,
                artifacts: vec![],
                error: Some(format!("Failed to list artifacts: {}", e)),
            }));
        }
    };

    // Parse artifact metadata from KV entries
    let mut artifacts = Vec::new();
    for entry in entries {
        // Try to parse as artifact metadata
        if let Ok(metadata) = serde_json::from_str::<ArtifactMetadata>(&entry.value) {
            // Filter by run_id if specified
            if let Some(ref filter_run_id) = run_id
                && metadata.run_id.as_deref() != Some(filter_run_id.as_str())
            {
                continue;
            }

            artifacts.push(CiArtifactInfo {
                blob_hash: metadata.blob_hash,
                name: metadata.name,
                size_bytes: metadata.size_bytes,
                content_type: metadata.content_type,
                created_at: metadata.created_at,
                metadata: metadata.extra,
            });
        }
    }

    info!(job_id = %job_id, count = artifacts.len(), "found artifacts");

    Ok(ClientRpcResponse::CiListArtifactsResult(CiListArtifactsResponse {
        success: true,
        artifacts,
        error: None,
    }))
}

/// Handle CiGetArtifact request.
///
/// Returns artifact metadata and a blob ticket for downloading.
async fn handle_get_artifact(ctx: &ClientProtocolContext, blob_hash: String) -> anyhow::Result<ClientRpcResponse> {
    info!(%blob_hash, "getting CI artifact");

    // Look up artifact metadata by blob hash
    // We scan for any artifact with this blob_hash since we don't know the job_id
    let prefix = "_ci:artifacts:".to_string();

    let scan_result = ctx
        .kv_store
        .scan(aspen_core::ScanRequest {
            prefix,
            limit: Some(1000), // Tiger Style: bounded search
            continuation_token: None,
        })
        .await;

    let entries = match scan_result {
        Ok(result) => result.entries,
        Err(e) => {
            warn!(blob_hash = %blob_hash, error = %e, "failed to scan for artifact");
            return Ok(ClientRpcResponse::CiGetArtifactResult(CiGetArtifactResponse {
                success: false,
                artifact: None,
                blob_ticket: None,
                error: Some(format!("Failed to find artifact: {}", e)),
            }));
        }
    };

    // Find the artifact with matching blob_hash
    let mut found_artifact = None;
    for entry in entries {
        if let Ok(metadata) = serde_json::from_str::<ArtifactMetadata>(&entry.value)
            && metadata.blob_hash == blob_hash
        {
            found_artifact = Some(CiArtifactInfo {
                blob_hash: metadata.blob_hash,
                name: metadata.name,
                size_bytes: metadata.size_bytes,
                content_type: metadata.content_type,
                created_at: metadata.created_at,
                metadata: metadata.extra,
            });
            break;
        }
    }

    let Some(artifact) = found_artifact else {
        return Ok(ClientRpcResponse::CiGetArtifactResult(CiGetArtifactResponse {
            success: false,
            artifact: None,
            blob_ticket: None,
            error: Some(format!("Artifact not found: {}", blob_hash)),
        }));
    };

    // Generate blob ticket for download
    #[cfg(feature = "blob")]
    let blob_ticket = if let Some(blob_store) = &ctx.blob_store {
        use aspen_blob::BlobStore;
        // Parse blob hash and generate ticket
        match iroh_blobs::Hash::from_str(&blob_hash) {
            Ok(hash) => {
                // Get blob ticket from the blob store
                match blob_store.ticket(&hash).await {
                    Ok(ticket) => Some(ticket.to_string()),
                    Err(e) => {
                        warn!(blob_hash = %blob_hash, error = %e, "failed to generate blob ticket");
                        None
                    }
                }
            }
            Err(e) => {
                warn!(blob_hash = %blob_hash, error = %e, "invalid blob hash format");
                None
            }
        }
    } else {
        None
    };

    #[cfg(not(feature = "blob"))]
    let blob_ticket: Option<String> = None;

    info!(blob_hash = %blob_hash, has_ticket = blob_ticket.is_some(), "artifact found");

    Ok(ClientRpcResponse::CiGetArtifactResult(CiGetArtifactResponse {
        success: true,
        artifact: Some(artifact),
        blob_ticket,
        error: None,
    }))
}

/// Internal artifact metadata structure stored in KV.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ArtifactMetadata {
    /// Blob hash in the distributed store.
    blob_hash: String,
    /// Artifact name (e.g., store path for Nix builds).
    name: String,
    /// Size in bytes.
    size_bytes: u64,
    /// Content type (e.g., "application/x-nix-nar").
    content_type: String,
    /// When the artifact was created (ISO 8601).
    created_at: String,
    /// Optional run_id for filtering.
    run_id: Option<String>,
    /// Additional metadata.
    #[serde(default)]
    extra: HashMap<String, String>,
}

// ============================================================================
// CI Log Handlers
// ============================================================================

/// Handle CiGetJobLogs request.
///
/// Fetches historical log chunks for a CI job from the KV store.
/// Logs are stored with keys: `_ci:logs:{run_id}:{job_id}:{chunk_index:010}`
async fn handle_get_job_logs(
    ctx: &ClientProtocolContext,
    run_id: String,
    job_id: String,
    start_index: u32,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_ci::log_writer::CiLogChunk;
    use aspen_ci::log_writer::CiLogCompleteMarker;
    use aspen_core::ScanRequest;

    let limit = limit.unwrap_or(DEFAULT_CI_LOG_FETCH_CHUNKS).min(MAX_CI_LOG_FETCH_CHUNKS);
    debug!(%run_id, %job_id, start_index, limit, "getting CI job logs");

    // Build the scan prefix for log chunks
    // Key format: _ci:logs:{run_id}:{job_id}:{chunk_index:010}
    let prefix = format!("{}{}:{}:", CI_LOG_KV_PREFIX, run_id, job_id);

    // Build the start key with zero-padded index
    let start_key = format!("{}{:010}", prefix, start_index);

    // Scan for log chunks starting from the requested index
    let scan_result = ctx
        .kv_store
        .scan(ScanRequest {
            prefix: start_key,
            limit: Some(limit),
            continuation_token: None,
        })
        .await;

    let entries = match scan_result {
        Ok(result) => result.entries,
        Err(e) => {
            warn!(run_id = %run_id, job_id = %job_id, error = %e, "failed to scan job logs");
            return Ok(ClientRpcResponse::CiGetJobLogsResult(CiGetJobLogsResponse {
                found: false,
                chunks: vec![],
                last_index: 0,
                has_more: false,
                is_complete: false,
                error: Some(format!("Failed to scan logs: {}", e)),
            }));
        }
    };

    // If no entries, check if any logs exist at all
    if entries.is_empty() && start_index == 0 {
        // Check if the log prefix exists at all
        let check_prefix = format!("{}{}:{}:", CI_LOG_KV_PREFIX, run_id, job_id);
        let check_result = ctx
            .kv_store
            .scan(ScanRequest {
                prefix: check_prefix,
                limit: Some(1),
                continuation_token: None,
            })
            .await;

        if check_result.is_ok() && check_result.unwrap().entries.is_empty() {
            // No logs exist for this job
            return Ok(ClientRpcResponse::CiGetJobLogsResult(CiGetJobLogsResponse {
                found: false,
                chunks: vec![],
                last_index: 0,
                has_more: false,
                is_complete: false,
                error: None,
            }));
        }
    }

    // Parse log chunks from entries
    let mut chunks = Vec::with_capacity(entries.len());
    let mut last_index = start_index;

    for entry in entries {
        // Skip completion marker
        if entry.key.ends_with(CI_LOG_COMPLETE_MARKER) {
            continue;
        }

        // Parse the chunk JSON
        match serde_json::from_str::<CiLogChunk>(&entry.value) {
            Ok(chunk) => {
                last_index = chunk.index;
                chunks.push(CiLogChunkInfo {
                    index: chunk.index,
                    content: chunk.content,
                    timestamp_ms: chunk.timestamp_ms,
                });
            }
            Err(e) => {
                warn!(
                    run_id = %run_id,
                    job_id = %job_id,
                    key = %entry.key,
                    error = %e,
                    "failed to parse log chunk"
                );
            }
        }
    }

    // Check for completion marker
    let completion_key = format!("{}{}:{}:{}", CI_LOG_KV_PREFIX, run_id, job_id, CI_LOG_COMPLETE_MARKER);
    let is_complete = match ctx.kv_store.read(aspen_core::ReadRequest::new(completion_key)).await {
        Ok(result) => {
            if let Some(kv) = result.kv {
                // Parse marker to verify it's valid
                serde_json::from_str::<CiLogCompleteMarker>(&kv.value).is_ok()
            } else {
                false
            }
        }
        Err(_) => false,
    };

    // Determine if there are more chunks
    let has_more = chunks.len() as u32 >= limit;

    info!(
        run_id = %run_id,
        job_id = %job_id,
        chunk_count = chunks.len(),
        last_index = last_index,
        is_complete = is_complete,
        "retrieved CI job logs"
    );

    Ok(ClientRpcResponse::CiGetJobLogsResult(CiGetJobLogsResponse {
        found: true,
        chunks,
        last_index,
        has_more,
        is_complete,
        error: None,
    }))
}

/// Handle CiSubscribeLogs request.
///
/// Returns information for subscribing to real-time log updates via WatchSession.
/// The client should use LOG_SUBSCRIBER_ALPN to watch the returned prefix.
async fn handle_subscribe_logs(
    ctx: &ClientProtocolContext,
    run_id: String,
    job_id: String,
    from_index: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_ci::log_writer::CiLogChunk;
    use aspen_core::ScanRequest;

    debug!(%run_id, %job_id, ?from_index, "subscribing to CI job logs");

    // Build the watch prefix
    let watch_prefix = format!("{}{}:{}:", CI_LOG_KV_PREFIX, run_id, job_id);

    // Determine current log index by scanning for existing chunks
    let current_index: u64 = if let Some(idx) = from_index {
        idx
    } else {
        // Find the latest chunk index
        let scan_result = ctx
            .kv_store
            .scan(ScanRequest {
                prefix: watch_prefix.clone(),
                limit: Some(MAX_CI_LOG_FETCH_CHUNKS),
                continuation_token: None,
            })
            .await;

        match scan_result {
            Ok(result) => {
                // Find the highest chunk index
                result
                    .entries
                    .iter()
                    .filter_map(|e| {
                        if e.key.ends_with(CI_LOG_COMPLETE_MARKER) {
                            return None;
                        }
                        serde_json::from_str::<CiLogChunk>(&e.value).ok().map(|c| u64::from(c.index))
                    })
                    .max()
                    .unwrap_or(0)
            }
            Err(_) => 0,
        }
    };

    // Check if the job is still running by looking for completion marker
    let completion_key = format!("{}{}:{}:{}", CI_LOG_KV_PREFIX, run_id, job_id, CI_LOG_COMPLETE_MARKER);
    let is_running = match ctx.kv_store.read(aspen_core::ReadRequest::new(completion_key)).await {
        Ok(result) => result.kv.is_none(),
        Err(_) => true, // Assume running if we can't check
    };

    // Check if the job exists at all
    let check_prefix = format!("{}{}:{}:", CI_LOG_KV_PREFIX, run_id, job_id);
    let found = match ctx
        .kv_store
        .scan(ScanRequest {
            prefix: check_prefix,
            limit: Some(1),
            continuation_token: None,
        })
        .await
    {
        Ok(result) => !result.entries.is_empty(),
        Err(_) => false,
    };

    info!(
        run_id = %run_id,
        job_id = %job_id,
        watch_prefix = %watch_prefix,
        current_index = current_index,
        is_running = is_running,
        found = found,
        "CI log subscription info prepared"
    );

    Ok(ClientRpcResponse::CiSubscribeLogsResult(CiSubscribeLogsResponse {
        found,
        watch_prefix,
        current_index,
        is_running,
        error: None,
    }))
}

/// Handle CiGetJobOutput request.
///
/// Returns full stdout/stderr for a completed job, resolving blob references if needed.
async fn handle_get_job_output(
    ctx: &ClientProtocolContext,
    _run_id: String,
    job_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    #[cfg(feature = "blob")]
    use aspen_blob::BlobStore;
    use aspen_jobs::OutputRef;

    // Job queue prefix (from aspen-jobs/src/manager.rs)
    const JOB_PREFIX: &str = "__jobs:";

    debug!(%job_id, "getting CI job output from job queue");

    // Look up the job directly from the job queue
    let job_key = format!("{}{}", JOB_PREFIX, job_id);
    let job_result = ctx.kv_store.read(aspen_core::ReadRequest::new(job_key)).await;

    let job_data_str = match job_result {
        Ok(result) => match result.kv {
            Some(kv) => kv.value,
            None => {
                return Ok(ClientRpcResponse::CiGetJobOutputResult(CiGetJobOutputResponse {
                    found: false,
                    stdout: None,
                    stderr: None,
                    stdout_was_blob: false,
                    stderr_was_blob: false,
                    stdout_size: 0,
                    stderr_size: 0,
                    error: Some(format!("Job {} not found in job queue", job_id)),
                }));
            }
        },
        Err(e) => {
            return Ok(ClientRpcResponse::CiGetJobOutputResult(CiGetJobOutputResponse {
                found: false,
                stdout: None,
                stderr: None,
                stdout_was_blob: false,
                stderr_was_blob: false,
                stdout_size: 0,
                stderr_size: 0,
                error: Some(format!("Failed to read job from queue: {}", e)),
            }));
        }
    };

    // Parse the job data
    let job_json: serde_json::Value = match serde_json::from_str(&job_data_str) {
        Ok(v) => v,
        Err(e) => {
            return Ok(ClientRpcResponse::CiGetJobOutputResult(CiGetJobOutputResponse {
                found: false,
                stdout: None,
                stderr: None,
                stdout_was_blob: false,
                stderr_was_blob: false,
                stdout_size: 0,
                stderr_size: 0,
                error: Some(format!("Failed to parse job data: {}", e)),
            }));
        }
    };

    // Navigate to find the job result: result -> Success -> data
    // The job structure is: { status, result: { Success: { data: {...} } }, ... }
    let job_data = job_json
        .get("result")
        .and_then(|result| result.get("Success"))
        .and_then(|success| success.get("data"));

    let job_data = match job_data {
        Some(data) => data,
        None => {
            // Check if job is still running or failed
            let status = job_json.get("status").and_then(|s| s.as_str()).unwrap_or("unknown");
            let error_msg = if status == "processing" || status == "pending" {
                format!("Job {} is still running (status: {})", job_id, status)
            } else {
                format!("Job {} not completed successfully (status: {})", job_id, status)
            };
            return Ok(ClientRpcResponse::CiGetJobOutputResult(CiGetJobOutputResponse {
                found: false,
                stdout: None,
                stderr: None,
                stdout_was_blob: false,
                stderr_was_blob: false,
                stdout_size: 0,
                stderr_size: 0,
                error: Some(error_msg),
            }));
        }
    };

    // Extract stdout and stderr references
    let stdout_value = job_data.get("stdout");
    let stderr_value = job_data.get("stderr");
    let stdout_full_size = job_data.get("stdout_full_size").and_then(|v| v.as_u64()).unwrap_or(0);
    let stderr_full_size = job_data.get("stderr_full_size").and_then(|v| v.as_u64()).unwrap_or(0);

    // Helper to resolve output (inline string or blob reference)
    async fn resolve_output(_ctx: &ClientProtocolContext, value: Option<&serde_json::Value>) -> (Option<String>, bool) {
        let Some(value) = value else {
            return (None, false);
        };

        // Try to parse as OutputRef
        let output_ref: OutputRef = match serde_json::from_value(value.clone()) {
            Ok(r) => r,
            Err(_) => {
                // Might be a plain string (old format)
                if let Some(s) = value.as_str() {
                    return (Some(s.to_string()), false);
                }
                return (None, false);
            }
        };

        match output_ref {
            OutputRef::Inline(s) => (Some(s), false),
            OutputRef::Blob { hash, .. } => {
                // Resolve from blob store
                #[cfg(feature = "blob")]
                {
                    if let Some(ref blob_store) = ctx.blob_store {
                        let hash = match iroh_blobs::Hash::from_str(&hash) {
                            Ok(h) => h,
                            Err(_) => return (Some(format!("[blob {} - invalid hash]", hash)), true),
                        };
                        match blob_store.get_bytes(&hash).await {
                            Ok(Some(bytes)) => {
                                let content = String::from_utf8_lossy(&bytes).to_string();
                                return (Some(content), true);
                            }
                            Ok(None) => {
                                return (Some(format!("[blob {} not found]", hash)), true);
                            }
                            Err(e) => {
                                return (Some(format!("[blob {} - error: {}]", hash, e)), true);
                            }
                        }
                    }
                }
                (Some(format!("[blob {} - blob store not available]", hash)), true)
            }
        }
    }

    let (stdout, stdout_was_blob) = resolve_output(ctx, stdout_value).await;
    let (stderr, stderr_was_blob) = resolve_output(ctx, stderr_value).await;

    info!(
        %job_id,
        stdout_was_blob = stdout_was_blob,
        stderr_was_blob = stderr_was_blob,
        stdout_size = stdout_full_size,
        stderr_size = stderr_full_size,
        "retrieved CI job output"
    );

    Ok(ClientRpcResponse::CiGetJobOutputResult(CiGetJobOutputResponse {
        found: true,
        stdout,
        stderr,
        stdout_was_blob,
        stderr_was_blob,
        stdout_size: stdout_full_size,
        stderr_size: stderr_full_size,
        error: None,
    }))
}
