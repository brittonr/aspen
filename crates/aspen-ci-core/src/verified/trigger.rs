//! Pure trigger pattern matching functions.
//!
//! These functions evaluate whether refs match trigger patterns
//! without side effects.
//!
//! # Tiger Style
//!
//! - Pure matching functions
//! - Explicit pattern syntax
//! - No I/O or system calls

/// Check if a ref matches a trigger pattern.
///
/// Supports:
/// - Exact match: "refs/heads/main"
/// - Wildcard suffix: "refs/heads/*"
/// - Glob pattern: "refs/heads/feature/*"
///
/// # Arguments
///
/// * `ref_name` - The ref being updated (e.g., "refs/heads/main")
/// * `pattern` - The trigger pattern
///
/// # Returns
///
/// `true` if the ref matches the pattern.
///
/// # Example
///
/// ```
/// use aspen_ci_core::verified::ref_matches_pattern;
///
/// // Exact match
/// assert!(ref_matches_pattern("refs/heads/main", "refs/heads/main"));
///
/// // Wildcard suffix
/// assert!(ref_matches_pattern("refs/heads/main", "refs/heads/*"));
/// assert!(ref_matches_pattern("refs/heads/feature/foo", "refs/heads/*"));
///
/// // Specific prefix
/// assert!(ref_matches_pattern("refs/heads/feature/foo", "refs/heads/feature/*"));
/// assert!(!ref_matches_pattern("refs/heads/bugfix/bar", "refs/heads/feature/*"));
///
/// // No match
/// assert!(!ref_matches_pattern("refs/tags/v1.0", "refs/heads/*"));
/// ```
pub fn ref_matches_pattern(ref_name: &str, pattern: &str) -> bool {
    // Exact match
    if ref_name == pattern {
        return true;
    }

    // Wildcard pattern ending with *
    if let Some(prefix) = pattern.strip_suffix('*') {
        return ref_name.starts_with(prefix);
    }

    false
}

/// Check if a ref matches any of the trigger patterns.
///
/// # Arguments
///
/// * `ref_name` - The ref being updated
/// * `patterns` - List of trigger patterns
///
/// # Returns
///
/// `true` if the ref matches any pattern.
///
/// # Example
///
/// ```
/// use aspen_ci_core::verified::ref_matches_any_pattern;
///
/// let patterns = &["refs/heads/main", "refs/heads/develop", "refs/tags/*"];
///
/// assert!(ref_matches_any_pattern("refs/heads/main", patterns));
/// assert!(ref_matches_any_pattern("refs/tags/v1.0", patterns));
/// assert!(!ref_matches_any_pattern("refs/heads/feature/foo", patterns));
/// ```
pub fn ref_matches_any_pattern(ref_name: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| ref_matches_pattern(ref_name, pattern))
}

/// Check if a path matches a trigger pattern.
///
/// Used for path-based triggers (e.g., only trigger when certain files change).
///
/// # Arguments
///
/// * `path` - The path being checked (e.g., "src/main.rs")
/// * `pattern` - The trigger pattern (e.g., "src/**", "*.rs")
///
/// # Returns
///
/// `true` if the path matches the pattern.
///
/// # Example
///
/// ```
/// use aspen_ci_core::verified::path_matches_pattern;
///
/// // Exact match
/// assert!(path_matches_pattern("README.md", "README.md"));
///
/// // Directory prefix
/// assert!(path_matches_pattern("src/main.rs", "src/*"));
/// assert!(path_matches_pattern("src/lib/utils.rs", "src/*"));
///
/// // Extension match (simple suffix)
/// assert!(path_matches_pattern("main.rs", "*.rs"));
/// assert!(!path_matches_pattern("main.py", "*.rs"));
/// ```
pub fn path_matches_pattern(path: &str, pattern: &str) -> bool {
    // Exact match
    if path == pattern {
        return true;
    }

    // Wildcard prefix: *.rs
    if let Some(suffix) = pattern.strip_prefix('*') {
        return path.ends_with(suffix);
    }

    // Wildcard suffix: src/*
    if let Some(prefix) = pattern.strip_suffix('*') {
        return path.starts_with(prefix);
    }

    // Double wildcard: src/**
    if let Some(prefix) = pattern.strip_suffix("**") {
        return path.starts_with(prefix);
    }

    false
}

/// Check if any path matches any trigger pattern.
///
/// # Arguments
///
/// * `paths` - List of changed paths
/// * `patterns` - List of trigger patterns
///
/// # Returns
///
/// `true` if any path matches any pattern.
///
/// # Example
///
/// ```
/// use aspen_ci_core::verified::any_path_matches;
///
/// let paths = &["src/main.rs", "README.md"];
/// let patterns = &["src/*", "docs/*"];
///
/// assert!(any_path_matches(paths, patterns));
///
/// let other_patterns = &["tests/*", "docs/*"];
/// assert!(!any_path_matches(paths, other_patterns));
/// ```
pub fn any_path_matches(paths: &[&str], patterns: &[&str]) -> bool {
    paths.iter().any(|path| patterns.iter().any(|pattern| path_matches_pattern(path, pattern)))
}

/// Extract the branch name from a ref.
///
/// # Arguments
///
/// * `ref_name` - Full ref name (e.g., "refs/heads/main")
///
/// # Returns
///
/// The branch name if it's a branch ref, `None` otherwise.
///
/// # Example
///
/// ```
/// use aspen_ci_core::verified::extract_branch_name;
///
/// assert_eq!(extract_branch_name("refs/heads/main"), Some("main"));
/// assert_eq!(extract_branch_name("refs/heads/feature/foo"), Some("feature/foo"));
/// assert_eq!(extract_branch_name("refs/tags/v1.0"), None);
/// ```
pub fn extract_branch_name(ref_name: &str) -> Option<&str> {
    ref_name.strip_prefix("refs/heads/")
}

/// Extract the tag name from a ref.
///
/// # Arguments
///
/// * `ref_name` - Full ref name (e.g., "refs/tags/v1.0")
///
/// # Returns
///
/// The tag name if it's a tag ref, `None` otherwise.
///
/// # Example
///
/// ```
/// use aspen_ci_core::verified::extract_tag_name;
///
/// assert_eq!(extract_tag_name("refs/tags/v1.0"), Some("v1.0"));
/// assert_eq!(extract_tag_name("refs/heads/main"), None);
/// ```
pub fn extract_tag_name(ref_name: &str) -> Option<&str> {
    ref_name.strip_prefix("refs/tags/")
}

/// Check if a ref is a branch ref.
///
/// # Arguments
///
/// * `ref_name` - Full ref name
///
/// # Returns
///
/// `true` if it's a branch ref.
///
/// # Example
///
/// ```
/// use aspen_ci_core::verified::is_branch_ref;
///
/// assert!(is_branch_ref("refs/heads/main"));
/// assert!(!is_branch_ref("refs/tags/v1.0"));
/// ```
#[inline]
pub fn is_branch_ref(ref_name: &str) -> bool {
    ref_name.starts_with("refs/heads/")
}

/// Check if a ref is a tag ref.
///
/// # Arguments
///
/// * `ref_name` - Full ref name
///
/// # Returns
///
/// `true` if it's a tag ref.
///
/// # Example
///
/// ```
/// use aspen_ci_core::verified::is_tag_ref;
///
/// assert!(is_tag_ref("refs/tags/v1.0"));
/// assert!(!is_tag_ref("refs/heads/main"));
/// ```
#[inline]
pub fn is_tag_ref(ref_name: &str) -> bool {
    ref_name.starts_with("refs/tags/")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // ref_matches_pattern tests
    // ========================================================================

    #[test]
    fn test_exact_match() {
        assert!(ref_matches_pattern("refs/heads/main", "refs/heads/main"));
    }

    #[test]
    fn test_wildcard_suffix() {
        assert!(ref_matches_pattern("refs/heads/main", "refs/heads/*"));
        assert!(ref_matches_pattern("refs/heads/feature/foo", "refs/heads/*"));
    }

    #[test]
    fn test_specific_prefix() {
        assert!(ref_matches_pattern("refs/heads/feature/foo", "refs/heads/feature/*"));
        assert!(!ref_matches_pattern("refs/heads/bugfix/bar", "refs/heads/feature/*"));
    }

    #[test]
    fn test_no_match() {
        assert!(!ref_matches_pattern("refs/tags/v1.0", "refs/heads/*"));
    }

    // ========================================================================
    // ref_matches_any_pattern tests
    // ========================================================================

    #[test]
    fn test_matches_any() {
        let patterns = &["refs/heads/main", "refs/heads/develop", "refs/tags/*"];
        assert!(ref_matches_any_pattern("refs/heads/main", patterns));
        assert!(ref_matches_any_pattern("refs/tags/v1.0", patterns));
    }

    #[test]
    fn test_matches_none() {
        let patterns = &["refs/heads/main", "refs/tags/*"];
        assert!(!ref_matches_any_pattern("refs/heads/feature/foo", patterns));
    }

    // ========================================================================
    // path_matches_pattern tests
    // ========================================================================

    #[test]
    fn test_path_exact() {
        assert!(path_matches_pattern("README.md", "README.md"));
    }

    #[test]
    fn test_path_prefix() {
        assert!(path_matches_pattern("src/main.rs", "src/*"));
        assert!(path_matches_pattern("src/lib/utils.rs", "src/*"));
    }

    #[test]
    fn test_path_suffix() {
        assert!(path_matches_pattern("main.rs", "*.rs"));
        assert!(!path_matches_pattern("main.py", "*.rs"));
    }

    // ========================================================================
    // any_path_matches tests
    // ========================================================================

    #[test]
    fn test_any_path_matches() {
        let paths = &["src/main.rs", "README.md"];
        let patterns = &["src/*", "docs/*"];
        assert!(any_path_matches(paths, patterns));
    }

    #[test]
    fn test_no_path_matches() {
        let paths = &["src/main.rs", "README.md"];
        let patterns = &["tests/*", "docs/*"];
        assert!(!any_path_matches(paths, patterns));
    }

    // ========================================================================
    // extract_branch_name tests
    // ========================================================================

    #[test]
    fn test_extract_branch() {
        assert_eq!(extract_branch_name("refs/heads/main"), Some("main"));
        assert_eq!(extract_branch_name("refs/heads/feature/foo"), Some("feature/foo"));
    }

    #[test]
    fn test_extract_branch_not_branch() {
        assert_eq!(extract_branch_name("refs/tags/v1.0"), None);
    }

    // ========================================================================
    // extract_tag_name tests
    // ========================================================================

    #[test]
    fn test_extract_tag() {
        assert_eq!(extract_tag_name("refs/tags/v1.0"), Some("v1.0"));
    }

    #[test]
    fn test_extract_tag_not_tag() {
        assert_eq!(extract_tag_name("refs/heads/main"), None);
    }

    // ========================================================================
    // is_branch_ref / is_tag_ref tests
    // ========================================================================

    #[test]
    fn test_is_branch() {
        assert!(is_branch_ref("refs/heads/main"));
        assert!(!is_branch_ref("refs/tags/v1.0"));
    }

    #[test]
    fn test_is_tag() {
        assert!(is_tag_ref("refs/tags/v1.0"));
        assert!(!is_tag_ref("refs/heads/main"));
    }
}
