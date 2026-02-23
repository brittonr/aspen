//! CI log operations: get_job_logs, subscribe_logs, get_job_output.

use std::sync::Arc;

use aspen_client_api::CiGetJobLogsResponse;
use aspen_client_api::CiGetJobOutputResponse;
use aspen_client_api::CiLogChunkInfo;
use aspen_client_api::CiSubscribeLogsResponse;
use aspen_client_api::ClientRpcResponse;
use aspen_core::CI_LOG_COMPLETE_MARKER;
use aspen_core::CI_LOG_KV_PREFIX;
use aspen_core::DEFAULT_CI_LOG_FETCH_CHUNKS;
use aspen_core::MAX_CI_LOG_FETCH_CHUNKS;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Handle CiGetJobLogs request.
///
/// Fetches historical log chunks for a CI job from the KV store.
/// Logs are stored with keys: `_ci:logs:{run_id}:{job_id}:{chunk_index:010}`
pub async fn handle_get_job_logs(
    kv_store: &dyn aspen_core::KeyValueStore,
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
    let scan_result = kv_store
        .scan(ScanRequest {
            prefix: start_key,
            limit_results: Some(limit),
            continuation_token: None,
        })
        .await;

    let entries = match scan_result {
        Ok(result) => result.entries,
        Err(e) => {
            warn!(run_id = %run_id, job_id = %job_id, error = %e, "failed to scan job logs");
            return Ok(ClientRpcResponse::CiGetJobLogsResult(CiGetJobLogsResponse {
                was_found: false,
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
        let check_result = kv_store
            .scan(ScanRequest {
                prefix: check_prefix,
                limit_results: Some(1),
                continuation_token: None,
            })
            .await;

        if let Ok(scan_response) = check_result
            && scan_response.entries.is_empty()
        {
            // No logs exist for this job
            return Ok(ClientRpcResponse::CiGetJobLogsResult(CiGetJobLogsResponse {
                was_found: false,
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
    let is_complete = match kv_store.read(aspen_core::ReadRequest::new(completion_key)).await {
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
        was_found: true,
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
pub async fn handle_subscribe_logs(
    kv_store: &dyn aspen_core::KeyValueStore,
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
        let scan_result = kv_store
            .scan(ScanRequest {
                prefix: watch_prefix.clone(),
                limit_results: Some(MAX_CI_LOG_FETCH_CHUNKS),
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
    let is_running = match kv_store.read(aspen_core::ReadRequest::new(completion_key)).await {
        Ok(result) => result.kv.is_none(),
        Err(_) => true, // Assume running if we can't check
    };

    // Check if the job exists at all
    let check_prefix = format!("{}{}:{}:", CI_LOG_KV_PREFIX, run_id, job_id);
    let was_found = match kv_store
        .scan(ScanRequest {
            prefix: check_prefix,
            limit_results: Some(1),
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
        was_found = was_found,
        "CI log subscription info prepared"
    );

    Ok(ClientRpcResponse::CiSubscribeLogsResult(CiSubscribeLogsResponse {
        was_found,
        watch_prefix,
        current_index,
        is_running,
        error: None,
    }))
}

/// Handle CiGetJobOutput request.
///
/// Returns full stdout/stderr for a completed job, resolving blob references if needed.
#[cfg(feature = "blob")]
pub async fn handle_get_job_output(
    kv_store: &dyn aspen_core::KeyValueStore,
    blob_store: Option<&Arc<aspen_blob::IrohBlobStore>>,
    _run_id: String,
    job_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    #[cfg(feature = "blob")]
    use aspen_blob::prelude::*;
    use aspen_jobs_worker_shell::OutputRef;

    // Job queue prefix (from aspen-jobs/src/manager.rs)
    const JOB_PREFIX: &str = "__jobs:";

    debug!(%job_id, "getting CI job output from job queue");

    // Look up the job directly from the job queue
    let job_key = format!("{}{}", JOB_PREFIX, job_id);
    let job_result = kv_store.read(aspen_core::ReadRequest::new(job_key)).await;

    let job_data_str = match job_result {
        Ok(result) => match result.kv {
            Some(kv) => kv.value,
            None => {
                return Ok(ClientRpcResponse::CiGetJobOutputResult(CiGetJobOutputResponse {
                    was_found: false,
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
                was_found: false,
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
                was_found: false,
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
                was_found: false,
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
    async fn resolve_output(
        blob_store: Option<&Arc<aspen_blob::IrohBlobStore>>,
        value: Option<&serde_json::Value>,
    ) -> (Option<String>, bool) {
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
                    if let Some(store) = blob_store {
                        let hash = match std::str::FromStr::from_str(&hash) {
                            Ok(h) => h,
                            Err(_) => return (Some(format!("[blob {} - invalid hash]", hash)), true),
                        };
                        match store.get_bytes(&hash).await {
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

    let (stdout, stdout_was_blob) = resolve_output(blob_store, stdout_value).await;
    let (stderr, stderr_was_blob) = resolve_output(blob_store, stderr_value).await;

    info!(
        %job_id,
        stdout_was_blob = stdout_was_blob,
        stderr_was_blob = stderr_was_blob,
        stdout_size = stdout_full_size,
        stderr_size = stderr_full_size,
        "retrieved CI job output"
    );

    Ok(ClientRpcResponse::CiGetJobOutputResult(CiGetJobOutputResponse {
        was_found: true,
        stdout,
        stderr,
        stdout_was_blob,
        stderr_was_blob,
        stdout_size: stdout_full_size,
        stderr_size: stderr_full_size,
        error: None,
    }))
}

/// Handle CiGetJobOutput request (no-blob variant).
///
/// Returns full stdout/stderr for a completed job, without blob resolution.
#[cfg(not(feature = "blob"))]
pub async fn handle_get_job_output(
    kv_store: &dyn aspen_core::KeyValueStore,
    _blob_store: Option<&Arc<()>>,
    _run_id: String,
    job_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_jobs_worker_shell::OutputRef;

    const JOB_PREFIX: &str = "__jobs:";

    debug!(%job_id, "getting CI job output from job queue (no blob)");

    let job_key = format!("{}{}", JOB_PREFIX, job_id);
    let job_result = kv_store.read(aspen_core::ReadRequest::new(job_key)).await;

    let job_data_str = match job_result {
        Ok(result) => match result.kv {
            Some(kv) => kv.value,
            None => {
                return Ok(ClientRpcResponse::CiGetJobOutputResult(CiGetJobOutputResponse {
                    was_found: false,
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
                was_found: false,
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

    let job_json: serde_json::Value = match serde_json::from_str(&job_data_str) {
        Ok(v) => v,
        Err(e) => {
            return Ok(ClientRpcResponse::CiGetJobOutputResult(CiGetJobOutputResponse {
                was_found: false,
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

    let job_data = job_json
        .get("result")
        .and_then(|result| result.get("Success"))
        .and_then(|success| success.get("data"));

    let job_data = match job_data {
        Some(data) => data,
        None => {
            let status = job_json.get("status").and_then(|s| s.as_str()).unwrap_or("unknown");
            let error_msg = if status == "processing" || status == "pending" {
                format!("Job {} is still running (status: {})", job_id, status)
            } else {
                format!("Job {} not completed successfully (status: {})", job_id, status)
            };
            return Ok(ClientRpcResponse::CiGetJobOutputResult(CiGetJobOutputResponse {
                was_found: false,
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

    let stdout_value = job_data.get("stdout");
    let stderr_value = job_data.get("stderr");
    let stdout_full_size = job_data.get("stdout_full_size").and_then(|v| v.as_u64()).unwrap_or(0);
    let stderr_full_size = job_data.get("stderr_full_size").and_then(|v| v.as_u64()).unwrap_or(0);

    // Extract inline output only (no blob resolution)
    fn extract_inline(value: Option<&serde_json::Value>) -> Option<String> {
        let value = value?;
        let output_ref: OutputRef = serde_json::from_value(value.clone()).ok()?;
        match output_ref {
            OutputRef::Inline(s) => Some(s),
            OutputRef::Blob { hash, .. } => Some(format!("[blob {} - blob feature not enabled]", hash)),
        }
    }

    let stdout = extract_inline(stdout_value);
    let stderr = extract_inline(stderr_value);

    info!(
        %job_id,
        stdout_size = stdout_full_size,
        stderr_size = stderr_full_size,
        "retrieved CI job output (no blob resolution)"
    );

    Ok(ClientRpcResponse::CiGetJobOutputResult(CiGetJobOutputResponse {
        was_found: true,
        stdout,
        stderr,
        stdout_was_blob: false,
        stderr_was_blob: false,
        stdout_size: stdout_full_size,
        stderr_size: stderr_full_size,
        error: None,
    }))
}
