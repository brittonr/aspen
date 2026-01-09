//! Secrets provider trait and implementations.
//!
//! Provides a unified interface for accessing secrets from various sources:
//! - SOPS-encrypted config files (bootstrap)
//! - Aspen KV store (runtime)

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use aspen_auth::CapabilityToken;
use aspen_core::kv::ReadRequest;
use async_trait::async_trait;
use iroh::PublicKey;
use iroh::SecretKey;
use tokio::sync::RwLock;
use tracing::debug;
use tracing::warn;

use crate::constants::MAX_PREBUILT_TOKENS;
use crate::constants::MAX_TRUSTED_ROOTS;
use crate::error::Result;
use crate::error::SecretsError;
use crate::sops::config::SecretsConfig;
use crate::sops::config::SecretsFile;

/// Provider for decrypted secrets.
///
/// Implementations must be thread-safe as they may be accessed
/// concurrently from multiple handlers.
#[async_trait]
pub trait SecretsProvider: Send + Sync {
    /// Get a decrypted string secret by key.
    async fn get_string(&self, key: &str) -> Result<String>;

    /// Get a decrypted binary secret by key.
    async fn get_bytes(&self, key: &str) -> Result<Vec<u8>>;

    /// Get trusted root public keys for TokenVerifier.
    async fn get_trusted_roots(&self) -> Result<Vec<PublicKey>>;

    /// Get token signing key (for TokenBuilder).
    async fn get_signing_key(&self) -> Result<SecretKey>;

    /// Get pre-built capability token by name.
    async fn get_token(&self, name: &str) -> Result<CapabilityToken>;

    /// List available pre-built token names.
    async fn list_tokens(&self) -> Result<Vec<String>>;

    /// Check if a secret exists.
    async fn exists(&self, key: &str) -> Result<bool>;
}

/// Secrets manager combining file-based and KV-based secrets.
///
/// File-based secrets are loaded at startup from a SOPS-encrypted file.
/// KV-based secrets are fetched on demand (with optional caching).
pub struct SecretsManager {
    /// Configuration.
    config: SecretsConfig,
    /// Decrypted file secrets (loaded at startup).
    file_secrets: SecretsFile,
    /// Parsed trusted roots (cached).
    trusted_roots: Vec<PublicKey>,
    /// Parsed signing key (cached).
    signing_key: Option<SecretKey>,
    /// Parsed tokens (cached).
    tokens: HashMap<String, CapabilityToken>,
    /// In-memory cache for runtime secrets.
    cache: RwLock<SecretCache>,
    /// Optional KV store for runtime secrets.
    kv_store: Option<Arc<dyn aspen_core::KeyValueStore>>,
}

/// Cache entry for decrypted secrets.
struct CacheEntry {
    value: Vec<u8>,
    expires_at: Option<Instant>,
}

/// In-memory cache for secrets.
struct SecretCache {
    entries: HashMap<String, CacheEntry>,
}

impl SecretCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn get(&self, key: &str) -> Option<&[u8]> {
        self.entries.get(key).and_then(|entry| {
            if let Some(expires_at) = entry.expires_at {
                if Instant::now() > expires_at {
                    return None;
                }
            }
            Some(entry.value.as_slice())
        })
    }

    fn insert(&mut self, key: String, value: Vec<u8>, ttl: Option<Duration>) {
        let expires_at = ttl.map(|d| Instant::now() + d);
        self.entries.insert(key, CacheEntry { value, expires_at });
    }

    fn clear_expired(&mut self) {
        let now = Instant::now();
        self.entries.retain(|_, entry| entry.expires_at.map_or(true, |expires| now <= expires));
    }
}

impl SecretsManager {
    /// Create a new secrets manager from decrypted secrets.
    ///
    /// The secrets should already be decrypted by the caller.
    pub fn new(config: SecretsConfig, file_secrets: SecretsFile) -> Result<Self> {
        // Parse trusted roots
        let mut trusted_roots = Vec::with_capacity(file_secrets.trusted_roots.len());
        for (i, root_hex) in file_secrets.trusted_roots.iter().enumerate() {
            if i >= MAX_TRUSTED_ROOTS {
                warn!(max = MAX_TRUSTED_ROOTS, "Ignoring trusted roots beyond maximum");
                break;
            }

            let bytes = hex::decode(root_hex).map_err(|e| SecretsError::ParseTrustedRoot {
                index: i,
                reason: format!("invalid hex: {e}"),
            })?;

            let key = PublicKey::try_from(bytes.as_slice()).map_err(|e| SecretsError::ParseTrustedRoot {
                index: i,
                reason: format!("invalid public key: {e}"),
            })?;

            trusted_roots.push(key);
        }

        debug!(count = trusted_roots.len(), "Loaded trusted roots");

        // Parse signing key
        let signing_key = if let Some(ref key_hex) = file_secrets.signing_key {
            let bytes = hex::decode(key_hex).map_err(|e| SecretsError::ParseSigningKey {
                reason: format!("invalid hex: {e}"),
            })?;

            let key = SecretKey::try_from(bytes.as_slice()).map_err(|e| SecretsError::ParseSigningKey {
                reason: format!("invalid secret key: {e}"),
            })?;

            debug!("Loaded signing key");
            Some(key)
        } else {
            None
        };

        // Parse pre-built tokens
        let mut tokens = HashMap::new();
        for (i, (name, token_b64)) in file_secrets.tokens.iter().enumerate() {
            if i >= MAX_PREBUILT_TOKENS {
                warn!(max = MAX_PREBUILT_TOKENS, "Ignoring tokens beyond maximum");
                break;
            }

            let token = CapabilityToken::from_base64(token_b64).map_err(|e| SecretsError::ParseToken {
                name: name.clone(),
                reason: e.to_string(),
            })?;

            tokens.insert(name.clone(), token);
        }

        debug!(count = tokens.len(), "Loaded pre-built tokens");

        Ok(Self {
            config,
            file_secrets,
            trusted_roots,
            signing_key,
            tokens,
            cache: RwLock::new(SecretCache::new()),
            kv_store: None,
        })
    }

    /// Attach a KV store for runtime secrets.
    pub fn with_kv_store(mut self, store: Arc<dyn aspen_core::KeyValueStore>) -> Self {
        self.kv_store = Some(store);
        self
    }

    /// Get the cache TTL duration.
    fn cache_ttl(&self) -> Option<Duration> {
        if self.config.cache_enabled && self.config.cache_ttl_secs > 0 {
            Some(Duration::from_secs(self.config.cache_ttl_secs))
        } else {
            None
        }
    }

    /// Build a TokenVerifier with trusted roots from secrets.
    pub fn build_token_verifier(&self) -> aspen_auth::TokenVerifier {
        let mut verifier = aspen_auth::TokenVerifier::new();
        for root in &self.trusted_roots {
            verifier = verifier.with_trusted_root(*root);
        }
        verifier
    }

    /// Build a TokenBuilder with signing key from secrets.
    pub fn build_token_builder(&self) -> Result<aspen_auth::TokenBuilder> {
        let signing_key = self.signing_key.clone().ok_or_else(|| SecretsError::SecretNotFound {
            key: "signing_key".into(),
        })?;
        Ok(aspen_auth::TokenBuilder::new(signing_key))
    }

    /// Get the number of trusted roots.
    pub fn trusted_root_count(&self) -> usize {
        self.trusted_roots.len()
    }

    /// Clear expired cache entries.
    pub async fn clear_expired_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear_expired();
    }
}

#[async_trait]
impl SecretsProvider for SecretsManager {
    async fn get_string(&self, key: &str) -> Result<String> {
        // Check file secrets first
        if let Some(value) = self.file_secrets.secrets.strings.get(key) {
            return Ok(value.clone());
        }

        // Check cache
        if self.config.cache_enabled {
            let cache = self.cache.read().await;
            if let Some(bytes) = cache.get(key) {
                return String::from_utf8(bytes.to_vec()).map_err(|e| SecretsError::DecodeSecret {
                    key: key.into(),
                    reason: e.to_string(),
                });
            }
        }

        // Try KV store
        if let Some(ref kv) = self.kv_store {
            let full_key = format!("{}{}", self.config.kv_secrets_prefix, key);
            let request = ReadRequest::new(&full_key);
            match kv.read(request).await {
                Ok(result) => {
                    if let Some(kv_entry) = result.kv {
                        let value = kv_entry.value;
                        // Cache the result
                        if self.config.cache_enabled {
                            let mut cache = self.cache.write().await;
                            cache.insert(key.into(), value.clone().into_bytes(), self.cache_ttl());
                        }

                        return Ok(value);
                    }
                }
                Err(e) => {
                    return Err(SecretsError::KvStore { reason: e.to_string() });
                }
            }
        }

        Err(SecretsError::SecretNotFound { key: key.into() })
    }

    async fn get_bytes(&self, key: &str) -> Result<Vec<u8>> {
        // Check file secrets first (base64 encoded)
        if let Some(value) = self.file_secrets.secrets.bytes.get(key) {
            use base64::Engine;
            return base64::engine::general_purpose::STANDARD.decode(value).map_err(|e| SecretsError::DecodeSecret {
                key: key.into(),
                reason: format!("invalid base64: {e}"),
            });
        }

        // Check cache
        if self.config.cache_enabled {
            let cache = self.cache.read().await;
            if let Some(bytes) = cache.get(key) {
                return Ok(bytes.to_vec());
            }
        }

        // Try KV store (values are stored as base64-encoded strings)
        if let Some(ref kv) = self.kv_store {
            use base64::Engine;
            let full_key = format!("{}{}", self.config.kv_secrets_prefix, key);
            let request = ReadRequest::new(&full_key);
            match kv.read(request).await {
                Ok(result) => {
                    if let Some(kv_entry) = result.kv {
                        let value = kv_entry.value;
                        // Decode from base64
                        let bytes = base64::engine::general_purpose::STANDARD.decode(&value).map_err(|e| {
                            SecretsError::DecodeSecret {
                                key: key.into(),
                                reason: format!("invalid base64: {e}"),
                            }
                        })?;

                        // Cache the decoded result
                        if self.config.cache_enabled {
                            let mut cache = self.cache.write().await;
                            cache.insert(key.into(), bytes.clone(), self.cache_ttl());
                        }

                        return Ok(bytes);
                    }
                }
                Err(e) => {
                    return Err(SecretsError::KvStore { reason: e.to_string() });
                }
            }
        }

        Err(SecretsError::SecretNotFound { key: key.into() })
    }

    async fn get_trusted_roots(&self) -> Result<Vec<PublicKey>> {
        Ok(self.trusted_roots.clone())
    }

    async fn get_signing_key(&self) -> Result<SecretKey> {
        self.signing_key.clone().ok_or_else(|| SecretsError::SecretNotFound {
            key: "signing_key".into(),
        })
    }

    async fn get_token(&self, name: &str) -> Result<CapabilityToken> {
        self.tokens.get(name).cloned().ok_or_else(|| SecretsError::SecretNotFound {
            key: format!("token:{name}"),
        })
    }

    async fn list_tokens(&self) -> Result<Vec<String>> {
        Ok(self.tokens.keys().cloned().collect())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        // Check file secrets
        if self.file_secrets.secrets.strings.contains_key(key) || self.file_secrets.secrets.bytes.contains_key(key) {
            return Ok(true);
        }

        // Check cache
        if self.config.cache_enabled {
            let cache = self.cache.read().await;
            if cache.get(key).is_some() {
                return Ok(true);
            }
        }

        // Check KV store
        if let Some(ref kv) = self.kv_store {
            let full_key = format!("{}{}", self.config.kv_secrets_prefix, key);
            let request = ReadRequest::new(&full_key);
            match kv.read(request).await {
                Ok(result) => return Ok(result.kv.is_some()),
                Err(e) => {
                    return Err(SecretsError::KvStore { reason: e.to_string() });
                }
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sops::config::SecretsData;

    fn test_config() -> SecretsConfig {
        SecretsConfig {
            enabled: true,
            cache_enabled: true,
            cache_ttl_secs: 300,
            ..Default::default()
        }
    }

    fn test_secrets_file() -> SecretsFile {
        let mut strings = HashMap::new();
        strings.insert("api_key".into(), "sk-test-123".into());

        let mut bytes = HashMap::new();
        bytes.insert("cert".into(), "dGVzdA==".into()); // "test" in base64

        SecretsFile {
            trusted_roots: vec![],
            signing_key: None,
            tokens: HashMap::new(),
            secrets: SecretsData { strings, bytes },
            sops: None,
        }
    }

    #[tokio::test]
    async fn test_get_string_from_file() {
        let manager = SecretsManager::new(test_config(), test_secrets_file()).unwrap();

        let result = manager.get_string("api_key").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "sk-test-123");
    }

    #[tokio::test]
    async fn test_get_bytes_from_file() {
        let manager = SecretsManager::new(test_config(), test_secrets_file()).unwrap();

        let result = manager.get_bytes("cert").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"test");
    }

    #[tokio::test]
    async fn test_secret_not_found() {
        let manager = SecretsManager::new(test_config(), test_secrets_file()).unwrap();

        let result = manager.get_string("nonexistent").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SecretsError::SecretNotFound { .. }));
    }

    #[tokio::test]
    async fn test_exists() {
        let manager = SecretsManager::new(test_config(), test_secrets_file()).unwrap();

        assert!(manager.exists("api_key").await.unwrap());
        assert!(manager.exists("cert").await.unwrap());
        assert!(!manager.exists("nonexistent").await.unwrap());
    }
}
