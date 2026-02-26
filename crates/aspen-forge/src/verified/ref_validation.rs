//! Pure ref name validation functions.
//!
//! These functions validate Git reference names according to Git's
//! ref naming rules, without I/O or side effects.
//!
//! # Tiger Style
//!
//! - Pure validation functions
//! - Explicit error conditions
//! - No I/O or system calls

/// Result of ref name validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefNameError {
    /// Ref name is empty.
    Empty,
    /// Ref name exceeds maximum length.
    TooLong { length: usize, max: usize },
    /// Ref name starts with a dot.
    StartsWithDot,
    /// Ref name ends with a dot.
    EndsWithDot,
    /// Ref name contains consecutive dots.
    ConsecutiveDots,
    /// Ref name ends with `.lock`.
    EndsWithLock,
    /// Ref name contains `..`.
    ContainsDoubleDot,
    /// Ref name starts with a slash.
    StartsWithSlash,
    /// Ref name ends with a slash.
    EndsWithSlash,
    /// Ref name contains consecutive slashes.
    ConsecutiveSlashes,
    /// Ref name contains invalid character.
    InvalidChar { char: char, position: usize },
    /// Ref name contains a control character.
    ControlChar { position: usize },
    /// Ref name contains `@{`.
    ContainsAtBrace,
    /// Component starts with a dot.
    ComponentStartsWithDot { component: String },
}

/// Validate a Git ref name.
///
/// Validates according to Git's ref naming rules:
/// - Cannot be empty
/// - Cannot start or end with `/`
/// - Cannot contain `..`, `@{`, consecutive slashes
/// - Cannot end with `.lock`
/// - Cannot contain control characters, space, tilde, caret, colon, asterisk
/// - Components cannot start with `.`
///
/// # Arguments
///
/// * `ref_name` - The ref name to validate
/// * `max_length` - Maximum allowed length in bytes
///
/// # Returns
///
/// `Ok(())` if valid, `Err(RefNameError)` otherwise.
///
/// # Example
///
/// ```
/// use aspen_forge::verified::{validate_ref_name, RefNameError};
///
/// assert!(validate_ref_name("heads/main", 200).is_ok());
/// assert!(validate_ref_name("tags/v1.0.0", 200).is_ok());
///
/// assert_eq!(validate_ref_name("", 200), Err(RefNameError::Empty));
/// assert_eq!(validate_ref_name(".hidden", 200), Err(RefNameError::StartsWithDot));
/// assert_eq!(validate_ref_name("refs/heads/main.lock", 200), Err(RefNameError::EndsWithLock));
/// ```
pub fn validate_ref_name(ref_name: &str, max_length: usize) -> Result<(), RefNameError> {
    // Empty check
    if ref_name.is_empty() {
        return Err(RefNameError::Empty);
    }

    // Length check
    if ref_name.len() > max_length {
        return Err(RefNameError::TooLong {
            length: ref_name.len(),
            max: max_length,
        });
    }

    // Cannot start with dot
    if ref_name.starts_with('.') {
        return Err(RefNameError::StartsWithDot);
    }

    // Cannot end with dot
    if ref_name.ends_with('.') {
        return Err(RefNameError::EndsWithDot);
    }

    // Cannot end with .lock
    if ref_name.ends_with(".lock") {
        return Err(RefNameError::EndsWithLock);
    }

    // Cannot start with slash
    if ref_name.starts_with('/') {
        return Err(RefNameError::StartsWithSlash);
    }

    // Cannot end with slash
    if ref_name.ends_with('/') {
        return Err(RefNameError::EndsWithSlash);
    }

    // Cannot contain @{
    if ref_name.contains("@{") {
        return Err(RefNameError::ContainsAtBrace);
    }

    // Check for consecutive dots
    if ref_name.contains("..") {
        return Err(RefNameError::ContainsDoubleDot);
    }

    // Check for consecutive slashes
    if ref_name.contains("//") {
        return Err(RefNameError::ConsecutiveSlashes);
    }

    // Check each character
    for (i, c) in ref_name.chars().enumerate() {
        // Control characters
        if c.is_control() {
            return Err(RefNameError::ControlChar { position: i });
        }

        // Invalid characters
        if matches!(c, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\') {
            return Err(RefNameError::InvalidChar { char: c, position: i });
        }
    }

    // Check each component doesn't start with dot
    for component in ref_name.split('/') {
        if component.starts_with('.') {
            return Err(RefNameError::ComponentStartsWithDot {
                component: component.to_string(),
            });
        }
    }

    Ok(())
}

/// Check if a ref name is a valid branch name.
///
/// Branch names must not contain certain patterns used for other refs.
///
/// # Arguments
///
/// * `name` - The branch name (without "heads/" prefix)
///
/// # Returns
///
/// `true` if the name is valid for a branch.
///
/// # Example
///
/// ```
/// use aspen_forge::verified::is_valid_branch_name;
///
/// assert!(is_valid_branch_name("main"));
/// assert!(is_valid_branch_name("feature/new-thing"));
/// assert!(!is_valid_branch_name("HEAD"));
/// assert!(!is_valid_branch_name("-starts-with-dash"));
/// ```
#[inline]
pub fn is_valid_branch_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Cannot be HEAD
    if name == "HEAD" {
        return false;
    }

    // Cannot start with dash
    if name.starts_with('-') {
        return false;
    }

    // Must pass general ref validation
    validate_ref_name(name, 200).is_ok()
}

/// Check if a ref name is a valid tag name.
///
/// Tags follow the same rules as branches.
///
/// # Arguments
///
/// * `name` - The tag name (without "tags/" prefix)
///
/// # Returns
///
/// `true` if the name is valid for a tag.
///
/// # Example
///
/// ```
/// use aspen_forge::verified::is_valid_tag_name;
///
/// assert!(is_valid_tag_name("v1.0.0"));
/// assert!(is_valid_tag_name("release/2024-01"));
/// assert!(!is_valid_tag_name(""));
/// ```
#[inline]
pub fn is_valid_tag_name(name: &str) -> bool {
    // Tags use the same rules as branches
    is_valid_branch_name(name)
}

/// Normalize a ref name by removing leading/trailing slashes.
///
/// # Arguments
///
/// * `ref_name` - The ref name to normalize
///
/// # Returns
///
/// The normalized ref name.
///
/// # Example
///
/// ```
/// use aspen_forge::verified::normalize_ref_name;
///
/// assert_eq!(normalize_ref_name("/heads/main/"), "heads/main");
/// assert_eq!(normalize_ref_name("heads/main"), "heads/main");
/// ```
#[inline]
pub fn normalize_ref_name(ref_name: &str) -> &str {
    ref_name.trim_matches('/')
}

/// Extract the ref type from a full ref name.
///
/// # Arguments
///
/// * `ref_name` - The full ref name (e.g., "heads/main", "tags/v1.0.0")
///
/// # Returns
///
/// The ref type prefix if recognized.
///
/// # Example
///
/// ```
/// use aspen_forge::verified::extract_ref_type;
///
/// assert_eq!(extract_ref_type("heads/main"), Some("heads"));
/// assert_eq!(extract_ref_type("tags/v1.0.0"), Some("tags"));
/// assert_eq!(extract_ref_type("main"), None);
/// ```
#[inline]
pub fn extract_ref_type(ref_name: &str) -> Option<&str> {
    ref_name.split('/').next().filter(|_| ref_name.contains('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // validate_ref_name tests
    // ========================================================================

    #[test]
    fn test_valid_ref_names() {
        assert!(validate_ref_name("heads/main", 200).is_ok());
        assert!(validate_ref_name("tags/v1.0.0", 200).is_ok());
        assert!(validate_ref_name("refs/remotes/origin/main", 200).is_ok());
        assert!(validate_ref_name("feature/my-feature", 200).is_ok());
    }

    #[test]
    fn test_empty_ref_name() {
        assert_eq!(validate_ref_name("", 200), Err(RefNameError::Empty));
    }

    #[test]
    fn test_too_long_ref_name() {
        let long_name = "a".repeat(201);
        match validate_ref_name(&long_name, 200) {
            Err(RefNameError::TooLong { length, max }) => {
                assert_eq!(length, 201);
                assert_eq!(max, 200);
            }
            other => panic!("Expected TooLong, got {:?}", other),
        }
    }

    #[test]
    fn test_starts_with_dot() {
        assert_eq!(validate_ref_name(".hidden", 200), Err(RefNameError::StartsWithDot));
    }

    #[test]
    fn test_ends_with_dot() {
        assert_eq!(validate_ref_name("file.", 200), Err(RefNameError::EndsWithDot));
    }

    #[test]
    fn test_ends_with_lock() {
        assert_eq!(validate_ref_name("refs/heads/main.lock", 200), Err(RefNameError::EndsWithLock));
    }

    #[test]
    fn test_starts_with_slash() {
        assert_eq!(validate_ref_name("/heads/main", 200), Err(RefNameError::StartsWithSlash));
    }

    #[test]
    fn test_ends_with_slash() {
        assert_eq!(validate_ref_name("heads/main/", 200), Err(RefNameError::EndsWithSlash));
    }

    #[test]
    fn test_contains_at_brace() {
        assert_eq!(validate_ref_name("refs@{1}", 200), Err(RefNameError::ContainsAtBrace));
    }

    #[test]
    fn test_contains_double_dot() {
        assert_eq!(validate_ref_name("heads..main", 200), Err(RefNameError::ContainsDoubleDot));
    }

    #[test]
    fn test_consecutive_slashes() {
        assert_eq!(validate_ref_name("heads//main", 200), Err(RefNameError::ConsecutiveSlashes));
    }

    #[test]
    fn test_invalid_chars() {
        assert!(matches!(
            validate_ref_name("heads/main branch", 200),
            Err(RefNameError::InvalidChar { char: ' ', .. })
        ));
        assert!(matches!(validate_ref_name("heads/main~1", 200), Err(RefNameError::InvalidChar { char: '~', .. })));
        assert!(matches!(validate_ref_name("heads/main^2", 200), Err(RefNameError::InvalidChar { char: '^', .. })));
    }

    #[test]
    fn test_component_starts_with_dot() {
        match validate_ref_name("heads/.hidden", 200) {
            Err(RefNameError::ComponentStartsWithDot { component }) => {
                assert_eq!(component, ".hidden");
            }
            other => panic!("Expected ComponentStartsWithDot, got {:?}", other),
        }
    }

    // ========================================================================
    // is_valid_branch_name tests
    // ========================================================================

    #[test]
    fn test_valid_branch_names() {
        assert!(is_valid_branch_name("main"));
        assert!(is_valid_branch_name("feature/new-thing"));
        assert!(is_valid_branch_name("fix-123"));
    }

    #[test]
    fn test_invalid_branch_names() {
        assert!(!is_valid_branch_name(""));
        assert!(!is_valid_branch_name("HEAD"));
        assert!(!is_valid_branch_name("-starts-with-dash"));
    }

    // ========================================================================
    // normalize_ref_name tests
    // ========================================================================

    #[test]
    fn test_normalize() {
        assert_eq!(normalize_ref_name("/heads/main/"), "heads/main");
        assert_eq!(normalize_ref_name("heads/main"), "heads/main");
        assert_eq!(normalize_ref_name("///main///"), "main");
    }

    // ========================================================================
    // extract_ref_type tests
    // ========================================================================

    #[test]
    fn test_extract_ref_type() {
        assert_eq!(extract_ref_type("heads/main"), Some("heads"));
        assert_eq!(extract_ref_type("tags/v1.0.0"), Some("tags"));
        assert_eq!(extract_ref_type("refs/remotes/origin/main"), Some("refs"));
        assert_eq!(extract_ref_type("main"), None);
    }
}
