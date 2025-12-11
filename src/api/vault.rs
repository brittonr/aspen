//! Vault types and operations for organizing key-value data.
//!
//! Vaults provide logical namespacing for keys using a prefix convention:
//! `vault:<vault_name>:<key>`. This allows grouping related data without
//! requiring schema changes to the underlying storage.
//!
//! # Key Format
//!
//! ```text
//! vault:config:app_name     -> vault="config", key="app_name"
//! vault:sessions:user123    -> vault="sessions", key="user123"
//! vault:metrics:cpu_usage   -> vault="metrics", key="cpu_usage"
//! ```
//!
//! # Tiger Style
//!
//! - Fixed prefix format prevents key injection
//! - Bounded vault name length (MAX_VAULT_NAME_LENGTH = 64)
//! - UTF-8 safe names (no colons allowed in vault names)

use serde::{Deserialize, Serialize};

/// Maximum length for vault names (bytes).
/// Tiger Style: Fixed limit prevents memory exhaustion.
pub const MAX_VAULT_NAME_LENGTH: usize = 64;

/// Maximum number of vaults allowed in the system.
/// Tiger Style: Bounded resources prevent unbounded growth.
pub const MAX_VAULTS: usize = 1000;

/// Maximum number of keys allowed per vault.
/// Tiger Style: Prevent individual vaults from growing unbounded.
pub const MAX_KEYS_PER_VAULT: usize = 10000;

/// Maximum size of a single value (bytes).
/// Tiger Style: 1MB limit prevents memory exhaustion from large values.
pub const MAX_VALUE_SIZE: usize = 1_048_576;

/// Maximum number of results returned from a scan operation.
/// Tiger Style: Bounded iteration prevents unbounded resource use.
pub const MAX_SCAN_RESULTS: usize = 1000;

/// Vault key prefix used in storage.
pub const VAULT_PREFIX: &str = "vault:";

/// Separator between vault name and key.
pub const VAULT_KEY_SEPARATOR: char = ':';

/// Information about a vault.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultInfo {
    /// Name of the vault.
    pub name: String,
    /// Number of keys in the vault.
    pub key_count: u64,
    /// Optional description/metadata for the vault.
    pub description: Option<String>,
}

/// A key-value pair within a vault context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultKeyValue {
    /// The key (without vault prefix).
    pub key: String,
    /// The value.
    pub value: String,
}

/// Request to list vaults.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VaultListRequest {
    /// Optional prefix filter for vault names.
    pub prefix: Option<String>,
    /// Maximum number of vaults to return (default: 100).
    pub limit: Option<u32>,
}

/// Response containing vault list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultListResponse {
    /// List of vaults.
    pub vaults: Vec<VaultInfo>,
    /// Total count of vaults (may be more than returned if limit applied).
    pub total_count: u64,
}

/// Request to list keys within a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultKeysRequest {
    /// Name of the vault.
    pub vault: String,
    /// Optional prefix filter for keys within the vault.
    pub prefix: Option<String>,
    /// Maximum number of keys to return (default: 100).
    pub limit: Option<u32>,
}

/// Response containing keys from a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultKeysResponse {
    /// Vault name.
    pub vault: String,
    /// List of key-value pairs.
    pub keys: Vec<VaultKeyValue>,
    /// Total count of keys in the vault.
    pub total_count: u64,
}

/// Request to write a key within a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultWriteRequest {
    /// Name of the vault.
    pub vault: String,
    /// Key within the vault (without prefix).
    pub key: String,
    /// Value to write.
    pub value: String,
}

/// Request to read a key from a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultReadRequest {
    /// Name of the vault.
    pub vault: String,
    /// Key within the vault (without prefix).
    pub key: String,
}

/// Request to delete a key from a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultDeleteRequest {
    /// Name of the vault.
    pub vault: String,
    /// Key within the vault (without prefix).
    pub key: String,
}

/// Construct the full storage key for a vault key.
///
/// # Examples
///
/// ```
/// use aspen::api::vault::make_vault_key;
/// assert_eq!(make_vault_key("config", "app_name"), "vault:config:app_name");
/// ```
pub fn make_vault_key(vault: &str, key: &str) -> String {
    format!("{}{}{}{}", VAULT_PREFIX, vault, VAULT_KEY_SEPARATOR, key)
}

/// Parse a storage key into vault name and key.
///
/// Returns `None` if the key doesn't have the vault prefix format.
///
/// # Examples
///
/// ```
/// use aspen::api::vault::parse_vault_key;
/// assert_eq!(parse_vault_key("vault:config:app_name"), Some(("config".to_string(), "app_name".to_string())));
/// assert_eq!(parse_vault_key("regular_key"), None);
/// ```
pub fn parse_vault_key(full_key: &str) -> Option<(String, String)> {
    if !full_key.starts_with(VAULT_PREFIX) {
        return None;
    }

    let rest = &full_key[VAULT_PREFIX.len()..];
    let separator_pos = rest.find(VAULT_KEY_SEPARATOR)?;

    let vault_name = &rest[..separator_pos];
    let key = &rest[separator_pos + 1..];

    Some((vault_name.to_string(), key.to_string()))
}

/// Validate a vault name.
///
/// # Rules
/// - Not empty
/// - Not longer than MAX_VAULT_NAME_LENGTH
/// - No colons (reserved as separator)
/// - Alphanumeric, underscore, hyphen only
pub fn validate_vault_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("vault name cannot be empty".to_string());
    }

    if name.len() > MAX_VAULT_NAME_LENGTH {
        return Err(format!(
            "vault name exceeds maximum length of {} bytes",
            MAX_VAULT_NAME_LENGTH
        ));
    }

    if name.contains(VAULT_KEY_SEPARATOR) {
        return Err("vault name cannot contain colons".to_string());
    }

    // Tiger Style: Explicit character validation
    for c in name.chars() {
        if !c.is_ascii_alphanumeric() && c != '_' && c != '-' {
            return Err(format!(
                "vault name contains invalid character '{}' - only alphanumeric, underscore, and hyphen allowed",
                c
            ));
        }
    }

    Ok(())
}

/// Construct a prefix for scanning all keys in a vault.
///
/// # Examples
///
/// ```
/// use aspen::api::vault::vault_scan_prefix;
/// assert_eq!(vault_scan_prefix("config"), "vault:config:");
/// ```
pub fn vault_scan_prefix(vault: &str) -> String {
    format!("{}{}{}", VAULT_PREFIX, vault, VAULT_KEY_SEPARATOR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_vault_key() {
        assert_eq!(
            make_vault_key("config", "app_name"),
            "vault:config:app_name"
        );
        assert_eq!(
            make_vault_key("sessions", "user123"),
            "vault:sessions:user123"
        );
    }

    #[test]
    fn test_parse_vault_key() {
        assert_eq!(
            parse_vault_key("vault:config:app_name"),
            Some(("config".to_string(), "app_name".to_string()))
        );
        assert_eq!(
            parse_vault_key("vault:my-vault:nested:key"),
            Some(("my-vault".to_string(), "nested:key".to_string()))
        );
        assert_eq!(parse_vault_key("regular_key"), None);
        assert_eq!(parse_vault_key("vault:"), None);
    }

    #[test]
    fn test_validate_vault_name() {
        assert!(validate_vault_name("config").is_ok());
        assert!(validate_vault_name("my-vault").is_ok());
        assert!(validate_vault_name("my_vault").is_ok());
        assert!(validate_vault_name("vault123").is_ok());

        assert!(validate_vault_name("").is_err());
        assert!(validate_vault_name("has:colon").is_err());
        assert!(validate_vault_name("has space").is_err());
        assert!(validate_vault_name(&"a".repeat(65)).is_err());
    }

    #[test]
    fn test_vault_scan_prefix() {
        assert_eq!(vault_scan_prefix("config"), "vault:config:");
    }
}
