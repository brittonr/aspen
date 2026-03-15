//! SOPS file encryption using Aspen Transit.
//!
//! Encrypts a plaintext file (TOML, JSON, or YAML):
//! 1. Generate data key via Transit
//! 2. Encrypt all values with the data key (AES-256-GCM)
//! 3. Compute and encrypt MAC
//! 4. Write SOPS metadata with encrypted data key

use std::path::PathBuf;

use tracing::info;

use super::client::TransitClient;
use super::format;
use super::mac::encrypt_mac;
use super::metadata::AspenTransitRecipient;
use super::metadata::SopsFileMetadata;
use super::sops_constants::DEFAULT_TRANSIT_KEY;
use super::sops_constants::DEFAULT_TRANSIT_MOUNT;
use super::sops_constants::MAX_SOPS_FILE_SIZE;
use super::sops_error::Result;
use super::sops_error::SopsError;

/// Configuration for encrypting a file.
#[derive(Debug, Clone)]
pub struct EncryptConfig {
    /// Path to the plaintext input file.
    pub input_path: PathBuf,
    /// Aspen cluster ticket.
    pub cluster_ticket: String,
    /// Transit key name for data key encryption.
    pub transit_key: String,
    /// Transit mount point.
    pub transit_mount: String,
    /// Also encrypt for these age recipients (offline fallback).
    pub age_recipients: Vec<String>,
    /// Only encrypt values whose key path matches this regex.
    pub encrypted_regex: Option<String>,
    /// Write encrypted output back to the input file.
    pub in_place: bool,
}

impl Default for EncryptConfig {
    fn default() -> Self {
        Self {
            input_path: PathBuf::new(),
            cluster_ticket: String::new(),
            transit_key: DEFAULT_TRANSIT_KEY.into(),
            transit_mount: DEFAULT_TRANSIT_MOUNT.into(),
            age_recipients: Vec::new(),
            encrypted_regex: None,
            in_place: false,
        }
    }
}

/// Encrypt a file using Aspen Transit.
///
/// Returns the encrypted file contents as a string.
pub async fn encrypt_file(config: &EncryptConfig) -> Result<String> {
    // Detect format
    let fmt = format::detect_format(&config.input_path)?;

    // Read file
    let contents = tokio::fs::read_to_string(&config.input_path).await.map_err(|e| SopsError::FileRead {
        path: config.input_path.clone(),
        source: e,
    })?;

    if contents.len() > MAX_SOPS_FILE_SIZE {
        return Err(SopsError::FileTooLarge {
            path: config.input_path.clone(),
            size_bytes: contents.len() as u64,
            max_bytes: MAX_SOPS_FILE_SIZE as u64,
        });
    }

    // Connect to Transit
    let client = TransitClient::connect(&config.cluster_ticket, Some(&config.transit_mount)).await?;

    // Generate data key
    let (data_key, encrypted_data_key, key_version) = client.generate_data_key(&config.transit_key).await?;

    // Ensure data key is 32 bytes
    let data_key_array = to_key_array(&data_key)?;

    // Build metadata
    let mut metadata = SopsFileMetadata::new();
    metadata.encrypted_regex = config.encrypted_regex.clone();

    let aspen_recipient = AspenTransitRecipient {
        cluster_ticket: config.cluster_ticket.clone(),
        mount: config.transit_mount.clone(),
        name: config.transit_key.clone(),
        enc: encrypted_data_key,
        key_version,
    };
    // Also add hc_vault_transit entry for Go SOPS keyservice interop
    metadata.add_hc_vault_mirror(&aspen_recipient);
    metadata.add_aspen_recipient(aspen_recipient);

    // Encrypt data key for age recipients if configured
    for age_recipient in &config.age_recipients {
        let age_enc = encrypt_data_key_for_age(age_recipient, &data_key)?;
        metadata.add_age_recipient(super::metadata::AgeRecipient {
            recipient: age_recipient.clone(),
            enc: Some(age_enc),
        });
    }

    // Encrypt values and inject metadata (format-agnostic dispatch)
    let (_output_before_mac, values) = format::encrypt_document(
        fmt,
        &contents,
        &data_key_array,
        config.encrypted_regex.as_deref(),
        &metadata,
        &config.input_path,
    )?;

    // Compute and encrypt MAC
    let encrypted_mac = encrypt_mac(&data_key_array, &values)?;
    metadata.mac = encrypted_mac;

    // Sync key_groups for Go SOPS 3.7+ interop
    metadata.sync_key_groups();

    // Re-render with final metadata (includes MAC)
    let (output, _) = format::encrypt_document(
        fmt,
        &contents,
        &data_key_array,
        config.encrypted_regex.as_deref(),
        &metadata,
        &config.input_path,
    )?;

    // Data key is automatically zeroized here (Zeroizing<Vec<u8>> dropped)
    drop(data_key);

    // Write in place if requested
    if config.in_place {
        tokio::fs::write(&config.input_path, output.as_bytes()).await.map_err(|e| SopsError::FileWrite {
            path: config.input_path.clone(),
            source: e,
        })?;
        info!(path = %config.input_path.display(), format = ?fmt, "Encrypted file in place");
    }

    Ok(output)
}

/// Convert a data key Vec to a fixed-size array.
fn to_key_array(key: &[u8]) -> Result<[u8; 32]> {
    if key.len() != 32 {
        return Err(SopsError::TransitEncrypt {
            key_name: String::new(),
            reason: format!("data key has wrong length: {} (expected 32)", key.len()),
        });
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(key);
    Ok(arr)
}

/// Encrypt a data key for an age recipient (public for updatekeys).
pub fn encrypt_data_key_for_age(recipient_str: &str, data_key: &[u8]) -> Result<String> {
    use std::io::Write;

    let recipient: age::x25519::Recipient = recipient_str.parse().map_err(|e| SopsError::AgeError {
        reason: format!("invalid age recipient '{recipient_str}': {e}"),
    })?;

    let recipients: Vec<Box<dyn age::Recipient>> = vec![Box::new(recipient)];
    let encryptor =
        age::Encryptor::with_recipients(recipients.iter().map(|r| r.as_ref())).expect("at least one recipient");

    let mut encrypted = Vec::new();
    let mut writer =
        age::armor::ArmoredWriter::wrap_output(&mut encrypted, age::armor::Format::AsciiArmor).map_err(|e| {
            SopsError::AgeError {
                reason: format!("failed to create armored writer: {e}"),
            }
        })?;

    let mut inner = encryptor.wrap_output(&mut writer).map_err(|e| SopsError::AgeError {
        reason: format!("failed to encrypt for age: {e}"),
    })?;

    inner.write_all(data_key).map_err(|e| SopsError::AgeError {
        reason: format!("failed to write data key: {e}"),
    })?;
    inner.finish().map_err(|e| SopsError::AgeError {
        reason: format!("failed to finish age encryption: {e}"),
    })?;
    writer.finish().map_err(|e| SopsError::AgeError {
        reason: format!("failed to finish armored writer: {e}"),
    })?;

    String::from_utf8(encrypted).map_err(|e| SopsError::AgeError {
        reason: format!("armored output is not UTF-8: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::super::sops_constants::DEFAULT_TRANSIT_KEY;
    use super::super::sops_constants::DEFAULT_TRANSIT_MOUNT;
    use super::*;

    #[test]
    fn to_key_array_accepts_32_bytes() {
        let key = [0xCDu8; 32];
        let arr = to_key_array(&key).unwrap();
        assert_eq!(arr, key);
    }

    #[test]
    fn to_key_array_rejects_wrong_lengths() {
        assert!(to_key_array(&[]).is_err());
        assert!(to_key_array(&[0u8; 31]).is_err());
        assert!(to_key_array(&[0u8; 33]).is_err());
        assert!(to_key_array(&[0u8; 64]).is_err());
    }

    #[test]
    fn encrypt_data_key_for_age_round_trip() {
        let identity = age::x25519::Identity::generate();
        let data_key = [0x55u8; 32];
        let encrypted = encrypt_data_key_for_age(&identity.to_public().to_string(), &data_key).unwrap();

        // Should be ASCII-armored
        assert!(encrypted.contains("-----BEGIN AGE ENCRYPTED FILE-----"));
        assert!(encrypted.contains("-----END AGE ENCRYPTED FILE-----"));
    }

    #[test]
    fn encrypt_data_key_for_age_invalid_recipient() {
        let result = encrypt_data_key_for_age("not-a-valid-age-recipient", &[0u8; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn encrypt_config_defaults() {
        let cfg = EncryptConfig::default();
        assert_eq!(cfg.input_path, PathBuf::new());
        assert!(cfg.cluster_ticket.is_empty());
        assert_eq!(cfg.transit_key, DEFAULT_TRANSIT_KEY);
        assert_eq!(cfg.transit_mount, DEFAULT_TRANSIT_MOUNT);
        assert!(cfg.age_recipients.is_empty());
        assert!(cfg.encrypted_regex.is_none());
        assert!(!cfg.in_place);
    }
}
