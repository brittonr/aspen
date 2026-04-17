//! Authenticated encryption envelope for secrets at rest.
//!
//! Uses ChaCha20-Poly1305 (RFC 8439) for AEAD. Each `EncryptedValue` carries
//! the version, epoch, nonce, and ciphertext (with Poly1305 tag appended).
//!
//! Formally verified — see `verus/envelope_spec.rs` for proofs (future).

use chacha20poly1305::ChaCha20Poly1305;
use chacha20poly1305::Nonce;
use chacha20poly1305::aead::Aead;
use chacha20poly1305::aead::KeyInit;
use serde::Deserialize;
use serde::Serialize;
use snafu::Snafu;

/// Current wire format version.
const ENVELOPE_VERSION: u8 = 1;

/// Magic bytes identifying an encrypted envelope.
/// ASCII "AENC" — chosen to be unlikely in postcard/JSON/base64 plaintext.
pub const ENVELOPE_MAGIC: [u8; 4] = [0x41, 0x45, 0x4E, 0x43];

/// Poly1305 authentication tag length.
const TAG_LEN: usize = 16;

/// Minimum serialized size: magic(4) + version(1) + epoch(8) + nonce(12) + tag(16).
const MIN_SERIALIZED_LEN: usize = 4usize.saturating_add(1).saturating_add(8).saturating_add(12).saturating_add(TAG_LEN);

/// An encrypted value envelope.
///
/// Layout: `[version: u8, epoch: u64, nonce: [u8; 12], ciphertext+tag: Vec<u8>]`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedValue {
    /// Wire format version (currently 1).
    pub version: u8,
    /// Epoch under which this value was encrypted.
    pub epoch: u64,
    /// 12-byte nonce (unique per encryption).
    pub nonce: [u8; 12],
    /// Ciphertext with appended Poly1305 tag (16 bytes).
    pub ciphertext: Vec<u8>,
}

/// Errors from envelope operations.
#[derive(Debug, Snafu)]
pub enum EnvelopeError {
    /// AEAD decryption failed (wrong key, tampered ciphertext, or wrong nonce).
    #[snafu(display("decryption failed: authentication tag mismatch"))]
    DecryptFailed,

    /// AEAD encryption failed (should not happen with valid inputs).
    #[snafu(display("encryption failed"))]
    EncryptFailed,

    /// Serialized bytes too short to contain a valid envelope.
    #[snafu(display("serialized envelope too short: {len} bytes (minimum {MIN_SERIALIZED_LEN})"))]
    TooShort { len: usize },

    /// Unsupported envelope version.
    #[snafu(display("unsupported envelope version: {version}"))]
    UnsupportedVersion { version: u8 },

    /// Value was encrypted at a different epoch than the current key.
    #[snafu(display("epoch mismatch: value encrypted at epoch {stored}, current key is epoch {current}"))]
    EpochMismatch { stored: u64, current: u64 },
}

/// Encrypt plaintext under the given key and epoch.
///
/// The nonce must be unique for each (key, plaintext) pair. Use
/// `NonceGenerator` for counter-based nonce management.
pub fn encrypt_value(
    key: &[u8; 32],
    epoch: u64,
    nonce: &[u8; 12],
    plaintext: &[u8],
) -> Result<EncryptedValue, EnvelopeError> {
    let cipher = ChaCha20Poly1305::new(key.into());
    let nonce_obj = Nonce::from_slice(nonce);
    let ciphertext = cipher.encrypt(nonce_obj, plaintext).map_err(|_| EnvelopeError::EncryptFailed)?;

    Ok(EncryptedValue {
        version: ENVELOPE_VERSION,
        epoch,
        nonce: *nonce,
        ciphertext,
    })
}

/// Decrypt an `EncryptedValue` using the given key.
///
/// Returns `DecryptFailed` if the key is wrong or the ciphertext was tampered.
pub fn decrypt_value(key: &[u8; 32], value: &EncryptedValue) -> Result<Vec<u8>, EnvelopeError> {
    if value.version != ENVELOPE_VERSION {
        return Err(EnvelopeError::UnsupportedVersion { version: value.version });
    }

    let cipher = ChaCha20Poly1305::new(key.into());
    let nonce = Nonce::from_slice(&value.nonce);
    cipher.decrypt(nonce, value.ciphertext.as_ref()).map_err(|_| EnvelopeError::DecryptFailed)
}

impl EncryptedValue {
    /// Serialize to compact binary wire format.
    ///
    /// Layout: `magic(4) || version(1) || epoch_be(8) || nonce(12) || ciphertext+tag(N)`
    pub fn to_bytes(&self) -> Vec<u8> {
        let header_len = 4usize.saturating_add(1).saturating_add(8).saturating_add(12);
        let mut buf = Vec::with_capacity(header_len.saturating_add(self.ciphertext.len()));
        buf.extend_from_slice(&ENVELOPE_MAGIC);
        buf.push(self.version);
        buf.extend_from_slice(&self.epoch.to_be_bytes());
        buf.extend_from_slice(&self.nonce);
        buf.extend_from_slice(&self.ciphertext);
        buf
    }

    /// Deserialize from compact binary wire format.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, EnvelopeError> {
        if bytes.len() < MIN_SERIALIZED_LEN {
            return Err(EnvelopeError::TooShort { len: bytes.len() });
        }

        // Check magic header
        if bytes[0..4] != ENVELOPE_MAGIC {
            return Err(EnvelopeError::UnsupportedVersion { version: bytes[0] });
        }

        let version = bytes[4];
        if version != ENVELOPE_VERSION {
            return Err(EnvelopeError::UnsupportedVersion { version });
        }

        let epoch = u64::from_be_bytes(bytes[5..13].try_into().expect("8 bytes for u64"));
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&bytes[13..25]);
        let ciphertext = bytes[25..].to_vec();

        Ok(Self {
            version,
            epoch,
            nonce,
            ciphertext,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        let mut k = [0u8; 32];
        k[0] = 0xAB;
        k[31] = 0xCD;
        k
    }

    fn test_nonce() -> [u8; 12] {
        let mut n = [0u8; 12];
        n[0] = 1;
        n
    }

    #[test]
    fn test_roundtrip_encrypt_decrypt() {
        let key = test_key();
        let nonce = test_nonce();
        let plaintext = b"cluster-secret-data-here";

        let encrypted = encrypt_value(&key, 42, &nonce, plaintext).unwrap();
        let decrypted = decrypt_value(&key, &encrypted).unwrap();

        assert_eq!(&decrypted, plaintext);
        assert_eq!(encrypted.epoch, 42);
        assert_eq!(encrypted.version, ENVELOPE_VERSION);
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let key = test_key();
        let nonce = test_nonce();
        let plaintext = b"secret";

        let mut encrypted = encrypt_value(&key, 1, &nonce, plaintext).unwrap();
        // Flip a bit in the ciphertext
        if let Some(byte) = encrypted.ciphertext.first_mut() {
            *byte ^= 0xFF;
        }

        let result = decrypt_value(&key, &encrypted);
        assert!(matches!(result, Err(EnvelopeError::DecryptFailed)));
    }

    #[test]
    fn test_wrong_key_fails() {
        let key = test_key();
        let nonce = test_nonce();
        let plaintext = b"secret";

        let encrypted = encrypt_value(&key, 1, &nonce, plaintext).unwrap();

        let mut wrong_key = [0u8; 32];
        wrong_key[0] = 0xFF;
        let result = decrypt_value(&wrong_key, &encrypted);
        assert!(matches!(result, Err(EnvelopeError::DecryptFailed)));
    }

    #[test]
    fn test_unknown_version_fails() {
        let key = test_key();
        let nonce = test_nonce();

        let bad = EncryptedValue {
            version: 99,
            epoch: 0,
            nonce,
            ciphertext: vec![0u8; 32],
        };
        let result = decrypt_value(&key, &bad);
        assert!(matches!(result, Err(EnvelopeError::UnsupportedVersion { version: 99 })));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let key = test_key();
        let nonce = test_nonce();
        let plaintext = b"roundtrip-test";

        let encrypted = encrypt_value(&key, 7, &nonce, plaintext).unwrap();
        let bytes = encrypted.to_bytes();
        let deserialized = EncryptedValue::from_bytes(&bytes).unwrap();

        assert_eq!(encrypted, deserialized);

        // Decrypt the deserialized value
        let decrypted = decrypt_value(&key, &deserialized).unwrap();
        assert_eq!(&decrypted, plaintext);
    }

    #[test]
    fn test_from_bytes_too_short() {
        let result = EncryptedValue::from_bytes(&[1, 2, 3]);
        assert!(matches!(result, Err(EnvelopeError::TooShort { .. })));
    }

    #[test]
    fn test_from_bytes_bad_magic() {
        let mut bytes = vec![0u8; MIN_SERIALIZED_LEN.saturating_add(1)];
        bytes[0] = 42; // bad magic
        let result = EncryptedValue::from_bytes(&bytes);
        assert!(matches!(result, Err(EnvelopeError::UnsupportedVersion { .. })));
    }

    #[test]
    fn test_from_bytes_bad_version() {
        let mut bytes = vec![0u8; MIN_SERIALIZED_LEN.saturating_add(1)];
        // Correct magic, bad version
        bytes[0..4].copy_from_slice(&ENVELOPE_MAGIC);
        bytes[4] = 99; // bad version
        let result = EncryptedValue::from_bytes(&bytes);
        assert!(matches!(result, Err(EnvelopeError::UnsupportedVersion { version: 99 })));
    }

    #[test]
    fn test_empty_plaintext() {
        let key = test_key();
        let nonce = test_nonce();

        let encrypted = encrypt_value(&key, 0, &nonce, b"").unwrap();
        let decrypted = decrypt_value(&key, &encrypted).unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn test_different_nonces_produce_different_ciphertext() {
        let key = test_key();
        let plaintext = b"same plaintext";

        let mut n1 = [0u8; 12];
        n1[0] = 1;
        let mut n2 = [0u8; 12];
        n2[0] = 2;

        let e1 = encrypt_value(&key, 0, &n1, plaintext).unwrap();
        let e2 = encrypt_value(&key, 0, &n2, plaintext).unwrap();

        assert_ne!(e1.ciphertext, e2.ciphertext);
    }
}
