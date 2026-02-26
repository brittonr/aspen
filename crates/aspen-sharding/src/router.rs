//! Shard routing for distributing keys across shards.
//!
//! The `ShardRouter` maps keys to shards using consistent hashing,
//! ensuring that keys are distributed uniformly and that minimal
//! redistribution occurs when shards are added or removed.
//!
//! # Tiger Style
//!
//! - Fixed limits: MAX_SHARDS enforced at construction time
//! - Explicit bounds: All operations validate inputs
//! - Thread-safe: Uses interior mutability for shard map updates

use std::collections::HashSet;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::Deserialize;
use serde::Serialize;

use crate::MAX_SHARDS;
use crate::MIN_SHARDS;
use crate::consistent_hash::JumpHash;

/// Unique identifier for a shard.
///
/// Shard IDs are assigned sequentially starting from 0.
pub type ShardId = u32;

/// Configuration for shard routing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShardConfig {
    /// Number of shards in the cluster.
    pub num_shards: u32,
}

impl ShardConfig {
    /// Create a new shard configuration.
    ///
    /// # Arguments
    ///
    /// * `num_shards` - Number of shards (must be 1..=MAX_SHARDS)
    ///
    /// # Panics
    ///
    /// Panics if `num_shards` is 0 or greater than MAX_SHARDS.
    pub fn new(num_shards: u32) -> Self {
        assert!(
            (MIN_SHARDS..=MAX_SHARDS).contains(&num_shards),
            "num_shards must be between {} and {}, got {}",
            MIN_SHARDS,
            MAX_SHARDS,
            num_shards
        );
        Self { num_shards }
    }
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self::new(crate::DEFAULT_SHARDS)
    }
}

/// A range of keys owned by a shard.
///
/// Used for range-based sharding schemes. Currently not used
/// by the default Jump hash router, but available for custom
/// routing strategies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShardRange {
    /// The shard that owns this range.
    pub shard_id: ShardId,
    /// Start key (inclusive).
    pub start_key: String,
    /// End key (exclusive). Empty string means no upper bound.
    pub end_key: String,
}

impl ShardRange {
    /// Create a new shard range.
    pub fn new(shard_id: ShardId, start_key: impl Into<String>, end_key: impl Into<String>) -> Self {
        Self {
            shard_id,
            start_key: start_key.into(),
            end_key: end_key.into(),
        }
    }

    /// Check if a key falls within this range.
    pub fn contains(&self, key: &str) -> bool {
        key >= self.start_key.as_str() && (self.end_key.is_empty() || key < self.end_key.as_str())
    }
}

/// Routes keys to shards using consistent hashing.
///
/// The router uses Jump consistent hash to distribute keys uniformly
/// across shards with minimal redistribution when shards change.
///
/// # Thread Safety
///
/// The router is thread-safe and can be shared across tasks.
#[derive(Debug)]
pub struct ShardRouter {
    /// Shard configuration.
    config: ShardConfig,
    /// Set of shards that are currently hosted on this node.
    local_shards: Arc<RwLock<HashSet<ShardId>>>,
}

impl ShardRouter {
    /// Create a new shard router.
    ///
    /// # Arguments
    ///
    /// * `config` - Shard configuration specifying number of shards
    pub fn new(config: ShardConfig) -> Self {
        Self {
            config,
            local_shards: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Get the shard ID for a given key.
    ///
    /// Uses Jump consistent hash for uniform distribution.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to route
    ///
    /// # Returns
    ///
    /// The shard ID that owns this key
    #[inline]
    pub fn get_shard_for_key(&self, key: &str) -> ShardId {
        JumpHash::hash(key, self.config.num_shards)
    }

    /// Get the shard IDs for a range of keys with a given prefix.
    ///
    /// Since prefix queries can span multiple shards, this returns
    /// all possible shards that might contain matching keys.
    ///
    /// For now, this returns all shards since prefix routing requires
    /// range-based sharding. Future optimization could use prefix hints.
    ///
    /// # Arguments
    ///
    /// * `_prefix` - The key prefix (currently ignored)
    ///
    /// # Returns
    ///
    /// All shard IDs (since prefix queries may span all shards)
    pub fn get_shards_for_prefix(&self, _prefix: &str) -> Vec<ShardId> {
        // Prefix queries must check all shards with consistent hashing
        (0..self.config.num_shards).collect()
    }

    /// Get the total number of shards.
    #[inline]
    pub fn num_shards(&self) -> u32 {
        self.config.num_shards
    }

    /// Register a shard as being hosted locally on this node.
    ///
    /// This is used to track which shards this node is responsible for.
    pub fn add_local_shard(&self, shard_id: ShardId) {
        assert!(shard_id < self.config.num_shards, "Invalid shard_id");
        self.local_shards.write().insert(shard_id);
    }

    /// Remove a shard from the local shard set.
    pub fn remove_local_shard(&self, shard_id: ShardId) {
        self.local_shards.write().remove(&shard_id);
    }

    /// Check if a shard is hosted locally.
    #[inline]
    pub fn is_local_shard(&self, shard_id: ShardId) -> bool {
        self.local_shards.read().contains(&shard_id)
    }

    /// Get all local shard IDs.
    pub fn local_shard_ids(&self) -> Vec<ShardId> {
        self.local_shards.read().iter().copied().collect()
    }

    /// Check if a key is owned by a local shard.
    #[inline]
    pub fn is_local_key(&self, key: &str) -> bool {
        let shard_id = self.get_shard_for_key(key);
        self.is_local_shard(shard_id)
    }

    /// Get the configuration.
    pub fn config(&self) -> &ShardConfig {
        &self.config
    }
}

impl Clone for ShardRouter {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            local_shards: Arc::clone(&self.local_shards),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_config_default() {
        let config = ShardConfig::default();
        assert_eq!(config.num_shards, crate::DEFAULT_SHARDS);
    }

    #[test]
    fn test_shard_config_custom() {
        let config = ShardConfig::new(8);
        assert_eq!(config.num_shards, 8);
    }

    #[test]
    #[should_panic(expected = "num_shards must be between")]
    fn test_shard_config_zero_panics() {
        ShardConfig::new(0);
    }

    #[test]
    #[should_panic(expected = "num_shards must be between")]
    fn test_shard_config_too_many_panics() {
        ShardConfig::new(MAX_SHARDS + 1);
    }

    #[test]
    fn test_router_key_routing() {
        let router = ShardRouter::new(ShardConfig::new(4));

        // Same key should always route to same shard
        let shard1 = router.get_shard_for_key("user:123");
        let shard2 = router.get_shard_for_key("user:123");
        assert_eq!(shard1, shard2);

        // Result should be within bounds
        assert!(shard1 < 4);
    }

    #[test]
    fn test_router_distribution() {
        let router = ShardRouter::new(ShardConfig::new(4));
        let mut counts = [0u32; 4];

        for i in 0..10000 {
            let key = format!("key_{}", i);
            let shard = router.get_shard_for_key(&key);
            counts[shard as usize] += 1;
        }

        // Each shard should have roughly 25% of keys
        for (i, &count) in counts.iter().enumerate() {
            assert!(count > 2000 && count < 3000, "Shard {} has {} keys, expected ~2500", i, count);
        }
    }

    #[test]
    fn test_local_shard_tracking() {
        let router = ShardRouter::new(ShardConfig::new(4));

        // Initially no local shards
        assert!(!router.is_local_shard(0));
        assert!(router.local_shard_ids().is_empty());

        // Add a local shard
        router.add_local_shard(1);
        assert!(!router.is_local_shard(0));
        assert!(router.is_local_shard(1));
        assert_eq!(router.local_shard_ids(), vec![1]);

        // Remove the shard
        router.remove_local_shard(1);
        assert!(!router.is_local_shard(1));
    }

    #[test]
    fn test_is_local_key() {
        let router = ShardRouter::new(ShardConfig::new(4));

        // Find a key that goes to shard 0
        let mut key_for_shard_0 = String::new();
        for i in 0..1000 {
            let key = format!("key_{}", i);
            if router.get_shard_for_key(&key) == 0 {
                key_for_shard_0 = key;
                break;
            }
        }

        // Not local until we register shard 0
        assert!(!router.is_local_key(&key_for_shard_0));

        router.add_local_shard(0);
        assert!(router.is_local_key(&key_for_shard_0));
    }

    #[test]
    fn test_prefix_routing() {
        let router = ShardRouter::new(ShardConfig::new(4));

        // Prefix queries return all shards
        let shards = router.get_shards_for_prefix("user:");
        assert_eq!(shards.len(), 4);
        assert!(shards.contains(&0));
        assert!(shards.contains(&1));
        assert!(shards.contains(&2));
        assert!(shards.contains(&3));
    }

    #[test]
    fn test_shard_range_contains() {
        let range = ShardRange::new(0, "a", "d");

        assert!(!range.contains("")); // before start
        assert!(range.contains("a")); // at start
        assert!(range.contains("b")); // middle
        assert!(range.contains("c")); // middle
        assert!(!range.contains("d")); // at end (exclusive)
        assert!(!range.contains("e")); // after end
    }

    #[test]
    fn test_shard_range_unbounded() {
        let range = ShardRange::new(0, "a", ""); // no upper bound

        assert!(!range.contains("")); // before start
        assert!(range.contains("a")); // at start
        assert!(range.contains("zzzzz")); // any key >= start
    }
}
