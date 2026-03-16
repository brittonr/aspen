//! SOPS MAC verification and encryption (imperative shell).
//!
//! Uses the verified pure functions from `verified::mac` for computation,
//! and wraps them with I/O for encrypting/decrypting the MAC value.

use sha2::Digest;
use sha2::Sha512;
use subtle::ConstantTimeEq;

use super::format::common::decrypt_sops_value_with_type;
use super::format::common::encrypt_sops_value;
use super::sops_error::Result;
use super::sops_error::SopsError;
use crate::verified::mac::compute_sops_mac;

/// Verify a SOPS MAC against the file's plaintext values.
///
/// Supports two MAC formats:
/// - **aspen-sops**: HMAC-SHA256 over sorted (path, value) pairs → 32-byte digest
/// - **Go sops**: SHA-512 over plaintext values in tree-walk order → 64-byte digest
///
/// The stored MAC is hex-encoded and encrypted as a SOPS value. Go sops uses
/// the `lastmodified` timestamp as AAD for the MAC encryption; aspen-sops
/// uses no AAD (auto-detected from nonce length).
pub fn verify_mac(
    encrypted_mac: &str,
    data_key: &[u8; 32],
    values: &[(String, String)],
    lastmodified: &str,
) -> Result<()> {
    // Go sops uses lastmodified as AAD for the MAC encryption
    let aad = lastmodified.as_bytes();
    let (stored_mac_hex, _type) =
        decrypt_sops_value_with_type(encrypted_mac, data_key, aad).map_err(|_| SopsError::MacVerificationFailed)?;

    let stored_mac_bytes: Vec<u8> =
        hex::decode(stored_mac_hex.as_bytes()).map_err(|_| SopsError::MacVerificationFailed)?;
    match stored_mac_bytes.len() {
        32 => {
            // aspen-sops: HMAC-SHA256 (values must be sorted by key path)
            let mut sorted = values.to_vec();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            let computed = compute_sops_mac(&sorted, data_key);
            if stored_mac_bytes.ct_eq(&computed).into() {
                Ok(())
            } else {
                Err(SopsError::MacVerificationFailed)
            }
        }
        64 => {
            // Go sops: SHA-512 over plaintext values only (no key paths,
            // tree-walk order). Values arrive pre-sorted by key path which
            // matches Go sops's tree walk for simple documents.
            let computed = compute_go_sops_mac(values);
            if stored_mac_bytes.ct_eq(&computed).into() {
                Ok(())
            } else {
                Err(SopsError::MacVerificationFailed)
            }
        }
        _ => Err(SopsError::MacVerificationFailed),
    }
}

/// Compute Go sops-compatible MAC: SHA-512 over plaintext values.
///
/// Go sops writes each plaintext value (converted to bytes via `ToBytes`)
/// into a SHA-512 hash in tree-walk order. For flat/simple documents,
/// this is the same order as alphabetical key sort.
fn compute_go_sops_mac(values: &[(String, String)]) -> [u8; 64] {
    let mut hasher = Sha512::new();
    for (_path, value) in values {
        hasher.update(value.as_bytes());
    }
    hasher.finalize().into()
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

        // Verify should succeed with same values (empty lastmodified for aspen-sops)
        verify_mac(&encrypted, &key, &values, "").unwrap();
    }

    #[test]
    fn test_mac_verification_fails_on_tampered_values() {
        let key = [42u8; 32];
        let original = vec![("key".to_string(), "original".to_string())];
        let tampered = vec![("key".to_string(), "tampered".to_string())];

        let encrypted = encrypt_mac(&key, &original).unwrap();

        // Verify with tampered values should fail
        let result = verify_mac(&encrypted, &key, &tampered, "");
        assert!(result.is_err());
    }

    #[test]
    fn test_mac_verification_fails_on_wrong_key() {
        let key1 = [42u8; 32];
        let key2 = [99u8; 32];
        let values = vec![("key".to_string(), "value".to_string())];

        let encrypted = encrypt_mac(&key1, &values).unwrap();

        // Verify with wrong key should fail (decryption will fail)
        let result = verify_mac(&encrypted, &key2, &values, "");
        assert!(result.is_err());
    }
}
