//! Configuration for SOPS-based secrets management.

use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::constants::DEFAULT_CACHE_TTL_SECS;
use crate::constants::SECRETS_SYSTEM_PREFIX;

/// Configuration for secrets management.
///
/// Secrets can be loaded from:
/// 1. SOPS-encrypted config files (decrypted at node startup)
/// 2. Aspen's distributed KV store (encrypted at rest)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfig {
    /// Enable SOPS-based secrets management.
    #[serde(default)]
    pub enabled: bool,

    /// Path to SOPS-encrypted secrets file (TOML format).
    ///
    /// This file is decrypted at node startup and provides:
    /// - Trusted root keys for token verification
    /// - Token signing keys
    /// - Pre-built capability tokens
    /// - Bootstrap secrets
    ///
    /// Example: `/etc/aspen/secrets.sops.toml`
    pub secrets_file: Option<PathBuf>,

    /// Path to age identity file for decryption.
    ///
    /// Default locations checked (in order):
    /// 1. This path (if set)
    /// 2. `$SOPS_AGE_KEY_FILE` environment variable
    /// 3. `$XDG_CONFIG_HOME/sops/age/keys.txt`
    /// 4. `$HOME/.config/sops/age/keys.txt`
    ///
    /// Generate with: `age-keygen -o /path/to/keys.txt`
    pub age_identity_file: Option<PathBuf>,

    /// Environment variable containing age identity.
    ///
    /// Takes precedence over `age_identity_file` if set.
    /// Standard SOPS convention is `SOPS_AGE_KEY`.
    #[serde(default = "default_age_identity_env")]
    pub age_identity_env: String,

    /// Prefix in Aspen KV store for runtime secrets.
    ///
    /// Secrets stored here are encrypted at rest and decrypted on access.
    /// Default: `_system:secrets:`
    #[serde(default = "default_kv_secrets_prefix")]
    pub kv_secrets_prefix: String,

    /// Cache decrypted secrets in memory.
    ///
    /// When enabled, decrypted secrets are cached to reduce KV reads.
    /// Set to `false` for highest security (re-decrypt each access).
    #[serde(default = "default_cache_enabled")]
    pub cache_enabled: bool,

    /// Cache TTL in seconds.
    ///
    /// After this time, cached secrets are evicted and re-fetched.
    /// 0 = no expiration (not recommended).
    /// Default: 300 (5 minutes)
    #[serde(default = "default_cache_ttl_secs")]
    pub cache_ttl_secs: u64,
}

fn default_age_identity_env() -> String {
    "SOPS_AGE_KEY".into()
}

fn default_kv_secrets_prefix() -> String {
    SECRETS_SYSTEM_PREFIX.into()
}

fn default_cache_enabled() -> bool {
    true
}

fn default_cache_ttl_secs() -> u64 {
    DEFAULT_CACHE_TTL_SECS
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            secrets_file: None,
            age_identity_file: None,
            age_identity_env: default_age_identity_env(),
            kv_secrets_prefix: default_kv_secrets_prefix(),
            cache_enabled: default_cache_enabled(),
            cache_ttl_secs: default_cache_ttl_secs(),
        }
    }
}

impl SecretsConfig {
    /// Create a new secrets config with the given secrets file.
    pub fn with_secrets_file(path: impl Into<PathBuf>) -> Self {
        Self {
            enabled: true,
            secrets_file: Some(path.into()),
            ..Default::default()
        }
    }

    /// Set the age identity file.
    pub fn with_age_identity_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.age_identity_file = Some(path.into());
        self
    }

    /// Set the age identity environment variable name.
    pub fn with_age_identity_env(mut self, env_var: impl Into<String>) -> Self {
        self.age_identity_env = env_var.into();
        self
    }

    /// Disable caching.
    pub fn without_cache(mut self) -> Self {
        self.cache_enabled = false;
        self
    }

    /// Set cache TTL.
    pub fn with_cache_ttl_secs(mut self, ttl: u64) -> Self {
        self.cache_ttl_secs = ttl;
        self
    }

    /// Check if secrets management is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled && self.secrets_file.is_some()
    }
}

/// Secrets file structure (SOPS TOML format).
///
/// This is the structure of the decrypted secrets file.
/// Before encryption, it looks like:
///
/// ```toml
/// trusted_roots = [
///     "a1b2c3d4...",  # 64 hex chars Ed25519 public key
/// ]
///
/// signing_key = "0123456789abcdef..."  # Ed25519 secret key
///
/// [tokens]
/// admin = "eyJ2ZXJzaW9..."  # base64 CapabilityToken
///
/// [secrets.strings]
/// api_key = "sk-live-xxxx"
///
/// [secrets.bytes]
/// tls_cert = "LS0tLS1CRUdJTi..."  # base64-encoded
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecretsFile {
    /// Trusted root public keys (hex-encoded Ed25519).
    ///
    /// These are the public keys that can issue root capability tokens.
    /// Tokens signed by these keys (or delegated from them) will be accepted.
    #[serde(default)]
    pub trusted_roots: Vec<String>,

    /// Token signing key (hex-encoded Ed25519 secret key).
    ///
    /// Used by this node to sign capability tokens.
    #[serde(default)]
    pub signing_key: Option<String>,

    /// Pre-built capability tokens by name.
    ///
    /// These are base64-encoded `CapabilityToken` structs that can be
    /// loaded at startup for service accounts.
    #[serde(default)]
    pub tokens: std::collections::HashMap<String, String>,

    /// String secrets by name.
    #[serde(default)]
    pub secrets: SecretsData,

    /// SOPS metadata (populated by sops encrypt).
    ///
    /// This field is present in encrypted files but ignored during parsing.
    #[serde(default)]
    pub sops: Option<SopsMetadata>,
}

/// String and binary secrets data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecretsData {
    /// String secrets (plaintext after decryption).
    #[serde(default)]
    pub strings: std::collections::HashMap<String, String>,

    /// Binary secrets (base64-encoded).
    #[serde(default)]
    pub bytes: std::collections::HashMap<String, String>,
}

/// SOPS metadata structure.
///
/// This is added by `sops encrypt` and contains information about
/// the encryption recipients and settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SopsMetadata {
    /// Age recipients.
    #[serde(default)]
    pub age: Vec<AgeRecipient>,

    /// Last modified timestamp.
    #[serde(default)]
    pub lastmodified: Option<String>,

    /// SOPS version.
    #[serde(default)]
    pub version: Option<String>,

    /// Encrypted regex pattern.
    #[serde(default)]
    pub encrypted_regex: Option<String>,

    /// MAC (Message Authentication Code).
    #[serde(default)]
    pub mac: Option<String>,
}

/// SOPS age recipient entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgeRecipient {
    /// Age public key (recipient).
    pub recipient: String,

    /// Encrypted data key (used by SOPS).
    #[serde(default)]
    pub enc: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SecretsConfig::default();
        assert!(!config.enabled);
        assert!(config.secrets_file.is_none());
        assert_eq!(config.age_identity_env, "SOPS_AGE_KEY");
        assert!(config.cache_enabled);
        assert_eq!(config.cache_ttl_secs, 300);
    }

    #[test]
    fn test_config_builder() {
        let config = SecretsConfig::with_secrets_file("/etc/aspen/secrets.sops.toml")
            .with_age_identity_file("/etc/aspen/age-key.txt")
            .with_cache_ttl_secs(600);

        assert!(config.enabled);
        assert!(config.secrets_file.is_some());
        assert!(config.age_identity_file.is_some());
        assert_eq!(config.cache_ttl_secs, 600);
    }

    #[test]
    fn test_parse_secrets_file() {
        let toml_str = r#"
            trusted_roots = ["a1b2c3d4"]
            signing_key = "0123456789abcdef"

            [tokens]
            admin = "eyJ2ZXJzaW9..."

            [secrets.strings]
            api_key = "sk-live-xxxx"

            [secrets.bytes]
            tls_cert = "LS0tLS1CRUdJTi..."
        "#;

        let secrets: SecretsFile = toml::from_str(toml_str).unwrap();
        assert_eq!(secrets.trusted_roots.len(), 1);
        assert!(secrets.signing_key.is_some());
        assert_eq!(secrets.tokens.len(), 1);
        assert_eq!(secrets.secrets.strings.len(), 1);
        assert_eq!(secrets.secrets.bytes.len(), 1);
    }
}
