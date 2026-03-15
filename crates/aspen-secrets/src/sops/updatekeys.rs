//! Update key groups in a SOPS file — add/remove recipients.
//!
//! Decrypts the data key using any available key group, then
//! re-encrypts it for the new set of recipients.

use std::path::PathBuf;

use tracing::info;
use zeroize::Zeroizing;

use super::client::TransitClient;
use super::format;
use super::metadata::AspenTransitRecipient;
use super::metadata::SopsFileMetadata;
use super::sops_constants::MAX_SOPS_FILE_SIZE;
use super::sops_error::Result;
use super::sops_error::SopsError;

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
    let fmt = format::detect_format(&config.input_path)?;

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

    let mut metadata =
        format::extract_metadata(fmt, &contents, &config.input_path)?.ok_or(SopsError::InvalidMetadata {
            reason: "no sops section found".into(),
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
        let key_name = config.transit_key.as_deref().unwrap_or(super::sops_constants::DEFAULT_TRANSIT_KEY);

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
    for age_recipient in &config.add_age {
        let enc = super::encrypt::encrypt_data_key_for_age(age_recipient, &data_key)?;
        metadata.add_age_recipient(super::metadata::AgeRecipient {
            recipient: age_recipient.clone(),
            enc: Some(enc),
        });
        info!(recipient = age_recipient, "Added age recipient");
    }

    // Sync hc_vault_transit entries with aspen_transit
    let hc_mirrors: Vec<_> = metadata
        .aspen_transit
        .iter()
        .map(|a| super::metadata::HcVaultTransitRecipient {
            vault_address: "aspen".into(),
            engine_path: a.mount.clone(),
            key_name: a.name.clone(),
            enc: a.enc.clone(),
            created_at: metadata.lastmodified.clone(),
        })
        .collect();
    metadata.hc_vault_transit = hc_mirrors;

    // Sync key_groups for Go SOPS 3.7+ interop
    metadata.sync_key_groups();

    metadata.touch();

    // Zeroize data key
    drop(data_key);

    // Update metadata in the document (format-agnostic)
    let output = format::update_metadata(fmt, &contents, &metadata, &config.input_path)?;

    if config.in_place {
        tokio::fs::write(&config.input_path, output.as_bytes()).await.map_err(|e| SopsError::FileWrite {
            path: config.input_path.clone(),
            source: e,
        })?;
        info!(path = %config.input_path.display(), format = ?fmt, "Updated keys in place");
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
