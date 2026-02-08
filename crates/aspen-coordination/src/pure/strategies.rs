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
// Work Stealing Heuristics
// ============================================================================

/// Information about a worker for steal target ranking.
#[derive(Debug, Clone)]
pub struct WorkerStealInfo {
    /// Worker index in the original list.
    pub index: u32,
    /// Current load (0.0 = idle, 1.0 = fully loaded).
    pub load: f32,
    /// Queue depth (items waiting).
    pub queue_depth: u32,
    /// Worker's node ID for locality scoring.
    pub node_id: String,
    /// Worker's tags for affinity matching.
    pub tags: Vec<String>,
}

/// A ranked steal target with computed score.
#[derive(Debug, Clone)]
pub struct RankedStealTarget {
    /// Worker index.
    pub index: u32,
    /// Computed score (lower is better for stealing from).
    pub score: f32,
}

/// Strategy for selecting steal sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StealStrategy {
    /// Steal from highest queue depth first.
    HighestQueueDepth,
    /// Steal from highest load first.
    HighestLoad,
    /// Balanced: consider both queue depth and load.
    Balanced,
}

/// Rank workers as potential steal sources.
///
/// Returns workers sorted by their suitability as steal sources (highest
/// queue depth / load first). Only includes workers above the queue threshold.
///
/// # Arguments
///
/// * `workers` - List of worker information
/// * `source_worker_load` - Load of the worker that would steal (for filtering)
/// * `strategy` - How to prioritize steal sources
/// * `min_queue_depth` - Minimum queue depth to be considered a source
///
/// # Returns
///
/// Vector of (index, score) pairs sorted by score descending (best sources first).
///
/// # Tiger Style
///
/// - Returns empty vec for empty input
/// - Filters out workers below threshold
#[inline]
pub fn rank_steal_targets(
    workers: &[WorkerStealInfo],
    source_worker_load: f32,
    strategy: StealStrategy,
    min_queue_depth: u32,
) -> Vec<RankedStealTarget> {
    let mut targets: Vec<RankedStealTarget> = workers
        .iter()
        .filter(|w| w.queue_depth > min_queue_depth && w.load > source_worker_load)
        .map(|w| {
            let score = match strategy {
                StealStrategy::HighestQueueDepth => w.queue_depth as f32,
                StealStrategy::HighestLoad => w.load * 100.0, // Scale for comparison
                StealStrategy::Balanced => {
                    // Weighted combination: 70% queue, 30% load
                    (w.queue_depth as f32 * 0.7) + (w.load * 100.0 * 0.3)
                }
            };
            RankedStealTarget {
                index: w.index,
                score,
            }
        })
        .collect();

    // Sort descending by score (highest = best steal source)
    targets.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    targets
}

/// Compute an affinity score for a worker based on locality.
///
/// Higher scores indicate better affinity (prefer routing to this worker).
///
/// # Arguments
///
/// * `worker_tags` - Tags the worker has
/// * `affinity_key` - Optional affinity key for consistent routing
/// * `preferred_node` - Optional preferred node ID
/// * `worker_node_id` - The worker's node ID
///
/// # Returns
///
/// Affinity score in [0.0, 100.0] where higher is better.
///
/// # Scoring
///
/// - Base score: 50.0
/// - Same node as preferred: +30.0
/// - Tag match with affinity key: +20.0
#[inline]
pub fn compute_affinity_score(
    worker_tags: &[String],
    affinity_key: Option<&str>,
    preferred_node: Option<&str>,
    worker_node_id: &str,
) -> f32 {
    let mut score: f32 = 50.0;

    // Bonus for same node
    if let Some(pref) = preferred_node {
        if pref == worker_node_id {
            score += 30.0;
        }
    }

    // Bonus for tag match with affinity key
    if let Some(key) = affinity_key {
        if worker_tags.iter().any(|t| t.contains(key)) {
            score += 20.0;
        }
    }

    score.clamp(0.0, 100.0)
}

/// Determine whether to continue stealing from sources.
///
/// Stealing should stop when we've accumulated enough work or when
/// continuing would overload the target worker.
///
/// # Arguments
///
/// * `accumulated` - Number of items already stolen
/// * `batch_limit` - Maximum items to steal in one round
/// * `target_load` - Current load of the stealing worker
/// * `load_ceiling` - Maximum load to reach (stop if would exceed)
/// * `estimated_load_per_item` - Estimated load increase per stolen item
///
/// # Returns
///
/// `true` if stealing should continue, `false` if should stop.
///
/// # Tiger Style
///
/// - Uses explicit bounds checking
/// - Prevents overloading the target worker
#[inline]
pub fn should_continue_stealing(
    accumulated: u32,
    batch_limit: u32,
    target_load: f32,
    load_ceiling: f32,
    estimated_load_per_item: f32,
) -> bool {
    // Stop if we've hit the batch limit
    if accumulated >= batch_limit {
        return false;
    }

    // Stop if stealing one more would exceed the load ceiling
    let projected_load = target_load + (accumulated as f32 * estimated_load_per_item);
    if projected_load >= load_ceiling {
        return false;
    }

    true
}

/// Compute rebalance pairs for load distribution across a group.
///
/// Identifies which workers should give work to which, to achieve
/// more even load distribution.
///
/// # Arguments
///
/// * `workers` - List of workers with their current loads
/// * `target_avg_load` - Target average load for the group
/// * `load_tolerance` - How much deviation from average is acceptable
///
/// # Returns
///
/// Vector of (from_index, to_index, suggested_items) tuples.
/// Each tuple indicates work should move from worker at from_index
/// to worker at to_index.
///
/// # Tiger Style
///
/// - Returns empty vec if group is already balanced
/// - Limits returned pairs to prevent unbounded output
#[inline]
pub fn compute_rebalance_pairs(
    workers: &[WorkerStealInfo],
    target_avg_load: f32,
    load_tolerance: f32,
) -> Vec<(u32, u32, u32)> {
    const MAX_PAIRS: usize = 10;

    if workers.len() < 2 {
        return vec![];
    }

    // Identify overloaded and underloaded workers
    let mut overloaded: Vec<(u32, f32)> = Vec::new();
    let mut underloaded: Vec<(u32, f32)> = Vec::new();

    for worker in workers {
        let deviation = worker.load - target_avg_load;
        if deviation > load_tolerance {
            overloaded.push((worker.index, deviation));
        } else if deviation < -load_tolerance {
            underloaded.push((worker.index, -deviation)); // Store as positive
        }
    }

    // Sort by deviation magnitude
    overloaded.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    underloaded.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut pairs = Vec::new();

    // Pair overloaded with underloaded
    for (over_idx, over_dev) in overloaded.iter() {
        for (under_idx, under_dev) in underloaded.iter() {
            if pairs.len() >= MAX_PAIRS {
                break;
            }

            // Suggest moving items proportional to the smaller deviation
            let transfer_load = over_dev.min(*under_dev);
            let suggested_items = (transfer_load * 10.0).round() as u32; // Rough estimate

            if suggested_items > 0 {
                pairs.push((*over_idx, *under_idx, suggested_items));
            }
        }
        if pairs.len() >= MAX_PAIRS {
            break;
        }
    }

    pairs
}

/// Compute the average load across a group of workers.
///
/// # Arguments
///
/// * `loads` - Load values for each worker
///
/// # Returns
///
/// Average load, or 0.0 if empty.
#[inline]
pub fn compute_group_average_load(loads: &[f32]) -> f32 {
    if loads.is_empty() {
        return 0.0;
    }
    let sum: f32 = loads.iter().sum();
    sum / loads.len() as f32
}

/// Check if a worker group is balanced within tolerance.
///
/// # Arguments
///
/// * `loads` - Load values for each worker
/// * `tolerance` - Maximum acceptable deviation from average
///
/// # Returns
///
/// `true` if all workers are within tolerance of the average.
#[inline]
pub fn is_group_balanced(loads: &[f32], tolerance: f32) -> bool {
    if loads.len() < 2 {
        return true;
    }

    let avg = compute_group_average_load(loads);

    loads.iter().all(|&load| (load - avg).abs() <= tolerance)
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

    // ------------------------------------------------------------------------
    // Work Stealing Heuristics Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_rank_steal_targets_empty() {
        let workers: Vec<WorkerStealInfo> = vec![];
        let result = rank_steal_targets(&workers, 0.1, StealStrategy::HighestQueueDepth, 5);
        assert!(result.is_empty());
    }

    #[test]
    fn test_rank_steal_targets_filters_by_threshold() {
        let workers = vec![
            WorkerStealInfo { index: 0, load: 0.8, queue_depth: 3, node_id: "n1".to_string(), tags: vec![] },
            WorkerStealInfo { index: 1, load: 0.9, queue_depth: 10, node_id: "n1".to_string(), tags: vec![] },
            WorkerStealInfo { index: 2, load: 0.7, queue_depth: 20, node_id: "n2".to_string(), tags: vec![] },
        ];
        let result = rank_steal_targets(&workers, 0.1, StealStrategy::HighestQueueDepth, 5);
        assert_eq!(result.len(), 2); // Only workers with queue_depth > 5
        assert_eq!(result[0].index, 2); // Highest queue first
        assert_eq!(result[1].index, 1);
    }

    #[test]
    fn test_rank_steal_targets_highest_load_strategy() {
        let workers = vec![
            WorkerStealInfo { index: 0, load: 0.9, queue_depth: 10, node_id: "n1".to_string(), tags: vec![] },
            WorkerStealInfo { index: 1, load: 0.5, queue_depth: 20, node_id: "n1".to_string(), tags: vec![] },
        ];
        let result = rank_steal_targets(&workers, 0.1, StealStrategy::HighestLoad, 5);
        assert_eq!(result[0].index, 0); // Highest load first
    }

    #[test]
    fn test_compute_affinity_score_base() {
        let tags: Vec<String> = vec![];
        let score = compute_affinity_score(&tags, None, None, "node1");
        assert!((score - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_affinity_score_same_node() {
        let tags: Vec<String> = vec![];
        let score = compute_affinity_score(&tags, None, Some("node1"), "node1");
        assert!((score - 80.0).abs() < 0.01); // 50 + 30
    }

    #[test]
    fn test_compute_affinity_score_tag_match() {
        let tags = vec!["user:123".to_string()];
        let score = compute_affinity_score(&tags, Some("123"), None, "node1");
        assert!((score - 70.0).abs() < 0.01); // 50 + 20
    }

    #[test]
    fn test_compute_affinity_score_all_bonuses() {
        let tags = vec!["user:123".to_string()];
        let score = compute_affinity_score(&tags, Some("123"), Some("node1"), "node1");
        assert!((score - 100.0).abs() < 0.01); // 50 + 30 + 20
    }

    #[test]
    fn test_should_continue_stealing_under_limit() {
        assert!(should_continue_stealing(5, 10, 0.2, 0.8, 0.05));
    }

    #[test]
    fn test_should_continue_stealing_at_batch_limit() {
        assert!(!should_continue_stealing(10, 10, 0.2, 0.8, 0.05));
    }

    #[test]
    fn test_should_continue_stealing_would_overload() {
        // 0.7 + (5 * 0.05) = 0.95, which exceeds 0.8
        assert!(!should_continue_stealing(5, 10, 0.7, 0.8, 0.05));
    }

    #[test]
    fn test_compute_rebalance_pairs_empty() {
        let workers: Vec<WorkerStealInfo> = vec![];
        let result = compute_rebalance_pairs(&workers, 0.5, 0.1);
        assert!(result.is_empty());
    }

    #[test]
    fn test_compute_rebalance_pairs_single_worker() {
        let workers = vec![
            WorkerStealInfo { index: 0, load: 0.9, queue_depth: 10, node_id: "n1".to_string(), tags: vec![] },
        ];
        let result = compute_rebalance_pairs(&workers, 0.5, 0.1);
        assert!(result.is_empty());
    }

    #[test]
    fn test_compute_rebalance_pairs_imbalanced() {
        let workers = vec![
            WorkerStealInfo { index: 0, load: 0.9, queue_depth: 20, node_id: "n1".to_string(), tags: vec![] },
            WorkerStealInfo { index: 1, load: 0.1, queue_depth: 0, node_id: "n2".to_string(), tags: vec![] },
        ];
        let result = compute_rebalance_pairs(&workers, 0.5, 0.1);
        assert!(!result.is_empty());
        // Should pair overloaded worker 0 with underloaded worker 1
        assert_eq!(result[0].0, 0); // from
        assert_eq!(result[0].1, 1); // to
    }

    #[test]
    fn test_compute_group_average_load_empty() {
        let loads: Vec<f32> = vec![];
        assert_eq!(compute_group_average_load(&loads), 0.0);
    }

    #[test]
    fn test_compute_group_average_load() {
        let loads = vec![0.2, 0.4, 0.6];
        let avg = compute_group_average_load(&loads);
        assert!((avg - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_is_group_balanced_single() {
        assert!(is_group_balanced(&[0.5], 0.1));
    }

    #[test]
    fn test_is_group_balanced_balanced() {
        assert!(is_group_balanced(&[0.45, 0.50, 0.55], 0.1));
    }

    #[test]
    fn test_is_group_balanced_imbalanced() {
        assert!(!is_group_balanced(&[0.2, 0.8], 0.1));
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
