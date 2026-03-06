//! Worker trait implementation for NixBuildWorker.

use std::sync::Arc;

use aspen_ci_core::log_writer::CiLogChunk;
use aspen_ci_core::log_writer::CiLogCompleteMarker;
use aspen_core::KeyValueStore;
use aspen_core::WriteRequest;
use aspen_jobs::Job;
use aspen_jobs::JobOutput;
use aspen_jobs::JobResult;
use aspen_jobs::Worker;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::warn;

use crate::config::INLINE_LOG_THRESHOLD;
use crate::executor::NixBuildWorker;
use crate::payload::NixBuildPayload;

/// KV key prefix for CI log chunks (matches aspen-constants).
const CI_LOG_KV_PREFIX: &str = "_ci:logs:";
/// Completion marker suffix (matches aspen-constants).
const CI_LOG_COMPLETE_MARKER: &str = "__complete__";
/// Maximum bytes buffered before flushing a chunk.
const FLUSH_THRESHOLD: usize = 8 * 1024;
/// Periodic flush interval in milliseconds.
/// Ensures partial buffers are written to KV for real-time streaming,
/// even when output is sparse and doesn't fill a full chunk.
const FLUSH_INTERVAL_MS: u64 = 500;

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
            metadata: [
                ("build_log".to_string(), log),
                ("node_id".to_string(), self.config.node_id.to_string()),
                ("cluster_id".to_string(), self.config.cluster_id.clone()),
            ]
            .into_iter()
            .collect(),
        })
    }
}

/// Bridge task: reads stderr lines from a channel, buffers them, and writes
/// log chunks to KV. Flushes on size threshold OR periodic timer to ensure
/// real-time streaming even with sparse output.
async fn log_bridge(mut rx: mpsc::Receiver<String>, kv_store: Arc<dyn KeyValueStore>, run_id: String, job_id: String) {
    use std::time::Duration;

    use tokio::time::interval;

    let mut chunk_index: u32 = 0;
    let mut buffer = String::new();
    let mut flush_interval = interval(Duration::from_millis(FLUSH_INTERVAL_MS));

    // Skip the immediate first tick
    flush_interval.tick().await;

    loop {
        tokio::select! {
            biased;

            msg = rx.recv() => {
                match msg {
                    Some(line) => {
                        buffer.push_str(&line);

                        // Flush when buffer exceeds threshold
                        if buffer.len() >= FLUSH_THRESHOLD {
                            flush_chunk(&kv_store, &run_id, &job_id, &mut chunk_index, &mut buffer).await;
                        }
                    }
                    None => {
                        // Channel closed — build finished
                        break;
                    }
                }
            }
            _ = flush_interval.tick() => {
                // Periodic flush: write partial buffer to KV so streaming
                // clients see output in real-time, not just at 8KB boundaries.
                if !buffer.is_empty() {
                    flush_chunk(&kv_store, &run_id, &job_id, &mut chunk_index, &mut buffer).await;
                }
            }
        }
    }

    // Flush remaining buffer
    if !buffer.is_empty() {
        flush_chunk(&kv_store, &run_id, &job_id, &mut chunk_index, &mut buffer).await;
    }

    // Write completion marker
    let marker_key = format!("{CI_LOG_KV_PREFIX}{run_id}:{job_id}:{CI_LOG_COMPLETE_MARKER}");
    let now_ms =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
    let marker = CiLogCompleteMarker {
        total_chunks: chunk_index,
        timestamp_ms: now_ms,
        status: "done".to_string(),
    };
    if let Ok(json) = serde_json::to_string(&marker) {
        let _ = kv_store.write(WriteRequest::set(marker_key, json)).await;
    }
}

async fn flush_chunk(
    kv_store: &Arc<dyn KeyValueStore>,
    run_id: &str,
    job_id: &str,
    chunk_index: &mut u32,
    buffer: &mut String,
) {
    let now_ms =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

    let chunk = CiLogChunk {
        index: *chunk_index,
        content: buffer.clone(),
        timestamp_ms: now_ms,
    };

    if let Ok(json) = serde_json::to_string(&chunk) {
        let key = format!("{CI_LOG_KV_PREFIX}{run_id}:{job_id}:{:010}", *chunk_index);
        if let Err(e) = kv_store.write(WriteRequest::set(key, json)).await {
            warn!(run_id = %run_id, job_id = %job_id, chunk = *chunk_index, error = %e, "Failed to write log chunk");
        }
    }

    *chunk_index += 1;
    buffer.clear();
}
