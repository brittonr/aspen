//! Key origin tracking for cluster-to-cluster conflict resolution.
//!
//! Each key can have an origin that tracks which cluster it came from
//! and its priority. This enables priority-based conflict resolution
//! when the same key is written by multiple peer clusters.
//!
//! # Storage
//!
//! Origins are stored in SQLite alongside regular KV data with the prefix
//! `__aspen_origin:` followed by the original key. This keeps origin metadata
//! in the same transactional boundary as the data it describes.
//!
//! # Priority Model
//!
//! Lower priority numbers win conflicts:
//! - Priority 0: Local cluster (highest priority)
//! - Priority 1+: Remote clusters (lower priority, higher number)

use aspen_core::hlc::SerializableTimestamp;
use serde::Deserialize;
use serde::Serialize;

/// Prefix for origin metadata keys in the KV store.
pub const ORIGIN_KEY_PREFIX: &str = "__aspen_origin:";

/// Tracks the origin of a key for conflict resolution.
///
/// When a key is imported from a peer cluster, its origin is recorded
/// to enable priority-based conflict resolution on subsequent updates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyOrigin {
    /// The cluster ID that owns this key.
    pub cluster_id: String,
    /// Priority of this cluster (lower = higher priority, 0 = local).
    pub priority: u32,
    /// HLC timestamp when this key was last updated (for ordering and audit).
    pub hlc_timestamp: SerializableTimestamp,
    /// Raft log index when imported (0 for local writes).
    pub log_index: u64,
}

impl KeyOrigin {
    /// Create a new origin for a local cluster write.
    pub fn local(cluster_id: impl Into<String>, hlc: &aspen_core::hlc::HLC) -> Self {
        Self {
            cluster_id: cluster_id.into(),
            priority: 0,
            hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
            log_index: 0,
        }
    }

    /// Create a new origin for a remote cluster import.
    pub fn remote(cluster_id: impl Into<String>, priority: u32, log_index: u64, hlc: &aspen_core::hlc::HLC) -> Self {
        Self {
            cluster_id: cluster_id.into(),
            priority,
            hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
            log_index,
        }
    }

    /// Check if a new write with the given priority should override this origin.
    ///
    /// Lower priority numbers have higher precedence, so a new write
    /// should override if its priority is strictly lower.
    pub fn should_replace(&self, new_priority: u32) -> bool {
        new_priority < self.priority
    }

    /// Check if this origin represents a local cluster.
    pub fn is_local(&self) -> bool {
        self.priority == 0
    }

    /// Generate the storage key for this origin.
    pub fn storage_key(key: &str) -> String {
        format!("{}{}", ORIGIN_KEY_PREFIX, key)
    }

    /// Parse the original key from a storage key.
    pub fn parse_key(storage_key: &str) -> Option<&str> {
        storage_key.strip_prefix(ORIGIN_KEY_PREFIX)
    }

    /// Serialize to JSON bytes for storage.
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Deserialize from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspen_core::hlc::create_hlc;

    #[test]
    fn test_local_origin() {
        let hlc = create_hlc("test-node");
        let origin = KeyOrigin::local("cluster-a", &hlc);
        assert_eq!(origin.cluster_id, "cluster-a");
        assert_eq!(origin.priority, 0);
        assert!(origin.is_local());
    }

    #[test]
    fn test_remote_origin() {
        let hlc = create_hlc("test-node");
        let origin = KeyOrigin::remote("cluster-b", 5, 100, &hlc);
        assert_eq!(origin.cluster_id, "cluster-b");
        assert_eq!(origin.priority, 5);
        assert_eq!(origin.log_index, 100);
        assert!(!origin.is_local());
    }

    #[test]
    fn test_should_replace() {
        let hlc = create_hlc("test-node");
        let origin = KeyOrigin::remote("cluster-b", 5, 100, &hlc);

        // Lower priority should replace
        assert!(origin.should_replace(3));
        assert!(origin.should_replace(0));

        // Same or higher priority should not replace
        assert!(!origin.should_replace(5));
        assert!(!origin.should_replace(10));
    }

    #[test]
    fn test_storage_key() {
        assert_eq!(KeyOrigin::storage_key("foo"), "__aspen_origin:foo");
        assert_eq!(KeyOrigin::storage_key("bar/baz"), "__aspen_origin:bar/baz");
    }

    #[test]
    fn test_parse_key() {
        assert_eq!(KeyOrigin::parse_key("__aspen_origin:foo"), Some("foo"));
        assert_eq!(KeyOrigin::parse_key("__aspen_origin:bar/baz"), Some("bar/baz"));
        assert_eq!(KeyOrigin::parse_key("other:foo"), None);
    }

    #[test]
    fn test_serialization() {
        let hlc = create_hlc("test-node");
        let origin = KeyOrigin::remote("cluster-b", 5, 100, &hlc);
        let bytes = origin.to_bytes();
        let parsed = KeyOrigin::from_bytes(&bytes).expect("should parse");
        assert_eq!(parsed, origin);
    }
}
