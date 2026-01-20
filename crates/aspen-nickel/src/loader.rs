//! Nickel configuration loader.
//!
//! Loads and evaluates Nickel configuration files, applying schema contracts
//! for validation and deserializing to Rust types.

use std::fs;
use std::path::Path;

use nickel_lang::Context;
use serde::de::DeserializeOwned;
use snafu::ResultExt;
use tracing::debug;
use tracing::instrument;

use crate::error::NickelConfigError;
use crate::error::{self};
use crate::schema;

/// Load a configuration from a Nickel file.
///
/// This function:
/// 1. Validates file size (Tiger Style: bounded resources)
/// 2. Reads the file content
/// 3. Evaluates the Nickel expression with schema contracts
/// 4. Deserializes to the target Rust type
///
/// # Arguments
///
/// * `path` - Path to the Nickel configuration file (.ncl)
///
/// # Type Parameters
///
/// * `T` - The target configuration type (must implement `DeserializeOwned`)
///
/// # Errors
///
/// Returns an error if:
/// - The file is too large (> MAX_CONFIG_FILE_SIZE)
/// - The file cannot be read
/// - The Nickel evaluation fails (syntax, type, or contract error)
/// - Deserialization to the target type fails
///
/// # Example
///
/// ```rust,ignore
/// use aspen_nickel::load_nickel_config;
/// use aspen_cluster::config::NodeConfig;
///
/// let config: NodeConfig = load_nickel_config(Path::new("cluster.ncl"))?;
/// ```
#[instrument(skip_all, fields(path = %path.display()))]
pub fn load_nickel_config<T: DeserializeOwned>(path: &Path) -> Result<T, NickelConfigError> {
    // Check file exists
    if !path.exists() {
        return Err(NickelConfigError::FileNotFound {
            path: path.to_path_buf(),
        });
    }

    // Tiger Style: Check file size before reading to prevent DoS
    let metadata = fs::metadata(path).context(error::ReadFileSnafu {
        path: path.to_path_buf(),
    })?;

    let max_size = aspen_constants::MAX_CONFIG_FILE_SIZE;
    if metadata.len() > max_size {
        return Err(NickelConfigError::FileTooLarge {
            size: metadata.len(),
            max: max_size,
        });
    }

    debug!(size = metadata.len(), "Reading Nickel config file");

    // Read file content
    let content = fs::read_to_string(path).context(error::ReadFileSnafu {
        path: path.to_path_buf(),
    })?;

    // Load with source name for better error messages
    load_nickel_config_str(&content, path.display().to_string())
}

/// Load a configuration from a Nickel source string.
///
/// This is useful for:
/// - Testing with inline configuration
/// - Programmatically generated configuration
/// - Configuration passed via environment or RPC
///
/// # Arguments
///
/// * `content` - The Nickel source code
/// * `source_name` - A name for the source (used in error messages)
///
/// # Type Parameters
///
/// * `T` - The target configuration type (must implement `DeserializeOwned`)
///
/// # Example
///
/// ```rust,ignore
/// use aspen_nickel::load_nickel_config_str;
///
/// let config: MyConfig = load_nickel_config_str(
///     "{ node_id = 1, cookie = \"test\" }",
///     "inline".to_string(),
/// )?;
/// ```
#[instrument(skip(content), fields(source = %source_name))]
pub fn load_nickel_config_str<T: DeserializeOwned>(content: &str, source_name: String) -> Result<T, NickelConfigError> {
    // Get the embedded schema
    let schema_source = schema::get_schema();

    // Wrap user config with schema validation
    // The schema defines contracts that validate the configuration
    let wrapped = format!(
        r#"
let schema = {schema_source} in
({content}) | schema.NodeConfig
"#
    );

    debug!("Evaluating Nickel configuration");

    // Create context with source name for error messages
    let mut ctx = Context::new().with_source_name(source_name);

    // Evaluate deeply to ensure all values are computed
    let expr = ctx.eval_deep(&wrapped)?;

    debug!("Deserializing to target type");

    // Deserialize to target type using serde
    let config: T = expr.to_serde().map_err(|e| NickelConfigError::Deserialization {
        message: format!("{e}"),
    })?;

    Ok(config)
}

/// Load a configuration without schema validation.
///
/// Use this for custom configuration types that don't match the standard
/// NodeConfig schema, or when you want to bypass validation.
///
/// # Warning
///
/// This bypasses contract validation. Prefer `load_nickel_config` when possible.
#[instrument(skip(content), fields(source = %source_name))]
#[cfg_attr(not(test), allow(dead_code))]
pub fn load_nickel_config_raw<T: DeserializeOwned>(content: &str, source_name: String) -> Result<T, NickelConfigError> {
    let mut ctx = Context::new().with_source_name(source_name);
    let expr = ctx.eval_deep(content)?;

    let config: T = expr.to_serde().map_err(|e| NickelConfigError::Deserialization {
        message: format!("{e}"),
    })?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use serde::Deserialize;
    use tempfile::NamedTempFile;

    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    struct SimpleConfig {
        name: String,
        value: i64,
    }

    #[test]
    fn test_load_simple_config() {
        let config: SimpleConfig =
            load_nickel_config_raw(r#"{ name = "test", value = 42 }"#, "test".to_string()).unwrap();

        assert_eq!(config.name, "test");
        assert_eq!(config.value, 42);
    }

    #[test]
    fn test_load_with_merge() {
        // In Nickel, merging requires using 'force' priority to override
        // or the fields must be compatible (e.g., records merge recursively)
        let config: SimpleConfig = load_nickel_config_raw(
            r#"
            let base = { name = "base", value | default = 1 } in
            base & { value = 99 }
            "#,
            "test".to_string(),
        )
        .unwrap();

        assert_eq!(config.name, "base");
        assert_eq!(config.value, 99);
    }

    #[test]
    fn test_load_with_function() {
        // In Nickel, use different parameter names to avoid shadowing field names
        let config: SimpleConfig = load_nickel_config_raw(
            r#"
            let make_config = fun n v => { name = n, value = v } in
            make_config "generated" 100
            "#,
            "test".to_string(),
        )
        .unwrap();

        assert_eq!(config.name, "generated");
        assert_eq!(config.value, 100);
    }

    #[test]
    fn test_syntax_error() {
        let result: Result<SimpleConfig, _> = load_nickel_config_raw("{ name = ", "test".to_string());

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, NickelConfigError::Evaluation { .. }));
    }

    #[test]
    fn test_file_not_found() {
        let result: Result<SimpleConfig, _> = load_nickel_config(Path::new("/nonexistent/path.ncl"));

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), NickelConfigError::FileNotFound { .. }));
    }

    #[test]
    fn test_load_from_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{ name = "file-test", value = 123 }}"#).unwrap();

        // Read file content and use load_nickel_config_raw for SimpleConfig
        let content = std::fs::read_to_string(file.path()).unwrap();
        let config: SimpleConfig = load_nickel_config_raw(&content, file.path().display().to_string()).unwrap();

        assert_eq!(config.name, "file-test");
        assert_eq!(config.value, 123);
    }

    #[test]
    fn test_load_from_file_with_merge() {
        let mut file = NamedTempFile::new().unwrap();
        // In Nickel, use | default on fields that can be overridden
        writeln!(
            file,
            r#"
            let base = {{ name | default = "base", value | default = 10 }} in
            base & {{ name = "merged" }}
            "#
        )
        .unwrap();

        // Read file content and use load_nickel_config_raw for SimpleConfig
        let content = std::fs::read_to_string(file.path()).unwrap();
        let config: SimpleConfig = load_nickel_config_raw(&content, file.path().display().to_string()).unwrap();

        assert_eq!(config.name, "merged");
        assert_eq!(config.value, 10);
    }

    #[test]
    fn test_deserialization_error() {
        // Missing required field
        let result: Result<SimpleConfig, _> = load_nickel_config_raw("{ name = \"test\" }", "test".to_string());

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), NickelConfigError::Deserialization { .. }));
    }

    #[test]
    fn test_type_mismatch_error() {
        // Wrong type for value field
        let result: Result<SimpleConfig, _> =
            load_nickel_config_raw(r#"{ name = "test", value = "not a number" }"#, "test".to_string());

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), NickelConfigError::Deserialization { .. }));
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct ConfigWithOptional {
        name: String,
        #[serde(default)]
        optional_field: Option<String>,
    }

    #[test]
    fn test_optional_fields() {
        let config: ConfigWithOptional = load_nickel_config_raw(r#"{ name = "test" }"#, "test".to_string()).unwrap();

        assert_eq!(config.name, "test");
        assert_eq!(config.optional_field, None);
    }

    #[test]
    fn test_optional_fields_provided() {
        let config: ConfigWithOptional =
            load_nickel_config_raw(r#"{ name = "test", optional_field = "present" }"#, "test".to_string()).unwrap();

        assert_eq!(config.name, "test");
        assert_eq!(config.optional_field, Some("present".to_string()));
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct ConfigWithNested {
        name: String,
        inner: InnerConfig,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct InnerConfig {
        enabled: bool,
        count: i32,
    }

    #[test]
    fn test_nested_config() {
        let config: ConfigWithNested = load_nickel_config_raw(
            r#"
            {
                name = "nested-test",
                inner = {
                    enabled = true,
                    count = 5,
                },
            }
            "#,
            "test".to_string(),
        )
        .unwrap();

        assert_eq!(config.name, "nested-test");
        assert!(config.inner.enabled);
        assert_eq!(config.inner.count, 5);
    }

    #[test]
    fn test_nickel_let_bindings() {
        let config: SimpleConfig = load_nickel_config_raw(
            r#"
            let prefix = "computed" in
            let multiplier = 10 in
            {
                name = prefix,
                value = 4 * multiplier,
            }
            "#,
            "test".to_string(),
        )
        .unwrap();

        assert_eq!(config.name, "computed");
        assert_eq!(config.value, 40);
    }

    #[test]
    fn test_nickel_conditional() {
        let config: SimpleConfig = load_nickel_config_raw(
            r#"
            let production = false in
            {
                name = if production then "prod" else "dev",
                value = if production then 1000 else 1,
            }
            "#,
            "test".to_string(),
        )
        .unwrap();

        assert_eq!(config.name, "dev");
        assert_eq!(config.value, 1);
    }
}
