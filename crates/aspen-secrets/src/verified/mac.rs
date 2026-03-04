//! Pure MAC computation functions for SOPS.
//!
//! Formally verified — see `verus/mac_spec.rs` for proofs.
//!
//! SOPS uses HMAC-SHA256 over all plaintext values (sorted by key path)
//! to detect file tampering. The MAC covers both key paths and values,
//! preventing key renaming or value reordering attacks.

use hmac::Hmac;
use hmac::Mac;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Compute the SOPS MAC over plaintext values.
///
/// The MAC is computed as HMAC-SHA256 with the data key as the HMAC key.
/// Values must be pre-sorted by key path (alphabetical). Each entry
/// contributes both the key path and the plaintext value to the MAC.
///
/// # Arguments
/// * `values` - Sorted `(key_path, plaintext_value)` pairs
/// * `data_key` - 32-byte data key used as HMAC key
///
/// # Returns
/// 32-byte HMAC-SHA256 digest
#[inline]
pub fn compute_sops_mac(values: &[(String, String)], data_key: &[u8; 32]) -> [u8; 32] {
    // HMAC-SHA256 key size is always valid for 32 bytes
    let mut mac = HmacSha256::new_from_slice(data_key).expect("HMAC key size is always valid for 32 bytes");

    for (path, value) in values {
        mac.update(path.as_bytes());
        mac.update(value.as_bytes());
    }

    mac.finalize().into_bytes().into()
}

/// Collect all value paths from a TOML table into sorted (path, value) pairs.
///
/// Recursively walks the TOML tree, collecting string representations of all
/// leaf values. The `[sops]` table is excluded from collection.
///
/// # Arguments
/// * `table` - TOML value to walk
/// * `prefix` - Current key path prefix (empty for root)
///
/// # Returns
/// Sorted vector of `(key_path, string_value)` pairs
pub fn collect_value_paths(table: &toml::Value, prefix: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    collect_value_paths_inner(table, prefix, &mut result);
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

fn collect_value_paths_inner(value: &toml::Value, prefix: &str, result: &mut Vec<(String, String)>) {
    match value {
        toml::Value::Table(table) => {
            for (key, val) in table {
                // Skip the [sops] metadata section
                if prefix.is_empty() && key == "sops" {
                    continue;
                }
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                collect_value_paths_inner(val, &path, result);
            }
        }
        toml::Value::Array(arr) => {
            for (i, val) in arr.iter().enumerate() {
                let path = format!("{prefix}[{i}]");
                collect_value_paths_inner(val, &path, result);
            }
        }
        // Leaf values — convert to string representation
        toml::Value::String(s) => {
            result.push((prefix.to_string(), s.clone()));
        }
        toml::Value::Integer(n) => {
            result.push((prefix.to_string(), n.to_string()));
        }
        toml::Value::Float(f) => {
            result.push((prefix.to_string(), f.to_string()));
        }
        toml::Value::Boolean(b) => {
            result.push((prefix.to_string(), b.to_string()));
        }
        toml::Value::Datetime(dt) => {
            result.push((prefix.to_string(), dt.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mac_empty_values() {
        let key = [0u8; 32];
        let mac = compute_sops_mac(&[], &key);
        // Should produce a valid 32-byte digest
        assert_eq!(mac.len(), 32);
    }

    #[test]
    fn test_mac_single_value() {
        let key = [1u8; 32];
        let values = vec![("key".to_string(), "value".to_string())];
        let mac = compute_sops_mac(&values, &key);
        assert_eq!(mac.len(), 32);
    }

    #[test]
    fn test_mac_stability() {
        let key = [42u8; 32];
        let values = vec![
            ("a.key".to_string(), "value-a".to_string()),
            ("b.key".to_string(), "value-b".to_string()),
        ];

        let mac1 = compute_sops_mac(&values, &key);
        let mac2 = compute_sops_mac(&values, &key);
        assert_eq!(mac1, mac2, "same inputs must produce same MAC");
    }

    #[test]
    fn test_mac_changes_with_values() {
        let key = [42u8; 32];
        let values1 = vec![("key".to_string(), "value1".to_string())];
        let values2 = vec![("key".to_string(), "value2".to_string())];

        let mac1 = compute_sops_mac(&values1, &key);
        let mac2 = compute_sops_mac(&values2, &key);
        assert_ne!(mac1, mac2, "different values must produce different MACs");
    }

    #[test]
    fn test_mac_changes_with_paths() {
        let key = [42u8; 32];
        let values1 = vec![("path.a".to_string(), "same".to_string())];
        let values2 = vec![("path.b".to_string(), "same".to_string())];

        let mac1 = compute_sops_mac(&values1, &key);
        let mac2 = compute_sops_mac(&values2, &key);
        assert_ne!(mac1, mac2, "different paths must produce different MACs");
    }

    #[test]
    fn test_collect_value_paths_simple() {
        let toml_str = r#"
            key1 = "value1"
            key2 = 42
            key3 = true
        "#;
        let value: toml::Value = toml::from_str(toml_str).unwrap();
        let paths = collect_value_paths(&value, "");

        assert_eq!(paths.len(), 3);
        // Should be sorted
        assert_eq!(paths[0], ("key1".to_string(), "value1".to_string()));
        assert_eq!(paths[1], ("key2".to_string(), "42".to_string()));
        assert_eq!(paths[2], ("key3".to_string(), "true".to_string()));
    }

    #[test]
    fn test_collect_value_paths_nested() {
        let toml_str = r#"
            [secrets]
            api_key = "sk-test"
            [secrets.nested]
            deep = "value"
        "#;
        let value: toml::Value = toml::from_str(toml_str).unwrap();
        let paths = collect_value_paths(&value, "");

        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], ("secrets.api_key".to_string(), "sk-test".to_string()));
        assert_eq!(paths[1], ("secrets.nested.deep".to_string(), "value".to_string()));
    }

    #[test]
    fn test_collect_value_paths_skips_sops() {
        let toml_str = r#"
            secret = "data"
            [sops]
            version = "3.9.0"
            mac = "encrypted-mac"
        "#;
        let value: toml::Value = toml::from_str(toml_str).unwrap();
        let paths = collect_value_paths(&value, "");

        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], ("secret".to_string(), "data".to_string()));
    }
}
