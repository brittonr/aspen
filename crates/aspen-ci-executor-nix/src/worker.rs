//! Worker trait implementation for NixBuildWorker.

use aspen_ci_executor_shell::log_bridge;
use aspen_core::KeyValueStore;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use aspen_jobs::Job;
use aspen_jobs::JobOutput;
use aspen_jobs::JobResult;
use aspen_jobs::Worker;
use async_trait::async_trait;
use serde::Serialize;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::warn;

use crate::config::INLINE_LOG_THRESHOLD;
use crate::executor::NixBuildWorker;
use crate::payload::NixBuildPayload;

#[async_trait]
impl Worker for NixBuildWorker {
    fn job_types(&self) -> Vec<String> {
        vec!["ci_nix_build".into()]
    }

    async fn execute(&self, job: Job) -> JobResult {
        // Parse payload
        let payload: NixBuildPayload = match serde_json::from_value(job.spec.payload.clone()) {
            Ok(p) => p,
            Err(e) => {
                return JobResult::failure(format!("Invalid NixBuildPayload: {e}"));
            }
        };

        // Set up log streaming if KV store and run_id are available.
        // Spawn a bridge task that reads stderr lines from a channel and
        // writes them as log chunks to KV.
        let (log_sender, log_state) = match (&self.config.kv_store, &payload.run_id) {
            (Some(kv_store), Some(run_id)) => {
                let job_id = job.id.to_string();
                debug!(run_id = %run_id, job_id = %job_id, "Starting CI log streaming");

                let (tx, rx) = mpsc::channel::<String>(1000);
                let kv = kv_store.clone();
                let rid = run_id.clone();
                let jid = job_id.clone();
                let handle = tokio::spawn(log_bridge(rx, kv, rid, jid));

                (Some(tx), Some(handle))
            }
            _ => (None, None),
        };

        // Execute build with optional log streaming
        let build_output = match self.execute_build(&payload, log_sender.clone()).await {
            Ok(output) => output,
            Err(e) => {
                // Drop sender to signal bridge, then await completion
                drop(log_sender);
                if let Some(handle) = log_state {
                    let _ = handle.await;
                }

                // Record build failure in cache if KV store is available
                if let Some(kv_store) = &self.config.kv_store {
                    let flake_ref = payload.flake_ref();
                    if let Err(cache_err) = record_build_failure(kv_store.as_ref(), &flake_ref).await {
                        warn!(
                            flake_ref = %flake_ref,
                            error = %cache_err,
                            "Failed to record build failure in cache"
                        );
                    }
                }

                return JobResult::failure(format!("Nix build failed: {e}"));
            }
        };

        // Drop sender to signal bridge, then await completion
        drop(log_sender);
        if let Some(handle) = log_state {
            let _ = handle.await;
        }

        // Upload store paths to blob store if requested (legacy)
        let uploaded_store_paths = if payload.should_upload_result {
            self.upload_store_paths(&build_output.output_paths, payload.job_name.as_deref()).await
        } else {
            vec![]
        };

        // Upload store paths to SNIX distributed cache if requested
        #[cfg(feature = "snix")]
        let uploaded_store_paths_snix = if payload.publish_to_cache {
            let paths_to_publish = if payload.cache_outputs.is_empty() {
                build_output.output_paths.clone()
            } else {
                build_output
                    .output_paths
                    .iter()
                    .filter(|p| payload.cache_outputs.iter().any(|output_name| p.contains(output_name)))
                    .cloned()
                    .collect()
            };
            self.upload_store_paths_snix(&paths_to_publish).await
        } else {
            vec![]
        };
        #[cfg(not(feature = "snix"))]
        let uploaded_store_paths_snix: Vec<serde_json::Value> = vec![];

        // Collect artifacts
        let artifacts = match self.collect_artifacts(&build_output.output_paths, &payload.artifacts).await {
            Ok(a) => a,
            Err(e) => {
                warn!(error = %e, "Failed to collect artifacts");
                vec![]
            }
        };

        let artifact_info: Vec<serde_json::Value> = artifacts
            .iter()
            .map(|a| {
                serde_json::json!({
                    "path": a.relative_path.display().to_string(),
                    "blob_hash": a.blob_hash,
                })
            })
            .collect();

        let log = if build_output.log.len() > INLINE_LOG_THRESHOLD {
            format!(
                "{}...\n[Log truncated, {} bytes total]",
                &build_output.log[..INLINE_LOG_THRESHOLD],
                build_output.log.len()
            )
        } else {
            build_output.log
        };

        let mut metadata = [
            ("build_log".to_string(), log),
            ("node_id".to_string(), self.config.node_id.to_string()),
            ("cluster_id".to_string(), self.config.cluster_id.clone()),
        ]
        .into_iter()
        .collect::<std::collections::HashMap<String, String>>();

        // Merge timing metadata
        metadata.extend(build_output.timings.to_metadata());

        JobResult::Success(JobOutput {
            data: serde_json::json!({
                "output_paths": build_output.output_paths,
                "uploaded_store_paths": uploaded_store_paths,
                "uploaded_store_paths_snix": uploaded_store_paths_snix,
                "artifacts": artifact_info,
                "log_truncated": build_output.log_truncated,
                "built_by_node": self.config.node_id,
                "cluster_id": self.config.cluster_id,
            }),
            metadata,
        })
    }
}

// log_bridge and flush_chunk are now in aspen_ci_executor_shell::common
// imported via `use aspen_ci_executor_shell::log_bridge;` at the top.

// ============================================================================
// Build Failure Cache
// ============================================================================

/// KV prefix for cached build failure paths.
const KV_PREFIX_CI_FAILED_PATHS: &str = "_ci:failed-paths:";

/// Default TTL for failure cache entries: 24 hours.
const DEFAULT_FAILURE_CACHE_TTL_MS: u64 = 24 * 60 * 60 * 1000;

/// A cached failure entry stored in the KV store.
#[derive(Debug, Serialize)]
struct FailureCacheEntry {
    /// The flake reference that failed (e.g., ".#packages.x86_64-linux.default").
    flake_ref: String,
    /// When this entry was created (Unix ms).
    created_at_ms: u64,
    /// TTL in milliseconds.
    ttl_ms: u64,
}

/// Record a failed flake reference in the failure cache.
async fn record_build_failure<S: KeyValueStore + ?Sized>(
    store: &S,
    flake_ref: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let now_ms =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

    let key = generate_failure_cache_key(flake_ref);
    let entry = FailureCacheEntry {
        flake_ref: flake_ref.to_string(),
        created_at_ms: now_ms,
        ttl_ms: DEFAULT_FAILURE_CACHE_TTL_MS,
    };

    let value = serde_json::to_string(&entry)?;
    let request = WriteRequest::from_command(WriteCommand::Set { key, value });

    store.write(request).await?;
    debug!(flake_ref = %flake_ref, "Cached build failure");

    Ok(())
}

/// Generate cache key from a flake reference using BLAKE3 hash.
fn generate_failure_cache_key(flake_ref: &str) -> String {
    let hash = blake3::hash(flake_ref.as_bytes());
    format!("{}{}", KV_PREFIX_CI_FAILED_PATHS, hash.to_hex())
}
