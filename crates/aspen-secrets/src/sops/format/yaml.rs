//! YAML format: encrypt/decrypt individual values in a YAML document.
//!
//! Uses `serde_yaml` to walk the YAML tree, encrypting/decrypting
//! leaf values in-place. Preserves YAML structure (mappings, sequences).

use serde_yaml::Value;

use super::common::check_value_count;
use super::common::decrypt_sops_value_with_type;
use super::common::encrypt_sops_value;
use super::common::is_sops_encrypted;
use super::common::validate_key_path;
use crate::sops::metadata::SopsFileMetadata;
use crate::sops::sops_error::Result;
use crate::sops::sops_error::SopsError;

/// Encrypt all leaf values in a YAML document.
///
/// Walks the YAML tree, encrypting string/number/bool values to
/// `ENC[AES256_GCM,data:...,iv:...,tag:...,type:<type>]` format.
/// Skips the `sops` key. Returns collected `(path, plaintext)` pairs
/// for MAC computation.
pub fn encrypt_yaml_values(
    root: &mut Value,
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

    if let Value::Mapping(map) = root {
        let keys: Vec<Value> = map.keys().cloned().collect();
        for key in keys {
            let key_str = yaml_key_to_string(&key);
            if key_str == "sops" {
                continue;
            }
            if let Some(item) = map.get_mut(&key) {
                encrypt_value_recursive(item, &key_str, data_key, regex.as_ref(), &mut values, &mut count)?;
            }
        }
    }

    values.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(values)
}

/// Decrypt all `ENC[...]` values in a YAML document.
///
/// Walks the YAML tree, decrypting encrypted values back to their
/// original types. Returns collected `(path, plaintext)` pairs for
/// MAC verification.
pub fn decrypt_yaml_values(root: &mut Value, data_key: &[u8; 32]) -> Result<Vec<(String, String)>> {
    let mut values = Vec::new();

    if let Value::Mapping(map) = root {
        let keys: Vec<Value> = map.keys().cloned().collect();
        for key in keys {
            let key_str = yaml_key_to_string(&key);
            if key_str == "sops" {
                continue;
            }
            if let Some(item) = map.get_mut(&key) {
                decrypt_value_recursive(item, &key_str, data_key, &mut values)?;
            }
        }
    }

    values.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(values)
}

/// Parse YAML, encrypt values, inject metadata, and serialize.
///
/// Returns `(encrypted_output, value_pairs_for_mac)`.
pub fn encrypt_document(
    contents: &str,
    data_key: &[u8; 32],
    encrypted_regex: Option<&str>,
    metadata: &SopsFileMetadata,
    input_path: &std::path::Path,
) -> Result<(String, Vec<(String, String)>)> {
    let mut root: Value = serde_yaml::from_str(contents).map_err(|e| SopsError::ParseFile {
        path: input_path.to_path_buf(),
        reason: e.to_string(),
    })?;

    let values = encrypt_yaml_values(&mut root, data_key, encrypted_regex)?;
    inject_metadata(&mut root, metadata)?;

    let output = serde_yaml::to_string(&root).map_err(|e| SopsError::Serialization {
        reason: format!("failed to serialize YAML: {e}"),
    })?;

    Ok((output, values))
}

/// Parse YAML, decrypt values, remove metadata, and serialize.
///
/// Returns `(decrypted_output, value_pairs_for_mac)`.
pub fn decrypt_document(
    contents: &str,
    data_key: &[u8; 32],
    input_path: &std::path::Path,
) -> Result<(String, Vec<(String, String)>)> {
    let mut root: Value = serde_yaml::from_str(contents).map_err(|e| SopsError::ParseFile {
        path: input_path.to_path_buf(),
        reason: e.to_string(),
    })?;

    let values = decrypt_yaml_values(&mut root, data_key)?;

    // Remove sops metadata from output
    if let Value::Mapping(map) = &mut root {
        map.remove(Value::String("sops".to_string()));
    }

    let output = serde_yaml::to_string(&root).map_err(|e| SopsError::Serialization {
        reason: format!("failed to serialize YAML: {e}"),
    })?;

    Ok((output, values))
}

/// Extract SOPS metadata from YAML contents.
pub fn extract_metadata_from_contents(
    contents: &str,
    input_path: &std::path::Path,
) -> Result<Option<SopsFileMetadata>> {
    let root: Value = serde_yaml::from_str(contents).map_err(|e| SopsError::ParseFile {
        path: input_path.to_path_buf(),
        reason: e.to_string(),
    })?;

    let sops_value = match root.get("sops") {
        Some(v) => v,
        None => return Ok(None),
    };

    let metadata: SopsFileMetadata =
        serde_yaml::from_value(sops_value.clone()).map_err(|e| SopsError::InvalidMetadata {
            reason: format!("invalid sops metadata in YAML: {e}"),
        })?;

    Ok(Some(metadata))
}

/// Inject SOPS metadata into a YAML document.
pub fn inject_metadata(root: &mut Value, metadata: &SopsFileMetadata) -> Result<()> {
    let sops_value = serde_yaml::to_value(metadata).map_err(|e| SopsError::Serialization {
        reason: format!("failed to serialize SOPS metadata to YAML: {e}"),
    })?;

    if let Value::Mapping(map) = root {
        map.insert(Value::String("sops".to_string()), sops_value);
    }
    Ok(())
}

/// Update metadata in an existing YAML document (for rotate/updatekeys).
pub fn update_metadata_in_document(
    contents: &str,
    metadata: &SopsFileMetadata,
    input_path: &std::path::Path,
) -> Result<String> {
    let mut root: Value = serde_yaml::from_str(contents).map_err(|e| SopsError::ParseFile {
        path: input_path.to_path_buf(),
        reason: e.to_string(),
    })?;

    inject_metadata(&mut root, metadata)?;

    serde_yaml::to_string(&root).map_err(|e| SopsError::Serialization {
        reason: format!("failed to serialize YAML: {e}"),
    })
}

/// Extract a single value from decrypted YAML by dotted path.
pub fn extract_value(yaml_str: &str, path: &str) -> Result<String> {
    let root: Value = serde_yaml::from_str(yaml_str).map_err(|e| SopsError::ParseFile {
        path: std::path::PathBuf::from("<decrypted>"),
        reason: e.to_string(),
    })?;

    let mut current = &root;
    for segment in path.split('.') {
        // Try as mapping key first, then as sequence index
        current = current
            .get(segment)
            .or_else(|| segment.parse::<usize>().ok().and_then(|i| current.get(i)))
            .ok_or_else(|| SopsError::ParseFile {
                path: std::path::PathBuf::from("<decrypted>"),
                reason: format!("key path '{path}' not found at segment '{segment}'"),
            })?;
    }

    match current {
        Value::String(s) => Ok(s.clone()),
        Value::Null => Ok("null".to_string()),
        other => serde_yaml::to_string(other)
            .map(|s| s.trim_end().to_string())
            .map_err(|e| SopsError::Serialization {
                reason: format!("failed to serialize extracted YAML value: {e}"),
            }),
    }
}

/// Convert a YAML key to a string for path construction.
fn yaml_key_to_string(key: &Value) -> String {
    match key {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => format!("{key:?}"),
    }
}

fn encrypt_value_recursive(
    value: &mut Value,
    path: &str,
    data_key: &[u8; 32],
    regex: Option<&regex::Regex>,
    values: &mut Vec<(String, String)>,
    count: &mut u32,
) -> Result<()> {
    validate_key_path(path)?;

    match value {
        Value::Mapping(map) => {
            let keys: Vec<Value> = map.keys().cloned().collect();
            for key in keys {
                let key_str = yaml_key_to_string(&key);
                let child_path = format!("{path}.{key_str}");
                if let Some(child) = map.get_mut(&key) {
                    encrypt_value_recursive(child, &child_path, data_key, regex, values, count)?;
                }
            }
        }
        Value::Sequence(seq) => {
            for (i, item) in seq.iter_mut().enumerate() {
                let child_path = format!("{path}[{i}]");
                encrypt_value_recursive(item, &child_path, data_key, regex, values, count)?;
            }
        }
        Value::String(s) => {
            let plaintext = s.clone();
            let should_encrypt = regex.as_ref().is_none_or(|re| re.is_match(path));

            values.push((path.to_string(), plaintext.clone()));
            check_value_count(count)?;

            if should_encrypt {
                let encrypted = encrypt_sops_value(&plaintext, data_key, "str")?;
                *value = Value::String(encrypted);
            }
        }
        Value::Number(n) => {
            let plaintext = n.to_string();
            let value_type = if n.is_f64() { "float" } else { "int" };
            let should_encrypt = regex.as_ref().is_none_or(|re| re.is_match(path));

            values.push((path.to_string(), plaintext.clone()));
            check_value_count(count)?;

            if should_encrypt {
                let encrypted = encrypt_sops_value(&plaintext, data_key, value_type)?;
                *value = Value::String(encrypted);
            }
        }
        Value::Bool(b) => {
            let plaintext = b.to_string();
            let should_encrypt = regex.as_ref().is_none_or(|re| re.is_match(path));

            values.push((path.to_string(), plaintext.clone()));
            check_value_count(count)?;

            if should_encrypt {
                let encrypted = encrypt_sops_value(&plaintext, data_key, "bool")?;
                *value = Value::String(encrypted);
            }
        }
        Value::Null => {
            values.push((path.to_string(), "null".to_string()));
        }
        Value::Tagged(tagged) => {
            // Handle tagged values by processing the inner value
            encrypt_value_recursive(&mut tagged.value, path, data_key, regex, values, count)?;
        }
    }
    Ok(())
}

fn decrypt_value_recursive(
    value: &mut Value,
    path: &str,
    data_key: &[u8; 32],
    values: &mut Vec<(String, String)>,
) -> Result<()> {
    match value {
        Value::Mapping(map) => {
            let keys: Vec<Value> = map.keys().cloned().collect();
            for key in keys {
                let key_str = yaml_key_to_string(&key);
                let child_path = format!("{path}.{key_str}");
                if let Some(child) = map.get_mut(&key) {
                    decrypt_value_recursive(child, &child_path, data_key, values)?;
                }
            }
        }
        Value::Sequence(seq) => {
            for (i, item) in seq.iter_mut().enumerate() {
                let child_path = format!("{path}[{i}]");
                decrypt_value_recursive(item, &child_path, data_key, values)?;
            }
        }
        Value::String(s) => {
            if is_sops_encrypted(s) {
                let (plaintext, value_type) = decrypt_sops_value_with_type(s, data_key)?;
                values.push((path.to_string(), plaintext.clone()));

                *value = restore_typed_yaml_value(&plaintext, &value_type);
            } else {
                values.push((path.to_string(), s.clone()));
            }
        }
        Value::Number(n) => {
            values.push((path.to_string(), n.to_string()));
        }
        Value::Bool(b) => {
            values.push((path.to_string(), b.to_string()));
        }
        Value::Null => {
            values.push((path.to_string(), "null".to_string()));
        }
        Value::Tagged(tagged) => {
            decrypt_value_recursive(&mut tagged.value, path, data_key, values)?;
        }
    }
    Ok(())
}

/// Restore a typed YAML value from a plaintext string and type tag.
fn restore_typed_yaml_value(plaintext: &str, value_type: &str) -> Value {
    match value_type {
        "int" => plaintext
            .parse::<i64>()
            .map(|n| Value::Number(serde_yaml::Number::from(n)))
            .unwrap_or_else(|_| Value::String(plaintext.to_string())),
        "float" => plaintext
            .parse::<f64>()
            .map(|f| Value::Number(serde_yaml::Number::from(f)))
            .unwrap_or_else(|_| Value::String(plaintext.to_string())),
        "bool" => plaintext.parse::<bool>().map(Value::Bool).unwrap_or_else(|_| Value::String(plaintext.to_string())),
        _ => Value::String(plaintext.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_yaml_document() {
        let key = [42u8; 32];
        let yaml_str = r#"
api_key: sk-test-123
port: 8080
debug: true
rate: 1.5
database:
  password: s3cret
"#;

        let mut root: Value = serde_yaml::from_str(yaml_str).unwrap();

        // Encrypt
        let values = encrypt_yaml_values(&mut root, &key, None).unwrap();
        assert_eq!(values.len(), 5);

        let encrypted_str = serde_yaml::to_string(&root).unwrap();
        assert!(encrypted_str.contains("ENC[AES256_GCM,"));
        assert!(encrypted_str.contains("api_key"));
        assert!(encrypted_str.contains("password"));

        // Decrypt
        let mut root2: Value = serde_yaml::from_str(&encrypted_str).unwrap();
        let dec_values = decrypt_yaml_values(&mut root2, &key).unwrap();

        assert_eq!(values, dec_values);
        assert_eq!(root2["api_key"], Value::String("sk-test-123".into()));
        assert_eq!(root2["database"]["password"], Value::String("s3cret".into()));
    }

    #[test]
    fn test_type_preservation() {
        let key = [42u8; 32];
        let yaml_str = r#"
name: test
count: 42
ratio: 3.14
enabled: true
"#;

        let mut root: Value = serde_yaml::from_str(yaml_str).unwrap();
        let _values = encrypt_yaml_values(&mut root, &key, None).unwrap();

        // All values should be encrypted strings now
        assert!(root["name"].is_string());
        assert!(root["count"].is_string());
        assert!(root["ratio"].is_string());
        assert!(root["enabled"].is_string());

        // Decrypt and verify types are restored
        let mut root2 = root;
        let _values = decrypt_yaml_values(&mut root2, &key).unwrap();

        assert_eq!(root2["name"].as_str(), Some("test"));
        assert_eq!(root2["count"].as_i64(), Some(42));
        assert!(root2["ratio"].as_f64().is_some());
        assert_eq!(root2["enabled"].as_bool(), Some(true));
    }

    #[test]
    fn test_encrypted_regex_filter() {
        let key = [42u8; 32];
        let yaml_str = r#"
api_key: secret
description: not secret
password: also secret
"#;

        let mut root: Value = serde_yaml::from_str(yaml_str).unwrap();
        let _values = encrypt_yaml_values(&mut root, &key, Some("key|password")).unwrap();

        assert!(root["api_key"].as_str().unwrap().starts_with("ENC["));
        assert!(root["password"].as_str().unwrap().starts_with("ENC["));
        assert_eq!(root["description"].as_str().unwrap(), "not secret");
    }

    #[test]
    fn test_nested_and_sequences() {
        let key = [42u8; 32];
        let yaml_str = r#"
servers:
  - host: db1.example.com
    port: 5432
  - host: db2.example.com
    port: 5433
config:
  nested:
    deep_secret: hidden
"#;

        let mut root: Value = serde_yaml::from_str(yaml_str).unwrap();
        let values = encrypt_yaml_values(&mut root, &key, None).unwrap();

        // 2 hosts + 2 ports + 1 deep_secret = 5
        assert_eq!(values.len(), 5);

        // Decrypt roundtrip
        let encrypted = serde_yaml::to_string(&root).unwrap();
        let mut root2: Value = serde_yaml::from_str(&encrypted).unwrap();
        let dec_values = decrypt_yaml_values(&mut root2, &key).unwrap();

        assert_eq!(values, dec_values);
        assert_eq!(root2["config"]["nested"]["deep_secret"], Value::String("hidden".into()));
    }

    #[test]
    fn test_null_values_preserved() {
        let key = [42u8; 32];
        let yaml_str = r#"
name: test
optional: null
"#;

        let mut root: Value = serde_yaml::from_str(yaml_str).unwrap();
        let values = encrypt_yaml_values(&mut root, &key, None).unwrap();

        assert_eq!(values.len(), 2);
        assert!(root["optional"].is_null());
    }

    #[test]
    fn test_metadata_injection_extraction() {
        let mut meta = SopsFileMetadata::new();
        meta.mac = "ENC[AES256_GCM,data:abc,iv:def,tag:ghi,type:str]".into();
        meta.lastmodified = "2026-03-04T10:00:00Z".into();

        let mut root: Value = serde_yaml::from_str("secret: value").unwrap();
        inject_metadata(&mut root, &meta).unwrap();

        let output = serde_yaml::to_string(&root).unwrap();
        assert!(output.contains("sops:"));

        let extracted = extract_metadata_from_contents(&output, std::path::Path::new("test.yaml")).unwrap().unwrap();
        assert_eq!(extracted.mac, meta.mac);
        assert_eq!(extracted.lastmodified, meta.lastmodified);
    }

    #[test]
    fn test_extract_value_dotted_path() {
        let yaml_str = "database:\n  host: localhost\n  port: 5432\n";

        assert_eq!(extract_value(yaml_str, "database.host").unwrap(), "localhost");
        assert_eq!(extract_value(yaml_str, "database.port").unwrap(), "5432");
    }

    #[test]
    fn test_encrypt_decrypt_document_roundtrip() {
        let key = [42u8; 32];
        let yaml_str = "api_key: sk-123\nport: 8080\n";
        let meta = SopsFileMetadata::new();

        let (encrypted, values) =
            encrypt_document(yaml_str, &key, None, &meta, std::path::Path::new("test.yaml")).unwrap();

        assert!(encrypted.contains("ENC["));
        assert!(encrypted.contains("sops:"));

        let (decrypted, dec_values) = decrypt_document(&encrypted, &key, std::path::Path::new("test.yaml")).unwrap();
        assert_eq!(values, dec_values);
        assert!(!decrypted.contains("sops:"));
        assert!(decrypted.contains("sk-123"));
    }
}
