//! Workspace seeding from blob store.
//!
//! This module handles populating a VM's workspace directory with source code
//! downloaded from the distributed blob store before job execution.

#![allow(dead_code)] // API surface for workspace seeding

use std::path::Path;
use std::sync::Arc;

use aspen_blob::BlobStore;
use iroh_blobs::Hash;
use tracing::{debug, info};

use super::error::{CloudHypervisorError, Result};

/// Seed a workspace directory from a blob.
///
/// The blob is expected to contain a tar.gz archive of the source tree.
/// The archive is extracted to the workspace directory.
///
/// # Arguments
/// * `blob_store` - The blob store to download from
/// * `source_hash` - The hex-encoded blake3 hash of the source blob
/// * `workspace_dir` - The directory to extract to
///
/// # Returns
/// Ok(bytes_written) on success, or an error if seeding failed
pub async fn seed_workspace_from_blob(
    blob_store: &Arc<dyn BlobStore>,
    source_hash: &str,
    workspace_dir: &Path,
) -> Result<u64> {
    // Parse the hash
    let hash = parse_hash(source_hash)?;

    info!(
        hash = %source_hash,
        workspace = ?workspace_dir,
        "seeding workspace from blob"
    );

    // Download the blob content
    let content = blob_store.get_bytes(&hash).await.map_err(|e| CloudHypervisorError::WorkspaceSeed {
        reason: format!("failed to download blob {}: {}", source_hash, e),
    })?;

    let content = content.ok_or_else(|| CloudHypervisorError::WorkspaceSeed {
        reason: format!("blob {} not found in store", source_hash),
    })?;

    let content_len = content.len() as u64;

    // Ensure workspace directory exists
    tokio::fs::create_dir_all(workspace_dir).await.map_err(|e| CloudHypervisorError::WorkspaceSeed {
        reason: format!("failed to create workspace directory: {}", e),
    })?;

    // Extract the archive
    // The blob should be a tar.gz archive
    extract_tar_gz(&content, workspace_dir).await?;

    info!(
        hash = %source_hash,
        bytes = content_len,
        workspace = ?workspace_dir,
        "workspace seeded successfully"
    );

    Ok(content_len)
}

/// Parse a hex-encoded hash string into an iroh Hash.
fn parse_hash(hash_str: &str) -> Result<Hash> {
    // Try parsing as hex (blake3 format)
    let bytes = hex::decode(hash_str).map_err(|e| CloudHypervisorError::WorkspaceSeed {
        reason: format!("invalid hash format '{}': {}", hash_str, e),
    })?;

    if bytes.len() != 32 {
        return Err(CloudHypervisorError::WorkspaceSeed {
            reason: format!("hash '{}' has invalid length: expected 32 bytes, got {}", hash_str, bytes.len()),
        });
    }

    let hash_bytes: [u8; 32] = bytes.try_into().map_err(|_| CloudHypervisorError::WorkspaceSeed {
        reason: format!("failed to convert hash bytes for '{}'", hash_str),
    })?;

    Ok(Hash::from(hash_bytes))
}

/// Extract a tar.gz archive to a directory.
async fn extract_tar_gz(data: &[u8], dest: &Path) -> Result<()> {
    use flate2::read::GzDecoder;
    use std::io::Cursor;
    use tar::Archive;

    // Copy data to owned buffer for 'static lifetime in spawn_blocking
    let data_owned = data.to_vec();
    let dest_owned = dest.to_path_buf();
    let dest_for_log = dest.to_path_buf();

    // Extract to destination
    // Run in blocking task since tar extraction is synchronous
    tokio::task::spawn_blocking(move || {
        let cursor = Cursor::new(data_owned);

        // Try to decompress as gzip
        let decompressor = GzDecoder::new(cursor);

        // Create tar archive from decompressed stream
        let mut archive = Archive::new(decompressor);

        archive.unpack(&dest_owned).map_err(|e| CloudHypervisorError::WorkspaceSeed {
            reason: format!("failed to extract archive: {}", e),
        })
    })
    .await
    .map_err(|e| CloudHypervisorError::WorkspaceSeed {
        reason: format!("archive extraction task panicked: {}", e),
    })??;

    debug!(dest = ?dest_for_log, "archive extracted");

    Ok(())
}

/// Seed workspace from raw file content (not compressed).
///
/// This is used when the blob is a single file rather than an archive.
pub async fn seed_workspace_with_file(
    blob_store: &Arc<dyn BlobStore>,
    source_hash: &str,
    workspace_dir: &Path,
    filename: &str,
) -> Result<u64> {
    let hash = parse_hash(source_hash)?;

    info!(
        hash = %source_hash,
        filename = %filename,
        workspace = ?workspace_dir,
        "seeding workspace with single file"
    );

    let content = blob_store.get_bytes(&hash).await.map_err(|e| CloudHypervisorError::WorkspaceSeed {
        reason: format!("failed to download blob {}: {}", source_hash, e),
    })?;

    let content = content.ok_or_else(|| CloudHypervisorError::WorkspaceSeed {
        reason: format!("blob {} not found in store", source_hash),
    })?;

    let content_len = content.len() as u64;

    // Ensure workspace directory exists
    tokio::fs::create_dir_all(workspace_dir).await.map_err(|e| CloudHypervisorError::WorkspaceSeed {
        reason: format!("failed to create workspace directory: {}", e),
    })?;

    // Write the file
    let file_path = workspace_dir.join(filename);
    tokio::fs::write(&file_path, &content).await.map_err(|e| CloudHypervisorError::WorkspaceSeed {
        reason: format!("failed to write file {}: {}", file_path.display(), e),
    })?;

    info!(
        hash = %source_hash,
        bytes = content_len,
        file = ?file_path,
        "file seeded successfully"
    );

    Ok(content_len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hash_valid() {
        // 32 bytes in hex = 64 characters
        let hash_str = "a".repeat(64);
        let result = parse_hash(&hash_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_hash_invalid_hex() {
        let result = parse_hash("not-valid-hex");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_hash_wrong_length() {
        // Too short
        let result = parse_hash("aabb");
        assert!(result.is_err());
    }
}
