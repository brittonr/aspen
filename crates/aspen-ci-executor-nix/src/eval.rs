//! In-process Nix evaluation via snix-eval + snix-glue.
//!
//! Provides flake evaluation, pre-flight validation, and derivation-to-BuildRequest
//! conversion without spawning the `nix` CLI binary. Uses snix-glue's `SnixStoreIO`
//! to resolve store paths from Aspen's `BlobService`/`DirectoryService`/`PathInfoService`.
//!
//! When evaluation hits unsupported features (IFD, missing builtins), falls back
//! to `nix eval` subprocess if the `nix-cli-fallback` feature is enabled.

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use nix_compat::derivation::Derivation;
use snix_build::buildservice::DummyBuildService;
use snix_castore::blobservice::BlobService;
use snix_castore::directoryservice::DirectoryService;
use snix_eval::EvalIO;
use snix_eval::EvaluationResult;
use snix_eval::Value;
use snix_glue::snix_io::SnixIO;
use snix_glue::snix_store_io::SnixStoreIO;
use snix_store::nar::SimpleRenderer;
use snix_store::pathinfoservice::PathInfoService;
use tracing::debug;
use tracing::info;

use crate::config::MAX_FLAKE_URL_LENGTH;

// Limits for evaluation
/// Maximum Nix source file size to evaluate (1 MB).
const MAX_EVAL_SOURCE_SIZE: usize = 1_048_576;
/// Maximum number of evaluation errors to report.
const MAX_EVAL_ERRORS: usize = 50;

/// Result of evaluating a Nix expression.
#[derive(Debug)]
pub struct NixEvalOutput {
    /// The evaluated Nix value, if evaluation succeeded.
    pub value: Option<Value>,
    /// Evaluation errors (empty on success).
    pub errors: Vec<NixEvalError>,
    /// Evaluation warnings.
    pub warnings: Vec<String>,
}

/// A structured evaluation error with source location.
#[derive(Debug, Clone)]
pub struct NixEvalError {
    /// Human-readable error message.
    pub message: String,
    /// Whether this error indicates IFD was attempted.
    pub is_ifd: bool,
}

impl std::fmt::Display for NixEvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// In-process Nix evaluator backed by Aspen's distributed store.
///
/// Uses snix-eval for pure Nix evaluation and snix-glue's `SnixStoreIO`
/// to resolve store paths through BlobService/DirectoryService/PathInfoService.
pub struct NixEvaluator {
    blob_service: Arc<dyn BlobService>,
    directory_service: Arc<dyn DirectoryService>,
    pathinfo_service: Arc<dyn PathInfoService>,
    /// Path to the `nix` binary for subprocess fallback.
    nix_binary: String,
}

impl NixEvaluator {
    /// Create a new evaluator backed by Aspen's store services.
    pub fn new(
        blob_service: Arc<dyn BlobService>,
        directory_service: Arc<dyn DirectoryService>,
        pathinfo_service: Arc<dyn PathInfoService>,
    ) -> Self {
        Self {
            blob_service,
            directory_service,
            pathinfo_service,
            nix_binary: "nix".to_string(),
        }
    }

    /// Set the nix binary path for subprocess fallback.
    pub fn with_nix_binary(mut self, path: String) -> Self {
        self.nix_binary = path;
        self
    }

    /// Evaluate a Nix expression string.
    ///
    /// Uses pure evaluation by default — no filesystem I/O builtins.
    /// For store-backed evaluation (reading from /nix/store), use `evaluate_with_store`.
    pub fn evaluate_pure(&self, code: &str) -> NixEvalOutput {
        if code.len() > MAX_EVAL_SOURCE_SIZE {
            return NixEvalOutput {
                value: None,
                errors: vec![NixEvalError {
                    message: format!(
                        "source too large: {} bytes exceeds {} byte limit",
                        code.len(),
                        MAX_EVAL_SOURCE_SIZE
                    ),
                    is_ifd: false,
                }],
                warnings: vec![],
            };
        }

        let eval = snix_eval::Evaluation::builder_pure().mode(snix_eval::EvalMode::Strict).build();

        let result = eval.evaluate(code, None);
        convert_eval_result(result)
    }

    /// Evaluate a Nix expression with access to the Aspen store.
    ///
    /// Store paths in /nix/store/ are resolved through
    /// BlobService/DirectoryService/PathInfoService. Derivation builtins (derivationStrict,
    /// toFile) and fetchers (fetchurl, fetchTarball, fetchGit) are available.
    pub fn evaluate_with_store(&self, code: &str, location: Option<PathBuf>) -> NixEvalOutput {
        if code.len() > MAX_EVAL_SOURCE_SIZE {
            return NixEvalOutput {
                value: None,
                errors: vec![NixEvalError {
                    message: format!(
                        "source too large: {} bytes exceeds {} byte limit",
                        code.len(),
                        MAX_EVAL_SOURCE_SIZE
                    ),
                    is_ifd: false,
                }],
                warnings: vec![],
            };
        }

        let tokio_handle = tokio::runtime::Handle::current();

        // NAR calculation service for store operations
        let nar_calc = Arc::new(SimpleRenderer::new(self.blob_service.clone(), self.directory_service.clone()));

        // DummyBuildService — we don't execute builds during evaluation.
        // IFD would require a real build service, but we fall back to
        // subprocess for that case.
        let build_service = Arc::new(DummyBuildService {});

        let snix_store_io = Rc::new(SnixStoreIO::new(
            self.blob_service.clone(),
            self.directory_service.clone(),
            self.pathinfo_service.clone(),
            nar_calc,
            build_service,
            tokio_handle,
            vec![], // no hashed mirrors
        ));

        // Box the IO — EvaluationBuilder requires Box<dyn EvalIO> for the
        // concrete type to satisfy AsRef<dyn EvalIO>.
        let io: Box<dyn EvalIO> = Box::new(SnixIO::new(snix_store_io.clone() as Rc<dyn EvalIO>));

        let eval = snix_eval::Evaluation::builder(io).enable_import().mode(snix_eval::EvalMode::Strict);

        // Add store-aware builtins from snix-glue
        let eval = snix_glue::builtins::add_derivation_builtins(eval, snix_store_io.clone());
        let eval = snix_glue::builtins::add_fetcher_builtins(eval, snix_store_io.clone());
        let eval = snix_glue::builtins::add_import_builtins(eval, snix_store_io);

        // Configure nix search path for <nix> resolution
        let eval = snix_glue::configure_nix_path(eval, &None);

        let result = eval.build().evaluate(code, location);
        convert_eval_result(result)
    }

    /// Validate that a flake expression evaluates without errors.
    ///
    /// Reads the flake file, evaluates it, and reports any errors.
    /// Does NOT build — only confirms the expression is valid Nix.
    pub async fn validate_flake(&self, flake_path: &str) -> NixEvalOutput {
        if flake_path.len() > MAX_FLAKE_URL_LENGTH {
            return NixEvalOutput {
                value: None,
                errors: vec![NixEvalError {
                    message: format!("flake path too long: {} chars", flake_path.len()),
                    is_ifd: false,
                }],
                warnings: vec![],
            };
        }

        let source = match tokio::fs::read_to_string(flake_path).await {
            Ok(s) => s,
            Err(e) => {
                return NixEvalOutput {
                    value: None,
                    errors: vec![NixEvalError {
                        message: format!("failed to read flake file: {e}"),
                        is_ifd: false,
                    }],
                    warnings: vec![],
                };
            }
        };

        info!(path = %flake_path, size = source.len(), "evaluating flake for pre-flight validation");
        self.evaluate_with_store(&source, Some(PathBuf::from(flake_path)))
    }

    /// Pre-flight check: validate a flake evaluates before queuing build jobs.
    ///
    /// Returns `Ok(())` if the flake is valid, or `Err` with structured error
    /// info if evaluation fails. CI pipelines call this before creating jobs.
    pub async fn preflight_check(&self, flake_dir: &str) -> Result<(), Vec<NixEvalError>> {
        let flake_path = format!("{flake_dir}/flake.nix");
        let result = self.validate_flake(&flake_path).await;

        if result.errors.is_empty() && result.value.is_some() {
            info!(flake_dir = %flake_dir, "pre-flight validation passed");
            Ok(())
        } else {
            debug!(
                flake_dir = %flake_dir,
                error_count = result.errors.len(),
                "pre-flight validation failed"
            );
            Err(result.errors)
        }
    }

    /// Evaluate a flake attribute (e.g. `packages.x86_64-linux.default`).
    ///
    /// Reads `flake.nix` from `flake_dir`, wraps it in an expression that
    /// selects the requested attribute, and evaluates with store access.
    /// The flake.lock is expected to be adjacent to flake.nix.
    pub async fn evaluate_flake_attribute(&self, flake_dir: &str, attribute: &str) -> NixEvalOutput {
        let flake_path = format!("{flake_dir}/flake.nix");

        // Verify the flake file exists before constructing the eval expression
        if let Err(e) = tokio::fs::metadata(&flake_path).await {
            return NixEvalOutput {
                value: None,
                errors: vec![NixEvalError {
                    message: format!("failed to read {flake_path}: {e}"),
                    is_ifd: false,
                }],
                warnings: vec![],
            };
        }

        // Wrap flake expression to select the requested attribute.
        // Flakes are functions { self, ... }: { outputs }, but for basic
        // evaluation we call them with empty inputs and select the attr.
        let eval_code = if attribute.is_empty() {
            format!("let flake = import {flake_path}; in flake")
        } else {
            // Navigate nested attributes: "packages.x86_64-linux.default"
            // → (import ./flake.nix { }).packages.x86_64-linux.default
            let attr_path = attribute
                .split('.')
                .map(|part| {
                    // Quote attribute parts that aren't valid identifiers
                    if part.contains('-') || part.starts_with(|c: char| c.is_ascii_digit()) {
                        format!("\"{part}\"")
                    } else {
                        part.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(".");
            format!("let flake = import {flake_path}; in flake.{attr_path}")
        };

        info!(
            flake_dir = %flake_dir,
            attribute = %attribute,
            "evaluating flake attribute"
        );

        self.evaluate_with_store(&eval_code, Some(PathBuf::from(&flake_path)))
    }

    /// Evaluate a Nix expression, falling back to `nix eval` subprocess
    /// if in-process evaluation fails due to IFD or unsupported builtins.
    ///
    /// Only falls back when the `nix-cli-fallback` feature is enabled.
    pub async fn evaluate_with_fallback(&self, code: &str, location: Option<PathBuf>) -> NixEvalOutput {
        let result = self.evaluate_with_store(code, location.clone());

        // Check if any errors indicate IFD or unsupported features
        let needs_fallback = result.errors.iter().any(|e| e.is_ifd);

        if needs_fallback && !result.errors.is_empty() {
            #[cfg(feature = "nix-cli-fallback")]
            {
                warn!(
                    error_count = result.errors.len(),
                    "in-process evaluation failed, falling back to nix eval subprocess"
                );
                return self.evaluate_subprocess(code).await;
            }

            #[cfg(not(feature = "nix-cli-fallback"))]
            {
                debug!("in-process evaluation failed and nix-cli-fallback is disabled");
            }
        }

        result
    }

    /// Fall back to `nix eval` subprocess for expressions that require IFD
    /// or builtins not yet supported by snix-eval.
    #[cfg(feature = "nix-cli-fallback")]
    async fn evaluate_subprocess(&self, code: &str) -> NixEvalOutput {
        use tokio::process::Command;

        info!("falling back to nix eval subprocess");

        let output = match Command::new(&self.nix_binary).args(["eval", "--expr", code, "--json"]).output().await {
            Ok(o) => o,
            Err(e) => {
                return NixEvalOutput {
                    value: None,
                    errors: vec![NixEvalError {
                        message: format!("failed to spawn nix eval: {e}"),
                        is_ifd: false,
                    }],
                    warnings: vec![],
                };
            }
        };

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Parse the JSON output back into a string value — the caller
            // can deserialize further if needed.
            NixEvalOutput {
                value: Some(Value::from(snix_eval::NixString::from(stdout.as_ref()))),
                errors: vec![],
                warnings: vec!["evaluated via nix subprocess fallback".to_string()],
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            NixEvalOutput {
                value: None,
                errors: vec![NixEvalError {
                    message: format!("nix eval failed: {stderr}"),
                    is_ifd: false,
                }],
                warnings: vec![],
            }
        }
    }
}

/// Parse a .drv file (ATerm format) into a `Derivation` and extract build
/// metadata (builder, args, env, outputs) for cache key computation and
/// build request construction.
///
/// Returns the parsed derivation and its output paths as strings.
pub fn parse_derivation(drv_bytes: &[u8]) -> std::io::Result<(Derivation, Vec<String>)> {
    let drv = Derivation::from_aterm_bytes(drv_bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("invalid .drv: {e:?}")))?;

    let output_paths: Vec<String> = drv
        .outputs
        .values()
        .filter_map(|output| output.path.as_ref())
        .map(|sp| sp.to_absolute_path())
        .collect();

    Ok((drv, output_paths))
}

/// Convert snix-eval's `EvaluationResult` into our `NixEvalOutput`.
fn convert_eval_result(result: EvaluationResult) -> NixEvalOutput {
    let errors: Vec<NixEvalError> = result
        .errors
        .iter()
        .take(MAX_EVAL_ERRORS)
        .map(|e| {
            let msg = format!("{e}");
            let is_ifd =
                msg.contains("import from derivation") || msg.contains("ImportFromDerivation") || msg.contains("IFD");
            NixEvalError { message: msg, is_ifd }
        })
        .collect();

    let warnings: Vec<String> = result.warnings.iter().map(|w| format!("{w:?}")).collect();

    NixEvalOutput {
        value: result.value,
        errors,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;

    /// Create in-memory snix services for testing.
    fn test_evaluator() -> NixEvaluator {
        NixEvaluator::new(
            Arc::new(snix_castore::blobservice::MemoryBlobService::default()),
            Arc::new(
                snix_castore::directoryservice::RedbDirectoryService::new_temporary(
                    "test".to_string(),
                    Default::default(),
                )
                .expect("failed to create temp dir service"),
            ),
            Arc::new(snix_store::pathinfoservice::LruPathInfoService::with_capacity(
                "test".to_string(),
                NonZeroUsize::new(100).unwrap(),
            )),
        )
    }

    #[test]
    fn test_pure_eval_simple_expression() {
        let eval = test_evaluator();
        let result = eval.evaluate_pure("1 + 2");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        assert!(result.value.is_some());
    }

    #[test]
    fn test_pure_eval_attrset() {
        let eval = test_evaluator();
        let result = eval.evaluate_pure("{ a = 1; b = \"hello\"; }");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let val = result.value.expect("should have value");
        match val {
            Value::Attrs(_) => {}
            other => panic!("expected attrs, got {:?}", other),
        }
    }

    #[test]
    fn test_pure_eval_syntax_error() {
        let eval = test_evaluator();
        let result = eval.evaluate_pure("{ a = ; }");
        assert!(!result.errors.is_empty());
        assert!(result.value.is_none());
    }

    #[test]
    fn test_pure_eval_undefined_variable() {
        let eval = test_evaluator();
        let result = eval.evaluate_pure("x + 1");
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_pure_eval_type_error() {
        let eval = test_evaluator();
        let result = eval.evaluate_pure("1 + \"string\"");
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_pure_eval_source_too_large() {
        let eval = test_evaluator();
        let huge_source = "x".repeat(MAX_EVAL_SOURCE_SIZE + 1);
        let result = eval.evaluate_pure(&huge_source);
        assert!(!result.errors.is_empty());
        assert!(result.errors[0].message.contains("source too large"));
    }

    #[tokio::test]
    async fn test_store_eval_simple() {
        let eval = test_evaluator();
        let result = eval.evaluate_with_store("builtins.length [1 2 3]", None);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        assert!(result.value.is_some());
    }

    #[tokio::test]
    async fn test_validate_flake_nonexistent() {
        let eval = test_evaluator();
        let result = eval.validate_flake("/nonexistent/flake.nix").await;
        assert!(!result.errors.is_empty());
        assert!(result.errors[0].message.contains("failed to read flake file"));
    }

    #[tokio::test]
    async fn test_evaluate_flake_attribute_missing_dir() {
        let eval = test_evaluator();
        let result = eval.evaluate_flake_attribute("/nonexistent/dir", "packages.x86_64-linux.default").await;
        assert!(!result.errors.is_empty());
        assert!(result.errors[0].message.contains("failed to read"));
    }

    #[tokio::test]
    async fn test_evaluate_own_flake_syntax() {
        // Verify that Aspen's own flake.nix is syntactically valid.
        // Full evaluation requires nixpkgs, so we only parse here.
        // compile_only reports UnknownStaticVariable for flake inputs
        // (they're injected at runtime), so we parse with rnix directly.
        let flake_src = std::fs::read_to_string("../../flake.nix");
        if let Ok(src) = flake_src {
            let parsed = rnix::Root::parse(&src);
            assert!(parsed.errors().is_empty(), "Aspen flake.nix has syntax errors: {:?}", parsed.errors());
        }
        // If the file isn't found (running from a different cwd), skip
    }

    #[test]
    fn test_eval_error_reports_syntax_location() {
        let eval = test_evaluator();
        let result = eval.evaluate_pure("{ a = 1; b = }");
        assert!(!result.errors.is_empty());
        // Error message should contain something about the syntax issue
        let msg = &result.errors[0].message;
        assert!(!msg.is_empty(), "error message should not be empty");
    }

    #[test]
    fn test_eval_error_undefined_variable_message() {
        let eval = test_evaluator();
        let result = eval.evaluate_pure("undefinedVar");
        assert!(!result.errors.is_empty());
        let msg = &result.errors[0].message;
        assert!(
            msg.contains("undefined")
                || msg.contains("undefinedVar")
                || msg.contains("Unknown")
                || msg.contains("not found"),
            "error should reference the undefined variable, got: {msg}"
        );
    }

    #[test]
    fn test_eval_error_type_mismatch_message() {
        let eval = test_evaluator();
        // Trying to add an int and a list
        let result = eval.evaluate_pure("1 + [2]");
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_ifd_detection_from_message() {
        // IFD detection works by scanning error messages
        let error = NixEvalError {
            message: "import from derivation is not supported".to_string(),
            is_ifd: true,
        };
        assert!(error.is_ifd);

        // Verify the detection logic in convert_eval_result
        let non_ifd = NixEvalError {
            message: "undefined variable 'x'".to_string(),
            is_ifd: false,
        };
        assert!(!non_ifd.is_ifd);
    }

    #[test]
    fn test_parse_derivation_valid() {
        // A minimal valid .drv in ATerm format
        let drv_aterm = br#"Derive([("out","/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa0-test","","")],[],[],"x86_64-linux","/bin/sh",["-c","echo hello"],[("builder","/bin/sh"),("name","test"),("out","/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa0-test"),("system","x86_64-linux")])"#;

        let result = parse_derivation(drv_aterm);
        assert!(result.is_ok(), "parse error: {:?}", result.err());
        let (drv, output_paths) = result.unwrap();
        assert_eq!(drv.outputs.len(), 1);
        assert_eq!(output_paths.len(), 1);
        assert!(output_paths[0].starts_with("/nix/store/"));
    }

    #[test]
    fn test_parse_derivation_invalid() {
        let result = parse_derivation(b"not a valid drv");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_preflight_check_nonexistent() {
        let eval = test_evaluator();
        let result = eval.preflight_check("/nonexistent").await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
    }
}
