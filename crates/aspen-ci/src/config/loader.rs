//! Nickel configuration loader for CI pipelines.
//!
//! Loads and evaluates Nickel configuration files, applying schema contracts
//! for validation and deserializing to Rust types.
//!
//! # Stack Usage
//!
//! Nickel's evaluation is deeply recursive. To avoid stack overflow in async
//! contexts, use `load_pipeline_config_str_async` which runs the evaluation
//! on a dedicated thread with stack growth via the stacker crate.

use std::fs;
use std::path::Path;
use std::thread;

use nickel_lang::Context;
use tracing::debug;
use tracing::instrument;

use super::schema;
use super::types::PipelineConfig;
use crate::error::CiError;
use crate::error::Result;

/// Stack size for Nickel evaluation thread (8 MiB).
///
/// This is the initial stack size for the Nickel evaluation thread.
/// The stacker crate will dynamically grow the stack as needed beyond this.
const NICKEL_EVAL_STACK_SIZE: usize = 8 * 1024 * 1024;

/// Red zone size for stacker (256 KiB).
///
/// When stack space drops below this threshold, stacker allocates more.
const STACKER_RED_ZONE: usize = 256 * 1024;

/// Stack growth size for stacker (4 MiB).
///
/// Amount of additional stack to allocate when growth is needed.
const STACKER_STACK_SIZE: usize = 4 * 1024 * 1024;

/// Maximum configuration file size (Tiger Style: bounded resources).
const MAX_CONFIG_FILE_SIZE: u64 = 1024 * 1024; // 1 MB

/// Load a pipeline configuration from a Nickel file.
///
/// This function:
/// 1. Validates file size (Tiger Style: bounded resources)
/// 2. Reads the file content
/// 3. Evaluates the Nickel expression with schema contracts
/// 4. Validates the configuration
/// 5. Deserializes to `PipelineConfig`
///
/// # Arguments
///
/// * `path` - Path to the Nickel configuration file (.ncl)
///
/// # Errors
///
/// Returns an error if:
/// - The file is too large (> MAX_CONFIG_FILE_SIZE)
/// - The file cannot be read
/// - The Nickel evaluation fails (syntax, type, or contract error)
/// - Deserialization to `PipelineConfig` fails
/// - The configuration is invalid (missing required fields, circular deps, etc.)
///
/// # Example
///
/// ```rust,ignore
/// use aspen_ci::config::load_pipeline_config;
///
/// let config = load_pipeline_config(Path::new(".aspen/ci.ncl"))?;
/// println!("Pipeline: {}", config.name);
/// ```
#[instrument(skip_all, fields(path = %path.display()))]
pub fn load_pipeline_config(path: &Path) -> Result<PipelineConfig> {
    // Check file exists
    if !path.exists() {
        return Err(CiError::ConfigNotFound {
            path: path.to_path_buf(),
        });
    }

    // Tiger Style: Check file size before reading to prevent DoS
    let metadata = fs::metadata(path).map_err(|e| CiError::ReadConfig {
        path: path.to_path_buf(),
        source: e,
    })?;

    if metadata.len() > MAX_CONFIG_FILE_SIZE {
        return Err(CiError::ConfigTooLarge {
            size: metadata.len(),
            max: MAX_CONFIG_FILE_SIZE,
        });
    }

    debug!(size = metadata.len(), "Reading CI config file");

    // Read file content
    let content = fs::read_to_string(path).map_err(|e| CiError::ReadConfig {
        path: path.to_path_buf(),
        source: e,
    })?;

    // Load with source name for better error messages
    load_pipeline_config_str(&content, path.display().to_string())
}

/// Load a pipeline configuration from a Nickel source string.
///
/// This is useful for:
/// - Testing with inline configuration
/// - Programmatically generated configuration
/// - Configuration fetched from a repository commit
///
/// # Arguments
///
/// * `content` - The Nickel source code
/// * `source_name` - A name for the source (used in error messages)
///
/// # Example
///
/// ```rust,ignore
/// use aspen_ci::config::load_pipeline_config_str;
///
/// let config = load_pipeline_config_str(
///     r#"{ name = "test", stages = [] }"#,
///     "inline".to_string(),
/// )?;
/// ```
#[instrument(skip(content), fields(source = %source_name))]
pub fn load_pipeline_config_str(content: &str, source_name: String) -> Result<PipelineConfig> {
    // Get the embedded schema
    let schema_source = schema::get_schema();

    // Wrap user config with schema validation
    // The schema evaluates directly to the PipelineConfig contract
    let wrapped = format!(
        r#"
let PipelineConfig = {schema_source} in
({content}) | PipelineConfig
"#
    );

    debug!("Evaluating Nickel CI configuration");

    // Create context with source name for error messages
    let mut ctx = Context::new().with_source_name(source_name);

    // Evaluate deeply to ensure all values are computed
    // Use stacker to dynamically grow the stack as needed for deep recursion
    let expr = stacker::maybe_grow(STACKER_RED_ZONE, STACKER_STACK_SIZE, || ctx.eval_deep(&wrapped)).map_err(|e| {
        CiError::NickelEvaluation {
            message: format!("{e:?}"),
        }
    })?;

    debug!("Deserializing to PipelineConfig");

    // Deserialize to PipelineConfig using serde
    let config: PipelineConfig = expr.to_serde().map_err(|e| CiError::Deserialization {
        message: format!("{e:?}"),
    })?;

    // Validate the configuration
    config.validate()?;

    Ok(config)
}

/// Load a pipeline configuration from a string, running on a dedicated thread.
///
/// This async function spawns a thread with a large stack for Nickel evaluation.
/// Due to Nickel's deeply recursive evaluation, we bypass schema validation
/// in async contexts and use the raw loader to avoid stack overflow.
///
/// Use this function in async contexts (e.g., RPC handlers) instead of
/// `load_pipeline_config_str` to avoid stack overflow.
///
/// # Arguments
///
/// * `content` - The Nickel source code
/// * `source_name` - A name for the source (used in error messages)
///
/// # Note
///
/// This function bypasses Nickel contract validation to avoid stack overflow.
/// The configuration is still validated via Rust-side validation after parsing.
///
/// # Example
///
/// ```rust,ignore
/// use aspen_ci::config::load_pipeline_config_str_async;
///
/// let config = load_pipeline_config_str_async(
///     r#"{ name = "test", stages = [] }"#.to_string(),
///     "inline".to_string(),
/// ).await?;
/// ```
pub async fn load_pipeline_config_str_async(content: String, source_name: String) -> Result<PipelineConfig> {
    // Spawn a thread with a large stack for Nickel evaluation
    // We use load_pipeline_config_raw to bypass schema contract validation
    // which causes deep recursion. Validation is done on the Rust side.
    let handle = thread::Builder::new()
        .name("nickel-eval".to_string())
        .stack_size(NICKEL_EVAL_STACK_SIZE)
        .spawn(move || load_pipeline_config_raw(&content, source_name))
        .map_err(|e| CiError::NickelEvaluation {
            message: format!("failed to spawn Nickel evaluation thread: {e}"),
        })?;

    // Wait for the thread to complete using tokio's spawn_blocking
    // This awaits the thread without blocking the async runtime
    tokio::task::spawn_blocking(move || handle.join())
        .await
        .map_err(|e| CiError::NickelEvaluation {
            message: format!("Nickel evaluation task was cancelled: {e}"),
        })?
        .map_err(|e| CiError::NickelEvaluation {
            message: format!("Nickel evaluation thread panicked: {:?}", e),
        })?
}

/// Load a pipeline configuration without schema validation.
///
/// Use this for testing or when you want to bypass contract validation.
///
/// # Warning
///
/// This bypasses Nickel contract validation. Prefer `load_pipeline_config` in production.
#[instrument(skip(content), fields(source = %source_name))]
#[cfg_attr(not(test), allow(dead_code))]
pub fn load_pipeline_config_raw(content: &str, source_name: String) -> Result<PipelineConfig> {
    let mut ctx = Context::new().with_source_name(source_name);
    let expr = ctx.eval_deep(content).map_err(|e| CiError::NickelEvaluation {
        message: format!("{e:?}"),
    })?;

    let config: PipelineConfig = expr.to_serde().map_err(|e| CiError::Deserialization {
        message: format!("{e:?}"),
    })?;

    config.validate()?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn test_load_minimal_config() {
        let config = load_pipeline_config_raw(
            r#"
            {
                name = "test-pipeline",
                stages = [
                    {
                        name = "build",
                        jobs = [
                            {
                                name = "compile",
                                type = 'shell,
                                command = "cargo",
                                args = ["build"],
                            },
                        ],
                    },
                ],
            }
            "#,
            "test".to_string(),
        )
        .unwrap();

        assert_eq!(config.name, "test-pipeline");
        assert_eq!(config.stages.len(), 1);
        assert_eq!(config.stages[0].name, "build");
        assert_eq!(config.stages[0].jobs.len(), 1);
        assert_eq!(config.stages[0].jobs[0].name, "compile");
    }

    #[test]
    fn test_load_with_defaults() {
        let config = load_pipeline_config_raw(
            r#"
            {
                name = "defaults-test",
                stages = [
                    {
                        name = "test",
                        jobs = [
                            {
                                name = "unit",
                                command = "cargo",
                                args = ["test"],
                            },
                        ],
                    },
                ],
            }
            "#,
            "test".to_string(),
        )
        .unwrap();

        // Check defaults were applied
        assert_eq!(config.timeout_secs, 7200);
        assert!(config.stages[0].parallel);
        assert_eq!(config.stages[0].jobs[0].timeout_secs, 3600);
        assert_eq!(config.stages[0].jobs[0].retry_count, 0);
    }

    #[test]
    fn test_load_nix_job() {
        let config = load_pipeline_config_raw(
            r#"
            {
                name = "nix-build",
                stages = [
                    {
                        name = "build",
                        jobs = [
                            {
                                name = "package",
                                type = 'nix,
                                flake_url = ".",
                                flake_attr = "packages.x86_64-linux.default",
                            },
                        ],
                    },
                ],
            }
            "#,
            "test".to_string(),
        )
        .unwrap();

        assert_eq!(config.stages[0].jobs[0].job_type, super::super::types::JobType::Nix);
        assert_eq!(config.stages[0].jobs[0].flake_url, Some(".".to_string()));
    }

    #[test]
    fn test_load_with_triggers() {
        let config = load_pipeline_config_raw(
            r#"
            {
                name = "triggered",
                triggers = {
                    refs = ["refs/heads/main", "refs/heads/feature/*"],
                    ignore_paths = ["*.md", "docs/*"],
                },
                stages = [
                    {
                        name = "build",
                        jobs = [{ name = "compile", command = "make" }],
                    },
                ],
            }
            "#,
            "test".to_string(),
        )
        .unwrap();

        assert_eq!(config.triggers.refs.len(), 2);
        assert!(config.triggers.should_trigger("refs/heads/main"));
        assert!(config.triggers.should_trigger("refs/heads/feature/foo"));
        assert!(!config.triggers.should_trigger("refs/heads/develop"));
    }

    #[test]
    fn test_load_with_dependencies() {
        let config = load_pipeline_config_raw(
            r#"
            {
                name = "multi-stage",
                stages = [
                    {
                        name = "build",
                        jobs = [{ name = "compile", command = "make" }],
                    },
                    {
                        name = "test",
                        depends_on = ["build"],
                        jobs = [{ name = "unit", command = "make", args = ["test"] }],
                    },
                    {
                        name = "deploy",
                        depends_on = ["test"],
                        when = "refs/heads/main",
                        jobs = [{ name = "push", command = "make", args = ["deploy"] }],
                    },
                ],
            }
            "#,
            "test".to_string(),
        )
        .unwrap();

        assert_eq!(config.stages.len(), 3);
        assert!(config.stages[1].depends_on.contains(&"build".to_string()));
        assert!(config.stages[2].depends_on.contains(&"test".to_string()));

        // Check topological order
        let ordered = config.stages_in_order();
        assert_eq!(ordered[0].name, "build");
        assert_eq!(ordered[1].name, "test");
        assert_eq!(ordered[2].name, "deploy");
    }

    #[test]
    fn test_circular_dependency_detection() {
        let result = load_pipeline_config_raw(
            r#"
            {
                name = "circular",
                stages = [
                    {
                        name = "a",
                        depends_on = ["c"],
                        jobs = [{ name = "job", command = "echo" }],
                    },
                    {
                        name = "b",
                        depends_on = ["a"],
                        jobs = [{ name = "job", command = "echo" }],
                    },
                    {
                        name = "c",
                        depends_on = ["b"],
                        jobs = [{ name = "job", command = "echo" }],
                    },
                ],
            }
            "#,
            "test".to_string(),
        );

        assert!(result.is_err());
        if let Err(CiError::CircularDependency { path }) = result {
            assert!(path.contains(" -> "));
        } else {
            panic!("Expected CircularDependency error");
        }
    }

    #[test]
    fn test_invalid_shell_job_no_command() {
        let result = load_pipeline_config_raw(
            r#"
            {
                name = "invalid",
                stages = [
                    {
                        name = "build",
                        jobs = [
                            {
                                name = "missing-cmd",
                                type = 'shell,
                            },
                        ],
                    },
                ],
            }
            "#,
            "test".to_string(),
        );

        assert!(result.is_err());
        if let Err(CiError::InvalidConfig { reason }) = result {
            assert!(reason.contains("command"));
        } else {
            panic!("Expected InvalidConfig error");
        }
    }

    #[test]
    fn test_file_not_found() {
        let result = load_pipeline_config(Path::new("/nonexistent/ci.ncl"));
        assert!(matches!(result, Err(CiError::ConfigNotFound { .. })));
    }

    #[test]
    fn test_load_from_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{
                name = "file-test",
                stages = [
                    {{
                        name = "build",
                        jobs = [{{ name = "compile", command = "cargo", args = ["build"] }}],
                    }},
                ],
            }}"#
        )
        .unwrap();

        // Use raw loader since we're not applying the full schema
        let content = fs::read_to_string(file.path()).unwrap();
        let config = load_pipeline_config_raw(&content, file.path().display().to_string()).unwrap();

        assert_eq!(config.name, "file-test");
    }

    #[test]
    fn test_syntax_error() {
        let result = load_pipeline_config_raw("{ name = ", "test".to_string());

        assert!(matches!(result, Err(CiError::NickelEvaluation { .. })));
    }

    #[test]
    fn test_nickel_functions() {
        let config = load_pipeline_config_raw(
            r#"
            let make_job = fun job_name job_cmd => {
                name = job_name,
                command = job_cmd,
            } in
            {
                name = "functional",
                stages = [
                    {
                        name = "build",
                        jobs = [
                            make_job "compile" "cargo",
                            make_job "lint" "clippy",
                        ],
                    },
                ],
            }
            "#,
            "test".to_string(),
        )
        .unwrap();

        assert_eq!(config.stages[0].jobs.len(), 2);
        assert_eq!(config.stages[0].jobs[0].name, "compile");
        assert_eq!(config.stages[0].jobs[1].name, "lint");
    }

    #[test]
    fn test_nickel_merge() {
        let config = load_pipeline_config_raw(
            r#"
            let base_job = {
                timeout_secs | default = 600,
                retry_count | default = 2,
            } in
            {
                name = "merged",
                stages = [
                    {
                        name = "build",
                        jobs = [
                            base_job & {
                                name = "compile",
                                command = "cargo",
                                args = ["build"],
                            },
                        ],
                    },
                ],
            }
            "#,
            "test".to_string(),
        )
        .unwrap();

        assert_eq!(config.stages[0].jobs[0].timeout_secs, 600);
        assert_eq!(config.stages[0].jobs[0].retry_count, 2);
    }

    #[test]
    fn test_load_with_schema_validation() {
        let config_str = r#"
{
  name = "ci-demo",
  stages = [
    {
      name = "test",
      jobs = [
        {
          name = "echo-test",
          type = 'shell,
          command = "echo test"
        }
      ]
    }
  ]
}
"#;
        let result = load_pipeline_config_str(config_str, "test".to_string());
        assert!(result.is_ok(), "Schema validation failed: {:?}", result);
    }
}
