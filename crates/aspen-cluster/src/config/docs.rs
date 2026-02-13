//! iroh-docs configuration for real-time KV synchronization.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// iroh-docs configuration for real-time KV synchronization.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DocsConfig {
    /// Enable iroh-docs integration for real-time KV synchronization.
    ///
    /// When enabled, committed KV operations are exported to an iroh-docs
    /// namespace for CRDT-based replication to clients.
    ///
    /// Default: true (docs enabled).
    #[serde(default = "default_docs_enabled")]
    pub enabled: bool,

    /// Enable periodic full sync from state machine to docs.
    ///
    /// When enabled, the docs exporter periodically scans the entire
    /// state machine to detect and correct any drift between the
    /// committed state and the docs namespace.
    ///
    /// Default: true (background sync enabled).
    #[serde(default = "default_enable_background_sync")]
    pub enable_background_sync: bool,

    /// Interval for background sync in seconds.
    ///
    /// Only relevant when enable_background_sync is true.
    ///
    /// Default: 60 seconds.
    #[serde(default = "default_background_sync_interval_secs")]
    pub background_sync_interval_secs: u64,

    /// Use in-memory storage for iroh-docs instead of persistent storage.
    ///
    /// When true, the docs store uses in-memory storage (data lost on restart).
    /// When false, uses persistent storage in data_dir/docs/.
    ///
    /// Default: false (persistent storage).
    #[serde(default)]
    pub in_memory: bool,

    /// Hex-encoded namespace secret for the docs namespace.
    ///
    /// If not provided, a new namespace is created on first start and the
    /// secret is persisted to data_dir/docs/namespace_secret.
    ///
    /// 64 hex characters = 32 bytes.
    pub namespace_secret: Option<String>,

    /// Hex-encoded author secret for signing docs entries.
    ///
    /// If not provided, a new author is created on first start and the
    /// secret is persisted to data_dir/docs/author_secret.
    ///
    /// 64 hex characters = 32 bytes.
    pub author_secret: Option<String>,
}

impl Default for DocsConfig {
    fn default() -> Self {
        Self {
            enabled: default_docs_enabled(),
            enable_background_sync: default_enable_background_sync(),
            background_sync_interval_secs: default_background_sync_interval_secs(),
            in_memory: false,
            namespace_secret: None,
            author_secret: None,
        }
    }
}

pub(crate) fn default_docs_enabled() -> bool {
    true
}

pub(crate) fn default_enable_background_sync() -> bool {
    true
}

pub(crate) fn default_background_sync_interval_secs() -> u64 {
    60
}
