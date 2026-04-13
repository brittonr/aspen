//! Core Nix build execution logic.

#[cfg(feature = "nix-cli-fallback")]
use std::process::Stdio;
#[cfg(feature = "nix-cache-proxy")]
use std::sync::Arc;
#[cfg(any(feature = "snix-build", feature = "nix-cli-fallback"))]
use std::time::Duration;
use std::time::Instant;

use aspen_ci_core::CiCoreError;
use aspen_ci_core::Result;
#[cfg(feature = "nix-cache-proxy")]
use aspen_ci_executor_shell::CacheProxy;
#[cfg(feature = "snix-build")]
use nix_compat::derivation::Derivation;
#[cfg(feature = "nix-cli-fallback")]
use tokio::io::AsyncBufReadExt;
#[cfg(feature = "nix-cli-fallback")]
use tokio::io::BufReader;
#[cfg(feature = "nix-cli-fallback")]
use tokio::process::Child;
#[cfg(feature = "nix-cli-fallback")]
use tokio::process::Command;
use tokio::sync::mpsc;
#[cfg(feature = "nix-cli-fallback")]
use tokio::task::JoinHandle;
#[cfg(feature = "nix-cli-fallback")]
use tracing::debug;
use tracing::info;
#[cfg(any(feature = "snix-build", feature = "nix-cache-proxy"))]
use tracing::warn;

#[cfg(feature = "nix-cli-fallback")]
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
    /// Whether outputs were already uploaded to SNIX PathInfoService
    /// by the native build path. When true, the worker skips the
    /// disk-based `upload_store_paths_snix` call (the output paths
    /// live in a temporary bwrap scratch dir that's already cleaned up).
    #[cfg_attr(not(feature = "snix"), allow(dead_code))]
    pub(crate) native_uploaded: bool,
}

/// Handles for concurrent stdout/stderr reader tasks.
#[cfg(feature = "nix-cli-fallback")]
struct OutputReaders {
    stdout_task: JoinHandle<std::result::Result<(), CiCoreError>>,
    stderr_task: JoinHandle<std::result::Result<(), CiCoreError>>,
    stdout_rx: mpsc::Receiver<String>,
    stderr_rx: mpsc::Receiver<String>,
}

/// Detected project type based on directory contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    /// Has `npins/sources.json` — can use zero-subprocess eval via snix-eval.
    Npins,
    /// Has `flake.nix` — uses flake evaluation (in-process or subprocess).
    Flake,
    /// Neither npins nor flake markers found.
    Unknown,
}

/// Detect project type by checking for marker files.
///
/// Checks `npins/sources.json` first (npins takes priority since it enables
/// the fully-native zero-subprocess path), then `flake.nix`.
pub fn detect_project_type(project_dir: &str) -> ProjectType {
    let base = std::path::Path::new(project_dir);
    if base.join("npins/sources.json").exists() {
        ProjectType::Npins
    } else if base.join("flake.nix").exists() {
        ProjectType::Flake
    } else {
        ProjectType::Unknown
    }
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
    /// In-process Nix evaluator for npins and flake evaluation.
    #[cfg(feature = "snix-eval")]
    pub(crate) evaluator: Option<crate::eval::NixEvaluator>,
}

impl NixBuildWorker {
    /// Create a new Nix build worker with the given configuration.
    pub fn new(config: NixBuildWorkerConfig) -> Self {
        Self {
            config,
            #[cfg(feature = "snix-build")]
            native_build_service: None,
            #[cfg(feature = "snix-eval")]
            evaluator: None,
        }
    }

    /// Create a worker with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(NixBuildWorkerConfig::default())
    }

    /// Initialize the in-process evaluator from the worker's snix services.
    ///
    /// Call after construction to enable zero-subprocess npins eval and
    /// in-process flake eval. If snix services are missing, the worker
    /// falls back to `nix eval` subprocess.
    #[cfg(feature = "snix-eval")]
    pub fn init_evaluator(&mut self) {
        if let (Some(bs), Some(ds), Some(ps)) = (
            &self.config.snix_blob_service,
            &self.config.snix_directory_service,
            &self.config.snix_pathinfo_service,
        ) {
            let eval = crate::eval::NixEvaluator::new(bs.clone(), ds.clone(), ps.clone())
                .with_nix_binary(self.config.nix_binary.clone());
            self.evaluator = Some(eval);
            info!("in-process NixEvaluator initialized");
        } else {
            info!("snix services not configured, NixEvaluator not available");
        }
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
    #[cfg(all(feature = "snix-build", feature = "nix-cli-fallback"))]
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

        // Timeout: 120 seconds for eval. First eval in a fresh store may need
        // to fetch nixpkgs (~50MB download) before it can resolve the .drvPath.
        let timeout = Duration::from_secs(120);
        let output = tokio::time::timeout(timeout, child.wait_with_output())
            .await
            .map_err(|_| CiCoreError::NixBuildFailed {
                flake: flake_ref.to_string(),
                reason: "nix eval --raw timed out after 120s resolving .drvPath".to_string(),
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
    /// Tries to resolve the derivation fully in-process via call-flake.nix + snix-eval
    /// (zero subprocesses). Falls back to `nix eval --raw .drvPath` subprocess if
    /// in-process eval fails (missing inputs, IFD, unsupported builtins).
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

        // Step 1: Try in-process flake eval via call-flake.nix + snix-eval.
        // Falls back to `nix eval` subprocess if in-process eval fails.
        let drv = match self.try_flake_eval_native(payload, flake_ref, &log_sender).await {
            Ok(drv) => drv,
            Err(eval_err) => {
                warn!(
                    error = %eval_err,
                    flake_ref = %flake_ref,
                    "in-process flake eval failed, falling back to nix eval subprocess"
                );
                if let Some(ref tx) = log_sender {
                    let _ = tx.send(format!("in-process eval failed ({eval_err}), using nix eval subprocess\n")).await;
                }
                // Fallback: resolve via subprocess, read .drv from disk
                #[cfg(feature = "nix-cli-fallback")]
                {
                    self.resolve_drv_and_parse(payload, flake_ref).await?
                }
                #[cfg(not(feature = "nix-cli-fallback"))]
                {
                    return Err(eval_err);
                }
            }
        };

        info!(
            output_count = drv.outputs.len(),
            system = %drv.system,
            "parsed derivation, starting native build"
        );

        if let Some(ref tx) = log_sender {
            let _ = tx.send(format!("native build: starting build for {flake_ref}\n")).await;
        }

        // Step 2: Ensure the build's input closure exists in local /nix/store.
        self.ensure_input_closure(&drv, payload, flake_ref, &log_sender).await?;

        // Step 2.5: Pre-build input verification — check all required paths exist
        // on disk before starting the sandbox. Catches missing inputs early with
        // a clear error listing every missing path, instead of opaque bwrap failures.
        if let Err(e) = crate::build_service::verify_inputs_present(&drv) {
            warn!(error = %e, "pre-build input verification failed");
            if let Some(ref tx) = log_sender {
                let _ = tx.send(format!("pre-build verification failed: {e}\n")).await;
            }
            return Err(CiCoreError::NixBuildFailed {
                flake: flake_ref.to_string(),
                reason: format!("pre-build input verification failed: {e}"),
            });
        }

        // Step 3+4: Build, upload, and package output.
        self.run_and_package_native_build(service, &drv, flake_ref, log_sender).await
    }

    #[cfg(feature = "snix-build")]
    async fn run_and_package_native_build(
        &self,
        service: &crate::build_service::NativeBuildService,
        drv: &nix_compat::derivation::Derivation,
        flake_ref: &str,
        log_sender: Option<tokio::sync::mpsc::Sender<String>>,
    ) -> std::result::Result<NixBuildOutput, aspen_ci_core::CiCoreError> {
        let build_result =
            service.build_derivation(drv, log_sender.clone()).await.map_err(|e| CiCoreError::NixBuildFailed {
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
            native_uploaded: true,
        })
    }

    /// Try in-process flake evaluation via embedded flake-compat + snix-eval.
    ///
    /// Uses NixOS/flake-compat to resolve inputs via snix-eval's native
    /// `fetchTarball`/`builtins.path` builtins. Falls back to the legacy
    /// call-flake.nix path, then to `nix eval` subprocess.
    #[cfg(feature = "snix-build")]
    async fn try_flake_eval_native(
        &self,
        payload: &NixBuildPayload,
        flake_ref: &str,
        log_sender: &Option<mpsc::Sender<String>>,
    ) -> std::result::Result<Derivation, CiCoreError> {
        let flake_dir = payload
            .working_dir
            .as_deref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| payload.flake_url.clone());
        let attribute = &payload.attribute;

        let evaluator = self.evaluator.as_ref().ok_or_else(|| CiCoreError::NixBuildFailed {
            flake: flake_ref.to_string(),
            reason: "NixEvaluator not initialized for flake eval".to_string(),
        })?;

        // Primary path: flake-compat — snix-eval handles all input fetching
        // (no flake.lock needed — flake-compat resolves inputs internally)
        if let Some(tx) = log_sender {
            let _ = tx.send("attempting in-process flake eval via flake-compat\n".to_string()).await;
        }

        let eval_clone = evaluator.clone();
        let dir = flake_dir.to_string();
        let attr = attribute.clone();
        let system = payload.system.clone();

        match tokio::task::spawn_blocking(move || eval_clone.evaluate_flake_via_compat(&dir, &attr, system.as_deref()))
            .await
        {
            Ok(Ok((_store_path, drv))) => {
                info!(
                    flake_ref = %flake_ref,
                    "flake native build completed (zero subprocesses)"
                );
                if let Some(tx) = log_sender {
                    let _ = tx.send("flake-compat eval succeeded (zero subprocesses)\n".to_string()).await;
                }
                return Ok(drv);
            }
            Ok(Err(e)) => {
                warn!(
                    error = %e,
                    flake_ref = %flake_ref,
                    "flake-compat eval failed, trying legacy call-flake.nix path"
                );
                if let Some(tx) = log_sender {
                    let _ = tx.send(format!("flake-compat eval failed ({e}), trying legacy path\n")).await;
                }
            }
            Err(e) => {
                warn!(
                    error = %e,
                    flake_ref = %flake_ref,
                    "flake-compat eval task panicked, trying legacy path"
                );
            }
        }

        // Fallback: legacy call-flake.nix + manual input resolution.
        // This path requires flake.lock — generate it now if missing.
        // Deferred to here so the primary flake-compat path (above) never
        // spawns a subprocess.
        let flake_path = std::path::Path::new(&flake_dir);
        if crate::eval::needs_flake_lock(flake_path) {
            if let Some(tx) = log_sender {
                let _ = tx.send("generating flake.lock for legacy eval path\n".to_string()).await;
            }
            match crate::eval::ensure_flake_lock(flake_path) {
                Ok(true) => {
                    if let Some(tx) = log_sender {
                        let _ = tx.send("flake.lock generated successfully\n".to_string()).await;
                    }
                }
                Ok(false) => {}
                Err(e) => {
                    return Err(CiCoreError::NixBuildFailed {
                        flake: flake_ref.to_string(),
                        reason: format!("flake-compat eval failed and flake.lock generation failed: {e}"),
                    });
                }
            }
        }

        let eval_clone = evaluator.clone();
        let dir = flake_dir.to_string();
        let attr = attribute.clone();
        let flake_ref_owned = flake_ref.to_string();

        let (_store_path, drv) = tokio::task::spawn_blocking(move || eval_clone.evaluate_flake_derivation(&dir, &attr))
            .await
            .map_err(|e| CiCoreError::NixBuildFailed {
                flake: flake_ref_owned.clone(),
                reason: format!("eval task panicked: {e}"),
            })?
            .map_err(|e| CiCoreError::NixBuildFailed {
                flake: flake_ref_owned,
                reason: format!("in-process flake eval failed: {e}"),
            })?;

        Ok(drv)
    }

    /// Fallback: resolve .drv path via `nix eval` subprocess, then read and parse it.
    #[cfg(all(feature = "snix-build", feature = "nix-cli-fallback"))]
    async fn resolve_drv_and_parse(
        &self,
        payload: &NixBuildPayload,
        flake_ref: &str,
    ) -> std::result::Result<Derivation, CiCoreError> {
        let drv_path = self.resolve_drv_path(payload, flake_ref).await?;

        let drv_bytes = tokio::fs::read(&drv_path).await.map_err(|e| CiCoreError::NixBuildFailed {
            flake: flake_ref.to_string(),
            reason: format!("failed to read {}: {e}", drv_path.display()),
        })?;

        let (drv, _output_paths) =
            crate::eval::parse_derivation(&drv_bytes).map_err(|e| CiCoreError::NixBuildFailed {
                flake: flake_ref.to_string(),
                reason: format!("failed to parse {}: {e}", drv_path.display()),
            })?;

        Ok(drv)
    }

    /// Ensure all store paths required by `drv` are present in the local /nix/store.
    ///
    /// Tries three strategies in order:
    /// 1. Castore materialization (PathInfoService + BlobService + DirectoryService).
    /// 2. Upstream binary cache fetch (narinfo + NAR from cache.nixos.org).
    /// 3. `nix-store --realise` subprocess (only when `nix-cli-fallback` is enabled).
    ///
    /// Returns `Ok(())` once all inputs are available, or an error if they could
    /// not be resolved and no subprocess fallback is configured.
    #[cfg(feature = "snix-build")]
    async fn ensure_input_closure(
        &self,
        drv: &Derivation,
        _payload: &NixBuildPayload,
        flake_ref: &str,
        log_sender: &Option<mpsc::Sender<String>>,
    ) -> std::result::Result<(), CiCoreError> {
        let required_paths = collect_required_store_paths(drv);
        let missing_paths = find_missing_store_paths(&required_paths);

        if missing_paths.is_empty() {
            info!(total = required_paths.len(), "all input store paths already present, skipping materialization");
            if let Some(tx) = log_sender.as_ref() {
                let _ = tx
                    .send(format!("all {} input store paths present (zero subprocesses)\n", required_paths.len()))
                    .await;
            }
            return Ok(());
        }

        let realise_start = Instant::now();
        let mut still_missing = missing_paths.clone();

        // Phase A: castore materialization
        if let (Some(pathinfo_svc), Some(blob_svc), Some(dir_svc)) = (
            &self.config.snix_pathinfo_service,
            &self.config.snix_blob_service,
            &self.config.snix_directory_service,
        ) {
            info!(
                missing = still_missing.len(),
                total = required_paths.len(),
                "materializing missing store paths from castore"
            );
            if let Some(tx) = log_sender.as_ref() {
                let _ = tx
                    .send(format!("materializing {} missing input store paths from castore\n", still_missing.len()))
                    .await;
            }

            match crate::materialize::materialize_store_paths(
                &still_missing,
                pathinfo_svc.as_ref(),
                blob_svc.as_ref(),
                dir_svc.as_ref(),
            )
            .await
            {
                Ok(report) => {
                    info!(
                        materialized = report.materialized,
                        skipped = report.skipped,
                        unresolved = report.unresolved.len(),
                        content_errors = report.content_errors.len(),
                        elapsed_ms = report.elapsed_ms,
                        "castore materialization complete"
                    );
                    if let Some(tx) = log_sender.as_ref() {
                        let _ = tx
                            .send(format!(
                                "castore materialization: {} materialized, {} skipped, {} unresolved in {}ms\n",
                                report.materialized,
                                report.skipped,
                                report.unresolved.len(),
                                report.elapsed_ms,
                            ))
                            .await;
                    }
                    still_missing = report.unresolved.clone();
                    still_missing.extend(
                        report.content_errors.iter().filter_map(|e| e.split(':').next().map(|s| s.to_string())),
                    );
                    still_missing.retain(|p| !std::path::Path::new(p.as_str()).exists());
                }
                Err(e) => {
                    warn!(error = %e, "castore materialization failed, paths remain unresolved");
                }
            }
        }

        if still_missing.is_empty() {
            let realise_ms = realise_start.elapsed().as_millis();
            info!(realise_ms = realise_ms, "all missing paths materialized from castore");
            if let Some(tx) = log_sender.as_ref() {
                let _ = tx.send(format!("all paths materialized from castore in {realise_ms}ms\n")).await;
            }
            return Ok(());
        }

        // Phase B: upstream binary cache
        #[cfg(feature = "snix-build")]
        if let (Some(config_cache), Some(bs), Some(ds), Some(ps)) = (
            &self.config.upstream_cache_config,
            &self.config.snix_blob_service,
            &self.config.snix_directory_service,
            &self.config.snix_pathinfo_service,
        ) {
            info!(missing = still_missing.len(), "trying upstream cache for remaining store paths");
            if let Some(tx) = log_sender.as_ref() {
                let _ = tx.send(format!("fetching {} paths from upstream cache\n", still_missing.len())).await;
            }

            let basenames: Vec<String> =
                still_missing.iter().filter_map(|p| p.strip_prefix("/nix/store/").map(|s| s.to_string())).collect();

            if let Ok(client) = crate::upstream_cache::UpstreamCacheClient::new(
                config_cache.clone(),
                std::sync::Arc::clone(bs),
                std::sync::Arc::clone(ds),
                std::sync::Arc::clone(ps),
            ) {
                match client.populate_closure(&basenames).await {
                    Ok(report) => {
                        info!(
                            fetched = report.fetched,
                            already_present = report.already_present,
                            unresolved = report.unresolved.len(),
                            "upstream cache population complete"
                        );
                        if let Some(tx) = log_sender.as_ref() {
                            let _ = tx
                                .send(format!(
                                    "upstream cache: {} fetched, {} already present, {} unresolved\n",
                                    report.fetched,
                                    report.already_present,
                                    report.unresolved.len()
                                ))
                                .await;
                        }

                        if report.fetched > 0 {
                            match crate::materialize::materialize_store_paths(
                                &still_missing,
                                ps.as_ref(),
                                bs.as_ref(),
                                ds.as_ref(),
                            )
                            .await
                            {
                                Ok(retry_report) => {
                                    still_missing = retry_report.unresolved.clone();
                                    still_missing.retain(|p| !std::path::Path::new(p.as_str()).exists());
                                }
                                Err(e) => {
                                    warn!(error = %e, "retry materialization after upstream fetch failed");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "upstream cache population failed");
                    }
                }
            }
        }

        if still_missing.is_empty() {
            let realise_ms = realise_start.elapsed().as_millis();
            info!(realise_ms = realise_ms, "all paths resolved via upstream cache + castore");
            if let Some(tx) = log_sender.as_ref() {
                let _ = tx.send(format!("all paths resolved via upstream cache in {realise_ms}ms\n")).await;
            }
            return Ok(());
        }

        // Phase C: subprocess last resort (nix-store --realise)
        #[cfg(feature = "nix-cli-fallback")]
        {
            info!(
                missing = still_missing.len(),
                "castore could not resolve all paths, falling back to nix-store --realise"
            );
            if let Some(tx) = log_sender.as_ref() {
                let _ = tx
                    .send(format!("fetching {} remaining store paths via nix-store --realise\n", still_missing.len()))
                    .await;
            }

            // First try realising the input derivations (builds their outputs)
            let input_drv_paths: Vec<String> = drv.input_derivations.keys().map(|p| p.to_absolute_path()).collect();
            if !input_drv_paths.is_empty() {
                let mut cmd = Command::new("nix-store");
                cmd.arg("--realise");
                for drv_path in &input_drv_paths {
                    cmd.arg(drv_path);
                }
                cmd.arg("--option").arg("substitute").arg("true");
                if let Some(ref dir) = _payload.working_dir {
                    cmd.current_dir(dir);
                }
                cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

                let realise_timeout = Duration::from_secs(_payload.timeout_secs.saturating_sub(60).max(120));
                let output = tokio::time::timeout(realise_timeout, cmd.output())
                    .await
                    .map_err(|_| CiCoreError::NixBuildFailed {
                        flake: flake_ref.to_string(),
                        reason: format!("realising input derivations timed out after {}s", realise_timeout.as_secs()),
                    })?
                    .map_err(|e| CiCoreError::NixBuildFailed {
                        flake: flake_ref.to_string(),
                        reason: format!("failed to spawn nix-store --realise: {e}"),
                    })?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(CiCoreError::NixBuildFailed {
                        flake: flake_ref.to_string(),
                        reason: format!(
                            "realising input derivations failed (exit {}): {}",
                            output.status.code().unwrap_or(-1),
                            stderr.chars().take(500).collect::<String>()
                        ),
                    });
                }
            }

            // Then fetch any remaining missing store paths (builder, sources)
            let final_missing: Vec<&String> =
                still_missing.iter().filter(|p| !std::path::Path::new(p.as_str()).exists()).collect();

            if !final_missing.is_empty() {
                info!(
                    missing = final_missing.len(),
                    paths = ?final_missing,
                    "fetching remaining missing paths via nix-store --realise"
                );
                let mut cmd = Command::new("nix-store");
                cmd.arg("--realise");
                for path in &final_missing {
                    cmd.arg(path.as_str());
                }
                cmd.arg("--option").arg("substitute").arg("true");
                cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

                let fetch_timeout = Duration::from_secs(120);
                let output = tokio::time::timeout(fetch_timeout, cmd.output())
                    .await
                    .map_err(|_| CiCoreError::NixBuildFailed {
                        flake: flake_ref.to_string(),
                        reason: "fetching extra store paths timed out".to_string(),
                    })?
                    .map_err(|e| CiCoreError::NixBuildFailed {
                        flake: flake_ref.to_string(),
                        reason: format!("nix-store --realise for extra paths failed: {e}"),
                    })?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(CiCoreError::NixBuildFailed {
                        flake: flake_ref.to_string(),
                        reason: format!(
                            "fetching extra store paths failed (exit {}): {}",
                            output.status.code().unwrap_or(-1),
                            stderr.chars().take(500).collect::<String>()
                        ),
                    });
                }
            }

            let realise_ms = realise_start.elapsed().as_millis();
            info!(realise_ms = realise_ms, "input closure realised");
            if let Some(tx) = log_sender.as_ref() {
                let _ = tx.send(format!("input closure realised in {realise_ms}ms\n")).await;
            }
            Ok(())
        }

        #[cfg(not(feature = "nix-cli-fallback"))]
        {
            Err(CiCoreError::NixBuildFailed {
                flake: flake_ref.to_string(),
                reason: format!(
                    "castore could not resolve {} store paths and nix-cli-fallback feature is disabled: {:?}",
                    still_missing.len(),
                    still_missing.iter().take(5).collect::<Vec<_>>()
                ),
            })
        }
    }

    /// Attempt a fully-native build for an npins project — zero subprocesses.
    ///
    /// Uses `NixEvaluator::evaluate_npins_drv_path()` to resolve the .drv path
    /// via snix-eval, then follows the same build path as `try_native_build()`.
    #[cfg(feature = "snix-build")]
    pub(crate) async fn try_npins_native_build(
        &self,
        payload: &NixBuildPayload,
        log_sender: Option<mpsc::Sender<String>>,
    ) -> std::result::Result<NixBuildOutput, CiCoreError> {
        let service = self.native_build_service.as_ref().ok_or_else(|| CiCoreError::NixBuildFailed {
            flake: payload.flake_url.clone(),
            reason: "native build service not initialized".to_string(),
        })?;

        let project_dir = payload.flake_url.clone();
        let attribute = payload.attribute.clone();

        // Use the pre-initialized evaluator (avoids re-creating per build).
        let evaluator = self.evaluator.as_ref().ok_or_else(|| CiCoreError::NixBuildFailed {
            flake: project_dir.clone(),
            reason: "NixEvaluator not initialized for npins eval".to_string(),
        })?;

        // Step 1: Evaluate to Derivation via snix-eval (no subprocess, no disk I/O).
        // The Derivation is extracted from KnownPaths in-memory — no .drv file needed.
        if let Some(ref tx) = log_sender {
            let _ = tx.send("npins native eval: resolving derivation via snix-eval\n".to_string()).await;
        }

        let dir = project_dir.clone();
        let attr = attribute.clone();
        let eval_clone = evaluator.clone();
        let (_drv_store_path, drv) =
            tokio::task::spawn_blocking(move || eval_clone.evaluate_npins_derivation(&dir, "default.nix", &attr))
                .await
                .map_err(|e| CiCoreError::NixBuildFailed {
                    flake: project_dir.clone(),
                    reason: format!("eval task panicked: {e}"),
                })?
                .map_err(|e| CiCoreError::NixBuildFailed {
                    flake: project_dir.clone(),
                    reason: format!("npins eval failed: {e}"),
                })?;

        let drv_path_str = _drv_store_path.to_absolute_path();
        info!(
            drv_path = %drv_path_str,
            output_count = drv.outputs.len(),
            system = %drv.system,
            "npins eval resolved derivation (zero subprocesses)"
        );

        if let Some(ref tx) = log_sender {
            let _ = tx.send(format!("npins native eval: {} → {}\n", project_dir, drv_path_str)).await;
        }

        // Step 2.5: Pre-build input verification
        if let Err(e) = crate::build_service::verify_inputs_present(&drv) {
            warn!(error = %e, "pre-build input verification failed (npins)");
            if let Some(ref tx) = log_sender {
                let _ = tx.send(format!("pre-build verification failed: {e}\n")).await;
            }
            return Err(CiCoreError::NixBuildFailed {
                flake: project_dir.clone(),
                reason: format!("pre-build input verification failed: {e}"),
            });
        }

        // Step 3: Execute native build
        let build_result =
            service.build_derivation(&drv, log_sender.clone()).await.map_err(|e| CiCoreError::NixBuildFailed {
                flake: project_dir.clone(),
                reason: format!("native build failed: {e}"),
            })?;

        let output_paths: Vec<String> = build_result.outputs.iter().map(|o| o.store_path.to_absolute_path()).collect();

        info!(
            output_paths = ?output_paths,
            resolve_ms = build_result.resolve_ms,
            build_ms = build_result.build_ms,
            "npins native build succeeded (zero subprocesses)"
        );

        // Step 4: Upload outputs to PathInfoService
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
                info!(count = uploaded.len(), "uploaded npins native build outputs to PathInfoService");
            }
        }

        let log = format!(
            "npins native build completed (zero subprocesses): {} outputs, resolve={}ms build={}ms\n",
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
            native_uploaded: true,
        })
    }

    /// Execute a Nix build.
    ///
    /// If `log_sender` is provided, stderr lines are also forwarded to it
    /// in real-time for streaming to the CI log infrastructure.
    #[allow(unused_variables)]
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

        // Phase 2: Build — priority: npins native, flake native, subprocess fallback.
        #[cfg(feature = "snix-build")]
        if let Some(native_output) =
            self.dispatch_native_builds(payload, &flake_ref, &mut timings, log_sender.clone()).await
        {
            return Ok(self
                .finalize_native_build(
                    native_output,
                    &mut timings,
                    #[cfg(feature = "nix-cache-proxy")]
                    cache_proxy,
                )
                .await);
        }

        #[cfg(feature = "nix-cli-fallback")]
        return self
            .run_subprocess_build(
                payload,
                &flake_ref,
                &mut timings,
                log_sender,
                #[cfg(feature = "nix-cache-proxy")]
                cache_proxy,
            )
            .await;

        #[cfg(not(feature = "nix-cli-fallback"))]
        {
            Err(CiCoreError::NixBuildFailed {
                flake: flake_ref.clone(),
                reason: "native build not available and nix-cli-fallback feature is disabled".to_string(),
            })
        }
    }

    #[cfg(feature = "snix-build")]
    async fn finalize_native_build(
        &self,
        native_output: NixBuildOutput,
        timings: &mut BuildPhaseTimings,
        #[cfg(feature = "nix-cache-proxy")] cache_proxy: Option<CacheProxy>,
    ) -> NixBuildOutput {
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
        NixBuildOutput {
            output_paths: native_output.output_paths,
            log: native_output.log,
            log_truncated: native_output.log_truncated,
            timings: std::mem::take(timings),
            native_uploaded: native_output.native_uploaded,
        }
    }

    #[cfg(feature = "snix-build")]
    async fn dispatch_native_builds(
        &self,
        payload: &NixBuildPayload,
        flake_ref: &str,
        timings: &mut BuildPhaseTimings,
        log_sender: Option<mpsc::Sender<String>>,
    ) -> Option<NixBuildOutput> {
        if self.has_native_builds() && detect_project_type(&payload.flake_url) == ProjectType::Npins {
            // npins project: try fully-native path (zero subprocesses)
            let build_start = Instant::now();
            match self.try_npins_native_build(payload, log_sender.clone()).await {
                Ok(output) => {
                    timings.record_build(build_start.elapsed());
                    return Some(output);
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        flake_ref = %flake_ref,
                        "npins native build failed, trying flake native path"
                    );
                    if let Some(ref tx) = log_sender {
                        let _ = tx.send(format!("npins eval failed ({e}), trying nix eval subprocess\n")).await;
                    }
                }
            }
            // Fall through to try_native_build (uses nix eval subprocess)
            let build_start = Instant::now();
            match self.try_native_build(payload, flake_ref, log_sender.clone()).await {
                Ok(output) => {
                    timings.record_build(build_start.elapsed());
                    Some(output)
                }
                Err(e) => {
                    warn!(error = %e, "flake native build also failed, falling back to subprocess");
                    if let Some(ref tx) = log_sender {
                        let _ = tx.send(format!("native build failed ({e}), falling back to nix build\n")).await;
                    }
                    None
                }
            }
        } else if self.has_native_builds() {
            // Flake project: try native build (nix eval subprocess + bwrap)
            let build_start = Instant::now();
            match self.try_native_build(payload, flake_ref, log_sender.clone()).await {
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
        }
    }

    #[cfg(feature = "nix-cli-fallback")]
    async fn run_subprocess_build(
        &self,
        payload: &NixBuildPayload,
        flake_ref: &str,
        timings: &mut BuildPhaseTimings,
        log_sender: Option<mpsc::Sender<String>>,
        #[cfg(feature = "nix-cache-proxy")] cache_proxy: Option<CacheProxy>,
    ) -> Result<NixBuildOutput> {
        let build_start_sub = Instant::now();

        let mut child = self.spawn_nix_build(payload, flake_ref)?;
        let readers = Self::spawn_output_readers(&mut child, flake_ref, self.config.is_verbose, log_sender)?;
        let (output_paths, log, log_size) = Self::collect_output(readers).await?;
        Self::wait_for_build_completion(&mut child, payload.timeout_secs, flake_ref, &log).await?;

        timings.record_build(build_start_sub.elapsed());

        let upload_start = Instant::now();
        #[cfg(feature = "nix-cache-proxy")]
        if let Some(proxy) = cache_proxy {
            debug!(flake_ref = %flake_ref, "Shutting down cache proxy after subprocess build");
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
            timings: std::mem::take(timings),
            native_uploaded: false,
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
    // SUBPROCESS-ESCAPE: This is the full `nix build` subprocess fallback path.
    #[cfg(feature = "nix-cli-fallback")]
    fn spawn_nix_build(&self, payload: &NixBuildPayload, flake_ref: &str) -> Result<Child> {
        let mut cmd = Command::new(&self.config.nix_binary);
        // Use --no-link instead of --out-link to avoid creating a "result"
        // symlink in cwd. Under ProtectSystem=strict the working directory
        // is often read-only (/). We parse output paths from --print-out-paths.
        cmd.arg("build").arg(flake_ref).arg("--no-link").arg("--print-out-paths");

        // Log the working directory and command for diagnosis
        info!(
            nix_binary = %self.config.nix_binary,
            flake_ref = %flake_ref,
            working_dir = ?payload.working_dir,
            sandbox = payload.sandbox,
            timeout_secs = payload.timeout_secs,
            "Spawning nix build subprocess"
        );

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

        // Log the resolved working directory for debugging Forge checkout issues
        if let Some(ref dir) = payload.working_dir {
            let has_flake = dir.join("flake.nix").exists();
            let has_lock = dir.join("flake.lock").exists();
            let has_git = dir.join(".git").exists();
            info!(
                working_dir = %dir.display(),
                has_flake_nix = has_flake,
                has_flake_lock = has_lock,
                has_dot_git = has_git,
                "CI working directory state before nix build"
            );
        }

        cmd.spawn().map_err(|e| CiCoreError::NixBuildFailed {
            flake: flake_ref.to_string(),
            reason: format!("Failed to spawn nix: {e}"),
        })
    }

    /// Configure cache substituter arguments on the nix build command.
    #[cfg(all(feature = "nix-cache-proxy", feature = "nix-cli-fallback"))]
    #[expect(
        dead_code,
        reason = "reserved for future use when cache proxy is integrated with subprocess path"
    )]
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
    #[cfg(feature = "nix-cli-fallback")]
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
    #[cfg(feature = "nix-cli-fallback")]
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
    #[cfg(feature = "nix-cli-fallback")]
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── detect_project_type ──────────────────────────────────────────────────

    #[test]
    fn test_detect_project_type_empty_dir_is_unknown() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(detect_project_type(dir.path().to_str().unwrap()), ProjectType::Unknown);
    }

    #[test]
    fn test_detect_project_type_flake_nix() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("flake.nix"), "{}").unwrap();
        assert_eq!(detect_project_type(dir.path().to_str().unwrap()), ProjectType::Flake);
    }

    #[test]
    fn test_detect_project_type_npins() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("npins")).unwrap();
        std::fs::write(dir.path().join("npins/sources.json"), "{}").unwrap();
        assert_eq!(detect_project_type(dir.path().to_str().unwrap()), ProjectType::Npins);
    }

    #[test]
    fn test_detect_project_type_npins_beats_flake() {
        // npins takes priority even when flake.nix is also present
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("npins")).unwrap();
        std::fs::write(dir.path().join("npins/sources.json"), "{}").unwrap();
        std::fs::write(dir.path().join("flake.nix"), "{}").unwrap();
        assert_eq!(detect_project_type(dir.path().to_str().unwrap()), ProjectType::Npins);
    }

    #[test]
    fn test_detect_project_type_npins_dir_without_sources_json_is_unknown() {
        // npins/ dir alone, without sources.json, does not trigger Npins
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("npins")).unwrap();
        assert_eq!(detect_project_type(dir.path().to_str().unwrap()), ProjectType::Unknown);
    }

    // ── find_missing_store_paths ─────────────────────────────────────────────

    #[test]
    fn test_find_missing_store_paths_empty_input() {
        assert!(find_missing_store_paths(&[]).is_empty());
    }

    #[test]
    fn test_find_missing_store_paths_nonexistent_returns_store_root() {
        // A store path whose store root does not exist should be returned
        let paths = vec!["/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-test/lib/foo.so".to_string()];
        let missing = find_missing_store_paths(&paths);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], "/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-test");
    }

    #[test]
    fn test_find_missing_store_paths_existing_path_omitted() {
        // Use a directory that is guaranteed to exist
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().to_str().unwrap();
        // Build a fake "store path" with enough components to satisfy the splitter
        // Path format: / seg1 / seg2 / seg3 / rest — splitn(5, '/') gives 5 parts.
        // We need components[..4].join("/") to be a real path.
        let fake = format!("{}/a/b/c", p);
        std::fs::create_dir_all(&fake).unwrap();
        // components[..4] covers the first 4 segments; since p itself has multiple
        // components we build a path that when split at depth-4 gives an existing dir.
        // Simpler: just check that the tmpdir itself is not reported missing.
        let components: Vec<&str> = p.splitn(5, '/').collect();
        if components.len() >= 4 {
            let root = components[..4].join("/");
            let paths = vec![format!("{}/extra", root)];
            let missing = find_missing_store_paths(&paths);
            // root exists → should NOT be in missing
            assert!(missing.is_empty(), "existing root should not be listed as missing: {root}");
        }
    }

    #[test]
    fn test_find_missing_store_paths_short_path_ignored() {
        // Paths with fewer than 4 components after splitting cannot name a store root
        let paths = vec!["/nix/short".to_string()];
        let missing = find_missing_store_paths(&paths);
        assert!(missing.is_empty(), "too-short paths should be silently skipped");
    }
}

/// Read stdout lines and send any `/nix/store/` paths through the channel.
#[cfg(feature = "nix-cli-fallback")]
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

/// Collect all store paths required by a derivation: outputs of input derivations,
/// input sources, and the builder itself (when it lives in /nix/store).
///
/// These paths must exist locally before the build sandbox is launched.
#[cfg(feature = "snix-build")]
fn collect_required_store_paths(drv: &Derivation) -> Vec<String> {
    let mut required: Vec<String> = Vec::new();
    for (drv_path, output_names) in &drv.input_derivations {
        let drv_abs = drv_path.to_absolute_path();
        if let Ok(bytes) = std::fs::read(&drv_abs)
            && let Ok(input_drv) = Derivation::from_aterm_bytes(&bytes)
        {
            for output_name in output_names {
                if let Some(output) = input_drv.outputs.get(output_name)
                    && let Some(path) = &output.path
                {
                    required.push(path.to_absolute_path());
                }
            }
        }
    }
    if drv.builder.starts_with("/nix/store/") {
        required.push(drv.builder.clone());
    }
    for source in &drv.input_sources {
        required.push(source.to_absolute_path());
    }
    required
}

/// Return the subset of `required` store paths whose top-level store entry
/// (`/nix/store/<hash>-<name>`) does not yet exist on disk.
// Used by ensure_input_closure (cfg snix-build) and tests; suppress dead_code
// warning in default-feature builds that don't enable snix-build.
#[cfg_attr(not(any(feature = "snix-build", test)), allow(dead_code))]
fn find_missing_store_paths(required: &[String]) -> Vec<String> {
    required
        .iter()
        .filter_map(|p| {
            let components: Vec<&str> = p.splitn(5, '/').collect();
            if components.len() >= 4 {
                let store_root = components[..4].join("/");
                if !std::path::Path::new(&store_root).exists() {
                    return Some(store_root);
                }
            }
            None
        })
        .collect()
}

/// Read stderr lines as build log output, optionally logging each line.
/// When `log_sender` is provided, lines are also forwarded for real-time streaming.
#[cfg(feature = "nix-cli-fallback")]
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
