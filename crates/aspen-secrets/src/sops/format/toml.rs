//! TOML format: encrypt/decrypt individual values in a TOML document.
//!
//! Uses `toml_edit` to preserve document structure (comments, formatting)
//! while encrypting/decrypting leaf values in-place.

use toml_edit::DocumentMut;
use toml_edit::Item;
use toml_edit::Value;

use super::common::build_aad;
use super::common::check_value_count;
// Re-export common functions for backwards compatibility
pub use super::common::decrypt_sops_value;
use super::common::decrypt_sops_value_with_type;
use super::common::encrypt_sops_value;
pub use super::common::encrypt_sops_value as encrypt_value;
use super::common::is_sops_encrypted;
pub use super::common::is_sops_encrypted as is_encrypted;
use super::common::validate_key_path;
use crate::sops::metadata::SopsFileMetadata;
use crate::sops::sops_error::Result;
use crate::sops::sops_error::SopsError;

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

/// Parse a TOML string, encrypt values, inject metadata, and serialize.
///
/// Returns `(encrypted_output, value_pairs_for_mac)`.
pub fn encrypt_document(
    contents: &str,
    data_key: &[u8; 32],
    encrypted_regex: Option<&str>,
    metadata: &SopsFileMetadata,
    input_path: &std::path::Path,
) -> Result<(String, Vec<(String, String)>)> {
    let mut doc: DocumentMut = contents.parse().map_err(|e: toml_edit::TomlError| SopsError::ParseFile {
        path: input_path.to_path_buf(),
        reason: e.to_string(),
    })?;

    let values = encrypt_toml_values(&mut doc, data_key, encrypted_regex)?;
    inject_metadata(&mut doc, metadata)?;

    Ok((doc.to_string(), values))
}

/// Parse a TOML string, extract metadata, decrypt values, and serialize.
///
/// Returns `(decrypted_output, metadata, value_pairs_for_mac)`.
pub fn decrypt_document(
    contents: &str,
    data_key: &[u8; 32],
    input_path: &std::path::Path,
) -> Result<(String, Vec<(String, String)>)> {
    let mut doc: DocumentMut = contents.parse().map_err(|e: toml_edit::TomlError| SopsError::ParseFile {
        path: input_path.to_path_buf(),
        reason: e.to_string(),
    })?;

    let values = decrypt_toml_values(&mut doc, data_key)?;
    doc.remove("sops");

    Ok((doc.to_string(), values))
}

/// Extract SOPS metadata from TOML contents.
pub fn extract_metadata_from_contents(
    contents: &str,
    input_path: &std::path::Path,
) -> Result<Option<SopsFileMetadata>> {
    let parsed: toml::Value = toml::from_str(contents).map_err(|e| SopsError::ParseFile {
        path: input_path.to_path_buf(),
        reason: e.to_string(),
    })?;
    crate::sops::metadata::extract_metadata_from_toml(&parsed)
}

/// Inject SOPS metadata into a TOML document.
pub fn inject_metadata(doc: &mut DocumentMut, metadata: &SopsFileMetadata) -> Result<()> {
    // Serialize metadata to a toml::Value, then convert to a string representation
    // of a full document where the metadata is under a [sops] key. This avoids the
    // TOML parsing issue where [[aspen_transit]] array-of-tables headers at root level
    // are not nested under [sops] when we just prefix with "[sops]\n".
    let sops_value = toml::Value::try_from(metadata).map_err(|e| SopsError::Serialization {
        reason: format!("failed to serialize SOPS metadata: {e}"),
    })?;

    // Wrap in a top-level table with "sops" key so array-of-tables serialize correctly
    let mut wrapper = toml::map::Map::new();
    wrapper.insert("sops".to_string(), sops_value);
    let wrapper_str = toml::to_string_pretty(&wrapper).map_err(|e| SopsError::Serialization {
        reason: format!("failed to format SOPS metadata wrapper: {e}"),
    })?;

    let sops_doc: DocumentMut = wrapper_str.parse().map_err(|e| SopsError::Serialization {
        reason: format!("failed to parse SOPS metadata back: {e}"),
    })?;

    if let Some(sops_item) = sops_doc.get("sops") {
        doc["sops"] = sops_item.clone();
    }
    Ok(())
}

/// Update metadata in an existing TOML document (for rotate/updatekeys).
pub fn update_metadata_in_document(
    contents: &str,
    metadata: &SopsFileMetadata,
    input_path: &std::path::Path,
) -> Result<String> {
    let mut doc: DocumentMut = contents.parse().map_err(|e: toml_edit::TomlError| SopsError::ParseFile {
        path: input_path.to_path_buf(),
        reason: e.to_string(),
    })?;

    inject_metadata(&mut doc, metadata)?;
    Ok(doc.to_string())
}

/// Extract a single value from decrypted TOML by dotted path.
pub fn extract_value(toml_str: &str, path: &str) -> Result<String> {
    let value: toml::Value = toml::from_str(toml_str).map_err(|e| SopsError::ParseFile {
        path: std::path::PathBuf::from("<decrypted>"),
        reason: e.to_string(),
    })?;

    let mut current = &value;
    for segment in path.split('.') {
        current = current.get(segment).ok_or_else(|| SopsError::ParseFile {
            path: std::path::PathBuf::from("<decrypted>"),
            reason: format!("key path '{path}' not found at segment '{segment}'"),
        })?;
    }

    match current {
        toml::Value::String(s) => Ok(s.clone()),
        other => Ok(other.to_string()),
    }
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
                check_value_count(count)?;

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
                    let aad = build_aad(path);
                    let (plaintext, value_type) = decrypt_sops_value_with_type(s, data_key, &aad)?;
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
