//! Core Nix build execution logic.

use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use aspen_ci_core::CiCoreError;
use aspen_ci_core::Result;
use aspen_ci_executor_shell::CacheProxy;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::config::MAX_LOG_SIZE;
use crate::config::NixBuildWorkerConfig;
use crate::payload::NixBuildPayload;

/// Output from a Nix build.
#[derive(Debug, Clone)]
pub(crate) struct NixBuildOutput {
    /// Paths to build outputs in /nix/store.
    pub(crate) output_paths: Vec<String>,
    /// Build log.
    pub(crate) log: String,
    /// Whether the log was truncated.
    pub(crate) log_truncated: bool,
}

/// Worker that executes Nix flake builds.
///
/// This worker:
/// 1. Validates the build payload
/// 2. Executes `nix build` with the specified flake reference
/// 3. Captures build logs
/// 4. Optionally stores artifacts in the blob store
/// 5. Returns build output paths and artifact hashes
pub struct NixBuildWorker {
    pub(crate) config: NixBuildWorkerConfig,
}

impl NixBuildWorker {
    /// Create a new Nix build worker with the given configuration.
    pub fn new(config: NixBuildWorkerConfig) -> Self {
        Self { config }
    }

    /// Create a worker with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(NixBuildWorkerConfig::default())
    }

    /// Execute a Nix build.
    pub(crate) async fn execute_build(&self, payload: &NixBuildPayload) -> Result<NixBuildOutput> {
        payload.validate()?;

        let flake_ref = payload.flake_ref();
        info!(
            cluster_id = %self.config.cluster_id,
            node_id = self.config.node_id,
            flake_ref = %flake_ref,
            "Starting Nix build"
        );

        // Start cache proxy if configured
        let cache_proxy = if self.config.can_use_cache_proxy() {
            let endpoint = self.config.iroh_endpoint.as_ref().ok_or_else(|| CiCoreError::NixBuildFailed {
                flake: flake_ref.clone(),
                reason: "iroh endpoint not configured".to_string(),
            })?;
            let gateway = self.config.gateway_node.ok_or_else(|| CiCoreError::NixBuildFailed {
                flake: flake_ref.clone(),
                reason: "gateway node not configured".to_string(),
            })?;

            match CacheProxy::start(Arc::clone(endpoint), gateway).await {
                Ok(proxy) => {
                    info!(
                        substituter_url = %proxy.substituter_url(),
                        "Started cache proxy for Nix build"
                    );
                    Some(proxy)
                }
                Err(e) => {
                    warn!(error = %e, "Failed to start cache proxy, continuing without substituter");
                    None
                }
            }
        } else {
            None
        };

        // Build the command
        let mut cmd = Command::new(&self.config.nix_binary);
        cmd.arg("build").arg(&flake_ref).arg("--out-link").arg("result").arg("--print-out-paths");

        // Add cache substituter args if proxy is running
        if let Some(ref proxy) = cache_proxy {
            let substituter_url = proxy.substituter_url();
            let public_key = self.config.cache_public_key.as_ref().ok_or_else(|| CiCoreError::NixBuildFailed {
                flake: flake_ref.clone(),
                reason: "cache public key not configured".to_string(),
            })?;

            // Prepend Aspen cache, with cache.nixos.org as fallback
            cmd.arg("--substituters").arg(format!("{} https://cache.nixos.org", substituter_url));

            // Include both keys for verification
            cmd.arg("--trusted-public-keys")
                .arg(format!("{} cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=", public_key));

            // Enable fallback to build from source if cache doesn't have it
            cmd.arg("--fallback");

            debug!(
                substituter = %substituter_url,
                "Configured Aspen cache as primary substituter"
            );
        }

        // Add sandbox flag
        if payload.sandbox {
            cmd.arg("--sandbox");
        } else {
            cmd.arg("--no-sandbox");
        }

        // Add verbose flag if configured
        if self.config.verbose {
            cmd.arg("-L"); // Print build logs
        }

        // Add extra arguments
        for arg in &payload.extra_args {
            cmd.arg(arg);
        }

        // Set working directory
        if let Some(ref dir) = payload.working_dir {
            cmd.current_dir(dir);
        }

        // Capture output
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn process
        let mut child = cmd.spawn().map_err(|e| CiCoreError::NixBuildFailed {
            flake: flake_ref.clone(),
            reason: format!("Failed to spawn nix: {e}"),
        })?;

        // Capture stdout and stderr concurrently to avoid deadlock.
        // If we read them sequentially, the process can block if one pipe buffer fills
        // while we're waiting on the other.
        let stdout = child.stdout.take().ok_or_else(|| CiCoreError::NixBuildFailed {
            flake: flake_ref.clone(),
            reason: "stdout pipe not available".to_string(),
        })?;
        let stderr = child.stderr.take().ok_or_else(|| CiCoreError::NixBuildFailed {
            flake: flake_ref.clone(),
            reason: "stderr pipe not available".to_string(),
        })?;

        // Use channels to collect output from concurrent tasks
        let (stdout_tx, mut stdout_rx) = mpsc::channel::<String>(100);
        let (stderr_tx, mut stderr_rx) = mpsc::channel::<String>(1000);

        let flake_ref_clone = flake_ref.clone();
        let stdout_task = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                match reader.read_line(&mut line).await {
                    Ok(0) => break Ok(()), // EOF
                    Ok(_) => {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() && trimmed.starts_with("/nix/store/") {
                            // Ignore send errors - receiver may have dropped
                            let _ = stdout_tx.send(trimmed.to_string()).await;
                        }
                        line.clear();
                    }
                    Err(e) => {
                        break Err(CiCoreError::NixBuildFailed {
                            flake: flake_ref_clone.clone(),
                            reason: format!("Failed to read stdout: {e}"),
                        });
                    }
                }
            }
        });

        let flake_ref_clone = flake_ref.clone();
        let verbose = self.config.verbose;
        let stderr_task = tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                match reader.read_line(&mut line).await {
                    Ok(0) => break Ok(()), // EOF
                    Ok(_) => {
                        if verbose {
                            debug!(line = %line.trim(), "nix build");
                        }
                        // Ignore send errors - receiver may have dropped
                        let _ = stderr_tx.send(line.clone()).await;
                        line.clear();
                    }
                    Err(e) => {
                        break Err(CiCoreError::NixBuildFailed {
                            flake: flake_ref_clone.clone(),
                            reason: format!("Failed to read stderr: {e}"),
                        });
                    }
                }
            }
        });

        // Collect results while tasks run
        let mut output_paths = Vec::new();
        let mut log_lines = Vec::new();
        let mut log_size = 0usize;

        // Collect from both channels concurrently
        loop {
            tokio::select! {
                Some(path) = stdout_rx.recv() => {
                    output_paths.push(path);
                }
                Some(line) = stderr_rx.recv() => {
                    if log_size < MAX_LOG_SIZE {
                        log_size += line.len();
                        log_lines.push(line);
                    }
                }
                else => break, // Both channels closed
            }
        }

        // Wait for tasks to complete and check for errors
        let stdout_result = stdout_task.await.map_err(|e| CiCoreError::NixBuildFailed {
            flake: flake_ref.clone(),
            reason: format!("stdout task panicked: {e}"),
        })?;
        stdout_result?;

        let stderr_result = stderr_task.await.map_err(|e| CiCoreError::NixBuildFailed {
            flake: flake_ref.clone(),
            reason: format!("stderr task panicked: {e}"),
        })?;
        stderr_result?;

        // Wait for completion with timeout
        let timeout = Duration::from_secs(payload.timeout_secs);
        let status = tokio::time::timeout(timeout, child.wait())
            .await
            .map_err(|_| CiCoreError::Timeout {
                timeout_secs: payload.timeout_secs,
            })?
            .map_err(|e| CiCoreError::NixBuildFailed {
                flake: flake_ref.clone(),
                reason: format!("Failed to wait for nix: {e}"),
            })?;

        let log = log_lines.join("");

        if !status.success() {
            let exit_code = status.code().unwrap_or(-1);
            return Err(CiCoreError::NixBuildFailed {
                flake: flake_ref,
                reason: format!("Build failed with exit code {exit_code}\n{log}"),
            });
        }

        // Shutdown cache proxy gracefully
        if let Some(proxy) = cache_proxy {
            debug!("Shutting down cache proxy");
            proxy.shutdown().await;
        }

        info!(
            cluster_id = %self.config.cluster_id,
            node_id = self.config.node_id,
            output_paths = ?output_paths,
            "Nix build completed successfully"
        );

        Ok(NixBuildOutput {
            output_paths,
            log,
            log_truncated: log_size >= MAX_LOG_SIZE,
        })
    }
}
