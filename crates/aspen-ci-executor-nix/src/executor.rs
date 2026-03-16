//! Core Nix build execution logic.

use std::process::Stdio;
#[cfg(feature = "nix-cache-proxy")]
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use aspen_ci_core::CiCoreError;
use aspen_ci_core::Result;
#[cfg(feature = "nix-cache-proxy")]
use aspen_ci_executor_shell::CacheProxy;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Child;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::debug;
use tracing::info;
#[cfg(any(feature = "snix-build", feature = "nix-cache-proxy"))]
use tracing::warn;

use crate::config::MAX_LOG_SIZE;
use crate::config::NixBuildWorkerConfig;
use crate::payload::NixBuildPayload;
use crate::timing::BuildPhaseTimings;

/// Output from a Nix build.
#[derive(Debug, Clone)]
pub(crate) struct NixBuildOutput {
    /// Paths to build outputs in /nix/store.
    pub(crate) output_paths: Vec<String>,
    /// Build log.
    pub(crate) log: String,
    /// Whether the log was truncated.
    pub(crate) log_truncated: bool,
    /// Build phase timings.
    pub(crate) timings: BuildPhaseTimings,
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
///
/// When the `snix-build` feature is enabled, builds execute in-process
/// via snix-build's `BuildService` (bubblewrap/OCI sandbox) instead of
/// shelling out to `nix build`. The subprocess path is retained behind
/// the `nix-cli-fallback` feature flag.
pub struct NixBuildWorker {
    pub(crate) config: NixBuildWorkerConfig,
    /// Native build service for in-process builds (when snix-build feature is enabled).
    #[cfg(feature = "snix-build")]
    pub(crate) native_build_service: Option<crate::build_service::NativeBuildService>,
}

impl NixBuildWorker {
    /// Create a new Nix build worker with the given configuration.
    pub fn new(config: NixBuildWorkerConfig) -> Self {
        Self {
            config,
            #[cfg(feature = "snix-build")]
            native_build_service: None,
        }
    }

    /// Create a worker with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(NixBuildWorkerConfig::default())
    }

    /// Initialize the native build service from the worker's config.
    ///
    /// Call this after construction to enable in-process builds.
    /// If initialization fails (missing services, sandbox unavailable),
    /// the worker falls back to subprocess execution.
    #[cfg(feature = "snix-build")]
    pub async fn init_native_build_service(&mut self) {
        match crate::build_service::init_from_config(&self.config).await {
            Some((service, backend)) => {
                info!(
                    backend = %backend,
                    "native build service initialized"
                );
                self.native_build_service = Some(service);
            }
            None => {
                info!("native build service not available, using subprocess fallback");
            }
        }
    }

    /// Check if native builds are available.
    #[cfg(feature = "snix-build")]
    pub fn has_native_builds(&self) -> bool {
        self.native_build_service.is_some()
    }

    /// Resolve a flake ref to a `.drv` file path via `nix eval --raw`.
    ///
    /// Runs `nix eval --raw <flake_ref>.drvPath` to get the derivation
    /// store path. This is the bridge between flake evaluation and the
    /// native build pipeline — cheap (~100ms) compared to the build itself.
    #[cfg(feature = "snix-build")]
    pub(crate) async fn resolve_drv_path(
        &self,
        payload: &NixBuildPayload,
        flake_ref: &str,
    ) -> std::result::Result<std::path::PathBuf, CiCoreError> {
        // Build the installable ref: "<flake_ref>.drvPath"
        // NOTE: we pass this as a positional installable, NOT --expr.
        // With --expr, nix interprets absolute paths like /tmp/foo as Nix
        // path literals, which are forbidden in pure evaluation mode.
        let installable = format!("{flake_ref}.drvPath");

        let mut cmd = Command::new(&self.config.nix_binary);
        cmd.arg("eval").arg("--raw").arg(&installable);

        if let Some(ref dir) = payload.working_dir {
            cmd.current_dir(dir);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd.spawn().map_err(|e| CiCoreError::NixBuildFailed {
            flake: flake_ref.to_string(),
            reason: format!("failed to spawn nix eval for drv path: {e}"),
        })?;

        // Timeout: 60 seconds for eval (generous, most take <5s)
        let timeout = Duration::from_secs(60);
        let output = tokio::time::timeout(timeout, child.wait_with_output())
            .await
            .map_err(|_| CiCoreError::NixBuildFailed {
                flake: flake_ref.to_string(),
                reason: "nix eval --raw timed out after 60s resolving .drvPath".to_string(),
            })?
            .map_err(|e| CiCoreError::NixBuildFailed {
                flake: flake_ref.to_string(),
                reason: format!("nix eval failed: {e}"),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CiCoreError::NixBuildFailed {
                flake: flake_ref.to_string(),
                reason: format!("nix eval .drvPath failed (exit {}): {stderr}", output.status.code().unwrap_or(-1)),
            });
        }

        let drv_path_str = String::from_utf8_lossy(&output.stdout);
        let drv_path = std::path::PathBuf::from(drv_path_str.trim());

        if !drv_path.to_string_lossy().starts_with("/nix/store/") || !drv_path.to_string_lossy().ends_with(".drv") {
            return Err(CiCoreError::NixBuildFailed {
                flake: flake_ref.to_string(),
                reason: format!("nix eval returned unexpected drv path: {}", drv_path.display()),
            });
        }

        debug!(
            drv_path = %drv_path.display(),
            flake_ref = %flake_ref,
            "resolved flake to derivation path"
        );

        Ok(drv_path)
    }

    /// Attempt a native in-process build via snix-build.
    ///
    /// Resolves the flake to a `.drv` path, parses the derivation,
    /// executes via NativeBuildService, uploads outputs to PathInfoService,
    /// and returns the build output. Falls back on any error.
    #[cfg(feature = "snix-build")]
    pub(crate) async fn try_native_build(
        &self,
        payload: &NixBuildPayload,
        flake_ref: &str,
        log_sender: Option<mpsc::Sender<String>>,
    ) -> std::result::Result<NixBuildOutput, CiCoreError> {
        let service = self.native_build_service.as_ref().ok_or_else(|| CiCoreError::NixBuildFailed {
            flake: flake_ref.to_string(),
            reason: "native build service not initialized".to_string(),
        })?;

        // Step 1: Resolve flake ref to .drv path
        let drv_path = self.resolve_drv_path(payload, flake_ref).await?;

        // Step 2: Read and parse the .drv file
        let drv_bytes = tokio::fs::read(&drv_path).await.map_err(|e| CiCoreError::NixBuildFailed {
            flake: flake_ref.to_string(),
            reason: format!("failed to read {}: {e}", drv_path.display()),
        })?;

        let (drv, _output_paths) =
            crate::eval::parse_derivation(&drv_bytes).map_err(|e| CiCoreError::NixBuildFailed {
                flake: flake_ref.to_string(),
                reason: format!("failed to parse {}: {e}", drv_path.display()),
            })?;

        info!(
            drv_path = %drv_path.display(),
            output_count = drv.outputs.len(),
            system = %drv.system,
            "parsed derivation, starting native build"
        );

        if let Some(ref tx) = log_sender {
            let _ = tx.send(format!("native build: resolved {} → {}\n", flake_ref, drv_path.display())).await;
        }

        // Step 3: Execute native build — call build_derivation directly so
        // we keep the NativeBuildResult for upload_native_outputs below.
        let build_result =
            service.build_derivation(&drv, log_sender.clone()).await.map_err(|e| CiCoreError::NixBuildFailed {
                flake: flake_ref.to_string(),
                reason: format!("native build failed: {e}"),
            })?;

        let output_paths: Vec<String> = build_result.outputs.iter().map(|o| o.store_path.to_absolute_path()).collect();

        info!(
            output_paths = ?output_paths,
            resolve_ms = build_result.resolve_ms,
            build_ms = build_result.build_ms,
            "native build succeeded"
        );

        // Step 4: Upload outputs to PathInfoService (more efficient than the
        // subprocess path — we already have the Nodes from the build, no need
        // to re-read from disk and re-create NAR archives).
        if let (Some(pathinfo_svc), Some(blob_svc), Some(dir_svc)) = (
            &self.config.snix_pathinfo_service,
            &self.config.snix_blob_service,
            &self.config.snix_directory_service,
        ) {
            let nar_calc =
                snix_store::nar::SimpleRenderer::new(std::sync::Arc::clone(blob_svc), std::sync::Arc::clone(dir_svc));
            let uploaded =
                crate::build_service::upload_native_outputs(pathinfo_svc.as_ref(), &nar_calc, &build_result.outputs)
                    .await;
            if !uploaded.is_empty() {
                info!(count = uploaded.len(), "uploaded native build outputs to PathInfoService");
            }
        }

        // Convert to NixBuildOutput format
        let log = format!(
            "native build completed: {} outputs, resolve={}ms build={}ms\n",
            output_paths.len(),
            build_result.resolve_ms,
            build_result.build_ms,
        );

        let mut timings = BuildPhaseTimings::default();
        timings.record_import(Duration::from_millis(build_result.resolve_ms));
        timings.record_build(Duration::from_millis(build_result.build_ms));

        Ok(NixBuildOutput {
            output_paths,
            log,
            log_truncated: false,
            timings,
        })
    }

    /// Execute a Nix build.
    ///
    /// If `log_sender` is provided, stderr lines are also forwarded to it
    /// in real-time for streaming to the CI log infrastructure.
    pub(crate) async fn execute_build(
        &self,
        payload: &NixBuildPayload,
        log_sender: Option<mpsc::Sender<String>>,
    ) -> Result<NixBuildOutput> {
        let mut timings = BuildPhaseTimings::default();

        payload.validate()?;

        let flake_ref = payload.flake_ref();
        info!(
            cluster_id = %self.config.cluster_id,
            node_id = self.config.node_id,
            flake_ref = %flake_ref,
            "Starting Nix build"
        );

        // Phase 1: Import/setup (resolve cache, start proxy)
        let import_start = Instant::now();

        // Lazy-resolve the cache public key in case it wasn't available at
        // worker startup (signing key created after cluster init).
        self.config.resolve_cache_public_key().await;

        #[cfg(feature = "nix-cache-proxy")]
        let cache_proxy = self.start_cache_proxy(&flake_ref).await?;
        #[cfg(not(feature = "nix-cache-proxy"))]
        let _cache_proxy = ();

        timings.record_import(import_start.elapsed());

        // Phase 2: Build execution
        //
        // Try native snix-build path first when available. Falls back to
        // subprocess on any error (eval failure, sandbox issue, etc.).

        #[cfg(feature = "snix-build")]
        let native_result = if self.has_native_builds() {
            let build_start = Instant::now();
            match self.try_native_build(payload, &flake_ref, log_sender.clone()).await {
                Ok(output) => {
                    timings.record_build(build_start.elapsed());
                    Some(output)
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        flake_ref = %flake_ref,
                        "native build failed, falling back to subprocess"
                    );
                    if let Some(ref tx) = log_sender {
                        let _ =
                            tx.send(format!("native build failed ({e}), falling back to nix build subprocess\n")).await;
                    }
                    None
                }
            }
        } else {
            None
        };

        #[cfg(feature = "snix-build")]
        if let Some(native_output) = native_result {
            // Phase 3: Upload/cleanup for native builds
            let upload_start = Instant::now();

            #[cfg(feature = "nix-cache-proxy")]
            if let Some(proxy) = cache_proxy {
                debug!("Shutting down cache proxy");
                proxy.shutdown().await;
            }

            timings.record_upload(upload_start.elapsed());

            info!(
                cluster_id = %self.config.cluster_id,
                node_id = self.config.node_id,
                output_paths = ?native_output.output_paths,
                import_ms = timings.import_ms,
                build_ms = timings.build_ms,
                upload_ms = timings.upload_ms,
                "Nix build completed successfully (native)"
            );

            return Ok(NixBuildOutput {
                output_paths: native_output.output_paths,
                log: native_output.log,
                log_truncated: native_output.log_truncated,
                timings,
            });
        }

        // Subprocess fallback path
        let build_start_sub = Instant::now();

        let mut child = self.spawn_nix_build(payload, &flake_ref)?;
        let readers = Self::spawn_output_readers(&mut child, &flake_ref, self.config.is_verbose, log_sender)?;
        let (output_paths, log, log_size) = Self::collect_output(readers).await?;
        Self::wait_for_build_completion(&mut child, payload.timeout_secs, &flake_ref, &log).await?;

        timings.record_build(build_start_sub.elapsed());

        // Phase 3: Upload/cleanup (proxy shutdown)
        let upload_start = Instant::now();

        #[cfg(feature = "nix-cache-proxy")]
        if let Some(proxy) = cache_proxy {
            debug!("Shutting down cache proxy");
            proxy.shutdown().await;
        }

        timings.record_upload(upload_start.elapsed());

        info!(
            cluster_id = %self.config.cluster_id,
            node_id = self.config.node_id,
            output_paths = ?output_paths,
            import_ms = timings.import_ms,
            build_ms = timings.build_ms,
            upload_ms = timings.upload_ms,
            "Nix build completed successfully"
        );

        Ok(NixBuildOutput {
            output_paths,
            log,
            log_truncated: log_size >= MAX_LOG_SIZE,
            timings,
        })
    }

    /// Start a cache proxy if the worker is configured to use the cluster cache.
    ///
    /// Returns `Ok(Some(proxy))` if started successfully, `Ok(None)` if not
    /// configured or if the proxy fails to start (with a warning logged).
    #[cfg(feature = "nix-cache-proxy")]
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
    fn spawn_nix_build(&self, payload: &NixBuildPayload, flake_ref: &str) -> Result<Child> {
        let mut cmd = Command::new(&self.config.nix_binary);
        // Use --no-link instead of --out-link to avoid creating a "result"
        // symlink in cwd. Under ProtectSystem=strict the working directory
        // is often read-only (/). We parse output paths from --print-out-paths.
        cmd.arg("build").arg(flake_ref).arg("--no-link").arg("--print-out-paths");

        if payload.sandbox {
            cmd.arg("--sandbox");
        } else {
            cmd.arg("--no-sandbox");
        }

        // Inject gateway-based substituter if configured
        if let Some(sub_args) = self.config.substituter_args() {
            for arg in &sub_args {
                cmd.arg(arg);
            }
            info!("Nix build using cluster cache substituter: {:?}", sub_args);
        } else {
            info!(
                gateway_url = ?self.config.gateway_url,
                has_key = self.config.get_public_key().is_some(),
                "No cache substituter available for this build"
            );
        }

        if self.config.is_verbose {
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
    #[cfg(feature = "nix-cache-proxy")]
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
    fn spawn_output_readers(
        child: &mut Child,
        flake_ref: &str,
        verbose: bool,
        log_sender: Option<mpsc::Sender<String>>,
    ) -> Result<OutputReaders> {
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
        let stderr_task = tokio::spawn(read_stderr_log(
            BufReader::new(stderr),
            stderr_tx,
            log_sender,
            flake_ref.to_string(),
            verbose,
        ));

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
/// When `log_sender` is provided, lines are also forwarded for real-time streaming.
async fn read_stderr_log(
    mut reader: BufReader<tokio::process::ChildStderr>,
    tx: mpsc::Sender<String>,
    log_sender: Option<mpsc::Sender<String>>,
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
                if let Some(ref log_tx) = log_sender {
                    let _ = log_tx.send(line.clone()).await;
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
