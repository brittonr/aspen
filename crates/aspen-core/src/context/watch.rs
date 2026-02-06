//! Watch registry for KV subscriptions.
//!
//! Provides traits and types for tracking active watch subscriptions
//! across the system.

use async_trait::async_trait;

/// Key origin information.
#[derive(Debug, Clone)]
pub struct KeyOrigin {
    /// Cluster ID where the key originated.
    pub cluster_id: String,
    /// Priority of the origin cluster.
    pub priority: u32,
    /// Timestamp when the key was created.
    pub timestamp_secs: u64,
}

impl KeyOrigin {
    /// Check if the key originated locally.
    pub fn is_local(&self) -> bool {
        // This would be implemented based on local cluster ID comparison
        false
    }
}

/// Information about an active watch subscription.
#[derive(Debug, Clone)]
pub struct WatchInfo {
    /// Unique watch ID.
    pub watch_id: u64,
    /// Key prefix being watched.
    pub prefix: String,
    /// Last sent log index.
    pub last_sent_index: u64,
    /// Number of events sent.
    pub events_sent: u64,
    /// Watch creation timestamp (ms since epoch).
    pub created_at_ms: u64,
    /// Whether to include previous value in events.
    pub include_prev_value: bool,
}

/// Registry for tracking active watch subscriptions.
///
/// This trait allows querying the status of active watches across the system.
/// Watches are created via the streaming protocol (LOG_SUBSCRIBER_ALPN), but
/// their status can be queried via the standard RPC interface.
///
/// # Tiger Style
///
/// - Bounded watch count via MAX_WATCHES constant
/// - Thread-safe via interior mutability
/// - Read operations are lock-free where possible
#[async_trait]
pub trait WatchRegistry: Send + Sync {
    /// Get all active watches.
    ///
    /// Returns a list of all currently active watch subscriptions.
    async fn get_all_watches(&self) -> Vec<WatchInfo>;

    /// Get a specific watch by ID.
    ///
    /// Returns `None` if the watch doesn't exist.
    async fn get_watch(&self, watch_id: u64) -> Option<WatchInfo>;

    /// Get the total number of active watches.
    async fn watch_count(&self) -> usize;

    /// Register a new watch subscription.
    ///
    /// Called by `LogSubscriberProtocolHandler` when a client creates a watch.
    /// Returns the assigned watch ID.
    fn register_watch(&self, prefix: String, include_prev_value: bool) -> u64;

    /// Update watch state after sending events.
    ///
    /// Called by `LogSubscriberProtocolHandler` after sending events to a subscriber.
    fn update_watch(&self, watch_id: u64, last_sent_index: u64, events_sent: u64);

    /// Unregister a watch subscription.
    ///
    /// Called by `LogSubscriberProtocolHandler` when a client disconnects or cancels.
    fn unregister_watch(&self, watch_id: u64);
}

/// In-memory implementation of WatchRegistry.
///
/// Thread-safe via `RwLock` for the watch map and atomic counters.
pub struct InMemoryWatchRegistry {
    watches: std::sync::RwLock<std::collections::HashMap<u64, WatchInfo>>,
    next_watch_id: std::sync::atomic::AtomicU64,
}

impl InMemoryWatchRegistry {
    /// Create a new in-memory watch registry.
    pub fn new() -> Self {
        Self {
            watches: std::sync::RwLock::new(std::collections::HashMap::new()),
            next_watch_id: std::sync::atomic::AtomicU64::new(1),
        }
    }
}

impl Default for InMemoryWatchRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WatchRegistry for InMemoryWatchRegistry {
    async fn get_all_watches(&self) -> Vec<WatchInfo> {
        // Tiger Style: Recover from poisoned locks instead of panicking.
        // If a thread panicked while holding this lock, we can still safely
        // read the data - the lock contents may be in an inconsistent state
        // but HashMap<u64, WatchInfo> has no invariants that can be violated.
        self.watches.read().unwrap_or_else(|poisoned| poisoned.into_inner()).values().cloned().collect()
    }

    async fn get_watch(&self, watch_id: u64) -> Option<WatchInfo> {
        self.watches.read().unwrap_or_else(|poisoned| poisoned.into_inner()).get(&watch_id).cloned()
    }

    async fn watch_count(&self) -> usize {
        self.watches.read().unwrap_or_else(|poisoned| poisoned.into_inner()).len()
    }

    fn register_watch(&self, prefix: String, include_prev_value: bool) -> u64 {
        let watch_id = self.next_watch_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let now_ms =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

        let info = WatchInfo {
            watch_id,
            prefix,
            last_sent_index: 0,
            events_sent: 0,
            created_at_ms: now_ms,
            include_prev_value,
        };

        // Tiger Style: Recover from poisoned locks - the HashMap insert is safe
        // even if the previous holder panicked mid-operation.
        self.watches.write().unwrap_or_else(|poisoned| poisoned.into_inner()).insert(watch_id, info);

        watch_id
    }

    fn update_watch(&self, watch_id: u64, last_sent_index: u64, events_sent: u64) {
        let mut watches = self.watches.write().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(info) = watches.get_mut(&watch_id) {
            info.last_sent_index = last_sent_index;
            info.events_sent = events_sent;
        }
    }

    fn unregister_watch(&self, watch_id: u64) {
        self.watches.write().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(&watch_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_watch_registry_new() {
        let registry = InMemoryWatchRegistry::new();
        assert_eq!(registry.watch_count().await, 0);
    }

    #[tokio::test]
    async fn test_in_memory_watch_registry_default() {
        let registry = InMemoryWatchRegistry::default();
        assert_eq!(registry.watch_count().await, 0);
    }

    #[tokio::test]
    async fn test_register_watch() {
        let registry = InMemoryWatchRegistry::new();

        let id1 = registry.register_watch("prefix1/".to_string(), false);
        assert_eq!(id1, 1);
        assert_eq!(registry.watch_count().await, 1);

        let id2 = registry.register_watch("prefix2/".to_string(), true);
        assert_eq!(id2, 2);
        assert_eq!(registry.watch_count().await, 2);

        // IDs should be unique and increasing
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn test_get_watch() {
        let registry = InMemoryWatchRegistry::new();

        let id = registry.register_watch("test/".to_string(), true);

        let info = registry.get_watch(id).await;
        assert!(info.is_some());

        let info = info.unwrap();
        assert_eq!(info.watch_id, id);
        assert_eq!(info.prefix, "test/");
        assert!(info.include_prev_value);
        assert_eq!(info.last_sent_index, 0);
        assert_eq!(info.events_sent, 0);
        assert!(info.created_at_ms > 0);
    }

    #[tokio::test]
    async fn test_get_watch_not_found() {
        let registry = InMemoryWatchRegistry::new();
        assert!(registry.get_watch(999).await.is_none());
    }

    #[tokio::test]
    async fn test_get_all_watches() {
        let registry = InMemoryWatchRegistry::new();

        registry.register_watch("a/".to_string(), false);
        registry.register_watch("b/".to_string(), true);
        registry.register_watch("c/".to_string(), false);

        let watches = registry.get_all_watches().await;
        assert_eq!(watches.len(), 3);

        // Check all prefixes are present
        let prefixes: Vec<_> = watches.iter().map(|w| w.prefix.as_str()).collect();
        assert!(prefixes.contains(&"a/"));
        assert!(prefixes.contains(&"b/"));
        assert!(prefixes.contains(&"c/"));
    }

    #[tokio::test]
    async fn test_update_watch() {
        let registry = InMemoryWatchRegistry::new();

        let id = registry.register_watch("test/".to_string(), false);

        // Initial state
        let info = registry.get_watch(id).await.unwrap();
        assert_eq!(info.last_sent_index, 0);
        assert_eq!(info.events_sent, 0);

        // Update
        registry.update_watch(id, 100, 42);

        // Verify update
        let info = registry.get_watch(id).await.unwrap();
        assert_eq!(info.last_sent_index, 100);
        assert_eq!(info.events_sent, 42);
    }

    #[tokio::test]
    async fn test_update_nonexistent_watch() {
        let registry = InMemoryWatchRegistry::new();

        // Should not panic or error
        registry.update_watch(999, 100, 42);
    }

    #[tokio::test]
    async fn test_unregister_watch() {
        let registry = InMemoryWatchRegistry::new();

        let id = registry.register_watch("test/".to_string(), false);
        assert_eq!(registry.watch_count().await, 1);

        registry.unregister_watch(id);
        assert_eq!(registry.watch_count().await, 0);
        assert!(registry.get_watch(id).await.is_none());
    }

    #[tokio::test]
    async fn test_unregister_nonexistent_watch() {
        let registry = InMemoryWatchRegistry::new();

        // Should not panic or error
        registry.unregister_watch(999);
    }

    #[tokio::test]
    async fn test_watch_ids_are_unique() {
        let registry = InMemoryWatchRegistry::new();

        let mut ids = std::collections::HashSet::new();
        for _ in 0..100 {
            let id = registry.register_watch("test/".to_string(), false);
            assert!(ids.insert(id), "Watch ID {} was not unique", id);
        }
    }

    #[test]
    fn test_watch_info_debug() {
        let info = WatchInfo {
            watch_id: 1,
            prefix: "test/".to_string(),
            last_sent_index: 100,
            events_sent: 42,
            created_at_ms: 1704326400000, // 2024-01-04 00:00:00 UTC
            include_prev_value: true,
        };

        let debug = format!("{:?}", info);
        assert!(debug.contains("watch_id: 1"));
        assert!(debug.contains("prefix: \"test/\""));
    }

    #[test]
    fn test_watch_info_clone() {
        let info = WatchInfo {
            watch_id: 1,
            prefix: "test/".to_string(),
            last_sent_index: 100,
            events_sent: 42,
            created_at_ms: 1704326400000,
            include_prev_value: true,
        };

        let cloned = info.clone();
        assert_eq!(cloned.watch_id, info.watch_id);
        assert_eq!(cloned.prefix, info.prefix);
        assert_eq!(cloned.last_sent_index, info.last_sent_index);
        assert_eq!(cloned.events_sent, info.events_sent);
        assert_eq!(cloned.created_at_ms, info.created_at_ms);
        assert_eq!(cloned.include_prev_value, info.include_prev_value);
    }

    // ========================================================================
    // KeyOrigin Tests
    // ========================================================================

    #[test]
    fn key_origin_construction() {
        let origin = KeyOrigin {
            cluster_id: "cluster-123".to_string(),
            priority: 10,
            timestamp_secs: 1704326400,
        };
        assert_eq!(origin.cluster_id, "cluster-123");
        assert_eq!(origin.priority, 10);
        assert_eq!(origin.timestamp_secs, 1704326400);
    }

    #[test]
    fn key_origin_debug() {
        let origin = KeyOrigin {
            cluster_id: "debug-cluster".to_string(),
            priority: 5,
            timestamp_secs: 1000000,
        };
        let debug = format!("{:?}", origin);
        assert!(debug.contains("KeyOrigin"));
        assert!(debug.contains("debug-cluster"));
        assert!(debug.contains("5"));
    }

    #[test]
    fn key_origin_clone() {
        let origin = KeyOrigin {
            cluster_id: "original".to_string(),
            priority: 100,
            timestamp_secs: 999999,
        };
        let cloned = origin.clone();
        assert_eq!(origin.cluster_id, cloned.cluster_id);
        assert_eq!(origin.priority, cloned.priority);
        assert_eq!(origin.timestamp_secs, cloned.timestamp_secs);
    }

    #[test]
    fn key_origin_is_local_default() {
        let origin = KeyOrigin {
            cluster_id: "any-cluster".to_string(),
            priority: 1,
            timestamp_secs: 0,
        };
        // Current implementation always returns false
        assert!(!origin.is_local());
    }

    #[test]
    fn key_origin_zero_priority() {
        let origin = KeyOrigin {
            cluster_id: "zero-priority".to_string(),
            priority: 0,
            timestamp_secs: 12345,
        };
        assert_eq!(origin.priority, 0);
    }

    #[test]
    fn key_origin_max_priority() {
        let origin = KeyOrigin {
            cluster_id: "max-priority".to_string(),
            priority: u32::MAX,
            timestamp_secs: 0,
        };
        assert_eq!(origin.priority, u32::MAX);
    }
}
