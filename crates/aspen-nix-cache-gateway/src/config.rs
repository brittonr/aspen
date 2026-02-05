//! Configuration for the Nix cache gateway.

use std::sync::Arc;

use ed25519_dalek::SigningKey;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::constants::DEFAULT_CACHE_PRIORITY;
use crate::constants::DEFAULT_STORE_DIR;
use crate::constants::MAX_GATEWAY_CONNECTIONS;
use crate::constants::MAX_STREAMS_PER_CONNECTION;
use crate::constants::STREAM_CHUNK_SIZE;

/// Configuration for the Nix binary cache HTTP/3 gateway.
///
/// All fields have sensible defaults via [`Default`].
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct NixCacheGatewayConfig {
    /// Nix store directory (default: "/nix/store").
    pub store_dir: String,

    /// Cache priority for Nix substituters (lower = higher priority).
    ///
    /// Default: 30 (higher priority than cache.nixos.org which uses 40).
    pub priority: u32,

    /// Enable mass query mode for batch narinfo lookups.
    pub want_mass_query: bool,

    /// Maximum concurrent HTTP/3 connections.
    pub max_connections: u32,

    /// Maximum concurrent streams per connection.
    pub max_streams_per_connection: u32,

    /// Chunk size for NAR streaming (bytes).
    pub nar_chunk_size_bytes: u32,

    /// Cache name for narinfo signatures (e.g., "aspen-cache-1").
    ///
    /// If None, narinfo files are not signed.
    pub cache_name: Option<String>,

    /// Ed25519 signing key for narinfo signatures.
    ///
    /// Loaded from SOPS secrets at startup. If None, signatures are disabled.
    #[serde(skip)]
    pub signing_key: Option<Arc<SigningKey>>,
}

impl Default for NixCacheGatewayConfig {
    fn default() -> Self {
        Self {
            store_dir: DEFAULT_STORE_DIR.to_string(),
            priority: DEFAULT_CACHE_PRIORITY,
            want_mass_query: true,
            max_connections: MAX_GATEWAY_CONNECTIONS,
            max_streams_per_connection: MAX_STREAMS_PER_CONNECTION,
            nar_chunk_size_bytes: STREAM_CHUNK_SIZE as u32,
            cache_name: None,
            signing_key: None,
        }
    }
}

impl NixCacheGatewayConfig {
    /// Create a new config with signing enabled.
    pub fn with_signing(mut self, cache_name: String, signing_key: SigningKey) -> Self {
        self.cache_name = Some(cache_name);
        self.signing_key = Some(Arc::new(signing_key));
        self
    }

    /// Check if signing is enabled.
    pub fn signing_enabled(&self) -> bool {
        self.cache_name.is_some() && self.signing_key.is_some()
    }
}
