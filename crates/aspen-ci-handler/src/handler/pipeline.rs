//! Pipeline operations: trigger, get_status, list_runs, cancel.

#[cfg(all(feature = "forge", feature = "blob"))]
use std::collections::HashMap;
use std::sync::Arc;

use aspen_client_api::CiCancelRunResponse;
use aspen_client_api::CiGetStatusResponse;
use aspen_client_api::CiJobInfo;
use aspen_client_api::CiStageInfo;
#[cfg(all(feature = "forge", feature = "blob"))]
use aspen_client_api::CiTriggerPipelineResponse;
use aspen_client_api::ClientRpcResponse;
use tracing::debug;
use tracing::info;
#[cfg(all(feature = "forge", feature = "blob"))]
use tracing::warn;

/// Type alias for forge node to match executor.
#[cfg(all(feature = "forge", feature = "blob"))]
pub type ForgeNodeRef = Arc<aspen_forge::ForgeNode<aspen_blob::IrohBlobStore, dyn aspen_core::KeyValueStore>>;

#[cfg(all(feature = "forge", feature = "blob"))]
fn ci_trigger_error_response(message: impl Into<String>) -> ClientRpcResponse {
    ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
        is_success: false,
        run_id: None,
        error: Some(message.into()),
    })
}

#[cfg(all(feature = "forge", feature = "blob"))]
fn ci_trigger_success_response(run_id: String) -> ClientRpcResponse {
    ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
        is_success: true,
        run_id: Some(run_id),
        error: None,
    })
}

#[cfg(all(feature = "forge", feature = "blob"))]
#[allow(clippy::result_large_err)]
fn parse_trigger_repo_id(repo_id: &str) -> Result<aspen_forge::identity::RepoId, ClientRpcResponse> {
    use aspen_forge::identity::RepoId;

    let parsed =
        RepoId::from_hex(repo_id).map_err(|error| ci_trigger_error_response(format!("Invalid repo_id: {}", error)))?;
    debug_assert_eq!(RepoId::from_hex(&parsed.to_hex()).ok(), Some(parsed));
    Ok(parsed)
}

#[cfg(all(feature = "forge", feature = "blob"))]
#[allow(clippy::result_large_err)]
fn parse_requested_commit_hash(commit_hash: Option<&str>) -> Result<Option<[u8; 32]>, ClientRpcResponse> {
    use super::helpers::parse_commit_hash;

    commit_hash
        .map(parse_commit_hash)
        .transpose()
        .map_err(|error| ci_trigger_error_response(format!("Invalid commit hash: {}", error)))
}

#[cfg(all(feature = "forge", feature = "blob"))]
fn normalize_trigger_ref_name(ref_name: &str) -> String {
    let ref_path = if ref_name.starts_with("refs/") {
        ref_name.strip_prefix("refs/").unwrap_or(ref_name).to_string()
    } else if ref_name.starts_with("heads/") || ref_name.starts_with("tags/") {
        ref_name.to_string()
    } else {
        format!("heads/{ref_name}")
    };
    debug_assert!(!ref_path.starts_with("refs/"));
    debug_assert!(!ref_path.is_empty());
    ref_path
}

#[cfg(all(feature = "forge", feature = "blob"))]
async fn resolve_trigger_commit_hash(
    forge_node: &ForgeNodeRef,
    repo_id: &str,
    repo_id_parsed: &aspen_forge::identity::RepoId,
    ref_name: &str,
    requested_commit_hash: Option<[u8; 32]>,
) -> Result<[u8; 32], ClientRpcResponse> {
    if let Some(commit_hash) = requested_commit_hash {
        return Ok(commit_hash);
    }

    let ref_path = normalize_trigger_ref_name(ref_name);
    match forge_node.refs.get(repo_id_parsed, &ref_path).await {
        Ok(Some(hash)) => {
            info!(
                repo_id = %repo_id,
                ref_path = %ref_path,
                commit = %hash,
                "ci-trigger: ref resolved to commit"
            );
            Ok(*hash.as_bytes())
        }
        Ok(None) => Err(ci_trigger_error_response(format!("Ref '{}' not found in repository", ref_name))),
        Err(error) => Err(ci_trigger_error_response(format!("Failed to resolve ref '{}': {}", ref_name, error))),
    }
}

#[cfg(all(feature = "forge", feature = "blob"))]
async fn load_trigger_pipeline_config(
    forge_node: &ForgeNodeRef,
    repo_id: &str,
    commit_hash: [u8; 32],
) -> Result<aspen_ci::config::types::PipelineConfig, ClientRpcResponse> {
    use aspen_ci::config::load_pipeline_config_str_async;

    use super::helpers::CI_CONFIG_PATH;
    use super::helpers::walk_tree_for_file;

    let commit_hash_blake3 = blake3::Hash::from_bytes(commit_hash);
    info!(commit = %commit_hash_blake3, "ci-trigger: fetching commit object");
    let commit = forge_node
        .git
        .get_commit(&commit_hash_blake3)
        .await
        .map_err(|error| ci_trigger_error_response(format!("Failed to get commit: {}", error)))?;

    info!(
        commit = %commit_hash_blake3,
        tree = %commit.tree(),
        "ci-trigger: commit resolved, walking tree for .aspen/ci.ncl"
    );
    let ci_config_content = match walk_tree_for_file(&forge_node.git, &commit.tree(), CI_CONFIG_PATH).await {
        Ok(Some(content)) => content,
        Ok(None) => {
            warn!(
                repo_id = %repo_id,
                tree = %commit.tree(),
                "ci-trigger: .aspen/ci.ncl NOT FOUND in commit tree"
            );
            return Err(ci_trigger_error_response("CI config file (.aspen/ci.ncl) not found in repository"));
        }
        Err(error) => {
            warn!(repo_id = %repo_id, error = %error, "ci-trigger: error reading CI config from tree");
            return Err(ci_trigger_error_response(format!("Failed to read CI config: {}", error)));
        }
    };

    info!(
        repo_id = %repo_id,
        config_size_bytes = ci_config_content.len(),
        "ci-trigger: found .aspen/ci.ncl"
    );
    let config_str = String::from_utf8(ci_config_content)
        .map_err(|error| ci_trigger_error_response(format!("CI config is not valid UTF-8: {}", error)))?;

    info!(repo_id = %repo_id, "ci-trigger: parsing Nickel CI config");
    let pipeline_config = load_pipeline_config_str_async(config_str, ".aspen/ci.ncl".to_string())
        .await
        .map_err(|error| ci_trigger_error_response(format!("Failed to parse CI config: {}", error)))?;

    info!(
        repo_id = %repo_id,
        stages = pipeline_config.stages.len(),
        "ci-trigger: Nickel config parsed"
    );
    Ok(pipeline_config)
}

#[cfg(all(feature = "forge", feature = "blob"))]
async fn cleanup_trigger_checkout(checkout_dir: &std::path::Path) {
    use aspen_ci::checkout::cleanup_checkout;

    let _ = cleanup_checkout(checkout_dir).await;
}

#[cfg(all(feature = "forge", feature = "blob"))]
async fn checkout_trigger_repository(
    forge_node: &ForgeNodeRef,
    repo_id: &str,
    commit_hash: &[u8; 32],
    run_id: &str,
) -> Result<std::path::PathBuf, ClientRpcResponse> {
    use aspen_ci::checkout::checkout_dir_for_run;
    use aspen_ci::checkout::checkout_repository;
    use aspen_ci::checkout::prepare_for_ci_build;

    let checkout_dir = checkout_dir_for_run(run_id);
    debug_assert!(checkout_dir.to_string_lossy().ends_with(run_id));

    info!(
        repo_id = %repo_id,
        commit = %hex::encode(commit_hash),
        checkout_dir = %checkout_dir.display(),
        "ci-trigger: checking out repository"
    );
    if let Err(error) = checkout_repository(forge_node, commit_hash, &checkout_dir).await {
        warn!(repo_id = %repo_id, error = %error, "ci-trigger: checkout failed");
        cleanup_trigger_checkout(&checkout_dir).await;
        return Err(ci_trigger_error_response(format!("Failed to checkout repository: {}", error)));
    }

    if let Err(error) = prepare_for_ci_build(&checkout_dir, commit_hash).await {
        cleanup_trigger_checkout(&checkout_dir).await;
        return Err(ci_trigger_error_response(format!("Failed to prepare checkout for CI build: {}", error)));
    }

    Ok(checkout_dir)
}

#[cfg(all(feature = "forge", feature = "blob"))]
fn build_trigger_context(
    repo_id: aspen_forge::identity::RepoId,
    commit_hash: [u8; 32],
    ref_name: &str,
    checkout_dir: &std::path::Path,
) -> aspen_ci::orchestrator::PipelineContext {
    use aspen_ci::orchestrator::PipelineContext;

    let checkout_dir_string = checkout_dir.to_string_lossy().to_string();
    let mut env = HashMap::new();
    env.insert("CI_CHECKOUT_DIR".to_string(), checkout_dir_string.clone());
    debug_assert_eq!(env.get("CI_CHECKOUT_DIR").map(String::as_str), Some(checkout_dir_string.as_str()));

    let context = PipelineContext {
        repo_id,
        commit_hash,
        ref_name: ref_name.to_string(),
        triggered_by: "rpc".to_string(),
        run_id: String::new(),
        env,
        checkout_dir: Some(checkout_dir.to_path_buf()),
        source_hash: None,
    };
    debug_assert!(context.checkout_dir.is_some());
    context
}

/// Handle CiTriggerPipeline request.
///
/// Triggers a new pipeline run for the given repository and ref.
#[cfg(all(feature = "forge", feature = "blob"))]
pub async fn handle_trigger_pipeline(
    orchestrator: Option<&Arc<aspen_ci::PipelineOrchestrator<dyn aspen_core::KeyValueStore>>>,
    forge_node: Option<&ForgeNodeRef>,
    repo_id: String,
    ref_name: String,
    commit_hash_opt: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    info!(
        repo_id = %repo_id,
        ref_name = %ref_name,
        has_orchestrator = orchestrator.is_some(),
        has_forge = forge_node.is_some(),
        "ci-trigger: starting pipeline trigger"
    );

    let Some(orchestrator) = orchestrator else {
        return Ok(ci_trigger_error_response("CI orchestrator not available"));
    };
    let Some(forge_node) = forge_node else {
        return Ok(ci_trigger_error_response("Forge not available - required for CI config"));
    };

    let repo_id_parsed = match parse_trigger_repo_id(&repo_id) {
        Ok(parsed) => parsed,
        Err(response) => return Ok(response),
    };
    let requested_commit_hash = match parse_requested_commit_hash(commit_hash_opt.as_deref()) {
        Ok(hash) => hash,
        Err(response) => return Ok(response),
    };
    let commit_hash = match resolve_trigger_commit_hash(
        forge_node,
        &repo_id,
        &repo_id_parsed,
        &ref_name,
        requested_commit_hash,
    )
    .await
    {
        Ok(hash) => hash,
        Err(response) => return Ok(response),
    };
    let pipeline_config = match load_trigger_pipeline_config(forge_node, &repo_id, commit_hash).await {
        Ok(config) => config,
        Err(response) => return Ok(response),
    };

    let run_id = uuid::Uuid::new_v4().to_string();
    let checkout_dir = match checkout_trigger_repository(forge_node, &repo_id, &commit_hash, &run_id).await {
        Ok(path) => path,
        Err(response) => return Ok(response),
    };
    let context = build_trigger_context(repo_id_parsed, commit_hash, &ref_name, &checkout_dir);

    info!(repo_id = %repo_id, "ci-trigger: starting pipeline via orchestrator");
    let run = match orchestrator.execute(pipeline_config, context).await {
        Ok(run) => run,
        Err(error) => return Ok(ci_trigger_error_response(format!("Failed to start pipeline: {}", error))),
    };

    info!(run_id = %run.id, "CI pipeline started successfully");
    Ok(ci_trigger_success_response(run.id))
}

/// Handle CiGetStatus request.
///
/// Returns the current status of a pipeline run.
pub async fn handle_get_status(
    orchestrator: Option<&Arc<aspen_ci::PipelineOrchestrator<dyn aspen_core::KeyValueStore>>>,
    run_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(orchestrator) = orchestrator else {
        return Ok(ClientRpcResponse::CiGetStatusResult(CiGetStatusResponse {
            was_found: false,
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
                was_found: false,
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
    let status_str = run.status.as_str().to_string();

    let stages: Vec<CiStageInfo> = run
        .stages
        .iter()
        .map(|s| CiStageInfo {
            name: s.name.clone(),
            status: s.status.as_str().to_string(),
            jobs: s
                .jobs
                .iter()
                .map(|(name, job)| CiJobInfo {
                    id: job.job_id.as_ref().map(|id| id.to_string()).unwrap_or_default(),
                    name: name.clone(),
                    status: job.status.as_str().to_string(),
                    started_at_ms: job.started_at.map(|t| t.timestamp_millis() as u64),
                    ended_at_ms: job.completed_at.map(|t| t.timestamp_millis() as u64),
                    error: job.error.clone(),
                })
                .collect(),
        })
        .collect();

    Ok(ClientRpcResponse::CiGetStatusResult(CiGetStatusResponse {
        was_found: true,
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

/// Handle CiGetRefStatus request.
///
/// Returns the latest pipeline run for a repository ref.
#[cfg(feature = "forge")]
pub async fn handle_get_ref_status(
    orchestrator: Option<&Arc<aspen_ci::PipelineOrchestrator<dyn aspen_core::KeyValueStore>>>,
    repo_id: String,
    ref_name: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_forge::identity::RepoId;

    let Some(orchestrator) = orchestrator else {
        return Ok(ClientRpcResponse::CiGetRefStatusResult(CiGetStatusResponse {
            was_found: false,
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

    let repo_id_parsed = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::CiGetRefStatusResult(CiGetStatusResponse {
                was_found: false,
                run_id: None,
                repo_id: Some(repo_id),
                ref_name: Some(ref_name),
                commit_hash: None,
                status: None,
                stages: vec![],
                created_at_ms: None,
                completed_at_ms: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    debug!(repo_id = %repo_id, ref_name = %ref_name, "getting CI ref status");

    match orchestrator.get_latest_run_for_ref(&repo_id_parsed, &ref_name).await {
        Some(run) => {
            let stages: Vec<CiStageInfo> = run
                .stages
                .iter()
                .map(|s| CiStageInfo {
                    name: s.name.clone(),
                    status: s.status.as_str().to_string(),
                    jobs: s
                        .jobs
                        .iter()
                        .map(|(name, job)| CiJobInfo {
                            id: job.job_id.as_ref().map(|id| id.to_string()).unwrap_or_default(),
                            name: name.clone(),
                            status: job.status.as_str().to_string(),
                            started_at_ms: job.started_at.map(|t| t.timestamp_millis() as u64),
                            ended_at_ms: job.completed_at.map(|t| t.timestamp_millis() as u64),
                            error: job.error.clone(),
                        })
                        .collect(),
                })
                .collect();

            Ok(ClientRpcResponse::CiGetRefStatusResult(CiGetStatusResponse {
                was_found: true,
                run_id: Some(run.id.clone()),
                repo_id: Some(run.context.repo_id.to_hex()),
                ref_name: Some(run.context.ref_name.clone()),
                commit_hash: Some(hex::encode(run.context.commit_hash)),
                status: Some(run.status.as_str().to_string()),
                stages,
                created_at_ms: Some(run.created_at.timestamp_millis() as u64),
                completed_at_ms: run.completed_at.map(|t| t.timestamp_millis() as u64),
                error: run.error_message.clone(),
            }))
        }
        None => Ok(ClientRpcResponse::CiGetRefStatusResult(CiGetStatusResponse {
            was_found: false,
            run_id: None,
            repo_id: Some(repo_id),
            ref_name: Some(ref_name),
            commit_hash: None,
            status: None,
            stages: vec![],
            created_at_ms: None,
            completed_at_ms: None,
            error: None,
        })),
    }
}

/// Handle CiGetRefStatus when forge feature is not enabled.
#[cfg(not(feature = "forge"))]
pub async fn handle_get_ref_status(
    _orchestrator: Option<&Arc<aspen_ci::PipelineOrchestrator<dyn aspen_core::KeyValueStore>>>,
    _repo_id: String,
    _ref_name: String,
) -> anyhow::Result<ClientRpcResponse> {
    Ok(ClientRpcResponse::error("CI_FEATURE_UNAVAILABLE", "CI ref status requires forge feature"))
}

/// Handle CiListRuns request.
///
/// Lists pipeline runs with optional filtering.
#[cfg(feature = "forge")]
pub async fn handle_list_runs(
    orchestrator: Option<&Arc<aspen_ci::PipelineOrchestrator<dyn aspen_core::KeyValueStore>>>,
    repo_id: Option<String>,
    status: Option<String>,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::CiListRunsResponse;
    use aspen_client_api::CiRunInfo;
    use aspen_forge::identity::RepoId;
    use tracing::warn;

    let Some(orchestrator) = orchestrator else {
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
            status: run.status.as_str().to_string(),
            created_at_ms: run.created_at.timestamp_millis() as u64,
            completed_at_ms: run.completed_at.map(|t| t.timestamp_millis() as u64),
        })
        .collect();

    Ok(ClientRpcResponse::CiListRunsResult(CiListRunsResponse { runs: run_infos }))
}

/// Handle CiCancelRun request.
///
/// Cancels a running pipeline.
pub async fn handle_cancel_run(
    orchestrator: Option<&Arc<aspen_ci::PipelineOrchestrator<dyn aspen_core::KeyValueStore>>>,
    run_id: String,
    reason: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(orchestrator) = orchestrator else {
        return Ok(ClientRpcResponse::CiCancelRunResult(CiCancelRunResponse {
            is_success: false,
            error: Some("CI orchestrator not available".to_string()),
        }));
    };

    info!(%run_id, ?reason, "cancelling CI pipeline");

    match orchestrator.cancel(&run_id).await {
        Ok(()) => Ok(ClientRpcResponse::CiCancelRunResult(CiCancelRunResponse {
            is_success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::CiCancelRunResult(CiCancelRunResponse {
            is_success: false,
            error: Some(format!("Failed to cancel pipeline: {}", e)),
        })),
    }
}

#[cfg(all(test, feature = "forge", feature = "blob"))]
mod tests {
    use std::path::Path;

    use aspen_client_api::CiTriggerPipelineResponse;
    use aspen_forge::identity::RepoId;

    use super::*;

    fn trigger_response(response: ClientRpcResponse) -> CiTriggerPipelineResponse {
        match response {
            ClientRpcResponse::CiTriggerPipelineResult(result) => result,
            other => panic!("expected CiTriggerPipelineResult, got {other:?}"),
        }
    }

    #[test]
    fn normalize_trigger_ref_name_handles_short_and_full_refs() {
        assert_eq!(normalize_trigger_ref_name("main"), "heads/main");
        assert_eq!(normalize_trigger_ref_name("refs/heads/main"), "heads/main");
        assert_eq!(normalize_trigger_ref_name("refs/tags/v1"), "tags/v1");
        // Already-normalized refs should pass through unchanged
        assert_eq!(normalize_trigger_ref_name("heads/main"), "heads/main");
        assert_eq!(normalize_trigger_ref_name("tags/v1.0"), "tags/v1.0");
        assert_eq!(normalize_trigger_ref_name("heads/feature/foo"), "heads/feature/foo");
    }

    #[test]
    fn parse_trigger_repo_id_rejects_invalid_hex() {
        let response = parse_trigger_repo_id("not-hex").expect_err("invalid repo id should fail");
        let result = trigger_response(response);
        assert!(!result.is_success);
        assert!(result.error.unwrap_or_default().contains("Invalid repo_id"));
    }

    #[test]
    fn parse_requested_commit_hash_rejects_wrong_length() {
        let response = parse_requested_commit_hash(Some("abcd")).expect_err("short commit hash should fail");
        let result = trigger_response(response);
        assert!(!result.is_success);
        assert!(result.error.unwrap_or_default().contains("Invalid commit hash"));
    }

    #[test]
    fn build_trigger_context_sets_checkout_env_and_empty_run_id() {
        let repo_id = RepoId([7u8; 32]);
        let commit_hash = [9u8; 32];
        let checkout_dir = Path::new("/tmp/aspen-checkout/test-run");
        let context = build_trigger_context(repo_id, commit_hash, "main", checkout_dir);

        assert_eq!(context.repo_id, repo_id);
        assert_eq!(context.commit_hash, commit_hash);
        assert_eq!(context.ref_name, "main");
        assert_eq!(context.triggered_by, "rpc");
        assert!(context.run_id.is_empty());
        assert_eq!(context.env.get("CI_CHECKOUT_DIR").map(String::as_str), Some("/tmp/aspen-checkout/test-run"));
        assert_eq!(context.checkout_dir.as_deref(), Some(checkout_dir));
        assert!(context.source_hash.is_none());
    }
}
