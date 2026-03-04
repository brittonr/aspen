//! TOML format: encrypt/decrypt individual values in a TOML document.
//!
//! Uses `toml_edit` to preserve document structure (comments, formatting)
//! while encrypting/decrypting leaf values in-place.

use aes_gcm::Aes256Gcm;
use aes_gcm::aead::Aead;
use aes_gcm::aead::KeyInit;
use aes_gcm::aead::generic_array::GenericArray;
use base64::Engine;
use rand::RngCore;
use toml_edit::DocumentMut;
use toml_edit::Item;
use toml_edit::Value;

use crate::constants::AES_GCM_NONCE_SIZE;
use crate::constants::AES_GCM_TAG_SIZE;
use crate::constants::MAX_KEY_PATH_LENGTH;
use crate::constants::MAX_VALUE_COUNT;
use crate::error::Result;
use crate::error::SopsError;

/// Encrypt all leaf values in a TOML document.
///
/// Walks the TOML tree, encrypting string/integer/float/bool values to
/// `ENC[AES256_GCM,data:...,iv:...,tag:...,type:<type>]` format.
/// Skips the `[sops]` table. Returns collected `(path, plaintext)` pairs
/// for MAC computation.
///
/// If `encrypted_regex` is provided, only values whose key path matches
/// the regex are encrypted; others are left as plaintext.
pub fn encrypt_toml_values(
    doc: &mut DocumentMut,
    data_key: &[u8; 32],
    encrypted_regex: Option<&str>,
) -> Result<Vec<(String, String)>> {
    let regex = encrypted_regex
        .map(|r| {
            regex::Regex::new(r).map_err(|e| SopsError::InvalidFormat {
                reason: format!("invalid encrypted_regex: {e}"),
            })
        })
        .transpose()?;

    let mut values = Vec::new();
    let mut count: u32 = 0;

    // Collect keys first to avoid borrow issues
    let keys: Vec<String> = doc.as_table().iter().map(|(k, _)| k.to_string()).collect();

    for key in &keys {
        if key == "sops" {
            continue;
        }
        if let Some(item) = doc.get_mut(key) {
            encrypt_item(item, key, data_key, regex.as_ref(), &mut values, &mut count)?;
        }
    }

    values.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(values)
}

/// Decrypt all `ENC[...]` values in a TOML document.
///
/// Walks the TOML tree, decrypting encrypted values back to plaintext.
/// Returns collected `(path, plaintext)` pairs for MAC verification.
pub fn decrypt_toml_values(doc: &mut DocumentMut, data_key: &[u8; 32]) -> Result<Vec<(String, String)>> {
    let mut values = Vec::new();

    let keys: Vec<String> = doc.as_table().iter().map(|(k, _)| k.to_string()).collect();

    for key in &keys {
        if key == "sops" {
            continue;
        }
        if let Some(item) = doc.get_mut(key) {
            decrypt_item(item, key, data_key, &mut values)?;
        }
    }

    values.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(values)
}

fn encrypt_item(
    item: &mut Item,
    path: &str,
    data_key: &[u8; 32],
    regex: Option<&regex::Regex>,
    values: &mut Vec<(String, String)>,
    count: &mut u32,
) -> Result<()> {
    validate_key_path(path)?;

    match item {
        Item::Value(val) => {
            let (plaintext, value_type) = value_to_string(val);
            if let Some((plaintext, value_type)) = plaintext.zip(value_type) {
                // Check regex filter
                let should_encrypt = regex.as_ref().is_none_or(|re| re.is_match(path));

                values.push((path.to_string(), plaintext.clone()));
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

                if should_encrypt {
                    let encrypted = encrypt_sops_value(&plaintext, data_key, value_type)?;
                    *val = Value::from(encrypted);
                }
            }
        }
        Item::Table(table) => {
            let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
            for key in keys {
                let child_path = format!("{path}.{key}");
                if let Some(child) = table.get_mut(&key) {
                    encrypt_item(child, &child_path, data_key, regex, values, count)?;
                }
            }
        }
        Item::ArrayOfTables(arr) => {
            for (i, table) in arr.iter_mut().enumerate() {
                let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
                for key in keys {
                    let child_path = format!("{path}[{i}].{key}");
                    if let Some(child) = table.get_mut(&key) {
                        encrypt_item(child, &child_path, data_key, regex, values, count)?;
                    }
                }
            }
        }
        Item::None => {}
    }
    Ok(())
}

fn decrypt_item(item: &mut Item, path: &str, data_key: &[u8; 32], values: &mut Vec<(String, String)>) -> Result<()> {
    match item {
        Item::Value(val) => {
            if let Some(s) = val.as_str() {
                if is_sops_encrypted(s) {
                    let (plaintext, value_type) = decrypt_sops_value_with_type(s, data_key)?;
                    values.push((path.to_string(), plaintext.clone()));

                    // Restore the original TOML type
                    *val = restore_typed_value(&plaintext, &value_type);
                } else {
                    // Unencrypted value — still collect for MAC
                    let (pt, _) = value_to_string(val);
                    if let Some(pt) = pt {
                        values.push((path.to_string(), pt));
                    }
                }
            } else {
                // Non-string values (shouldn't be encrypted)
                let (pt, _) = value_to_string(val);
                if let Some(pt) = pt {
                    values.push((path.to_string(), pt));
                }
            }
        }
        Item::Table(table) => {
            let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
            for key in keys {
                let child_path = format!("{path}.{key}");
                if let Some(child) = table.get_mut(&key) {
                    decrypt_item(child, &child_path, data_key, values)?;
                }
            }
        }
        Item::ArrayOfTables(arr) => {
            for (i, table) in arr.iter_mut().enumerate() {
                let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
                for key in keys {
                    let child_path = format!("{path}[{i}].{key}");
                    if let Some(child) = table.get_mut(&key) {
                        decrypt_item(child, &child_path, data_key, values)?;
                    }
                }
            }
        }
        Item::None => {}
    }
    Ok(())
}

/// Extract a string representation and SOPS type from a TOML value.
fn value_to_string(val: &Value) -> (Option<String>, Option<&'static str>) {
    match val {
        Value::String(s) => (Some(s.value().clone()), Some("str")),
        Value::Integer(n) => (Some(n.value().to_string()), Some("int")),
        Value::Float(f) => (Some(f.value().to_string()), Some("float")),
        Value::Boolean(b) => (Some(b.value().to_string()), Some("bool")),
        _ => (None, None),
    }
}

/// Restore a typed TOML value from a plaintext string and type tag.
fn restore_typed_value(plaintext: &str, value_type: &str) -> Value {
    match value_type {
        "int" => plaintext.parse::<i64>().map(Value::from).unwrap_or_else(|_| Value::from(plaintext)),
        "float" => plaintext.parse::<f64>().map(Value::from).unwrap_or_else(|_| Value::from(plaintext)),
        "bool" => plaintext.parse::<bool>().map(Value::from).unwrap_or_else(|_| Value::from(plaintext)),
        _ => Value::from(plaintext),
    }
}

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
fn decrypt_sops_value_with_type(encrypted: &str, data_key: &[u8; 32]) -> Result<(String, String)> {
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

fn validate_key_path(path: &str) -> Result<()> {
    if path.len() as u32 > MAX_KEY_PATH_LENGTH {
        return Err(SopsError::KeyPathTooLong {
            length: path.len() as u32,
            max: MAX_KEY_PATH_LENGTH,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_string() {
        let key = [42u8; 32];
        let encrypted = encrypt_sops_value("hello world", &key, "str").unwrap();
        assert!(is_sops_encrypted(&encrypted));
        assert!(encrypted.contains("AES256_GCM"));
        assert!(encrypted.contains("type:str"));

        let decrypted = decrypt_sops_value(&encrypted, &key).unwrap();
        assert_eq!(decrypted, "hello world");
    }

    #[test]
    fn test_encrypt_decrypt_integer() {
        let key = [42u8; 32];
        let encrypted = encrypt_sops_value("42", &key, "int").unwrap();
        assert!(encrypted.contains("type:int"));

        let decrypted = decrypt_sops_value(&encrypted, &key).unwrap();
        assert_eq!(decrypted, "42");
    }

    #[test]
    fn test_encrypt_decrypt_float() {
        let key = [42u8; 32];
        let encrypted = encrypt_sops_value("3.14", &key, "float").unwrap();
        assert!(encrypted.contains("type:float"));

        let decrypted = decrypt_sops_value(&encrypted, &key).unwrap();
        assert_eq!(decrypted, "3.14");
    }

    #[test]
    fn test_encrypt_decrypt_bool() {
        let key = [42u8; 32];
        let encrypted = encrypt_sops_value("true", &key, "bool").unwrap();
        assert!(encrypted.contains("type:bool"));

        let decrypted = decrypt_sops_value(&encrypted, &key).unwrap();
        assert_eq!(decrypted, "true");
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = [42u8; 32];
        let key2 = [99u8; 32];
        let encrypted = encrypt_sops_value("secret", &key1, "str").unwrap();

        let result = decrypt_sops_value(&encrypted, &key2);
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_decrypt_toml_document() {
        let key = [42u8; 32];
        let toml_str = r#"
api_key = "sk-test-123"
port = 8080
debug = true
rate = 1.5

[database]
password = "s3cret"
"#;

        let mut doc: DocumentMut = toml_str.parse().unwrap();

        // Encrypt
        let values = encrypt_toml_values(&mut doc, &key, None).unwrap();
        assert_eq!(values.len(), 5);

        let encrypted_str = doc.to_string();
        assert!(encrypted_str.contains("ENC[AES256_GCM,"));
        // Keys should still be visible
        assert!(encrypted_str.contains("api_key"));
        assert!(encrypted_str.contains("password"));

        // Decrypt
        let mut doc2: DocumentMut = encrypted_str.parse().unwrap();
        let dec_values = decrypt_toml_values(&mut doc2, &key).unwrap();

        // Values should match (sorted)
        assert_eq!(values, dec_values);

        // Check actual decrypted values
        let decrypted_str = doc2.to_string();
        assert!(decrypted_str.contains("sk-test-123"));
        assert!(decrypted_str.contains("s3cret"));
    }

    #[test]
    fn test_encrypted_regex_filter() {
        let key = [42u8; 32];
        let toml_str = r#"
api_key = "secret"
description = "not secret"
password = "also secret"
"#;

        let mut doc: DocumentMut = toml_str.parse().unwrap();
        let _values = encrypt_toml_values(&mut doc, &key, Some("key|password")).unwrap();

        let result = doc.to_string();
        // api_key and password should be encrypted
        assert!(doc["api_key"].as_str().unwrap().starts_with("ENC["));
        assert!(doc["password"].as_str().unwrap().starts_with("ENC["));
        // description should remain plaintext
        assert_eq!(doc["description"].as_str().unwrap(), "not secret");
    }

    #[test]
    fn test_is_sops_encrypted() {
        assert!(is_sops_encrypted("ENC[AES256_GCM,data:abc,iv:def,tag:ghi,type:str]"));
        assert!(!is_sops_encrypted("plaintext"));
        assert!(!is_sops_encrypted("ENC[incomplete"));
    }

    #[test]
    fn test_restore_typed_value() {
        let v = restore_typed_value("42", "int");
        assert_eq!(v.as_integer(), Some(42));

        let v = restore_typed_value("3.14", "float");
        assert!(v.as_float().is_some());

        let v = restore_typed_value("true", "bool");
        assert_eq!(v.as_bool(), Some(true));

        let v = restore_typed_value("hello", "str");
        assert_eq!(v.as_str(), Some("hello"));
    }
}
