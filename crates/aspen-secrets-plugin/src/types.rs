//! Internal types for the secrets plugin.

use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

// =============================================================================
// KV Storage Types
// =============================================================================

/// Versioned secret stored in KV.
///
/// Key format: `__secrets:kv:{mount}:data:{path}:v{version}`
/// Metadata key: `__secrets:kv:{mount}:meta:{path}`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedSecret {
    /// Secret data (key-value pairs).
    pub data: HashMap<String, String>,

    /// Version number.
    pub version: u64,

    /// Creation time (Unix timestamp in milliseconds).
    pub created_time_ms: u64,

    /// Whether this version has been soft-deleted.
    pub deleted: bool,

    /// Whether this version has been permanently destroyed.
    pub destroyed: bool,

    /// Deletion time if soft-deleted (Unix timestamp in milliseconds).
    pub deletion_time_ms: Option<u64>,
}

/// Secret path metadata stored in KV.
///
/// Key format: `__secrets:kv:{mount}:meta:{path}`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretPathMetadata {
    /// Current (latest) version number.
    pub current_version: u64,

    /// Maximum versions to retain.
    pub max_versions: u32,

    /// Whether CAS is required for writes.
    pub cas_required: bool,

    /// Creation time (Unix timestamp in milliseconds).
    pub created_time_ms: u64,

    /// Last update time (Unix timestamp in milliseconds).
    pub updated_time_ms: u64,

    /// Custom metadata key-value pairs.
    pub custom_metadata: Option<HashMap<String, String>>,
}

impl SecretPathMetadata {
    /// Create new metadata for a fresh secret path.
    pub fn new(now_ms: u64) -> Self {
        Self {
            current_version: 0,
            max_versions: DEFAULT_MAX_VERSIONS,
            cas_required: false,
            created_time_ms: now_ms,
            updated_time_ms: now_ms,
            custom_metadata: None,
        }
    }
}

// =============================================================================
// Transit Storage Types
// =============================================================================

/// Transit key stored in KV.
///
/// Key format: `__secrets:transit:{mount}:key:{name}`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitKeyEntry {
    /// Key name.
    pub name: String,

    /// Key type (e.g., "aes256-gcm", "ed25519").
    pub key_type: String,

    /// Current version number.
    pub current_version: u64,

    /// Key material per version (version -> base64-encoded key bytes).
    pub versions: HashMap<u64, String>,

    /// Whether deletion is allowed.
    pub allow_deletion: bool,

    /// Whether export is allowed.
    pub allow_export: bool,

    /// Minimum decryption version (versions below this cannot decrypt).
    pub min_decryption_version: u64,

    /// Creation time (Unix timestamp in milliseconds).
    pub created_time_ms: u64,
}

impl TransitKeyEntry {
    /// Create a new transit key entry.
    pub fn new(name: String, key_type: String, key_material_b64: String, now_ms: u64) -> Self {
        let mut versions = HashMap::new();
        versions.insert(1, key_material_b64);

        Self {
            name,
            key_type,
            current_version: 1,
            versions,
            allow_deletion: false,
            allow_export: false,
            min_decryption_version: 1,
            created_time_ms: now_ms,
        }
    }
}

// =============================================================================
// Constants
// =============================================================================

/// Default number of versions to keep per secret.
pub const DEFAULT_MAX_VERSIONS: u32 = 10;

/// Maximum secret path length.
pub const MAX_SECRET_PATH_LENGTH: usize = 512;

/// Maximum KV pairs per secret.
pub const MAX_KV_PAIRS_PER_SECRET: usize = 100;

/// Maximum transit key name length.
pub const MAX_TRANSIT_KEY_NAME_LENGTH: usize = 128;

/// Maximum plaintext size for transit encrypt (32 KB).
pub const MAX_PLAINTEXT_SIZE: usize = 32 * 1024;

/// Ciphertext wire format prefix.
pub const TRANSIT_CIPHERTEXT_PREFIX: &str = "aspen:v";

/// Maximum mount name length.
pub const MAX_MOUNT_NAME_LENGTH: usize = 64;

/// XChaCha20-Poly1305 nonce size (24 bytes).
pub const XCHACHA_NONCE_SIZE: usize = 24;

/// XChaCha20-Poly1305 key size (32 bytes).
pub const XCHACHA_KEY_SIZE: usize = 32;

/// Ed25519 secret key size (32 bytes).
pub const ED25519_SECRET_KEY_SIZE: usize = 32;

/// Ed25519 public key size (32 bytes).
pub const _ED25519_PUBLIC_KEY_SIZE: usize = 32;

/// Ed25519 signature size (64 bytes).
pub const _ED25519_SIGNATURE_SIZE: usize = 64;

// =============================================================================
// Key Formatting
// =============================================================================

/// Build the KV key for a versioned secret's data.
pub fn secret_data_key(mount: &str, path: &str, version: u64) -> String {
    format!("__secrets:kv:{}:data:{}:v{}", mount, path, version)
}

/// Build the KV key for a secret's path metadata.
pub fn secret_meta_key(mount: &str, path: &str) -> String {
    format!("__secrets:kv:{}:meta:{}", mount, path)
}

/// Build the KV key prefix for scanning all versions of a secret.
pub fn secret_data_prefix(mount: &str, path: &str) -> String {
    format!("__secrets:kv:{}:data:{}:v", mount, path)
}

/// Build the KV key prefix for scanning secrets under a path.
pub fn secret_list_prefix(mount: &str, path: &str) -> String {
    format!("__secrets:kv:{}:meta:{}", mount, path)
}

/// Build the KV key for a transit key entry.
pub fn transit_key_key(mount: &str, name: &str) -> String {
    format!("__secrets:transit:{}:key:{}", mount, name)
}

/// Build the KV key prefix for scanning all transit keys.
pub fn transit_key_prefix(mount: &str) -> String {
    format!("__secrets:transit:{}:key:", mount)
}

/// Validate a mount name.
pub fn validate_mount(mount: &str) -> Result<(), String> {
    if mount.is_empty() {
        return Err("mount name cannot be empty".to_string());
    }
    if mount.len() > MAX_MOUNT_NAME_LENGTH {
        return Err(format!("mount name too long: {} (max {})", mount.len(), MAX_MOUNT_NAME_LENGTH));
    }
    if !mount.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("mount name contains invalid characters".to_string());
    }
    Ok(())
}

/// Validate a secret path.
pub fn validate_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("secret path cannot be empty".to_string());
    }
    if path.len() > MAX_SECRET_PATH_LENGTH {
        return Err(format!("secret path too long: {} (max {})", path.len(), MAX_SECRET_PATH_LENGTH));
    }
    Ok(())
}
