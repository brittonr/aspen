//! SOPS MAC verification and encryption (imperative shell).
//!
//! Uses the verified pure functions from `verified::mac` for computation,
//! and wraps them with I/O for encrypting/decrypting the MAC value.

use subtle::ConstantTimeEq;

use super::format::common::decrypt_sops_value;
use super::format::common::encrypt_sops_value;
use super::sops_error::Result;
use super::sops_error::SopsError;
use crate::verified::mac::compute_sops_mac;

/// Verify a SOPS MAC against the file's plaintext values.
///
/// 1. Decrypts the stored MAC using the data key
/// 2. Recomputes the MAC over the provided values
/// 3. Compares in constant time to prevent timing attacks
///
/// # Errors
/// Returns `MacVerificationFailed` if the MAC doesn't match.
pub fn verify_mac(encrypted_mac: &str, data_key: &[u8; 32], values: &[(String, String)]) -> Result<()> {
    // Decrypt the stored MAC
    let stored_mac_hex = decrypt_sops_value(encrypted_mac, data_key).map_err(|_| SopsError::MacVerificationFailed)?;

    // Decode the hex-encoded MAC
    let stored_mac_bytes: Vec<u8> =
        hex::decode(stored_mac_hex.as_bytes()).map_err(|_| SopsError::MacVerificationFailed)?;

    if stored_mac_bytes.len() != 32 {
        return Err(SopsError::MacVerificationFailed);
    }

    // Recompute MAC
    let computed_mac = compute_sops_mac(values, data_key);

    // Constant-time comparison
    if stored_mac_bytes.ct_eq(&computed_mac).into() {
        Ok(())
    } else {
        Err(SopsError::MacVerificationFailed)
    }
}

/// Compute and encrypt a SOPS MAC.
///
/// 1. Computes HMAC-SHA256 over all values
/// 2. Hex-encodes the MAC
/// 3. Encrypts the hex string as a SOPS `ENC[AES256_GCM,...]` value
///
/// # Returns
/// Encrypted MAC string in SOPS format.
pub fn encrypt_mac(data_key: &[u8; 32], values: &[(String, String)]) -> Result<String> {
    let mac = compute_sops_mac(values, data_key);
    let mac_hex = hex::encode(mac);

    encrypt_sops_value(&mac_hex, data_key, "str").map_err(|e| SopsError::Serialization {
        reason: format!("failed to encrypt MAC: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_verify_mac_roundtrip() {
        let key = [42u8; 32];
        let values = vec![
            ("a.secret".to_string(), "password123".to_string()),
            ("b.token".to_string(), "tok-abc".to_string()),
        ];

        let encrypted = encrypt_mac(&key, &values).unwrap();
        assert!(encrypted.starts_with("ENC[AES256_GCM,"));

        // Verify should succeed with same values
        verify_mac(&encrypted, &key, &values).unwrap();
    }

    #[test]
    fn test_mac_verification_fails_on_tampered_values() {
        let key = [42u8; 32];
        let original = vec![("key".to_string(), "original".to_string())];
        let tampered = vec![("key".to_string(), "tampered".to_string())];

        let encrypted = encrypt_mac(&key, &original).unwrap();

        // Verify with tampered values should fail
        let result = verify_mac(&encrypted, &key, &tampered);
        assert!(result.is_err());
    }

    #[test]
    fn test_mac_verification_fails_on_wrong_key() {
        let key1 = [42u8; 32];
        let key2 = [99u8; 32];
        let values = vec![("key".to_string(), "value".to_string())];

        let encrypted = encrypt_mac(&key1, &values).unwrap();

        // Verify with wrong key should fail (decryption will fail)
        let result = verify_mac(&encrypted, &key2, &values);
        assert!(result.is_err());
    }
}
