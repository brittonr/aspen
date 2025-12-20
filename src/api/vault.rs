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
//! # Reserved Prefixes
//!
//! - `_system:` - Reserved for internal cluster data, rejected from client writes
//! - `vault:` - User data namespace with quota enforcement
//!
//! # Tiger Style
//!
//! - Fixed prefix format prevents key injection
//! - Bounded vault name length (MAX_VAULT_NAME_LENGTH = 64)
//! - UTF-8 safe names (no colons allowed in vault names)
//! - Quota enforcement: MAX_VAULTS, MAX_KEYS_PER_VAULT
//! - Global per-vault rate limiting

use serde::{Deserialize, Serialize};
use thiserror::Error;

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

/// Reserved prefix for internal system data.
/// Client writes to keys starting with this prefix are rejected.
pub const SYSTEM_PREFIX: &str = "_system:";

/// Prefix for vault metadata stored in system namespace.
/// Format: `_system:vault_meta:{vault_name}`
pub const VAULT_META_PREFIX: &str = "_system:vault_meta:";

/// Maximum vault writes per second (global per-vault).
/// Tiger Style: Rate limiting prevents resource exhaustion.
pub const MAX_VAULT_WRITES_PER_SECOND: u32 = 100;

/// Maximum vault reads per second (global per-vault).
pub const MAX_VAULT_READS_PER_SECOND: u32 = 1000;

/// Maximum number of vaults tracked in rate limiter LRU cache.
pub const MAX_RATE_LIMITER_VAULTS: usize = 1000;

/// Errors that can occur during vault operations.
///
/// Tiger Style: Errors include actionable context (which limit, what value).
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VaultError {
    /// Vault name is invalid (empty, too long, invalid characters).
    #[error("invalid vault name '{name}': {reason}")]
    InvalidVaultName { name: String, reason: String },

    /// Vault has exceeded its key quota.
    #[error("vault '{vault}' quota exceeded: limit is {limit} keys")]
    VaultQuotaExceeded { vault: String, limit: usize },

    /// Attempt to write to reserved system prefix.
    #[error("key prefix '_system:' is reserved for internal use")]
    SystemPrefixReserved,

    /// Maximum number of vaults exceeded.
    #[error("maximum vaults exceeded: limit is {limit}")]
    TooManyVaults { limit: usize },

    /// Rate limit exceeded for vault operations.
    #[error("rate limit exceeded for vault '{vault}': retry after {retry_after_ms}ms")]
    RateLimited { vault: String, retry_after_ms: u64 },
}

/// Metadata for a vault stored at `_system:vault_meta:{vault_name}`.
///
/// Used for quota enforcement with synchronous updates via SetMulti.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultMetadata {
    /// Number of keys currently in this vault.
    pub key_count: u64,
    /// Unix timestamp when the vault was created.
    pub created_at: u64,
}

impl VaultMetadata {
    /// Create new vault metadata with initial key count.
    pub fn new(key_count: u64, created_at: u64) -> Self {
        Self {
            key_count,
            created_at,
        }
    }

    /// Construct the storage key for this vault's metadata.
    pub fn storage_key(vault_name: &str) -> String {
        format!("{}{}", VAULT_META_PREFIX, vault_name)
    }
}

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

/// Check if a key uses the reserved system prefix.
///
/// Returns true if the key starts with `_system:`.
///
/// # Examples
///
/// ```
/// use aspen::api::vault::is_system_key;
/// assert!(is_system_key("_system:vault_meta:myapp"));
/// assert!(!is_system_key("vault:myapp:config"));
/// assert!(!is_system_key("regular_key"));
/// ```
pub fn is_system_key(key: &str) -> bool {
    key.starts_with(SYSTEM_PREFIX)
}

/// Check if a key is a vault key.
///
/// Returns true if the key starts with `vault:`.
///
/// # Examples
///
/// ```
/// use aspen::api::vault::is_vault_key;
/// assert!(is_vault_key("vault:myapp:config"));
/// assert!(!is_vault_key("_system:internal"));
/// assert!(!is_vault_key("regular_key"));
/// ```
pub fn is_vault_key(key: &str) -> bool {
    key.starts_with(VAULT_PREFIX)
}

/// Validate a key for client write operations.
///
/// Returns an error if:
/// - Key uses reserved `_system:` prefix
/// - Key uses `vault:` prefix but vault name is invalid
///
/// # Examples
///
/// ```
/// use aspen::api::vault::validate_client_key;
/// assert!(validate_client_key("vault:myapp:config").is_ok());
/// assert!(validate_client_key("regular_key").is_ok());
/// assert!(validate_client_key("_system:internal").is_err());
/// assert!(validate_client_key("vault:bad name:key").is_err());
/// ```
pub fn validate_client_key(key: &str) -> Result<(), VaultError> {
    // Reject system prefix
    if is_system_key(key) {
        return Err(VaultError::SystemPrefixReserved);
    }

    // Validate vault name if it's a vault key
    if let Some((vault_name, _)) = parse_vault_key(key)
        && let Err(reason) = validate_vault_name(&vault_name)
    {
        return Err(VaultError::InvalidVaultName {
            name: vault_name,
            reason,
        });
    }

    Ok(())
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

    #[test]
    fn test_is_system_key() {
        assert!(is_system_key("_system:vault_meta:myapp"));
        assert!(is_system_key("_system:internal"));
        assert!(!is_system_key("vault:myapp:config"));
        assert!(!is_system_key("regular_key"));
        assert!(!is_system_key(""));
    }

    #[test]
    fn test_is_vault_key() {
        assert!(is_vault_key("vault:myapp:config"));
        assert!(is_vault_key("vault:"));
        assert!(!is_vault_key("_system:internal"));
        assert!(!is_vault_key("regular_key"));
        assert!(!is_vault_key(""));
    }

    #[test]
    fn test_validate_client_key() {
        // Valid keys
        assert!(validate_client_key("vault:myapp:config").is_ok());
        assert!(validate_client_key("vault:my-app:nested:key").is_ok());
        assert!(validate_client_key("regular_key").is_ok());
        assert!(validate_client_key("any:other:format").is_ok());

        // System prefix rejected
        assert!(matches!(
            validate_client_key("_system:internal"),
            Err(VaultError::SystemPrefixReserved)
        ));
        assert!(matches!(
            validate_client_key("_system:vault_meta:myapp"),
            Err(VaultError::SystemPrefixReserved)
        ));

        // Invalid vault names
        assert!(matches!(
            validate_client_key("vault:bad name:key"),
            Err(VaultError::InvalidVaultName { .. })
        ));
        assert!(matches!(
            validate_client_key("vault::key"),
            Err(VaultError::InvalidVaultName { .. })
        ));
    }

    #[test]
    fn test_vault_metadata() {
        let meta = VaultMetadata::new(42, 1700000000);
        assert_eq!(meta.key_count, 42);
        assert_eq!(meta.created_at, 1700000000);

        assert_eq!(
            VaultMetadata::storage_key("myapp"),
            "_system:vault_meta:myapp"
        );
    }

    #[test]
    fn test_vault_error_display() {
        assert_eq!(
            VaultError::InvalidVaultName {
                name: "bad name".to_string(),
                reason: "contains space".to_string()
            }
            .to_string(),
            "invalid vault name 'bad name': contains space"
        );

        assert_eq!(
            VaultError::VaultQuotaExceeded {
                vault: "myapp".to_string(),
                limit: 10000
            }
            .to_string(),
            "vault 'myapp' quota exceeded: limit is 10000 keys"
        );

        assert_eq!(
            VaultError::SystemPrefixReserved.to_string(),
            "key prefix '_system:' is reserved for internal use"
        );

        assert_eq!(
            VaultError::TooManyVaults { limit: 1000 }.to_string(),
            "maximum vaults exceeded: limit is 1000"
        );

        assert_eq!(
            VaultError::RateLimited {
                vault: "myapp".to_string(),
                retry_after_ms: 500
            }
            .to_string(),
            "rate limit exceeded for vault 'myapp': retry after 500ms"
        );
    }
}
