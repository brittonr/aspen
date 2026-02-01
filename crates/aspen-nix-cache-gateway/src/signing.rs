//! Narinfo signing for Nix binary cache.
//!
//! Implements Ed25519 signatures for narinfo files, compatible with Nix's
//! trusted-public-keys verification.

use std::sync::Arc;

use aspen_secrets::transit::SignRequest;
use aspen_secrets::transit::TransitStore;
use async_trait::async_trait;
use base64::Engine;
use ed25519_dalek::Signer;
use ed25519_dalek::SigningKey;
use ed25519_dalek::VerifyingKey;

use crate::error::NixCacheError;
use crate::error::Result;

/// Signer for narinfo files.
///
/// Uses Ed25519 signatures with the format: `{cache_name}:{base64_signature}`
#[derive(Clone)]
pub struct NarinfoSigner {
    /// Cache name (e.g., "aspen-cache-1").
    cache_name: String,
    /// Ed25519 signing key.
    signing_key: SigningKey,
}

impl NarinfoSigner {
    /// Create a new signer with the given cache name and key.
    pub fn new(cache_name: String, signing_key: SigningKey) -> Self {
        Self {
            cache_name,
            signing_key,
        }
    }

    /// Create a signer from a base64-encoded seed.
    ///
    /// The seed should be 32 bytes (256 bits) base64-encoded.
    pub fn from_seed_base64(cache_name: String, seed_b64: &str) -> Result<Self> {
        let seed_bytes =
            base64::engine::general_purpose::STANDARD.decode(seed_b64).map_err(|e| NixCacheError::Signing {
                message: format!("failed to decode signing key seed: {}", e),
            })?;

        let seed: [u8; 32] = seed_bytes.try_into().map_err(|_| NixCacheError::Signing {
            message: "signing key seed must be 32 bytes".to_string(),
        })?;

        let signing_key = SigningKey::from_bytes(&seed);
        Ok(Self::new(cache_name, signing_key))
    }

    /// Get the cache name.
    pub fn cache_name(&self) -> &str {
        &self.cache_name
    }

    /// Compute the fingerprint for signing.
    ///
    /// Format: `1;{store_path};{nar_hash};{nar_size};{refs}`
    /// where refs is a comma-separated list of store paths.
    pub fn fingerprint(store_path: &str, nar_hash: &str, nar_size: u64, references: &[String]) -> String {
        let refs = references.join(",");
        format!("1;{};{};{};{}", store_path, nar_hash, nar_size, refs)
    }

    /// Sign a narinfo fingerprint.
    ///
    /// Returns the signature in Nix format: `{cache_name}:{base64_signature}`
    pub fn sign(&self, fingerprint: &str) -> String {
        let signature = self.signing_key.sign(fingerprint.as_bytes());
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
        format!("{}:{}", self.cache_name, sig_b64)
    }

    /// Sign a narinfo with the given fields.
    ///
    /// Convenience method that computes the fingerprint and signs it.
    pub fn sign_narinfo(&self, store_path: &str, nar_hash: &str, nar_size: u64, references: &[String]) -> String {
        let fingerprint = Self::fingerprint(store_path, nar_hash, nar_size, references);
        self.sign(&fingerprint)
    }

    /// Get the public key in Nix format.
    ///
    /// Format: `{cache_name}:{base64_public_key}`
    ///
    /// This can be added to `trusted-public-keys` in nix.conf.
    pub fn public_key(&self) -> String {
        let verifying_key: VerifyingKey = self.signing_key.verifying_key();
        let pk_b64 = base64::engine::general_purpose::STANDARD.encode(verifying_key.as_bytes());
        format!("{}:{}", self.cache_name, pk_b64)
    }

    /// Create a new signer that uses Aspen's Transit secrets engine for signing.
    ///
    /// This retrieves the public key from Transit during construction and caches it.
    /// The private key remains secure within the Transit backend.
    pub async fn from_transit(
        cache_name: String,
        transit_store: Arc<dyn TransitStore>,
        key_name: String,
    ) -> Result<TransitNarinfoSigner> {
        // Get the key metadata to validate the key exists and get public key
        let transit_key = transit_store
            .read_key(&key_name)
            .await
            .map_err(|e| NixCacheError::Signing {
                message: format!("Failed to read key from Transit: {}", e),
            })?
            .ok_or_else(|| NixCacheError::Signing {
                message: format!("Key '{}' not found in Transit", key_name),
            })?;

        // Get the public key from the current version
        let current_version =
            transit_key.versions.get(&transit_key.current_version).ok_or_else(|| NixCacheError::Signing {
                message: format!("Current version {} not found for key '{}'", transit_key.current_version, key_name),
            })?;

        let public_key_bytes = current_version.public_key.as_ref().ok_or_else(|| NixCacheError::Signing {
            message: format!("Key '{}' does not have a public key (not an asymmetric key)", key_name),
        })?;

        // Encode public key as base64
        let cached_public_key = base64::engine::general_purpose::STANDARD.encode(public_key_bytes);

        Ok(TransitNarinfoSigner {
            cache_name,
            transit_store,
            key_name,
            cached_public_key,
        })
    }
}

/// Trait for different narinfo signing backends.
#[async_trait]
pub trait NarinfoSigningProvider: Send + Sync {
    /// Get the cache name.
    fn cache_name(&self) -> &str;

    /// Sign a narinfo fingerprint.
    ///
    /// Returns the signature in Nix format: `{cache_name}:{base64_signature}`
    async fn sign(&self, fingerprint: &str) -> Result<String>;

    /// Sign a narinfo with the given fields.
    ///
    /// Convenience method that computes the fingerprint and signs it.
    async fn sign_narinfo(
        &self,
        store_path: &str,
        nar_hash: &str,
        nar_size: u64,
        references: &[String],
    ) -> Result<String> {
        let fingerprint = NarinfoSigner::fingerprint(store_path, nar_hash, nar_size, references);
        self.sign(&fingerprint).await
    }

    /// Get the public key in Nix format.
    ///
    /// Format: `{cache_name}:{base64_public_key}`
    async fn public_key(&self) -> Result<String>;
}

/// Local implementation of the signing provider trait.
#[async_trait]
impl NarinfoSigningProvider for NarinfoSigner {
    fn cache_name(&self) -> &str {
        &self.cache_name
    }

    async fn sign(&self, fingerprint: &str) -> Result<String> {
        Ok(self.sign(fingerprint))
    }

    async fn public_key(&self) -> Result<String> {
        Ok(self.public_key())
    }
}

/// Narinfo signer using Aspen's Transit secrets engine.
///
/// Uses remote Ed25519 signing where the private key never leaves the Transit store.
/// Public key is cached for performance.
pub struct TransitNarinfoSigner {
    /// Cache name (e.g., "aspen-cache-1").
    cache_name: String,
    /// Transit secrets store for remote signing.
    transit_store: Arc<dyn TransitStore>,
    /// Key name in Transit (e.g., "narinfo-signing-key").
    key_name: String,
    /// Cached public key in base64 format.
    cached_public_key: String,
}

#[async_trait]
impl NarinfoSigningProvider for TransitNarinfoSigner {
    fn cache_name(&self) -> &str {
        &self.cache_name
    }

    async fn sign(&self, fingerprint: &str) -> Result<String> {
        let sign_request = SignRequest {
            key_name: self.key_name.clone(),
            input: fingerprint.as_bytes().to_vec(),
            hash_algorithm: None, // Use default (SHA-256) for Ed25519
            prehashed: false,
            key_version: None, // Use latest version
        };

        let sign_response = self.transit_store.sign(sign_request).await.map_err(|e| NixCacheError::Signing {
            message: format!("Transit signing failed: {}", e),
        })?;

        // Extract base64 signature from Transit format
        // Transit format: "aspen:v<version>:<base64-signature>"
        let signature_str = sign_response.signature;
        let sig_b64 = if let Some(colon_pos) = signature_str.rfind(':') {
            &signature_str[colon_pos + 1..]
        } else {
            return Err(NixCacheError::Signing {
                message: format!("Invalid Transit signature format: {}", signature_str),
            });
        };

        // Return in Nix format: cache_name:base64_signature
        Ok(format!("{}:{}", self.cache_name, sig_b64))
    }

    async fn public_key(&self) -> Result<String> {
        // Return cached public key in Nix format
        Ok(format!("{}:{}", self.cache_name, self.cached_public_key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_format() {
        let fp = NarinfoSigner::fingerprint(
            "/nix/store/abc123-hello",
            "sha256:deadbeef",
            12345,
            &["/nix/store/dep1-foo".to_string(), "/nix/store/dep2-bar".to_string()],
        );
        assert_eq!(fp, "1;/nix/store/abc123-hello;sha256:deadbeef;12345;/nix/store/dep1-foo,/nix/store/dep2-bar");
    }

    #[test]
    fn test_fingerprint_no_refs() {
        let fp = NarinfoSigner::fingerprint("/nix/store/abc123-hello", "sha256:deadbeef", 12345, &[]);
        assert_eq!(fp, "1;/nix/store/abc123-hello;sha256:deadbeef;12345;");
    }

    #[test]
    fn test_sign_and_verify_format() {
        // Generate a test key
        let seed = [0u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let signer = NarinfoSigner::new("test-cache".to_string(), signing_key);

        let sig = signer.sign("test fingerprint");

        // Verify format: cache_name:base64_signature
        assert!(sig.starts_with("test-cache:"));
        let parts: Vec<&str> = sig.splitn(2, ':').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "test-cache");

        // Verify base64 is valid
        base64::engine::general_purpose::STANDARD
            .decode(parts[1])
            .expect("signature should be valid base64");
    }

    #[test]
    fn test_public_key_format() {
        let seed = [0u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let signer = NarinfoSigner::new("test-cache".to_string(), signing_key);

        let pk = signer.public_key();

        // Verify format
        assert!(pk.starts_with("test-cache:"));
        let parts: Vec<&str> = pk.splitn(2, ':').collect();
        assert_eq!(parts.len(), 2);

        // Verify base64 is valid and correct length (32 bytes = 44 base64 chars with padding)
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(parts[1])
            .expect("public key should be valid base64");
        assert_eq!(decoded.len(), 32);
    }

    #[test]
    fn test_from_seed_base64() {
        let seed = [42u8; 32];
        let seed_b64 = base64::engine::general_purpose::STANDARD.encode(seed);

        let signer = NarinfoSigner::from_seed_base64("test".to_string(), &seed_b64).unwrap();
        assert_eq!(signer.cache_name(), "test");
    }

    #[test]
    fn test_from_seed_base64_invalid() {
        let result = NarinfoSigner::from_seed_base64("test".to_string(), "not valid base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_seed_base64_wrong_length() {
        let short_seed = [0u8; 16]; // Should be 32
        let seed_b64 = base64::engine::general_purpose::STANDARD.encode(short_seed);

        let result = NarinfoSigner::from_seed_base64("test".to_string(), &seed_b64);
        assert!(result.is_err());
    }
}
