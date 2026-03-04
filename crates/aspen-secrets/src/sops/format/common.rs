//! Format-agnostic SOPS value encryption/decryption.
//!
//! These functions work on individual string values and are shared
//! across all file formats (TOML, JSON, YAML).

use aes_gcm::Aes256Gcm;
use aes_gcm::aead::Aead;
use aes_gcm::aead::KeyInit;
use aes_gcm::aead::generic_array::GenericArray;
use base64::Engine;
use rand::RngCore;

use crate::sops::sops_constants::AES_GCM_NONCE_SIZE;
use crate::sops::sops_constants::AES_GCM_TAG_SIZE;
use crate::sops::sops_constants::MAX_KEY_PATH_LENGTH;
use crate::sops::sops_constants::MAX_VALUE_COUNT;
use crate::sops::sops_error::Result;
use crate::sops::sops_error::SopsError;

/// Check if a string is SOPS-encrypted.
pub fn is_sops_encrypted(s: &str) -> bool {
    s.starts_with("ENC[") && s.ends_with(']')
}

/// Encrypt a single plaintext value to SOPS format.
///
/// Produces: `ENC[AES256_GCM,data:<base64>,iv:<base64>,tag:<base64>,type:<type>]`
pub fn encrypt_sops_value(plaintext: &str, data_key: &[u8; 32], value_type: &str) -> Result<String> {
    let cipher = Aes256Gcm::new(GenericArray::from_slice(data_key));

    let mut nonce_bytes = [0u8; AES_GCM_NONCE_SIZE];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = GenericArray::from_slice(&nonce_bytes);

    let ciphertext_with_tag = cipher.encrypt(nonce, plaintext.as_bytes()).map_err(|e| SopsError::ValueEncrypt {
        key_path: String::new(),
        reason: format!("AES-GCM encryption failed: {e}"),
    })?;

    // AES-GCM appends 16-byte tag to ciphertext
    if ciphertext_with_tag.len() < AES_GCM_TAG_SIZE {
        return Err(SopsError::ValueEncrypt {
            key_path: String::new(),
            reason: "ciphertext too short".into(),
        });
    }

    let ct_len = ciphertext_with_tag.len() - AES_GCM_TAG_SIZE;
    let (ct_data, tag_data) = ciphertext_with_tag.split_at(ct_len);

    let b64 = base64::engine::general_purpose::STANDARD;
    let data_b64 = b64.encode(ct_data);
    let iv_b64 = b64.encode(nonce_bytes);
    let tag_b64 = b64.encode(tag_data);

    Ok(format!("ENC[AES256_GCM,data:{data_b64},iv:{iv_b64},tag:{tag_b64},type:{value_type}]"))
}

/// Decrypt a single SOPS-encrypted value, returning plaintext.
pub fn decrypt_sops_value(encrypted: &str, data_key: &[u8; 32]) -> Result<String> {
    let (plaintext, _type) = decrypt_sops_value_with_type(encrypted, data_key)?;
    Ok(plaintext)
}

/// Decrypt a SOPS value, returning both plaintext and the type tag.
pub fn decrypt_sops_value_with_type(encrypted: &str, data_key: &[u8; 32]) -> Result<(String, String)> {
    let inner = encrypted.strip_prefix("ENC[").and_then(|s| s.strip_suffix(']')).ok_or_else(|| {
        SopsError::InvalidCiphertext {
            reason: "not in ENC[...] format".into(),
        }
    })?;

    let parts: std::collections::HashMap<&str, &str> = inner
        .split(',')
        .filter_map(|part| {
            let mut kv = part.splitn(2, ':');
            Some((kv.next()?, kv.next()?))
        })
        .collect();

    // Verify cipher type
    let first_part = inner.split(',').next().unwrap_or("");
    if !first_part.starts_with("AES256_GCM") {
        return Err(SopsError::InvalidCiphertext {
            reason: format!("unsupported cipher: {first_part}"),
        });
    }

    let b64 = base64::engine::general_purpose::STANDARD;

    let data = b64
        .decode(parts.get("data").ok_or_else(|| SopsError::InvalidCiphertext {
            reason: "missing 'data'".into(),
        })?)
        .map_err(|e| SopsError::InvalidCiphertext {
            reason: format!("invalid base64 in data: {e}"),
        })?;

    let iv = b64
        .decode(parts.get("iv").ok_or_else(|| SopsError::InvalidCiphertext {
            reason: "missing 'iv'".into(),
        })?)
        .map_err(|e| SopsError::InvalidCiphertext {
            reason: format!("invalid base64 in iv: {e}"),
        })?;

    let tag = b64
        .decode(parts.get("tag").ok_or_else(|| SopsError::InvalidCiphertext {
            reason: "missing 'tag'".into(),
        })?)
        .map_err(|e| SopsError::InvalidCiphertext {
            reason: format!("invalid base64 in tag: {e}"),
        })?;

    let value_type = parts.get("type").unwrap_or(&"str").to_string();

    if iv.len() != AES_GCM_NONCE_SIZE {
        return Err(SopsError::InvalidCiphertext {
            reason: format!("invalid IV length: {} (expected {})", iv.len(), AES_GCM_NONCE_SIZE),
        });
    }
    if tag.len() != AES_GCM_TAG_SIZE {
        return Err(SopsError::InvalidCiphertext {
            reason: format!("invalid tag length: {} (expected {})", tag.len(), AES_GCM_TAG_SIZE),
        });
    }

    let cipher = Aes256Gcm::new(GenericArray::from_slice(data_key));
    let nonce = GenericArray::from_slice(&iv);

    // Combine ciphertext + tag (AEAD expects tag appended)
    let mut combined = data;
    combined.extend_from_slice(&tag);

    let plaintext = cipher.decrypt(nonce, combined.as_ref()).map_err(|e| SopsError::InvalidCiphertext {
        reason: format!("AES-GCM decryption failed: {e}"),
    })?;

    let plaintext_str = String::from_utf8(plaintext).map_err(|e| SopsError::InvalidCiphertext {
        reason: format!("decrypted value is not valid UTF-8: {e}"),
    })?;

    Ok((plaintext_str, value_type))
}

/// Validate that a key path is within resource bounds.
pub fn validate_key_path(path: &str) -> Result<()> {
    if path.len() as u32 > MAX_KEY_PATH_LENGTH {
        return Err(SopsError::KeyPathTooLong {
            length: path.len() as u32,
            max: MAX_KEY_PATH_LENGTH,
        });
    }
    Ok(())
}

/// Increment a value counter and check resource bounds.
pub fn check_value_count(count: &mut u32) -> Result<()> {
    *count = count.checked_add(1).ok_or(SopsError::TooManyValues {
        count: u32::MAX,
        max: MAX_VALUE_COUNT,
    })?;
    if *count > MAX_VALUE_COUNT {
        return Err(SopsError::TooManyValues {
            count: *count,
            max: MAX_VALUE_COUNT,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [42u8; 32];
        let encrypted = encrypt_sops_value("hello world", &key, "str").unwrap();
        assert!(is_sops_encrypted(&encrypted));
        assert!(encrypted.contains("AES256_GCM"));
        assert!(encrypted.contains("type:str"));

        let decrypted = decrypt_sops_value(&encrypted, &key).unwrap();
        assert_eq!(decrypted, "hello world");
    }

    #[test]
    fn test_decrypt_with_type() {
        let key = [42u8; 32];
        let encrypted = encrypt_sops_value("42", &key, "int").unwrap();
        let (val, typ) = decrypt_sops_value_with_type(&encrypted, &key).unwrap();
        assert_eq!(val, "42");
        assert_eq!(typ, "int");
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = [42u8; 32];
        let key2 = [99u8; 32];
        let encrypted = encrypt_sops_value("secret", &key1, "str").unwrap();
        assert!(decrypt_sops_value(&encrypted, &key2).is_err());
    }

    #[test]
    fn test_is_sops_encrypted() {
        assert!(is_sops_encrypted("ENC[AES256_GCM,data:abc,iv:def,tag:ghi,type:str]"));
        assert!(!is_sops_encrypted("plaintext"));
        assert!(!is_sops_encrypted("ENC[incomplete"));
    }

    #[test]
    fn test_validate_key_path() {
        assert!(validate_key_path("a.b.c").is_ok());
        let long = "x".repeat(MAX_KEY_PATH_LENGTH as usize + 1);
        assert!(validate_key_path(&long).is_err());
    }
}
