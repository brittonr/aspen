//! Pure worker coordinator computation functions.
//!
//! This module contains pure functions for distributed worker operations.
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - Uses saturating arithmetic for all calculations
//! - Time is passed explicitly (no calls to system time)
//! - Deterministic behavior for testing and verification

// ============================================================================
// Key Prefixes
// ============================================================================

/// Worker stats key prefix.
pub const WORKER_STATS_PREFIX: &str = "__worker_stats:";

/// Worker group key prefix.
pub const WORKER_GROUP_PREFIX: &str = "__worker_group:";

/// Steal hint key prefix.
pub const STEAL_HINT_PREFIX: &str = "__worker_coord:steal:";

// ============================================================================
// Key Generation
// ============================================================================

/// Generate the key for worker stats storage.
///
/// # Example
///
/// ```ignore
/// assert_eq!(worker_stats_key("worker-1"), "__worker_stats:worker-1");
/// ```
#[inline]
pub fn worker_stats_key(worker_id: &str) -> String {
    format!("{}{}", WORKER_STATS_PREFIX, worker_id)
}

/// Generate the key for worker group storage.
///
/// # Example
///
/// ```ignore
/// assert_eq!(worker_group_key("group-a"), "__worker_group:group-a");
/// ```
#[inline]
pub fn worker_group_key(group_id: &str) -> String {
    format!("{}{}", WORKER_GROUP_PREFIX, group_id)
}

/// Generate the key for a steal hint.
///
/// The key encodes both target and source worker for efficient scanning.
///
/// # Example
///
/// ```ignore
/// assert_eq!(steal_hint_key("target-1", "source-2"), "__worker_coord:steal:target-1:source-2");
/// ```
#[inline]
pub fn steal_hint_key(target_id: &str, source_id: &str) -> String {
    format!("{}{}:{}", STEAL_HINT_PREFIX, target_id, source_id)
}

/// Generate the prefix for scanning steal hints for a specific target.
///
/// # Example
///
/// ```ignore
/// assert_eq!(steal_hint_prefix("worker-1"), "__worker_coord:steal:worker-1:");
/// ```
#[inline]
pub fn steal_hint_prefix(target_id: &str) -> String {
    format!("{}{}:", STEAL_HINT_PREFIX, target_id)
}

// ============================================================================
// Worker Filtering
// ============================================================================

/// Check if a worker matches a load threshold filter.
///
/// # Arguments
///
/// * `worker_load` - Worker's current load
/// * `max_load` - Maximum load threshold (None means no filter)
///
/// # Returns
///
/// `true` if worker passes the load filter.
#[inline]
pub fn matches_load_filter(worker_load: f32, max_load: Option<f32>) -> bool {
    match max_load {
        Some(threshold) => worker_load <= threshold,
        None => true,
    }
}

/// Check if a worker matches a node ID filter.
///
/// # Arguments
///
/// * `worker_node_id` - Worker's node ID
/// * `filter_node_id` - Required node ID (None means no filter)
///
/// # Returns
///
/// `true` if worker passes the node filter.
#[inline]
pub fn matches_node_filter(worker_node_id: &str, filter_node_id: Option<&str>) -> bool {
    match filter_node_id {
        Some(node) => worker_node_id == node,
        None => true,
    }
}

/// Check if a worker matches a capability filter.
///
/// # Arguments
///
/// * `capabilities` - Worker's capabilities
/// * `required_capability` - Required capability (None means no filter)
///
/// # Returns
///
/// `true` if worker passes the capability filter.
#[inline]
pub fn matches_capability_filter<S: AsRef<str>>(capabilities: &[S], required_capability: Option<&str>) -> bool {
    match required_capability {
        Some(cap) => can_handle_job(capabilities, cap),
        None => true,
    }
}

/// Check if a worker matches tag requirements.
///
/// # Arguments
///
/// * `worker_tags` - Worker's tags
/// * `required_tags` - Required tags (None or empty means no filter)
///
/// # Returns
///
/// `true` if worker has all required tags.
#[inline]
pub fn matches_tags_filter<S1: AsRef<str>, S2: AsRef<str>>(
    worker_tags: &[S1],
    required_tags: Option<&[S2]>,
) -> bool {
    match required_tags {
        Some(tags) if !tags.is_empty() => {
            tags.iter().all(|req| worker_tags.iter().any(|t| t.as_ref() == req.as_ref()))
        }
        _ => true,
    }
}

// ============================================================================
// Steal Hint Computation
// ============================================================================

/// Compute the expiration deadline for a steal hint.
///
/// # Arguments
///
/// * `now_ms` - Current time in Unix milliseconds
/// * `ttl_ms` - Time-to-live in milliseconds
///
/// # Returns
///
/// Expiration deadline in Unix milliseconds.
#[inline]
pub fn compute_steal_hint_deadline(now_ms: u64, ttl_ms: u64) -> u64 {
    now_ms.saturating_add(ttl_ms)
}

/// Determine batch size for work stealing.
///
/// # Arguments
///
/// * `source_queue_depth` - How many items the source has queued
/// * `max_steal_batch` - Maximum items to steal in one batch
///
/// # Returns
///
/// Number of items to attempt to steal.
#[inline]
pub fn compute_steal_batch_size(source_queue_depth: usize, max_steal_batch: usize) -> usize {
    // Steal at most half of what the source has, capped at max
    (source_queue_depth / 2).min(max_steal_batch)
}

/// Calculate a worker's available capacity.
///
/// Capacity is 0 if the worker is not healthy, otherwise it's
/// the inverse of the load (1.0 - load), clamped to [0, 1].
///
/// # Arguments
///
/// * `load` - Current load (0.0 = idle, 1.0 = fully loaded)
/// * `is_healthy` - Whether the worker is healthy
///
/// # Returns
///
/// Available capacity as a float in [0.0, 1.0].
///
/// # Example
///
/// ```ignore
/// assert_eq!(calculate_available_capacity(0.3, true), 0.7);
/// assert_eq!(calculate_available_capacity(0.3, false), 0.0);
/// ```
#[inline]
pub fn calculate_available_capacity(load: f32, is_healthy: bool) -> f32 {
    if !is_healthy {
        return 0.0;
    }
    (1.0 - load).clamp(0.0, 1.0)
}

/// Check if a worker can handle a specific job type.
///
/// A worker can handle a job if:
/// - It has no capabilities (can handle anything), OR
/// - The job type is in its capabilities list
///
/// # Arguments
///
/// * `capabilities` - Worker's declared capabilities
/// * `job_type` - The job type to check
///
/// # Returns
///
/// `true` if the worker can handle the job type.
#[inline]
pub fn can_handle_job<S: AsRef<str>>(capabilities: &[S], job_type: &str) -> bool {
    capabilities.is_empty() || capabilities.iter().any(|c| c.as_ref() == job_type)
}

/// Check if a worker is alive based on heartbeat.
///
/// # Arguments
///
/// * `last_heartbeat_ms` - Last heartbeat timestamp in Unix milliseconds
/// * `now_ms` - Current time in Unix milliseconds
/// * `timeout_ms` - Heartbeat timeout in milliseconds
///
/// # Returns
///
/// `true` if the worker is alive (heartbeat within timeout).
///
/// # Tiger Style
///
/// - Uses saturating_sub to prevent underflow
#[inline]
pub fn is_worker_alive(last_heartbeat_ms: u64, now_ms: u64, timeout_ms: u64) -> bool {
    now_ms.saturating_sub(last_heartbeat_ms) < timeout_ms
}

/// Check if a steal hint has expired.
///
/// # Arguments
///
/// * `expires_at_ms` - Expiration deadline in Unix milliseconds
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// `true` if the hint has expired.
#[inline]
pub fn is_steal_hint_expired(expires_at_ms: u64, now_ms: u64) -> bool {
    now_ms > expires_at_ms
}

/// Calculate remaining TTL for a steal hint.
///
/// # Arguments
///
/// * `expires_at_ms` - Expiration deadline in Unix milliseconds
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// Remaining time in milliseconds (0 if expired).
///
/// # Tiger Style
///
/// - Uses saturating_sub to prevent underflow
#[inline]
pub fn steal_hint_remaining_ttl(expires_at_ms: u64, now_ms: u64) -> u64 {
    expires_at_ms.saturating_sub(now_ms)
}

/// Check if a worker should be considered as a steal target.
///
/// A worker is a good steal target (can receive stolen work) if:
/// - It is healthy
/// - It is alive
/// - Its load is below the threshold
/// - It has room for more jobs
///
/// # Arguments
///
/// * `is_healthy` - Whether worker is healthy
/// * `is_alive` - Whether worker is alive (heartbeat recent)
/// * `load` - Current worker load
/// * `steal_load_threshold` - Maximum load to be considered a target
/// * `active_jobs` - Current number of active jobs
/// * `max_concurrent` - Maximum concurrent jobs
///
/// # Returns
///
/// `true` if the worker is a good steal target.
#[inline]
pub fn is_steal_target(
    is_healthy: bool,
    is_alive: bool,
    load: f32,
    steal_load_threshold: f32,
    active_jobs: usize,
    max_concurrent: usize,
) -> bool {
    is_healthy && is_alive && load < steal_load_threshold && active_jobs < max_concurrent
}

/// Check if a worker should be considered as a steal source.
///
/// A worker is a good steal source (has work to give) if:
/// - It is healthy
/// - It is alive
/// - Its queue depth exceeds the threshold
///
/// # Arguments
///
/// * `is_healthy` - Whether worker is healthy
/// * `is_alive` - Whether worker is alive (heartbeat recent)
/// * `queue_depth` - Current queue depth
/// * `steal_queue_threshold` - Minimum queue depth to be a source
///
/// # Returns
///
/// `true` if the worker is a good steal source.
#[inline]
pub fn is_steal_source(is_healthy: bool, is_alive: bool, queue_depth: usize, steal_queue_threshold: usize) -> bool {
    is_healthy && is_alive && queue_depth > steal_queue_threshold
}

/// Simple hash function for consistent hashing.
///
/// # Arguments
///
/// * `s` - String to hash
///
/// # Returns
///
/// A 64-bit hash value.
#[inline]
pub fn simple_hash(s: &str) -> u64 {
    let mut hash = 0u64;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Key Generation Tests
    // ========================================================================

    #[test]
    fn test_worker_stats_key() {
        assert_eq!(worker_stats_key("worker-1"), "__worker_stats:worker-1");
        assert_eq!(worker_stats_key(""), "__worker_stats:");
    }

    #[test]
    fn test_worker_group_key() {
        assert_eq!(worker_group_key("group-a"), "__worker_group:group-a");
    }

    #[test]
    fn test_steal_hint_key() {
        assert_eq!(steal_hint_key("target-1", "source-2"), "__worker_coord:steal:target-1:source-2");
    }

    #[test]
    fn test_steal_hint_prefix() {
        assert_eq!(steal_hint_prefix("worker-1"), "__worker_coord:steal:worker-1:");
    }

    // ========================================================================
    // Filter Tests
    // ========================================================================

    #[test]
    fn test_matches_load_filter() {
        assert!(matches_load_filter(0.3, Some(0.5)));
        assert!(matches_load_filter(0.5, Some(0.5)));
        assert!(!matches_load_filter(0.6, Some(0.5)));
        assert!(matches_load_filter(1.0, None)); // No filter
    }

    #[test]
    fn test_matches_node_filter() {
        assert!(matches_node_filter("node-1", Some("node-1")));
        assert!(!matches_node_filter("node-2", Some("node-1")));
        assert!(matches_node_filter("any-node", None)); // No filter
    }

    #[test]
    fn test_matches_capability_filter() {
        let caps = vec!["email", "sms"];
        assert!(matches_capability_filter(&caps, Some("email")));
        assert!(!matches_capability_filter(&caps, Some("push")));
        assert!(matches_capability_filter(&caps, None)); // No filter
    }

    #[test]
    fn test_matches_capability_filter_empty_caps() {
        let caps: Vec<String> = vec![];
        assert!(matches_capability_filter(&caps, Some("anything"))); // Empty caps = handles all
    }

    #[test]
    fn test_matches_tags_filter() {
        let tags = vec!["region:us-east", "gpu:true"];
        assert!(matches_tags_filter(&tags, Some(&["region:us-east"][..])));
        assert!(matches_tags_filter(&tags, Some(&["region:us-east", "gpu:true"][..])));
        assert!(!matches_tags_filter(&tags, Some(&["region:eu-west"][..])));
        assert!(matches_tags_filter(&tags, None::<&[&str]>)); // No filter
        let empty: &[&str] = &[];
        assert!(matches_tags_filter(&tags, Some(empty))); // Empty filter = pass
    }

    // ========================================================================
    // Steal Hint Computation Tests
    // ========================================================================

    #[test]
    fn test_compute_steal_hint_deadline() {
        assert_eq!(compute_steal_hint_deadline(1000, 30000), 31000);
        assert_eq!(compute_steal_hint_deadline(u64::MAX, 1), u64::MAX); // Saturates
    }

    #[test]
    fn test_compute_steal_batch_size() {
        assert_eq!(compute_steal_batch_size(20, 10), 10); // Half is 10, capped at 10
        assert_eq!(compute_steal_batch_size(6, 10), 3);   // Half is 3
        assert_eq!(compute_steal_batch_size(1, 10), 0);   // Half is 0
        assert_eq!(compute_steal_batch_size(100, 5), 5);  // Capped at max
    }

    // ========================================================================
    // Capacity Tests
    // ========================================================================

    #[test]
    fn test_available_capacity_healthy() {
        assert!((calculate_available_capacity(0.3, true) - 0.7).abs() < 0.001);
        assert!((calculate_available_capacity(0.0, true) - 1.0).abs() < 0.001);
        assert!((calculate_available_capacity(1.0, true) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_available_capacity_unhealthy() {
        assert_eq!(calculate_available_capacity(0.3, false), 0.0);
        assert_eq!(calculate_available_capacity(0.0, false), 0.0);
    }

    #[test]
    fn test_available_capacity_clamped() {
        // Negative load (shouldn't happen but should be handled)
        assert_eq!(calculate_available_capacity(-0.5, true), 1.0);
        // Load > 1 (shouldn't happen but should be handled)
        assert_eq!(calculate_available_capacity(1.5, true), 0.0);
    }

    #[test]
    fn test_can_handle_job_no_capabilities() {
        let caps: Vec<String> = vec![];
        assert!(can_handle_job(&caps, "anything"));
    }

    #[test]
    fn test_can_handle_job_with_capabilities() {
        let caps = vec!["email", "sms"];
        assert!(can_handle_job(&caps, "email"));
        assert!(can_handle_job(&caps, "sms"));
        assert!(!can_handle_job(&caps, "push"));
    }

    #[test]
    fn test_is_worker_alive() {
        assert!(is_worker_alive(1000, 1500, 1000));
        assert!(!is_worker_alive(1000, 2500, 1000));
        assert!(is_worker_alive(1000, 1000, 1000)); // At timeout, still alive
    }

    #[test]
    fn test_is_worker_alive_underflow() {
        // now_ms < last_heartbeat_ms (clock skew)
        assert!(is_worker_alive(2000, 1000, 1000));
    }

    #[test]
    fn test_steal_hint_expired() {
        assert!(is_steal_hint_expired(1000, 2000));
        assert!(!is_steal_hint_expired(2000, 1000));
        assert!(!is_steal_hint_expired(1000, 1000));
    }

    #[test]
    fn test_steal_hint_remaining_ttl() {
        assert_eq!(steal_hint_remaining_ttl(2000, 1000), 1000);
        assert_eq!(steal_hint_remaining_ttl(1000, 2000), 0);
    }

    #[test]
    fn test_is_steal_target() {
        assert!(is_steal_target(true, true, 0.1, 0.2, 1, 10));
        assert!(!is_steal_target(false, true, 0.1, 0.2, 1, 10)); // Not healthy
        assert!(!is_steal_target(true, false, 0.1, 0.2, 1, 10)); // Not alive
        assert!(!is_steal_target(true, true, 0.3, 0.2, 1, 10)); // Load too high
        assert!(!is_steal_target(true, true, 0.1, 0.2, 10, 10)); // At capacity
    }

    #[test]
    fn test_is_steal_source() {
        assert!(is_steal_source(true, true, 15, 10));
        assert!(!is_steal_source(false, true, 15, 10)); // Not healthy
        assert!(!is_steal_source(true, false, 15, 10)); // Not alive
        assert!(!is_steal_source(true, true, 5, 10)); // Queue too shallow
        assert!(!is_steal_source(true, true, 10, 10)); // At threshold, not above
    }

    #[test]
    fn test_simple_hash_deterministic() {
        let h1 = simple_hash("test");
        let h2 = simple_hash("test");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_simple_hash_different() {
        let h1 = simple_hash("test1");
        let h2 = simple_hash("test2");
        assert_ne!(h1, h2);
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use super::*;
    use bolero::check;

    #[test]
    fn prop_capacity_bounded() {
        check!().with_type::<(f32, bool)>().for_each(|(load, healthy)| {
            let cap = calculate_available_capacity(*load, *healthy);
            assert!(cap >= 0.0 && cap <= 1.0);
        });
    }

    #[test]
    fn prop_unhealthy_zero_capacity() {
        check!().with_type::<f32>().for_each(|load| {
            assert_eq!(calculate_available_capacity(*load, false), 0.0);
        });
    }

    #[test]
    fn prop_alive_check_consistent() {
        check!()
            .with_type::<(u64, u64, u64)>()
            .for_each(|(last, now, timeout)| {
                let alive = is_worker_alive(*last, *now, *timeout);
                let elapsed = now.saturating_sub(*last);
                if elapsed >= *timeout {
                    assert!(!alive);
                } else {
                    assert!(alive);
                }
            });
    }

    #[test]
    fn prop_hash_deterministic() {
        check!().with_type::<String>().for_each(|s| {
            assert_eq!(simple_hash(s), simple_hash(s));
        });
    }
}
