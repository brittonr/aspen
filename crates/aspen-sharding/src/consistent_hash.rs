//! Consistent hashing implementations for key-to-shard mapping.
//!
//! This module provides consistent hash functions that map keys to shard IDs
//! with minimal redistribution when shards are added or removed.
//!
//! # Jump Consistent Hash
//!
//! The primary implementation uses Jump Consistent Hash, which provides:
//! - O(1) computation (actually O(log n) but very fast)
//! - Perfect balance: each bucket gets 1/n of the keys
//! - Minimal redistribution: only 1/n keys move when adding a bucket
//! - No memory overhead (unlike ring-based approaches)
//!
//! Reference: "A Fast, Minimal Memory, Consistent Hash Algorithm"
//! by John Lamping and Eric Veach, Google 2014
//!
//! # Tiger Style
//!
//! - Pure functions with deterministic output
//! - No dynamic allocation
//! - Explicit bounds checking

use std::hash::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

/// Jump consistent hash implementation.
///
/// Maps a key to one of `num_buckets` buckets using the Jump hash algorithm.
/// The algorithm is deterministic and produces uniform distribution.
#[derive(Debug, Clone, Copy, Default)]
pub struct JumpHash;

impl JumpHash {
    /// Hash a key to a bucket in the range [0, num_buckets).
    ///
    /// # Arguments
    ///
    /// * `key` - The key to hash (will be hashed to u64 first)
    /// * `num_buckets` - The number of buckets (must be > 0)
    ///
    /// # Returns
    ///
    /// A bucket index in [0, num_buckets)
    ///
    /// # Panics
    ///
    /// Panics if `num_buckets` is 0.
    ///
    /// # Example
    ///
    /// ```
    /// use aspen_sharding::JumpHash;
    ///
    /// let bucket = JumpHash::hash("user:123", 4);
    /// assert!(bucket < 4);
    /// ```
    pub fn hash<K: Hash>(key: K, num_buckets: u32) -> u32 {
        assert!(num_buckets > 0, "num_buckets must be > 0");

        // Hash the key to u64
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let key_hash = hasher.finish();

        Self::hash_u64(key_hash, num_buckets)
    }

    /// Hash a pre-hashed u64 key to a bucket.
    ///
    /// This is useful when you've already computed a hash and want to
    /// avoid re-hashing.
    ///
    /// # Arguments
    ///
    /// * `key` - Pre-hashed key as u64
    /// * `num_buckets` - The number of buckets (must be > 0)
    ///
    /// # Returns
    ///
    /// A bucket index in [0, num_buckets)
    #[inline]
    pub fn hash_u64(mut key: u64, num_buckets: u32) -> u32 {
        // Jump consistent hash algorithm
        // Based on the Google paper, optimized for Rust
        let num_buckets_i64 = i64::from(num_buckets);
        let mut bucket = 0_u32;
        let mut j: i64 = 0;

        while j < num_buckets_i64 {
            if let Ok(next_bucket) = u32::try_from(j) {
                bucket = next_bucket;
            }
            // Linear congruential generator for deterministic random sequence
            key = key.wrapping_mul(2862933555777941757).wrapping_add(1);
            // Use upper bits of key for floating point calculation
            j = ((f64::from(bucket.saturating_add(1)))
                * (((1_i64) << 31) as f64 / ((key >> 33).wrapping_add(1) as f64))) as i64;
        }

        bucket
    }

    /// Check if a key would move when changing from old_buckets to new_buckets.
    ///
    /// This is useful for planning shard migrations.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to check
    /// * `old_buckets` - Previous number of buckets
    /// * `new_buckets` - New number of buckets
    ///
    /// # Returns
    ///
    /// `true` if the key would be assigned to a different bucket
    pub fn would_move<K: Hash>(key: K, old_buckets: u32, new_buckets: u32) -> bool {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let key_hash = hasher.finish();

        Self::hash_u64(key_hash, old_buckets) != Self::hash_u64(key_hash, new_buckets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic() {
        // Same key should always hash to same bucket
        let bucket1 = JumpHash::hash("test_key", 10);
        let bucket2 = JumpHash::hash("test_key", 10);
        assert_eq!(bucket1, bucket2);
    }

    #[test]
    fn test_bounds() {
        // Result should always be within bounds
        for num_buckets in 1..100 {
            for i in 0..1000 {
                let key = format!("key_{}", i);
                let bucket = JumpHash::hash(&key, num_buckets);
                assert!(bucket < num_buckets, "bucket {} >= num_buckets {}", bucket, num_buckets);
            }
        }
    }

    #[test]
    fn test_distribution() {
        const NUM_BUCKETS: u32 = 4;
        const NUM_BUCKETS_USIZE: usize = 4;
        const NUM_KEYS: u32 = 10_000;
        const EXPECTED_KEYS_PER_BUCKET: u32 = 2_500;
        const MAX_DEVIATION_KEYS: u32 = 250;

        // Test that distribution is roughly uniform
        let mut counts = [0_u32; NUM_BUCKETS_USIZE];

        for sample_index in 0..NUM_KEYS {
            let key = format!("key_{}", sample_index);
            let bucket = JumpHash::hash(&key, NUM_BUCKETS);
            if let Ok(bucket_index) = usize::try_from(bucket) {
                counts[bucket_index] = counts[bucket_index].saturating_add(1);
            }
        }

        assert_eq!(counts.iter().copied().sum::<u32>(), NUM_KEYS);

        // Each bucket should have roughly 25% of keys (within 10%)
        for (bucket_index, &count) in counts.iter().enumerate() {
            let deviation = (count as i32 - EXPECTED_KEYS_PER_BUCKET as i32).unsigned_abs();
            assert!(
                deviation <= MAX_DEVIATION_KEYS,
                "Bucket {} has {} keys, expected ~{} (deviation {})",
                bucket_index,
                count,
                EXPECTED_KEYS_PER_BUCKET,
                deviation
            );
        }
    }

    #[test]
    fn test_minimal_redistribution() {
        const OLD_BUCKETS: u32 = 4;
        const NEW_BUCKETS: u32 = 5;
        const NUM_KEYS: u32 = 10_000;
        const TOLERANCE: f64 = 0.05;

        // When adding a bucket, at most 1/(n+1) keys should move
        let mut moved = 0_u32;

        for sample_index in 0..NUM_KEYS {
            let key = format!("key_{}", sample_index);
            if JumpHash::would_move(&key, OLD_BUCKETS, NEW_BUCKETS) {
                moved = moved.saturating_add(1);
            }
        }

        assert!(moved <= NUM_KEYS, "moved keys must remain bounded by sample count");

        // Expected: ~20% should move (1/5 = 0.2)
        let expected_move_rate = 1.0 / NEW_BUCKETS as f64;
        let actual_move_rate = moved as f64 / NUM_KEYS as f64;

        assert!(
            (actual_move_rate - expected_move_rate).abs() < TOLERANCE,
            "Move rate {} differs from expected {} by more than {}",
            actual_move_rate,
            expected_move_rate,
            TOLERANCE
        );
    }

    #[test]
    #[should_panic(expected = "num_buckets must be > 0")]
    fn test_zero_buckets_panics() {
        JumpHash::hash("key", 0);
    }

    #[test]
    fn test_single_bucket() {
        // With one bucket, everything goes to bucket 0
        for i in 0..100 {
            let key = format!("key_{}", i);
            assert_eq!(JumpHash::hash(&key, 1), 0);
        }
    }
}
