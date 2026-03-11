//! Core job execution logic: orchestration, request building, and streaming.

use std::path::PathBuf;
#[cfg(feature = "nix-cache-proxy")]
use std::sync::Arc;

use aspen_jobs::Job;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::LocalExecutorPayload;
use super::LocalExecutorWorker;
use super::nix::inject_nix_flags_with_flake_rewrite;
use super::output::OutputRef;
use super::output::spawn_log_consumer;
use crate::agent::protocol::ExecutionRequest;
use crate::agent::protocol::ExecutionResult;
use crate::agent::protocol::LogMessage;
#[cfg(feature = "nix-cache-proxy")]
use crate::cache_proxy::CacheProxy;
use crate::common::ArtifactCollectionResult;
use crate::common::ArtifactUploadResult;

/// Information about nix build outputs uploaded to the blob store.
#[derive(Debug, Default)]
pub(crate) struct NixOutputInfo {
    /// Nix store paths parsed from stdout (requires --print-out-paths).
    pub output_paths: Vec<String>,
    /// Blob hash of the uploaded binary (if found and uploaded).
    pub binary_blob_hash: Option<String>,
    /// Size of the uploaded binary in bytes.
    pub binary_size: Option<u64>,
    /// Path within the nix output where the binary was found.
    pub binary_path: Option<String>,
}

/// Parse nix store paths from stdout.
///
/// `nix build --print-out-paths` prints one store path per line to stdout.
fn parse_nix_output_paths(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter(|line| line.starts_with("/nix/store/"))
        .map(|line| line.trim().to_string())
        .collect()
}

impl LocalExecutorWorker {
    /// Store output in blob store if large, inline if small.
    ///
    /// Outputs <= 64KB are stored inline. Larger outputs are stored in the blob
    /// store (if available) with only a hash reference kept in the job record.
    /// Falls back to truncation if blob storage fails.
    pub(super) async fn store_output(&self, data: &str, job_id: &str, stream: &str) -> OutputRef {
        /// Inline output threshold (64 KB).
        const INLINE_OUTPUT_THRESHOLD: u64 = 64 * 1024;

        let bytes = data.as_bytes();

        if bytes.len() as u64 <= INLINE_OUTPUT_THRESHOLD {
            return OutputRef::Inline {
                content: data.to_string(),
            };
        }

        if let Some(ref blob_store) = self.blob_store {
            match blob_store.add_bytes(bytes).await {
                Ok(result) => {
                    info!(
                        job_id,
                        stream,
                        hash = %result.blob_ref.hash.to_hex(),
                        size = bytes.len(),
                        "Stored output in blob store"
                    );
                    return OutputRef::Blob {
                        hash: result.blob_ref.hash.to_hex().to_string(),
                        size: bytes.len() as u64,
                    };
                }
                Err(e) => {
                    warn!(job_id, stream, error = ?e, "Failed to store output in blob store");
                }
            }
        }

        // Fallback: truncate keeping tail (where errors typically appear)
        let max = INLINE_OUTPUT_THRESHOLD as usize;
        let skip = bytes.len().saturating_sub(max);
        let truncated = format!("...[{} bytes truncated]...\n{}", skip, &data[skip..]);
        OutputRef::Inline { content: truncated }
    }

    /// Execute a job and collect artifacts.
    ///
    /// This is the main orchestration function that coordinates:
    /// 1. Workspace setup (directory creation, checkout copying, blob seeding)
    /// 2. Cache proxy startup (for nix commands with cluster cache)
    /// 3. Command execution with log streaming
    /// 4. SNIX store path upload (for successful nix builds)
    /// 5. Artifact collection and upload
    /// 6. Workspace and proxy cleanup
    pub(super) async fn execute_job(
        &self,
        job: &Job,
        payload: &LocalExecutorPayload,
    ) -> Result<(ExecutionResult, ArtifactCollectionResult, Option<ArtifactUploadResult>, NixOutputInfo), String> {
        let job_id = job.id.to_string();

        // Phase 1: Set up workspace (returns workspace path and optional flake store path)
        let (job_workspace, flake_store_path) = self
            .setup_job_workspace(&job_id, payload)
            .await
            .map_err(|e| format!("workspace setup failed: {}", e))?;

        // Phase 2: Start cache proxy for nix commands if configured
        #[cfg(feature = "nix-cache-proxy")]
        let cache_proxy = if payload.command == "nix" && self.config.can_use_cache_proxy() {
            let endpoint =
                self.config.iroh_endpoint.as_ref().ok_or_else(|| "iroh endpoint not configured".to_string())?;
            let gateway = self.config.gateway_node.ok_or_else(|| "gateway node not configured".to_string())?;

            match CacheProxy::start(Arc::clone(endpoint), gateway).await {
                Ok(proxy) => {
                    info!(
                        job_id = %job_id,
                        substituter_url = %proxy.substituter_url(),
                        "Started cache proxy for nix command"
                    );
                    Some(proxy)
                }
                Err(e) => {
                    warn!(job_id = %job_id, error = ?e, "Failed to start cache proxy, proceeding without substituter");
                    None
                }
            }
        } else {
            None
        };

        // Phase 3: Build and execute request
        #[cfg(feature = "nix-cache-proxy")]
        let request = self.build_execution_request(
            &job_id,
            payload,
            &job_workspace,
            flake_store_path.as_ref(),
            cache_proxy.as_ref(),
        );
        #[cfg(not(feature = "nix-cache-proxy"))]
        let request = self.build_execution_request(&job_id, payload, &job_workspace, flake_store_path.as_ref());
        let result = self.execute_with_streaming(&job_id, request, payload).await;

        // Phase 4: Shut down cache proxy
        #[cfg(feature = "nix-cache-proxy")]
        if let Some(proxy) = cache_proxy {
            proxy.shutdown().await;
        }

        let result = result?;

        info!(
            job_id = %job_id,
            exit_code = result.exit_code,
            duration_ms = result.duration_ms,
            "job completed"
        );

        // Phase 3: Upload nix store paths to SNIX (on successful nix builds)
        #[cfg(feature = "snix")]
        {
            let is_nix_command = payload.command == "nix";
            let is_successful = result.exit_code == 0 && result.error.is_none();

            if is_nix_command && is_successful {
                info!(
                    job_id = %job_id,
                    command = %payload.command,
                    "checking for SNIX upload - nix build succeeded"
                );

                // Parse output paths from nix build output
                let output_paths = self.parse_nix_output_paths(&result.stdout);
                if !output_paths.is_empty() {
                    info!(
                        job_id = %job_id,
                        output_paths = ?output_paths,
                        count = output_paths.len(),
                        "found nix output paths for SNIX upload"
                    );
                    let uploaded = self.upload_store_paths_snix(&job_id, &output_paths).await;
                    if !uploaded.is_empty() {
                        info!(
                            job_id = %job_id,
                            count = uploaded.len(),
                            "Uploaded store paths to SNIX binary cache"
                        );
                    } else {
                        warn!(
                            job_id = %job_id,
                            paths_count = output_paths.len(),
                            "SNIX upload returned empty - no paths were uploaded"
                        );
                    }
                } else {
                    warn!(
                        job_id = %job_id,
                        stdout_len = result.stdout.len(),
                        "no nix output paths found in stdout (expected with --print-out-paths)"
                    );
                    debug!(
                        job_id = %job_id,
                        stdout = %result.stdout.chars().take(500).collect::<String>(),
                        "stdout preview for SNIX path parsing"
                    );
                }
            } else if is_nix_command {
                debug!(
                    job_id = %job_id,
                    exit_code = result.exit_code,
                    has_error = result.error.is_some(),
                    "skipping SNIX upload - nix build did not succeed"
                );
            }
        }

        // Phase 4: Parse nix output paths and upload binary to blob store
        let nix_output = self.collect_nix_output(&job_id, &result, payload).await;

        // Phase 5: Collect artifacts (only on success)
        let (artifacts, upload_result) =
            self.collect_and_upload_artifacts(&job_id, &result, payload, &job_workspace).await;

        // Phase 6: Clean up workspace
        self.cleanup_workspace(&job_id, &job_workspace).await;

        Ok((result, artifacts, upload_result, nix_output))
    }

    /// Parse nix output paths from stdout and upload the main binary to blobs.
    async fn collect_nix_output(
        &self,
        job_id: &str,
        result: &ExecutionResult,
        payload: &LocalExecutorPayload,
    ) -> NixOutputInfo {
        let mut nix_output = NixOutputInfo::default();

        let is_nix = payload.command == "nix";
        let is_success = result.exit_code == 0 && result.error.is_none();
        if !is_nix || !is_success {
            return nix_output;
        }

        nix_output.output_paths = parse_nix_output_paths(&result.stdout);
        if nix_output.output_paths.is_empty() {
            debug!(
                job_id = %job_id,
                stdout_len = result.stdout.len(),
                "no nix output paths in stdout (--print-out-paths missing?)"
            );
            return nix_output;
        }

        info!(
            job_id = %job_id,
            output_paths = ?nix_output.output_paths,
            "parsed nix output paths"
        );

        // Find the main binary in the first output path. Convention: bin/<name>
        // where <name> is derived from flake_attr or the derivation name.
        let output_path = &nix_output.output_paths[0];
        let bin_dir = format!("{}/bin", output_path);

        let binary_path = if let Ok(mut entries) = tokio::fs::read_dir(&bin_dir).await {
            let mut found = None;
            while let Ok(Some(entry)) = entries.next_entry().await {
                if entry.file_type().await.map(|t| t.is_file()).unwrap_or(false) {
                    found = Some(entry.path());
                    break;
                }
            }
            found
        } else {
            debug!(job_id = %job_id, bin_dir = %bin_dir, "no bin/ directory in nix output");
            None
        };

        let binary_path = match binary_path {
            Some(p) => p,
            None => return nix_output,
        };

        // Read binary size
        let metadata = match tokio::fs::metadata(&binary_path).await {
            Ok(m) => m,
            Err(e) => {
                warn!(job_id = %job_id, path = ?binary_path, error = ?e, "failed to stat nix output binary");
                return nix_output;
            }
        };

        nix_output.binary_size = Some(metadata.len());
        nix_output.binary_path = Some(binary_path.display().to_string());

        // Upload to blob store if available
        let blob_store = match self.blob_store.as_ref() {
            Some(bs) => bs,
            None => {
                info!(job_id = %job_id, "no blob store — skipping nix output upload");
                return nix_output;
            }
        };

        match tokio::fs::read(&binary_path).await {
            Ok(bytes) => match blob_store.add_bytes(&bytes).await {
                Ok(add_result) => {
                    let hash = add_result.blob_ref.hash.to_hex().to_string();
                    info!(
                        job_id = %job_id,
                        path = ?binary_path,
                        hash = %hash,
                        size = bytes.len(),
                        "uploaded nix output binary to blob store"
                    );
                    nix_output.binary_blob_hash = Some(hash);
                }
                Err(e) => {
                    warn!(job_id = %job_id, error = ?e, "failed to upload nix output to blob store");
                }
            },
            Err(e) => {
                warn!(job_id = %job_id, path = ?binary_path, error = ?e, "failed to read nix output binary");
            }
        }

        nix_output
    }

    /// Build an execution request from the payload.
    ///
    /// If `flake_store_path` is provided, flake references like `.#attr` in the command args
    /// will be rewritten to use the store path directly (e.g., `/nix/store/xxx#attr`).
    ///
    /// If `cache_proxy` is provided (requires `nix-cache-proxy` feature), the nix command
    /// will be configured to use the cluster's binary cache as a substituter.
    #[cfg(feature = "nix-cache-proxy")]
    fn build_execution_request(
        &self,
        job_id: &str,
        payload: &LocalExecutorPayload,
        job_workspace: &std::path::Path,
        flake_store_path: Option<&PathBuf>,
        cache_proxy: Option<&CacheProxy>,
    ) -> ExecutionRequest {
        let working_dir = if payload.working_dir.starts_with('/') {
            PathBuf::from(&payload.working_dir)
        } else {
            job_workspace.join(&payload.working_dir)
        };

        let (command, mut args) = if payload.command == "nix" {
            inject_nix_flags_with_flake_rewrite(&payload.args, flake_store_path, job_id)
        } else {
            (payload.command.clone(), payload.args.clone())
        };

        // Add cache substituter args for nix commands if proxy is running
        if payload.command == "nix"
            && let Some(proxy) = cache_proxy
            && let Some(public_key) = self.config.cache_public_key.as_ref()
        {
            let substituter_url = proxy.substituter_url();

            // Prepend Aspen cache, with cache.nixos.org as fallback
            args.push("--substituters".to_string());
            args.push(format!("{} https://cache.nixos.org", substituter_url));

            // Include both keys for verification
            args.push("--trusted-public-keys".to_string());
            args.push(format!("{} cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=", public_key));

            // Enable fallback to build from source if cache doesn't have it
            args.push("--fallback".to_string());

            debug!(
                job_id = %job_id,
                substituter = %substituter_url,
                "Added cache substituter to nix command"
            );
        }

        let mut env = payload.env.clone();
        env.entry("HOME".to_string()).or_insert_with(|| "/tmp".to_string());

        ExecutionRequest {
            id: job_id.to_string(),
            command,
            args,
            working_dir,
            env,
            timeout_secs: payload.timeout_secs,
        }
    }

    /// Build an execution request from the payload (without cache proxy support).
    ///
    /// If `flake_store_path` is provided, flake references like `.#attr` in the command args
    /// will be rewritten to use the store path directly (e.g., `/nix/store/xxx#attr`).
    #[cfg(not(feature = "nix-cache-proxy"))]
    fn build_execution_request(
        &self,
        job_id: &str,
        payload: &LocalExecutorPayload,
        job_workspace: &std::path::Path,
        flake_store_path: Option<&PathBuf>,
    ) -> ExecutionRequest {
        let working_dir = if payload.working_dir.starts_with('/') {
            PathBuf::from(&payload.working_dir)
        } else {
            job_workspace.join(&payload.working_dir)
        };

        let (command, args) = if payload.command == "nix" {
            inject_nix_flags_with_flake_rewrite(&payload.args, flake_store_path, job_id)
        } else {
            (payload.command.clone(), payload.args.clone())
        };

        let mut env = payload.env.clone();
        env.entry("HOME".to_string()).or_insert_with(|| "/tmp".to_string());

        ExecutionRequest {
            id: job_id.to_string(),
            command,
            args,
            working_dir,
            env,
            timeout_secs: payload.timeout_secs,
        }
    }

    /// Execute a request with log streaming and return the result.
    ///
    /// If the worker has a KV store and the payload has a `run_id`, spawns
    /// a `log_bridge` task for real-time CI log streaming via `_ci:logs:*` KV keys.
    async fn execute_with_streaming(
        &self,
        job_id: &str,
        request: ExecutionRequest,
        payload: &LocalExecutorPayload,
    ) -> Result<ExecutionResult, String> {
        let (log_tx, log_rx) = mpsc::channel::<LogMessage>(1024);

        info!(
            job_id = %job_id,
            command = %payload.command,
            args = ?payload.args,
            timeout_secs = payload.timeout_secs,
            "executing job"
        );

        // Set up KV log bridge for CI log streaming if KV store and run_id are available.
        let kv_log_sender = match (&self.config.kv_store, &payload.run_id) {
            (Some(kv_store), Some(run_id)) if !run_id.is_empty() => {
                let (tx, rx) = mpsc::channel::<String>(1000);
                let kv = kv_store.clone();
                let rid = run_id.clone();
                let jid = job_id.to_string();
                tracing::debug!(run_id = %rid, job_id = %jid, "Starting CI log streaming for shell job");
                let handle = tokio::spawn(crate::common::log_bridge(rx, kv, rid, jid));
                Some((tx, handle))
            }
            _ => None,
        };

        // If an external log sink is set (e.g., VM worker streaming to cluster),
        // interpose a forwarder that sends messages to both the internal consumer
        // and the external sink.
        let kv_log_tx = kv_log_sender.as_ref().map(|(tx, _)| tx.clone());
        let log_consumer = if self.log_sink.is_some() || kv_log_tx.is_some() {
            let (fwd_tx, fwd_rx) = mpsc::channel::<LogMessage>(1024);
            let sink_clone = self.log_sink.clone();
            let kv_tx = kv_log_tx;

            // Forwarder task: reads from log_rx, sends to internal consumer,
            // external sink (if set), and KV log bridge (if set)
            tokio::spawn(async move {
                let mut log_rx = log_rx;
                while let Some(msg) = log_rx.recv().await {
                    // Forward to external sink (best-effort, don't block execution)
                    if let Some(ref sink) = sink_clone {
                        let _ = sink.try_send(msg.clone());
                    }
                    // Forward to KV log bridge for CI log streaming
                    if let Some(ref kv) = kv_tx {
                        let line = match &msg {
                            LogMessage::Stdout(s) | LogMessage::Stderr(s) => s.clone(),
                            LogMessage::Complete(_) | LogMessage::Heartbeat { .. } => String::new(),
                        };
                        if !line.is_empty() {
                            let _ = kv.send(line).await;
                        }
                    }
                    // Forward to internal consumer
                    if fwd_tx.send(msg).await.is_err() {
                        break;
                    }
                }
            });

            spawn_log_consumer(job_id.to_string(), fwd_rx)
        } else {
            spawn_log_consumer(job_id.to_string(), log_rx)
        };

        let exec_result = self.executor.execute(request, log_tx).await;

        // Log consumer task is non-critical; if it panics, empty logs are acceptable
        let (collected_stdout, collected_stderr) = log_consumer.await.unwrap_or_default();

        // Drop KV log sender and await bridge completion (flushes remaining buffer + writes marker)
        if let Some((tx, handle)) = kv_log_sender {
            drop(tx);
            let _ = handle.await;
        }

        match exec_result {
            Ok(mut result) => {
                if result.stdout.is_empty() {
                    result.stdout = collected_stdout;
                }
                if result.stderr.is_empty() {
                    result.stderr = collected_stderr;
                }
                Ok(result)
            }
            Err(e) => Err(format!("execution failed: {}", e)),
        }
    }
}
