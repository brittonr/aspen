//! Shared types and constants for the Aspen WASM plugin system.
//!
//! This crate defines the API boundary between native host code and WASM
//! guest plugins. Both sides depend on these types to ensure a stable
//! serialization contract.

use serde::Deserialize;
use serde::Serialize;

pub mod manifest;

pub use manifest::PluginInfo;
pub use manifest::PluginManifest;
pub use manifest::PluginPermissions;

/// Maximum priority value for WASM plugins.
pub const MAX_PLUGIN_PRIORITY: u32 = 999;

/// Minimum priority value for WASM plugins (ensures they run after native handlers).
pub const MIN_PLUGIN_PRIORITY: u32 = 900;

/// KV key prefix for plugin manifests in the cluster store.
pub const PLUGIN_KV_PREFIX: &str = "plugins/handlers/";

/// Maximum number of loaded WASM plugins per node.
pub const MAX_PLUGINS: u32 = 64;

/// Default fuel budget for a single plugin invocation.
pub const PLUGIN_DEFAULT_FUEL: u64 = 500_000_000;

/// Default memory limit for a single plugin instance (128 MB).
pub const PLUGIN_DEFAULT_MEMORY: u64 = 128 * 1024 * 1024;

/// Lifecycle state of a plugin instance.
///
/// Plugins transition through these states during their lifecycle:
/// `Loading` -> `Initializing` -> `Ready` -> `Stopping` -> `Stopped`
///
/// Plugins may also transition to `Degraded` (from `Ready`) or `Failed`
/// (from any state) based on health checks or runtime errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginState {
    /// Plugin is being loaded from storage.
    Loading,
    /// Plugin is initializing (calling guest init functions).
    Initializing,
    /// Plugin is ready to handle requests.
    Ready,
    /// Plugin is operational but degraded (e.g., slow responses, partial failures).
    Degraded,
    /// Plugin is gracefully shutting down.
    Stopping,
    /// Plugin has stopped and released all resources.
    Stopped,
    /// Plugin has failed and cannot process requests.
    Failed,
}

impl PluginState {
    /// Returns `true` if the plugin is in an active state that can handle requests.
    ///
    /// Active states are: `Initializing`, `Ready`, and `Degraded`.
    pub fn is_active(&self) -> bool {
        matches!(self, PluginState::Initializing | PluginState::Ready | PluginState::Degraded)
    }
}

/// Health status and metadata for a plugin instance.
///
/// Used for monitoring, observability, and health check endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHealth {
    /// Current lifecycle state of the plugin.
    pub state: PluginState,
    /// Optional human-readable status message.
    pub message: Option<String>,
    /// Timestamp of last health check in milliseconds since Unix epoch.
    pub last_check_ms: u64,
}

impl PluginHealth {
    /// Creates a healthy plugin status with state `Ready`.
    ///
    /// # Arguments
    /// * `msg` - Human-readable status message
    pub fn healthy(msg: impl Into<String>) -> Self {
        Self {
            state: PluginState::Ready,
            message: Some(msg.into()),
            last_check_ms: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()
                as u64,
        }
    }

    /// Creates a degraded plugin status with state `Degraded`.
    ///
    /// # Arguments
    /// * `msg` - Human-readable description of degradation
    pub fn degraded(msg: impl Into<String>) -> Self {
        Self {
            state: PluginState::Degraded,
            message: Some(msg.into()),
            last_check_ms: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()
                as u64,
        }
    }

    /// Returns `true` if the plugin is in a healthy state (`Ready`).
    pub fn is_healthy(&self) -> bool {
        self.state == PluginState::Ready
    }
}

// ---------------------------------------------------------------------------
// KV Batch Operations
// ---------------------------------------------------------------------------

/// A single operation in a KV batch write.
///
/// Batch operations are serialized as JSON and passed across the WASM boundary.
/// The host validates all keys against the plugin's namespace prefixes before
/// executing any operations.
///
/// Tiger Style: Validate all inputs before side effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KvBatchOp {
    /// Set a key to a value.
    Set { key: String, value: String },
    /// Delete a key.
    Delete { key: String },
}

// ---------------------------------------------------------------------------
// Timer / Scheduler
// ---------------------------------------------------------------------------

/// Configuration for a scheduled timer.
///
/// Timers are identified by name within a plugin. Registering a timer
/// with the same name as an existing one replaces it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerConfig {
    /// Unique name for this timer within the plugin.
    pub name: String,
    /// Interval in milliseconds for periodic timers, or delay for one-shot.
    pub interval_ms: u64,
    /// Whether this timer repeats. If false, fires once after `interval_ms`.
    pub repeating: bool,
}

/// Maximum number of active timers per plugin.
pub const MAX_TIMERS_PER_PLUGIN: usize = 16;

/// Minimum timer interval in milliseconds (1 second).
pub const MIN_TIMER_INTERVAL_MS: u64 = 1_000;

/// Maximum timer interval in milliseconds (24 hours).
pub const MAX_TIMER_INTERVAL_MS: u64 = 86_400_000;

// ---------------------------------------------------------------------------
// Hook Event Subscriptions
// ---------------------------------------------------------------------------

/// Maximum number of active hook subscriptions per plugin.
///
/// Tiger Style: Bounded resource allocation prevents a single plugin
/// from consuming unbounded memory with subscriptions.
pub const MAX_HOOK_SUBSCRIPTIONS_PER_PLUGIN: usize = 16;

/// Maximum length of a hook subscription pattern string.
pub const MAX_HOOK_PATTERN_LENGTH: usize = 256;
