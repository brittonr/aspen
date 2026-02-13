//! SNIX content-addressed storage configuration.
//!
//! SNIX provides decomposed content-addressed storage for Nix artifacts.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// SNIX content-addressed storage configuration.
///
/// SNIX provides decomposed content-addressed storage for Nix artifacts:
/// - Blobs: Raw content chunks stored in iroh-blobs
/// - Directories: Merkle tree nodes stored in Raft KV
/// - PathInfo: Nix store path metadata stored in Raft KV
///
/// This enables efficient deduplication and P2P distribution of build artifacts.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SnixConfig {
    /// Enable SNIX storage layer.
    ///
    /// When enabled, Nix build artifacts are stored using SNIX's decomposed
    /// content-addressed format instead of monolithic NAR archives.
    ///
    /// Default: false
    #[serde(default)]
    pub enabled: bool,

    /// KV key prefix for directory metadata.
    ///
    /// Directory nodes (Merkle tree structure) are stored under this prefix.
    ///
    /// Default: "_snix:dir:"
    #[serde(default = "default_snix_dir_prefix")]
    pub directory_prefix: String,

    /// KV key prefix for PathInfo metadata.
    ///
    /// Nix store path metadata (NAR hash, size, references) stored under this prefix.
    ///
    /// Default: "_snix:pathinfo:"
    #[serde(default = "default_snix_pathinfo_prefix")]
    pub pathinfo_prefix: String,

    /// Enable automatic migration from legacy NAR storage.
    ///
    /// When enabled, existing NAR archives in iroh-blobs are automatically
    /// decomposed into SNIX format during background migration.
    ///
    /// Default: false
    #[serde(default)]
    pub migration_enabled: bool,

    /// Number of concurrent migration workers.
    ///
    /// Controls parallelism for background migration from legacy NAR storage.
    /// Higher values speed up migration but increase resource usage.
    ///
    /// Tiger Style: Max 16 workers.
    ///
    /// Default: 4
    #[serde(default = "default_migration_workers")]
    pub migration_workers: u32,
}

impl Default for SnixConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            directory_prefix: default_snix_dir_prefix(),
            pathinfo_prefix: default_snix_pathinfo_prefix(),
            migration_enabled: false,
            migration_workers: default_migration_workers(),
        }
    }
}

pub(crate) fn default_snix_dir_prefix() -> String {
    "_snix:dir:".to_string()
}

pub(crate) fn default_snix_pathinfo_prefix() -> String {
    "_snix:pathinfo:".to_string()
}

pub(crate) fn default_migration_workers() -> u32 {
    4
}
