//! SOPS file format detection and dispatch.
//!
//! Currently supports TOML. YAML and JSON are planned for v2.

pub mod toml;

use std::path::Path;

/// Supported SOPS file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SopsFormat {
    /// TOML format (.toml)
    Toml,
    /// YAML format (.yaml, .yml) — planned for v2
    Yaml,
    /// JSON format (.json) — planned for v2
    Json,
}

/// Detect the file format from a path's extension.
pub fn detect_format(path: &Path) -> crate::error::Result<SopsFormat> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("toml") => Ok(SopsFormat::Toml),
        Some("yaml" | "yml") => Err(crate::error::SopsError::InvalidFormat {
            reason: "YAML support is planned for v2".into(),
        }),
        Some("json") => Err(crate::error::SopsError::InvalidFormat {
            reason: "JSON support is planned for v2".into(),
        }),
        Some(ext) => Err(crate::error::SopsError::InvalidFormat {
            reason: format!("unknown file extension: .{ext}"),
        }),
        None => Err(crate::error::SopsError::InvalidFormat {
            reason: "file has no extension; specify format explicitly".into(),
        }),
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
    fn test_detect_unsupported() {
        assert!(detect_format(Path::new("secrets.yaml")).is_err());
        assert!(detect_format(Path::new("secrets.json")).is_err());
        assert!(detect_format(Path::new("secrets.xml")).is_err());
        assert!(detect_format(Path::new("noextension")).is_err());
    }
}
