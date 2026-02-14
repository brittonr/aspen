//! Core job execution logic: orchestration, request building, and streaming.

use std::path::PathBuf;
#[cfg(feature = "nix-cache-proxy")]
use std::sync::Arc;

use aspen_jobs::Job;
use tokio::sync::mpsc;
#[cfg(any(feature = "nix-cache-proxy", feature = "snix"))]
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
    ) -> Result<(ExecutionResult, ArtifactCollectionResult, Option<ArtifactUploadResult>), String> {
        let job_id = job.id.to_string();

        // Phase 1: Set up workspace (returns workspace path and optional flake store path)
        let (job_workspace, flake_store_path) = self
            .setup_job_workspace(&job_id, payload)
            .await
            .map_err(|e| format!("workspace setup failed: {}", e))?;

        // Phase 2: Start cache proxy for nix commands if configured
        #[cfg(feature = "nix-cache-proxy")]
        let cache_proxy = if payload.command == "nix" && self.config.can_use_cache_proxy() {
            let endpoint = self.config.iroh_endpoint.as_ref().expect("validated by can_use_cache_proxy");
            let gateway = self.config.gateway_node.expect("validated by can_use_cache_proxy");

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
        if result.exit_code == 0 && result.error.is_none() && payload.command == "nix" {
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
        }
        #[cfg(feature = "snix")]
        if (result.exit_code != 0 || result.error.is_some()) && payload.command == "nix" {
            debug!(
                job_id = %job_id,
                exit_code = result.exit_code,
                has_error = result.error.is_some(),
                "skipping SNIX upload - nix build did not succeed"
            );
        }

        // Phase 4: Collect artifacts (only on success)
        let (artifacts, upload_result) =
            self.collect_and_upload_artifacts(&job_id, &result, payload, &job_workspace).await;

        // Phase 5: Clean up workspace
        self.cleanup_workspace(&job_id, &job_workspace).await;

        Ok((result, artifacts, upload_result))
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
        {
            let substituter_url = proxy.substituter_url();
            let public_key = self.config.cache_public_key.as_ref().expect("validated by can_use_cache_proxy");

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

        let log_consumer = spawn_log_consumer(job_id.to_string(), log_rx);
        let exec_result = self.executor.execute(request, log_tx).await;

        // Log consumer task is non-critical; if it panics, empty logs are acceptable
        let (collected_stdout, collected_stderr) = log_consumer.await.unwrap_or_default();

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
