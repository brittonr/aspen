//! Membership State Machine Model
//!
//! Abstract state model for formal verification of cluster membership operations.
//!
//! # Key Invariants
//!
//! 1. **MEMB-1: Quorum Safety**: Removing a voter maintains quorum
//! 2. **MEMB-2: Voter Limit**: Number of voters <= max_voters
//! 3. **MEMB-3: Learner Lag Bound**: Promoted learners must be caught up
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-raft/verus/membership_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Spec Functions
    // ========================================================================

    /// Specification for quorum calculation
    ///
    /// Quorum is the minimum number of nodes required for consensus:
    /// (voters / 2) + 1 (majority)
    pub open spec fn quorum_spec(voter_count: int) -> int {
        (voter_count / 2) + 1
    }

    /// Specification for safe voter removal
    ///
    /// A voter can be safely removed if the remaining count >= quorum
    pub open spec fn can_remove_safely_spec(voter_count: int) -> bool {
        voter_count > 0 && (voter_count - 1) >= quorum_spec(voter_count)
    }

    /// Specification for learner lag check
    pub open spec fn learner_caught_up_spec(lag: u64, threshold: u64) -> bool {
        lag <= threshold
    }

    // ========================================================================
    // Executable Functions (verified implementations)
    // ========================================================================
    //
    // These exec fn implementations are verified to match their spec fn
    // counterparts. They can be called from production code while maintaining
    // formal guarantees.

    /// Calculate the replication lag for a learner node.
    ///
    /// The lag is the number of log entries the learner is behind the leader.
    /// Uses saturating subtraction to handle the case where the learner might
    /// temporarily appear ahead (due to timing or metrics collection).
    ///
    /// # Arguments
    ///
    /// * `leader_last_log` - The leader's last log index
    /// * `learner_matched` - The learner's matched log index
    ///
    /// # Returns
    ///
    /// Number of log entries the learner is behind (0 if caught up or ahead).
    pub fn compute_learner_lag(leader_last_log: u64, learner_matched: u64) -> (result: u64)
        ensures
            learner_matched >= leader_last_log ==> result == 0,
            learner_matched < leader_last_log ==> result == leader_last_log - learner_matched
    {
        leader_last_log.saturating_sub(learner_matched)
    }

    /// Check if a learner's lag is within acceptable threshold.
    ///
    /// # Arguments
    ///
    /// * `lag` - The learner's replication lag (entries behind leader)
    /// * `threshold` - Maximum acceptable lag
    ///
    /// # Returns
    ///
    /// `true` if the learner is caught up enough (lag <= threshold).
    pub fn is_learner_caught_up(lag: u64, threshold: u64) -> (result: bool)
        ensures result == (lag <= threshold)
    {
        lag <= threshold
    }

    /// Calculate the quorum size for a cluster.
    ///
    /// Quorum is the minimum number of nodes required for consensus,
    /// calculated as (voters / 2) + 1 (majority).
    ///
    /// # Arguments
    ///
    /// * `voter_count` - Total number of voters in the cluster
    ///
    /// # Returns
    ///
    /// Minimum number of nodes required for quorum.
    pub fn calculate_quorum_size(voter_count: u32) -> (result: u32)
        ensures result == (voter_count / 2) + 1
    {
        (voter_count / 2) + 1
    }

    /// Check if removing a node would maintain quorum.
    ///
    /// When replacing a failed voter, we need to verify the cluster
    /// will still have quorum after the removal.
    ///
    /// # Arguments
    ///
    /// * `current_voter_count` - Current number of voters
    ///
    /// # Returns
    ///
    /// `true` if removing one voter would still leave quorum.
    pub fn can_remove_voter_safely(current_voter_count: u32) -> (result: bool)
        ensures
            current_voter_count == 0 ==> result == false,
            current_voter_count > 0 ==> result == (
                // Inline calculate_quorum_size formula: (voter_count / 2) + 1
                (current_voter_count as int - 1) >= ((current_voter_count as int / 2) + 1)
            )
    {
        if current_voter_count == 0 {
            return false;
        }
        let remaining = current_voter_count - 1;
        let quorum = calculate_quorum_size(current_voter_count);
        remaining >= quorum
    }

    /// Check if adding a voter would exceed the maximum limit.
    ///
    /// # Arguments
    ///
    /// * `current_voter_count` - Current number of voters
    /// * `max_voters` - Maximum allowed voters
    ///
    /// # Returns
    ///
    /// `true` if adding one more voter would exceed the limit.
    pub fn would_exceed_max_voters(current_voter_count: u32, max_voters: u32) -> (result: bool)
        ensures result == (current_voter_count >= max_voters)
    {
        current_voter_count >= max_voters
    }

    /// Check if a cluster has quorum with given healthy node count.
    ///
    /// # Arguments
    ///
    /// * `total_voters` - Total number of voters in the cluster
    /// * `healthy_count` - Number of healthy/responsive voters
    ///
    /// # Returns
    ///
    /// `true` if the healthy count meets quorum requirements.
    pub fn has_quorum(total_voters: u32, healthy_count: u32) -> (result: bool)
        ensures result == (healthy_count as int >= quorum_spec(total_voters as int))
    {
        healthy_count >= calculate_quorum_size(total_voters)
    }

    /// Calculate required lag threshold for learner promotion.
    ///
    /// # Arguments
    ///
    /// * `base_threshold` - Base threshold in log entries
    /// * `safety_margin` - Additional safety margin
    ///
    /// # Returns
    ///
    /// Effective threshold for learner promotion eligibility.
    pub fn calculate_promotion_threshold(base_threshold: u64, safety_margin: u64) -> (result: u64)
        ensures
            base_threshold as int + safety_margin as int <= u64::MAX as int ==>
                result == base_threshold + safety_margin,
            base_threshold as int + safety_margin as int > u64::MAX as int ==>
                result == u64::MAX
    {
        base_threshold.saturating_add(safety_margin)
    }

    /// Check if a learner is eligible for promotion.
    ///
    /// Combines lag check with additional health/stability criteria.
    ///
    /// # Arguments
    ///
    /// * `leader_last_log` - The leader's last log index
    /// * `learner_matched` - The learner's matched log index
    /// * `threshold` - Maximum acceptable lag for promotion
    /// * `is_healthy` - Whether the learner is currently healthy
    ///
    /// # Returns
    ///
    /// `true` if the learner is eligible for promotion.
    pub fn is_promotion_eligible(
        leader_last_log: u64,
        learner_matched: u64,
        threshold: u64,
        is_healthy: bool,
    ) -> (result: bool)
        ensures result == (
            is_healthy &&
            // Inline compute_learner_lag: saturating_sub
            (if learner_matched >= leader_last_log { 0u64 } else { (leader_last_log - learner_matched) as u64 }) <= threshold
        )
    {
        is_healthy && compute_learner_lag(leader_last_log, learner_matched) <= threshold
    }

    /// Calculate minimum cluster size for fault tolerance.
    ///
    /// Returns the minimum number of voters needed to tolerate `f` failures.
    /// Formula: 2f + 1 (to maintain majority after f failures)
    ///
    /// # Arguments
    ///
    /// * `fault_tolerance` - Number of failures to tolerate
    ///
    /// # Returns
    ///
    /// Minimum cluster size, saturating at u32::MAX.
    #[inline]
    pub fn min_cluster_size_for_tolerance(fault_tolerance: u32) -> (result: u32)
        ensures
            fault_tolerance as int * 2 + 1 <= u32::MAX as int ==>
                result == (fault_tolerance * 2 + 1) as u32,
            fault_tolerance as int * 2 + 1 > u32::MAX as int ==>
                result == u32::MAX
    {
        let doubled = if fault_tolerance > u32::MAX / 2 {
            u32::MAX
        } else {
            fault_tolerance * 2
        };
        if doubled > u32::MAX - 1 {
            u32::MAX
        } else {
            doubled + 1
        }
    }

    /// Calculate fault tolerance from cluster size.
    ///
    /// Returns the number of failures that can be tolerated with given cluster size.
    /// Formula: (voters - 1) / 2
    ///
    /// # Arguments
    ///
    /// * `voter_count` - Number of voters in the cluster
    ///
    /// # Returns
    ///
    /// Number of failures that can be tolerated.
    pub fn fault_tolerance_for_size(voter_count: u32) -> (result: u32)
        ensures
            voter_count == 0 ==> result == 0,
            voter_count > 0 ==> result == (voter_count - 1) / 2
    {
        if voter_count == 0 {
            0
        } else {
            (voter_count - 1) / 2
        }
    }
}
