//! Nix-based CI pipeline configuration via snix-serde.
//!
//! Parses `.aspen/ci.nix` files into typed Rust structs using `snix_serde::from_str`.
//! Only pure Nix evaluation is allowed — no I/O builtins, no `builtins.readFile`,
//! no import-from-derivation.
//!
//! # Example
//!
//! A `.aspen/ci.nix` file:
//! ```nix
//! {
//!   stages = [
//!     {
//!       name = "check";
//!       jobs = [
//!         { name = "clippy"; type = "shell"; command = "cargo clippy"; }
//!       ];
//!     }
//!     {
//!       name = "build";
//!       jobs = [
//!         { name = "build-node"; type = "nix"; attribute = "packages.x86_64-linux.aspen-node"; }
//!       ];
//!     }
//!   ];
//! }
//! ```

use serde::Deserialize;
use tracing::debug;
use tracing::info;

/// Maximum size of a ci.nix file (256 KB).
const MAX_CONFIG_SIZE: usize = 256 * 1024;

/// A CI pipeline definition deserialized from Nix.
#[derive(Debug, Clone, Deserialize)]
pub struct NixPipelineConfig {
    /// Pipeline stages, executed in order.
    pub stages: Vec<NixStageConfig>,
    /// Pipeline-level timeout in seconds.
    #[serde(default = "default_pipeline_timeout")]
    pub timeout_secs: u64,
}

fn default_pipeline_timeout() -> u64 {
    3600
}

/// A pipeline stage containing one or more jobs.
#[derive(Debug, Clone, Deserialize)]
pub struct NixStageConfig {
    /// Stage name (must be unique within the pipeline).
    pub name: String,
    /// Jobs in this stage. Jobs within a stage can run in parallel.
    pub jobs: Vec<NixJobConfig>,
    /// Whether to continue on job failure within this stage.
    #[serde(default)]
    pub continue_on_failure: bool,
}

/// A CI job definition.
#[derive(Debug, Clone, Deserialize)]
pub struct NixJobConfig {
    /// Job name (must be unique within the stage).
    pub name: String,
    /// Job type: "shell", "nix", or "vm".
    #[serde(rename = "type")]
    pub job_type: String,
    /// Shell command (for type = "shell").
    #[serde(default)]
    pub command: String,
    /// Nix flake attribute (for type = "nix").
    #[serde(default)]
    pub attribute: String,
    /// Working directory relative to repo root.
    #[serde(default)]
    pub working_dir: String,
    /// Job-level timeout in seconds.
    #[serde(default = "default_job_timeout")]
    pub timeout_secs: u64,
    /// Extra arguments passed to the builder.
    #[serde(default)]
    pub extra_args: Vec<String>,
}

fn default_job_timeout() -> u64 {
    1800
}

/// Errors from Nix config parsing.
#[derive(Debug)]
pub enum NixConfigError {
    /// File I/O error.
    ReadError(String),
    /// File exceeds size limit.
    TooLarge { size: usize, max: usize },
    /// Nix evaluation or deserialization error.
    EvalError(String),
    /// Impure operation detected.
    ImpureError(String),
    /// Validation error after deserialization.
    ValidationError(String),
}

impl std::fmt::Display for NixConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NixConfigError::ReadError(e) => write!(f, "failed to read config: {e}"),
            NixConfigError::TooLarge { size, max } => {
                write!(f, "config too large: {size} bytes exceeds {max} byte limit")
            }
            NixConfigError::EvalError(e) => write!(f, "nix evaluation error: {e}"),
            NixConfigError::ImpureError(e) => write!(f, "impure operation rejected: {e}"),
            NixConfigError::ValidationError(e) => write!(f, "config validation error: {e}"),
        }
    }
}

impl std::error::Error for NixConfigError {}

/// Load and parse a Nix CI pipeline config from a string.
///
/// Uses pure Nix evaluation only — no I/O builtins.
pub fn parse_nix_config(source: &str) -> Result<NixPipelineConfig, NixConfigError> {
    if source.len() > MAX_CONFIG_SIZE {
        return Err(NixConfigError::TooLarge {
            size: source.len(),
            max: MAX_CONFIG_SIZE,
        });
    }

    // Check for obvious impure patterns before evaluation
    check_impure_patterns(source)?;

    debug!(size = source.len(), "parsing Nix CI config");

    // Use snix_serde::from_str which creates a pure evaluator internally.
    // The default EvaluationBuilder::new_pure() disables import and I/O builtins.
    let config: NixPipelineConfig =
        snix_serde::from_str(source).map_err(|e| NixConfigError::EvalError(format!("{e}")))?;

    validate_config(&config)?;

    info!(
        stages = config.stages.len(),
        total_jobs = config.stages.iter().map(|s| s.jobs.len()).sum::<usize>(),
        "parsed Nix CI config"
    );

    Ok(config)
}

/// Load and parse a Nix CI pipeline config from a file path.
pub async fn load_nix_config(path: &str) -> Result<NixPipelineConfig, NixConfigError> {
    let source = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| NixConfigError::ReadError(format!("{path}: {e}")))?;

    parse_nix_config(&source)
}

/// Scan source for impure builtin usage before evaluation.
///
/// This is a best-effort pre-check. The evaluator itself enforces purity
/// by using `EvaluationBuilder::new_pure()` which disables impure builtins.
fn check_impure_patterns(source: &str) -> Result<(), NixConfigError> {
    let impure_patterns = [
        "builtins.readFile",
        "builtins.readDir",
        "builtins.pathExists",
        "builtins.fetchurl",
        "builtins.fetchTarball",
        "builtins.fetchGit",
        "builtins.getEnv",
        "builtins.currentSystem",
        "builtins.currentTime",
    ];

    for pattern in &impure_patterns {
        if source.contains(pattern) {
            return Err(NixConfigError::ImpureError(format!("{pattern} is not allowed in CI config")));
        }
    }

    Ok(())
}

/// Validate a parsed config for semantic correctness.
fn validate_config(config: &NixPipelineConfig) -> Result<(), NixConfigError> {
    if config.stages.is_empty() {
        return Err(NixConfigError::ValidationError("pipeline must have at least one stage".to_string()));
    }

    for stage in &config.stages {
        if stage.name.is_empty() {
            return Err(NixConfigError::ValidationError("stage name must not be empty".to_string()));
        }
        if stage.jobs.is_empty() {
            return Err(NixConfigError::ValidationError(format!("stage '{}' must have at least one job", stage.name)));
        }
        for job in &stage.jobs {
            if job.name.is_empty() {
                return Err(NixConfigError::ValidationError(format!("job in stage '{}' has empty name", stage.name)));
            }
            match job.job_type.as_str() {
                "shell" => {
                    if job.command.is_empty() {
                        return Err(NixConfigError::ValidationError(format!(
                            "shell job '{}' in stage '{}' has no command",
                            job.name, stage.name
                        )));
                    }
                }
                "nix" => {
                    if job.attribute.is_empty() {
                        return Err(NixConfigError::ValidationError(format!(
                            "nix job '{}' in stage '{}' has no attribute",
                            job.name, stage.name
                        )));
                    }
                }
                "vm" => {} // VM jobs have different config
                other => {
                    return Err(NixConfigError::ValidationError(format!(
                        "unknown job type '{other}' in job '{}' stage '{}'",
                        job.name, stage.name
                    )));
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_config() {
        let config = parse_nix_config(
            r#"
            {
                stages = [
                    {
                        name = "check";
                        jobs = [
                            { name = "clippy"; type = "shell"; command = "cargo clippy"; }
                        ];
                    }
                ];
            }
            "#,
        )
        .expect("should parse");

        assert_eq!(config.stages.len(), 1);
        assert_eq!(config.stages[0].name, "check");
        assert_eq!(config.stages[0].jobs.len(), 1);
        assert_eq!(config.stages[0].jobs[0].name, "clippy");
        assert_eq!(config.stages[0].jobs[0].job_type, "shell");
        assert_eq!(config.stages[0].jobs[0].command, "cargo clippy");
    }

    #[test]
    fn test_parse_nested_config() {
        let config = parse_nix_config(
            r#"
            {
                timeout_secs = 7200;
                stages = [
                    {
                        name = "lint";
                        continue_on_failure = true;
                        jobs = [
                            { name = "fmt"; type = "shell"; command = "cargo fmt --check"; }
                            { name = "clippy"; type = "shell"; command = "cargo clippy"; }
                        ];
                    }
                    {
                        name = "build";
                        jobs = [
                            {
                                name = "build-node";
                                type = "nix";
                                attribute = "packages.x86_64-linux.aspen-node";
                                timeout_secs = 3600;
                            }
                        ];
                    }
                ];
            }
            "#,
        )
        .expect("should parse");

        assert_eq!(config.timeout_secs, 7200);
        assert_eq!(config.stages.len(), 2);
        assert!(config.stages[0].continue_on_failure);
        assert_eq!(config.stages[0].jobs.len(), 2);
        assert_eq!(config.stages[1].jobs[0].attribute, "packages.x86_64-linux.aspen-node");
        assert_eq!(config.stages[1].jobs[0].timeout_secs, 3600);
    }

    #[test]
    fn test_parse_with_nix_let_binding() {
        let config = parse_nix_config(
            r#"
            let
                mkShellJob = name: cmd: {
                    inherit name;
                    type = "shell";
                    command = cmd;
                };
            in {
                stages = [
                    {
                        name = "check";
                        jobs = [
                            (mkShellJob "fmt" "cargo fmt --check")
                            (mkShellJob "clippy" "cargo clippy")
                        ];
                    }
                ];
            }
            "#,
        )
        .expect("should parse nix with let bindings");

        assert_eq!(config.stages[0].jobs.len(), 2);
        assert_eq!(config.stages[0].jobs[0].name, "fmt");
        assert_eq!(config.stages[0].jobs[1].name, "clippy");
    }

    #[test]
    fn test_parse_with_nix_conditional() {
        let config = parse_nix_config(
            r#"
            let
                includeTests = true;
            in {
                stages = [
                    {
                        name = "build";
                        jobs = [
                            { name = "build"; type = "nix"; attribute = "default"; }
                        ];
                    }
                ] ++ (if includeTests then [
                    {
                        name = "test";
                        jobs = [
                            { name = "test"; type = "shell"; command = "cargo test"; }
                        ];
                    }
                ] else []);
            }
            "#,
        )
        .expect("should parse nix with conditionals");

        assert_eq!(config.stages.len(), 2);
        assert_eq!(config.stages[1].name, "test");
    }

    #[test]
    fn test_type_mismatch_error() {
        let result = parse_nix_config(
            r#"
            {
                stages = "not a list";
            }
            "#,
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("error") || msg.contains("type") || msg.contains("expected"),
            "should report type error, got: {msg}"
        );
    }

    #[test]
    fn test_impure_readfile_rejected() {
        let result = parse_nix_config(
            r#"
            {
                stages = [{
                    name = builtins.readFile ./name.txt;
                    jobs = [{ name = "x"; type = "shell"; command = "true"; }];
                }];
            }
            "#,
        );

        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("impure") || msg.contains("readFile"));
    }

    #[test]
    fn test_impure_getenv_rejected() {
        let result = parse_nix_config(
            r#"
            {
                stages = [{
                    name = builtins.getEnv "HOME";
                    jobs = [{ name = "x"; type = "shell"; command = "true"; }];
                }];
            }
            "#,
        );

        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("impure") || msg.contains("getEnv"));
    }

    #[test]
    fn test_empty_stages_rejected() {
        let result = parse_nix_config("{ stages = []; }");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_stage_name_rejected() {
        let result = parse_nix_config(
            r#"
            {
                stages = [{
                    name = "";
                    jobs = [{ name = "x"; type = "shell"; command = "true"; }];
                }];
            }
            "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_shell_job_missing_command_rejected() {
        let result = parse_nix_config(
            r#"
            {
                stages = [{
                    name = "check";
                    jobs = [{ name = "x"; type = "shell"; }];
                }];
            }
            "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_nix_job_missing_attribute_rejected() {
        let result = parse_nix_config(
            r#"
            {
                stages = [{
                    name = "build";
                    jobs = [{ name = "x"; type = "nix"; }];
                }];
            }
            "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_job_type_rejected() {
        let result = parse_nix_config(
            r#"
            {
                stages = [{
                    name = "check";
                    jobs = [{ name = "x"; type = "docker"; command = "echo hi"; }];
                }];
            }
            "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_source_too_large_rejected() {
        let huge = format!(
            "{{ stages = [{{ name = \"x\"; jobs = [{{ name = \"y\"; type = \"shell\"; command = \"{}\"; }}]; }}]; }}",
            "a".repeat(MAX_CONFIG_SIZE)
        );
        let result = parse_nix_config(&huge);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_timeout_values() {
        let config = parse_nix_config(
            r#"
            {
                stages = [{
                    name = "check";
                    jobs = [{ name = "lint"; type = "shell"; command = "true"; }];
                }];
            }
            "#,
        )
        .expect("should parse");

        assert_eq!(config.timeout_secs, 3600);
        assert_eq!(config.stages[0].jobs[0].timeout_secs, 1800);
    }

    #[test]
    fn test_extra_args() {
        let config = parse_nix_config(
            r#"
            {
                stages = [{
                    name = "build";
                    jobs = [{
                        name = "build-node";
                        type = "nix";
                        attribute = "default";
                        extra_args = ["--impure" "--verbose"];
                    }];
                }];
            }
            "#,
        )
        .expect("should parse");

        assert_eq!(config.stages[0].jobs[0].extra_args, vec!["--impure", "--verbose"]);
    }
}
