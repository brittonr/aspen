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
pub fn matches_tags_filter<S1: AsRef<str>, S2: AsRef<str>>(worker_tags: &[S1], required_tags: Option<&[S2]>) -> bool {
    match required_tags {
        Some(tags) if !tags.is_empty() => tags.iter().all(|req| worker_tags.iter().any(|t| t.as_ref() == req.as_ref())),
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
pub fn compute_steal_batch_size(source_queue_depth: u32, max_steal_batch: u32) -> u32 {
    assert!(max_steal_batch > 0, "WORKER: max_steal_batch must be > 0");
    // Steal at most half of what the source has, capped at max
    let result = (source_queue_depth / 2).min(max_steal_batch);
    assert!(result <= max_steal_batch, "WORKER: steal batch size must be <= max: {result} > {max_steal_batch}");
    result
}

/// Calculate a worker's available capacity (u32 version, Verus-aligned).
///
/// Returns the number of additional tasks that can be assigned.
///
/// # Arguments
///
/// * `capacity` - Maximum worker capacity
/// * `current_load` - Current load count
///
/// # Returns
///
/// Number of additional tasks that can be assigned.
///
/// # Example
///
/// ```ignore
/// assert_eq!(calculate_available_capacity(10, 3), 7);
/// assert_eq!(calculate_available_capacity(5, 5), 0);
/// ```
#[inline]
pub fn calculate_available_capacity(capacity: u32, current_load: u32) -> u32 {
    assert!(capacity > 0, "WORKER: capacity must be > 0, got {capacity}");
    let result = capacity.saturating_sub(current_load);
    assert!(result <= capacity, "WORKER: available capacity must be <= total capacity: {result} > {capacity}");
    result
}

/// Calculate a worker's available capacity (float version for load balancing).
///
/// Returns available capacity as a fraction (0.0 = no capacity, 1.0 = full capacity).
/// If the worker is unhealthy, returns 0.0.
///
/// # Arguments
///
/// * `load` - Current load as a fraction (0.0 to 1.0)
/// * `is_healthy` - Whether the worker is healthy
///
/// # Returns
///
/// Available capacity as a fraction.
#[inline]
pub fn calculate_available_capacity_f32(load: f32, is_healthy: bool) -> f32 {
    if !is_healthy {
        return 0.0;
    }
    let result = (1.0 - load.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    assert!((0.0..=1.0).contains(&result), "WORKER: available capacity must be in [0.0, 1.0], got {result}");
    result
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
    active_jobs: u32,
    max_concurrent: u32,
) -> bool {
    // Worker must be operational
    let is_operational = is_healthy && is_alive;
    // Worker must have capacity to receive work
    let has_capacity = load < steal_load_threshold && active_jobs < max_concurrent;

    is_operational && has_capacity
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
pub fn is_steal_source(is_healthy: bool, is_alive: bool, queue_depth: u32, steal_queue_threshold: u32) -> bool {
    // Worker must be operational
    let is_operational = is_healthy && is_alive;
    // Worker must have excess work to share
    let has_excess_work = queue_depth > steal_queue_threshold;

    is_operational && has_excess_work
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

// ============================================================================
// Worker Lease Operations
// ============================================================================

/// Calculate the deadline for a worker lease.
///
/// # Arguments
///
/// * `current_time_ms` - Current time in Unix milliseconds
/// * `lease_duration_ms` - Lease duration in milliseconds
///
/// # Returns
///
/// Lease deadline in Unix milliseconds.
#[inline]
pub fn calculate_worker_lease_deadline(current_time_ms: u64, lease_duration_ms: u64) -> u64 {
    current_time_ms.saturating_add(lease_duration_ms)
}

/// Check if a worker's lease has expired.
///
/// # Arguments
///
/// * `lease_deadline_ms` - Lease deadline in Unix milliseconds
/// * `current_time_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// `true` if the lease has expired.
#[inline]
pub fn is_worker_lease_expired(lease_deadline_ms: u64, current_time_ms: u64) -> bool {
    current_time_ms > lease_deadline_ms
}

/// Check if a worker is active (has valid lease and is healthy).
///
/// # Arguments
///
/// * `active` - Whether the worker is marked active
/// * `lease_deadline_ms` - Lease deadline in Unix milliseconds
/// * `current_time_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// `true` if the worker is active.
#[inline]
pub fn is_worker_active(active: bool, lease_deadline_ms: u64, current_time_ms: u64) -> bool {
    active && (current_time_ms <= lease_deadline_ms)
}

// ============================================================================
// Worker Load Operations
// ============================================================================

/// Increment worker load count.
///
/// # Arguments
///
/// * `current_load` - Current load count
///
/// # Returns
///
/// Incremented load count.
#[inline]
pub fn increment_worker_load(current_load: u32) -> u32 {
    current_load.saturating_add(1)
}

/// Decrement worker load count.
///
/// # Arguments
///
/// * `current_load` - Current load count
///
/// # Returns
///
/// Decremented load count.
#[inline]
pub fn decrement_worker_load(current_load: u32) -> u32 {
    current_load.saturating_sub(1)
}

/// Check if a worker has capacity for more tasks.
///
/// # Arguments
///
/// * `current_load` - Current load count
/// * `capacity` - Maximum capacity
///
/// # Returns
///
/// `true` if the worker has capacity.
#[inline]
pub fn worker_has_capacity(current_load: u32, capacity: u32) -> bool {
    current_load < capacity
}

/// Calculate worker load factor.
///
/// # Arguments
///
/// * `current_load` - Current load count
/// * `max_capacity` - Maximum capacity
///
/// # Returns
///
/// Load factor as a float in [0.0, 1.0].
#[inline]
pub fn calculate_worker_load_factor(current_load: u32, max_capacity: u32) -> f32 {
    if max_capacity == 0 {
        return 0.0;
    }
    let result = (current_load as f32 / max_capacity as f32).clamp(0.0, 1.0);
    assert!((0.0..=1.0).contains(&result), "WORKER: load factor must be in [0.0, 1.0], got {result}");
    assert!(
        current_load <= max_capacity || result == 1.0,
        "WORKER: load {current_load} exceeds capacity {max_capacity} but factor is not 1.0"
    );
    result
}

/// Check if a task can be assigned to a worker.
///
/// A task can be assigned if:
/// - Worker is active
/// - Worker has capacity
///
/// # Arguments
///
/// * `worker_active` - Whether the worker is active
/// * `current_load` - Current load count
/// * `capacity` - Maximum capacity
///
/// # Returns
///
/// `true` if the task can be assigned.
#[inline]
pub fn can_assign_task_to_worker(worker_active: bool, current_load: u32, capacity: u32) -> bool {
    worker_active && current_load < capacity
}

/// Maximum worker capacity constant
pub const MAX_WORKER_CAPACITY: u32 = 1000;

/// Check if worker capacity configuration is valid.
///
/// # Arguments
///
/// * `capacity` - Maximum capacity
///
/// # Returns
///
/// `true` if the capacity is valid (> 0 and <= MAX_WORKER_CAPACITY).
#[inline]
pub fn is_valid_worker_capacity(capacity: u32) -> bool {
    capacity > 0 && capacity <= MAX_WORKER_CAPACITY
}

/// Calculate time until worker lease expiration.
///
/// # Arguments
///
/// * `lease_deadline_ms` - Lease deadline in Unix milliseconds
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// Time until expiration in milliseconds (0 if already expired).
#[inline]
pub fn time_until_worker_lease_expiration(lease_deadline_ms: u64, now_ms: u64) -> u64 {
    lease_deadline_ms.saturating_sub(now_ms)
}

/// Check if lease deadline computation would overflow.
///
/// # Arguments
///
/// * `current_time_ms` - Current time
/// * `lease_duration_ms` - Lease duration
///
/// # Returns
///
/// `true` if deadline can be computed without overflow.
#[inline]
pub fn can_compute_lease_deadline(current_time_ms: u64, lease_duration_ms: u64) -> bool {
    current_time_ms <= u64::MAX - lease_duration_ms
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
        assert_eq!(compute_steal_batch_size(20u32, 10u32), 10); // Half is 10, capped at 10
        assert_eq!(compute_steal_batch_size(6u32, 10u32), 3); // Half is 3
        assert_eq!(compute_steal_batch_size(1u32, 10u32), 0); // Half is 0
        assert_eq!(compute_steal_batch_size(100u32, 5u32), 5); // Capped at max
    }

    // ========================================================================
    // Capacity Tests
    // ========================================================================

    #[test]
    fn test_available_capacity_u32() {
        assert_eq!(calculate_available_capacity(10, 3), 7);
        assert_eq!(calculate_available_capacity(5, 5), 0);
        assert_eq!(calculate_available_capacity(5, 10), 0); // saturates
    }

    #[test]
    fn test_available_capacity_f32_healthy() {
        assert!((calculate_available_capacity_f32(0.3, true) - 0.7).abs() < 0.001);
        assert!((calculate_available_capacity_f32(0.0, true) - 1.0).abs() < 0.001);
        assert!((calculate_available_capacity_f32(1.0, true) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_available_capacity_f32_unhealthy() {
        assert_eq!(calculate_available_capacity_f32(0.3, false), 0.0);
        assert_eq!(calculate_available_capacity_f32(0.0, false), 0.0);
    }

    #[test]
    fn test_available_capacity_f32_clamped() {
        // Negative load (shouldn't happen but should be handled)
        assert_eq!(calculate_available_capacity_f32(-0.5, true), 1.0);
        // Load > 1 (shouldn't happen but should be handled)
        assert_eq!(calculate_available_capacity_f32(1.5, true), 0.0);
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
        assert!(is_steal_target(true, true, 0.1, 0.2, 1u32, 10u32));
        assert!(!is_steal_target(false, true, 0.1, 0.2, 1u32, 10u32)); // Not healthy
        assert!(!is_steal_target(true, false, 0.1, 0.2, 1u32, 10u32)); // Not alive
        assert!(!is_steal_target(true, true, 0.3, 0.2, 1u32, 10u32)); // Load too high
        assert!(!is_steal_target(true, true, 0.1, 0.2, 10u32, 10u32)); // At capacity
    }

    #[test]
    fn test_is_steal_source() {
        assert!(is_steal_source(true, true, 15u32, 10u32));
        assert!(!is_steal_source(false, true, 15u32, 10u32)); // Not healthy
        assert!(!is_steal_source(true, false, 15u32, 10u32)); // Not alive
        assert!(!is_steal_source(true, true, 5u32, 10u32)); // Queue too shallow
        assert!(!is_steal_source(true, true, 10u32, 10u32)); // At threshold, not above
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
    use bolero::check;

    use super::*;

    #[test]
    fn prop_capacity_bounded() {
        check!().with_type::<(f32, bool)>().for_each(|(load, healthy)| {
            let cap = calculate_available_capacity_f32(*load, *healthy);
            assert!(cap >= 0.0 && cap <= 1.0);
        });
    }

    #[test]
    fn prop_unhealthy_zero_capacity() {
        check!().with_type::<f32>().for_each(|load| {
            assert_eq!(calculate_available_capacity_f32(*load, false), 0.0);
        });
    }

    #[test]
    fn prop_alive_check_consistent() {
        check!().with_type::<(u64, u64, u64)>().for_each(|(last, now, timeout)| {
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
