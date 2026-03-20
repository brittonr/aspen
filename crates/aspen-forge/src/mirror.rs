//! Mirror configuration and worker for repository mirroring.
//!
//! Mirrors periodically sync refs from an upstream repository to a local copy.
//! Only fast-forward updates are applied automatically; diverged refs are skipped
//! with a warning.

use serde::Deserialize;
use serde::Serialize;

use crate::identity::RepoId;

/// Configuration for a mirrored repository.
///
/// Stored in KV at `forge:mirror:{repo_id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorConfig {
    /// The upstream repository to mirror from.
    pub upstream_repo_id: RepoId,

    /// Optional public key of the upstream cluster.
    /// `None` means the upstream is in the same cluster.
    pub upstream_cluster: Option<iroh::PublicKey>,

    /// Sync interval in seconds (clamped to [60, 3600]).
    pub interval_secs: u32,

    /// Whether this mirror is currently enabled.
    pub enabled: bool,

    /// Timestamp of last successful sync (milliseconds, 0 if never synced).
    pub last_sync_ms: u64,

    /// Number of refs synced in the last sync.
    pub last_synced_refs_count: u32,
}

impl MirrorConfig {
    /// Create a new mirror config with validated interval.
    ///
    /// The interval is clamped to [`MIN_MIRROR_INTERVAL_SECS`, `MAX_MIRROR_INTERVAL_SECS`].
    pub fn new(upstream_repo_id: RepoId, interval_secs: u32) -> Self {
        let clamped =
            interval_secs.clamp(crate::constants::MIN_MIRROR_INTERVAL_SECS, crate::constants::MAX_MIRROR_INTERVAL_SECS);

        Self {
            upstream_repo_id,
            upstream_cluster: None,
            interval_secs: clamped,
            enabled: true,
            last_sync_ms: 0,
            last_synced_refs_count: 0,
        }
    }

    /// Set the upstream cluster.
    pub fn with_upstream_cluster(mut self, cluster: iroh::PublicKey) -> Self {
        self.upstream_cluster = Some(cluster);
        self
    }

    /// Check if the mirror is due for a sync based on current time.
    pub fn is_due(&self, now_ms: u64) -> bool {
        if !self.enabled {
            return false;
        }
        let interval_ms = u64::from(self.interval_secs).saturating_mul(1000);
        now_ms.saturating_sub(self.last_sync_ms) >= interval_ms
    }
}

/// Status of a mirror.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorStatus {
    /// The mirror configuration.
    pub config: MirrorConfig,

    /// Whether the mirror is due for sync.
    pub is_due: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mirror_config_interval_clamping() {
        // Below minimum
        let config = MirrorConfig::new(RepoId([0; 32]), 10);
        assert_eq!(config.interval_secs, crate::constants::MIN_MIRROR_INTERVAL_SECS);

        // Above maximum
        let config = MirrorConfig::new(RepoId([0; 32]), 99999);
        assert_eq!(config.interval_secs, crate::constants::MAX_MIRROR_INTERVAL_SECS);

        // Within range
        let config = MirrorConfig::new(RepoId([0; 32]), 300);
        assert_eq!(config.interval_secs, 300);
    }

    #[test]
    fn test_mirror_config_is_due() {
        let mut config = MirrorConfig::new(RepoId([0; 32]), 60);
        config.last_sync_ms = 1000;

        // Not due yet (only 30s passed)
        assert!(!config.is_due(31_000));

        // Due (61s passed)
        assert!(config.is_due(62_000));

        // Disabled mirror is never due
        config.enabled = false;
        assert!(!config.is_due(999_999));
    }

    #[test]
    fn test_mirror_config_serialization() {
        let config = MirrorConfig::new(RepoId([1; 32]), 300);
        let json = serde_json::to_string(&config).unwrap();
        let recovered: MirrorConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.upstream_repo_id, config.upstream_repo_id);
        assert_eq!(recovered.interval_secs, 300);
        assert!(recovered.enabled);
    }
}
