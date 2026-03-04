//! Update key groups in a SOPS file — add/remove recipients.
//!
//! Decrypts the data key using any available key group, then
//! re-encrypts it for the new set of recipients.

use std::path::PathBuf;

use toml_edit::DocumentMut;
use tracing::info;
use zeroize::Zeroizing;

use crate::client::TransitClient;
use crate::constants::MAX_SOPS_FILE_SIZE;
use crate::error::Result;
use crate::error::SopsError;
use crate::metadata::AspenTransitRecipient;
use crate::metadata::SopsFileMetadata;
use crate::metadata::extract_metadata;

/// Configuration for updating key groups.
#[derive(Debug, Clone)]
pub struct UpdateKeysConfig {
    /// Path to the SOPS-encrypted file.
    pub input_path: PathBuf,
    /// Aspen cluster ticket for adding a Transit recipient.
    pub cluster_ticket: Option<String>,
    /// Transit key name (when adding a Transit recipient).
    pub transit_key: Option<String>,
    /// Transit mount point.
    pub transit_mount: String,
    /// Age recipients to add.
    pub add_age: Vec<String>,
    /// Age recipients to remove (by public key).
    pub remove_age: Vec<String>,
    /// Update in place.
    pub in_place: bool,
}

/// Update key groups in a SOPS-encrypted file.
///
/// 1. Decrypt the data key using any available key group
/// 2. Add/remove recipients as requested
/// 3. Re-encrypt the data key for all recipients
///
/// Returns the updated file contents.
pub async fn update_keys(config: &UpdateKeysConfig) -> Result<String> {
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

    let parsed: toml::Value = toml::from_str(&contents).map_err(|e| SopsError::ParseFile {
        path: config.input_path.clone(),
        reason: e.to_string(),
    })?;

    let mut metadata = extract_metadata(&parsed)?.ok_or(SopsError::InvalidMetadata {
        reason: "no [sops] section found".into(),
    })?;

    // Decrypt the data key using any available key group
    let data_key = decrypt_data_key_from_any(&metadata, config).await?;

    // Remove age recipients
    for age_key in &config.remove_age {
        metadata.remove_age_recipient(age_key);
        info!(recipient = age_key, "Removed age recipient");
    }

    // Add new Transit recipient
    if let Some(ref ticket) = config.cluster_ticket {
        let key_name = config.transit_key.as_deref().unwrap_or(crate::constants::DEFAULT_TRANSIT_KEY);

        let client = TransitClient::connect(ticket, Some(&config.transit_mount)).await?;
        let (enc, key_version) = client.encrypt_data(key_name, &data_key).await?;

        metadata.add_aspen_recipient(AspenTransitRecipient {
            cluster_ticket: ticket.clone(),
            mount: config.transit_mount.clone(),
            name: key_name.to_string(),
            enc,
            key_version,
        });
        info!(key = key_name, "Added Aspen Transit recipient");
    }

    // Add age recipients
    #[cfg(feature = "age-fallback")]
    for age_recipient in &config.add_age {
        let enc = crate::encrypt::encrypt_data_key_for_age(age_recipient, &data_key)?;
        metadata.add_age_recipient(crate::metadata::AgeRecipient {
            recipient: age_recipient.clone(),
            enc: Some(enc),
        });
        info!(recipient = age_recipient, "Added age recipient");
    }

    metadata.touch();

    // Rebuild document with updated metadata
    let mut doc: DocumentMut = contents.parse().map_err(|e: toml_edit::TomlError| SopsError::ParseFile {
        path: config.input_path.clone(),
        reason: e.to_string(),
    })?;

    let sops_value = toml::Value::try_from(&metadata)
        .map_err(|e: toml::ser::Error| SopsError::Serialization { reason: e.to_string() })?;
    let sops_str = toml::to_string_pretty(&sops_value)
        .map_err(|e: toml::ser::Error| SopsError::Serialization { reason: e.to_string() })?;
    let sops_doc: DocumentMut = format!("[sops]\n{sops_str}")
        .parse()
        .map_err(|e: toml_edit::TomlError| SopsError::Serialization { reason: e.to_string() })?;

    if let Some(sops_item) = sops_doc.get("sops") {
        doc["sops"] = sops_item.clone();
    }

    // Zeroize data key
    drop(data_key);

    let output = doc.to_string();

    if config.in_place {
        tokio::fs::write(&config.input_path, output.as_bytes()).await.map_err(|e| SopsError::FileWrite {
            path: config.input_path.clone(),
            source: e,
        })?;
        info!(path = %config.input_path.display(), "Updated keys in place");
    }

    Ok(output)
}

/// Decrypt the data key from any available key group in metadata.
async fn decrypt_data_key_from_any(
    metadata: &SopsFileMetadata,
    config: &UpdateKeysConfig,
) -> Result<Zeroizing<Vec<u8>>> {
    // Try Transit recipients
    for recipient in &metadata.aspen_transit {
        let ticket = config.cluster_ticket.as_deref().unwrap_or(&recipient.cluster_ticket);

        match TransitClient::connect(ticket, Some(&recipient.mount)).await {
            Ok(client) => match client.decrypt_data_key(&recipient.name, &recipient.enc).await {
                Ok(key) => return Ok(key),
                Err(e) => {
                    tracing::warn!(error = %e, "Transit decrypt failed, trying next");
                    continue;
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "Transit connect failed, trying next");
                continue;
            }
        }
    }

    Err(SopsError::NoMatchingKeyGroup)
}
