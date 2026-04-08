//! Transparent encryption/decryption layer for secrets at rest.
//!
//! `SecretsEncryption` wraps the envelope and nonce modules into a
//! high-level API for the secrets engine. Values are encrypted before
//! storage and decrypted on retrieval.
//!
//! The encryption key is derived from the cluster root secret via HKDF.
//! Key material is zeroized on drop.

use zeroize::ZeroizeOnDrop;

use crate::envelope;
use crate::envelope::EncryptedValue;
use crate::envelope::EnvelopeError;
use crate::kdf;
use crate::nonce::NonceGenerator;

/// Secrets encryption context.
///
/// Holds the derived at-rest encryption key and a nonce generator.
/// Implements `ZeroizeOnDrop` to clear key material from memory.
#[derive(ZeroizeOnDrop)]
pub struct SecretsEncryption {
    /// Derived at-rest encryption key (32 bytes).
    key: [u8; 32],
    /// Current epoch for new encryptions.
    #[zeroize(skip)]
    epoch: u64,
    /// Counter-based nonce generator.
    #[zeroize(skip)]
    nonce_gen: NonceGenerator,
}

impl SecretsEncryption {
    /// Create a new encryption context from a cluster root secret.
    ///
    /// Derives the at-rest key using HKDF with `CONTEXT_SECRETS_AT_REST`.
    pub fn new(
        cluster_secret: &[u8; 32],
        cluster_id: &[u8],
        epoch: u64,
        node_id: u32,
        initial_nonce_counter: u64,
    ) -> Self {
        let key = kdf::derive_key(cluster_secret, kdf::CONTEXT_SECRETS_AT_REST, cluster_id, epoch);
        Self {
            key,
            epoch,
            nonce_gen: NonceGenerator::new(node_id, initial_nonce_counter),
        }
    }

    /// Encrypt plaintext for storage.
    ///
    /// Returns the serialized `EncryptedValue` bytes.
    /// The nonce counter value is returned for persistence.
    pub fn wrap_write(&self, plaintext: &[u8]) -> Result<(Vec<u8>, u64), EnvelopeError> {
        let (nonce, counter) = self.nonce_gen.next_nonce();
        let encrypted = envelope::encrypt_value(&self.key, self.epoch, &nonce, plaintext)?;
        Ok((encrypted.to_bytes(), counter))
    }

    /// Decrypt a stored value.
    ///
    /// Deserializes the bytes and decrypts with the current key.
    /// For multi-epoch support, callers should check the epoch field
    /// and use the appropriate `SecretsEncryption` instance.
    pub fn unwrap_read(&self, stored: &[u8]) -> Result<Vec<u8>, EnvelopeError> {
        let encrypted = EncryptedValue::from_bytes(stored)?;
        envelope::decrypt_value(&self.key, &encrypted)
    }

    /// Get the epoch of the stored value without decrypting.
    pub fn peek_epoch(stored: &[u8]) -> Result<u64, EnvelopeError> {
        if stored.len() < 9 {
            return Err(EnvelopeError::TooShort { len: stored.len() });
        }
        let epoch = u64::from_be_bytes(stored[1..9].try_into().expect("8 bytes for u64"));
        Ok(epoch)
    }

    /// Current epoch.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Current nonce counter (for persistence).
    pub fn nonce_counter(&self) -> u64 {
        self.nonce_gen.current_counter()
    }

    /// Rotate to a new epoch with a new cluster secret.
    ///
    /// Derives a fresh key, zeroizes the old one. Returns the old key
    /// for re-encryption (caller must zeroize after use).
    pub fn rotate(&mut self, new_secret: &[u8; 32], cluster_id: &[u8], new_epoch: u64) -> [u8; 32] {
        let old_key = self.key;
        self.key = kdf::derive_key(new_secret, kdf::CONTEXT_SECRETS_AT_REST, cluster_id, new_epoch);
        self.epoch = new_epoch;
        old_key
    }

    /// Decrypt with a specific key (for re-encryption of old-epoch values).
    pub fn decrypt_with_key(key: &[u8; 32], stored: &[u8]) -> Result<Vec<u8>, EnvelopeError> {
        let encrypted = EncryptedValue::from_bytes(stored)?;
        envelope::decrypt_value(key, &encrypted)
    }
}

// Manual Debug to avoid leaking key material
impl std::fmt::Debug for SecretsEncryption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretsEncryption")
            .field("epoch", &self.epoch)
            .field("key", &"[REDACTED]")
            .finish()
    }
}

/// Check if stored bytes look like an encrypted envelope (version prefix check).
pub fn is_encrypted(stored: &[u8]) -> bool {
    !stored.is_empty() && stored[0] == 1 // ENVELOPE_VERSION
}

/// Errors when the secrets encryption layer is unavailable.
#[derive(Debug, snafu::Snafu)]
pub enum SecretsUnavailableError {
    /// Cluster secret could not be reconstructed (below quorum).
    #[snafu(display("secrets unavailable: cluster below quorum for secret reconstruction"))]
    BelowQuorum,

    /// Encryption/decryption error.
    #[snafu(display("secrets encryption error: {source}"))]
    Envelope { source: EnvelopeError },
}

impl From<EnvelopeError> for SecretsUnavailableError {
    fn from(source: EnvelopeError) -> Self {
        SecretsUnavailableError::Envelope { source }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_secret() -> [u8; 32] {
        let mut s = [0u8; 32];
        s[0] = 0xDE;
        s[1] = 0xAD;
        s
    }

    #[test]
    fn test_wrap_unwrap_roundtrip() {
        let secret = test_secret();
        let enc = SecretsEncryption::new(&secret, b"test-cluster", 1, 1, 0);

        let plaintext = b"transit-key-material-here";
        let (stored, _counter) = enc.wrap_write(plaintext).unwrap();
        let recovered = enc.unwrap_read(&stored).unwrap();
        assert_eq!(&recovered, plaintext);
    }

    #[test]
    fn test_different_epoch_keys_incompatible() {
        let secret = test_secret();
        let enc1 = SecretsEncryption::new(&secret, b"cluster", 1, 1, 0);
        let enc2 = SecretsEncryption::new(&secret, b"cluster", 2, 1, 0);

        let (stored, _) = enc1.wrap_write(b"data").unwrap();
        let result = enc2.unwrap_read(&stored);
        assert!(result.is_err());
    }

    #[test]
    fn test_peek_epoch() {
        let secret = test_secret();
        let enc = SecretsEncryption::new(&secret, b"cluster", 42, 1, 0);

        let (stored, _) = enc.wrap_write(b"data").unwrap();
        let epoch = SecretsEncryption::peek_epoch(&stored).unwrap();
        assert_eq!(epoch, 42);
    }

    #[test]
    fn test_rotate_and_reencrypt() {
        let secret1 = test_secret();
        let mut enc = SecretsEncryption::new(&secret1, b"cluster", 1, 1, 0);

        let (stored_v1, _) = enc.wrap_write(b"important-secret").unwrap();

        // Rotate to epoch 2
        let mut secret2 = [0u8; 32];
        secret2[0] = 0xBE;
        secret2[1] = 0xEF;
        let mut old_key = enc.rotate(&secret2, b"cluster", 2);

        // Decrypt old value with old key
        let plaintext = SecretsEncryption::decrypt_with_key(&old_key, &stored_v1).unwrap();
        assert_eq!(&plaintext, b"important-secret");

        // Re-encrypt with new key
        let (stored_v2, _) = enc.wrap_write(&plaintext).unwrap();
        let recovered = enc.unwrap_read(&stored_v2).unwrap();
        assert_eq!(&recovered, b"important-secret");

        // Old key can't decrypt new value
        let result = SecretsEncryption::decrypt_with_key(&old_key, &stored_v2);
        assert!(result.is_err());

        old_key.fill(0);
    }

    #[test]
    fn test_is_encrypted() {
        let secret = test_secret();
        let enc = SecretsEncryption::new(&secret, b"cluster", 1, 1, 0);

        let (stored, _) = enc.wrap_write(b"data").unwrap();
        assert!(is_encrypted(&stored));
        assert!(!is_encrypted(b"plaintext-data"));
        assert!(!is_encrypted(b""));
    }

    #[test]
    fn test_debug_redacts_key() {
        let secret = test_secret();
        let enc = SecretsEncryption::new(&secret, b"cluster", 1, 1, 0);
        let debug = format!("{:?}", enc);
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("0xDE"));
    }

    #[test]
    fn test_multiple_wraps_produce_different_ciphertext() {
        let secret = test_secret();
        let enc = SecretsEncryption::new(&secret, b"cluster", 1, 1, 0);

        let (s1, _) = enc.wrap_write(b"same").unwrap();
        let (s2, _) = enc.wrap_write(b"same").unwrap();
        assert_ne!(s1, s2, "same plaintext should produce different ciphertext (different nonces)");
    }
}
