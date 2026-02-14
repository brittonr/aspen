//! Secrets/vault operation types.
//!
//! Request/response types for Vault-compatible secrets management including
//! KV v2 secrets engine, Transit encryption, PKI certificates, and Nix cache signing.

use serde::Deserialize;
use serde::Serialize;

/// Secrets domain request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecretsRequest {
    // KV v2 Secrets Engine
    /// Read a secret from the KV v2 secrets engine.
    SecretsKvRead {
        mount: String,
        path: String,
        version: Option<u64>,
    },
    /// Write a secret to the KV v2 secrets engine.
    SecretsKvWrite {
        mount: String,
        path: String,
        data: std::collections::HashMap<String, String>,
        cas: Option<u64>,
    },
    /// Soft-delete secret versions.
    SecretsKvDelete {
        mount: String,
        path: String,
        versions: Vec<u64>,
    },
    /// Permanently destroy secret versions.
    SecretsKvDestroy {
        mount: String,
        path: String,
        versions: Vec<u64>,
    },
    /// Undelete soft-deleted secret versions.
    SecretsKvUndelete {
        mount: String,
        path: String,
        versions: Vec<u64>,
    },
    /// List secrets under a path prefix.
    SecretsKvList { mount: String, path: String },
    /// Get metadata for a secret.
    SecretsKvMetadata { mount: String, path: String },
    /// Update metadata for a secret.
    SecretsKvUpdateMetadata {
        mount: String,
        path: String,
        max_versions: Option<u32>,
        cas_required: Option<bool>,
        custom_metadata: Option<std::collections::HashMap<String, String>>,
    },
    /// Delete a secret and all its versions.
    SecretsKvDeleteMetadata { mount: String, path: String },

    // Transit Secrets Engine
    /// Create a new encryption key.
    SecretsTransitCreateKey {
        mount: String,
        name: String,
        key_type: String,
    },
    /// Encrypt data using a Transit key.
    SecretsTransitEncrypt {
        mount: String,
        name: String,
        plaintext: Vec<u8>,
        context: Option<Vec<u8>>,
    },
    /// Decrypt ciphertext using a Transit key.
    SecretsTransitDecrypt {
        mount: String,
        name: String,
        ciphertext: String,
        context: Option<Vec<u8>>,
    },
    /// Sign data using a Transit key.
    SecretsTransitSign { mount: String, name: String, data: Vec<u8> },
    /// Verify a signature using a Transit key.
    SecretsTransitVerify {
        mount: String,
        name: String,
        data: Vec<u8>,
        signature: String,
    },
    /// Rotate a Transit key to a new version.
    SecretsTransitRotateKey { mount: String, name: String },
    /// List all keys in the Transit engine.
    SecretsTransitListKeys { mount: String },
    /// Rewrap ciphertext with the latest key version.
    SecretsTransitRewrap {
        mount: String,
        name: String,
        ciphertext: String,
        context: Option<Vec<u8>>,
    },
    /// Generate a data key for envelope encryption.
    SecretsTransitDatakey {
        mount: String,
        name: String,
        key_type: String,
    },

    // PKI Secrets Engine
    /// Generate a root CA certificate.
    SecretsPkiGenerateRoot {
        mount: String,
        common_name: String,
        ttl_days: Option<u32>,
    },
    /// Generate an intermediate CA CSR.
    SecretsPkiGenerateIntermediate { mount: String, common_name: String },
    /// Set the signed intermediate certificate.
    SecretsPkiSetSignedIntermediate { mount: String, certificate: String },
    /// Create a role for certificate issuance.
    SecretsPkiCreateRole {
        mount: String,
        name: String,
        allowed_domains: Vec<String>,
        max_ttl_days: u32,
        allow_bare_domains: bool,
        allow_wildcards: bool,
        allow_subdomains: bool,
    },
    /// Issue a certificate using a role.
    SecretsPkiIssue {
        mount: String,
        role: String,
        common_name: String,
        alt_names: Vec<String>,
        ttl_days: Option<u32>,
    },
    /// Revoke a certificate.
    SecretsPkiRevoke { mount: String, serial: String },
    /// Get the Certificate Revocation List.
    SecretsPkiGetCrl { mount: String },
    /// List all issued certificates.
    SecretsPkiListCerts { mount: String },
    /// Get PKI role configuration.
    SecretsPkiGetRole { mount: String, name: String },
    /// List all PKI roles.
    SecretsPkiListRoles { mount: String },

    // Nix Cache Signing operations
    /// Create a signing key for a Nix cache.
    SecretsNixCacheCreateKey { mount: String, cache_name: String },
    /// Get the public key for a cache.
    SecretsNixCacheGetPublicKey { mount: String, cache_name: String },
    /// Rotate a cache signing key.
    SecretsNixCacheRotateKey { mount: String, cache_name: String },
    /// Delete a cache signing key.
    SecretsNixCacheDeleteKey { mount: String, cache_name: String },
    /// List all cache signing keys.
    SecretsNixCacheListKeys { mount: String },
}

impl SecretsRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            // KV v2 read operations
            Self::SecretsKvRead { mount, path, .. }
            | Self::SecretsKvList { mount, path }
            | Self::SecretsKvMetadata { mount, path } => Some(Operation::Read {
                key: format!("_secrets:{}:{}", mount, path),
            }),
            // KV v2 write operations
            Self::SecretsKvWrite { mount, path, .. }
            | Self::SecretsKvDelete { mount, path, .. }
            | Self::SecretsKvDestroy { mount, path, .. }
            | Self::SecretsKvUndelete { mount, path, .. }
            | Self::SecretsKvUpdateMetadata { mount, path, .. }
            | Self::SecretsKvDeleteMetadata { mount, path } => Some(Operation::Write {
                key: format!("_secrets:{}:{}", mount, path),
                value: vec![],
            }),

            // Transit read operations
            Self::SecretsTransitListKeys { mount } => Some(Operation::Read {
                key: format!("_secrets:{}:", mount),
            }),
            Self::SecretsTransitCreateKey { mount, name, .. } | Self::SecretsTransitRotateKey { mount, name } => {
                Some(Operation::Write {
                    key: format!("_secrets:{}:{}", mount, name),
                    value: vec![],
                })
            }
            Self::SecretsTransitEncrypt { mount, name, .. }
            | Self::SecretsTransitDecrypt { mount, name, .. }
            | Self::SecretsTransitSign { mount, name, .. }
            | Self::SecretsTransitVerify { mount, name, .. }
            | Self::SecretsTransitRewrap { mount, name, .. }
            | Self::SecretsTransitDatakey { mount, name, .. } => Some(Operation::Read {
                key: format!("_secrets:{}:{}", mount, name),
            }),

            // PKI read operations
            Self::SecretsPkiGetCrl { mount }
            | Self::SecretsPkiListCerts { mount }
            | Self::SecretsPkiListRoles { mount } => Some(Operation::Read {
                key: format!("_secrets:{}:", mount),
            }),
            Self::SecretsPkiGetRole { mount, name } => Some(Operation::Read {
                key: format!("_secrets:{}:{}", mount, name),
            }),
            // PKI write operations
            Self::SecretsPkiGenerateRoot { mount, .. }
            | Self::SecretsPkiGenerateIntermediate { mount, .. }
            | Self::SecretsPkiSetSignedIntermediate { mount, .. }
            | Self::SecretsPkiCreateRole { mount, .. }
            | Self::SecretsPkiIssue { mount, .. }
            | Self::SecretsPkiRevoke { mount, .. } => Some(Operation::Write {
                key: format!("_secrets:{}:", mount),
                value: vec![],
            }),

            // Nix cache signing key operations
            Self::SecretsNixCacheGetPublicKey { mount, .. } => Some(Operation::Read {
                key: format!("_secrets:{}:", mount),
            }),
            Self::SecretsNixCacheListKeys { mount } => Some(Operation::Read {
                key: format!("_secrets:{}:", mount),
            }),
            Self::SecretsNixCacheCreateKey { mount, .. }
            | Self::SecretsNixCacheRotateKey { mount, .. }
            | Self::SecretsNixCacheDeleteKey { mount, .. } => Some(Operation::Write {
                key: format!("_secrets:{}:", mount),
                value: vec![],
            }),
        }
    }
}

// =============================================================================
// Secrets KV v2 Response Types
// =============================================================================

/// Version metadata for a KV secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsKvVersionMetadata {
    /// Version number.
    pub version: u64,
    /// Creation time (Unix timestamp in milliseconds).
    pub created_time_unix_ms: u64,
    /// Deletion time if soft-deleted (Unix timestamp in milliseconds).
    pub deletion_time_unix_ms: Option<u64>,
    /// Whether this version has been permanently destroyed.
    pub destroyed: bool,
}

/// Secrets KV read result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsKvReadResultResponse {
    /// Whether the read was successful.
    pub success: bool,
    /// Secret data (key-value pairs).
    pub data: Option<std::collections::HashMap<String, String>>,
    /// Version metadata.
    pub metadata: Option<SecretsKvVersionMetadata>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets KV write result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsKvWriteResultResponse {
    /// Whether the write was successful.
    pub success: bool,
    /// Version number of the written secret.
    pub version: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets KV delete/destroy/undelete result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsKvDeleteResultResponse {
    /// Whether the operation was successful.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets KV list result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsKvListResultResponse {
    /// Whether the list was successful.
    pub success: bool,
    /// Secret keys (names only, not values).
    pub keys: Vec<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Version info in metadata response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsKvVersionInfo {
    /// Version number.
    pub version: u64,
    /// Creation time (Unix timestamp in milliseconds).
    pub created_time_unix_ms: u64,
    /// Whether this version is deleted.
    pub deleted: bool,
    /// Whether this version is destroyed.
    pub destroyed: bool,
}

/// Secrets KV metadata result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsKvMetadataResultResponse {
    /// Whether the read was successful.
    pub success: bool,
    /// Current version number.
    pub current_version: Option<u64>,
    /// Maximum versions to retain.
    pub max_versions: Option<u32>,
    /// Whether CAS is required for writes.
    pub cas_required: Option<bool>,
    /// Creation time (Unix timestamp in milliseconds).
    pub created_time_unix_ms: Option<u64>,
    /// Last update time (Unix timestamp in milliseconds).
    pub updated_time_unix_ms: Option<u64>,
    /// All version info.
    pub versions: Vec<SecretsKvVersionInfo>,
    /// Custom metadata.
    pub custom_metadata: Option<std::collections::HashMap<String, String>>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Secrets Transit Response Types
// =============================================================================

/// Secrets Transit encrypt result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsTransitEncryptResultResponse {
    /// Whether the encryption was successful.
    pub success: bool,
    /// Ciphertext (prefixed with key version).
    pub ciphertext: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets Transit decrypt result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsTransitDecryptResultResponse {
    /// Whether the decryption was successful.
    pub success: bool,
    /// Decrypted plaintext.
    pub plaintext: Option<Vec<u8>>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets Transit sign result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsTransitSignResultResponse {
    /// Whether the signing was successful.
    pub success: bool,
    /// Signature (prefixed with key version).
    pub signature: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets Transit verify result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsTransitVerifyResultResponse {
    /// Whether the verification request was successful.
    pub success: bool,
    /// Whether the signature is valid.
    pub valid: Option<bool>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets Transit datakey result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsTransitDatakeyResultResponse {
    /// Whether the operation was successful.
    pub success: bool,
    /// Plaintext data key (if requested).
    pub plaintext: Option<Vec<u8>>,
    /// Encrypted/wrapped data key.
    pub ciphertext: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets Transit key operation result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsTransitKeyResultResponse {
    /// Whether the operation was successful.
    pub success: bool,
    /// Key name.
    pub name: Option<String>,
    /// Current key version.
    pub version: Option<u64>,
    /// Key type.
    pub key_type: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets Transit list keys result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsTransitListResultResponse {
    /// Whether the list was successful.
    pub success: bool,
    /// Key names.
    pub keys: Vec<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Secrets PKI Response Types
// =============================================================================

/// Secrets PKI certificate result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsPkiCertificateResultResponse {
    /// Whether the operation was successful.
    pub success: bool,
    /// Certificate in PEM format.
    pub certificate: Option<String>,
    /// Private key in PEM format (only for issued certs, not for get operations).
    pub private_key: Option<String>,
    /// Certificate serial number.
    pub serial: Option<String>,
    /// Certificate signing request (for intermediate CA).
    pub csr: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets PKI revoke result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsPkiRevokeResultResponse {
    /// Whether the revocation was successful.
    pub success: bool,
    /// Revoked serial number.
    pub serial: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets PKI CRL result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsPkiCrlResultResponse {
    /// Whether the request was successful.
    pub success: bool,
    /// CRL in PEM format.
    pub crl: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets PKI list result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsPkiListResultResponse {
    /// Whether the list was successful.
    pub success: bool,
    /// Serial numbers (for certs) or role names (for roles).
    pub items: Vec<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets PKI role configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsPkiRoleConfig {
    /// Role name.
    pub name: String,
    /// Allowed domain patterns.
    pub allowed_domains: Vec<String>,
    /// Maximum TTL in days.
    pub max_ttl_days: u32,
    /// Allow bare domains.
    pub allow_bare_domains: bool,
    /// Allow wildcard certificates.
    pub allow_wildcards: bool,
}

/// Secrets PKI role result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsPkiRoleResultResponse {
    /// Whether the request was successful.
    pub success: bool,
    /// Role configuration.
    pub role: Option<SecretsPkiRoleConfig>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Nix Cache Signing Key Response Types
// =============================================================================

/// Nix cache signing key operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsNixCacheKeyResultResponse {
    /// Whether the operation was successful.
    pub success: bool,
    /// Public key in Nix format ("{cache_name}:{base64_key}").
    pub public_key: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Nix cache signing key deletion result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsNixCacheDeleteResultResponse {
    /// Whether the deletion was successful.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Nix cache signing keys list result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsNixCacheListResultResponse {
    /// Whether the operation was successful.
    pub success: bool,
    /// List of cache names that have signing keys.
    pub cache_names: Option<Vec<String>>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Legacy Vault Types (for backwards compatibility)
// =============================================================================

/// List vaults response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultListResponse {
    /// All vaults.
    pub vaults: Vec<VaultInfo>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Information about a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultInfo {
    /// Vault name.
    pub name: String,
    /// Number of keys in vault.
    pub key_count: u64,
}

/// Vault keys response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultKeysResponse {
    /// Vault name.
    pub vault: String,
    /// Keys in the vault.
    pub keys: Vec<VaultKeyValue>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Key-value pair within a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultKeyValue {
    /// Key name (without vault prefix).
    pub key: String,
    /// Value.
    pub value: String,
}
