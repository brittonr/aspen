//! SOPS file format detection and dispatch.
//!
//! Supports TOML, JSON, and YAML formats. Each format module handles
//! encrypting/decrypting leaf values while preserving document structure.

pub mod common;
pub mod json;
pub mod toml;
pub mod yaml;

use std::path::Path;

// Re-export common functions at format level
pub use common::decrypt_sops_value;
pub use common::encrypt_sops_value;
pub use common::is_sops_encrypted;

use crate::sops::metadata::SopsFileMetadata;
use crate::sops::sops_error::Result;

/// Supported SOPS file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SopsFormat {
    /// TOML format (.toml)
    Toml,
    /// YAML format (.yaml, .yml)
    Yaml,
    /// JSON format (.json)
    Json,
}

impl SopsFormat {
    /// Return the file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            SopsFormat::Toml => "toml",
            SopsFormat::Yaml => "yaml",
            SopsFormat::Json => "json",
        }
    }
}

/// Detect the file format from a path's extension.
pub fn detect_format(path: &Path) -> Result<SopsFormat> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("toml") => Ok(SopsFormat::Toml),
        Some("yaml" | "yml") => Ok(SopsFormat::Yaml),
        Some("json") => Ok(SopsFormat::Json),
        Some(ext) => Err(crate::sops::sops_error::SopsError::InvalidFormat {
            reason: format!("unknown file extension: .{ext}"),
        }),
        None => Err(crate::sops::sops_error::SopsError::InvalidFormat {
            reason: "file has no extension; specify format explicitly".into(),
        }),
    }
}

/// Encrypt a document's values and inject SOPS metadata.
///
/// Dispatches to the appropriate format module based on the detected format.
/// Returns `(encrypted_output, value_pairs_for_mac)`.
pub fn encrypt_document(
    format: SopsFormat,
    contents: &str,
    data_key: &[u8; 32],
    encrypted_regex: Option<&str>,
    metadata: &SopsFileMetadata,
    input_path: &Path,
) -> Result<(String, Vec<(String, String)>)> {
    match format {
        SopsFormat::Toml => toml::encrypt_document(contents, data_key, encrypted_regex, metadata, input_path),
        SopsFormat::Json => json::encrypt_document(contents, data_key, encrypted_regex, metadata, input_path),
        SopsFormat::Yaml => yaml::encrypt_document(contents, data_key, encrypted_regex, metadata, input_path),
    }
}

/// Decrypt a document's values and remove SOPS metadata.
///
/// Dispatches to the appropriate format module based on the detected format.
/// Returns `(decrypted_output, value_pairs_for_mac)`.
pub fn decrypt_document(
    format: SopsFormat,
    contents: &str,
    data_key: &[u8; 32],
    input_path: &Path,
) -> Result<(String, Vec<(String, String)>)> {
    match format {
        SopsFormat::Toml => toml::decrypt_document(contents, data_key, input_path),
        SopsFormat::Json => json::decrypt_document(contents, data_key, input_path),
        SopsFormat::Yaml => yaml::decrypt_document(contents, data_key, input_path),
    }
}

/// Extract SOPS metadata from file contents.
///
/// Dispatches to the appropriate format module.
pub fn extract_metadata(format: SopsFormat, contents: &str, input_path: &Path) -> Result<Option<SopsFileMetadata>> {
    match format {
        SopsFormat::Toml => toml::extract_metadata_from_contents(contents, input_path),
        SopsFormat::Json => json::extract_metadata_from_contents(contents, input_path),
        SopsFormat::Yaml => yaml::extract_metadata_from_contents(contents, input_path),
    }
}

/// Update SOPS metadata in an existing encrypted document.
///
/// Dispatches to the appropriate format module.
pub fn update_metadata(
    format: SopsFormat,
    contents: &str,
    metadata: &SopsFileMetadata,
    input_path: &Path,
) -> Result<String> {
    match format {
        SopsFormat::Toml => toml::update_metadata_in_document(contents, metadata, input_path),
        SopsFormat::Json => json::update_metadata_in_document(contents, metadata, input_path),
        SopsFormat::Yaml => yaml::update_metadata_in_document(contents, metadata, input_path),
    }
}

/// Extract a single value from a decrypted document by dotted path.
///
/// Dispatches to the appropriate format module.
pub fn extract_value(format: SopsFormat, contents: &str, path: &str) -> Result<String> {
    match format {
        SopsFormat::Toml => toml::extract_value(contents, path),
        SopsFormat::Json => json::extract_value(contents, path),
        SopsFormat::Yaml => yaml::extract_value(contents, path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_toml() {
        assert_eq!(detect_format(Path::new("secrets.toml")).unwrap(), SopsFormat::Toml);
        assert_eq!(detect_format(Path::new("foo.sops.toml")).unwrap(), SopsFormat::Toml);
    }

    #[test]
    fn test_detect_json() {
        assert_eq!(detect_format(Path::new("secrets.json")).unwrap(), SopsFormat::Json);
        assert_eq!(detect_format(Path::new("foo.sops.json")).unwrap(), SopsFormat::Json);
    }

    #[test]
    fn test_detect_yaml() {
        assert_eq!(detect_format(Path::new("secrets.yaml")).unwrap(), SopsFormat::Yaml);
        assert_eq!(detect_format(Path::new("secrets.yml")).unwrap(), SopsFormat::Yaml);
        assert_eq!(detect_format(Path::new("foo.sops.yaml")).unwrap(), SopsFormat::Yaml);
    }

    #[test]
    fn test_detect_unsupported() {
        assert!(detect_format(Path::new("secrets.xml")).is_err());
        assert!(detect_format(Path::new("noextension")).is_err());
    }

    #[test]
    fn test_format_extension() {
        assert_eq!(SopsFormat::Toml.extension(), "toml");
        assert_eq!(SopsFormat::Json.extension(), "json");
        assert_eq!(SopsFormat::Yaml.extension(), "yaml");
    }
}
