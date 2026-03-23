//! Secret decryption and key extraction.
//!
//! Decrypts SOPS files using `aspen_secrets::sops::decrypt` and extracts
//! individual key paths from the decrypted output.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use tracing::debug;

use crate::manifest::FormatType;
use crate::manifest::Manifest;
use crate::manifest::SecretEntry;

/// Decrypted plaintext data from a SOPS file.
struct PlainData {
    /// Parsed key-value data (for yaml/json with key extraction).
    keys: Option<serde_json::Value>,
    /// Raw bytes (for binary format or whole-file extraction).
    raw: Vec<u8>,
}

/// Decrypt all secrets in the manifest.
///
/// Caches decrypted SOPS files so each file is decrypted only once,
/// even when multiple secrets reference the same file.
pub async fn decrypt_secrets(manifest: &Manifest, age_key_file: Option<&str>) -> Result<HashMap<String, Vec<u8>>> {
    let cluster_ticket = std::env::var("ASPEN_CLUSTER_TICKET").ok();
    let mut source_files: HashMap<String, PlainData> = HashMap::new();
    let mut results: HashMap<String, Vec<u8>> = HashMap::new();

    for secret in &manifest.secrets {
        if !secret.format.is_supported() {
            bail!("secret '{}': format '{}' is not supported; use yaml or json", secret.name, secret.format);
        }

        let plain = decrypt_sops_file(
            &secret.sops_file,
            secret.format,
            cluster_ticket.as_deref(),
            age_key_file,
            &mut source_files,
        )
        .await
        .with_context(|| format!("failed to decrypt '{}' for secret '{}'", secret.sops_file, secret.name))?;

        let value = extract_key(plain, secret)?;
        results.insert(secret.name.clone(), value);
    }

    Ok(results)
}

/// Decrypt a SOPS file, caching the result.
async fn decrypt_sops_file<'a>(
    sops_file: &str,
    format: FormatType,
    cluster_ticket: Option<&str>,
    age_key_file: Option<&str>,
    cache: &'a mut HashMap<String, PlainData>,
) -> Result<&'a PlainData> {
    if !cache.contains_key(sops_file) {
        let config = aspen_secrets::sops::DecryptConfig {
            input_path: PathBuf::from(sops_file),
            cluster_ticket: cluster_ticket.map(String::from),
            output_path: None,
            extract_path: None,
            age_identity: age_key_file.map(PathBuf::from),
        };

        let decrypted = aspen_secrets::sops::decrypt::decrypt_file(&config)
            .await
            .with_context(|| format!("SOPS decryption failed for '{sops_file}'"))?;

        let raw = decrypted.into_bytes();
        let keys = match format {
            FormatType::Binary => None,
            FormatType::Yaml => {
                let val: serde_json::Value =
                    serde_yaml::from_slice(&raw).with_context(|| format!("cannot parse YAML from '{sops_file}'"))?;
                Some(val)
            }
            FormatType::Json => {
                let val: serde_json::Value =
                    serde_json::from_slice(&raw).with_context(|| format!("cannot parse JSON from '{sops_file}'"))?;
                Some(val)
            }
            FormatType::Dotenv | FormatType::Ini => {
                bail!("format '{}' is not supported", format);
            }
        };

        cache.insert(sops_file.to_string(), PlainData { keys, raw });
    }

    cache
        .get(sops_file)
        .ok_or_else(|| anyhow::anyhow!("failed to retrieve decrypted data for '{}' from cache", sops_file))
}

/// Extract a key path from decrypted data.
fn extract_key(plain: &PlainData, secret: &SecretEntry) -> Result<Vec<u8>> {
    match secret.format {
        FormatType::Binary => {
            debug!(secret = %secret.name, "using raw binary data");
            Ok(plain.raw.clone())
        }
        FormatType::Yaml | FormatType::Json => {
            if secret.key.is_empty() {
                debug!(secret = %secret.name, "using whole file (empty key)");
                return Ok(plain.raw.clone());
            }

            let keys = plain.keys.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "secret '{}' in '{}': no parsed keys available for yaml/json format",
                    secret.name,
                    secret.sops_file
                )
            })?;

            let value = recurse_key(keys, &secret.key).with_context(|| {
                format!("secret '{}' in '{}': key '{}' not found", secret.name, secret.sops_file, secret.key)
            })?;

            Ok(value.into_bytes())
        }
        _ => bail!("unsupported format '{}'", secret.format),
    }
}

/// Traverse a JSON value by `/`-separated key path, matching Go's `recurseSecretKey`.
///
/// Returns the string value at the path, or an error if the path doesn't exist
/// or the final value is not a string.
fn recurse_key(value: &serde_json::Value, key_path: &str) -> Result<String> {
    let mut current = value;
    let mut path_so_far = String::new();

    for part in key_path.split('/') {
        if !path_so_far.is_empty() {
            path_so_far.push('/');
        }
        path_so_far.push_str(part);

        match current {
            serde_json::Value::Object(map) => {
                current = map.get(part).with_context(|| format!("the key '{path_so_far}' cannot be found"))?;
            }
            _ => bail!("key '{path_so_far}' does not refer to a dictionary"),
        }
    }

    match current {
        serde_json::Value::String(s) => Ok(s.clone()),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::Bool(b) => Ok(b.to_string()),
        serde_json::Value::Null => Ok("".to_string()),
        _ => bail!("the value of key '{key_path}' is not a scalar"),
    }
}

/// Validate that a SOPS file exists and contains expected keys.
/// Used by `-check-mode=sopsfile`.
pub fn validate_sops_file(secret: &SecretEntry) -> Result<()> {
    if !secret.format.is_supported() {
        bail!("secret '{}': format '{}' is not supported; use yaml or json", secret.name, secret.format);
    }

    let path = std::path::Path::new(&secret.sops_file);
    if !path.exists() {
        bail!("secret '{}': SOPS file '{}' does not exist", secret.name, secret.sops_file);
    }

    // For yaml/json with a key path, verify the key exists in the encrypted structure
    if !secret.key.is_empty() && matches!(secret.format, FormatType::Yaml | FormatType::Json) {
        let contents = std::fs::read_to_string(path).with_context(|| format!("cannot read '{}'", secret.sops_file))?;

        let parsed: serde_json::Value =
            match secret.format {
                FormatType::Yaml => serde_yaml::from_str(&contents)
                    .with_context(|| format!("cannot parse YAML '{}'", secret.sops_file))?,
                FormatType::Json => serde_json::from_str(&contents)
                    .with_context(|| format!("cannot parse JSON '{}'", secret.sops_file))?,
                _ => unreachable!(),
            };

        // Check the key path exists in the encrypted document structure
        // (values will be encrypted strings like "ENC[AES256_GCM,...]" but keys are plaintext)
        recurse_key(&parsed, &secret.key).with_context(|| {
            format!(
                "secret '{}' in '{}': key '{}' not found in encrypted document",
                secret.name, secret.sops_file, secret.key
            )
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recurse_key_simple() {
        let val: serde_json::Value = serde_json::json!({
            "password": "secret123"
        });
        assert_eq!(recurse_key(&val, "password").unwrap(), "secret123");
    }

    #[test]
    fn recurse_key_nested() {
        let val: serde_json::Value = serde_json::json!({
            "database": {
                "password": "secret123"
            }
        });
        assert_eq!(recurse_key(&val, "database/password").unwrap(), "secret123");
    }

    #[test]
    fn recurse_key_deeply_nested() {
        let val: serde_json::Value = serde_json::json!({
            "a": { "b": { "c": "deep" } }
        });
        assert_eq!(recurse_key(&val, "a/b/c").unwrap(), "deep");
    }

    #[test]
    fn recurse_key_number() {
        let val: serde_json::Value = serde_json::json!({
            "port": 5432
        });
        assert_eq!(recurse_key(&val, "port").unwrap(), "5432");
    }

    #[test]
    fn recurse_key_bool() {
        let val: serde_json::Value = serde_json::json!({
            "enabled": true
        });
        assert_eq!(recurse_key(&val, "enabled").unwrap(), "true");
    }

    #[test]
    fn recurse_key_missing() {
        let val: serde_json::Value = serde_json::json!({
            "database": { "host": "localhost" }
        });
        let err = recurse_key(&val, "database/password").unwrap_err();
        assert!(err.to_string().contains("cannot be found"));
    }

    #[test]
    fn recurse_key_not_dict() {
        let val: serde_json::Value = serde_json::json!({
            "name": "flat"
        });
        let err = recurse_key(&val, "name/nested").unwrap_err();
        assert!(err.to_string().contains("does not refer to a dictionary"));
    }

    #[test]
    fn recurse_key_not_scalar() {
        let val: serde_json::Value = serde_json::json!({
            "database": { "host": "localhost" }
        });
        let err = recurse_key(&val, "database").unwrap_err();
        assert!(err.to_string().contains("not a scalar"));
    }
}
