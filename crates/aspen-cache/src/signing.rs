//! Nix binary cache narinfo signing.
//!
//! Thin wrappers around `nix_compat::narinfo::{SigningKey, VerifyingKey, parse_keypair}`
//! for Ed25519 signing of narinfo documents. Keys are serialized in Nix's
//! `cache-name:base64(key)` format, compatible with `nix-store --generate-binary-cache-key`.
//!
//! # Key Format
//!
//! Nix cache keys use the format `{name}:{base64(key_bytes)}`:
//! - Secret key: `{name}:{base64(ed25519_secret_key || ed25519_public_key)}` (64 bytes)
//! - Public key: `{name}:{base64(ed25519_public_key)}` (32 bytes)
//!
//! # Signature Format
//!
//! narinfo signatures use: `{name}:{base64(ed25519_signature)}` (64 bytes)
//! The signed message is the fingerprint: `1;{store_path};{nar_hash};{nar_size};{refs}`

#[cfg(feature = "kv-index")]
use std::sync::Arc;

#[cfg(feature = "kv-index")]
use aspen_traits::KeyValueStore;
#[cfg(feature = "kv-index")]
use aspen_kv_types::ReadRequest;
#[cfg(feature = "kv-index")]
use aspen_kv_types::WriteRequest;
use data_encoding::BASE64;
use ed25519_dalek::Signer;
use nix_compat::narinfo;
use snafu::Snafu;
use tracing::info;

// Tiger Style: explicit bounds
/// Maximum cache name length.
pub const MAX_CACHE_NAME_LENGTH: usize = 128;
/// Default cache name.
pub const DEFAULT_CACHE_NAME: &str = "aspen-cache";

/// KV key for the cache signing secret key.
pub const CACHE_SIGNING_KEY_KV: &str = "_sys:nix-cache:signing-key";
/// KV key for the cache public key.
pub const CACHE_PUBLIC_KEY_KV: &str = "_sys:nix-cache:public-key";
/// KV key for the cache name.
pub const CACHE_NAME_KV: &str = "_sys:nix-cache:name";

/// Errors from cache signing operations.
#[derive(Debug, Snafu)]
pub enum SigningError {
    /// Cache name is too long.
    #[snafu(display("cache name too long: {length} bytes (max {MAX_CACHE_NAME_LENGTH})"))]
    CacheNameTooLong { length: usize },

    /// Cache name contains invalid characters (must not contain ':').
    #[snafu(display("cache name contains ':' which is the key format delimiter"))]
    CacheNameContainsColon,

    /// Invalid key format — expected `name:base64data`.
    #[snafu(display("invalid key format: {message}"))]
    InvalidKeyFormat { message: String },

    /// Base64 decoding failed.
    #[snafu(display("base64 decode error: {message}"))]
    Base64Decode { message: String },

    /// Invalid key length.
    #[snafu(display("invalid key length: got {got} bytes, expected {expected}"))]
    InvalidKeyLength { got: usize, expected: usize },

    /// Signature verification failed.
    #[snafu(display("signature verification failed"))]
    VerificationFailed,

    /// KV store operation failed.
    #[snafu(display("KV store error: {message}"))]
    KvError { message: String },
}

pub type Result<T> = std::result::Result<T, SigningError>;

/// A Nix cache signing keypair with its associated cache name.
///
/// Stores the raw `ed25519_dalek::SigningKey` for serialization, and constructs
/// `nix_compat::narinfo::SigningKey` on-the-fly for signing operations.
#[derive(Clone)]
pub struct CacheSigningKey {
    /// The cache name (e.g., "aspen-cache").
    name: String,
    /// The Ed25519 signing key.
    signing_key: ed25519_dalek::SigningKey,
}

impl CacheSigningKey {
    /// Generate a new random signing keypair.
    pub fn generate(name: &str) -> Result<Self> {
        validate_cache_name(name)?;
        let signing_key = ed25519_dalek::SigningKey::generate(&mut rand_core_06::OsRng);
        Ok(Self {
            name: name.to_string(),
            signing_key,
        })
    }

    /// Parse a signing key from Nix format: `name:base64(secret_key_bytes)`.
    ///
    /// The secret key bytes are 64 bytes: 32 bytes secret scalar + 32 bytes public key.
    /// Format validation follows nix-compat's `parse_keypair` logic.
    pub fn from_nix_format(key_str: &str) -> Result<Self> {
        let (name, data) = key_str.split_once(':').ok_or(SigningError::InvalidKeyFormat {
            message: "expected 'name:base64data'".to_string(),
        })?;

        if name.is_empty() || data.is_empty() {
            return Err(SigningError::InvalidKeyFormat {
                message: "name and data must not be empty".to_string(),
            });
        }

        let bytes =
            BASE64.decode(data.as_bytes()).map_err(|e| SigningError::Base64Decode { message: e.to_string() })?;

        // Nix secret keys are 64 bytes: secret scalar (32) + public key (32)
        if bytes.len() != 64 {
            return Err(SigningError::InvalidKeyLength {
                got: bytes.len(),
                expected: 64,
            });
        }

        let secret_bytes: [u8; 32] = bytes[..32].try_into().map_err(|_| SigningError::InvalidKeyLength {
            got: bytes.len(),
            expected: 64,
        })?;

        let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret_bytes);

        Ok(Self {
            name: name.to_string(),
            signing_key,
        })
    }

    /// Serialize to Nix secret key format: `name:base64(secret || public)`.
    pub fn to_nix_secret_key(&self) -> String {
        let mut key_bytes = Vec::with_capacity(64);
        key_bytes.extend_from_slice(self.signing_key.as_bytes());
        key_bytes.extend_from_slice(self.signing_key.verifying_key().as_bytes());
        format!("{}:{}", self.name, BASE64.encode(&key_bytes))
    }

    /// Get the public key in Nix format: `name:base64(public_key)`.
    pub fn to_nix_public_key(&self) -> String {
        format!("{}:{}", self.name, BASE64.encode(self.signing_key.verifying_key().as_bytes()))
    }

    /// Sign a narinfo fingerprint and return the signature in Nix format.
    ///
    /// Returns `name:base64(ed25519_signature)`.
    pub fn sign_fingerprint(&self, fingerprint: &str) -> String {
        let signature = self.signing_key.sign(fingerprint.as_bytes());
        format!("{}:{}", self.name, BASE64.encode(&signature.to_bytes()))
    }

    /// Get a nix-compat `SigningKey` for direct use with `NarInfo::add_signature`.
    pub fn to_nix_compat_signing_key(&self) -> narinfo::SigningKey<ed25519_dalek::SigningKey> {
        narinfo::SigningKey::new(self.name.clone(), self.signing_key.clone())
    }

    /// Get the cache name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A Nix cache public key for verification.
///
/// Wraps `nix_compat::narinfo::VerifyingKey`.
pub struct CacheVerifyingKey {
    inner: narinfo::VerifyingKey,
}

impl CacheVerifyingKey {
    /// Parse a public key from Nix format: `name:base64(public_key_bytes)`.
    pub fn from_nix_format(key_str: &str) -> Result<Self> {
        let inner = narinfo::VerifyingKey::parse(key_str)
            .map_err(|e| SigningError::InvalidKeyFormat { message: e.to_string() })?;
        Ok(Self { inner })
    }

    /// Verify a signature against a fingerprint.
    ///
    /// The signature should be in Nix format: `name:base64(sig_bytes)`.
    pub fn verify_signature(&self, fingerprint: &str, signature_str: &str) -> Result<bool> {
        let sig_ref = narinfo::SignatureRef::parse(signature_str)
            .map_err(|e| SigningError::InvalidKeyFormat { message: e.to_string() })?;

        // Name must match
        if *sig_ref.name() != self.inner.name() {
            return Ok(false);
        }

        Ok(self.inner.verify(fingerprint, &sig_ref))
    }
}

/// Port trait for loading and saving cache signing keys.
#[async_trait::async_trait]
pub trait SigningKeyStore: Send + Sync {
    /// Load the signing secret key, if one has been stored.
    async fn load_signing_key(&self) -> Result<Option<String>>;

    /// Save the signing key material (secret, public, and cache name).
    async fn save_signing_key(&self, secret: &str, public: &str, name: &str) -> Result<()>;

    /// Load just the public key, if one has been stored.
    async fn load_public_key(&self) -> Result<Option<String>>;
}

/// KV-backed implementation of `SigningKeyStore`.
#[cfg(feature = "kv-index")]
pub struct KvSigningKeyStore {
    kv: Arc<dyn KeyValueStore>,
}

#[cfg(feature = "kv-index")]
impl KvSigningKeyStore {
    /// Create a new KV-backed signing key store.
    pub fn new(kv: Arc<dyn KeyValueStore>) -> Self {
        Self { kv }
    }
}

#[cfg(feature = "kv-index")]
#[async_trait::async_trait]
impl SigningKeyStore for KvSigningKeyStore {
    async fn load_signing_key(&self) -> Result<Option<String>> {
        let read_result = self
            .kv
            .read(ReadRequest::new(CACHE_SIGNING_KEY_KV))
            .await
            .map_err(|e| SigningError::KvError { message: e.to_string() })?;
        Ok(read_result.kv.map(|kv| kv.value))
    }

    async fn save_signing_key(&self, secret: &str, public: &str, name: &str) -> Result<()> {
        self.kv
            .write(WriteRequest::set(CACHE_SIGNING_KEY_KV, secret))
            .await
            .map_err(|e| SigningError::KvError { message: e.to_string() })?;
        self.kv
            .write(WriteRequest::set(CACHE_PUBLIC_KEY_KV, public))
            .await
            .map_err(|e| SigningError::KvError { message: e.to_string() })?;
        self.kv
            .write(WriteRequest::set(CACHE_NAME_KV, name))
            .await
            .map_err(|e| SigningError::KvError { message: e.to_string() })?;
        Ok(())
    }

    async fn load_public_key(&self) -> Result<Option<String>> {
        let read_result = self
            .kv
            .read(ReadRequest::new(CACHE_PUBLIC_KEY_KV))
            .await
            .map_err(|e| SigningError::KvError { message: e.to_string() })?;
        Ok(read_result.kv.map(|kv| kv.value))
    }
}

/// Load an existing signing key from the store, or generate and save a new one.
///
/// Returns the signing key and the public key in Nix format.
/// This is idempotent — if a key already exists, it's returned as-is.
pub async fn ensure_signing_key(
    store: &dyn SigningKeyStore,
    cache_name: &str,
) -> std::result::Result<(CacheSigningKey, String), SigningError> {
    if let Some(secret_str) = store.load_signing_key().await? {
        let key = CacheSigningKey::from_nix_format(&secret_str)?;
        let public_key = key.to_nix_public_key();
        info!(cache_name = key.name(), "loaded existing cache signing key");
        return Ok((key, public_key));
    }

    let key = CacheSigningKey::generate(cache_name)?;
    let secret_str = key.to_nix_secret_key();
    let public_str = key.to_nix_public_key();

    store.save_signing_key(&secret_str, &public_str, cache_name).await?;

    info!(cache_name, public_key = %public_str, "generated new cache signing key");
    Ok((key, public_str))
}

/// Validate a cache name.
fn validate_cache_name(name: &str) -> Result<()> {
    if name.len() > MAX_CACHE_NAME_LENGTH {
        return Err(SigningError::CacheNameTooLong { length: name.len() });
    }
    if name.contains(':') {
        return Err(SigningError::CacheNameContainsColon);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_roundtrip() {
        let key = CacheSigningKey::generate("test-cache").unwrap();
        let secret_str = key.to_nix_secret_key();
        let public_str = key.to_nix_public_key();

        assert!(secret_str.starts_with("test-cache:"));
        assert!(public_str.starts_with("test-cache:"));

        // Round-trip secret key
        let key2 = CacheSigningKey::from_nix_format(&secret_str).unwrap();
        assert_eq!(key2.to_nix_secret_key(), secret_str);
        assert_eq!(key2.to_nix_public_key(), public_str);
    }

    #[test]
    fn test_sign_and_verify() {
        let key = CacheSigningKey::generate("my-cache").unwrap();
        let fingerprint = "1;/nix/store/abc123-hello;sha256:deadbeef;1024;";

        let signature = key.sign_fingerprint(fingerprint);
        assert!(signature.starts_with("my-cache:"));

        let verifier = CacheVerifyingKey::from_nix_format(&key.to_nix_public_key()).unwrap();
        assert!(verifier.verify_signature(fingerprint, &signature).unwrap());
    }

    #[test]
    fn test_verify_wrong_fingerprint() {
        let key = CacheSigningKey::generate("my-cache").unwrap();
        let signature = key.sign_fingerprint("correct fingerprint");

        let verifier = CacheVerifyingKey::from_nix_format(&key.to_nix_public_key()).unwrap();
        assert!(!verifier.verify_signature("wrong fingerprint", &signature).unwrap());
    }

    #[test]
    fn test_verify_wrong_name() {
        let key = CacheSigningKey::generate("cache-a").unwrap();
        let signature = key.sign_fingerprint("test");

        // Create verifier with different name
        let other_key = CacheSigningKey::generate("cache-b").unwrap();
        let verifier = CacheVerifyingKey::from_nix_format(&other_key.to_nix_public_key()).unwrap();
        assert!(!verifier.verify_signature("test", &signature).unwrap());
    }

    #[test]
    fn test_invalid_cache_name_colon() {
        let result = CacheSigningKey::generate("bad:name");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_cache_name_too_long() {
        let long_name = "a".repeat(MAX_CACHE_NAME_LENGTH + 1);
        let result = CacheSigningKey::generate(&long_name);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_key_format() {
        assert!(CacheSigningKey::from_nix_format("no-colon").is_err());
        assert!(CacheSigningKey::from_nix_format(":no-name").is_err());
        assert!(CacheSigningKey::from_nix_format("no-data:").is_err());
    }

    #[test]
    fn test_nix_key_secret_length() {
        // Nix secret keys are 64 bytes base64-encoded
        let key = CacheSigningKey::generate("test").unwrap();
        let secret_str = key.to_nix_secret_key();
        let colon_pos = secret_str.find(':').unwrap();
        let data = &secret_str[colon_pos + 1..];
        let bytes = BASE64.decode(data.as_bytes()).unwrap();
        assert_eq!(bytes.len(), 64);
    }

    #[test]
    fn test_nix_key_public_length() {
        let key = CacheSigningKey::generate("test").unwrap();
        let public_str = key.to_nix_public_key();
        let colon_pos = public_str.find(':').unwrap();
        let data = &public_str[colon_pos + 1..];
        let bytes = BASE64.decode(data.as_bytes()).unwrap();
        assert_eq!(bytes.len(), 32);
    }

    /// In-memory SigningKeyStore fixture for testing the port trait.
    struct InMemorySigningKeyStore {
        secret: std::sync::Mutex<Option<String>>,
        public: std::sync::Mutex<Option<String>>,
    }

    impl InMemorySigningKeyStore {
        fn new() -> Self {
            Self {
                secret: std::sync::Mutex::new(None),
                public: std::sync::Mutex::new(None),
            }
        }
    }

    #[async_trait::async_trait]
    impl SigningKeyStore for InMemorySigningKeyStore {
        async fn load_signing_key(&self) -> Result<Option<String>> {
            Ok(self.secret.lock().unwrap().clone())
        }

        async fn save_signing_key(&self, secret: &str, public: &str, _name: &str) -> Result<()> {
            *self.secret.lock().unwrap() = Some(secret.to_string());
            *self.public.lock().unwrap() = Some(public.to_string());
            Ok(())
        }

        async fn load_public_key(&self) -> Result<Option<String>> {
            Ok(self.public.lock().unwrap().clone())
        }
    }

    #[tokio::test]
    async fn ensure_signing_key_generates_when_empty() {
        let store = InMemorySigningKeyStore::new();
        let (key, public) = ensure_signing_key(&store, "test-cache").await.unwrap();
        assert!(public.starts_with("test-cache:"));
        assert_eq!(key.name(), "test-cache");
        assert!(store.secret.lock().unwrap().is_some());
    }

    #[tokio::test]
    async fn ensure_signing_key_loads_existing() {
        let store = InMemorySigningKeyStore::new();
        let (key1, pub1) = ensure_signing_key(&store, "c").await.unwrap();
        let (key2, pub2) = ensure_signing_key(&store, "c").await.unwrap();
        assert_eq!(pub1, pub2);
        assert_eq!(key1.to_nix_secret_key(), key2.to_nix_secret_key());
    }

    #[tokio::test]
    async fn ensure_signing_key_rejects_malformed_stored_key() {
        let store = InMemorySigningKeyStore::new();
        *store.secret.lock().unwrap() = Some("not-a-valid-key".to_string());
        let result = ensure_signing_key(&store, "c").await;
        assert!(result.is_err());
    }

    /// Failing store that returns errors on every operation.
    struct FailingSigningKeyStore;

    #[async_trait::async_trait]
    impl SigningKeyStore for FailingSigningKeyStore {
        async fn load_signing_key(&self) -> Result<Option<String>> {
            Err(SigningError::KvError {
                message: "simulated storage failure".to_string(),
            })
        }
        async fn save_signing_key(&self, _: &str, _: &str, _: &str) -> Result<()> {
            Err(SigningError::KvError {
                message: "simulated storage failure".to_string(),
            })
        }
        async fn load_public_key(&self) -> Result<Option<String>> {
            Err(SigningError::KvError {
                message: "simulated storage failure".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn ensure_signing_key_propagates_storage_errors() {
        let result = ensure_signing_key(&FailingSigningKeyStore, "c").await;
        assert!(matches!(result, Err(SigningError::KvError { .. })));
    }
}
