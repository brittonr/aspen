//! Job payload for Nix builds.

use std::path::PathBuf;

use aspen_ci_core::CiCoreError;
use aspen_ci_core::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::config::DEFAULT_TIMEOUT_SECS;
use crate::config::MAX_ATTR_LENGTH;
use crate::config::MAX_FLAKE_URL_LENGTH;
use crate::config::MAX_TIMEOUT_SECS;

fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_SECS
}

fn default_true() -> bool {
    true
}

/// Job payload for Nix builds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NixBuildPayload {
    /// CI job name for status tracking.
    #[serde(default)]
    pub job_name: Option<String>,

    /// Pipeline run ID for log streaming.
    #[serde(default)]
    pub run_id: Option<String>,

    /// Flake URL (e.g., ".", "github:owner/repo", "path:/some/path").
    pub flake_url: String,

    /// Attribute path within the flake (e.g., "packages.x86_64-linux.default").
    pub attribute: String,

    /// Extra arguments to pass to `nix build`.
    #[serde(default)]
    pub extra_args: Vec<String>,

    /// Working directory for the build.
    #[serde(default)]
    pub working_dir: Option<PathBuf>,

    /// Build timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Whether to use sandbox mode.
    #[serde(default = "default_true")]
    pub sandbox: bool,

    /// Cache key for build caching.
    #[serde(default)]
    pub cache_key: Option<String>,

    /// Glob patterns for artifacts to collect.
    #[serde(default)]
    pub artifacts: Vec<String>,

    /// Whether to upload build results to the blob store as NAR archives.
    /// Defaults to true when a blob store is configured.
    #[serde(default = "default_true")]
    pub should_upload_result: bool,

    /// Whether to publish output store paths to the distributed Nix cache.
    /// Defaults to true. When enabled, build outputs are registered in the
    /// snix PathInfoService so they're available via the cache gateway.
    #[serde(default = "default_true")]
    pub publish_to_cache: bool,

    /// Specific outputs to publish to cache (e.g., ["out", "dev"]).
    /// If empty, all outputs are published. Only used when `publish_to_cache` is true.
    #[serde(default)]
    pub cache_outputs: Vec<String>,

    /// Source archive hash for workspace seeding in VM workers.
    ///
    /// When a `ci_nix_build` job is dispatched to a VM worker, the host's
    /// `working_dir` is inaccessible. This hash lets the VM download the
    /// checkout from the blob store instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_hash: Option<String>,
}

impl NixBuildPayload {
    /// Validate the payload.
    pub fn validate(&self) -> Result<()> {
        if self.flake_url.is_empty() {
            return Err(CiCoreError::InvalidConfig {
                reason: "flake_url cannot be empty".to_string(),
            });
        }

        if self.flake_url.len() > MAX_FLAKE_URL_LENGTH {
            return Err(CiCoreError::InvalidConfig {
                reason: format!("flake_url too long: {} bytes (max: {})", self.flake_url.len(), MAX_FLAKE_URL_LENGTH),
            });
        }

        if self.attribute.len() > MAX_ATTR_LENGTH {
            return Err(CiCoreError::InvalidConfig {
                reason: format!("attribute too long: {} bytes (max: {})", self.attribute.len(), MAX_ATTR_LENGTH),
            });
        }

        if self.timeout_secs > MAX_TIMEOUT_SECS {
            return Err(CiCoreError::InvalidConfig {
                reason: format!("timeout too long: {} seconds (max: {})", self.timeout_secs, MAX_TIMEOUT_SECS),
            });
        }

        Ok(())
    }

    /// Build the flake reference string.
    pub fn flake_ref(&self) -> String {
        if self.attribute.is_empty() {
            self.flake_url.clone()
        } else {
            format!("{}#{}", self.flake_url, self.attribute)
        }
    }
}
