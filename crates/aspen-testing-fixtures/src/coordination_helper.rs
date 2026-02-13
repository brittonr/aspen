//! Helper utilities for testing coordination primitives.
//!
//! This module provides test helpers that simplify testing distributed
//! coordination primitives like locks, counters, barriers, and queues.

use std::sync::Arc;
use std::time::Duration;

use aspen_testing_core::DeterministicKeyValueStore;
use aspen_traits::KeyValueStore;

/// Helper for testing coordination primitives.
///
/// Provides utilities for setting up test scenarios, managing
/// simulated time, and asserting coordination invariants.
///
/// # Example
///
/// ```ignore
/// let helper = CoordinationTestHelper::new().await;
///
/// // Test lock acquisition
/// let lock_key = helper.lock_key("test-lock");
/// helper.assert_unlocked(&lock_key).await;
///
/// // Simulate lock acquisition
/// helper.acquire_lock(&lock_key, "holder-1").await;
/// helper.assert_locked(&lock_key, "holder-1").await;
/// ```
pub struct CoordinationTestHelper {
    /// The underlying KV store.
    store: Arc<DeterministicKeyValueStore>,
    /// Key prefix for coordination primitives.
    prefix: String,
    /// Current simulated time in milliseconds.
    current_time_ms: u64,
}

impl CoordinationTestHelper {
    /// Create a new coordination test helper.
    pub async fn new() -> Self {
        Self {
            store: DeterministicKeyValueStore::new(),
            prefix: "coordination/".to_string(),
            current_time_ms: 0,
        }
    }

    /// Create a helper with a custom KV store.
    pub fn with_store(store: Arc<DeterministicKeyValueStore>) -> Self {
        Self {
            store,
            prefix: "coordination/".to_string(),
            current_time_ms: 0,
        }
    }

    /// Create a helper with a custom prefix.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Get the underlying KV store.
    pub fn store(&self) -> &Arc<DeterministicKeyValueStore> {
        &self.store
    }

    /// Get a key for a lock primitive.
    pub fn lock_key(&self, name: &str) -> String {
        format!("{}locks/{}", self.prefix, name)
    }

    /// Get a key for a counter primitive.
    pub fn counter_key(&self, name: &str) -> String {
        format!("{}counters/{}", self.prefix, name)
    }

    /// Get a key for a barrier primitive.
    pub fn barrier_key(&self, name: &str) -> String {
        format!("{}barriers/{}", self.prefix, name)
    }

    /// Get a key for a queue primitive.
    pub fn queue_key(&self, name: &str) -> String {
        format!("{}queues/{}", self.prefix, name)
    }

    /// Get the current simulated time.
    pub fn current_time_ms(&self) -> u64 {
        self.current_time_ms
    }

    /// Advance simulated time by the given duration.
    pub fn advance_time(&mut self, duration: Duration) {
        self.current_time_ms += duration.as_millis() as u64;
    }

    /// Set the simulated time to a specific value.
    pub fn set_time_ms(&mut self, time_ms: u64) {
        self.current_time_ms = time_ms;
    }

    /// Check if a key exists in the store.
    pub async fn key_exists(&self, key: &str) -> bool {
        use aspen_kv_types::ReadRequest;
        self.store.read(ReadRequest::new(key)).await.is_ok()
    }

    /// Get the value at a key, if it exists.
    pub async fn get_value(&self, key: &str) -> Option<String> {
        use aspen_kv_types::ReadRequest;
        self.store.read(ReadRequest::new(key)).await.ok().and_then(|r| r.kv).map(|kv| kv.value)
    }

    /// Set a value at a key.
    pub async fn set_value(&self, key: &str, value: &str) -> Result<(), String> {
        use aspen_kv_types::WriteRequest;
        self.store.write(WriteRequest::set(key, value)).await.map(|_| ()).map_err(|e| e.to_string())
    }

    /// Delete a key.
    pub async fn delete_key(&self, key: &str) -> bool {
        use aspen_kv_types::DeleteRequest;
        self.store.delete(DeleteRequest::new(key)).await.map(|r| r.deleted).unwrap_or(false)
    }

    /// Assert that a lock is not held.
    pub async fn assert_unlocked(&self, lock_key: &str) {
        assert!(!self.key_exists(lock_key).await, "Lock at {} should not be held", lock_key);
    }

    /// Assert that a lock is held by a specific holder.
    pub async fn assert_locked(&self, lock_key: &str, expected_holder: &str) {
        let value = self.get_value(lock_key).await;
        assert!(value.is_some(), "Lock at {} should be held", lock_key);
        let holder = value.unwrap();
        assert!(
            holder.contains(expected_holder),
            "Lock at {} should be held by {}, but is held by {}",
            lock_key,
            expected_holder,
            holder
        );
    }

    /// Assert that a counter has a specific value.
    pub async fn assert_counter_value(&self, counter_key: &str, expected: i64) {
        let value = self.get_value(counter_key).await;
        let actual: i64 = value.as_ref().and_then(|v| v.parse().ok()).unwrap_or(0);
        assert_eq!(actual, expected, "Counter at {} should be {}, but is {}", counter_key, expected, actual);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_helper_creation() {
        let helper = CoordinationTestHelper::new().await;
        assert_eq!(helper.current_time_ms(), 0);
    }

    #[tokio::test]
    async fn test_key_generation() {
        let helper = CoordinationTestHelper::new().await;

        assert_eq!(helper.lock_key("test"), "coordination/locks/test");
        assert_eq!(helper.counter_key("test"), "coordination/counters/test");
        assert_eq!(helper.barrier_key("test"), "coordination/barriers/test");
        assert_eq!(helper.queue_key("test"), "coordination/queues/test");
    }

    #[tokio::test]
    async fn test_custom_prefix() {
        let helper = CoordinationTestHelper::new().await.with_prefix("custom/");

        assert_eq!(helper.lock_key("test"), "custom/locks/test");
    }

    #[tokio::test]
    async fn test_time_manipulation() {
        let mut helper = CoordinationTestHelper::new().await;

        assert_eq!(helper.current_time_ms(), 0);

        helper.advance_time(Duration::from_secs(1));
        assert_eq!(helper.current_time_ms(), 1000);

        helper.advance_time(Duration::from_millis(500));
        assert_eq!(helper.current_time_ms(), 1500);

        helper.set_time_ms(5000);
        assert_eq!(helper.current_time_ms(), 5000);
    }

    #[tokio::test]
    async fn test_value_operations() {
        let helper = CoordinationTestHelper::new().await;
        let key = "test/key";

        // Key should not exist initially
        assert!(!helper.key_exists(key).await);
        assert!(helper.get_value(key).await.is_none());

        // Set a value
        helper.set_value(key, "test-value").await.unwrap();
        assert!(helper.key_exists(key).await);
        assert_eq!(helper.get_value(key).await, Some("test-value".to_string()));

        // Delete the key
        assert!(helper.delete_key(key).await);
        assert!(!helper.key_exists(key).await);
    }

    #[tokio::test]
    async fn test_lock_assertions() {
        let helper = CoordinationTestHelper::new().await;
        let lock_key = helper.lock_key("test-lock");

        // Lock should be unlocked initially
        helper.assert_unlocked(&lock_key).await;

        // Simulate acquiring the lock
        helper.set_value(&lock_key, "holder-1").await.unwrap();
        helper.assert_locked(&lock_key, "holder-1").await;

        // Release the lock
        helper.delete_key(&lock_key).await;
        helper.assert_unlocked(&lock_key).await;
    }

    #[tokio::test]
    async fn test_counter_assertions() {
        let helper = CoordinationTestHelper::new().await;
        let counter_key = helper.counter_key("test-counter");

        // Counter should be 0 initially (no key)
        helper.assert_counter_value(&counter_key, 0).await;

        // Set counter to 5
        helper.set_value(&counter_key, "5").await.unwrap();
        helper.assert_counter_value(&counter_key, 5).await;

        // Update counter to -3
        helper.set_value(&counter_key, "-3").await.unwrap();
        helper.assert_counter_value(&counter_key, -3).await;
    }
}
