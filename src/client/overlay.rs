//! Client overlay for priority-based lookup across clusters.
//!
//! The overlay manages multiple cluster subscriptions and routes
//! read/write operations according to priority ordering.
//!
//! ## Read Path
//!
//! 1. Check local cache (per-subscription)
//! 2. Query subscriptions in priority order
//! 3. Return first match (no merge between clusters)
//!
//! ## Write Path
//!
//! 1. Find first subscription with write access
//! 2. Route write to that cluster's leader
//! 3. Invalidate cache for affected keys

use std::collections::HashMap;
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use super::cache::LocalCache;
use super::constants::MAX_SUBSCRIPTIONS;
use super::subscription::{AccessLevel, ClusterSubscription};

/// Result of a read operation.
#[derive(Debug, Clone)]
pub struct ReadResult {
    /// The value, if found.
    pub value: Option<String>,
    /// Which subscription provided the value.
    pub source: Option<String>,
    /// Whether the result came from cache.
    pub from_cache: bool,
}

/// Result of a write operation.
#[derive(Debug, Clone)]
pub struct WriteResult {
    /// The subscription that handled the write.
    pub source: String,
    /// Whether the write was successful.
    pub success: bool,
}

/// Errors that can occur during overlay operations.
#[derive(Debug, Error)]
pub enum OverlayError {
    #[error("no subscriptions available")]
    NoSubscriptions,
    #[error("no write-capable subscription available")]
    NoWriteSubscription,
    #[error("subscription limit exceeded (max: {max})")]
    SubscriptionLimitExceeded { max: usize },
    #[error("subscription not found: {id}")]
    SubscriptionNotFound { id: String },
    #[error("operation failed: {reason}")]
    OperationFailed { reason: String },
    #[error("cluster connection error: {reason}")]
    ConnectionError { reason: String },
}

/// Client overlay for managing multiple cluster subscriptions.
///
/// Provides a unified interface for reading and writing across
/// multiple Aspen clusters with priority-based routing.
pub struct ClientOverlay {
    /// Active subscriptions, ordered by priority.
    subscriptions: RwLock<Vec<ClusterSubscription>>,
    /// Per-subscription caches.
    caches: RwLock<HashMap<String, Arc<LocalCache>>>,
    /// Callback for reading from a subscription.
    /// In a real implementation, this would connect to the cluster.
    read_handler: Option<Arc<dyn Fn(&str, &str) -> Option<String> + Send + Sync>>,
    /// Callback for writing to a subscription.
    write_handler: Option<Arc<dyn Fn(&str, &str, &str) -> bool + Send + Sync>>,
}

impl ClientOverlay {
    /// Create a new client overlay.
    pub fn new() -> Self {
        Self {
            subscriptions: RwLock::new(Vec::new()),
            caches: RwLock::new(HashMap::new()),
            read_handler: None,
            write_handler: None,
        }
    }

    /// Add a subscription to the overlay.
    pub async fn add_subscription(
        &self,
        subscription: ClusterSubscription,
    ) -> Result<(), OverlayError> {
        let mut subs = self.subscriptions.write().await;

        if subs.len() >= MAX_SUBSCRIPTIONS {
            return Err(OverlayError::SubscriptionLimitExceeded {
                max: MAX_SUBSCRIPTIONS,
            });
        }

        // Check for duplicate ID
        if subs.iter().any(|s| s.id == subscription.id) {
            // Update existing subscription
            if let Some(existing) = subs.iter_mut().find(|s| s.id == subscription.id) {
                *existing = subscription.clone();
            }
        } else {
            subs.push(subscription.clone());
        }

        // Sort by priority (lower = higher priority)
        subs.sort_by_key(|s| s.priority);

        // Create cache for this subscription if caching is enabled
        if subscription.cache_config.enabled {
            let cache = Arc::new(LocalCache::new(
                &subscription.id,
                subscription.cache_config.ttl,
                subscription.cache_config.max_entries,
            ));
            self.caches
                .write()
                .await
                .insert(subscription.id.clone(), cache);
        }

        Ok(())
    }

    /// Remove a subscription from the overlay.
    pub async fn remove_subscription(&self, id: &str) -> Result<(), OverlayError> {
        let mut subs = self.subscriptions.write().await;
        let before = subs.len();
        subs.retain(|s| s.id != id);

        if subs.len() == before {
            return Err(OverlayError::SubscriptionNotFound { id: id.to_string() });
        }

        // Remove cache
        self.caches.write().await.remove(id);

        Ok(())
    }

    /// Get all subscriptions.
    pub async fn subscriptions(&self) -> Vec<ClusterSubscription> {
        self.subscriptions.read().await.clone()
    }

    /// Read a key from the overlay.
    ///
    /// Queries subscriptions in priority order and returns the first match.
    /// Checks cache first if enabled for the subscription.
    pub async fn read(&self, key: &str) -> Result<ReadResult, OverlayError> {
        let subs = self.subscriptions.read().await;
        if subs.is_empty() {
            return Err(OverlayError::NoSubscriptions);
        }

        let caches = self.caches.read().await;

        for sub in subs.iter() {
            // Check cache first
            if let Some(cache) = caches.get(&sub.id) {
                if let Some(value) = cache.get(key) {
                    debug!(
                        subscription = %sub.id,
                        key,
                        "cache hit"
                    );
                    return Ok(ReadResult {
                        value: Some(value),
                        source: Some(sub.id.clone()),
                        from_cache: true,
                    });
                }
            }

            // Try to read from cluster
            if let Some(handler) = &self.read_handler
                && let Some(value) = handler(&sub.id, key) {
                // Cache the result
                if let Some(cache) = caches.get(&sub.id) {
                    cache.set(key, &value);
                }

                debug!(
                    subscription = %sub.id,
                    key,
                    "found value"
                );
                return Ok(ReadResult {
                    value: Some(value),
                    source: Some(sub.id.clone()),
                    from_cache: false,
                });
            }
        }

        // Key not found in any subscription
        Ok(ReadResult {
            value: None,
            source: None,
            from_cache: false,
        })
    }

    /// Write a key-value pair.
    ///
    /// Routes to the first subscription with write access.
    pub async fn write(&self, key: &str, value: &str) -> Result<WriteResult, OverlayError> {
        let subs = self.subscriptions.read().await;

        // Find first write-capable subscription
        let write_sub = subs
            .iter()
            .find(|s| s.access == AccessLevel::ReadWrite)
            .ok_or(OverlayError::NoWriteSubscription)?;

        let sub_id = write_sub.id.clone();

        // Perform write
        let success = if let Some(handler) = &self.write_handler {
            handler(&sub_id, key, value)
        } else {
            warn!(subscription = %sub_id, "no write handler configured");
            false
        };

        if success {
            // Invalidate cache for this key across all subscriptions
            let caches = self.caches.read().await;
            for cache in caches.values() {
                cache.invalidate(key);
            }
        }

        Ok(WriteResult {
            source: sub_id,
            success,
        })
    }

    /// Delete a key.
    ///
    /// Routes to the first subscription with write access.
    pub async fn delete(&self, key: &str) -> Result<WriteResult, OverlayError> {
        // For now, delete is implemented as write with empty value
        // A real implementation would have a proper delete operation
        self.write(key, "").await
    }

    /// Invalidate cache entries for a key across all subscriptions.
    pub async fn invalidate(&self, key: &str) {
        let caches = self.caches.read().await;
        for cache in caches.values() {
            cache.invalidate(key);
        }
    }

    /// Invalidate all cache entries matching a prefix.
    pub async fn invalidate_prefix(&self, prefix: &str) {
        let caches = self.caches.read().await;
        for cache in caches.values() {
            cache.invalidate_prefix(prefix);
        }
    }

    /// Clear all caches.
    pub async fn clear_caches(&self) {
        let caches = self.caches.read().await;
        for cache in caches.values() {
            cache.clear();
        }
    }

    /// Set the read handler for testing.
    #[cfg(test)]
    pub fn set_read_handler<F>(&mut self, handler: F)
    where
        F: Fn(&str, &str) -> Option<String> + Send + Sync + 'static,
    {
        self.read_handler = Some(Arc::new(handler));
    }

    /// Set the write handler for testing.
    #[cfg(test)]
    pub fn set_write_handler<F>(&mut self, handler: F)
    where
        F: Fn(&str, &str, &str) -> bool + Send + Sync + 'static,
    {
        self.write_handler = Some(Arc::new(handler));
    }
}

impl Default for ClientOverlay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[tokio::test]
    async fn test_subscription_ordering() {
        let overlay = ClientOverlay::new();

        // Add subscriptions out of order
        overlay
            .add_subscription(
                ClusterSubscription::new("sub-2", "Second", "cluster-b").with_priority(2),
            )
            .await
            .unwrap();
        overlay
            .add_subscription(
                ClusterSubscription::new("sub-1", "First", "cluster-a").with_priority(1),
            )
            .await
            .unwrap();
        overlay
            .add_subscription(
                ClusterSubscription::new("sub-0", "Primary", "cluster-c").with_priority(0),
            )
            .await
            .unwrap();

        let subs = overlay.subscriptions().await;
        assert_eq!(subs[0].id, "sub-0");
        assert_eq!(subs[1].id, "sub-1");
        assert_eq!(subs[2].id, "sub-2");
    }

    #[tokio::test]
    async fn test_read_priority() {
        let mut overlay = ClientOverlay::new();

        // Set up test data - first subscription has key
        let data = Arc::new(Mutex::new(HashMap::<String, HashMap<String, String>>::new()));
        {
            let mut d = data.lock().unwrap();
            d.insert("sub-1".to_string(), {
                let mut m = HashMap::new();
                m.insert("key1".to_string(), "value-from-1".to_string());
                m
            });
            d.insert("sub-2".to_string(), {
                let mut m = HashMap::new();
                m.insert("key1".to_string(), "value-from-2".to_string());
                m.insert("key2".to_string(), "value-from-2".to_string());
                m
            });
        }

        let data_clone = data.clone();
        overlay.set_read_handler(move |sub_id, key| {
            let d = data_clone.lock().unwrap();
            d.get(sub_id).and_then(|m| m.get(key).cloned())
        });

        overlay
            .add_subscription(
                ClusterSubscription::new("sub-1", "First", "cluster-a").with_priority(1),
            )
            .await
            .unwrap();
        overlay
            .add_subscription(
                ClusterSubscription::new("sub-2", "Second", "cluster-b").with_priority(2),
            )
            .await
            .unwrap();

        // key1 exists in both - should get from sub-1 (higher priority)
        let result = overlay.read("key1").await.unwrap();
        assert_eq!(result.value, Some("value-from-1".to_string()));
        assert_eq!(result.source, Some("sub-1".to_string()));

        // key2 only exists in sub-2
        let result = overlay.read("key2").await.unwrap();
        assert_eq!(result.value, Some("value-from-2".to_string()));
        assert_eq!(result.source, Some("sub-2".to_string()));

        // key3 doesn't exist anywhere
        let result = overlay.read("key3").await.unwrap();
        assert_eq!(result.value, None);
    }

    #[tokio::test]
    async fn test_write_routing() {
        let mut overlay = ClientOverlay::new();

        let writes = Arc::new(Mutex::new(Vec::new()));
        let writes_clone = writes.clone();
        overlay.set_write_handler(move |sub_id, key, value| {
            writes_clone.lock().unwrap().push((
                sub_id.to_string(),
                key.to_string(),
                value.to_string(),
            ));
            true
        });

        // Add read-only and read-write subscriptions
        overlay
            .add_subscription(
                ClusterSubscription::new("readonly", "Read Only", "cluster-a").with_priority(0),
            )
            .await
            .unwrap();
        overlay
            .add_subscription(
                ClusterSubscription::new("readwrite", "Read Write", "cluster-b")
                    .with_priority(1)
                    .with_access(AccessLevel::ReadWrite),
            )
            .await
            .unwrap();

        // Write should go to readwrite subscription (first with write access)
        let result = overlay.write("key1", "value1").await.unwrap();
        assert_eq!(result.source, "readwrite");
        assert!(result.success);

        let recorded = writes.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(
            recorded[0],
            (
                "readwrite".to_string(),
                "key1".to_string(),
                "value1".to_string()
            )
        );
    }
}
