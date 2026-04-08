//! Transparent encryption/decryption layer for secrets at rest.
//!
//! `SecretsEncryption` wraps the envelope and nonce modules into a
//! high-level API for the secrets engine. Values are encrypted before
//! storage and decrypted on retrieval.
//!
//! The encryption key is derived from the cluster root secret via HKDF.
//! Key material is zeroized on drop.

use std::collections::BTreeMap;

use crate::envelope;
use crate::envelope::EncryptedValue;
use crate::envelope::EnvelopeError;
use crate::kdf;
use crate::nonce::NonceGenerator;

/// Secrets encryption context.
///
/// Holds derived at-rest encryption keys for the current and prior epochs.
/// Values encrypted at any known epoch can be decrypted. New writes use
/// the current epoch's key. All key material is zeroized on drop.
pub struct SecretsEncryption {
    /// All known epoch keys: epoch → derived key.
    /// The current epoch's key is always present.
    keys: BTreeMap<u64, [u8; 32]>,
    /// Current epoch for new encryptions.
    epoch: u64,
    /// Counter-based nonce generator.
    nonce_gen: NonceGenerator,
}

impl Drop for SecretsEncryption {
    fn drop(&mut self) {
        for key in self.keys.values_mut() {
            key.fill(0);
        }
    }
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
        let mut keys = BTreeMap::new();
        keys.insert(epoch, key);
        Self {
            keys,
            epoch,
            nonce_gen: NonceGenerator::new(node_id, initial_nonce_counter),
        }
    }

    /// Encrypt plaintext for storage.
    ///
    /// Returns the serialized `EncryptedValue` bytes.
    /// The nonce counter value is returned for persistence.
    pub fn wrap_write(&self, plaintext: &[u8]) -> Result<(Vec<u8>, u64), EnvelopeError> {
        let key = self.keys.get(&self.epoch).expect("current epoch key must exist");
        let (nonce, counter) = self.nonce_gen.next_nonce();
        let encrypted = envelope::encrypt_value(key, self.epoch, &nonce, plaintext)?;
        Ok((encrypted.to_bytes(), counter))
    }

    /// Decrypt a stored value.
    ///
    /// Deserializes the bytes, looks up the key for the stored value's
    /// epoch, and decrypts. Supports mixed-epoch reads during re-encryption:
    /// values encrypted at any known epoch are decryptable.
    ///
    /// Returns `EpochMismatch` only if the stored epoch has no known key.
    pub fn unwrap_read(&self, stored: &[u8]) -> Result<Vec<u8>, EnvelopeError> {
        let encrypted = EncryptedValue::from_bytes(stored)?;
        let key = self.keys.get(&encrypted.epoch).ok_or(EnvelopeError::EpochMismatch {
            stored: encrypted.epoch,
            current: self.epoch,
        })?;
        envelope::decrypt_value(key, &encrypted)
    }

    /// Get the epoch of the stored value without decrypting.
    ///
    /// Wire format: `magic(4) + version(1) + epoch(8) + ...`
    pub fn peek_epoch(stored: &[u8]) -> Result<u64, EnvelopeError> {
        // Need at least magic(4) + version(1) + epoch(8) = 13 bytes
        if stored.len() < 13 {
            return Err(EnvelopeError::TooShort { len: stored.len() });
        }
        let epoch = u64::from_be_bytes(stored[5..13].try_into().expect("8 bytes for u64"));
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

    /// Add a prior epoch's key (e.g., recovered from the encrypted chain).
    ///
    /// This enables decrypting values still stored at the old epoch
    /// while re-encryption is in progress.
    pub fn add_epoch_key(&mut self, epoch: u64, key: [u8; 32]) {
        self.keys.insert(epoch, key);
    }

    /// Remove a prior epoch's key (after re-encryption completes).
    ///
    /// Zeroizes the key material before removal.
    /// Panics if `epoch` is the current epoch — removing the active
    /// write key would break `wrap_write`.
    pub fn remove_epoch_key(&mut self, epoch: u64) {
        assert!(
            epoch != self.epoch,
            "cannot remove the current epoch key (epoch {})",
            self.epoch
        );
        if let Some(k) = self.keys.get_mut(&epoch) {
            k.fill(0);
        }
        self.keys.remove(&epoch);
    }

    /// Rotate to a new epoch with a new cluster secret.
    ///
    /// Derives a fresh key and adds it. The old epoch's key is retained
    /// so mixed-epoch reads keep working until re-encryption finishes.
    /// Call `remove_epoch_key` to drop old keys after re-encryption.
    pub fn rotate(&mut self, new_secret: &[u8; 32], cluster_id: &[u8], new_epoch: u64) {
        let new_key = kdf::derive_key(new_secret, kdf::CONTEXT_SECRETS_AT_REST, cluster_id, new_epoch);
        self.keys.insert(new_epoch, new_key);
        self.epoch = new_epoch;
    }

    /// Decrypt with a specific key (for re-encryption of old-epoch values).
    pub fn decrypt_with_key(key: &[u8; 32], stored: &[u8]) -> Result<Vec<u8>, EnvelopeError> {
        let encrypted = EncryptedValue::from_bytes(stored)?;
        envelope::decrypt_value(key, &encrypted)
    }

    /// How many epoch keys are currently held.
    pub fn epoch_key_count(&self) -> usize {
        self.keys.len()
    }
}

// Manual Debug to avoid leaking key material
impl std::fmt::Debug for SecretsEncryption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretsEncryption")
            .field("epoch", &self.epoch)
            .field("keys", &format!("[{} epoch keys REDACTED]", self.keys.len()))
            .finish()
    }
}

/// Try to parse stored bytes as an encrypted envelope.
///
/// Returns `Ok(plaintext)` if decryption succeeds, or `None` if the
/// bytes don't look like a valid envelope (wrong magic, too short, etc.).
/// This is the correct migration pattern: try decrypt, fall back to
/// treating as plaintext. No heuristic — parse failure is unambiguous.
/// Try to parse and decrypt stored bytes as an encrypted envelope.
///
/// Returns:
/// - `Ok(Some(plaintext))` if the bytes are a valid envelope and decryption succeeds
/// - `Ok(None)` if the bytes are not a valid envelope (legacy plaintext)
///
/// The function never returns `Err`. Any failure during parsing or
/// decryption is treated as "not an envelope" because legacy plaintext
/// can accidentally have bytes that partially resemble an envelope
/// header. Only a successful authenticated decryption (Poly1305 tag
/// verified) produces `Some`.
pub fn try_decrypt(
    enc: &SecretsEncryption,
    stored: &[u8],
) -> Option<Vec<u8>> {
    // Quick check: does it start with the envelope magic?
    if stored.len() < 4 || stored[0..4] != envelope::ENVELOPE_MAGIC {
        return None;
    }
    // Try to parse and decrypt. ANY failure means "not our envelope":
    // - TooShort / UnsupportedVersion: bytes don't form a valid envelope
    // - EpochMismatch: epoch in bytes doesn't match any known key
    // - DecryptFailed: Poly1305 tag mismatch (plaintext, not real ciphertext)
    enc.unwrap_read(stored).ok()
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
    fn test_unknown_epoch_returns_epoch_mismatch() {
        // A context that only knows epoch 1 cannot decrypt epoch 2 values
        let secret = test_secret();
        let enc1 = SecretsEncryption::new(&secret, b"cluster", 1, 1, 0);
        let enc2 = SecretsEncryption::new(&secret, b"cluster", 2, 1, 0);

        let (stored_at_2, _) = enc2.wrap_write(b"data").unwrap();
        let result = enc1.unwrap_read(&stored_at_2);
        assert!(
            matches!(result, Err(EnvelopeError::EpochMismatch { stored: 2, current: 1 })),
            "expected EpochMismatch, got: {:?}",
            result
        );
    }

    #[test]
    fn test_mixed_epoch_reads_after_rotate() {
        // After rotation, both old and new epoch values are readable
        let secret1 = test_secret();
        let mut enc = SecretsEncryption::new(&secret1, b"cluster", 1, 1, 0);

        let (stored_v1, _) = enc.wrap_write(b"epoch-1-secret").unwrap();

        // Rotate to epoch 2 — old key is retained
        let mut secret2 = [0u8; 32];
        secret2[0] = 0xBE;
        secret2[1] = 0xEF;
        enc.rotate(&secret2, b"cluster", 2);
        assert_eq!(enc.epoch_key_count(), 2);

        // Write at epoch 2
        let (stored_v2, _) = enc.wrap_write(b"epoch-2-secret").unwrap();

        // Both are readable
        let recovered_v1 = enc.unwrap_read(&stored_v1).unwrap();
        assert_eq!(&recovered_v1, b"epoch-1-secret");

        let recovered_v2 = enc.unwrap_read(&stored_v2).unwrap();
        assert_eq!(&recovered_v2, b"epoch-2-secret");

        // After removing old key, epoch 1 values become unreadable
        enc.remove_epoch_key(1);
        assert_eq!(enc.epoch_key_count(), 1);
        let result = enc.unwrap_read(&stored_v1);
        assert!(matches!(result, Err(EnvelopeError::EpochMismatch { .. })));

        // Epoch 2 still works
        let recovered_v2_again = enc.unwrap_read(&stored_v2).unwrap();
        assert_eq!(&recovered_v2_again, b"epoch-2-secret");
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
    fn test_rotate_reencrypt_and_drop_old_key() {
        let secret1 = test_secret();
        let mut enc = SecretsEncryption::new(&secret1, b"cluster", 1, 1, 0);

        let (stored_v1, _) = enc.wrap_write(b"important-secret").unwrap();

        // Rotate to epoch 2
        let mut secret2 = [0u8; 32];
        secret2[0] = 0xBE;
        secret2[1] = 0xEF;
        enc.rotate(&secret2, b"cluster", 2);

        // Old-epoch value is still readable (key retained after rotate)
        let plaintext = enc.unwrap_read(&stored_v1).unwrap();
        assert_eq!(&plaintext, b"important-secret");

        // Re-encrypt with new epoch key
        let (stored_v2, _) = enc.wrap_write(&plaintext).unwrap();
        let recovered = enc.unwrap_read(&stored_v2).unwrap();
        assert_eq!(&recovered, b"important-secret");

        // Drop old key after re-encryption
        enc.remove_epoch_key(1);
        let result = enc.unwrap_read(&stored_v1);
        assert!(matches!(result, Err(EnvelopeError::EpochMismatch { .. })));
    }

    #[test]
    fn test_try_decrypt_on_encrypted_value() {
        let secret = test_secret();
        let enc = SecretsEncryption::new(&secret, b"cluster", 1, 1, 0);

        let (stored, _) = enc.wrap_write(b"secret-data").unwrap();
        let result = try_decrypt(&enc, &stored);
        assert_eq!(result, Some(b"secret-data".to_vec()));
    }

    #[test]
    fn test_try_decrypt_returns_none_for_plaintext() {
        let secret = test_secret();
        let enc = SecretsEncryption::new(&secret, b"cluster", 1, 1, 0);

        // Short plaintext
        assert_eq!(try_decrypt(&enc, b"plaintext-data"), None);
        assert_eq!(try_decrypt(&enc, b""), None);
        assert_eq!(try_decrypt(&enc, &[0x01, 0x02, 0x03]), None);

        // Long plaintext
        assert_eq!(try_decrypt(&enc, &vec![0x01u8; 100]), None);
    }

    #[test]
    fn test_try_decrypt_returns_none_for_plaintext_with_magic_and_bad_version() {
        let secret = test_secret();
        let enc = SecretsEncryption::new(&secret, b"cluster", 1, 1, 0);

        // AENC magic + unsupported version byte → UnsupportedVersion → None
        let mut fake = vec![0u8; 100];
        fake[0..4].copy_from_slice(&envelope::ENVELOPE_MAGIC);
        fake[4] = 99;
        assert_eq!(try_decrypt(&enc, &fake), None);
    }

    #[test]
    fn test_try_decrypt_returns_none_for_plaintext_with_magic_and_valid_version() {
        // Plaintext that starts with AENC + valid version (1) + enough
        // bytes to parse as an envelope. Decryption will fail because
        // the Poly1305 tag won't authenticate. try_decrypt returns None.
        let secret = test_secret();
        let enc = SecretsEncryption::new(&secret, b"cluster", 1, 1, 0);

        let mut fake = vec![0xAAu8; 100];
        fake[0..4].copy_from_slice(&envelope::ENVELOPE_MAGIC);
        fake[4] = 1; // valid version
        // bytes 5..13 = epoch, 13..25 = nonce, 25.. = "ciphertext"
        // All garbage — Poly1305 won't verify → DecryptFailed → None
        assert_eq!(try_decrypt(&enc, &fake), None);
    }

    #[test]
    fn test_try_decrypt_returns_none_for_unknown_epoch() {
        // Valid magic + version, but epoch doesn't match any known key
        let secret = test_secret();
        let enc = SecretsEncryption::new(&secret, b"cluster", 1, 1, 0);

        // Encrypt something at epoch 1, then create a new context
        // that only knows epoch 5
        let (stored, _) = enc.wrap_write(b"data").unwrap();
        let enc2 = SecretsEncryption::new(&secret, b"other-cluster", 5, 1, 0);
        // Epoch 1 is unknown to enc2 → EpochMismatch → None
        assert_eq!(try_decrypt(&enc2, &stored), None);
    }

    #[test]
    #[should_panic(expected = "cannot remove the current epoch key")]
    fn test_remove_current_epoch_key_panics() {
        let secret = test_secret();
        let mut enc = SecretsEncryption::new(&secret, b"cluster", 1, 1, 0);
        enc.remove_epoch_key(1); // current epoch → panic
    }

    #[test]
    fn test_debug_redacts_key() {
        let secret = test_secret();
        let enc = SecretsEncryption::new(&secret, b"cluster", 1, 1, 0);
        let debug = format!("{:?}", enc);
        assert!(debug.contains("REDACTED"), "debug output should redact keys: {}", debug);
        assert!(debug.contains("1 epoch keys"), "should show key count: {}", debug);
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
