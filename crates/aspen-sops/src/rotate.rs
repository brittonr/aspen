//! SOPS key rotation — re-wrap data key with latest Transit key version.
//!
//! After rotating a Transit key, this re-encrypts only the data key.
//! The encrypted values and MAC stay unchanged (same data key, just re-wrapped).

use std::path::PathBuf;

use tracing::info;

use crate::client::TransitClient;
use crate::constants::MAX_SOPS_FILE_SIZE;
use crate::error::Result;
use crate::error::SopsError;
use crate::format;

/// Configuration for rotating a file's data key wrapping.
#[derive(Debug, Clone)]
pub struct RotateConfig {
    /// Path to the SOPS-encrypted file.
    pub input_path: PathBuf,
    /// Aspen cluster ticket (optional — uses metadata ticket).
    pub cluster_ticket: Option<String>,
    /// Rotate in place.
    pub in_place: bool,
}

/// Rotate the data key wrapping in a SOPS file.
///
/// For each Aspen Transit recipient, calls `SecretsTransitRewrap` to
/// re-encrypt the data key with the latest key version. The encrypted
/// values and MAC are unchanged.
///
/// Returns the updated file contents.
pub async fn rotate_file(config: &RotateConfig) -> Result<String> {
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

    // Rewrap data key for each Transit recipient
    for recipient in &mut metadata.aspen_transit {
        let ticket = config.cluster_ticket.as_deref().unwrap_or(&recipient.cluster_ticket);

        let client = TransitClient::connect(ticket, Some(&recipient.mount)).await?;
        let (new_enc, new_version) = client.rewrap_data_key(&recipient.name, &recipient.enc).await?;

        info!(
            key = recipient.name,
            old_version = recipient.key_version,
            new_version = new_version,
            "Rewrapped data key"
        );

        recipient.enc = new_enc;
        recipient.key_version = new_version;
    }

    metadata.touch();

    // Update metadata in the document (format-agnostic)
    let output = format::update_metadata(fmt, &contents, &metadata, &config.input_path)?;

    if config.in_place {
        tokio::fs::write(&config.input_path, output.as_bytes()).await.map_err(|e| SopsError::FileWrite {
            path: config.input_path.clone(),
            source: e,
        })?;
        info!(path = %config.input_path.display(), format = ?fmt, "Rotated file in place");
    }

    Ok(output)
}
