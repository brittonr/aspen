//! Shared types and constants for the Aspen WASM plugin system.
//!
//! This crate defines the API boundary between native host code and WASM
//! guest plugins. Both sides depend on these types to ensure a stable
//! serialization contract.

use serde::Deserialize;
use serde::Serialize;

pub mod manifest;
pub mod resolve;

pub use manifest::PluginDependency;
pub use manifest::PluginInfo;
pub use manifest::PluginManifest;
pub use manifest::PluginPermissions;
pub use manifest::PluginSignatureInfo;

/// Maximum priority value for WASM plugins.
pub const MAX_PLUGIN_PRIORITY: u32 = 999;

/// Minimum priority value for WASM plugins (ensures they run after native handlers).
pub const MIN_PLUGIN_PRIORITY: u32 = 900;

/// KV key prefix for plugin manifests in the cluster store.
pub const PLUGIN_KV_PREFIX: &str = "plugins/handlers/";

/// Maximum number of loaded WASM plugins per node.
pub const MAX_PLUGINS: u32 = 64;

/// Current plugin API version.
///
/// Bump policy:
/// - Patch: bug fixes, new optional host functions
/// - Minor: new features plugins should adopt, new required guest exports
/// - Major: breaking changes to host function signatures or encoding
///
/// See `docs/HOST_ABI.md` for the version changelog.
pub const PLUGIN_API_VERSION: &str = "0.3.0";

/// Maximum dependencies per plugin.
pub const MAX_PLUGIN_DEPENDENCIES: usize = 16;

/// Maximum description length in bytes.
pub const MAX_PLUGIN_DESCRIPTION_SIZE: usize = 1024;

/// Maximum tags per plugin.
pub const MAX_PLUGIN_TAGS: usize = 16;

/// Maximum tag length in bytes.
pub const MAX_PLUGIN_TAG_SIZE: usize = 64;

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
// Plugin Metrics
// ---------------------------------------------------------------------------

/// Per-plugin runtime metrics tracked by the host.
///
/// All fields use atomic operations for lock-free concurrent access.
/// Metrics are collected during request dispatch and exposed via the
/// plugin health/status API.
///
/// Tiger Style: Lock-free atomics prevent metrics collection from
/// blocking request processing.
#[derive(Debug)]
pub struct PluginMetrics {
    /// Total requests dispatched to this plugin.
    pub request_count: std::sync::atomic::AtomicU64,
    /// Requests that completed successfully.
    pub success_count: std::sync::atomic::AtomicU64,
    /// Requests that returned an error.
    pub error_count: std::sync::atomic::AtomicU64,
    /// Cumulative execution time in nanoseconds.
    pub total_duration_ns: std::sync::atomic::AtomicU64,
    /// Maximum single-request execution time in nanoseconds.
    pub max_duration_ns: std::sync::atomic::AtomicU64,
    /// Epoch milliseconds of the most recent request.
    pub last_request_epoch_ms: std::sync::atomic::AtomicU64,
    /// Number of requests currently in-flight.
    pub active_requests: std::sync::atomic::AtomicU64,
}

impl Default for PluginMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginMetrics {
    /// Create zeroed metrics.
    pub fn new() -> Self {
        Self {
            request_count: std::sync::atomic::AtomicU64::new(0),
            success_count: std::sync::atomic::AtomicU64::new(0),
            error_count: std::sync::atomic::AtomicU64::new(0),
            total_duration_ns: std::sync::atomic::AtomicU64::new(0),
            max_duration_ns: std::sync::atomic::AtomicU64::new(0),
            last_request_epoch_ms: std::sync::atomic::AtomicU64::new(0),
            active_requests: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Record a completed request.
    pub fn record(&self, duration_ns: u64, success: bool) {
        use std::sync::atomic::Ordering::Relaxed;
        self.request_count.fetch_add(1, Relaxed);
        if success {
            self.success_count.fetch_add(1, Relaxed);
        } else {
            self.error_count.fetch_add(1, Relaxed);
        }
        self.total_duration_ns.fetch_add(duration_ns, Relaxed);
        // Update max using CAS loop
        let mut current = self.max_duration_ns.load(Relaxed);
        while duration_ns > current {
            match self.max_duration_ns.compare_exchange_weak(current, duration_ns, Relaxed, Relaxed) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
        let now_ms =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
        self.last_request_epoch_ms.store(now_ms, Relaxed);
    }

    /// Snapshot current metrics as a serializable value.
    pub fn snapshot(&self) -> PluginMetricsSnapshot {
        use std::sync::atomic::Ordering::Relaxed;
        PluginMetricsSnapshot {
            request_count: self.request_count.load(Relaxed),
            success_count: self.success_count.load(Relaxed),
            error_count: self.error_count.load(Relaxed),
            total_duration_ns: self.total_duration_ns.load(Relaxed),
            max_duration_ns: self.max_duration_ns.load(Relaxed),
            last_request_epoch_ms: self.last_request_epoch_ms.load(Relaxed),
            active_requests: self.active_requests.load(Relaxed),
            avg_duration_ns: {
                let count = self.request_count.load(Relaxed);
                self.total_duration_ns.load(Relaxed).checked_div(count).unwrap_or(0)
            },
        }
    }
}

/// Serializable snapshot of plugin metrics at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetricsSnapshot {
    /// Total requests dispatched to this plugin.
    pub request_count: u64,
    /// Requests that completed successfully.
    pub success_count: u64,
    /// Requests that returned an error.
    pub error_count: u64,
    /// Cumulative execution time in nanoseconds.
    pub total_duration_ns: u64,
    /// Maximum single-request execution time in nanoseconds.
    pub max_duration_ns: u64,
    /// Epoch milliseconds of the most recent request.
    pub last_request_epoch_ms: u64,
    /// Number of requests currently in-flight.
    pub active_requests: u64,
    /// Average execution time in nanoseconds (derived).
    pub avg_duration_ns: u64,
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

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering::Relaxed;

    use super::*;

    #[test]
    fn metrics_new_is_zeroed() {
        let m = PluginMetrics::new();
        assert_eq!(m.request_count.load(Relaxed), 0);
        assert_eq!(m.success_count.load(Relaxed), 0);
        assert_eq!(m.error_count.load(Relaxed), 0);
        assert_eq!(m.total_duration_ns.load(Relaxed), 0);
        assert_eq!(m.max_duration_ns.load(Relaxed), 0);
        assert_eq!(m.active_requests.load(Relaxed), 0);
    }

    #[test]
    fn metrics_record_success() {
        let m = PluginMetrics::new();
        m.record(1_000_000, true); // 1ms
        let snap = m.snapshot();
        assert_eq!(snap.request_count, 1);
        assert_eq!(snap.success_count, 1);
        assert_eq!(snap.error_count, 0);
        assert_eq!(snap.total_duration_ns, 1_000_000);
        assert_eq!(snap.max_duration_ns, 1_000_000);
        assert_eq!(snap.avg_duration_ns, 1_000_000);
        assert!(snap.last_request_epoch_ms > 0);
    }

    #[test]
    fn metrics_record_error() {
        let m = PluginMetrics::new();
        m.record(500_000, false);
        let snap = m.snapshot();
        assert_eq!(snap.request_count, 1);
        assert_eq!(snap.success_count, 0);
        assert_eq!(snap.error_count, 1);
        assert_eq!(snap.total_duration_ns, 500_000);
    }

    #[test]
    fn metrics_max_duration_tracks_peak() {
        let m = PluginMetrics::new();
        m.record(100, true);
        m.record(500, true);
        m.record(300, true);
        let snap = m.snapshot();
        assert_eq!(snap.max_duration_ns, 500);
        assert_eq!(snap.request_count, 3);
        assert_eq!(snap.avg_duration_ns, 300); // (100+500+300)/3 = 300
    }

    #[test]
    fn metrics_active_requests_manual() {
        let m = PluginMetrics::new();
        m.active_requests.fetch_add(1, Relaxed);
        m.active_requests.fetch_add(1, Relaxed);
        assert_eq!(m.snapshot().active_requests, 2);
        m.active_requests.fetch_sub(1, Relaxed);
        assert_eq!(m.snapshot().active_requests, 1);
    }

    #[test]
    fn metrics_snapshot_serializes_to_json() {
        let m = PluginMetrics::new();
        m.record(42_000, true);
        let snap = m.snapshot();
        let json = serde_json::to_string(&snap).expect("serialize");
        assert!(json.contains("\"request_count\":1"));
        assert!(json.contains("\"success_count\":1"));
        assert!(json.contains("\"avg_duration_ns\":42000"));
        // Roundtrip
        let deser: PluginMetricsSnapshot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deser.request_count, 1);
        assert_eq!(deser.total_duration_ns, 42_000);
    }

    #[test]
    fn metrics_avg_zero_requests() {
        let m = PluginMetrics::new();
        let snap = m.snapshot();
        assert_eq!(snap.avg_duration_ns, 0); // No division by zero
    }

    #[test]
    fn api_version_is_semver() {
        let ver = semver::Version::parse(PLUGIN_API_VERSION);
        assert!(ver.is_ok(), "PLUGIN_API_VERSION must be valid semver: {PLUGIN_API_VERSION}");
    }
}
