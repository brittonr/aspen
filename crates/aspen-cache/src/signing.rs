//! Nix binary cache narinfo signing.
//!
//! Implements Ed25519 signing of narinfo documents using the Nix fingerprint
//! format. Keys are serialized in Nix's `cache-name:base64(key)` format,
//! compatible with `nix-store --generate-binary-cache-key`.
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

use std::sync::Arc;

use aspen_core::KeyValueStore;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::WriteRequest;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use ed25519_dalek::Signer;
use ed25519_dalek::SigningKey;
use ed25519_dalek::Verifier;
use ed25519_dalek::VerifyingKey;
use rand_core_06::OsRng;
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
    #[snafu(display("invalid key format: expected 'name:base64data'"))]
    InvalidKeyFormat,

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
#[derive(Clone)]
pub struct CacheSigningKey {
    /// The cache name (e.g., "aspen-cache").
    name: String,
    /// The Ed25519 signing key.
    signing_key: SigningKey,
}

impl CacheSigningKey {
    /// Generate a new random signing keypair.
    pub fn generate(name: &str) -> Result<Self> {
        validate_cache_name(name)?;
        let signing_key = SigningKey::generate(&mut OsRng);
        Ok(Self {
            name: name.to_string(),
            signing_key,
        })
    }

    /// Parse a signing key from Nix format: `name:base64(secret_key_bytes)`.
    ///
    /// The secret key bytes are 64 bytes: 32 bytes secret scalar + 32 bytes public key.
    pub fn from_nix_format(key_str: &str) -> Result<Self> {
        let (name, data) = split_nix_key(key_str)?;
        let bytes = BASE64.decode(data).map_err(|e| SigningError::Base64Decode { message: e.to_string() })?;

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

        let signing_key = SigningKey::from_bytes(&secret_bytes);

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
        format!("{}:{}", self.name, BASE64.encode(signature.to_bytes()))
    }

    /// Get the cache name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A Nix cache public key for verification.
pub struct CacheVerifyingKey {
    /// The cache name.
    name: String,
    /// The Ed25519 verifying key.
    verifying_key: VerifyingKey,
}

impl CacheVerifyingKey {
    /// Parse a public key from Nix format: `name:base64(public_key_bytes)`.
    pub fn from_nix_format(key_str: &str) -> Result<Self> {
        let (name, data) = split_nix_key(key_str)?;
        let bytes = BASE64.decode(data).map_err(|e| SigningError::Base64Decode { message: e.to_string() })?;

        if bytes.len() != 32 {
            return Err(SigningError::InvalidKeyLength {
                got: bytes.len(),
                expected: 32,
            });
        }

        let key_bytes: [u8; 32] =
            bytes.try_into().map_err(|_| SigningError::InvalidKeyLength { got: 32, expected: 32 })?;

        let verifying_key = VerifyingKey::from_bytes(&key_bytes).map_err(|_| SigningError::VerificationFailed)?;

        Ok(Self {
            name: name.to_string(),
            verifying_key,
        })
    }

    /// Verify a signature against a fingerprint.
    ///
    /// The signature should be in Nix format: `name:base64(sig_bytes)`.
    pub fn verify_signature(&self, fingerprint: &str, signature_str: &str) -> Result<bool> {
        let (sig_name, sig_data) = split_nix_key(signature_str)?;

        // Name must match
        if sig_name != self.name {
            return Ok(false);
        }

        let sig_bytes = BASE64.decode(sig_data).map_err(|e| SigningError::Base64Decode { message: e.to_string() })?;

        if sig_bytes.len() != 64 {
            return Ok(false);
        }

        let sig_array: [u8; 64] = sig_bytes.try_into().unwrap();
        let signature = ed25519_dalek::Signature::from_bytes(&sig_array);

        Ok(self.verifying_key.verify(fingerprint.as_bytes(), &signature).is_ok())
    }
}

/// Load an existing signing key from KV, or generate and store a new one.
///
/// Returns the signing key and the public key in Nix format.
/// This is idempotent — if a key already exists, it's returned as-is.
pub async fn ensure_signing_key(
    kv_store: &Arc<dyn KeyValueStore>,
    cache_name: &str,
) -> std::result::Result<(CacheSigningKey, String), SigningError> {
    // Try to load existing key
    let read_result = kv_store
        .read(ReadRequest::new(CACHE_SIGNING_KEY_KV))
        .await
        .map_err(|e| SigningError::KvError { message: e.to_string() })?;

    if let Some(kv) = read_result.kv {
        let key = CacheSigningKey::from_nix_format(&kv.value)?;
        let public_key = key.to_nix_public_key();
        info!(cache_name = key.name(), "loaded existing cache signing key");
        return Ok((key, public_key));
    }

    // Generate new key
    let key = CacheSigningKey::generate(cache_name)?;
    let secret_str = key.to_nix_secret_key();
    let public_str = key.to_nix_public_key();

    // Store secret key
    kv_store
        .write(WriteRequest::set(CACHE_SIGNING_KEY_KV, &secret_str))
        .await
        .map_err(|e| SigningError::KvError { message: e.to_string() })?;

    // Store public key (for easy retrieval by clients)
    kv_store
        .write(WriteRequest::set(CACHE_PUBLIC_KEY_KV, &public_str))
        .await
        .map_err(|e| SigningError::KvError { message: e.to_string() })?;

    // Store cache name
    kv_store
        .write(WriteRequest::set(CACHE_NAME_KV, cache_name))
        .await
        .map_err(|e| SigningError::KvError { message: e.to_string() })?;

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

/// Split a Nix-format key string into (name, base64_data).
fn split_nix_key(key_str: &str) -> Result<(&str, &str)> {
    let colon_pos = key_str.find(':').ok_or(SigningError::InvalidKeyFormat)?;
    let name = &key_str[..colon_pos];
    let data = &key_str[colon_pos + 1..];
    if name.is_empty() || data.is_empty() {
        return Err(SigningError::InvalidKeyFormat);
    }
    Ok((name, data))
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
        let (_, data) = split_nix_key(&secret_str).unwrap();
        let bytes = BASE64.decode(data).unwrap();
        assert_eq!(bytes.len(), 64);
    }

    #[test]
    fn test_nix_key_public_length() {
        // Nix public keys are 32 bytes base64-encoded
        let key = CacheSigningKey::generate("test").unwrap();
        let public_str = key.to_nix_public_key();
        let (_, data) = split_nix_key(&public_str).unwrap();
        let bytes = BASE64.decode(data).unwrap();
        assert_eq!(bytes.len(), 32);
    }
}
