//! Local LRU cache for subscription data.
//!
//! Provides per-subscription caching with TTL-based expiration.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;
use std::time::Instant;

/// A cached entry with TTL tracking.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// The cached value.
    value: String,
    /// When this entry expires.
    expires_at: Instant,
    /// When this entry was last accessed (for LRU).
    last_access: Instant,
}

impl CacheEntry {
    fn new(value: String, ttl: Duration) -> Self {
        let now = Instant::now();
        Self {
            value,
            expires_at: now + ttl,
            last_access: now,
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    #[allow(dead_code)] // Reserved for future LRU eviction
    fn touch(&mut self) {
        self.last_access = Instant::now();
    }
}

/// Local cache for a single subscription.
pub struct LocalCache {
    /// Cached entries by key.
    entries: RwLock<HashMap<String, CacheEntry>>,
    /// Default TTL for new entries.
    default_ttl: Duration,
    /// Maximum number of entries.
    max_entries: usize,
    /// Subscription ID this cache belongs to.
    subscription_id: String,
}

impl LocalCache {
    /// Create a new local cache.
    pub fn new(subscription_id: impl Into<String>, default_ttl: Duration, max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            default_ttl,
            max_entries,
            subscription_id: subscription_id.into(),
        }
    }

    /// Get a cached value if it exists and hasn't expired.
    pub fn get(&self, key: &str) -> Option<String> {
        // Try read lock first
        {
            let entries = self.entries.read().ok()?;
            if let Some(entry) = entries.get(key)
                && !entry.is_expired()
            {
                return Some(entry.value.clone());
            }
        }

        // If entry is expired, remove it (needs write lock)
        let mut entries = self.entries.write().ok()?;
        if let Some(entry) = entries.get(key)
            && entry.is_expired()
        {
            entries.remove(key);
        }
        None
    }

    /// Set a cached value with the default TTL.
    pub fn set(&self, key: impl Into<String>, value: impl Into<String>) {
        self.set_with_ttl(key, value, self.default_ttl);
    }

    /// Set a cached value with a custom TTL.
    pub fn set_with_ttl(&self, key: impl Into<String>, value: impl Into<String>, ttl: Duration) {
        let key = key.into();
        let entry = CacheEntry::new(value.into(), ttl);

        let mut entries = match self.entries.write() {
            Ok(e) => e,
            Err(_) => return, // Lock poisoned, skip caching
        };

        // If at capacity, evict oldest entries
        if entries.len() >= self.max_entries && !entries.contains_key(&key) {
            self.evict_lru(&mut entries);
        }

        entries.insert(key, entry);
    }

    /// Invalidate a specific key.
    pub fn invalidate(&self, key: &str) {
        if let Ok(mut entries) = self.entries.write() {
            entries.remove(key);
        }
    }

    /// Invalidate all entries matching a prefix.
    pub fn invalidate_prefix(&self, prefix: &str) {
        if let Ok(mut entries) = self.entries.write() {
            entries.retain(|k, _| !k.starts_with(prefix));
        }
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.write() {
            entries.clear();
        }
    }

    /// Get the number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.read().map(|e| e.len()).unwrap_or(0)
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the subscription ID for this cache.
    pub fn subscription_id(&self) -> &str {
        &self.subscription_id
    }

    /// Evict expired entries and return the count removed.
    pub fn evict_expired(&self) -> usize {
        let mut entries = match self.entries.write() {
            Ok(e) => e,
            Err(_) => return 0,
        };

        let before = entries.len();
        entries.retain(|_, entry| !entry.is_expired());
        before - entries.len()
    }

    /// Evict the least recently used entry.
    fn evict_lru(&self, entries: &mut HashMap<String, CacheEntry>) {
        // Find the oldest entry
        let oldest = entries.iter().min_by_key(|(_, entry)| entry.last_access).map(|(k, _)| k.clone());

        if let Some(key) = oldest {
            entries.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        let cache = LocalCache::new("test", Duration::from_secs(60), 100);

        cache.set("key1", "value1");
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
        assert_eq!(cache.get("key2"), None);
    }

    #[test]
    fn test_cache_expiry() {
        let cache = LocalCache::new("test", Duration::from_millis(10), 100);

        cache.set("key1", "value1");
        assert_eq!(cache.get("key1"), Some("value1".to_string()));

        // Wait for expiry
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(cache.get("key1"), None);
    }

    #[test]
    fn test_cache_invalidation() {
        let cache = LocalCache::new("test", Duration::from_secs(60), 100);

        cache.set("user:1", "alice");
        cache.set("user:2", "bob");
        cache.set("config:a", "value");

        cache.invalidate("user:1");
        assert_eq!(cache.get("user:1"), None);
        assert_eq!(cache.get("user:2"), Some("bob".to_string()));

        cache.invalidate_prefix("user:");
        assert_eq!(cache.get("user:2"), None);
        assert_eq!(cache.get("config:a"), Some("value".to_string()));
    }

    #[test]
    fn test_cache_capacity() {
        let cache = LocalCache::new("test", Duration::from_secs(60), 3);

        cache.set("key1", "value1");
        cache.set("key2", "value2");
        cache.set("key3", "value3");
        assert_eq!(cache.len(), 3);

        // Adding a 4th should evict one
        cache.set("key4", "value4");
        assert_eq!(cache.len(), 3);
    }
}
