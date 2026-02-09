//! Pure fencing token validation functions.
//!
//! This module provides cross-primitive validation for fencing tokens,
//! split-brain detection, and quorum checks. These functions help ensure
//! consistency across distributed coordination primitives.
//!
//! # Tiger Style
//!
//! - All calculations use saturating arithmetic
//! - Deterministic behavior (no I/O, no system calls)
//! - Explicit types (u64, u32, not usize)
//! - No panics - all functions are total

use std::collections::HashMap;

// ============================================================================
// Fencing Token Validation
// ============================================================================

/// Result of fencing token validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FencingValidation {
    /// All tokens are valid and consistent.
    Valid,
    /// A token is stale (lower than expected).
    StaleToken {
        /// The stale token value.
        token: u64,
        /// The minimum expected token.
        min_expected: u64,
    },
    /// Tokens are inconsistent across primitives.
    Inconsistent,
}

/// Validate that a fencing token is not stale.
///
/// A token is stale if it's lower than the minimum expected token,
/// which indicates that a newer holder has taken ownership.
///
/// # Arguments
///
/// * `token` - The fencing token to validate
/// * `min_expected` - The minimum expected token value
///
/// # Returns
///
/// `true` if the token is valid (>= min_expected).
#[inline]
pub fn is_token_valid(token: u64, min_expected: u64) -> bool {
    token >= min_expected
}

/// Validate fencing tokens across multiple primitives.
///
/// Ensures that the tokens are monotonically increasing and consistent.
/// This is useful when a client holds multiple locks/leases and needs
/// to ensure they're all still valid.
///
/// # Arguments
///
/// * `lock_token` - Fencing token from a distributed lock
/// * `election_token` - Fencing token from leader election
/// * `rwlock_token` - Fencing token from read-write lock
/// * `min_lock_token` - Minimum expected lock token
/// * `min_election_token` - Minimum expected election token
/// * `min_rwlock_token` - Minimum expected rwlock token
///
/// # Returns
///
/// Validation result indicating whether tokens are valid.
#[inline]
pub fn validate_consistent_fencing_tokens(
    lock_token: u64,
    election_token: u64,
    rwlock_token: u64,
    min_lock_token: u64,
    min_election_token: u64,
    min_rwlock_token: u64,
) -> FencingValidation {
    // Check each token against its minimum
    if lock_token < min_lock_token {
        return FencingValidation::StaleToken {
            token: lock_token,
            min_expected: min_lock_token,
        };
    }

    if election_token < min_election_token {
        return FencingValidation::StaleToken {
            token: election_token,
            min_expected: min_election_token,
        };
    }

    if rwlock_token < min_rwlock_token {
        return FencingValidation::StaleToken {
            token: rwlock_token,
            min_expected: min_rwlock_token,
        };
    }

    FencingValidation::Valid
}

// ============================================================================
// Split-Brain Detection
// ============================================================================

/// Result of split-brain check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplitBrainCheck {
    /// No split-brain detected.
    Healthy,
    /// Split-brain detected: multiple leaders or inconsistent tokens.
    SplitBrain {
        /// Token that indicates a conflict.
        conflicting_token: u64,
        /// Source of the conflicting token.
        source: String,
    },
}

/// Check for split-brain conditions based on observed tokens.
///
/// Split-brain occurs when multiple nodes believe they are the leader
/// or hold exclusive resources. This manifests as observing tokens
/// that are higher than our own (someone else thinks they're the leader)
/// or tokens from different nodes that both claim leadership.
///
/// # Arguments
///
/// * `observed_tokens` - Map of node_id -> token from other nodes
/// * `my_token` - Our current fencing token
/// * `my_node_id` - Our node identifier
///
/// # Returns
///
/// Check result indicating whether split-brain is detected.
#[inline]
pub fn check_for_split_brain(
    observed_tokens: &HashMap<String, u64>,
    my_token: u64,
    my_node_id: &str,
) -> SplitBrainCheck {
    for (node_id, &token) in observed_tokens {
        // Skip our own token
        if node_id == my_node_id {
            continue;
        }

        // If another node has a token >= ours, there may be a split-brain
        // (we both think we're the leader)
        if token >= my_token {
            return SplitBrainCheck::SplitBrain {
                conflicting_token: token,
                source: node_id.clone(),
            };
        }
    }

    SplitBrainCheck::Healthy
}

/// Check if we should step down based on observed tokens.
///
/// A node should step down if it observes a token higher than its own,
/// indicating that another node has been elected leader.
///
/// # Arguments
///
/// * `observed_tokens` - Map of node_id -> token from other nodes
/// * `my_token` - Our current fencing token
/// * `my_node_id` - Our node identifier
///
/// # Returns
///
/// `true` if we should step down.
#[inline]
pub fn should_step_down(observed_tokens: &HashMap<String, u64>, my_token: u64, my_node_id: &str) -> bool {
    for (node_id, &token) in observed_tokens {
        if node_id != my_node_id && token > my_token {
            return true;
        }
    }
    false
}

// ============================================================================
// Quorum Validation
// ============================================================================

/// Compute the quorum threshold for a cluster.
///
/// Quorum is the minimum number of nodes required for consensus.
/// Uses the formula: (total_nodes / 2) + 1
///
/// # Arguments
///
/// * `total_nodes` - Total number of nodes in the cluster
///
/// # Returns
///
/// Minimum nodes required for quorum.
///
/// # Example
///
/// ```ignore
/// assert_eq!(compute_quorum_threshold(5), 3);
/// assert_eq!(compute_quorum_threshold(4), 3);
/// assert_eq!(compute_quorum_threshold(3), 2);
/// assert_eq!(compute_quorum_threshold(1), 1);
/// ```
#[inline]
pub fn compute_quorum_threshold(total_nodes: u32) -> u32 {
    if total_nodes == 0 {
        return 0;
    }
    (total_nodes / 2) + 1
}

/// Check if we have quorum with the given number of healthy nodes.
///
/// # Arguments
///
/// * `total_nodes` - Total number of nodes in the cluster
/// * `healthy_nodes` - Number of nodes that are healthy/reachable
///
/// # Returns
///
/// `true` if we have quorum.
#[inline]
pub fn has_quorum(total_nodes: u32, healthy_nodes: u32) -> bool {
    healthy_nodes >= compute_quorum_threshold(total_nodes)
}

/// Check if a cluster partition would lose quorum.
///
/// Given a potential partition, determine if our side would
/// maintain quorum.
///
/// # Arguments
///
/// * `total_nodes` - Total nodes in the cluster
/// * `nodes_on_our_side` - Nodes in our partition
///
/// # Returns
///
/// `true` if we would maintain quorum.
#[inline]
pub fn partition_maintains_quorum(total_nodes: u32, nodes_on_our_side: u32) -> bool {
    has_quorum(total_nodes, nodes_on_our_side)
}

// ============================================================================
// Failover Decision
// ============================================================================

/// Result of failover decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailoverDecision {
    /// Continue with current leader.
    Continue,
    /// Trigger failover to a new leader.
    TriggerFailover,
    /// Wait before making a decision (transient state).
    Wait,
}

/// Determine whether to trigger failover based on leader health.
///
/// Failover should be triggered when:
/// - The leader heartbeat has been missing for too long
/// - Multiple consecutive failures have occurred
///
/// # Arguments
///
/// * `heartbeat_age_ms` - Time since last leader heartbeat (milliseconds)
/// * `election_timeout_ms` - Election timeout threshold (milliseconds)
/// * `consecutive_failures` - Number of consecutive heartbeat failures
/// * `max_failures` - Maximum failures before triggering failover
///
/// # Returns
///
/// Decision on whether to trigger failover.
#[inline]
pub fn should_trigger_failover(
    heartbeat_age_ms: u64,
    election_timeout_ms: u64,
    consecutive_failures: u32,
    max_failures: u32,
) -> FailoverDecision {
    // Immediate failover if too many consecutive failures
    if consecutive_failures >= max_failures {
        return FailoverDecision::TriggerFailover;
    }

    // Failover if heartbeat age exceeds election timeout
    if heartbeat_age_ms > election_timeout_ms {
        return FailoverDecision::TriggerFailover;
    }

    // Wait if heartbeat is stale but not yet at timeout
    let warning_threshold = election_timeout_ms / 2;
    if heartbeat_age_ms > warning_threshold {
        return FailoverDecision::Wait;
    }

    FailoverDecision::Continue
}

/// Compute the next election timeout with jitter.
///
/// Adds randomized jitter to prevent thundering herd during elections.
///
/// # Arguments
///
/// * `base_timeout_ms` - Base election timeout
/// * `jitter_factor` - Jitter as fraction of base (e.g., 0.2 for 20%)
/// * `random_value` - Random value in [0, 1] for jitter calculation
///
/// # Returns
///
/// Timeout with jitter applied.
#[inline]
pub fn compute_election_timeout_with_jitter(base_timeout_ms: u64, jitter_factor: f32, random_value: f32) -> u64 {
    let jitter_range = (base_timeout_ms as f32 * jitter_factor) as u64;
    let jitter = (jitter_range as f32 * random_value.clamp(0.0, 1.0)) as u64;
    base_timeout_ms.saturating_add(jitter)
}

// ============================================================================
// Lease Validation
// ============================================================================

/// Check if a lease is still valid.
///
/// # Arguments
///
/// * `lease_expires_at_ms` - Lease expiration time (Unix ms)
/// * `now_ms` - Current time (Unix ms)
/// * `grace_period_ms` - Grace period before lease is considered truly expired
///
/// # Returns
///
/// `true` if the lease is still valid (within grace period).
#[inline]
pub fn is_lease_valid(lease_expires_at_ms: u64, now_ms: u64, grace_period_ms: u64) -> bool {
    let effective_expiry = lease_expires_at_ms.saturating_add(grace_period_ms);
    now_ms <= effective_expiry
}

/// Compute when a lease should be renewed.
///
/// Leases should be renewed before they expire, typically at 50-75% of their TTL.
///
/// # Arguments
///
/// * `lease_acquired_at_ms` - When the lease was acquired (Unix ms)
/// * `lease_ttl_ms` - Lease TTL in milliseconds
/// * `renew_at_fraction` - Fraction of TTL at which to renew (e.g., 0.5)
///
/// # Returns
///
/// Time at which the lease should be renewed (Unix ms).
#[inline]
pub fn compute_lease_renew_time(lease_acquired_at_ms: u64, lease_ttl_ms: u64, renew_at_fraction: f32) -> u64 {
    let renew_after_ms = (lease_ttl_ms as f32 * renew_at_fraction.clamp(0.0, 1.0)) as u64;
    lease_acquired_at_ms.saturating_add(renew_after_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Fencing Token Tests
    // ========================================================================

    #[test]
    fn test_is_token_valid() {
        assert!(is_token_valid(10, 5));
        assert!(is_token_valid(5, 5));
        assert!(!is_token_valid(4, 5));
    }

    #[test]
    fn test_validate_consistent_fencing_tokens_all_valid() {
        let result = validate_consistent_fencing_tokens(10, 20, 30, 5, 15, 25);
        assert_eq!(result, FencingValidation::Valid);
    }

    #[test]
    fn test_validate_consistent_fencing_tokens_stale_lock() {
        let result = validate_consistent_fencing_tokens(4, 20, 30, 5, 15, 25);
        assert!(matches!(result, FencingValidation::StaleToken {
            token: 4,
            min_expected: 5
        }));
    }

    #[test]
    fn test_validate_consistent_fencing_tokens_stale_election() {
        let result = validate_consistent_fencing_tokens(10, 14, 30, 5, 15, 25);
        assert!(matches!(result, FencingValidation::StaleToken {
            token: 14,
            min_expected: 15
        }));
    }

    // ========================================================================
    // Split-Brain Tests
    // ========================================================================

    #[test]
    fn test_check_for_split_brain_healthy() {
        let mut observed = HashMap::new();
        observed.insert("node-2".to_string(), 5u64);
        observed.insert("node-3".to_string(), 3u64);

        let result = check_for_split_brain(&observed, 10, "node-1");
        assert_eq!(result, SplitBrainCheck::Healthy);
    }

    #[test]
    fn test_check_for_split_brain_detected() {
        let mut observed = HashMap::new();
        observed.insert("node-2".to_string(), 15u64);

        let result = check_for_split_brain(&observed, 10, "node-1");
        assert!(matches!(result, SplitBrainCheck::SplitBrain {
            conflicting_token: 15,
            ..
        }));
    }

    #[test]
    fn test_check_for_split_brain_ignores_self() {
        let mut observed = HashMap::new();
        observed.insert("node-1".to_string(), 20u64); // Self with higher token

        let result = check_for_split_brain(&observed, 10, "node-1");
        assert_eq!(result, SplitBrainCheck::Healthy);
    }

    #[test]
    fn test_should_step_down() {
        let mut observed = HashMap::new();
        observed.insert("node-2".to_string(), 15u64);

        assert!(should_step_down(&observed, 10, "node-1"));
        assert!(!should_step_down(&observed, 20, "node-1"));
    }

    // ========================================================================
    // Quorum Tests
    // ========================================================================

    #[test]
    fn test_compute_quorum_threshold() {
        assert_eq!(compute_quorum_threshold(0), 0);
        assert_eq!(compute_quorum_threshold(1), 1);
        assert_eq!(compute_quorum_threshold(2), 2);
        assert_eq!(compute_quorum_threshold(3), 2);
        assert_eq!(compute_quorum_threshold(4), 3);
        assert_eq!(compute_quorum_threshold(5), 3);
        assert_eq!(compute_quorum_threshold(7), 4);
    }

    #[test]
    fn test_has_quorum() {
        assert!(has_quorum(5, 3));
        assert!(has_quorum(5, 4));
        assert!(!has_quorum(5, 2));
    }

    #[test]
    fn test_partition_maintains_quorum() {
        // In a 5-node cluster, 3 nodes maintains quorum
        assert!(partition_maintains_quorum(5, 3));
        assert!(!partition_maintains_quorum(5, 2));
    }

    // ========================================================================
    // Failover Tests
    // ========================================================================

    #[test]
    fn test_failover_continue() {
        let result = should_trigger_failover(1000, 10000, 0, 3);
        assert_eq!(result, FailoverDecision::Continue);
    }

    #[test]
    fn test_failover_wait() {
        // 6000ms is > 50% of 10000ms timeout
        let result = should_trigger_failover(6000, 10000, 1, 3);
        assert_eq!(result, FailoverDecision::Wait);
    }

    #[test]
    fn test_failover_trigger_timeout() {
        let result = should_trigger_failover(15000, 10000, 1, 3);
        assert_eq!(result, FailoverDecision::TriggerFailover);
    }

    #[test]
    fn test_failover_trigger_failures() {
        let result = should_trigger_failover(1000, 10000, 3, 3);
        assert_eq!(result, FailoverDecision::TriggerFailover);
    }

    #[test]
    fn test_election_timeout_with_jitter() {
        let timeout = compute_election_timeout_with_jitter(1000, 0.2, 0.5);
        assert!(timeout >= 1000 && timeout <= 1200);

        // Edge cases
        let no_jitter = compute_election_timeout_with_jitter(1000, 0.2, 0.0);
        assert_eq!(no_jitter, 1000);

        let max_jitter = compute_election_timeout_with_jitter(1000, 0.2, 1.0);
        assert_eq!(max_jitter, 1200);
    }

    // ========================================================================
    // Lease Tests
    // ========================================================================

    #[test]
    fn test_is_lease_valid() {
        // Lease expires at 1000, no grace period
        assert!(is_lease_valid(1000, 900, 0));
        assert!(is_lease_valid(1000, 1000, 0));
        assert!(!is_lease_valid(1000, 1001, 0));

        // With 100ms grace period
        assert!(is_lease_valid(1000, 1050, 100));
        assert!(!is_lease_valid(1000, 1101, 100));
    }

    #[test]
    fn test_compute_lease_renew_time() {
        // Lease acquired at 1000, TTL 10000, renew at 50%
        let renew_time = compute_lease_renew_time(1000, 10000, 0.5);
        assert_eq!(renew_time, 6000); // 1000 + (10000 * 0.5)

        // Renew at 75%
        let renew_time = compute_lease_renew_time(1000, 10000, 0.75);
        assert_eq!(renew_time, 8500); // 1000 + (10000 * 0.75)
    }

    #[test]
    fn test_compute_lease_renew_time_clamps_fraction() {
        // Fraction > 1 should be clamped
        let renew_time = compute_lease_renew_time(1000, 10000, 1.5);
        assert_eq!(renew_time, 11000); // 1000 + 10000

        // Fraction < 0 should be clamped
        let renew_time = compute_lease_renew_time(1000, 10000, -0.5);
        assert_eq!(renew_time, 1000); // 1000 + 0
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use bolero::check;

    use super::*;

    #[test]
    fn prop_quorum_is_majority() {
        check!().with_type::<u32>().for_each(|total| {
            if *total > 0 {
                let quorum = compute_quorum_threshold(*total);
                assert!(quorum > *total / 2, "Quorum must be > half");
                assert!(quorum <= *total, "Quorum must be <= total");
            }
        });
    }

    #[test]
    fn prop_lease_renew_before_expiry() {
        check!().with_type::<(u64, u64, f32)>().for_each(|(acquired, ttl, fraction)| {
            let renew = compute_lease_renew_time(*acquired, *ttl, *fraction);
            let expires = acquired.saturating_add(*ttl);
            // Renew time should be before or at expiry
            assert!(renew <= expires);
        });
    }

    #[test]
    fn prop_election_timeout_bounded() {
        check!().with_type::<(u64, f32)>().for_each(|(base, random)| {
            let timeout = compute_election_timeout_with_jitter(*base, 0.2, *random);
            // Timeout should be at least base
            assert!(timeout >= *base);
            // Timeout should not exceed base + 20% (jitter factor)
            let max = base.saturating_add((*base as f32 * 0.2) as u64);
            assert!(timeout <= max);
        });
    }
}
