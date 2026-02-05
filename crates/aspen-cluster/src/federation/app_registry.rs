//! Application registry for federation discovery.
//!
//! This module provides a formal [`AppManifest`] type and [`AppRegistry`] service
//! for managing applications installed on an Aspen cluster. This replaces the
//! previous hardcoded `capabilities: Vec<String>` approach with rich metadata.
//!
//! # Overview
//!
//! Applications running on an Aspen cluster can register their manifests,
//! which are then announced to the federation via gossip and DHT. This enables:
//!
//! - **Discovery by app**: Find clusters running a specific application
//! - **Capability queries**: Find clusters with specific fine-grained features
//! - **Version awareness**: Ensure protocol compatibility between clusters
//! - **Dynamic registration**: Register/unregister apps at runtime
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                          AppRegistry                             │
//! ├──────────────────────────────────────────────────────────────────┤
//! │                                                                   │
//! │   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
//! │   │ AppManifest │  │ AppManifest │  │ AppManifest │              │
//! │   │ (forge)     │  │ (ci)        │  │ (snix)      │              │
//! │   │ v1.0.0      │  │ v2.1.0      │  │ v0.5.0      │              │
//! │   │ [git,issues]│  │ [build,test]│  │ [store]     │              │
//! │   └─────────────┘  └─────────────┘  └─────────────┘              │
//! │          │                │                │                      │
//! │          └────────────────┼────────────────┘                      │
//! │                           │                                       │
//! │                           ▼                                       │
//! │              ┌──────────────────────────┐                        │
//! │              │  to_announcement_list()  │                        │
//! │              └─────────────┬────────────┘                        │
//! │                            │                                      │
//! └────────────────────────────┼──────────────────────────────────────┘
//!                              │
//!                              ▼
//!              ┌─────────────────────────────────┐
//!              │  FederationGossipService        │
//!              │  FederationDiscoveryService     │
//!              │  (announces apps to federation) │
//!              └─────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ## Creating and Registering Applications
//!
//! ```
//! use aspen_cluster::federation::{AppManifest, AppRegistry};
//!
//! // Create a manifest for your application
//! let manifest = AppManifest::new("forge", "1.0.0")
//!     .with_name("Aspen Forge")
//!     .with_capabilities(vec!["git", "issues", "patches", "discussions"]);
//!
//! // Register with the cluster's app registry
//! let registry = AppRegistry::new();
//! assert!(registry.register(manifest));
//!
//! // Check registration
//! assert!(registry.has_app("forge"));
//! ```
//!
//! ## Querying Applications
//!
//! ```
//! use aspen_cluster::federation::{AppManifest, AppRegistry};
//!
//! let registry = AppRegistry::new();
//! registry.register(AppManifest::new("forge", "1.0.0")
//!     .with_capabilities(vec!["git", "issues"]));
//! registry.register(AppManifest::new("ci", "1.0.0")
//!     .with_capabilities(vec!["build", "issues"]));
//!
//! // Query by app ID
//! if let Some(forge) = registry.get_app("forge") {
//!     println!("Forge version: {}", forge.version);
//! }
//!
//! // Find apps with specific capability
//! let apps_with_issues = registry.find_apps_with_capability("issues");
//! assert_eq!(apps_with_issues.len(), 2);  // Both forge and ci have "issues"
//!
//! // Get all capabilities (deduplicated)
//! let all_caps = registry.all_capabilities();
//! assert!(all_caps.contains(&"git".to_string()));
//! ```
//!
//! ## Shared Registry
//!
//! For multi-threaded access, use the [`SharedAppRegistry`] type:
//!
//! ```
//! use aspen_cluster::federation::{shared_registry, AppManifest};
//!
//! let registry = shared_registry();
//! registry.register(AppManifest::new("forge", "1.0.0"));
//!
//! // Clone Arc for use in other tasks
//! let registry_clone = registry.clone();
//! ```
//!
//! # AppManifest Fields
//!
//! | Field | Type | Purpose |
//! |-------|------|---------|
//! | `app_id` | String | Unique identifier (e.g., "forge", "ci") |
//! | `version` | String | Semantic version for compatibility |
//! | `name` | String | Human-readable display name |
//! | `capabilities` | `Vec<String>` | Fine-grained feature flags |
//! | `public_key` | `Vec<u8>` | Optional Ed25519 key for app-level signing |
//!
//! # Naming Conventions
//!
//! - **App IDs**: Lowercase, alphanumeric with hyphens (e.g., "forge", "aspen-ci")
//! - **Versions**: Semantic versioning (e.g., "1.0.0", "2.1.0-beta")
//! - **Capabilities**: Lowercase, descriptive features (e.g., "git", "issues", "patches")
//!
//! # Tiger Style Compliance
//!
//! All inputs are truncated to prevent resource exhaustion:
//!
//! | Resource | Limit | Constant |
//! |----------|-------|----------|
//! | Apps per cluster | 32 | [`MAX_APPS_PER_CLUSTER`] |
//! | Capabilities per app | 16 | [`MAX_CAPABILITIES_PER_APP`] |
//! | App ID length | 64 chars | [`MAX_APP_ID_LENGTH`] |
//! | App name length | 128 chars | [`MAX_APP_NAME_LENGTH`] |
//! | Capability length | 64 chars | [`MAX_CAPABILITY_LENGTH`] |
//!
//! Exceeding these limits results in **silent truncation**, not errors. This
//! ensures robust handling of potentially malicious or malformed input.
//!
//! # Thread Safety
//!
//! [`AppRegistry`] uses `parking_lot::RwLock` internally, making all methods
//! thread-safe. Multiple readers can access concurrently; writers get exclusive access.
//!
//! # Integration with Federation
//!
//! The registry integrates with federation services:
//!
//! 1. **Gossip**: `FederationGossipService::with_app_registry()` automatically includes registered
//!    apps in `ClusterOnline` messages
//!
//! 2. **Discovery**: Apps are included in `ClusterAnnouncement` via
//!    `registry.to_announcement_list()`
//!
//! 3. **Remote queries**: Use `DiscoveredCluster::has_app()` and `DiscoveredCluster::get_app()` to
//!    check remote clusters
//!
//! # Version Compatibility
//!
//! The registry does not enforce version compatibility - that's left to
//! applications. However, version information is preserved for applications
//! to implement their own compatibility checks:
//!
//! ```ignore
//! if let Some(remote_forge) = remote_cluster.get_app("forge") {
//!     if !is_compatible(&local_forge.version, &remote_forge.version) {
//!         warn!("Forge version mismatch: {} vs {}", local, remote);
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::info;
use tracing::warn;

// ============================================================================
// Constants (Tiger Style: Fixed limits)
// ============================================================================

/// Maximum number of applications per cluster.
pub const MAX_APPS_PER_CLUSTER: usize = 32;

/// Maximum number of capabilities per application.
pub const MAX_CAPABILITIES_PER_APP: usize = 16;

/// Maximum length of an application ID.
pub const MAX_APP_ID_LENGTH: usize = 64;

/// Maximum length of an application name.
pub const MAX_APP_NAME_LENGTH: usize = 128;

/// Maximum length of a capability string.
pub const MAX_CAPABILITY_LENGTH: usize = 64;

// ============================================================================
// AppManifest
// ============================================================================

/// Application manifest for federation discovery.
///
/// Describes an application installed on a cluster, enabling other clusters
/// to discover and interact with it.
///
/// # Fields
///
/// - `app_id`: Unique identifier (e.g., "forge", "ci", "snix")
/// - `version`: Semantic version string
/// - `name`: Human-readable display name
/// - `capabilities`: Fine-grained features the app provides
/// - `public_key`: Optional Ed25519 key for app-level signing
///
/// # Example
///
/// ```
/// use aspen_cluster::federation::AppManifest;
///
/// let manifest = AppManifest::new("forge", "1.0.0")
///     .with_name("Aspen Forge")
///     .with_capabilities(vec!["git", "issues", "patches", "discussions"]);
///
/// assert!(manifest.has_capability("git"));
/// assert!(!manifest.has_capability("unknown"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppManifest {
    /// Unique application identifier.
    ///
    /// Should be lowercase, alphanumeric with hyphens (e.g., "forge", "aspen-ci").
    /// Tiger Style: Truncated to MAX_APP_ID_LENGTH characters.
    pub app_id: String,

    /// Semantic version string (e.g., "1.0.0", "2.1.0-beta").
    ///
    /// Used for protocol compatibility checks between clusters.
    pub version: String,

    /// Human-readable application name.
    ///
    /// Tiger Style: Truncated to MAX_APP_NAME_LENGTH characters.
    pub name: String,

    /// Capabilities this application provides.
    ///
    /// Fine-grained feature flags that can be queried during discovery.
    /// For example, Forge might have: ["git", "issues", "patches", "discussions"].
    ///
    /// Tiger Style: Limited to MAX_CAPABILITIES_PER_APP entries.
    pub capabilities: Vec<String>,

    /// Optional Ed25519 public key for app-level signing.
    ///
    /// When present, this key can be used to verify signatures on
    /// app-specific operations (separate from cluster identity).
    /// Stored as `Vec<u8>` for better postcard serialization compatibility.
    /// Empty vector means no key.
    #[serde(default)]
    pub public_key: Vec<u8>,
}

impl AppManifest {
    /// Create a new application manifest.
    ///
    /// # Arguments
    ///
    /// * `app_id` - Unique application identifier
    /// * `version` - Semantic version string
    ///
    /// # Tiger Style
    ///
    /// - `app_id` is truncated to MAX_APP_ID_LENGTH
    pub fn new(app_id: impl Into<String>, version: impl Into<String>) -> Self {
        let mut app_id = app_id.into();
        app_id.truncate(MAX_APP_ID_LENGTH);

        Self {
            app_id,
            version: version.into(),
            name: String::new(),
            capabilities: Vec::new(),
            public_key: Vec::new(),
        }
    }

    /// Set the human-readable name (builder pattern).
    ///
    /// Tiger Style: Truncated to MAX_APP_NAME_LENGTH.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        let mut name = name.into();
        name.truncate(MAX_APP_NAME_LENGTH);
        self.name = name;
        self
    }

    /// Set the capabilities (builder pattern).
    ///
    /// Tiger Style: Limited to MAX_CAPABILITIES_PER_APP, each truncated to MAX_CAPABILITY_LENGTH.
    pub fn with_capabilities(mut self, capabilities: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.capabilities = capabilities
            .into_iter()
            .take(MAX_CAPABILITIES_PER_APP)
            .map(|c| {
                let mut s = c.into();
                s.truncate(MAX_CAPABILITY_LENGTH);
                s
            })
            .collect();
        self
    }

    /// Set the public key (builder pattern).
    pub fn with_public_key(mut self, key: [u8; 32]) -> Self {
        self.public_key = key.to_vec();
        self
    }

    /// Get the public key if present.
    pub fn get_public_key(&self) -> Option<[u8; 32]> {
        if self.public_key.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&self.public_key);
            Some(key)
        } else {
            None
        }
    }

    /// Check if this app has a specific capability.
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }

    /// Get a short display string.
    pub fn short(&self) -> String {
        format!("{}@{}", self.app_id, self.version)
    }
}

impl Default for AppManifest {
    fn default() -> Self {
        Self::new("unknown", "0.0.0")
    }
}

// ============================================================================
// AppRegistry
// ============================================================================

/// Registry of applications installed on this cluster.
///
/// The `AppRegistry` manages `AppManifest` entries and provides methods for
/// registration, lookup, and federation discovery integration.
///
/// # Thread Safety
///
/// All methods are thread-safe and can be called from multiple tasks concurrently.
///
/// # Example
///
/// ```ignore
/// use aspen_cluster::federation::{AppRegistry, AppManifest};
///
/// let registry = AppRegistry::new();
///
/// // Register Forge
/// let forge = AppManifest::new("forge", "1.0.0")
///     .with_capabilities(vec!["git", "issues"]);
/// registry.register(forge);
///
/// // Check registration
/// assert!(registry.has_app("forge"));
/// assert!(!registry.has_app("unknown"));
///
/// // Query capabilities
/// let apps_with_git = registry.find_apps_with_capability("git");
/// assert_eq!(apps_with_git.len(), 1);
/// ```
#[derive(Debug, Default)]
pub struct AppRegistry {
    /// Registered applications (app_id -> manifest).
    apps: RwLock<HashMap<String, AppManifest>>,
}

impl AppRegistry {
    /// Create a new empty app registry.
    pub fn new() -> Self {
        Self {
            apps: RwLock::new(HashMap::new()),
        }
    }

    /// Register an application.
    ///
    /// If an application with the same ID is already registered, it is replaced.
    ///
    /// # Tiger Style
    ///
    /// - Returns `false` if MAX_APPS_PER_CLUSTER would be exceeded (and doesn't register)
    /// - Returns `true` if registration succeeded
    pub fn register(&self, manifest: AppManifest) -> bool {
        let mut apps = self.apps.write();

        // Check if already registered (update case)
        if apps.contains_key(&manifest.app_id) {
            info!(
                app_id = %manifest.app_id,
                version = %manifest.version,
                "updating registered application"
            );
            apps.insert(manifest.app_id.clone(), manifest);
            return true;
        }

        // Check capacity
        if apps.len() >= MAX_APPS_PER_CLUSTER {
            warn!(
                app_id = %manifest.app_id,
                max = MAX_APPS_PER_CLUSTER,
                "cannot register application: registry at capacity"
            );
            return false;
        }

        info!(
            app_id = %manifest.app_id,
            version = %manifest.version,
            capabilities = ?manifest.capabilities,
            "registered application"
        );
        apps.insert(manifest.app_id.clone(), manifest);
        true
    }

    /// Unregister an application.
    ///
    /// Returns `true` if the application was registered and removed.
    pub fn unregister(&self, app_id: &str) -> bool {
        let mut apps = self.apps.write();
        if apps.remove(app_id).is_some() {
            info!(app_id = %app_id, "unregistered application");
            true
        } else {
            debug!(app_id = %app_id, "application not registered");
            false
        }
    }

    /// Check if an application is registered.
    pub fn has_app(&self, app_id: &str) -> bool {
        self.apps.read().contains_key(app_id)
    }

    /// Get an application manifest by ID.
    pub fn get_app(&self, app_id: &str) -> Option<AppManifest> {
        self.apps.read().get(app_id).cloned()
    }

    /// List all registered applications.
    pub fn list_apps(&self) -> Vec<AppManifest> {
        self.apps.read().values().cloned().collect()
    }

    /// Get all capabilities across all registered apps.
    ///
    /// Returns a deduplicated list of all capabilities.
    pub fn all_capabilities(&self) -> Vec<String> {
        let apps = self.apps.read();
        let mut capabilities: Vec<String> = apps.values().flat_map(|m| m.capabilities.iter().cloned()).collect();
        capabilities.sort();
        capabilities.dedup();
        capabilities
    }

    /// Find apps with a specific capability.
    pub fn find_apps_with_capability(&self, capability: &str) -> Vec<AppManifest> {
        self.apps.read().values().filter(|m| m.has_capability(capability)).cloned().collect()
    }

    /// Get the number of registered applications.
    pub fn len(&self) -> usize {
        self.apps.read().len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.apps.read().is_empty()
    }

    /// Convert to a list suitable for announcements.
    ///
    /// This is used when building `ClusterAnnouncement` messages.
    pub fn to_announcement_list(&self) -> Vec<AppManifest> {
        self.list_apps()
    }

    /// Create from an announcement list.
    ///
    /// This is used when processing received `ClusterAnnouncement` messages.
    pub fn from_announcement_list(apps: Vec<AppManifest>) -> Self {
        let registry = Self::new();
        for app in apps.into_iter().take(MAX_APPS_PER_CLUSTER) {
            registry.register(app);
        }
        registry
    }
}

impl Clone for AppRegistry {
    fn clone(&self) -> Self {
        Self {
            apps: RwLock::new(self.apps.read().clone()),
        }
    }
}

// ============================================================================
// Thread-safe Arc wrapper
// ============================================================================

/// Thread-safe shared app registry.
pub type SharedAppRegistry = Arc<AppRegistry>;

/// Create a new shared app registry.
pub fn shared_registry() -> SharedAppRegistry {
    Arc::new(AppRegistry::new())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_manifest_creation() {
        let manifest = AppManifest::new("forge", "1.0.0")
            .with_name("Aspen Forge")
            .with_capabilities(vec!["git", "issues", "patches"]);

        assert_eq!(manifest.app_id, "forge");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.name, "Aspen Forge");
        assert_eq!(manifest.capabilities.len(), 3);
        assert!(manifest.has_capability("git"));
        assert!(!manifest.has_capability("unknown"));
    }

    #[test]
    fn test_app_manifest_tiger_style_truncation() {
        let long_id = "a".repeat(100);
        let long_name = "b".repeat(200);
        let many_caps: Vec<String> = (0..50).map(|i| format!("cap{i}")).collect();

        let manifest = AppManifest::new(long_id, "1.0.0").with_name(long_name).with_capabilities(many_caps);

        assert_eq!(manifest.app_id.len(), MAX_APP_ID_LENGTH);
        assert_eq!(manifest.name.len(), MAX_APP_NAME_LENGTH);
        assert_eq!(manifest.capabilities.len(), MAX_CAPABILITIES_PER_APP);
    }

    #[test]
    fn test_app_manifest_serialization() {
        let manifest = AppManifest::new("forge", "1.0.0").with_name("Forge").with_capabilities(vec!["git"]);

        let bytes = postcard::to_allocvec(&manifest).unwrap();
        let restored: AppManifest = postcard::from_bytes(&bytes).unwrap();

        assert_eq!(manifest, restored);
    }

    #[test]
    fn test_app_registry_register() {
        let registry = AppRegistry::new();

        let forge = AppManifest::new("forge", "1.0.0");
        assert!(registry.register(forge));
        assert!(registry.has_app("forge"));
        assert!(!registry.has_app("unknown"));
    }

    #[test]
    fn test_app_registry_update() {
        let registry = AppRegistry::new();

        let forge_v1 = AppManifest::new("forge", "1.0.0");
        let forge_v2 = AppManifest::new("forge", "2.0.0");

        assert!(registry.register(forge_v1));
        assert!(registry.register(forge_v2));

        let app = registry.get_app("forge").unwrap();
        assert_eq!(app.version, "2.0.0");
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_app_registry_unregister() {
        let registry = AppRegistry::new();

        let forge = AppManifest::new("forge", "1.0.0");
        registry.register(forge);
        assert!(registry.has_app("forge"));

        assert!(registry.unregister("forge"));
        assert!(!registry.has_app("forge"));

        assert!(!registry.unregister("forge")); // Already removed
    }

    #[test]
    fn test_app_registry_capacity_limit() {
        let registry = AppRegistry::new();

        // Fill to capacity
        for i in 0..MAX_APPS_PER_CLUSTER {
            let manifest = AppManifest::new(format!("app{i}"), "1.0.0");
            assert!(registry.register(manifest));
        }

        assert_eq!(registry.len(), MAX_APPS_PER_CLUSTER);

        // Next registration should fail
        let overflow = AppManifest::new("overflow", "1.0.0");
        assert!(!registry.register(overflow));
        assert!(!registry.has_app("overflow"));
    }

    #[test]
    fn test_app_registry_all_capabilities() {
        let registry = AppRegistry::new();

        let forge = AppManifest::new("forge", "1.0.0").with_capabilities(vec!["git", "issues"]);
        let ci = AppManifest::new("ci", "1.0.0").with_capabilities(vec!["build", "issues"]);

        registry.register(forge);
        registry.register(ci);

        let caps = registry.all_capabilities();
        assert_eq!(caps.len(), 3); // git, issues (deduped), build
        assert!(caps.contains(&"git".to_string()));
        assert!(caps.contains(&"issues".to_string()));
        assert!(caps.contains(&"build".to_string()));
    }

    #[test]
    fn test_app_registry_find_by_capability() {
        let registry = AppRegistry::new();

        let forge = AppManifest::new("forge", "1.0.0").with_capabilities(vec!["git", "issues"]);
        let ci = AppManifest::new("ci", "1.0.0").with_capabilities(vec!["build"]);

        registry.register(forge);
        registry.register(ci);

        let git_apps = registry.find_apps_with_capability("git");
        assert_eq!(git_apps.len(), 1);
        assert_eq!(git_apps[0].app_id, "forge");

        let build_apps = registry.find_apps_with_capability("build");
        assert_eq!(build_apps.len(), 1);
        assert_eq!(build_apps[0].app_id, "ci");
    }

    #[test]
    fn test_app_registry_clone() {
        let registry = AppRegistry::new();
        registry.register(AppManifest::new("forge", "1.0.0"));

        let cloned = registry.clone();
        assert!(cloned.has_app("forge"));

        // Modifications to clone don't affect original
        cloned.register(AppManifest::new("ci", "1.0.0"));
        assert!(!registry.has_app("ci"));
        assert!(cloned.has_app("ci"));
    }

    #[test]
    fn test_announcement_roundtrip() {
        let registry = AppRegistry::new();
        registry.register(AppManifest::new("forge", "1.0.0").with_capabilities(vec!["git"]));
        registry.register(AppManifest::new("ci", "1.0.0").with_capabilities(vec!["build"]));

        let list = registry.to_announcement_list();
        let restored = AppRegistry::from_announcement_list(list);

        assert_eq!(restored.len(), 2);
        assert!(restored.has_app("forge"));
        assert!(restored.has_app("ci"));
    }
}
