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
    fn test_system_prefix_value() {
        assert_eq!(SYSTEM_PREFIX, "_system:");
    }

    #[test]
    fn test_is_system_key() {
        assert!(is_system_key("_system:metadata"));
        assert!(is_system_key("_system:internal"));
        assert!(is_system_key("_system:"));
        assert!(!is_system_key("myapp/config"));
        assert!(!is_system_key("regular_key"));
        assert!(!is_system_key(""));
    }

    #[test]
    fn test_is_system_key_case_sensitive() {
        assert!(!is_system_key("_SYSTEM:key"));
        assert!(!is_system_key("_System:key"));
    }

    #[test]
    fn test_is_system_key_similar_but_not_prefix() {
        assert!(!is_system_key("_systemkey"));
        assert!(!is_system_key("system:key"));
        assert!(!is_system_key("__system:key"));
        assert!(!is_system_key(" _system:key"));
    }

    #[test]
    fn test_validate_client_key() {
        assert!(validate_client_key("myapp/config").is_ok());
        assert!(validate_client_key("regular_key").is_ok());
        assert!(validate_client_key("").is_ok());
        assert!(matches!(validate_client_key("_system:internal"), Err(VaultError::SystemPrefixReserved)));
    }

    #[test]
    fn test_vault_error_display() {
        let err = VaultError::SystemPrefixReserved;
        assert!(format!("{}", err).contains("reserved"));
    }
}
