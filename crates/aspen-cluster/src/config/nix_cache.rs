//! Nix binary cache configuration.
//!
//! Enables serving Nix store paths via HTTP/3 over Iroh QUIC.

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Nix binary cache configuration.
///
/// Enables serving Nix store paths via HTTP/3 over Iroh QUIC. Clients connect
/// using the `iroh+h3` ALPN and can fetch NARs, narinfo files, and perform
/// cache queries.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NixCacheConfig {
    /// Enable the Nix binary cache HTTP/3 gateway.
    ///
    /// When enabled, the node serves Nix store paths via the Nix binary cache
    /// protocol over HTTP/3 (using Iroh's QUIC transport).
    ///
    /// Default: false
    #[serde(default, rename = "enabled")]
    pub is_enabled: bool,

    /// Nix store directory path.
    ///
    /// The local Nix store directory to serve. This is used for cache metadata.
    ///
    /// Default: "/nix/store"
    #[serde(default = "default_nix_store_dir")]
    pub store_dir: PathBuf,

    /// Cache priority for substitution (lower = preferred).
    ///
    /// When clients query multiple caches, lower priority values are tried first.
    /// cache.nixos.org uses priority 40, so values < 40 will be preferred.
    ///
    /// Default: 30
    #[serde(default = "default_nix_cache_priority")]
    pub priority: u32,

    /// Enable mass query support for efficient batch lookups.
    ///
    /// When enabled, clients can query multiple store paths in a single request.
    /// This significantly improves performance for large dependency trees.
    ///
    /// Default: true
    #[serde(default = "default_want_mass_query")]
    pub want_mass_query: bool,

    /// Optional cache name for signing.
    ///
    /// When set, NARs are signed with the corresponding key from the secrets
    /// manager. The signing key must be loaded via the `secrets` feature.
    ///
    /// Example: "cache.example.com-1"
    pub cache_name: Option<String>,

    /// Name of the Transit signing key for narinfo signatures.
    ///
    /// When both `cache_name` and `signing_key_name` are set, narinfo files
    /// are signed using the Ed25519 key stored in the Transit secrets engine.
    /// The key must exist and be accessible via the node's secrets configuration.
    ///
    /// Example: "nix-cache-signing-key"
    pub signing_key_name: Option<String>,

    /// Transit mount path for the signing key.
    ///
    /// Specifies the Transit secrets engine mount where the signing key is stored.
    /// If not specified, uses the default "nix-cache" mount.
    ///
    /// Example: "transit" or "nix-cache"
    #[serde(default = "default_nix_cache_transit_mount")]
    pub transit_mount: String,

    /// Enable CI substituter for Nix build workers.
    ///
    /// When enabled, NixBuildWorker uses the cluster's Nix binary cache
    /// as a substituter during builds. This requires `enabled` to be true
    /// (the gateway must be running to serve cache requests).
    ///
    /// The worker starts a local HTTP proxy that bridges Nix's HTTP requests
    /// to the Aspen cache gateway over Iroh QUIC.
    ///
    /// Default: true (enabled when nix_cache.enabled)
    #[serde(default = "default_enable_ci_substituter")]
    pub enable_ci_substituter: bool,
}

impl Default for NixCacheConfig {
    fn default() -> Self {
        Self {
            is_enabled: false,
            store_dir: default_nix_store_dir(),
            priority: default_nix_cache_priority(),
            want_mass_query: default_want_mass_query(),
            cache_name: None,
            signing_key_name: None,
            transit_mount: default_nix_cache_transit_mount(),
            enable_ci_substituter: default_enable_ci_substituter(),
        }
    }
}

pub(crate) fn default_nix_store_dir() -> PathBuf {
    PathBuf::from("/nix/store")
}

pub(crate) fn default_nix_cache_priority() -> u32 {
    30
}

pub(crate) fn default_want_mass_query() -> bool {
    true
}

pub(crate) fn default_nix_cache_transit_mount() -> String {
    "nix-cache".to_string()
}

pub(crate) fn default_enable_ci_substituter() -> bool {
    true
}
