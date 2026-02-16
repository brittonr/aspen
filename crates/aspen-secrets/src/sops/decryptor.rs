//! SOPS file decryption using age.
//!
//! SOPS encrypts individual values while keeping keys in plaintext.
//! The encrypted values have the format:
//! `ENC[AES256_GCM,data:base64...,iv:base64...,tag:base64...,type:str]`
//!
//! SOPS uses a data key (encrypted with age) to encrypt values with AES-256-GCM.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use tracing::debug;
use tracing::trace;

use crate::constants::MAX_DECRYPTED_AGE_SIZE;
use crate::constants::MAX_SECRETS_FILE_SIZE;
use crate::error::Result;
use crate::error::SecretsError;
use crate::sops::config::SecretsFile;
use crate::sops::config::SopsMetadata;

/// Load and decrypt a SOPS-encrypted secrets file.
///
/// # Arguments
///
/// * `path` - Path to the SOPS-encrypted file
/// * `identity` - Age identity for decryption
///
/// # Returns
///
/// Decrypted `SecretsFile` with all values in plaintext.
pub async fn decrypt_secrets_file(path: &Path, identity: &age::x25519::Identity) -> Result<SecretsFile> {
    // Read file contents
    let contents = tokio::fs::read_to_string(path).await.map_err(|e| SecretsError::ReadFile {
        path: path.to_path_buf(),
        source: e,
    })?;

    if contents.len() > MAX_SECRETS_FILE_SIZE {
        return Err(SecretsError::FileTooLarge {
            path: path.to_path_buf(),
            size: contents.len(),
            max: MAX_SECRETS_FILE_SIZE,
        });
    }

    decrypt_secrets_string(&contents, path, identity)
}

/// Decrypt a SOPS-encrypted TOML string.
pub fn decrypt_secrets_string(contents: &str, path: &Path, identity: &age::x25519::Identity) -> Result<SecretsFile> {
    // Parse the TOML to extract SOPS metadata
    let raw_value: toml::Value = toml::from_str(contents).map_err(|e| SecretsError::ParseFile {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })?;

    // Check if this is a SOPS-encrypted file
    let sops_metadata = extract_sops_metadata(&raw_value)?;

    // If no SOPS metadata, treat as plaintext
    let Some(sops) = sops_metadata else {
        debug!("No SOPS metadata found, treating as plaintext");
        let secrets: SecretsFile = toml::from_str(contents).map_err(|e| SecretsError::ParseFile {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;
        return Ok(secrets);
    };
    debug!(
        version = ?sops.version,
        recipients = sops.age.len(),
        "Decrypting SOPS file"
    );

    // Find our encrypted data key
    let data_key = decrypt_data_key(&sops, identity)?;

    // Decrypt all encrypted values in the TOML
    let decrypted_value = decrypt_toml_value(&raw_value, &data_key)?;

    // Parse the decrypted TOML into SecretsFile
    let secrets: SecretsFile = decrypted_value.try_into().map_err(|e: toml::de::Error| SecretsError::ParseFile {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })?;

    Ok(secrets)
}

/// Extract SOPS metadata from a TOML value.
fn extract_sops_metadata(value: &toml::Value) -> Result<Option<SopsMetadata>> {
    let table = match value.as_table() {
        Some(t) => t,
        None => return Ok(None),
    };

    let sops_value = match table.get("sops") {
        Some(v) => v,
        None => return Ok(None),
    };

    let metadata: SopsMetadata = sops_value
        .clone()
        .try_into()
        .map_err(|e: toml::de::Error| SecretsError::SopsMetadata { reason: e.to_string() })?;

    Ok(Some(metadata))
}

/// Decrypt the SOPS data key using our age identity.
fn decrypt_data_key(sops: &SopsMetadata, identity: &age::x25519::Identity) -> Result<[u8; 32]> {
    // Find an age recipient we can decrypt
    for recipient in &sops.age {
        if let Some(enc) = &recipient.enc {
            trace!(recipient = %recipient.recipient, "Trying age recipient");

            // The encrypted data key is armored age ciphertext
            match decrypt_age_ciphertext(enc, identity) {
                Ok(data_key) => {
                    if data_key.len() != 32 {
                        return Err(SecretsError::Decryption {
                            reason: format!("data key has wrong length: {} (expected 32)", data_key.len()),
                        });
                    }
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&data_key);
                    return Ok(key);
                }
                Err(e) => {
                    trace!(error = %e, "Failed to decrypt with this recipient");
                    continue;
                }
            }
        }
    }

    Err(SecretsError::Decryption {
        reason: "no matching age recipient found".into(),
    })
}

/// Decrypt age ciphertext (armored format).
///
/// Uses bounded reads to prevent encryption bomb attacks where decrypted
/// data expands beyond expected limits. The SOPS data key is exactly 32 bytes,
/// so we enforce a generous limit of 1 KB.
fn decrypt_age_ciphertext(ciphertext: &str, identity: &age::x25519::Identity) -> Result<Vec<u8>> {
    use std::iter;

    let decryptor =
        age::Decryptor::new_buffered(age::armor::ArmoredReader::new(ciphertext.as_bytes())).map_err(|e| {
            SecretsError::Decryption {
                reason: format!("failed to parse age ciphertext: {e}"),
            }
        })?;

    // Check if this is a passphrase-encrypted file
    if decryptor.is_scrypt() {
        return Err(SecretsError::Decryption {
            reason: "passphrase-encrypted files not supported".into(),
        });
    }

    let mut reader =
        decryptor
            .decrypt(iter::once(identity as &dyn age::Identity))
            .map_err(|e| SecretsError::Decryption {
                reason: format!("failed to decrypt: {e}"),
            })?;

    // Use bounded reads to prevent encryption bombs.
    // The SOPS data key is 32 bytes, but we allow up to MAX_DECRYPTED_AGE_SIZE
    // to provide generous headroom while still protecting against attacks.
    let mut plaintext = Vec::new();
    let mut buffer = [0u8; 256];
    let mut total_read = 0usize;

    loop {
        let bytes_read = reader.read(&mut buffer).map_err(|e| SecretsError::Decryption {
            reason: format!("failed to read decrypted data: {e}"),
        })?;

        if bytes_read == 0 {
            break;
        }

        total_read = total_read.saturating_add(bytes_read);
        if total_read > MAX_DECRYPTED_AGE_SIZE {
            return Err(SecretsError::Decryption {
                reason: format!(
                    "decrypted data exceeds maximum size ({} bytes), possible encryption bomb",
                    MAX_DECRYPTED_AGE_SIZE
                ),
            });
        }

        plaintext.extend_from_slice(&buffer[..bytes_read]);
    }

    Ok(plaintext)
}

/// Recursively decrypt all SOPS-encrypted values in a TOML tree.
fn decrypt_toml_value(value: &toml::Value, data_key: &[u8; 32]) -> Result<toml::Value> {
    match value {
        toml::Value::String(s) => {
            if is_sops_encrypted(s) {
                let decrypted = decrypt_sops_value(s, data_key)?;
                Ok(toml::Value::String(decrypted))
            } else {
                Ok(value.clone())
            }
        }
        toml::Value::Array(arr) => {
            let decrypted: Result<Vec<toml::Value>> = arr.iter().map(|v| decrypt_toml_value(v, data_key)).collect();
            Ok(toml::Value::Array(decrypted?))
        }
        toml::Value::Table(table) => {
            // Skip the 'sops' metadata table
            let decrypted: Result<toml::map::Map<String, toml::Value>> = table
                .iter()
                .filter(|(k, _)| *k != "sops")
                .map(|(k, v)| Ok((k.clone(), decrypt_toml_value(v, data_key)?)))
                .collect();
            Ok(toml::Value::Table(decrypted?))
        }
        // Non-string primitives pass through unchanged
        _ => Ok(value.clone()),
    }
}

/// Check if a string is SOPS-encrypted.
///
/// SOPS encrypted values have the format:
/// `ENC[AES256_GCM,data:base64...,iv:base64...,tag:base64...,type:str]`
fn is_sops_encrypted(s: &str) -> bool {
    s.starts_with("ENC[") && s.ends_with(']')
}

/// Decrypt a single SOPS-encrypted value.
///
/// Format: `ENC[AES256_GCM,data:base64,iv:base64,tag:base64,type:str]`
fn decrypt_sops_value(encrypted: &str, data_key: &[u8; 32]) -> Result<String> {
    use aes_gcm::Aes256Gcm;
    use aes_gcm::aead::Aead;
    use aes_gcm::aead::KeyInit;
    use aes_gcm::aead::generic_array::GenericArray;
    use base64::Engine;

    // Parse the SOPS format
    let inner =
        encrypted
            .strip_prefix("ENC[")
            .and_then(|s| s.strip_suffix(']'))
            .ok_or_else(|| SecretsError::Decryption {
                reason: "invalid SOPS encrypted value format".into(),
            })?;

    let parts: HashMap<&str, &str> = inner
        .split(',')
        .filter_map(|part| {
            let mut kv = part.splitn(2, ':');
            Some((kv.next()?, kv.next()?))
        })
        .collect();

    // Verify this is AES256_GCM (SOPS standard)
    // The cipher type is the first part before any colon
    let first_part = inner.split(',').next().unwrap_or("");
    if !first_part.starts_with("AES256_GCM") {
        return Err(SecretsError::Decryption {
            reason: format!("unsupported cipher type: {first_part}"),
        });
    }

    let data_b64 = parts.get("data").ok_or_else(|| SecretsError::Decryption {
        reason: "missing 'data' in SOPS value".into(),
    })?;

    let iv_b64 = parts.get("iv").ok_or_else(|| SecretsError::Decryption {
        reason: "missing 'iv' in SOPS value".into(),
    })?;

    let tag_b64 = parts.get("tag").ok_or_else(|| SecretsError::Decryption {
        reason: "missing 'tag' in SOPS value".into(),
    })?;

    let b64 = base64::engine::general_purpose::STANDARD;

    let ciphertext = b64.decode(data_b64).map_err(|e| SecretsError::Decryption {
        reason: format!("invalid base64 in data: {e}"),
    })?;

    let iv = b64.decode(iv_b64).map_err(|e| SecretsError::Decryption {
        reason: format!("invalid base64 in iv: {e}"),
    })?;

    let tag = b64.decode(tag_b64).map_err(|e| SecretsError::Decryption {
        reason: format!("invalid base64 in tag: {e}"),
    })?;

    // For AES-256-GCM, the IV is 12 bytes and tag is 16 bytes
    if iv.len() != 12 {
        return Err(SecretsError::Decryption {
            reason: format!("invalid IV length: {} (expected 12)", iv.len()),
        });
    }

    if tag.len() != 16 {
        return Err(SecretsError::Decryption {
            reason: format!("invalid tag length: {} (expected 16)", tag.len()),
        });
    }

    // Use AES-256-GCM for SOPS compatibility
    let cipher = Aes256Gcm::new(GenericArray::from_slice(data_key));
    let nonce = GenericArray::from_slice(&iv);

    // Combine ciphertext and tag (AEAD format expects tag appended)
    let mut combined = ciphertext;
    combined.extend_from_slice(&tag);

    let plaintext = cipher.decrypt(nonce, combined.as_ref()).map_err(|e| SecretsError::Decryption {
        reason: format!("AES-GCM decryption failed: {e}"),
    })?;

    String::from_utf8(plaintext).map_err(|e| SecretsError::Decryption {
        reason: format!("decrypted value is not valid UTF-8: {e}"),
    })
}

/// Load age identity from various sources.
///
/// Checks in order:
/// 1. Environment variable (SOPS_AGE_KEY)
/// 2. Specified identity file
/// 3. Default locations
pub async fn load_age_identity(identity_file: Option<&Path>, identity_env: &str) -> Result<age::x25519::Identity> {
    // Try environment variable first
    if let Ok(key) = std::env::var(identity_env) {
        return parse_age_identity(&key);
    }

    // Try the key file environment variable
    if let Ok(path) = std::env::var("SOPS_AGE_KEY_FILE") {
        return load_identity_from_file(Path::new(&path)).await;
    }

    // Try specified identity file
    if let Some(path) = identity_file {
        return load_identity_from_file(path).await;
    }

    // Try default locations
    let default_paths = get_default_identity_paths();
    for path in default_paths {
        if path.exists() {
            return load_identity_from_file(&path).await;
        }
    }

    Err(SecretsError::IdentityNotFound {
        path: identity_file.map(|p| p.to_path_buf()).unwrap_or_else(|| "~/.config/sops/age/keys.txt".into()),
    })
}

/// Get default age identity file paths.
fn get_default_identity_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    // XDG config home
    if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        paths.push(Path::new(&xdg_config).join("sops/age/keys.txt"));
    }

    // Home directory
    if let Ok(home) = std::env::var("HOME") {
        paths.push(Path::new(&home).join(".config/sops/age/keys.txt"));
    }

    paths
}

/// Load identity from a file.
async fn load_identity_from_file(path: &Path) -> Result<age::x25519::Identity> {
    let contents = tokio::fs::read_to_string(path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            SecretsError::IdentityNotFound {
                path: path.to_path_buf(),
            }
        } else {
            SecretsError::LoadIdentity {
                path: path.to_path_buf(),
                reason: e.to_string(),
            }
        }
    })?;

    parse_age_identity(&contents)
}

/// Parse an age identity from a string.
///
/// The string can be:
/// - A single age secret key (AGE-SECRET-KEY-1...)
/// - A file with multiple keys (one per line, # comments allowed)
fn parse_age_identity(contents: &str) -> Result<age::x25519::Identity> {
    for line in contents.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Try to parse as an age identity
        if line.starts_with("AGE-SECRET-KEY-1") {
            let identity = line
                .parse::<age::x25519::Identity>()
                .map_err(|e| SecretsError::InvalidIdentity { reason: e.to_string() })?;
            return Ok(identity);
        }
    }

    Err(SecretsError::InvalidIdentity {
        reason: "no AGE-SECRET-KEY-1 found in identity".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_sops_encrypted() {
        assert!(is_sops_encrypted("ENC[AES256_GCM,data:abc,iv:def,tag:ghi,type:str]"));
        assert!(!is_sops_encrypted("plaintext"));
        assert!(!is_sops_encrypted("ENC[incomplete"));
    }

    #[test]
    fn test_parse_plaintext_secrets_file() {
        let toml_str = r#"
            trusted_roots = ["a1b2c3d4"]
            signing_key = "0123456789abcdef"
        "#;

        let identity = age::x25519::Identity::generate();
        let result = decrypt_secrets_string(toml_str, Path::new("test.toml"), &identity);

        assert!(result.is_ok());
        let secrets = result.unwrap();
        assert_eq!(secrets.trusted_roots.len(), 1);
    }

    #[test]
    fn test_parse_age_identity() {
        use age::secrecy::ExposeSecret;
        // Generate a test identity
        let identity = age::x25519::Identity::generate();
        let key_str = identity.to_string().expose_secret().to_string();

        let parsed = parse_age_identity(&key_str);
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_parse_age_identity_with_comments() {
        use age::secrecy::ExposeSecret;
        let identity = age::x25519::Identity::generate();
        let key_str = identity.to_string().expose_secret().to_string();
        let contents = format!(
            "# This is a comment\n\
             # Created: 2024-01-01\n\
             {}\n",
            key_str
        );

        let parsed = parse_age_identity(&contents);
        assert!(parsed.is_ok());
    }
}
