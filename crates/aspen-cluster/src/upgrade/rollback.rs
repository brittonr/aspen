//! Rollback logic for failed or completed upgrades.
//!
//! Two paths:
//! - Nix: `nix-env --profile <path> --rollback` (restores previous generation)
//! - Blob: restore `.bak` file, then restart

use std::path::Path;
use std::path::PathBuf;

use tracing::info;

use super::restart;
use super::types::NodeUpgradeError;
use super::types::RestartMethod;
use super::types::Result;
use super::types::UpgradeMethod;

/// Execute a rollback: restore previous binary and restart.
pub async fn execute_rollback(upgrade_method: &UpgradeMethod, restart_method: &RestartMethod) -> Result<()> {
    match upgrade_method {
        UpgradeMethod::Nix { profile_path } => {
            rollback_nix(profile_path).await?;
        }
        UpgradeMethod::Blob {
            binary_path,
            staging_dir: _,
        } => {
            rollback_blob(binary_path).await?;
        }
    }

    info!("rollback complete, initiating restart");
    // Rollback doesn't know the resolved binary — execve will use /proc/self/exe.
    // For Nix rollbacks, the profile already points to the previous generation,
    // but /proc/self/exe is the currently running binary which is fine for rollback.
    restart::restart_process(restart_method, None).await
}

/// Nix rollback: switch profile to previous generation.
async fn rollback_nix(profile_path: &Path) -> Result<()> {
    let profile_str = profile_path.display().to_string();
    info!(profile = %profile_str, "rolling back nix profile");

    let output = tokio::process::Command::new("nix-env")
        .args(["--profile", &profile_str, "--rollback"])
        .output()
        .await
        .map_err(|e| NodeUpgradeError::NixRollbackFailed {
            reason: format!("nix-env exec failed: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(NodeUpgradeError::NixRollbackFailed {
            reason: format!("nix-env --rollback failed: {stderr}"),
        });
    }

    info!(profile = %profile_str, "nix rollback complete");
    Ok(())
}

/// Blob rollback: restore .bak file.
async fn rollback_blob(binary_path: &Path) -> Result<()> {
    let bak_path = binary_path.with_extension("bak");

    if !bak_path.exists() {
        return Err(NodeUpgradeError::BinaryRollbackFailed {
            reason: format!("no .bak file at {}", bak_path.display()),
        });
    }

    info!(
        bak = %bak_path.display(),
        target = %binary_path.display(),
        "restoring backup binary"
    );

    // Atomic rename (same filesystem).
    tokio::fs::rename(&bak_path, binary_path)
        .await
        .map_err(|e| NodeUpgradeError::BinaryRollbackFailed {
            reason: format!("rename failed: {e}"),
        })?;

    info!(binary = %binary_path.display(), "blob rollback complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rollback_blob_no_bak() {
        let result = rollback_blob(&PathBuf::from("/tmp/nonexistent-aspen-node")).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            NodeUpgradeError::BinaryRollbackFailed { reason } => {
                assert!(reason.contains(".bak"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[tokio::test]
    async fn test_rollback_blob_with_bak() {
        let dir = tempfile::TempDir::new().unwrap();
        let binary_path = dir.path().join("aspen-node");
        let bak_path = dir.path().join("aspen-node.bak");

        // Create the "old" binary backup.
        tokio::fs::write(&bak_path, b"old-binary-content").await.unwrap();
        // Create the "new" (failed) binary.
        tokio::fs::write(&binary_path, b"new-binary-content").await.unwrap();

        let result = rollback_blob(&binary_path).await;
        assert!(result.is_ok());

        // Verify the binary now has the old content.
        let content = tokio::fs::read(&binary_path).await.unwrap();
        assert_eq!(content, b"old-binary-content");

        // .bak should be gone (renamed over).
        assert!(!bak_path.exists());
    }
}
