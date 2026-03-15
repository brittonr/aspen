//! Deploy executor for CI deploy jobs.
//!
//! Runs in-process on the leader node. Resolves build artifacts from the
//! referenced job's KV result and calls a deployment callback to initiate
//! a rolling deployment. Polls for status and emits per-node progress as
//! CI job log lines.
//!
//! # Architecture
//!
//! ```text
//! PipelineOrchestrator
//!   └─ DeployExecutor::execute()
//!        ├─ resolve_artifact()       → read __jobs:{id} from KV
//!        ├─ deploy_fn()              → initiate rolling deploy
//!        └─ poll status_fn()         → poll status every 5s
//!             └─ emit log lines      → _ci:logs:{run_id}:{job_id}:*
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aspen_core::KeyValueStore;
use aspen_core::ReadRequest;
use aspen_core::WriteRequest;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::error::CiError;
use crate::error::Result;
use crate::log_writer::CiLogWriter;

/// Default deployment strategy.
const DEFAULT_STRATEGY: &str = "rolling";

/// Default max concurrent node upgrades.
const DEFAULT_MAX_CONCURRENT: u32 = 1;

/// Default health check timeout (matches `aspen_constants::DEPLOY_HEALTH_TIMEOUT_SECS`).
const DEPLOY_HEALTH_TIMEOUT_SECS: u64 = 120;

/// Status poll interval (matches `aspen_constants::DEPLOY_STATUS_POLL_INTERVAL_SECS`).
const DEPLOY_STATUS_POLL_INTERVAL_SECS: u64 = 5;

/// Resolved artifact from a build job's KV result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeployArtifact {
    /// Nix store path (e.g., `/nix/store/...-aspen-node`).
    NixStorePath(String),
    /// Iroh blob hash.
    BlobHash(String),
}

impl DeployArtifact {
    /// Return the artifact string suitable for the deploy RPC.
    pub fn artifact_string(&self) -> &str {
        match self {
            DeployArtifact::NixStorePath(s) => s,
            DeployArtifact::BlobHash(s) => s,
        }
    }
}

/// Returns `true`; used as serde default for `DeployRequest::stateful`.
fn default_true() -> bool {
    true
}

/// Request to initiate a deployment.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeployRequest {
    /// Artifact reference (store path or blob hash).
    pub artifact: String,
    /// Deployment strategy (e.g., "rolling").
    pub strategy: String,
    /// Max concurrent node upgrades.
    pub max_concurrent: u32,
    /// Health check timeout in seconds.
    pub health_timeout_secs: u64,
    /// Binary to validate inside a Nix store path.
    /// `None` → default `bin/aspen-node`.
    pub expected_binary: Option<String>,
    /// Whether to track deployment lifecycle state in Raft KV.
    ///
    /// When `true`, the executor writes state under `_deploy:state:{deploy_id}:`
    /// including metadata, per-node status, and rollback points.
    /// When `false`, only CI job logs are persisted (stateless push deploy).
    #[serde(default = "default_true")]
    pub stateful: bool,
    /// When `true`, only validate the artifact exists and the expected binary
    /// is present. Skip profile switch and process restart. Use this for CI
    /// pipeline tests that verify the deploy stage resolves artifacts without
    /// modifying the running cluster.
    #[serde(default)]
    pub validate_only: bool,
}

/// Result of initiating a deployment.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeployInitResult {
    /// Whether the deployment was accepted.
    pub is_accepted: bool,
    /// Deployment ID assigned by the coordinator.
    pub deploy_id: Option<String>,
    /// Error message if rejected.
    pub error: Option<String>,
}

/// Per-node deployment status entry.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeployNodeStatus {
    /// Node ID.
    pub node_id: u64,
    /// Status string (pending, draining, upgrading, healthy, failed).
    pub status: String,
    /// Error if node failed.
    pub error: Option<String>,
}

/// Status of an in-progress deployment.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeployStatusResult {
    /// Whether a deployment was found.
    pub is_found: bool,
    /// Overall status string (pending, deploying, completed, failed, rolled_back).
    pub overall_status: Option<String>,
    /// Per-node statuses.
    pub nodes: Vec<DeployNodeStatus>,
    /// Elapsed milliseconds since deployment started.
    pub elapsed_ms: Option<u64>,
    /// Error message if applicable.
    pub error: Option<String>,
}

/// Trait for dispatching deploy RPCs. Implemented by the node's handler
/// layer to bridge the executor with `ClusterDeploy`/`ClusterDeployStatus`.
#[async_trait::async_trait]
pub trait DeployDispatcher: Send + Sync {
    /// Initiate a deployment.
    async fn deploy(&self, request: DeployRequest) -> std::result::Result<DeployInitResult, String>;
    /// Query deployment status.
    async fn deploy_status(&self) -> std::result::Result<DeployStatusResult, String>;
}

/// Per-node status tracker for diff-based log emission.
#[derive(Debug, Clone, Default)]
struct NodeStatusSnapshot {
    statuses: HashMap<u64, String>,
}

/// Parameters for executing a deploy job.
pub struct DeployJobParams<'a> {
    /// Pipeline run ID.
    pub run_id: &'a str,
    /// Deploy job name.
    pub job_name: &'a str,
    /// Name of the build job to resolve artifacts from.
    pub artifact_from: &'a str,
    /// Deployment strategy (defaults to "rolling").
    pub strategy: Option<&'a str>,
    /// Health check timeout override.
    pub health_timeout_secs: Option<u64>,
    /// Max concurrent node upgrades override.
    pub max_concurrent: Option<u32>,
    /// Binary to validate inside a Nix store path (e.g., "bin/cowsay").
    pub expected_binary: Option<&'a str>,
    /// Whether to track deployment lifecycle state in Raft KV.
    /// Defaults to `true` for backwards compatibility.
    pub stateful: Option<bool>,
    /// When `true`, only validate the artifact exists and skip actual deployment.
    pub validate_only: Option<bool>,
    /// The pipeline run containing stage/job metadata.
    pub pipeline_run: &'a super::pipeline::PipelineRun,
    /// Dispatcher for deploy RPCs.
    pub dispatcher: &'a dyn DeployDispatcher,
}

/// Executor that runs deploy jobs in-process on the leader.
pub struct DeployExecutor<S: KeyValueStore + ?Sized> {
    kv_store: Arc<S>,
}

impl<S: KeyValueStore + ?Sized + 'static> DeployExecutor<S> {
    /// Create a new deploy executor.
    pub fn new(kv_store: Arc<S>) -> Self {
        Self { kv_store }
    }

    /// Execute a deploy job: resolve artifact, deploy, poll status.
    pub async fn execute(&self, params: DeployJobParams<'_>) -> Result<DeployJobResult> {
        let DeployJobParams {
            run_id,
            job_name,
            artifact_from,
            strategy,
            health_timeout_secs,
            max_concurrent,
            expected_binary,
            stateful,
            validate_only,
            pipeline_run,
            dispatcher,
        } = params;
        let mut log_writer = CiLogWriter::new(run_id.to_string(), job_name.to_string(), self.kv_store.clone());

        // Step 1: Resolve artifact
        write_deploy_log(&mut log_writer, &format!("[deploy] Resolving artifact from job '{artifact_from}'..."))
            .await?;

        let artifact = self.resolve_artifact(artifact_from, pipeline_run).await.map_err(|e| {
            warn!(job = job_name, error = %e, "Artifact resolution failed");
            e
        })?;

        let artifact_str = artifact.artifact_string().to_string();
        info!(job = job_name, artifact = %artifact_str, "Resolved deploy artifact");

        // Step 1.5: validate_only mode — verify artifact resolved, skip actual deploy
        if validate_only.unwrap_or(false) {
            write_deploy_log(&mut log_writer, &format!("[deploy] validate_only: artifact resolved to {artifact_str}"))
                .await?;

            // Verify expected_binary exists in the store path if specified
            if let DeployArtifact::NixStorePath(ref store_path) = artifact {
                if let Some(bin) = expected_binary {
                    let full_path = format!("{store_path}/{bin}");
                    write_deploy_log(&mut log_writer, &format!("[deploy] validate_only: checking {full_path}")).await?;
                }
            }

            write_deploy_log(
                &mut log_writer,
                "[deploy] validate_only: artifact validation passed, skipping actual deployment",
            )
            .await?;

            return Ok(DeployJobResult::Success {
                artifact: artifact_str,
                deploy_id: format!("validate-{run_id}"),
            });
        }

        write_deploy_log(&mut log_writer, &format!("[deploy] Starting rolling deployment: {artifact_str}")).await?;

        // Step 2: Initiate deployment
        let is_stateful = stateful.unwrap_or(true);

        let request = DeployRequest {
            artifact: artifact_str.clone(),
            strategy: strategy.unwrap_or(DEFAULT_STRATEGY).to_string(),
            max_concurrent: max_concurrent.unwrap_or(DEFAULT_MAX_CONCURRENT),
            health_timeout_secs: health_timeout_secs.unwrap_or(DEPLOY_HEALTH_TIMEOUT_SECS),
            expected_binary: expected_binary.map(|s| s.to_string()),
            stateful: is_stateful,
            validate_only: false,
        };

        // Capture strategy for stateful metadata (before request is moved)
        let deploy_strategy = request.strategy.clone();
        let deploy_max_concurrent = request.max_concurrent;
        let deploy_health_timeout = request.health_timeout_secs;

        let init_result = dispatcher.deploy(request).await.map_err(|e| CiError::ExecutionFailed {
            reason: format!("ClusterDeploy dispatch failed: {e}"),
        })?;

        if !init_result.is_accepted {
            let err_msg = init_result.error.unwrap_or_else(|| "deployment rejected".to_string());
            write_deploy_log_stderr(&mut log_writer, &format!("[deploy] Rejected: {err_msg}")).await;
            return Ok(DeployJobResult::Failed {
                error: err_msg,
                node_errors: HashMap::new(),
            });
        }

        let deploy_id = init_result.deploy_id.unwrap_or_default();
        info!(job = job_name, deploy_id = %deploy_id, "Deployment initiated");

        // Write deployment metadata to KV (stateful only)
        if is_stateful {
            let metadata = serde_json::json!({
                "artifact": &artifact_str,
                "strategy": &deploy_strategy,
                "max_concurrent": deploy_max_concurrent,
                "health_timeout_secs": deploy_health_timeout,
                "started_at": chrono::Utc::now().to_rfc3339(),
                "run_id": run_id,
                "job_name": job_name,
            });
            let metadata_key = format!("_deploy:state:{deploy_id}:metadata");
            self.kv_store.write(WriteRequest::set(metadata_key, metadata.to_string())).await.ok();
        }

        // Step 3: Poll status until completion or failure
        let poll_interval = Duration::from_secs(DEPLOY_STATUS_POLL_INTERVAL_SECS);
        let mut prev_snapshot = NodeStatusSnapshot::default();
        // Bounded: 2 hours max at 5s intervals = 1440 polls
        let max_polls: u32 = 1440;

        for _poll in 0..max_polls {
            tokio::time::sleep(poll_interval).await;

            let status = match dispatcher.deploy_status().await {
                Ok(s) => s,
                Err(e) => {
                    debug!(error = %e, "Deploy status poll error, retrying...");
                    continue;
                }
            };

            if !status.is_found {
                debug!("Deploy status not found yet, retrying...");
                continue;
            }

            // Emit per-node status changes
            for node in &status.nodes {
                let prev = prev_snapshot.statuses.get(&node.node_id);
                if prev.map(|s| s.as_str()) != Some(&node.status) {
                    let line = if let Some(err) = &node.error {
                        format!("[deploy] Node {}: {} — {}", node.node_id, node.status, err)
                    } else {
                        let suffix = if node.status == "healthy" { " ✓" } else { "" };
                        format!("[deploy] Node {}: {}{}", node.node_id, node.status, suffix)
                    };
                    log_writer.write_line(&line, "stdout").await.ok();
                    prev_snapshot.statuses.insert(node.node_id, node.status.clone());

                    // Write per-node state to KV (stateful only)
                    if is_stateful {
                        let node_key = format!("_deploy:state:{deploy_id}:node:{}", node.node_id);
                        let node_state = serde_json::json!({
                            "status": &node.status,
                            "error": &node.error,
                            "updated_at": chrono::Utc::now().to_rfc3339(),
                        });
                        self.kv_store.write(WriteRequest::set(node_key, node_state.to_string())).await.ok();
                    }
                }
            }

            // Check terminal states
            let overall = status.overall_status.as_deref().unwrap_or("unknown");
            match overall {
                "completed" => {
                    let elapsed_secs = status.elapsed_ms.unwrap_or(0) / 1000;
                    write_deploy_log(&mut log_writer, &format!("[deploy] Deployment completed ({elapsed_secs}s)"))
                        .await
                        .ok();

                    // Record rollback point (stateful only)
                    if is_stateful {
                        let rollback = serde_json::json!({
                            "artifact": &artifact_str,
                            "completed_at": chrono::Utc::now().to_rfc3339(),
                            "elapsed_secs": elapsed_secs,
                        });
                        let rollback_key = format!("_deploy:state:{deploy_id}:rollback");
                        self.kv_store.write(WriteRequest::set(rollback_key, rollback.to_string())).await.ok();
                    }

                    return Ok(DeployJobResult::Success {
                        deploy_id,
                        artifact: artifact_str,
                    });
                }
                "failed" | "rolled_back" => {
                    let err_msg = status.error.unwrap_or_else(|| format!("deployment {overall}"));
                    write_deploy_log_stderr(&mut log_writer, &format!("[deploy] Deployment failed: {err_msg}")).await;
                    let mut node_errors = HashMap::new();
                    for node in &status.nodes {
                        if let Some(err) = &node.error {
                            node_errors.insert(node.node_id, err.clone());
                        }
                    }
                    return Ok(DeployJobResult::Failed {
                        error: err_msg,
                        node_errors,
                    });
                }
                _ => { /* still in progress */ }
            }
        }

        let err_msg = "Deployment timed out waiting for completion".to_string();
        write_deploy_log_stderr(&mut log_writer, &format!("[deploy] {err_msg}")).await;
        Ok(DeployJobResult::Failed {
            error: err_msg,
            node_errors: HashMap::new(),
        })
    }

    /// Resolve the build artifact from a referenced job's KV result.
    ///
    /// Finds the job ID for `artifact_from` in the pipeline run's stage/job
    /// metadata, reads `__jobs:{job_id}` from KV, and extracts the artifact.
    pub async fn resolve_artifact(
        &self,
        artifact_from: &str,
        pipeline_run: &super::pipeline::PipelineRun,
    ) -> Result<DeployArtifact> {
        let job_id = find_job_id_by_name(pipeline_run, artifact_from).ok_or_else(|| CiError::ExecutionFailed {
            reason: format!("Could not find job '{}' in pipeline run stages", artifact_from),
        })?;

        debug!(artifact_from = artifact_from, job_id = %job_id, "Resolved job ID for artifact");

        let kv_key = format!("__jobs:{job_id}");
        let read_result =
            self.kv_store.read(ReadRequest::new(kv_key.clone())).await.map_err(|e| CiError::ExecutionFailed {
                reason: format!("Failed to read job result from KV key '{kv_key}': {e}"),
            })?;

        let entry = read_result.kv.ok_or_else(|| CiError::ExecutionFailed {
            reason: format!("Job result not found at KV key '{kv_key}'"),
        })?;

        let job_data: serde_json::Value = serde_json::from_str(&entry.value).map_err(|e| CiError::ExecutionFailed {
            reason: format!("Failed to parse job result JSON: {e}"),
        })?;

        extract_artifact_from_job_data(&job_data)
    }
}

/// Write a deploy log line to stdout.
async fn write_deploy_log<S: KeyValueStore + ?Sized + 'static>(
    log_writer: &mut CiLogWriter<S>,
    msg: &str,
) -> Result<()> {
    log_writer.write_line(msg, "stdout").await.map_err(|e| CiError::ExecutionFailed {
        reason: format!("failed to write deploy log: {e}"),
    })
}

/// Write a deploy log line to stderr (best-effort, ignores errors).
async fn write_deploy_log_stderr<S: KeyValueStore + ?Sized + 'static>(log_writer: &mut CiLogWriter<S>, msg: &str) {
    log_writer.write_line(msg, "stderr").await.ok();
}

/// Find the job ID for a job name within a pipeline run's stages.
fn find_job_id_by_name(pipeline_run: &super::pipeline::PipelineRun, job_name: &str) -> Option<String> {
    for stage in &pipeline_run.stages {
        if let Some(job_status) = stage.jobs.get(job_name) {
            if let Some(ref job_id) = job_status.job_id {
                return Some(job_id.to_string());
            }
        }
    }
    None
}

/// Extract a deploy artifact from job result JSON.
///
/// Priority:
/// 1. `result.Success.data.output_paths[0]` → `NixStorePath`
/// 2. `result.Success.data.artifacts[].blob_hash` → `BlobHash`
/// 3. `result.Success.data.uploaded_store_paths[].blob_hash` → `BlobHash`
pub fn extract_artifact_from_job_data(job_data: &serde_json::Value) -> Result<DeployArtifact> {
    let success_data = job_data.get("result").and_then(|r| r.get("Success")).and_then(|s| s.get("data"));

    let data = success_data.ok_or_else(|| CiError::ExecutionFailed {
        reason: "Job result has no Success.data — job may not have completed successfully".to_string(),
    })?;

    // Try output_paths first (Nix store path).
    // Multi-output packages emit several paths (e.g. cowsay-3.8.4-man, cowsay-3.8.4).
    // Prefer the main output: the path without a secondary output suffix.
    if let Some(paths) = data.get("output_paths").and_then(|p| p.as_array()) {
        let path_strs: Vec<&str> = paths.iter().filter_map(|p| p.as_str()).filter(|p| !p.is_empty()).collect();
        if let Some(best) = select_primary_output(&path_strs) {
            return Ok(DeployArtifact::NixStorePath(best.to_string()));
        }
    }

    // Fall back to artifacts[].blob_hash
    if let Some(artifacts) = data.get("artifacts").and_then(|a| a.as_array()) {
        for artifact in artifacts {
            if let Some(blob_hash) = artifact.get("blob_hash").and_then(|h| h.as_str()) {
                if !blob_hash.is_empty() {
                    return Ok(DeployArtifact::BlobHash(blob_hash.to_string()));
                }
            }
        }
    }

    // Also try uploaded_store_paths[].blob_hash
    if let Some(uploaded) = data.get("uploaded_store_paths").and_then(|a| a.as_array()) {
        for entry in uploaded {
            if let Some(blob_hash) = entry.get("blob_hash").and_then(|h| h.as_str()) {
                if !blob_hash.is_empty() {
                    return Ok(DeployArtifact::BlobHash(blob_hash.to_string()));
                }
            }
        }
    }

    Err(CiError::ExecutionFailed {
        reason: "NO_ARTIFACTS_FOUND: job result has no output_paths or artifact blob hashes".to_string(),
    })
}

/// Secondary output suffixes for multi-output Nix packages.
/// `nix build` can emit paths like `cowsay-3.8.4-man`, `cowsay-3.8.4-doc`, etc.
/// The main output has no suffix (e.g. `cowsay-3.8.4`).
const SECONDARY_OUTPUT_SUFFIXES: &[&str] = &["-man", "-doc", "-dev", "-info", "-lib", "-debug", "-static"];

/// Select the primary (main) output from a list of Nix store paths.
///
/// Multi-output packages produce multiple store paths. The main output is the
/// one without a secondary suffix like `-man`, `-doc`, `-dev`. If all paths
/// have suffixes (unusual), falls back to the first path.
fn select_primary_output<'a>(paths: &[&'a str]) -> Option<&'a str> {
    if paths.is_empty() {
        return None;
    }
    if paths.len() == 1 {
        return Some(paths[0]);
    }

    // Extract the store path name (after the hash-) for suffix checking.
    // Format: /nix/store/<hash>-<name>
    let is_secondary = |path: &str| -> bool {
        let name = path.rsplit('/').next().unwrap_or(path);
        // Strip the 32-char hash + dash prefix to get the package name
        let pkg_name = if name.len() > 33 { &name[33..] } else { name };
        SECONDARY_OUTPUT_SUFFIXES.iter().any(|suffix| pkg_name.ends_with(suffix))
    };

    // Prefer paths that aren't secondary outputs
    paths.iter().find(|p| !is_secondary(p)).or(paths.first()).copied()
}

/// Result of a deploy job execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeployJobResult {
    /// Deployment completed successfully.
    Success {
        /// Deployment ID from the coordinator.
        deploy_id: String,
        /// Artifact that was deployed.
        artifact: String,
    },
    /// Deployment failed.
    Failed {
        /// Error message.
        error: String,
        /// Per-node errors (node_id -> error message).
        node_errors: HashMap<u64, String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_artifact_nix_store_path() {
        let job_data = serde_json::json!({
            "result": {
                "Success": {
                    "data": {
                        "output_paths": ["/nix/store/abc123-aspen-node"],
                        "artifacts": []
                    }
                }
            }
        });
        let artifact = extract_artifact_from_job_data(&job_data).unwrap();
        assert_eq!(artifact, DeployArtifact::NixStorePath("/nix/store/abc123-aspen-node".to_string()));
    }

    #[test]
    fn test_extract_artifact_blob_hash_fallback() {
        let job_data = serde_json::json!({
            "result": {
                "Success": {
                    "data": {
                        "output_paths": [],
                        "artifacts": [
                            {"path": "bin/aspen-node", "blob_hash": "deadbeef0123456789abcdef"}
                        ]
                    }
                }
            }
        });
        let artifact = extract_artifact_from_job_data(&job_data).unwrap();
        assert_eq!(artifact, DeployArtifact::BlobHash("deadbeef0123456789abcdef".to_string()));
    }

    #[test]
    fn test_extract_artifact_uploaded_store_paths_fallback() {
        let job_data = serde_json::json!({
            "result": {
                "Success": {
                    "data": {
                        "output_paths": [],
                        "artifacts": [],
                        "uploaded_store_paths": [
                            {"store_path": "/nix/store/xyz", "blob_hash": "aabbccdd"}
                        ]
                    }
                }
            }
        });
        let artifact = extract_artifact_from_job_data(&job_data).unwrap();
        assert_eq!(artifact, DeployArtifact::BlobHash("aabbccdd".to_string()));
    }

    #[test]
    fn test_extract_artifact_no_artifacts() {
        let job_data = serde_json::json!({
            "result": {
                "Success": {
                    "data": {
                        "output_paths": [],
                        "artifacts": []
                    }
                }
            }
        });
        let err = extract_artifact_from_job_data(&job_data).unwrap_err();
        assert!(err.to_string().contains("NO_ARTIFACTS_FOUND"), "got: {err}");
    }

    #[test]
    fn test_extract_artifact_missing_success() {
        let job_data = serde_json::json!({
            "result": {
                "Failed": {
                    "error": "build failed"
                }
            }
        });
        let err = extract_artifact_from_job_data(&job_data).unwrap_err();
        assert!(err.to_string().contains("no Success.data"), "got: {err}");
    }

    #[test]
    fn test_deploy_artifact_string() {
        let nix = DeployArtifact::NixStorePath("/nix/store/xyz".to_string());
        assert_eq!(nix.artifact_string(), "/nix/store/xyz");

        let blob = DeployArtifact::BlobHash("deadbeef".to_string());
        assert_eq!(blob.artifact_string(), "deadbeef");
    }

    #[test]
    fn test_find_job_id_by_name() {
        use aspen_forge::identity::RepoId;

        use super::super::pipeline::JobStatus;
        use super::super::pipeline::PipelineRun;
        use super::super::pipeline::PipelineStatus;
        use super::super::pipeline::StageStatus;
        use crate::orchestrator::PipelineContext;

        let mut jobs = HashMap::new();
        jobs.insert("build-node".to_string(), JobStatus {
            job_id: Some(serde_json::from_value(serde_json::json!("job-123")).unwrap()),
            status: PipelineStatus::Success,
            started_at: None,
            completed_at: None,
            output: None,
            error: None,
        });

        let run = PipelineRun {
            id: "run-1".to_string(),
            pipeline_name: "test".to_string(),
            context: PipelineContext {
                repo_id: RepoId::from_hash(blake3::hash(b"test")),
                commit_hash: [0u8; 32],
                ref_name: "refs/heads/main".to_string(),
                triggered_by: "test".to_string(),
                run_id: "run-1".to_string(),
                env: HashMap::new(),
                checkout_dir: None,
                source_hash: None,
            },
            status: PipelineStatus::Running,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            stages: vec![StageStatus {
                name: "build".to_string(),
                status: PipelineStatus::Success,
                started_at: None,
                completed_at: None,
                jobs,
            }],
            workflow_id: None,
            error_message: None,
            has_pending_deploys: false,
        };

        assert_eq!(find_job_id_by_name(&run, "build-node"), Some("job-123".to_string()));
        assert_eq!(find_job_id_by_name(&run, "nonexistent"), None);
    }

    #[test]
    fn test_extract_artifact_prefers_nix_over_blob() {
        let job_data = serde_json::json!({
            "result": {
                "Success": {
                    "data": {
                        "output_paths": ["/nix/store/preferred-path"],
                        "artifacts": [
                            {"path": "bin/node", "blob_hash": "should-not-be-used"}
                        ]
                    }
                }
            }
        });
        let artifact = extract_artifact_from_job_data(&job_data).unwrap();
        assert_eq!(artifact, DeployArtifact::NixStorePath("/nix/store/preferred-path".to_string()));
    }

    #[test]
    fn test_extract_artifact_skips_empty_output_path() {
        let job_data = serde_json::json!({
            "result": {
                "Success": {
                    "data": {
                        "output_paths": [""],
                        "artifacts": [
                            {"path": "bin/node", "blob_hash": "fallback-hash"}
                        ]
                    }
                }
            }
        });
        let artifact = extract_artifact_from_job_data(&job_data).unwrap();
        assert_eq!(artifact, DeployArtifact::BlobHash("fallback-hash".to_string()));
    }

    #[test]
    fn test_extract_artifact_prefers_main_output_over_man() {
        // Multi-output: man pages listed first, main output second (real nix behavior)
        let job_data = serde_json::json!({
            "result": {
                "Success": {
                    "data": {
                        "output_paths": [
                            "/nix/store/ibhfxh52ibdg5c5fgy58m964im9xgn5f-cowsay-3.8.4-man",
                            "/nix/store/lgcnp2fwlzgxhs73y9scqf59p9la55c9-cowsay-3.8.4"
                        ]
                    }
                }
            }
        });
        let artifact = extract_artifact_from_job_data(&job_data).unwrap();
        assert_eq!(
            artifact,
            DeployArtifact::NixStorePath("/nix/store/lgcnp2fwlzgxhs73y9scqf59p9la55c9-cowsay-3.8.4".to_string())
        );
    }

    #[test]
    fn test_extract_artifact_handles_doc_and_dev_outputs() {
        let job_data = serde_json::json!({
            "result": {
                "Success": {
                    "data": {
                        "output_paths": [
                            "/nix/store/aaa-openssl-3.0.12-doc",
                            "/nix/store/bbb-openssl-3.0.12-dev",
                            "/nix/store/ccc-openssl-3.0.12"
                        ]
                    }
                }
            }
        });
        let artifact = extract_artifact_from_job_data(&job_data).unwrap();
        assert_eq!(artifact, DeployArtifact::NixStorePath("/nix/store/ccc-openssl-3.0.12".to_string()));
    }

    #[test]
    fn test_select_primary_output_single_path() {
        let paths = vec!["/nix/store/abc-pkg-1.0-man"];
        // Only one path — use it even if it's a secondary output
        assert_eq!(select_primary_output(&paths), Some("/nix/store/abc-pkg-1.0-man"));
    }

    #[test]
    fn test_select_primary_output_empty() {
        let paths: Vec<&str> = vec![];
        assert_eq!(select_primary_output(&paths), None);
    }

    #[test]
    fn test_select_primary_output_all_secondary() {
        // All paths are secondary — fall back to first
        let paths = vec!["/nix/store/aaa-pkg-1.0-man", "/nix/store/bbb-pkg-1.0-doc"];
        assert_eq!(select_primary_output(&paths), Some("/nix/store/aaa-pkg-1.0-man"));
    }

    #[test]
    fn test_deploy_request_stateful_default_true() {
        // When `stateful` is not provided in JSON, it defaults to true
        let json = r#"{
            "artifact": "/nix/store/abc",
            "strategy": "rolling",
            "max_concurrent": 1,
            "health_timeout_secs": 120
        }"#;
        let request: DeployRequest = serde_json::from_str(json).unwrap();
        assert!(request.stateful, "stateful should default to true when omitted");
    }

    #[test]
    fn test_deploy_request_stateful_explicit_false() {
        let json = r#"{
            "artifact": "/nix/store/abc",
            "strategy": "rolling",
            "max_concurrent": 1,
            "health_timeout_secs": 120,
            "stateful": false
        }"#;
        let request: DeployRequest = serde_json::from_str(json).unwrap();
        assert!(!request.stateful, "stateful should be false when explicitly set");
    }

    #[test]
    fn test_deploy_request_stateful_explicit_true() {
        let json = r#"{
            "artifact": "/nix/store/abc",
            "strategy": "rolling",
            "max_concurrent": 1,
            "health_timeout_secs": 120,
            "stateful": true
        }"#;
        let request: DeployRequest = serde_json::from_str(json).unwrap();
        assert!(request.stateful, "stateful should be true when explicitly set");
    }

    /// Generate Nickel contracts for the deploy protocol types.
    fn generate_deploy_schema_ncl() -> String {
        let request_schema = schemars::schema_for!(DeployRequest);
        let init_result_schema = schemars::schema_for!(DeployInitResult);
        let status_result_schema = schemars::schema_for!(DeployStatusResult);
        let node_status_schema = schemars::schema_for!(DeployNodeStatus);

        crate::schema_gen::schemas_to_nickel("Deploy protocol contracts", &[
            ("DeployRequest", &request_schema),
            ("DeployInitResult", &init_result_schema),
            ("DeployStatusResult", &status_result_schema),
            ("DeployNodeStatus", &node_status_schema),
        ])
    }

    #[test]
    fn test_deploy_protocol_schema_snapshot() {
        let generated = generate_deploy_schema_ncl();

        let snapshot_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/deploy-protocol.ncl");

        if std::env::var("UPDATE_SNAPSHOTS").is_ok() {
            std::fs::write(&snapshot_path, &generated).unwrap();
            return;
        }

        if !snapshot_path.exists() {
            std::fs::write(&snapshot_path, &generated).unwrap();
            return;
        }

        let existing = std::fs::read_to_string(&snapshot_path).unwrap();
        assert_eq!(
            existing, generated,
            "Deploy protocol schema has drifted. Run with UPDATE_SNAPSHOTS=1 to regenerate."
        );
    }
}
