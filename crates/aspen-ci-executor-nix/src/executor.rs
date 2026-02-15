//! Core Nix build execution logic.

use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use aspen_ci_core::CiCoreError;
use aspen_ci_core::Result;
use aspen_ci_executor_shell::CacheProxy;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Child;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
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

/// Handles for concurrent stdout/stderr reader tasks.
struct OutputReaders {
    stdout_task: JoinHandle<std::result::Result<(), CiCoreError>>,
    stderr_task: JoinHandle<std::result::Result<(), CiCoreError>>,
    stdout_rx: mpsc::Receiver<String>,
    stderr_rx: mpsc::Receiver<String>,
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

        let cache_proxy = self.start_cache_proxy(&flake_ref).await?;
        let mut child = self.spawn_nix_build(payload, &flake_ref, cache_proxy.as_ref())?;
        let readers = Self::spawn_output_readers(&mut child, &flake_ref, self.config.verbose)?;
        let (output_paths, log, log_size) = Self::collect_output(readers).await?;
        Self::wait_for_build_completion(&mut child, payload.timeout_secs, &flake_ref, &log).await?;

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

    /// Start a cache proxy if the worker is configured to use the cluster cache.
    ///
    /// Returns `Ok(Some(proxy))` if started successfully, `Ok(None)` if not
    /// configured or if the proxy fails to start (with a warning logged).
    async fn start_cache_proxy(&self, flake_ref: &str) -> Result<Option<CacheProxy>> {
        if !self.config.can_use_cache_proxy() {
            return Ok(None);
        }

        let endpoint = self.config.iroh_endpoint.as_ref().ok_or_else(|| CiCoreError::NixBuildFailed {
            flake: flake_ref.to_string(),
            reason: "iroh endpoint not configured".to_string(),
        })?;
        let gateway = self.config.gateway_node.ok_or_else(|| CiCoreError::NixBuildFailed {
            flake: flake_ref.to_string(),
            reason: "gateway node not configured".to_string(),
        })?;

        match CacheProxy::start(Arc::clone(endpoint), gateway).await {
            Ok(proxy) => {
                info!(
                    substituter_url = %proxy.substituter_url(),
                    "Started cache proxy for Nix build"
                );
                Ok(Some(proxy))
            }
            Err(e) => {
                warn!(error = %e, "Failed to start cache proxy, continuing without substituter");
                Ok(None)
            }
        }
    }

    /// Build and spawn the `nix build` command.
    ///
    /// Configures the command with cache substituters, sandbox mode, extra args,
    /// working directory, and piped stdout/stderr, then spawns the child process.
    fn spawn_nix_build(
        &self,
        payload: &NixBuildPayload,
        flake_ref: &str,
        cache_proxy: Option<&CacheProxy>,
    ) -> Result<Child> {
        let mut cmd = Command::new(&self.config.nix_binary);
        cmd.arg("build").arg(flake_ref).arg("--out-link").arg("result").arg("--print-out-paths");

        if let Some(proxy) = cache_proxy {
            self.configure_cache_substituter(&mut cmd, proxy, flake_ref)?;
        }

        if payload.sandbox {
            cmd.arg("--sandbox");
        } else {
            cmd.arg("--no-sandbox");
        }

        if self.config.verbose {
            cmd.arg("-L");
        }

        for arg in &payload.extra_args {
            cmd.arg(arg);
        }

        if let Some(ref dir) = payload.working_dir {
            cmd.current_dir(dir);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        cmd.spawn().map_err(|e| CiCoreError::NixBuildFailed {
            flake: flake_ref.to_string(),
            reason: format!("Failed to spawn nix: {e}"),
        })
    }

    /// Configure cache substituter arguments on the nix build command.
    fn configure_cache_substituter(&self, cmd: &mut Command, proxy: &CacheProxy, flake_ref: &str) -> Result<()> {
        let substituter_url = proxy.substituter_url();
        let public_key = self.config.cache_public_key.as_ref().ok_or_else(|| CiCoreError::NixBuildFailed {
            flake: flake_ref.to_string(),
            reason: "cache public key not configured".to_string(),
        })?;

        cmd.arg("--substituters").arg(format!("{} https://cache.nixos.org", substituter_url));
        cmd.arg("--trusted-public-keys")
            .arg(format!("{} cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=", public_key));
        cmd.arg("--fallback");

        debug!(
            substituter = %substituter_url,
            "Configured Aspen cache as primary substituter"
        );

        Ok(())
    }

    /// Spawn concurrent tasks to read stdout and stderr from the child process.
    ///
    /// Returns reader handles and channels for collecting output. Stdout is
    /// filtered for `/nix/store/` paths; stderr is forwarded as build log lines.
    fn spawn_output_readers(child: &mut Child, flake_ref: &str, verbose: bool) -> Result<OutputReaders> {
        let stdout = child.stdout.take().ok_or_else(|| CiCoreError::NixBuildFailed {
            flake: flake_ref.to_string(),
            reason: "stdout pipe not available".to_string(),
        })?;
        let stderr = child.stderr.take().ok_or_else(|| CiCoreError::NixBuildFailed {
            flake: flake_ref.to_string(),
            reason: "stderr pipe not available".to_string(),
        })?;

        let (stdout_tx, stdout_rx) = mpsc::channel::<String>(100);
        let (stderr_tx, stderr_rx) = mpsc::channel::<String>(1000);

        let stdout_task = tokio::spawn(read_stdout_paths(BufReader::new(stdout), stdout_tx, flake_ref.to_string()));
        let stderr_task =
            tokio::spawn(read_stderr_log(BufReader::new(stderr), stderr_tx, flake_ref.to_string(), verbose));

        Ok(OutputReaders {
            stdout_task,
            stderr_task,
            stdout_rx,
            stderr_rx,
        })
    }

    /// Collect output paths and log lines from the reader channels, then
    /// await the reader tasks to check for errors.
    async fn collect_output(readers: OutputReaders) -> Result<(Vec<String>, String, usize)> {
        let OutputReaders {
            stdout_task,
            stderr_task,
            mut stdout_rx,
            mut stderr_rx,
        } = readers;

        let mut output_paths = Vec::new();
        let mut log_lines = Vec::new();
        let mut log_size = 0usize;

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
                else => break,
            }
        }

        stdout_task.await.map_err(|e| CiCoreError::NixBuildFailed {
            flake: String::new(),
            reason: format!("stdout task panicked: {e}"),
        })??;

        stderr_task.await.map_err(|e| CiCoreError::NixBuildFailed {
            flake: String::new(),
            reason: format!("stderr task panicked: {e}"),
        })??;

        let log = log_lines.join("");
        Ok((output_paths, log, log_size))
    }

    /// Wait for the nix build process to exit, enforcing a timeout.
    ///
    /// Returns an error if the process times out, fails to wait, or exits
    /// with a non-zero status code.
    async fn wait_for_build_completion(child: &mut Child, timeout_secs: u64, flake_ref: &str, log: &str) -> Result<()> {
        let timeout = Duration::from_secs(timeout_secs);
        let status = tokio::time::timeout(timeout, child.wait())
            .await
            .map_err(|_| CiCoreError::Timeout { timeout_secs })?
            .map_err(|e| CiCoreError::NixBuildFailed {
                flake: flake_ref.to_string(),
                reason: format!("Failed to wait for nix: {e}"),
            })?;

        if !status.success() {
            let exit_code = status.code().unwrap_or(-1);
            return Err(CiCoreError::NixBuildFailed {
                flake: flake_ref.to_string(),
                reason: format!("Build failed with exit code {exit_code}\n{log}"),
            });
        }

        Ok(())
    }
}

/// Read stdout lines and send any `/nix/store/` paths through the channel.
async fn read_stdout_paths(
    mut reader: BufReader<tokio::process::ChildStdout>,
    tx: mpsc::Sender<String>,
    flake_ref: String,
) -> std::result::Result<(), CiCoreError> {
    let mut line = String::new();
    loop {
        match reader.read_line(&mut line).await {
            Ok(0) => break Ok(()),
            Ok(_) => {
                let trimmed = line.trim();
                if !trimmed.is_empty() && trimmed.starts_with("/nix/store/") {
                    let _ = tx.send(trimmed.to_string()).await;
                }
                line.clear();
            }
            Err(e) => {
                break Err(CiCoreError::NixBuildFailed {
                    flake: flake_ref,
                    reason: format!("Failed to read stdout: {e}"),
                });
            }
        }
    }
}

/// Read stderr lines as build log output, optionally logging each line.
async fn read_stderr_log(
    mut reader: BufReader<tokio::process::ChildStderr>,
    tx: mpsc::Sender<String>,
    flake_ref: String,
    verbose: bool,
) -> std::result::Result<(), CiCoreError> {
    let mut line = String::new();
    loop {
        match reader.read_line(&mut line).await {
            Ok(0) => break Ok(()),
            Ok(_) => {
                if verbose {
                    debug!(line = %line.trim(), "nix build");
                }
                let _ = tx.send(line.clone()).await;
                line.clear();
            }
            Err(e) => {
                break Err(CiCoreError::NixBuildFailed {
                    flake: flake_ref,
                    reason: format!("Failed to read stderr: {e}"),
                });
            }
        }
    }
}
