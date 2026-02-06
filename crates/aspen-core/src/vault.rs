//! Key validation for reserved prefixes.
//!
//! Provides validation to protect reserved key prefixes from client writes.
//!
//! # Reserved Prefixes
//!
//! - `_system:` - Reserved for internal cluster data, rejected from client writes
//!
//! # Tiger Style
//!
//! - Explicit prefix checking prevents accidental system key overwrites
//! - Simple validation with clear error messages

use thiserror::Error;

/// Reserved prefix for internal system data.
/// Client writes to keys starting with this prefix are rejected.
pub const SYSTEM_PREFIX: &str = "_system:";

/// Errors that can occur during key validation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VaultError {
    /// Attempt to write to reserved system prefix.
    #[error("key prefix '_system:' is reserved for internal use")]
    SystemPrefixReserved,
}

/// Check if a key uses the reserved system prefix.
///
/// Returns true if the key starts with `_system:`.
pub fn is_system_key(key: &str) -> bool {
    key.starts_with(SYSTEM_PREFIX)
}

/// Validate a key for client write operations.
///
/// Returns an error if the key uses the reserved `_system:` prefix.
/// All other keys are allowed.
pub fn validate_client_key(key: &str) -> Result<(), VaultError> {
    if is_system_key(key) {
        return Err(VaultError::SystemPrefixReserved);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_system_key() {
        assert!(is_system_key("_system:metadata"));
        assert!(is_system_key("_system:internal"));
        assert!(!is_system_key("myapp/config"));
        assert!(!is_system_key("regular_key"));
        assert!(!is_system_key(""));
    }

    #[test]
    fn test_validate_client_key() {
        assert!(validate_client_key("myapp/config").is_ok());
        assert!(validate_client_key("any/path/format").is_ok());
        assert!(validate_client_key("regular_key").is_ok());
        assert!(validate_client_key("any:other:format").is_ok());
        assert!(validate_client_key("").is_ok());

        assert!(matches!(validate_client_key("_system:internal"), Err(VaultError::SystemPrefixReserved)));
    }

    // =========================================================================
    // SYSTEM_PREFIX Constant Tests
    // =========================================================================

    #[test]
    fn test_system_prefix_value() {
        assert_eq!(SYSTEM_PREFIX, "_system:");
    }

    #[test]
    fn test_system_prefix_ends_with_colon() {
        assert!(SYSTEM_PREFIX.ends_with(':'));
    }

    // =========================================================================
    // is_system_key Edge Cases
    // =========================================================================

    #[test]
    fn test_is_system_key_exact_prefix() {
        // Just the prefix itself (no additional path)
        assert!(is_system_key("_system:"));
    }

    #[test]
    fn test_is_system_key_with_various_suffixes() {
        assert!(is_system_key("_system:a"));
        assert!(is_system_key("_system:config/settings"));
        assert!(is_system_key("_system:nested/path/key"));
        assert!(is_system_key("_system:with-dashes"));
        assert!(is_system_key("_system:with_underscores"));
        assert!(is_system_key("_system:with.dots"));
    }

    #[test]
    fn test_is_system_key_case_sensitive() {
        // Prefix matching is case-sensitive
        assert!(!is_system_key("_SYSTEM:key"));
        assert!(!is_system_key("_System:key"));
        assert!(!is_system_key("_sYsTeM:key"));
    }

    #[test]
    fn test_is_system_key_similar_but_not_prefix() {
        // These look similar but don't start with the exact prefix
        assert!(!is_system_key("_systemkey")); // Missing colon
        assert!(!is_system_key("system:key")); // Missing underscore
        assert!(!is_system_key("__system:key")); // Extra underscore
        assert!(!is_system_key(" _system:key")); // Leading space
        assert!(!is_system_key("x_system:key")); // Leading character
    }

    #[test]
    fn test_is_system_key_unicode() {
        // Unicode after prefix is still a system key
        assert!(is_system_key("_system:test"));
        assert!(!is_system_key("test:_system:embedded"));
    }

    // =========================================================================
    // validate_client_key Edge Cases
    // =========================================================================

    #[test]
    fn test_validate_client_key_whitespace() {
        // Keys with whitespace are allowed (not system keys)
        assert!(validate_client_key(" ").is_ok());
        assert!(validate_client_key("key with spaces").is_ok());
        assert!(validate_client_key("\t\n").is_ok());
    }

    #[test]
    fn test_validate_client_key_special_characters() {
        assert!(validate_client_key("key!@#$%^&*()").is_ok());
        assert!(validate_client_key("path/to/key").is_ok());
        assert!(validate_client_key("key=value").is_ok());
        assert!(validate_client_key("key?query").is_ok());
    }

    #[test]
    fn test_validate_client_key_long_key() {
        let long_key = "x".repeat(10000);
        assert!(validate_client_key(&long_key).is_ok());
    }

    #[test]
    fn test_validate_client_key_long_system_key() {
        let long_system_key = format!("{}{}", SYSTEM_PREFIX, "x".repeat(10000));
        assert!(validate_client_key(&long_system_key).is_err());
    }

    // =========================================================================
    // VaultError Tests
    // =========================================================================

    #[test]
    fn test_vault_error_debug() {
        let err = VaultError::SystemPrefixReserved;
        let debug = format!("{:?}", err);
        assert!(debug.contains("SystemPrefixReserved"));
    }

    #[test]
    fn test_vault_error_display() {
        let err = VaultError::SystemPrefixReserved;
        let display = format!("{}", err);
        assert!(display.contains("_system:"));
        assert!(display.contains("reserved"));
    }

    #[test]
    fn test_vault_error_clone() {
        let err1 = VaultError::SystemPrefixReserved;
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_vault_error_equality() {
        let err1 = VaultError::SystemPrefixReserved;
        let err2 = VaultError::SystemPrefixReserved;
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_vault_error_is_error_trait() {
        let err: Box<dyn std::error::Error> = Box::new(VaultError::SystemPrefixReserved);
        assert!(err.to_string().contains("reserved"));
    }
}
