//! Narinfo signing for Nix binary cache.
//!
//! Implements Ed25519 signatures for narinfo files, compatible with Nix's
//! trusted-public-keys verification.

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_format() {
        let fp = NarinfoSigner::fingerprint("/nix/store/abc123-hello", "sha256:deadbeef", 12345, &[
            "/nix/store/dep1-foo".to_string(),
            "/nix/store/dep2-bar".to_string(),
        ]);
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
