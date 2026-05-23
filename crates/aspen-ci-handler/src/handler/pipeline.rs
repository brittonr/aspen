//! Pipeline operations: trigger, get_status, list_runs, cancel.

use std::collections::BTreeMap;
#[cfg(all(feature = "forge", feature = "blob"))]
use std::collections::HashMap;
use std::sync::Arc;

use aspen_client_api::CI_RUN_RECEIPT_SCHEMA;
use aspen_client_api::CiArtifactInfo;
use aspen_client_api::CiCancelRunResponse;
use aspen_client_api::CiGetRunReceiptResponse;
use aspen_client_api::CiGetStatusResponse;
use aspen_client_api::CiJobInfo;
use aspen_client_api::CiRunReceipt;
use aspen_client_api::CiRunReceiptJob;
use aspen_client_api::CiRunReceiptStage;
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
) -> Result<(std::path::PathBuf, BTreeMap<String, String>), ClientRpcResponse> {
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

    let flake_input_paths = prefetch_trigger_flake_inputs(&checkout_dir).await?;
    rewrite_trigger_checkout_flake_inputs(&checkout_dir, &flake_input_paths);

    Ok((checkout_dir, flake_input_paths))
}

#[cfg(all(feature = "forge", feature = "blob"))]
async fn prefetch_trigger_flake_inputs(
    checkout_dir: &std::path::Path,
) -> Result<BTreeMap<String, String>, ClientRpcResponse> {
    use serde_json::json;
    use tokio::process::Command;

    let output = Command::new("nix")
        .args(["flake", "archive", "--json"])
        .current_dir(checkout_dir)
        .output()
        .await
        .map_err(|error| ci_trigger_error_response(format!("Failed to prefetch flake inputs: {error}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ci_trigger_error_response(format!(
            "Failed to prefetch flake inputs: {}",
            stderr.chars().take(500).collect::<String>()
        )));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| ci_trigger_error_response(format!("Invalid flake archive UTF-8: {error}")))?;
    let archive_json: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|error| ci_trigger_error_response(format!("Failed to parse flake archive JSON: {error}")))?;

    let mut paths = BTreeMap::new();
    collect_trigger_flake_archive_paths(&archive_json, &mut paths);
    info!(checkout_dir = %checkout_dir.display(), input_count = paths.len(), "ci-trigger: prefetched flake inputs");
    Ok(paths
        .into_iter()
        .map(|(name, path)| {
            let nar_hash = std::process::Command::new("nix")
                .args(["hash", "path", "--type", "sha256", "--sri"])
                .arg(&path)
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .map(|stdout| stdout.trim().to_string())
                .filter(|hash| !hash.is_empty());
            let encoded = match nar_hash {
                Some(nar_hash) => json!({ "path": path, "narHash": nar_hash }).to_string(),
                None => path,
            };
            (name, encoded)
        })
        .collect())
}

#[cfg(all(feature = "forge", feature = "blob"))]
fn collect_trigger_flake_archive_paths(json: &serde_json::Value, paths: &mut BTreeMap<String, String>) {
    if let Some(inputs) = json.get("inputs").and_then(|v| v.as_object()) {
        for (name, value) in inputs {
            if let Some(path) = value.get("path").and_then(|v| v.as_str()) {
                paths.insert(name.clone(), path.to_string());
            }
            collect_trigger_flake_archive_paths(value, paths);
        }
    }
}

#[cfg(all(feature = "forge", feature = "blob"))]
fn rewrite_trigger_checkout_flake_inputs(checkout_dir: &std::path::Path, flake_input_paths: &BTreeMap<String, String>) {
    if flake_input_paths.is_empty() {
        debug!(checkout_dir = %checkout_dir.display(), "ci-trigger: no prefetched flake inputs available for checkout rewrite");
        return;
    }

    match aspen_ci_executor_shell::local_executor::rewrite_flake_lock_with_store_paths(checkout_dir, flake_input_paths)
    {
        Ok(()) => info!(
            checkout_dir = %checkout_dir.display(),
            inputs = flake_input_paths.len(),
            "ci-trigger: rewrote checkout flake inputs before VM source archive"
        ),
        Err(error) => warn!(
            checkout_dir = %checkout_dir.display(),
            inputs = flake_input_paths.len(),
            error = %error,
            "ci-trigger: failed to rewrite checkout flake inputs before VM source archive"
        ),
    }
}

#[cfg(all(feature = "forge", feature = "blob"))]
fn build_trigger_context(
    repo_id: aspen_forge::identity::RepoId,
    commit_hash: [u8; 32],
    ref_name: &str,
    checkout_dir: &std::path::Path,
    source_hash: Option<String>,
    flake_input_paths: BTreeMap<String, String>,
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
        source_hash,
        flake_input_paths,
    };
    debug_assert!(context.checkout_dir.is_some());
    context
}

#[cfg(all(feature = "forge", feature = "blob"))]
#[allow(clippy::result_large_err)]
async fn create_trigger_source_archive(
    orchestrator: &aspen_ci::PipelineOrchestrator<dyn aspen_core::KeyValueStore>,
    run_id: &str,
    checkout_dir: &std::path::Path,
) -> Result<Option<String>, ClientRpcResponse> {
    let Some(blob_store) = orchestrator.blob_store() else {
        warn!(
            run_id = %run_id,
            checkout_dir = %checkout_dir.display(),
            "ci-trigger: no blob store configured; RPC-triggered VM jobs will not have a source archive"
        );
        return Ok(None);
    };

    match aspen_ci_executor_shell::create_source_archive(checkout_dir, &blob_store).await {
        Ok(hash) => {
            info!(
                run_id = %run_id,
                source_hash = %hash,
                checkout_dir = %checkout_dir.display(),
                "ci-trigger: created source archive for VM jobs"
            );
            Ok(Some(hash))
        }
        Err(error) => {
            cleanup_trigger_checkout(checkout_dir).await;
            Err(ci_trigger_error_response(format!("Failed to create source archive for CI workspace: {}", error)))
        }
    }
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
    let (checkout_dir, flake_input_paths) =
        match checkout_trigger_repository(forge_node, &repo_id, &commit_hash, &run_id).await {
            Ok(result) => result,
            Err(response) => return Ok(response),
        };
    let source_hash = match create_trigger_source_archive(orchestrator.as_ref(), &run_id, &checkout_dir).await {
        Ok(hash) => hash,
        Err(response) => return Ok(response),
    };
    let context =
        build_trigger_context(repo_id_parsed, commit_hash, &ref_name, &checkout_dir, source_hash, flake_input_paths);

    info!(repo_id = %repo_id, "ci-trigger: starting pipeline via orchestrator");
    let run = match orchestrator.execute(pipeline_config, context).await {
        Ok(run) => run,
        Err(error) => return Ok(ci_trigger_error_response(format!("Failed to start pipeline: {}", error))),
    };

    info!(run_id = %run.id, "CI pipeline started successfully");
    Ok(ci_trigger_success_response(run.id))
}

fn timestamp_ms(time: chrono::DateTime<chrono::Utc>) -> u64 {
    time.timestamp_millis() as u64
}

fn optional_timestamp_ms(time: Option<chrono::DateTime<chrono::Utc>>) -> Option<u64> {
    time.map(timestamp_ms)
}

fn artifact_info_from_metadata(metadata: super::artifacts::ArtifactMetadata) -> CiArtifactInfo {
    CiArtifactInfo {
        blob_hash: metadata.blob_hash,
        name: metadata.name,
        size_bytes: metadata.size_bytes,
        content_type: metadata.content_type,
        created_at: metadata.created_at,
        metadata: metadata.extra.into_iter().collect(),
    }
}

async fn collect_receipt_artifacts_for_job(
    kv_store: &dyn aspen_core::KeyValueStore,
    job_id: &str,
) -> anyhow::Result<Vec<CiArtifactInfo>> {
    let entries = kv_store
        .scan(aspen_core::ScanRequest {
            prefix: format!("_ci:artifacts:{job_id}:"),
            limit_results: Some(100),
            continuation_token: None,
        })
        .await?
        .entries;

    let mut artifacts = Vec::with_capacity(entries.len());
    for entry in entries {
        if let Ok(metadata) = serde_json::from_str::<super::artifacts::ArtifactMetadata>(&entry.value) {
            artifacts.push(artifact_info_from_metadata(metadata));
        }
    }
    artifacts.sort_by(|left, right| left.name.cmp(&right.name).then_with(|| left.blob_hash.cmp(&right.blob_hash)));
    Ok(artifacts)
}

async fn collect_receipt_artifacts(
    kv_store: &dyn aspen_core::KeyValueStore,
    run: &aspen_ci::orchestrator::PipelineRun,
) -> anyhow::Result<BTreeMap<String, Vec<CiArtifactInfo>>> {
    let mut artifacts_by_job_id = BTreeMap::new();
    for stage in &run.stages {
        for job in stage.jobs.values() {
            if let Some(job_id) = &job.job_id {
                let job_id = job_id.to_string();
                let artifacts = collect_receipt_artifacts_for_job(kv_store, &job_id).await?;
                artifacts_by_job_id.insert(job_id, artifacts);
            }
        }
    }
    Ok(artifacts_by_job_id)
}

fn pipeline_run_to_receipt_with_artifacts(
    run: &aspen_ci::orchestrator::PipelineRun,
    artifacts_by_job_id: &BTreeMap<String, Vec<CiArtifactInfo>>,
) -> CiRunReceipt {
    let stages = run
        .stages
        .iter()
        .map(|stage| {
            let mut jobs: Vec<CiRunReceiptJob> = stage
                .jobs
                .iter()
                .map(|(name, job)| {
                    let job_id = job.job_id.as_ref().map(|id| id.to_string());
                    let mut artifacts =
                        job_id.as_ref().and_then(|id| artifacts_by_job_id.get(id)).cloned().unwrap_or_default();
                    artifacts.sort_by(|left, right| {
                        left.name.cmp(&right.name).then_with(|| left.blob_hash.cmp(&right.blob_hash))
                    });
                    CiRunReceiptJob {
                        name: name.clone(),
                        job_id,
                        status: job.status.as_str().to_string(),
                        started_at_ms: optional_timestamp_ms(job.started_at),
                        completed_at_ms: optional_timestamp_ms(job.completed_at),
                        error: job.error.clone(),
                        artifacts,
                    }
                })
                .collect();
            jobs.sort_by(|left, right| left.name.cmp(&right.name));
            CiRunReceiptStage {
                name: stage.name.clone(),
                status: stage.status.as_str().to_string(),
                started_at_ms: optional_timestamp_ms(stage.started_at),
                completed_at_ms: optional_timestamp_ms(stage.completed_at),
                jobs,
            }
        })
        .collect();

    CiRunReceipt {
        schema: CI_RUN_RECEIPT_SCHEMA.to_string(),
        run_id: run.id.clone(),
        pipeline_name: run.pipeline_name.clone(),
        repo_id: run.context.repo_id.to_hex(),
        ref_name: run.context.ref_name.clone(),
        commit_hash: hex::encode(run.context.commit_hash),
        status: run.status.as_str().to_string(),
        created_at_ms: timestamp_ms(run.created_at),
        started_at_ms: optional_timestamp_ms(run.started_at),
        completed_at_ms: optional_timestamp_ms(run.completed_at),
        error: run.error_message.clone(),
        stages,
    }
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

/// Handle CiGetRunReceipt request.
pub async fn handle_get_run_receipt(
    orchestrator: Option<&Arc<aspen_ci::PipelineOrchestrator<dyn aspen_core::KeyValueStore>>>,
    kv_store: &dyn aspen_core::KeyValueStore,
    run_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(orchestrator) = orchestrator else {
        return Ok(ClientRpcResponse::CiGetRunReceiptResult(CiGetRunReceiptResponse {
            was_found: false,
            receipt: None,
            error: Some("CI orchestrator not available".to_string()),
        }));
    };

    debug!(run_id = %run_id, "getting CI pipeline receipt");
    let Some(run) = orchestrator.get_run(&run_id).await else {
        return Ok(ClientRpcResponse::CiGetRunReceiptResult(CiGetRunReceiptResponse {
            was_found: false,
            receipt: None,
            error: Some("Pipeline run not found".to_string()),
        }));
    };

    let artifacts_by_job_id = match collect_receipt_artifacts(kv_store, &run).await {
        Ok(artifacts) => artifacts,
        Err(error) => {
            return Ok(ClientRpcResponse::CiGetRunReceiptResult(CiGetRunReceiptResponse {
                was_found: true,
                receipt: None,
                error: Some(format!("Failed to collect CI receipt artifacts: {error}")),
            }));
        }
    };

    Ok(ClientRpcResponse::CiGetRunReceiptResult(CiGetRunReceiptResponse {
        was_found: true,
        receipt: Some(pipeline_run_to_receipt_with_artifacts(&run, &artifacts_by_job_id)),
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
    use std::collections::HashMap;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::Arc;

    use aspen_blob::InMemoryBlobStore;
    use aspen_blob::prelude::BlobStore;
    use aspen_ci::orchestrator::JobStatus;
    use aspen_ci::orchestrator::PipelineContext;
    use aspen_ci::orchestrator::PipelineOrchestratorConfig;
    use aspen_ci::orchestrator::PipelineRun;
    use aspen_ci::orchestrator::PipelineStatus;
    use aspen_ci::orchestrator::StageStatus;
    use aspen_client_api::CI_STATUS_CANCELLED;
    use aspen_client_api::CI_STATUS_CHECKOUT_FAILED;
    use aspen_client_api::CI_STATUS_FAILED;
    use aspen_client_api::CI_STATUS_SUCCESS;
    use aspen_client_api::CI_TERMINAL_STATUS_LABELS;
    use aspen_client_api::CiTriggerPipelineResponse;
    use aspen_forge::identity::RepoId;
    use chrono::TimeZone;
    use chrono::Utc;

    use super::*;

    fn unique_temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("aspen-ci-handler-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).expect("temp dir is created");
        path
    }

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
    fn ci_terminal_status_contract_matches_pipeline_status() {
        let terminal_statuses = [
            PipelineStatus::CheckoutFailed,
            PipelineStatus::Success,
            PipelineStatus::Failed,
            PipelineStatus::Cancelled,
        ];
        let terminal_labels: Vec<&str> = terminal_statuses.iter().map(|status| status.as_str()).collect();

        assert_eq!(terminal_labels, CI_TERMINAL_STATUS_LABELS);
        assert_eq!(CI_TERMINAL_STATUS_LABELS, [
            CI_STATUS_CHECKOUT_FAILED,
            CI_STATUS_SUCCESS,
            CI_STATUS_FAILED,
            CI_STATUS_CANCELLED
        ]);

        for status in terminal_statuses {
            assert!(status.is_terminal(), "{} must be terminal", status.as_str());
        }
    }

    #[test]
    fn build_trigger_context_sets_checkout_env_and_empty_run_id() {
        let repo_id = RepoId([7u8; 32]);
        let commit_hash = [9u8; 32];
        let checkout_dir = Path::new("/tmp/aspen-checkout/test-run");
        let source_hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string();
        let mut flake_input_paths = BTreeMap::new();
        flake_input_paths.insert("ucan-src".to_string(), "/nix/store/ucan-src".to_string());
        let context = build_trigger_context(
            repo_id,
            commit_hash,
            "main",
            checkout_dir,
            Some(source_hash.clone()),
            flake_input_paths.clone(),
        );

        assert_eq!(context.repo_id, repo_id);
        assert_eq!(context.commit_hash, commit_hash);
        assert_eq!(context.ref_name, "main");
        assert_eq!(context.triggered_by, "rpc");
        assert!(context.run_id.is_empty());
        assert_eq!(context.env.get("CI_CHECKOUT_DIR").map(String::as_str), Some("/tmp/aspen-checkout/test-run"));
        assert_eq!(context.checkout_dir.as_deref(), Some(checkout_dir));
        assert_eq!(context.source_hash.as_deref(), Some(source_hash.as_str()));
        assert_eq!(context.flake_input_paths, flake_input_paths);
    }

    #[tokio::test]
    async fn rpc_trigger_source_archive_materializes_flake_root() {
        let kv_store: Arc<dyn aspen_core::KeyValueStore> = aspen_testing_core::DeterministicKeyValueStore::new();
        let job_manager = Arc::new(aspen_jobs::JobManager::new(kv_store.clone()));
        let workflow_manager = Arc::new(aspen_jobs::WorkflowManager::new(job_manager.clone(), kv_store.clone()));
        let blob_store: Arc<dyn BlobStore> = Arc::new(InMemoryBlobStore::new());
        let orchestrator = aspen_ci::PipelineOrchestrator::new(
            PipelineOrchestratorConfig::default(),
            workflow_manager,
            job_manager,
            Some(blob_store.clone()),
            kv_store,
        );

        let checkout_dir = unique_temp_dir("checkout");
        tokio::fs::write(checkout_dir.join("flake.nix"), b"{ outputs = { self }: {}; }\n")
            .await
            .expect("flake fixture is written");
        tokio::fs::create_dir_all(checkout_dir.join(".aspen")).await.expect("ci config dir is created");
        tokio::fs::write(checkout_dir.join(".aspen/ci.ncl"), b"{ stages = [] }\n")
            .await
            .expect("ci config fixture is written");

        let source_hash = create_trigger_source_archive(&orchestrator, "run-rpc-source", &checkout_dir)
            .await
            .expect("source archive creation succeeds")
            .expect("blob-backed orchestrator returns a source hash");
        let workspace_dir = unique_temp_dir("workspace");
        aspen_ci_executor_shell::seed_workspace_from_blob(&blob_store, &source_hash, &workspace_dir)
            .await
            .expect("source archive seeds workspace");

        assert!(workspace_dir.join("flake.nix").is_file(), "RPC-triggered VM workspace must contain flake.nix");

        let _ = tokio::fs::remove_dir_all(checkout_dir).await;
        let _ = tokio::fs::remove_dir_all(workspace_dir).await;
    }

    fn receipt_test_run(run_id: &str, job_id: &str) -> PipelineRun {
        let repo_id = RepoId([7u8; 32]);
        let created_at = Utc.with_ymd_and_hms(2026, 5, 3, 20, 0, 0).single().expect("valid time");
        let started_at = Utc.with_ymd_and_hms(2026, 5, 3, 20, 1, 0).single().expect("valid time");
        let completed_at = Utc.with_ymd_and_hms(2026, 5, 3, 20, 2, 0).single().expect("valid time");
        let mut jobs = HashMap::new();
        jobs.insert("zeta".to_string(), JobStatus {
            job_id: None,
            status: PipelineStatus::Success,
            started_at: Some(started_at),
            completed_at: Some(completed_at),
            output: None,
            error: None,
        });
        jobs.insert("alpha".to_string(), JobStatus {
            job_id: Some(serde_json::from_value(serde_json::json!(job_id)).unwrap()),
            status: PipelineStatus::Failed,
            started_at: Some(started_at),
            completed_at: Some(completed_at),
            output: None,
            error: Some("boom".to_string()),
        });
        PipelineRun {
            id: run_id.to_string(),
            pipeline_name: "dogfood".to_string(),
            context: PipelineContext {
                repo_id,
                commit_hash: [9u8; 32],
                ref_name: "refs/heads/main".to_string(),
                triggered_by: "test".to_string(),
                run_id: run_id.to_string(),
                env: HashMap::new(),
                checkout_dir: None,
                source_hash: None,
                flake_input_paths: std::collections::BTreeMap::new(),
            },
            status: PipelineStatus::Failed,
            created_at,
            started_at: Some(started_at),
            completed_at: Some(completed_at),
            stages: vec![StageStatus {
                name: "build".to_string(),
                status: PipelineStatus::Failed,
                started_at: Some(started_at),
                completed_at: Some(completed_at),
                jobs,
            }],
            workflow_id: None,
            error_message: Some("pipeline failed".to_string()),
            has_pending_deploys: false,
        }
    }

    #[test]
    fn pipeline_run_to_receipt_is_schema_versioned_and_sorts_jobs() {
        let repo_id = RepoId([7u8; 32]);
        let created_at = Utc.with_ymd_and_hms(2026, 5, 3, 20, 0, 0).single().expect("valid time");
        let started_at = Utc.with_ymd_and_hms(2026, 5, 3, 20, 1, 0).single().expect("valid time");
        let completed_at = Utc.with_ymd_and_hms(2026, 5, 3, 20, 2, 0).single().expect("valid time");
        let mut jobs = HashMap::new();
        jobs.insert("zeta".to_string(), JobStatus {
            job_id: None,
            status: PipelineStatus::Success,
            started_at: Some(started_at),
            completed_at: Some(completed_at),
            output: None,
            error: None,
        });
        jobs.insert("alpha".to_string(), JobStatus {
            job_id: Some(serde_json::from_value(serde_json::json!("job-alpha")).unwrap()),
            status: PipelineStatus::Failed,
            started_at: Some(started_at),
            completed_at: Some(completed_at),
            output: None,
            error: Some("boom".to_string()),
        });
        let run = PipelineRun {
            id: "run-1".to_string(),
            pipeline_name: "dogfood".to_string(),
            context: PipelineContext {
                repo_id,
                commit_hash: [9u8; 32],
                ref_name: "refs/heads/main".to_string(),
                triggered_by: "test".to_string(),
                run_id: "run-1".to_string(),
                env: HashMap::new(),
                checkout_dir: None,
                source_hash: None,
                flake_input_paths: std::collections::BTreeMap::new(),
            },
            status: PipelineStatus::Failed,
            created_at,
            started_at: Some(started_at),
            completed_at: Some(completed_at),
            stages: vec![StageStatus {
                name: "build".to_string(),
                status: PipelineStatus::Failed,
                started_at: Some(started_at),
                completed_at: Some(completed_at),
                jobs,
            }],
            workflow_id: None,
            error_message: Some("pipeline failed".to_string()),
            has_pending_deploys: false,
        };

        let mut artifacts_by_job_id = BTreeMap::new();
        artifacts_by_job_id.insert("job-alpha".to_string(), vec![
            CiArtifactInfo {
                blob_hash: "bbb".to_string(),
                name: "z-result".to_string(),
                size_bytes: 20,
                content_type: "application/octet-stream".to_string(),
                created_at: "2026-05-03T20:02:00Z".to_string(),
                metadata: BTreeMap::new(),
            },
            CiArtifactInfo {
                blob_hash: "aaa".to_string(),
                name: "a-result".to_string(),
                size_bytes: 10,
                content_type: "text/plain".to_string(),
                created_at: "2026-05-03T20:02:00Z".to_string(),
                metadata: BTreeMap::new(),
            },
        ]);

        let receipt = pipeline_run_to_receipt_with_artifacts(&run, &artifacts_by_job_id);

        assert_eq!(receipt.schema, CI_RUN_RECEIPT_SCHEMA);
        assert_eq!(receipt.run_id, "run-1");
        assert_eq!(receipt.repo_id, repo_id.to_hex());
        assert_eq!(receipt.commit_hash, hex::encode([9u8; 32]));
        assert_eq!(receipt.created_at_ms, created_at.timestamp_millis() as u64);
        assert_eq!(receipt.stages[0].jobs[0].name, "alpha");
        assert_eq!(receipt.stages[0].jobs[1].name, "zeta");
        assert_eq!(receipt.stages[0].jobs[0].error.as_deref(), Some("boom"));
        assert_eq!(receipt.stages[0].jobs[0].artifacts.len(), 2);
        assert_eq!(receipt.stages[0].jobs[0].artifacts[0].name, "a-result");
        assert_eq!(receipt.stages[0].jobs[0].artifacts[0].blob_hash, "aaa");
        assert!(receipt.stages[0].jobs[1].artifacts.is_empty());
    }

    #[tokio::test]
    async fn get_run_receipt_reads_artifact_metadata_from_kv() {
        let kv_store: Arc<dyn aspen_core::KeyValueStore> = aspen_testing_core::DeterministicKeyValueStore::new();
        let job_manager = Arc::new(aspen_jobs::JobManager::new(kv_store.clone()));
        let workflow_manager = Arc::new(aspen_jobs::WorkflowManager::new(job_manager.clone(), kv_store.clone()));
        let orchestrator = Arc::new(aspen_ci::PipelineOrchestrator::new(
            aspen_ci::PipelineOrchestratorConfig::default(),
            workflow_manager,
            job_manager,
            None,
            kv_store.clone(),
        ));

        let run_id = "run-native-artifact-receipt";
        let job_id = "job-native-artifacts";
        let run = receipt_test_run(run_id, job_id);
        kv_store
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::Set {
                    key: format!("_ci:runs:{run_id}"),
                    value: serde_json::to_string(&run).expect("run serializes"),
                },
            })
            .await
            .expect("run is written to KV");

        for (key_suffix, name, blob_hash, size_bytes) in
            [("z-result", "z-result", "bbb", 20), ("a-result", "a-result", "aaa", 10)]
        {
            let mut extra = HashMap::new();
            extra.insert("producer".to_string(), "native-ci".to_string());
            let metadata = super::super::artifacts::ArtifactMetadata {
                blob_hash: blob_hash.to_string(),
                name: name.to_string(),
                size_bytes,
                content_type: "text/plain".to_string(),
                created_at: "2026-05-03T20:02:00Z".to_string(),
                run_id: Some(run_id.to_string()),
                extra,
            };
            kv_store
                .write(aspen_core::WriteRequest {
                    command: aspen_core::WriteCommand::Set {
                        key: format!("_ci:artifacts:{job_id}:{key_suffix}"),
                        value: serde_json::to_string(&metadata).expect("artifact metadata serializes"),
                    },
                })
                .await
                .expect("artifact metadata is written to KV");
        }

        let response = handle_get_run_receipt(Some(&orchestrator), kv_store.as_ref(), run_id.to_string())
            .await
            .expect("receipt handler succeeds");
        let result = match response {
            ClientRpcResponse::CiGetRunReceiptResult(result) => result,
            other => panic!("expected CiGetRunReceiptResult, got {other:?}"),
        };

        assert!(result.was_found);
        assert!(result.error.is_none());
        let receipt = result.receipt.expect("receipt is returned");
        let alpha = receipt.stages[0].jobs.iter().find(|job| job.name == "alpha").expect("alpha job is present");
        assert_eq!(alpha.job_id.as_deref(), Some(job_id));
        assert_eq!(alpha.artifacts.len(), 2);
        assert_eq!(alpha.artifacts[0].name, "a-result");
        assert_eq!(alpha.artifacts[0].blob_hash, "aaa");
        assert_eq!(alpha.artifacts[0].metadata.get("producer").map(String::as_str), Some("native-ci"));
        assert_eq!(alpha.artifacts[1].name, "z-result");
    }
}
