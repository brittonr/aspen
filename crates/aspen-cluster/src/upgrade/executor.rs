//! NodeUpgradeExecutor — orchestrates the full upgrade lifecycle for a single node.

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use aspen_constants::api::DRAIN_TIMEOUT_SECS;
use aspen_deploy::DeployArtifact;
use aspen_deploy::NodeDeployStatus;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::drain;
use super::restart;
use super::rollback;
use super::status;
use super::types::*;

/// Executes node-level upgrades: drain → replace binary → restart.
pub struct NodeUpgradeExecutor {
    config: NodeUpgradeConfig,
    drain_state: Arc<DrainState>,
}

impl NodeUpgradeExecutor {
    /// Create a new executor with the given configuration.
    pub fn new(config: NodeUpgradeConfig) -> Self {
        Self {
            config,
            drain_state: DrainState::new(),
        }
    }

    /// Create an executor with a shared drain state (for integration with RPC handlers).
    pub fn with_drain_state(config: NodeUpgradeConfig, drain_state: Arc<DrainState>) -> Self {
        Self { config, drain_state }
    }

    /// Get a reference to the drain state (for RPC handlers to check).
    pub fn drain_state(&self) -> &Arc<DrainState> {
        &self.drain_state
    }

    /// Execute the full upgrade: drain → replace → restart.
    ///
    /// Reports status transitions to the provided callback. If `status_writer` is
    /// None, status changes are only logged.
    ///
    /// Returns Ok(()) on success. The process may not return from this function
    /// if restart succeeds (execve replaces the process).
    pub async fn execute(&self, artifact: &DeployArtifact, status_writer: Option<&dyn StatusWriter>) -> Result<()> {
        let node_id = self.config.node_id;

        // Phase 1: Drain
        self.report_status(status_writer, NodeDeployStatus::Draining).await;

        let drain_timeout = Duration::from_secs(self.config.drain_timeout_secs);
        let drain_result = drain::execute_drain(&self.drain_state, drain_timeout).await;

        if !drain_result.completed {
            warn!(
                node_id,
                cancelled_ops = drain_result.cancelled_ops,
                "drain timeout: proceeding with upgrade despite in-flight operations"
            );
        }

        // Phase 2: Replace binary
        self.report_status(status_writer, NodeDeployStatus::Upgrading).await;

        match artifact {
            DeployArtifact::NixStorePath(store_path) => {
                self.upgrade_nix(store_path).await?;
            }
            DeployArtifact::BlobHash(blob_hash) => {
                self.upgrade_blob(blob_hash).await?;
            }
        }

        // Phase 3: Restart
        self.report_status(status_writer, NodeDeployStatus::Restarting).await;

        info!(node_id, "binary replaced, initiating restart");
        restart::restart_process(&self.config.restart_method).await?;

        // If we reach here (shouldn't for execve), report healthy.
        // Systemd restart will kill this process, so this is a no-op path.
        Ok(())
    }

    /// Execute Nix profile switch.
    async fn upgrade_nix(&self, store_path: &str) -> Result<()> {
        let profile_path = match &self.config.upgrade_method {
            UpgradeMethod::Nix { profile_path } => profile_path,
            UpgradeMethod::Blob { .. } => {
                return Err(NodeUpgradeError::NixProfileSwitchFailed {
                    reason: "upgrade method is blob, not nix".into(),
                });
            }
        };

        // Verify the binary exists at the store path.
        let binary_path = PathBuf::from(store_path).join("bin/aspen-node");
        if !binary_path.exists() {
            // Attempt to fetch from binary cache via nix-store --realise.
            info!(store_path, "store path not local, attempting realise");
            let realise =
                tokio::process::Command::new("nix-store").args(["--realise", store_path]).output().await.map_err(
                    |e| NodeUpgradeError::StorePathUnavailable {
                        path: format!("{store_path}: {e}"),
                    },
                )?;

            if !realise.status.success() {
                let stderr = String::from_utf8_lossy(&realise.stderr);
                return Err(NodeUpgradeError::StorePathUnavailable {
                    path: format!("{store_path}: {stderr}"),
                });
            }

            // Verify again after realise.
            if !binary_path.exists() {
                return Err(NodeUpgradeError::StorePathUnavailable {
                    path: format!("{}: bin/aspen-node not found", store_path),
                });
            }
        }

        // Switch the Nix profile.
        let profile_str = profile_path.display().to_string();
        info!(store_path, profile = %profile_str, "switching nix profile");

        let switch = tokio::process::Command::new("nix-env")
            .args(["--profile", &profile_str, "--set", store_path])
            .output()
            .await
            .map_err(|e| NodeUpgradeError::NixProfileSwitchFailed {
                reason: format!("nix-env exec failed: {e}"),
            })?;

        if !switch.status.success() {
            let stderr = String::from_utf8_lossy(&switch.stderr);
            return Err(NodeUpgradeError::NixProfileSwitchFailed {
                reason: format!("nix-env --set failed: {stderr}"),
            });
        }

        info!(store_path, "nix profile switch complete");
        Ok(())
    }

    /// Execute blob-based binary replacement.
    async fn upgrade_blob(&self, blob_hash: &str) -> Result<()> {
        let (binary_path, staging_dir) = match &self.config.upgrade_method {
            UpgradeMethod::Blob {
                binary_path,
                staging_dir,
            } => (binary_path, staging_dir),
            UpgradeMethod::Nix { .. } => {
                return Err(NodeUpgradeError::BinaryReplacementFailed {
                    reason: "upgrade method is nix, not blob".into(),
                });
            }
        };

        // Ensure staging directory exists.
        tokio::fs::create_dir_all(staging_dir).await.map_err(|e| NodeUpgradeError::ArtifactFetchFailed {
            reason: format!("create staging dir: {e}"),
        })?;

        let staging_path = staging_dir.join(format!("aspen-node-{blob_hash}"));

        // The blob must be staged to disk by the RPC handler before calling
        // the executor. The handler (handle_node_upgrade in deploy.rs) downloads
        // the blob via iroh-blobs BlobRead and writes it to the staging path.
        if !staging_path.exists() {
            return Err(NodeUpgradeError::ArtifactFetchFailed {
                reason: format!("staged blob not found at {}", staging_path.display()),
            });
        }

        // Validate: run `--version` on the staged binary.
        validate_binary(&staging_path).await?;

        // Preserve the old binary as .bak.
        let bak_path = binary_path.with_extension("bak");
        if binary_path.exists() {
            tokio::fs::copy(binary_path, &bak_path)
                .await
                .map_err(|e| NodeUpgradeError::BinaryReplacementFailed {
                    reason: format!("backup copy failed: {e}"),
                })?;
            info!(bak = %bak_path.display(), "preserved old binary as .bak");
        }

        // Atomic rename (same filesystem).
        tokio::fs::rename(&staging_path, binary_path)
            .await
            .map_err(|e| NodeUpgradeError::BinaryReplacementFailed {
                reason: format!("atomic rename failed: {e}"),
            })?;

        // Set executable permission.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            tokio::fs::set_permissions(binary_path, perms).await.map_err(|e| {
                NodeUpgradeError::BinaryReplacementFailed {
                    reason: format!("set permissions failed: {e}"),
                }
            })?;
        }

        info!(binary = %binary_path.display(), "binary replacement complete");
        Ok(())
    }

    /// Report a status transition.
    async fn report_status(&self, writer: Option<&dyn StatusWriter>, status: NodeDeployStatus) {
        info!(node_id = self.config.node_id, status = status.as_status_str(), "upgrade status transition");
        if let Some(w) = writer {
            if let Err(e) = w.write_status(self.config.node_id, &status).await {
                error!(
                    node_id = self.config.node_id,
                    error = %e,
                    "failed to write upgrade status"
                );
            }
        }
    }
}

/// Validate a binary by running `--version` and checking exit code.
pub async fn validate_binary(path: &Path) -> Result<()> {
    let output = tokio::process::Command::new(path).arg("--version").output().await.map_err(|e| {
        NodeUpgradeError::BinaryValidationFailed {
            reason: format!("exec failed: {e}"),
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(NodeUpgradeError::BinaryValidationFailed {
            reason: format!("exit code {}: {}", output.status, stderr),
        });
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    info!(version = %version_str.trim(), "binary validation passed");
    Ok(())
}

/// Trait for writing deployment status to cluster KV.
///
/// Implemented by the deployment coordinator to report per-node
/// status transitions via `_sys:deploy:node:{node_id}`.
#[async_trait::async_trait]
pub trait StatusWriter: Send + Sync {
    /// Write a status transition for the given node.
    async fn write_status(
        &self,
        node_id: u64,
        status: &NodeDeployStatus,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_upgrade_config() {
        let config = NodeUpgradeConfig {
            node_id: 1,
            upgrade_method: UpgradeMethod::Nix {
                profile_path: PathBuf::from("/nix/var/nix/profiles/aspen-node"),
            },
            restart_method: RestartMethod::Systemd {
                unit_name: "aspen-node".into(),
            },
            drain_timeout_secs: DRAIN_TIMEOUT_SECS,
        };

        assert_eq!(config.node_id, 1);
        assert_eq!(config.drain_timeout_secs, 30);
    }

    #[test]
    fn test_node_upgrade_config_blob() {
        let config = NodeUpgradeConfig {
            node_id: 2,
            upgrade_method: UpgradeMethod::Blob {
                binary_path: PathBuf::from("/usr/local/bin/aspen-node"),
                staging_dir: PathBuf::from("/tmp/aspen-staging"),
            },
            restart_method: RestartMethod::Execve,
            drain_timeout_secs: 60,
        };

        assert_eq!(config.node_id, 2);
        assert_eq!(config.restart_method, RestartMethod::Execve);
    }

    #[test]
    fn test_executor_new() {
        let config = NodeUpgradeConfig {
            node_id: 1,
            upgrade_method: UpgradeMethod::Nix {
                profile_path: PathBuf::from("/nix/var/nix/profiles/aspen-node"),
            },
            restart_method: RestartMethod::Systemd {
                unit_name: "aspen-node".into(),
            },
            drain_timeout_secs: DRAIN_TIMEOUT_SECS,
        };

        let executor = NodeUpgradeExecutor::new(config);
        assert!(!executor.drain_state().is_draining());
        assert_eq!(executor.drain_state().in_flight_count(), 0);
    }

    #[test]
    fn test_executor_shared_drain_state() {
        let drain = DrainState::new();
        let config = NodeUpgradeConfig {
            node_id: 1,
            upgrade_method: UpgradeMethod::Nix {
                profile_path: PathBuf::from("/nix/var/nix/profiles/aspen-node"),
            },
            restart_method: RestartMethod::Systemd {
                unit_name: "aspen-node".into(),
            },
            drain_timeout_secs: DRAIN_TIMEOUT_SECS,
        };

        let executor = NodeUpgradeExecutor::with_drain_state(config, drain.clone());
        // Drain state is shared.
        drain.is_draining.store(true, std::sync::atomic::Ordering::Release);
        assert!(executor.drain_state().is_draining());
    }
}
