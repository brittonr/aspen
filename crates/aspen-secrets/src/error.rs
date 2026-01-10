//! Error types for secrets management.
//!
//! All errors include actionable hints to help operators diagnose issues.

use std::path::PathBuf;

use thiserror::Error;

/// Errors that can occur during secrets management operations.
#[derive(Debug, Error)]
pub enum SecretsError {
    // ========================================================================
    // SOPS / Age Decryption Errors
    // ========================================================================
    /// Failed to read secrets file.
    #[error(
        "failed to read secrets file at {path}: {source}\n\
         Hint: Ensure the file exists and is readable"
    )]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Secrets file is too large.
    #[error(
        "secrets file at {path} is too large: {size} bytes (max: {max} bytes)\n\
         Hint: Split secrets across multiple files or reduce secret count"
    )]
    FileTooLarge { path: PathBuf, size: usize, max: usize },

    /// Failed to parse secrets file format.
    #[error(
        "failed to parse secrets file at {path}: {reason}\n\
         Hint: Ensure the file is valid TOML format"
    )]
    ParseFile { path: PathBuf, reason: String },

    /// Failed to load age identity.
    #[error(
        "failed to load age identity from {path}: {reason}\n\
         Hint: Generate with: age-keygen -o {path}\n\
         Or set SOPS_AGE_KEY environment variable"
    )]
    LoadIdentity { path: PathBuf, reason: String },

    /// Age identity not found.
    #[error(
        "age identity not found at {path}\n\
         Hint: Generate with: age-keygen -o {path}\n\
         Or set SOPS_AGE_KEY environment variable with the age secret key"
    )]
    IdentityNotFound { path: PathBuf },

    /// Failed to parse age identity.
    #[error(
        "invalid age identity format: {reason}\n\
         Hint: Age identity should start with 'AGE-SECRET-KEY-1'"
    )]
    InvalidIdentity { reason: String },

    /// Failed to decrypt secrets.
    #[error(
        "failed to decrypt secrets: {reason}\n\
         Hint: Check that your age identity matches the encryption recipients.\n\
         Try: SOPS_AGE_KEY=$(cat ~/.config/sops/age/keys.txt) aspen-node ..."
    )]
    Decryption { reason: String },

    /// SOPS metadata missing or invalid.
    #[error(
        "SOPS metadata missing or invalid in secrets file: {reason}\n\
         Hint: Ensure the file was encrypted with SOPS:\n\
         sops --encrypt --age <public-key> secrets.toml > secrets.sops.toml"
    )]
    SopsMetadata { reason: String },

    // ========================================================================
    // Secret Access Errors
    // ========================================================================
    /// Secret not found.
    #[error("secret not found: {key}")]
    SecretNotFound { key: String },

    /// Failed to decode secret value.
    #[error(
        "failed to decode secret '{key}': {reason}\n\
         Hint: Check the encoding format (hex, base64, or plain text)"
    )]
    DecodeSecret { key: String, reason: String },

    /// Failed to parse trusted root key.
    #[error(
        "failed to parse trusted root key at index {index}: {reason}\n\
         Hint: Trusted roots should be 64-character hex-encoded Ed25519 public keys"
    )]
    ParseTrustedRoot { index: usize, reason: String },

    /// Failed to parse signing key.
    #[error(
        "failed to parse signing key: {reason}\n\
         Hint: Signing key should be a 64-character hex-encoded Ed25519 secret key"
    )]
    ParseSigningKey { reason: String },

    /// Failed to parse capability token.
    #[error(
        "failed to parse capability token '{name}': {reason}\n\
         Hint: Tokens should be base64-encoded CapabilityToken structs"
    )]
    ParseToken { name: String, reason: String },

    // ========================================================================
    // KV v2 Errors
    // ========================================================================
    /// Secret path too long.
    #[error("secret path too long: {length} characters (max: {max})")]
    PathTooLong { length: usize, max: usize },

    /// Secret value too large.
    #[error("secret value too large: {size} bytes (max: {max})")]
    ValueTooLarge { size: usize, max: usize },

    /// Too many versions.
    #[error("too many versions for secret: {count} (max: {max})")]
    TooManyVersions { count: u32, max: u32 },

    /// Version not found.
    #[error("version {version} not found for secret '{path}'")]
    VersionNotFound { path: String, version: u64 },

    /// Version already destroyed.
    #[error("version {version} of secret '{path}' has been destroyed")]
    VersionDestroyed { path: String, version: u64 },

    /// Check-and-set failed.
    #[error("check-and-set failed for secret '{path}': expected version {expected}, found {actual}")]
    CasFailed { path: String, expected: u64, actual: u64 },

    // ========================================================================
    // Transit Errors
    // ========================================================================
    /// Transit key not found.
    #[error("transit key not found: {name}")]
    TransitKeyNotFound { name: String },

    /// Transit key already exists.
    #[error("transit key already exists: {name}")]
    TransitKeyExists { name: String },

    /// Transit key name too long.
    #[error("transit key name too long: {length} characters (max: {max})")]
    TransitKeyNameTooLong { length: usize, max: usize },

    /// Plaintext too large for encryption.
    #[error("plaintext too large: {size} bytes (max: {max})")]
    PlaintextTooLarge { size: usize, max: usize },

    /// Invalid ciphertext format.
    #[error(
        "invalid ciphertext format: {reason}\n\
         Hint: Ciphertext should be in format 'aspen:v<version>:<base64-data>'"
    )]
    InvalidCiphertext { reason: String },

    /// Key version too old for decryption.
    #[error("key version {version} is below minimum decryption version {min_version} for key '{name}'")]
    KeyVersionTooOld {
        name: String,
        version: u32,
        min_version: u32,
    },

    /// Key deletion not allowed.
    #[error(
        "deletion not allowed for key '{name}'\n\
         Hint: Enable deletion with: aspen-cli transit update-key {name} --deletion-allowed=true"
    )]
    KeyDeletionNotAllowed { name: String },

    /// Key export not allowed.
    #[error(
        "export not allowed for key '{name}'\n\
         Hint: Key must be created with exportable=true to be exported"
    )]
    KeyExportNotAllowed { name: String },

    /// Unsupported key type.
    #[error("unsupported key type: {key_type}")]
    UnsupportedKeyType { key_type: String },

    /// Signature verification failed.
    #[error("signature verification failed for key '{name}'")]
    SignatureVerificationFailed { name: String },

    // ========================================================================
    // PKI Errors
    // ========================================================================
    /// CA not initialized.
    #[error(
        "certificate authority not initialized\n\
         Hint: Initialize with: aspen-cli pki generate-root ..."
    )]
    CaNotInitialized,

    /// CA already initialized.
    #[error("certificate authority already initialized for mount '{mount}'")]
    CaAlreadyInitialized { mount: String },

    /// Role not found.
    #[error("PKI role not found: {name}")]
    RoleNotFound { name: String },

    /// Role already exists.
    #[error("PKI role already exists: {name}")]
    RoleExists { name: String },

    /// Certificate not found.
    #[error("certificate not found: serial {serial}")]
    CertificateNotFound { serial: String },

    /// Certificate already revoked.
    #[error("certificate already revoked: serial {serial}")]
    CertificateAlreadyRevoked { serial: String },

    /// Common name not allowed by role.
    #[error(
        "common name '{cn}' not allowed by role '{role}'\n\
         Hint: Check allowed_domains in role configuration"
    )]
    CommonNameNotAllowed { cn: String, role: String },

    /// SAN not allowed by role.
    #[error(
        "SAN '{san}' not allowed by role '{role}'\n\
         Hint: Check allowed_uri_sans and allow_ip_sans in role configuration"
    )]
    SanNotAllowed { san: String, role: String },

    /// TTL exceeds maximum.
    #[error("requested TTL {requested_secs}s exceeds maximum {max_secs}s for role '{role}'")]
    TtlExceedsMax {
        role: String,
        requested_secs: u64,
        max_secs: u64,
    },

    /// Too many SANs.
    #[error("too many SANs: {count} (max: {max})")]
    TooManySans { count: u32, max: u32 },

    /// Certificate generation failed.
    #[error("certificate generation failed: {reason}")]
    CertificateGeneration { reason: String },

    /// Invalid certificate format or content.
    #[error("invalid certificate: {reason}")]
    InvalidCertificate { reason: String },

    // ========================================================================
    // Storage Errors
    // ========================================================================
    /// KV store error.
    #[error("KV store error: {reason}")]
    KvStore { reason: String },

    /// Encryption error.
    #[error("encryption error: {reason}")]
    Encryption { reason: String },

    /// Serialization error.
    #[error("serialization error: {reason}")]
    Serialization { reason: String },

    // ========================================================================
    // General Errors
    // ========================================================================
    /// Mount not found.
    #[error("mount not found: {name}")]
    MountNotFound { name: String },

    /// Mount already exists.
    #[error("mount already exists: {name}")]
    MountExists { name: String },

    /// Too many mounts.
    #[error("too many mounts: {count} (max: {max})")]
    TooManyMounts { count: u32, max: u32 },

    /// Internal error.
    #[error("internal error: {reason}")]
    Internal { reason: String },
}

/// Result type for secrets operations.
pub type Result<T> = std::result::Result<T, SecretsError>;

impl SecretsError {
    /// Check if this error indicates a "not found" condition.
    pub fn is_not_found(&self) -> bool {
        matches!(
            self,
            SecretsError::SecretNotFound { .. }
                | SecretsError::VersionNotFound { .. }
                | SecretsError::TransitKeyNotFound { .. }
                | SecretsError::RoleNotFound { .. }
                | SecretsError::CertificateNotFound { .. }
                | SecretsError::MountNotFound { .. }
                | SecretsError::IdentityNotFound { .. }
        )
    }

    /// Check if this error indicates a conflict/already exists condition.
    pub fn is_conflict(&self) -> bool {
        matches!(
            self,
            SecretsError::TransitKeyExists { .. }
                | SecretsError::RoleExists { .. }
                | SecretsError::CaAlreadyInitialized { .. }
                | SecretsError::MountExists { .. }
                | SecretsError::CertificateAlreadyRevoked { .. }
        )
    }

    /// Check if this error indicates an authorization failure.
    pub fn is_forbidden(&self) -> bool {
        matches!(
            self,
            SecretsError::KeyDeletionNotAllowed { .. }
                | SecretsError::KeyExportNotAllowed { .. }
                | SecretsError::CommonNameNotAllowed { .. }
                | SecretsError::SanNotAllowed { .. }
        )
    }
}
