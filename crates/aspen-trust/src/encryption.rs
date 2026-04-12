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

    /// Copy all known epoch keys from an earlier context.
    ///
    /// Used during epoch rotation so the new context can still decrypt
    /// old-epoch values until background re-encryption completes.
    pub fn copy_epoch_keys_from(&mut self, other: &Self) {
        for (epoch, key) in &other.keys {
            self.keys.entry(*epoch).or_insert(*key);
        }
    }

    /// Remove a prior epoch's key (after re-encryption completes).
    ///
    /// Zeroizes the key material before removal.
    /// Panics if `epoch` is the current epoch — removing the active
    /// write key would break `wrap_write`.
    pub fn remove_epoch_key(&mut self, epoch: u64) {
        assert!(epoch != self.epoch, "cannot remove the current epoch key (epoch {})", self.epoch);
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

/// Result of trying to decrypt stored bytes.
#[derive(Debug)]
pub enum DecryptOutcome {
    /// Successfully decrypted an encrypted envelope.
    Decrypted(Vec<u8>),
    /// Bytes are not an encrypted envelope (legacy plaintext).
    /// Safe to return the raw bytes to the caller.
    NotAnEnvelope,
    /// Bytes are a structurally valid envelope but decryption failed.
    /// This means the ciphertext was tampered with, the key is wrong,
    /// or the epoch is unknown. This is an error — do NOT fall back
    /// to plaintext.
    AuthenticationFailed(envelope::EnvelopeError),
}

/// Try to parse and decrypt stored bytes as an encrypted envelope.
///
/// Two-phase detection:
/// 1. **Parse**: Try `EncryptedValue::from_bytes()`. If this fails (wrong magic, too short, bad
///    version), the bytes are NOT an envelope → `NotAnEnvelope`.
/// 2. **Decrypt**: If parsing succeeds, the bytes ARE an envelope. Try to decrypt. If decryption
///    fails (tampered, wrong key, unknown epoch), that's an authentication error →
///    `AuthenticationFailed`.
///
/// This preserves both properties:
/// - Legacy plaintext is never misclassified (parse fails → fallback)
/// - Tampered encrypted values are detected (parse succeeds, decrypt fails → error)
pub fn try_decrypt(enc: &SecretsEncryption, stored: &[u8]) -> DecryptOutcome {
    // Phase 1: try to parse as an envelope
    let encrypted = match envelope::EncryptedValue::from_bytes(stored) {
        Ok(ev) => ev,
        Err(_) => return DecryptOutcome::NotAnEnvelope,
    };

    // Phase 2: we have a structurally valid envelope — decrypt it
    let key = match enc.keys.get(&encrypted.epoch) {
        Some(k) => k,
        None => {
            return DecryptOutcome::AuthenticationFailed(envelope::EnvelopeError::EpochMismatch {
                stored: encrypted.epoch,
                current: enc.epoch,
            });
        }
    };

    match envelope::decrypt_value(key, &encrypted) {
        Ok(plaintext) => DecryptOutcome::Decrypted(plaintext),
        Err(e) => DecryptOutcome::AuthenticationFailed(e),
    }
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
        match try_decrypt(&enc, &stored) {
            DecryptOutcome::Decrypted(pt) => assert_eq!(pt, b"secret-data"),
            other => panic!("expected Decrypted, got {:?}", other),
        }
    }

    #[test]
    fn test_try_decrypt_not_envelope_for_plaintext() {
        let secret = test_secret();
        let enc = SecretsEncryption::new(&secret, b"cluster", 1, 1, 0);

        // Short plaintext
        assert!(matches!(try_decrypt(&enc, b"plaintext-data"), DecryptOutcome::NotAnEnvelope));
        assert!(matches!(try_decrypt(&enc, b""), DecryptOutcome::NotAnEnvelope));
        assert!(matches!(try_decrypt(&enc, &[0x01, 0x02, 0x03]), DecryptOutcome::NotAnEnvelope));

        // Long plaintext
        assert!(matches!(try_decrypt(&enc, &vec![0x01u8; 100]), DecryptOutcome::NotAnEnvelope));
    }

    #[test]
    fn test_try_decrypt_not_envelope_for_bad_version() {
        let secret = test_secret();
        let enc = SecretsEncryption::new(&secret, b"cluster", 1, 1, 0);

        // AENC magic + unsupported version byte → parse failure → NotAnEnvelope
        let mut fake = vec![0u8; 100];
        fake[0..4].copy_from_slice(&envelope::ENVELOPE_MAGIC);
        fake[4] = 99;
        assert!(matches!(try_decrypt(&enc, &fake), DecryptOutcome::NotAnEnvelope));
    }

    #[test]
    fn test_try_decrypt_auth_failure_for_tampered_envelope() {
        // Bytes with AENC + valid version (1) + enough structure to
        // parse as an envelope. Poly1305 tag won't verify →
        // AuthenticationFailed (not NotAnEnvelope).
        let secret = test_secret();
        let enc = SecretsEncryption::new(&secret, b"cluster", 1, 1, 0);

        let mut fake = vec![0xAAu8; 100];
        fake[0..4].copy_from_slice(&envelope::ENVELOPE_MAGIC);
        fake[4] = 1; // valid version
        assert!(matches!(try_decrypt(&enc, &fake), DecryptOutcome::AuthenticationFailed(_)));
    }

    #[test]
    fn test_try_decrypt_auth_failure_for_real_tampered_value() {
        // Encrypt a real value, then flip a ciphertext byte.
        // Must return AuthenticationFailed, not NotAnEnvelope.
        let secret = test_secret();
        let enc = SecretsEncryption::new(&secret, b"cluster", 1, 1, 0);

        let (mut stored, _) = enc.wrap_write(b"real-secret").unwrap();
        // Flip a byte in the ciphertext area (after magic+version+epoch+nonce = 25 bytes)
        if stored.len() > 26 {
            stored[26] ^= 0xFF;
        }
        assert!(matches!(try_decrypt(&enc, &stored), DecryptOutcome::AuthenticationFailed(_)));
    }

    #[test]
    fn test_try_decrypt_auth_failure_for_unknown_epoch() {
        // Valid envelope but the epoch doesn't match any known key
        let secret = test_secret();
        let enc = SecretsEncryption::new(&secret, b"cluster", 1, 1, 0);

        let (stored, _) = enc.wrap_write(b"data").unwrap();
        let enc2 = SecretsEncryption::new(&secret, b"other-cluster", 5, 1, 0);
        assert!(matches!(try_decrypt(&enc2, &stored), DecryptOutcome::AuthenticationFailed(_)));
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
