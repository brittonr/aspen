//! SOPS edit — decrypt to temp file, open in editor, re-encrypt on save.

use std::path::PathBuf;

use tracing::info;

use crate::decrypt::DecryptConfig;
use crate::decrypt::decrypt_file;
use crate::encrypt::EncryptConfig;
use crate::encrypt::encrypt_file;
use crate::error::Result;
use crate::error::SopsError;
use crate::format;

/// Configuration for the edit operation.
#[derive(Debug, Clone)]
pub struct EditConfig {
    /// Path to the SOPS-encrypted file.
    pub input_path: PathBuf,
    /// Aspen cluster ticket.
    pub cluster_ticket: String,
    /// Editor command (default: $EDITOR or vi).
    pub editor: Option<String>,
    /// Transit key name.
    pub transit_key: String,
    /// Transit mount point.
    pub transit_mount: String,
}

/// Edit a SOPS-encrypted file.
///
/// 1. Decrypt to a temp file (mode 0600)
/// 2. Open in $EDITOR
/// 3. If file changed: re-encrypt with same key groups
/// 4. If unchanged: no-op
/// 5. Securely delete temp file
pub async fn edit_file(config: &EditConfig) -> Result<()> {
    // Detect format for proper temp file extension
    let fmt = format::detect_format(&config.input_path)?;

    // Decrypt
    let decrypt_config = DecryptConfig {
        input_path: config.input_path.clone(),
        cluster_ticket: Some(config.cluster_ticket.clone()),
        output_path: None,
        extract_path: None,
        #[cfg(feature = "age-fallback")]
        age_identity: None,
    };

    let decrypted = decrypt_file(&decrypt_config).await?;

    // Write to temp file with restricted permissions (use correct extension)
    let tmp_dir = std::env::var("TMPDIR")
        .or_else(|_| std::env::var("XDG_RUNTIME_DIR"))
        .unwrap_or_else(|_| "/tmp".into());

    let tmp_path = PathBuf::from(&tmp_dir).join(format!(".aspen-sops-edit-{}.{}", std::process::id(), fmt.extension()));

    // Write with 0600 permissions
    tokio::fs::write(&tmp_path, &decrypted).await.map_err(|e| SopsError::FileWrite {
        path: tmp_path.clone(),
        source: e,
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&tmp_path, perms).map_err(|e| SopsError::FileWrite {
            path: tmp_path.clone(),
            source: e,
        })?;
    }

    // Record hash before editing
    let hash_before = blake3::hash(decrypted.as_bytes());

    // Determine editor
    let editor = config
        .editor
        .clone()
        .or_else(|| std::env::var("EDITOR").ok())
        .or_else(|| std::env::var("VISUAL").ok())
        .unwrap_or_else(|| "vi".into());

    // Spawn editor
    let status =
        tokio::process::Command::new(&editor)
            .arg(&tmp_path)
            .status()
            .await
            .map_err(|e| SopsError::EditorFailed {
                reason: format!("failed to launch editor '{editor}': {e}"),
            })?;

    if !status.success() {
        // Clean up temp file before returning error
        secure_delete(&tmp_path).await;
        return Err(SopsError::EditorFailed {
            reason: format!("editor exited with status: {status}"),
        });
    }

    // Read the edited file
    let edited = tokio::fs::read_to_string(&tmp_path).await.map_err(|e| SopsError::FileRead {
        path: tmp_path.clone(),
        source: e,
    })?;

    // Check if changed
    let hash_after = blake3::hash(edited.as_bytes());

    if hash_before == hash_after {
        info!("No changes detected, skipping re-encryption");
        secure_delete(&tmp_path).await;
        return Ok(());
    }

    // Write the edited plaintext to the original path temporarily,
    // then encrypt in place
    tokio::fs::write(&config.input_path, &edited).await.map_err(|e| SopsError::FileWrite {
        path: config.input_path.clone(),
        source: e,
    })?;

    let encrypt_config = EncryptConfig {
        input_path: config.input_path.clone(),
        cluster_ticket: config.cluster_ticket.clone(),
        transit_key: config.transit_key.clone(),
        transit_mount: config.transit_mount.clone(),
        in_place: true,
        ..Default::default()
    };

    encrypt_file(&encrypt_config).await?;

    info!(path = %config.input_path.display(), format = ?fmt, "Re-encrypted edited file");

    // Secure cleanup
    secure_delete(&tmp_path).await;

    Ok(())
}

/// Overwrite a file with zeros and then delete it.
async fn secure_delete(path: &PathBuf) {
    // Best-effort: overwrite with zeros
    if let Ok(metadata) = tokio::fs::metadata(path).await {
        let zeros = vec![0u8; metadata.len() as usize];
        let _ = tokio::fs::write(path, &zeros).await;
    }
    let _ = tokio::fs::remove_file(path).await;
}
