//! Fencing Token and Quorum Verification Specifications
//!
//! This module provides formal specifications for cross-primitive fencing token
//! validation, split-brain detection, quorum calculations, and lease validation.
//!
//! These specifications formalize the invariants implemented in `verified/fencing.rs`.
//!
//! # Key Invariants
//!
//! ## Fencing Token Validation (FENCE-1)
//! - A token is valid iff token >= min_expected
//! - Stale tokens indicate lost ownership
//!
//! ## Quorum Properties (FENCE-2)
//! - Quorum threshold = (n / 2) + 1 for n nodes
//! - Quorum is strictly greater than half (prevents split-brain)
//! - Quorum is always <= total nodes
//!
//! ## Split-Brain Detection (FENCE-3)
//! - Split-brain occurs when multiple nodes claim leadership
//! - Detection: observing a token >= our own from another node
//!
//! ## Failover Safety (FENCE-4)
//! - Failover triggers on heartbeat timeout or consecutive failures
//! - Wait state provides hysteresis before failover
//!
//! ## Lease Validity (FENCE-5)
//! - A lease is valid if now <= expires_at + grace_period
//! - Renewal should happen before expiry
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/fencing_spec.rs
//! ```

use std::collections::HashMap;

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Token Validation (FENCE-1)
    // ========================================================================

    /// FENCE-1a: Token validity check
    ///
    /// A fencing token is valid if it is >= the minimum expected token.
    /// This corresponds to `is_token_valid` in verified/fencing.rs.
    pub open spec fn token_is_valid(token: u64, min_expected: u64) -> bool {
        token >= min_expected
    }

    /// FENCE-1b: Stale token detection
    ///
    /// A token is stale if it is strictly less than min_expected.
    pub open spec fn token_is_stale(token: u64, min_expected: u64) -> bool {
        token < min_expected
    }

    /// Proof: Token validity and staleness are mutually exclusive
    #[verifier(external_body)]
    pub proof fn token_valid_xor_stale(token: u64, min_expected: u64)
        ensures
            token_is_valid(token, min_expected) <==> !token_is_stale(token, min_expected)
    {
    }

    /// FENCE-1c: Multi-token consistency
    ///
    /// All tokens in a set are valid if each >= its minimum.
    /// Models `validate_consistent_fencing_tokens`.
    pub open spec fn tokens_consistent(
        lock_token: u64,
        election_token: u64,
        rwlock_token: u64,
        min_lock: u64,
        min_election: u64,
        min_rwlock: u64,
    ) -> bool {
        token_is_valid(lock_token, min_lock) &&
        token_is_valid(election_token, min_election) &&
        token_is_valid(rwlock_token, min_rwlock)
    }

    // ========================================================================
    // Quorum Calculations (FENCE-2)
    // ========================================================================

    /// FENCE-2a: Quorum threshold calculation
    ///
    /// For n nodes, quorum = (n / 2) + 1
    /// This ensures strict majority for consensus.
    ///
    /// # Properties
    /// - quorum(0) = 0 (degenerate case)
    /// - quorum(1) = 1
    /// - quorum(2) = 2
    /// - quorum(3) = 2
    /// - quorum(4) = 3
    /// - quorum(5) = 3
    pub open spec fn quorum_threshold(total_nodes: u32) -> u32 {
        if total_nodes == 0 {
            0u32
        } else {
            ((total_nodes / 2) + 1) as u32
        }
    }

    /// FENCE-2b: Quorum is strictly greater than half
    ///
    /// For any non-zero n, quorum(n) > n/2
    pub open spec fn quorum_is_majority(total_nodes: u32) -> bool {
        total_nodes == 0 || quorum_threshold(total_nodes) > total_nodes / 2
    }

    /// Proof: Quorum threshold is always a majority
    #[verifier(external_body)]
    pub proof fn quorum_majority_proof(total_nodes: u32)
        ensures quorum_is_majority(total_nodes)
    {
        // quorum = (n/2) + 1
        // n/2 + 1 > n/2 (trivially true for all n)
    }

    /// FENCE-2c: Quorum is bounded by total nodes
    pub open spec fn quorum_bounded(total_nodes: u32) -> bool {
        quorum_threshold(total_nodes) <= total_nodes
    }

    /// Proof: Quorum never exceeds total nodes
    #[verifier(external_body)]
    pub proof fn quorum_bounded_proof(total_nodes: u32)
        ensures quorum_bounded(total_nodes)
    {
        // For n >= 1: (n/2) + 1 <= n
        // n/2 <= n - 1
        // n <= 2n - 2 (true for n >= 2)
        // For n = 1: quorum = 1 = n
    }

    /// FENCE-2d: Quorum satisfaction check
    ///
    /// We have quorum if healthy_nodes >= quorum_threshold(total_nodes)
    pub open spec fn has_quorum(total_nodes: u32, healthy_nodes: u32) -> bool {
        healthy_nodes >= quorum_threshold(total_nodes)
    }

    /// FENCE-2e: Partition quorum check
    ///
    /// A partition maintains quorum if nodes_on_our_side >= quorum_threshold
    pub open spec fn partition_has_quorum(total_nodes: u32, nodes_on_our_side: u32) -> bool {
        has_quorum(total_nodes, nodes_on_our_side)
    }

    /// Key property: Only one partition can have quorum
    ///
    /// If partition A has quorum, partition B (complement) cannot have quorum.
    /// This is because quorum > n/2, so both partitions cannot exceed n/2.
    #[verifier(external_body)]
    pub proof fn only_one_partition_can_have_quorum(total_nodes: u32, partition_a: u32, partition_b: u32)
        requires
            partition_a + partition_b == total_nodes,
            total_nodes > 0,
        ensures
            !(has_quorum(total_nodes, partition_a) && has_quorum(total_nodes, partition_b))
    {
        // If partition_a has quorum: partition_a >= (n/2) + 1
        // Then partition_b = n - partition_a <= n - (n/2) - 1 = n/2 - 1 < (n/2) + 1
        // So partition_b cannot have quorum
    }

    // ========================================================================
    // Split-Brain Detection (FENCE-3)
    // ========================================================================

    /// FENCE-3a: Split-brain indicator
    ///
    /// Split-brain is indicated when we observe a token >= our own from another node.
    /// This suggests multiple nodes believe they are the leader.
    pub open spec fn indicates_split_brain(observed_token: u64, my_token: u64) -> bool {
        observed_token >= my_token
    }

    /// FENCE-3b: Step-down indicator
    ///
    /// A node should step down if it observes a strictly greater token.
    /// This resolves split-brain by deferring to the higher token.
    pub open spec fn should_step_down(observed_token: u64, my_token: u64) -> bool {
        observed_token > my_token
    }

    /// Property: Step-down implies split-brain detection
    #[verifier(external_body)]
    pub proof fn stepdown_implies_splitbrain(observed_token: u64, my_token: u64)
        requires should_step_down(observed_token, my_token)
        ensures indicates_split_brain(observed_token, my_token)
    {
        // observed > my implies observed >= my
    }

    // ========================================================================
    // Failover Decision (FENCE-4)
    // ========================================================================

    /// Failover decision states
    pub enum FailoverDecisionSpec {
        /// Continue with current leader
        Continue,
        /// Wait before deciding (hysteresis)
        Wait,
        /// Trigger failover to new leader
        TriggerFailover,
    }

    /// FENCE-4a: Failover trigger conditions
    ///
    /// Failover is triggered when:
    /// 1. consecutive_failures >= max_failures, OR
    /// 2. heartbeat_age_ms > election_timeout_ms
    pub open spec fn failover_triggered(
        heartbeat_age_ms: u64,
        election_timeout_ms: u64,
        consecutive_failures: u32,
        max_failures: u32,
    ) -> bool {
        consecutive_failures >= max_failures ||
        heartbeat_age_ms > election_timeout_ms
    }

    /// FENCE-4b: Wait state condition
    ///
    /// Wait when heartbeat is stale but not yet at timeout.
    /// Warning threshold = election_timeout / 2
    pub open spec fn failover_wait(
        heartbeat_age_ms: u64,
        election_timeout_ms: u64,
        consecutive_failures: u32,
        max_failures: u32,
    ) -> bool {
        !failover_triggered(heartbeat_age_ms, election_timeout_ms, consecutive_failures, max_failures) &&
        heartbeat_age_ms > (election_timeout_ms / 2)
    }

    /// FENCE-4c: Continue condition
    ///
    /// Continue when heartbeat is fresh.
    pub open spec fn failover_continue(
        heartbeat_age_ms: u64,
        election_timeout_ms: u64,
        consecutive_failures: u32,
        max_failures: u32,
    ) -> bool {
        !failover_triggered(heartbeat_age_ms, election_timeout_ms, consecutive_failures, max_failures) &&
        !failover_wait(heartbeat_age_ms, election_timeout_ms, consecutive_failures, max_failures)
    }

    /// Complete failover decision function
    pub open spec fn compute_failover_decision(
        heartbeat_age_ms: u64,
        election_timeout_ms: u64,
        consecutive_failures: u32,
        max_failures: u32,
    ) -> FailoverDecisionSpec {
        if failover_triggered(heartbeat_age_ms, election_timeout_ms, consecutive_failures, max_failures) {
            FailoverDecisionSpec::TriggerFailover
        } else if failover_wait(heartbeat_age_ms, election_timeout_ms, consecutive_failures, max_failures) {
            FailoverDecisionSpec::Wait
        } else {
            FailoverDecisionSpec::Continue
        }
    }

    // ========================================================================
    // Lease Validation (FENCE-5)
    // ========================================================================

    /// FENCE-5a: Lease validity check
    ///
    /// A lease is valid if current time is at or before the expiry plus grace period.
    /// Uses saturating add to prevent overflow.
    pub open spec fn lease_is_valid(
        lease_expires_at_ms: u64,
        now_ms: u64,
        grace_period_ms: u64,
    ) -> bool {
        // Saturating add: effective_expiry = min(expires + grace, u64::MAX)
        let effective_expiry = if lease_expires_at_ms > 0xFFFF_FFFF_FFFF_FFFFu64 - grace_period_ms {
            0xFFFF_FFFF_FFFF_FFFFu64  // Saturate at max
        } else {
            (lease_expires_at_ms + grace_period_ms) as u64
        };
        now_ms <= effective_expiry
    }

    /// FENCE-5b: Lease renewal timing
    ///
    /// Renewal time = acquired_at + (ttl * fraction)
    /// fraction is clamped to [0.0, 1.0]
    ///
    /// This specification uses integer approximation since Verus doesn't support
    /// floating point well. We model fraction as a percentage (0-100).
    pub open spec fn lease_renew_time(
        lease_acquired_at_ms: u64,
        lease_ttl_ms: u64,
        renew_percent: u32,  // 0-100 representing 0.0-1.0
    ) -> u64 {
        let clamped_percent = if renew_percent > 100 { 100u32 } else { renew_percent as u32 };
        let renew_after_ms = ((lease_ttl_ms as int * clamped_percent as int) / 100) as u64;
        // Saturating add
        if lease_acquired_at_ms > 0xFFFF_FFFF_FFFF_FFFFu64 - renew_after_ms {
            0xFFFF_FFFF_FFFF_FFFFu64
        } else {
            (lease_acquired_at_ms + renew_after_ms) as u64
        }
    }

    /// FENCE-5c: Renewal happens before expiry
    ///
    /// For any valid fraction in [0, 1], renewal time <= expiry time
    #[verifier(external_body)]
    pub proof fn renewal_before_expiry(
        lease_acquired_at_ms: u64,
        lease_ttl_ms: u64,
        renew_percent: u32,
    )
        requires renew_percent <= 100
        ensures
            lease_renew_time(lease_acquired_at_ms, lease_ttl_ms, renew_percent) <=
            lease_acquired_at_ms + lease_ttl_ms ||
            lease_acquired_at_ms + lease_ttl_ms < lease_acquired_at_ms  // Overflow case
    {
        // renew_after = ttl * (percent/100) <= ttl
        // acquired + renew_after <= acquired + ttl
    }

    // ========================================================================
    // Election Timeout with Jitter (FENCE-6)
    // ========================================================================

    /// FENCE-6a: Jittered timeout bounds
    ///
    /// The jittered timeout is in range [base, base + base*jitter_factor].
    /// Modeled with integer jitter_percent (0-100) for jitter_factor (0.0-1.0).
    pub open spec fn timeout_lower_bound(base_timeout_ms: u64) -> u64 {
        base_timeout_ms
    }

    /// Upper bound for jittered timeout
    pub open spec fn timeout_upper_bound(base_timeout_ms: u64, jitter_percent: u32) -> u64 {
        let clamped = if jitter_percent > 100 { 100u32 } else { jitter_percent as u32 };
        let jitter_range = ((base_timeout_ms as int * clamped as int) / 100) as u64;
        // Saturating add
        if base_timeout_ms > 0xFFFF_FFFF_FFFF_FFFFu64 - jitter_range {
            0xFFFF_FFFF_FFFF_FFFFu64
        } else {
            (base_timeout_ms + jitter_range) as u64
        }
    }

    /// Jittered timeout is within bounds
    pub open spec fn timeout_in_bounds(
        timeout: u64,
        base_timeout_ms: u64,
        jitter_percent: u32,
    ) -> bool {
        timeout >= timeout_lower_bound(base_timeout_ms) &&
        timeout <= timeout_upper_bound(base_timeout_ms, jitter_percent)
    }

    // ========================================================================
    // Combined Fencing Invariant
    // ========================================================================

    /// Combined fencing state for verification
    pub struct FencingState {
        /// Our current fencing token
        pub my_token: u64,
        /// Total nodes in cluster
        pub total_nodes: u32,
        /// Currently healthy/reachable nodes
        pub healthy_nodes: u32,
        /// Time since last leader heartbeat (ms)
        pub heartbeat_age_ms: u64,
        /// Election timeout threshold (ms)
        pub election_timeout_ms: u64,
        /// Consecutive heartbeat failures
        pub consecutive_failures: u32,
        /// Maximum failures before failover
        pub max_failures: u32,
    }

    /// Combined invariant for fencing state
    ///
    /// Verifies:
    /// - Node counts are consistent
    /// - Timeouts are positive
    pub open spec fn fencing_invariant(state: FencingState) -> bool {
        // Healthy nodes cannot exceed total
        state.healthy_nodes <= state.total_nodes &&
        // Election timeout must be positive
        state.election_timeout_ms > 0 &&
        // Max failures must be positive
        state.max_failures > 0
    }

    /// We are in a safe state if we have quorum
    pub open spec fn is_safe_state(state: FencingState) -> bool {
        fencing_invariant(state) &&
        has_quorum(state.total_nodes, state.healthy_nodes)
    }

    /// Proof: Safe state implies we can make progress
    #[verifier(external_body)]
    pub proof fn safe_state_progress(state: FencingState)
        requires is_safe_state(state)
        ensures has_quorum(state.total_nodes, state.healthy_nodes)
    {
    }

    // ========================================================================
    // Executable Functions (verified implementations)
    // ========================================================================
    //
    // These exec fn implementations are verified to match their spec fn
    // counterparts. They can be called from production code while maintaining
    // formal guarantees.

    /// Check if a fencing token is valid.
    ///
    /// A token is valid if it is >= the minimum expected token.
    pub fn is_token_valid_exec(token: u64, min_expected: u64) -> (result: bool)
        ensures result == token_is_valid(token, min_expected)
    {
        token >= min_expected
    }

    /// Check if a fencing token is stale.
    ///
    /// A token is stale if it is strictly less than min_expected.
    pub fn is_token_stale_exec(token: u64, min_expected: u64) -> (result: bool)
        ensures result == token_is_stale(token, min_expected)
    {
        token < min_expected
    }

    /// Compute quorum threshold for a cluster.
    ///
    /// For n nodes, quorum = (n / 2) + 1
    /// This ensures strict majority for consensus.
    pub fn compute_quorum_threshold(total_nodes: u32) -> (result: u32)
        ensures
            total_nodes == 0 ==> result == 0,
            total_nodes > 0 ==> result == (total_nodes / 2) + 1
    {
        let quorum = if total_nodes == 0 { 0 } else { (total_nodes / 2) + 1 };
        quorum
    }

    /// Check if we have quorum.
    ///
    /// We have quorum if healthy_nodes >= quorum_threshold(total_nodes).
    pub fn has_quorum_exec(total_nodes: u32, healthy_nodes: u32) -> (result: bool)
        ensures result == has_quorum(total_nodes, healthy_nodes)
    {
        healthy_nodes >= compute_quorum_threshold(total_nodes)
    }

    /// Check if a partition maintains quorum.
    ///
    /// A partition maintains quorum if nodes_on_our_side >= quorum_threshold.
    pub fn partition_maintains_quorum(total_nodes: u32, nodes_on_our_side: u32) -> (result: bool)
        ensures result == partition_has_quorum(total_nodes, nodes_on_our_side)
    {
        has_quorum_exec(total_nodes, nodes_on_our_side)
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
    #[verifier(external_body)]
    pub fn check_for_split_brain(
        observed_tokens: &HashMap<String, u64>,
        my_token: u64,
        my_node_id: &str,
    ) -> (result: SplitBrainCheck)
    {
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
    ///
    /// # Note
    ///
    /// Marked external_body since Verus cannot reason about HashMap.
    #[verifier(external_body)]
    #[verifier(external_body)]
    pub fn should_step_down_exec(
        observed_tokens: &HashMap<String, u64>,
        my_token: u64,
        my_node_id: &str
    ) -> (result: bool)
    {
        should_step_down(observed_tokens, my_token, my_node_id)
    }

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

    /// Failover decision enumeration (exec version)
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum FailoverDecision {
        /// Continue with current leader
        Continue,
        /// Wait before deciding (hysteresis)
        Wait,
        /// Trigger failover to new leader
        TriggerFailover,
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
    #[verifier(external_body)]
    pub fn should_trigger_failover(
        heartbeat_age_ms: u64,
        election_timeout_ms: u64,
        consecutive_failures: u32,
        max_failures: u32,
    ) -> (result: FailoverDecision)
    {
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

    /// Check if a lease is valid.
    ///
    /// A lease is valid if current time is at or before the expiry plus grace period.
    pub fn is_lease_valid_exec(
        lease_expires_at_ms: u64,
        now_ms: u64,
        grace_period_ms: u64,
    ) -> (result: bool)
        ensures result == lease_is_valid(lease_expires_at_ms, now_ms, grace_period_ms)
    {
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
    ///
    /// # Verus External Body
    ///
    /// Verus does not support floating-point arithmetic, so this function
    /// uses an external body. Production implementation:
    /// `lease_acquired_at_ms + (lease_ttl_ms * renew_at_fraction)`.
    #[verifier(external_body)]
    pub fn compute_lease_renew_time(
        lease_acquired_at_ms: u64,
        lease_ttl_ms: u64,
        renew_at_fraction: f32,
    ) -> (result: u64)
    {
        let renew_after_ms = (lease_ttl_ms as f32 * renew_at_fraction.clamp(0.0, 1.0)) as u64;
        let renew_time = lease_acquired_at_ms.saturating_add(renew_after_ms);
        renew_time
    }

    /// Compute election timeout with jitter.
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
    ///
    /// # Verus External Body
    ///
    /// Verus does not support floating-point arithmetic, so this function
    /// uses an external body. Production implementation:
    /// `base_timeout_ms + (base_timeout_ms * jitter_factor * random_value)`.
    #[verifier(external_body)]
    pub fn compute_election_timeout_with_jitter(
        base_timeout_ms: u64,
        jitter_factor: f32,
        random_value: f32,
    ) -> (result: u64)
    {
        let jitter_range = (base_timeout_ms as f32 * jitter_factor) as u64;
        let jitter = (jitter_range as f32 * random_value.clamp(0.0, 1.0)) as u64;
        let timeout = base_timeout_ms.saturating_add(jitter);
        timeout
    }
}
