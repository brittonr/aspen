//! Structured flake reference parsing using `nix_compat::flakeref::FlakeRef`.
//!
//! Replaces raw string concatenation with type-safe parsing and validation.

use nix_compat::flakeref::FlakeRef;

/// Parsed flake reference with optional attribute path.
#[derive(Debug)]
pub struct ParsedFlakeRef {
    /// The parsed flake reference.
    pub flake_ref: FlakeRef,
    /// Attribute path (e.g., "packages.x86_64-linux.default").
    pub attribute: String,
    /// Original flake URL string (before attribute).
    pub raw_url: String,
}

impl ParsedFlakeRef {
    /// Parse a flake URL and attribute into a structured reference.
    ///
    /// Returns `Err` if the flake URL is not a valid flake reference.
    pub fn parse(flake_url: &str, attribute: &str) -> Result<Self, FlakeRefParseError> {
        // Normalize bare paths to path: URLs. The nix CLI accepts "/foo" and "./foo"
        // as shorthand for "path:/foo" and "path:./foo", but nix-compat's FlakeRef
        // parser requires the explicit scheme.
        let normalized = if flake_url.starts_with('/') || flake_url == "." || flake_url.starts_with("./") {
            format!("path:{flake_url}")
        } else {
            flake_url.to_string()
        };

        let flake_ref: FlakeRef = normalized.parse().map_err(|e| FlakeRefParseError {
            url: flake_url.to_string(),
            reason: format!("{e}"),
        })?;

        Ok(Self {
            flake_ref,
            attribute: attribute.to_string(),
            raw_url: flake_url.to_string(),
        })
    }

    /// Format as a complete flake reference string (URL#attribute).
    pub fn to_flake_ref_string(&self) -> String {
        let url = self.flake_ref.to_uri().to_string();
        if self.attribute.is_empty() {
            url
        } else {
            format!("{url}#{}", self.attribute)
        }
    }

    /// Extract the `rev` field if present (works across all FlakeRef variants).
    pub fn rev(&self) -> Option<&str> {
        match &self.flake_ref {
            FlakeRef::GitHub { rev, .. }
            | FlakeRef::GitLab { rev, .. }
            | FlakeRef::SourceHut { rev, .. }
            | FlakeRef::Git { rev, .. }
            | FlakeRef::Path { rev, .. }
            | FlakeRef::File { rev, .. }
            | FlakeRef::Tarball { rev, .. }
            | FlakeRef::Indirect { rev, .. } => rev.as_deref(),
            FlakeRef::Mercurial { rev, .. } => rev.as_deref(),
            // Non-exhaustive: future variants handled gracefully
            _ => None,
        }
    }

    /// Extract the `ref` field if present (works across variants that have it).
    pub fn git_ref(&self) -> Option<&str> {
        match &self.flake_ref {
            FlakeRef::GitHub { r#ref, .. }
            | FlakeRef::GitLab { r#ref, .. }
            | FlakeRef::SourceHut { r#ref, .. }
            | FlakeRef::Git { r#ref, .. }
            | FlakeRef::Indirect { r#ref, .. }
            | FlakeRef::Mercurial { r#ref, .. } => r#ref.as_deref(),
            _ => None,
        }
    }

    /// Compute a normalized cache key component from the flake reference.
    ///
    /// Uses rev if available (most specific), falls back to ref, then raw URL.
    pub fn cache_key_component(&self) -> String {
        if let Some(rev) = self.rev() {
            return rev.to_string();
        }
        if let Some(git_ref) = self.git_ref() {
            return git_ref.to_string();
        }
        self.raw_url.clone()
    }
}

/// Error when a flake URL cannot be parsed.
#[derive(Debug, Clone)]
pub struct FlakeRefParseError {
    pub url: String,
    pub reason: String,
}

impl std::fmt::Display for FlakeRefParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid flake reference '{}': {}", self.url, self.reason)
    }
}

impl std::error::Error for FlakeRefParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_flakeref() {
        let parsed = ParsedFlakeRef::parse("github:owner/repo", "").unwrap();
        assert!(matches!(parsed.flake_ref, FlakeRef::GitHub { .. }));
        assert!(parsed.rev().is_none());
        assert!(parsed.git_ref().is_none());
    }

    #[test]
    fn test_parse_github_with_attribute() {
        let parsed = ParsedFlakeRef::parse("github:owner/repo", "packages.x86_64-linux.default").unwrap();
        let ref_str = parsed.to_flake_ref_string();
        assert!(ref_str.contains("github:"));
        assert!(ref_str.ends_with("#packages.x86_64-linux.default"));
    }

    #[test]
    fn test_parse_github_with_rev() {
        let parsed = ParsedFlakeRef::parse("github:owner/repo?rev=abc123", "").unwrap();
        assert_eq!(parsed.rev(), Some("abc123"));
    }

    #[test]
    fn test_parse_github_with_ref() {
        let parsed = ParsedFlakeRef::parse("github:owner/repo?ref=develop", "").unwrap();
        assert_eq!(parsed.git_ref(), Some("develop"));
    }

    #[test]
    fn test_parse_gitlab_flakeref() {
        let parsed = ParsedFlakeRef::parse("gitlab:group/project", "").unwrap();
        assert!(matches!(parsed.flake_ref, FlakeRef::GitLab { .. }));
    }

    #[test]
    fn test_parse_path_flakeref() {
        let parsed = ParsedFlakeRef::parse("path:./my-project", "").unwrap();
        assert!(matches!(parsed.flake_ref, FlakeRef::Path { .. }));
    }

    #[test]
    fn test_parse_git_flakeref() {
        let parsed = ParsedFlakeRef::parse("git+https://example.com/repo.git?ref=main", "").unwrap();
        assert!(matches!(parsed.flake_ref, FlakeRef::Git { .. }));
        assert_eq!(parsed.git_ref(), Some("main"));
    }

    #[test]
    fn test_parse_indirect_flakeref() {
        // indirect+https scheme triggers the Indirect variant
        let parsed = ParsedFlakeRef::parse("indirect+https://nixpkgs", "").unwrap();
        assert!(matches!(parsed.flake_ref, FlakeRef::Indirect { .. }));
    }

    #[test]
    fn test_invalid_flakeref() {
        let result = ParsedFlakeRef::parse("not a valid url at all!", "");
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_key_with_rev() {
        let parsed = ParsedFlakeRef::parse("github:owner/repo?rev=deadbeef", "").unwrap();
        assert_eq!(parsed.cache_key_component(), "deadbeef");
    }

    #[test]
    fn test_cache_key_with_ref_no_rev() {
        let parsed = ParsedFlakeRef::parse("github:owner/repo?ref=main", "").unwrap();
        assert_eq!(parsed.cache_key_component(), "main");
    }

    #[test]
    fn test_cache_key_fallback_to_url() {
        let parsed = ParsedFlakeRef::parse("github:owner/repo", "").unwrap();
        assert_eq!(parsed.cache_key_component(), "github:owner/repo");
    }

    #[test]
    fn test_flakeref_roundtrip_github() {
        let parsed = ParsedFlakeRef::parse("github:owner/repo?ref=v1.0", "pkg").unwrap();
        let formatted = parsed.to_flake_ref_string();
        assert!(formatted.contains("owner/repo"));
        assert!(formatted.contains("ref=v1.0"));
        assert!(formatted.ends_with("#pkg"));
    }
}
