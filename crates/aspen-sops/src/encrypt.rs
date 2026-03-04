//! SOPS file encryption using Aspen Transit.
//!
//! Encrypts a plaintext file (TOML, JSON, or YAML):
//! 1. Generate data key via Transit
//! 2. Encrypt all values with the data key (AES-256-GCM)
//! 3. Compute and encrypt MAC
//! 4. Write SOPS metadata with encrypted data key

use std::path::PathBuf;

use tracing::info;

use crate::client::TransitClient;
use crate::constants::DEFAULT_TRANSIT_KEY;
use crate::constants::DEFAULT_TRANSIT_MOUNT;
use crate::constants::MAX_SOPS_FILE_SIZE;
use crate::error::Result;
use crate::error::SopsError;
use crate::format;
use crate::mac::encrypt_mac;
use crate::metadata::AspenTransitRecipient;
use crate::metadata::SopsFileMetadata;

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

    metadata.add_aspen_recipient(AspenTransitRecipient {
        cluster_ticket: config.cluster_ticket.clone(),
        mount: config.transit_mount.clone(),
        name: config.transit_key.clone(),
        enc: encrypted_data_key,
        key_version,
    });

    // Encrypt data key for age recipients if configured
    #[cfg(feature = "age-fallback")]
    for age_recipient in &config.age_recipients {
        let age_enc = encrypt_data_key_for_age(age_recipient, &data_key)?;
        metadata.add_age_recipient(crate::metadata::AgeRecipient {
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
#[cfg(feature = "age-fallback")]
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
