//! Pure load balancing strategy computation functions.
//!
//! This module contains pure functions for worker selection strategies.
//! All functions are deterministic and side-effect free, making them ideal for:
//!
//! - Unit testing with explicit inputs/outputs
//! - Property-based testing with Bolero
//! - Formal verification
//!
//! # Tiger Style
//!
//! - Uses saturating arithmetic for all calculations
//! - Deterministic behavior (no I/O, no system calls)
//! - Explicit types (u64, u32, not usize)
//! - No panics - all functions are total

use std::hash::Hash;
use std::hash::Hasher;

// ============================================================================
// Load Score Calculation
// ============================================================================

/// Calculate a composite load score for least-loaded selection.
///
/// Lower scores indicate a better candidate for receiving work.
///
/// # Arguments
///
/// * `load` - Current CPU/resource load (0.0 = idle, 1.0 = fully loaded)
/// * `queue_depth` - Number of jobs waiting in queue
/// * `max_concurrent` - Maximum concurrent jobs the worker can handle
/// * `load_weight` - Weight for load component (typically 0.7)
/// * `queue_weight` - Weight for queue component (typically 0.3)
///
/// # Returns
///
/// Composite score in [0.0, 1.0] where lower is better.
///
/// # Example
///
/// ```ignore
/// let score = calculate_load_score(0.3, 5, 10, 0.7, 0.3);
/// assert!(score > 0.0 && score < 1.0);
/// ```
///
/// # Tiger Style
///
/// - Clamps load to [0.0, 1.0] to handle edge cases
/// - Uses max(1) to prevent division by zero
#[inline]
pub fn calculate_load_score(
    load: f32,
    queue_depth: u32,
    max_concurrent: u32,
    load_weight: f32,
    queue_weight: f32,
) -> f32 {
    let clamped_load = load.clamp(0.0, 1.0);
    let max_concurrent_safe = max_concurrent.max(1);
    let queue_ratio = (queue_depth as f32 / max_concurrent_safe as f32).clamp(0.0, 1.0);

    let load_score = clamped_load * load_weight;
    let queue_score = queue_ratio * queue_weight;

    (load_score + queue_score).clamp(0.0, 1.0)
}

// ============================================================================
// Round Robin Selection
// ============================================================================

/// Compute the next round-robin index.
///
/// # Arguments
///
/// * `eligible_count` - Number of eligible workers
/// * `current_counter` - Current counter value
///
/// # Returns
///
/// Tuple of (selected_index, next_counter) or None if no eligible workers.
///
/// # Tiger Style
///
/// - Returns None for empty input instead of panicking
/// - Uses wrapping arithmetic for counter
#[inline]
pub fn compute_round_robin_selection(eligible_count: u32, current_counter: u32) -> Option<(u32, u32)> {
    if eligible_count == 0 {
        return None;
    }
    let selected = current_counter % eligible_count;
    let next_counter = current_counter.wrapping_add(1) % eligible_count;
    Some((selected, next_counter))
}

// ============================================================================
// Hashing Functions
// ============================================================================

/// Hash a string key to u64 using the standard library hasher.
///
/// Uses `DefaultHasher` for consistent hashing within a process.
/// Note: This hash is NOT stable across Rust versions.
///
/// # Arguments
///
/// * `key` - String to hash
///
/// # Returns
///
/// 64-bit hash value.
///
/// # Example
///
/// ```ignore
/// let h1 = hash_key("user:123");
/// let h2 = hash_key("user:123");
/// assert_eq!(h1, h2); // Deterministic within same process
/// ```
#[inline]
pub fn hash_key(key: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

/// Generate virtual node keys for consistent hashing.
///
/// Creates `replicas` virtual keys for a single worker to provide
/// better distribution on the hash ring.
///
/// # Arguments
///
/// * `worker_id` - Unique worker identifier
/// * `replica_index` - Index of this virtual node (0..replicas)
///
/// # Returns
///
/// Hash of the virtual node key.
#[inline]
pub fn compute_virtual_node_hash(worker_id: &str, replica_index: u32) -> u64 {
    // Format: "worker_id:replica_index"
    // Use a simple combination to avoid allocation
    let mut hash = 0u64;
    for byte in worker_id.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    hash = hash.wrapping_mul(31).wrapping_add(b':' as u64);
    // Add replica index bytes
    hash = hash.wrapping_mul(31).wrapping_add((replica_index >> 24) as u64);
    hash = hash.wrapping_mul(31).wrapping_add(((replica_index >> 16) & 0xFF) as u64);
    hash = hash.wrapping_mul(31).wrapping_add(((replica_index >> 8) & 0xFF) as u64);
    hash = hash.wrapping_mul(31).wrapping_add((replica_index & 0xFF) as u64);
    hash
}

/// Find node in a sorted hash ring using binary search.
///
/// The ring is a sorted slice of (hash, index) pairs. This function
/// finds the first node with hash >= key_hash, wrapping to the first
/// node if needed.
///
/// # Arguments
///
/// * `ring` - Sorted slice of (hash, worker_index) pairs
/// * `key_hash` - Hash of the key to look up
///
/// # Returns
///
/// Worker index, or None if ring is empty.
///
/// # Tiger Style
///
/// - Returns None for empty ring instead of panicking
#[inline]
pub fn lookup_hash_ring(ring: &[(u64, u32)], key_hash: u64) -> Option<u32> {
    if ring.is_empty() {
        return None;
    }

    // Binary search for the first node with hash >= key_hash
    match ring.binary_search_by_key(&key_hash, |&(h, _)| h) {
        Ok(idx) => Some(ring[idx].1),
        Err(idx) => {
            // Wrap around to the first node if we're past the end
            let idx = if idx >= ring.len() { 0 } else { idx };
            Some(ring[idx].1)
        }
    }
}

// ============================================================================
// Work Stealing Thresholds
// ============================================================================

/// Check if a worker is idle enough to receive stolen work.
///
/// # Arguments
///
/// * `load` - Current worker load
/// * `steal_threshold` - Maximum load to be considered idle
/// * `queue_depth` - Current queue depth
///
/// # Returns
///
/// `true` if the worker can receive stolen work.
#[inline]
pub fn is_worker_idle_for_stealing(load: f32, steal_threshold: f32, queue_depth: u32) -> bool {
    load < steal_threshold && queue_depth == 0
}

// ============================================================================
// Selection Logic
// ============================================================================

/// Result of selecting from a scored list with preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionResult {
    /// Selected the lowest-scored worker
    Best(u32),
    /// Selected a worker matching the preferred node
    Preferred(u32),
    /// No eligible workers
    None,
}

/// Select best worker from scored candidates, with optional preference.
///
/// For critical priority, always returns the best (lowest score).
/// For other priorities, prefers a worker on the preferred node if available.
///
/// # Arguments
///
/// * `scored` - Slice of (worker_index, score) pairs, sorted by score ascending
/// * `preferred_node_indices` - Indices of workers on the preferred node (if any)
/// * `is_critical` - Whether this is a critical priority job
///
/// # Returns
///
/// Selection result indicating which worker was chosen and why.
///
/// # Tiger Style
///
/// - Returns enum variant instead of magic values
/// - Handles empty input gracefully
#[inline]
pub fn select_from_scored(
    scored: &[(u32, f32)],
    preferred_node_indices: &[u32],
    is_critical: bool,
) -> SelectionResult {
    if scored.is_empty() {
        return SelectionResult::None;
    }

    let best_idx = scored[0].0;

    // Critical jobs always get the best worker
    if is_critical {
        return SelectionResult::Best(best_idx);
    }

    // Try to find a worker on the preferred node
    if !preferred_node_indices.is_empty() {
        for &idx in preferred_node_indices {
            if scored.iter().any(|(i, _)| *i == idx) {
                return SelectionResult::Preferred(idx);
            }
        }
    }

    SelectionResult::Best(best_idx)
}

// ============================================================================
// Metrics Helpers
// ============================================================================

/// Compute running average with a new sample.
///
/// # Arguments
///
/// * `current_avg` - Current average value
/// * `count` - Number of samples so far (before adding new one)
/// * `new_value` - New sample to add
///
/// # Returns
///
/// Updated average, or new_value if count was 0.
///
/// # Tiger Style
///
/// - Uses saturating arithmetic
/// - Handles count=0 edge case
#[inline]
pub fn compute_running_average(current_avg: u64, count: u64, new_value: u64) -> u64 {
    if count == 0 {
        return new_value;
    }
    // avg = (avg * count + new) / (count + 1)
    // To prevent overflow: avg + (new - avg) / (count + 1)
    let new_count = count.saturating_add(1);
    if new_value >= current_avg {
        current_avg.saturating_add((new_value - current_avg) / new_count)
    } else {
        current_avg.saturating_sub((current_avg - new_value) / new_count)
    }
}

// ============================================================================
// Worker Filtering (Pure Predicates)
// ============================================================================

/// Check if a worker matches required tags.
///
/// # Arguments
///
/// * `worker_tags` - Tags the worker has
/// * `required_tags` - Tags that are required
///
/// # Returns
///
/// `true` if worker has all required tags (or no tags are required).
#[inline]
pub fn worker_matches_tags<S1: AsRef<str>, S2: AsRef<str>>(
    worker_tags: &[S1],
    required_tags: &[S2],
) -> bool {
    required_tags.iter().all(|req| {
        worker_tags.iter().any(|t| t.as_ref() == req.as_ref())
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------
    // Load Score Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_load_score_basic() {
        // Low load, empty queue -> low score
        let score = calculate_load_score(0.1, 0, 10, 0.7, 0.3);
        assert!(score < 0.2);

        // High load, full queue -> high score
        let score = calculate_load_score(0.9, 10, 10, 0.7, 0.3);
        assert!(score > 0.8);
    }

    #[test]
    fn test_load_score_weights() {
        // With only load weight
        let score = calculate_load_score(0.5, 5, 10, 1.0, 0.0);
        assert!((score - 0.5).abs() < 0.01);

        // With only queue weight
        let score = calculate_load_score(0.5, 5, 10, 0.0, 1.0);
        assert!((score - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_load_score_edge_cases() {
        // Zero max_concurrent (should not panic)
        let score = calculate_load_score(0.5, 5, 0, 0.7, 0.3);
        assert!(score >= 0.0 && score <= 1.0);

        // Negative load (clamped)
        let score = calculate_load_score(-0.5, 0, 10, 0.7, 0.3);
        assert!(score >= 0.0);

        // Load > 1 (clamped)
        let score = calculate_load_score(1.5, 0, 10, 0.7, 0.3);
        assert!(score <= 1.0);
    }

    // ------------------------------------------------------------------------
    // Round Robin Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_round_robin_basic() {
        let result = compute_round_robin_selection(3, 0);
        assert_eq!(result, Some((0, 1)));

        let result = compute_round_robin_selection(3, 1);
        assert_eq!(result, Some((1, 2)));

        let result = compute_round_robin_selection(3, 2);
        assert_eq!(result, Some((2, 0)));
    }

    #[test]
    fn test_round_robin_empty() {
        assert_eq!(compute_round_robin_selection(0, 0), None);
        assert_eq!(compute_round_robin_selection(0, 100), None);
    }

    #[test]
    fn test_round_robin_wraparound() {
        // Counter larger than eligible count
        let result = compute_round_robin_selection(3, 100);
        assert_eq!(result, Some((1, 2))); // 100 % 3 = 1
    }

    // ------------------------------------------------------------------------
    // Hash Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_hash_key_deterministic() {
        let h1 = hash_key("test");
        let h2 = hash_key("test");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_key_different() {
        let h1 = hash_key("test1");
        let h2 = hash_key("test2");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_virtual_node_hash_deterministic() {
        let h1 = compute_virtual_node_hash("worker1", 5);
        let h2 = compute_virtual_node_hash("worker1", 5);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_virtual_node_hash_different_replicas() {
        let h1 = compute_virtual_node_hash("worker1", 0);
        let h2 = compute_virtual_node_hash("worker1", 1);
        assert_ne!(h1, h2);
    }

    // ------------------------------------------------------------------------
    // Hash Ring Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_lookup_hash_ring_empty() {
        let ring: Vec<(u64, u32)> = vec![];
        assert_eq!(lookup_hash_ring(&ring, 12345), None);
    }

    #[test]
    fn test_lookup_hash_ring_single() {
        let ring = vec![(1000, 0)];
        assert_eq!(lookup_hash_ring(&ring, 500), Some(0));
        assert_eq!(lookup_hash_ring(&ring, 1500), Some(0)); // Wraps
    }

    #[test]
    fn test_lookup_hash_ring_multiple() {
        let ring = vec![(1000, 0), (2000, 1), (3000, 2)];
        assert_eq!(lookup_hash_ring(&ring, 500), Some(0));   // Before first
        assert_eq!(lookup_hash_ring(&ring, 1500), Some(1));  // Between first and second
        assert_eq!(lookup_hash_ring(&ring, 2500), Some(2));  // Between second and third
        assert_eq!(lookup_hash_ring(&ring, 3500), Some(0));  // After last, wraps
    }

    #[test]
    fn test_lookup_hash_ring_exact_match() {
        let ring = vec![(1000, 0), (2000, 1), (3000, 2)];
        assert_eq!(lookup_hash_ring(&ring, 2000), Some(1));
    }

    // ------------------------------------------------------------------------
    // Work Stealing Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_worker_idle_for_stealing() {
        assert!(is_worker_idle_for_stealing(0.1, 0.3, 0));
        assert!(!is_worker_idle_for_stealing(0.4, 0.3, 0)); // Load too high
        assert!(!is_worker_idle_for_stealing(0.1, 0.3, 5)); // Has queued work
    }

    // ------------------------------------------------------------------------
    // Selection Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_select_from_scored_empty() {
        let scored: Vec<(u32, f32)> = vec![];
        assert_eq!(select_from_scored(&scored, &[], false), SelectionResult::None);
    }

    #[test]
    fn test_select_from_scored_critical() {
        let scored = vec![(0, 0.1), (1, 0.5), (2, 0.9)];
        let preferred = vec![2]; // Worst score but preferred

        // Critical should pick best regardless
        assert_eq!(select_from_scored(&scored, &preferred, true), SelectionResult::Best(0));
    }

    #[test]
    fn test_select_from_scored_preferred() {
        let scored = vec![(0, 0.1), (1, 0.5), (2, 0.9)];
        let preferred = vec![1];

        // Non-critical should pick preferred
        assert_eq!(select_from_scored(&scored, &preferred, false), SelectionResult::Preferred(1));
    }

    #[test]
    fn test_select_from_scored_no_preference() {
        let scored = vec![(0, 0.1), (1, 0.5), (2, 0.9)];

        // No preference -> best
        assert_eq!(select_from_scored(&scored, &[], false), SelectionResult::Best(0));
    }

    // ------------------------------------------------------------------------
    // Running Average Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_running_average_first_sample() {
        assert_eq!(compute_running_average(0, 0, 100), 100);
    }

    #[test]
    fn test_running_average_subsequent() {
        // avg=100, count=1, new=200 -> (100 + 200) / 2 = 150
        let avg = compute_running_average(100, 1, 200);
        assert_eq!(avg, 150);
    }

    #[test]
    fn test_running_average_decrease() {
        // avg=100, count=1, new=0 -> (100 + 0) / 2 = 50
        let avg = compute_running_average(100, 1, 0);
        assert_eq!(avg, 50);
    }

    // ------------------------------------------------------------------------
    // Tag Matching Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_worker_matches_tags_empty_required() {
        let tags = vec!["region:us-east", "gpu:true"];
        let required: Vec<&str> = vec![];
        assert!(worker_matches_tags(&tags, &required));
    }

    #[test]
    fn test_worker_matches_tags_all_match() {
        let tags = vec!["region:us-east", "gpu:true"];
        let required = vec!["region:us-east"];
        assert!(worker_matches_tags(&tags, &required));
    }

    #[test]
    fn test_worker_matches_tags_missing() {
        let tags = vec!["region:us-east"];
        let required = vec!["gpu:true"];
        assert!(!worker_matches_tags(&tags, &required));
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use super::*;
    use bolero::check;

    #[test]
    fn prop_load_score_bounded() {
        check!()
            .with_type::<(f32, u32, u32, f32, f32)>()
            .for_each(|(load, queue, max, lw, qw)| {
                let score = calculate_load_score(*load, *queue, *max, *lw, *qw);
                assert!(score >= 0.0 && score <= 1.0);
            });
    }

    #[test]
    fn prop_round_robin_bounded() {
        check!()
            .with_type::<(u32, u32)>()
            .for_each(|(count, counter)| {
                if let Some((selected, next)) = compute_round_robin_selection(*count, *counter) {
                    assert!(selected < *count);
                    assert!(next < *count);
                }
            });
    }

    #[test]
    fn prop_hash_ring_lookup_valid() {
        check!()
            .with_type::<(Vec<(u64, u32)>, u64)>()
            .for_each(|(ring, key)| {
                let mut sorted_ring = ring.clone();
                sorted_ring.sort_by_key(|&(h, _)| h);

                if let Some(idx) = lookup_hash_ring(&sorted_ring, *key) {
                    // Index should exist in the ring
                    assert!(sorted_ring.iter().any(|(_, i)| *i == idx));
                }
            });
    }

    #[test]
    fn prop_virtual_node_hash_deterministic() {
        check!()
            .with_type::<(String, u32)>()
            .for_each(|(worker_id, replica)| {
                let h1 = compute_virtual_node_hash(worker_id, *replica);
                let h2 = compute_virtual_node_hash(worker_id, *replica);
                assert_eq!(h1, h2);
            });
    }

    #[test]
    fn prop_running_average_no_overflow() {
        check!()
            .with_type::<(u64, u64, u64)>()
            .for_each(|(avg, count, new)| {
                // Should never panic
                let _ = compute_running_average(*avg, *count, *new);
            });
    }
}
