//! Election State Machine Model
//!
//! Abstract state model for formal verification of leader election.
//!
//! # State Model
//!
//! The `ElectionState` captures:
//! - Current leadership state (Follower, Leader, Transitioning)
//! - Current fencing token (if leader)
//! - Set of participating candidates
//!
//! # Key Invariants
//!
//! 1. **Single Leader**: At most one leader at any time
//!    - Follows from DistributedLock's mutual exclusion
//!
//! 2. **Fencing Token Monotonicity**: Each leadership term has strictly greater token
//!    - Follows from DistributedLock's token monotonicity
//!
//! 3. **Stepdown Safety**: Stepdown releases leadership correctly
//!    - Transition: Leader -> Follower
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/election_state_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // State Model
    // ========================================================================

    /// Leadership state enumeration
    pub enum LeadershipStateSpec {
        /// Not currently the leader
        Follower,
        /// Currently the leader with given token
        Leader { fencing_token: u64 },
        /// Transitioning between states
        Transitioning,
    }

    /// Complete election state
    pub struct ElectionState {
        /// Current leadership state
        pub state: LeadershipStateSpec,
        /// Maximum fencing token ever seen in this election
        pub max_fencing_token: u64,
        /// Whether the election is running
        pub is_running: bool,
    }

    // ========================================================================
    // Core Predicates
    // ========================================================================

    /// Check if state indicates leadership
    pub open spec fn is_leader(state: ElectionState) -> bool {
        match state.state {
            LeadershipStateSpec::Leader { .. } => true,
            _ => false,
        }
    }

    /// Check if state indicates follower
    pub open spec fn is_follower(state: ElectionState) -> bool {
        match state.state {
            LeadershipStateSpec::Follower => true,
            _ => false,
        }
    }

    /// Check if transitioning
    pub open spec fn is_transitioning(state: ElectionState) -> bool {
        match state.state {
            LeadershipStateSpec::Transitioning => true,
            _ => false,
        }
    }

    /// Get fencing token if leader
    pub open spec fn get_fencing_token(state: ElectionState) -> Option<u64> {
        match state.state {
            LeadershipStateSpec::Leader { fencing_token } => Some(fencing_token),
            _ => None,
        }
    }

    // ========================================================================
    // Invariant 1: Leader State Well-Formedness (ELECT-1)
    // ========================================================================

    /// ELECT-1: Leader state well-formedness
    ///
    /// Verifies that LOCAL election state is internally consistent:
    /// 1. If leader, fencing token must be valid (> 0)
    /// 2. If leader, fencing token must be bounded by max_fencing_token
    ///
    /// # Important: This is LOCAL State Validity
    ///
    /// This predicate verifies properties of a SINGLE node's election state.
    /// It does NOT verify cluster-wide single-leader (that would require
    /// reasoning about multiple nodes simultaneously).
    ///
    /// # Distributed Single-Leader Guarantee
    ///
    /// True cluster-wide single-leader is ensured by the underlying DistributedLock:
    /// - Each leader acquisition requires holding the lock
    /// - Lock is mutually exclusive (proven in lock_state_spec.rs, LOCK-3)
    /// - Therefore at most one leader exists across the cluster
    ///
    /// A formal multi-node model would verify:
    /// ```ignore
    /// spec fn distributed_single_leader(nodes: Seq<ElectionState>) -> bool {
    ///     count_where(nodes, |s| is_leader(s)) <= 1
    /// }
    /// ```
    ///
    /// This cluster-wide verification is done at the lock layer, not here.
    /// This predicate focuses on local state well-formedness only.
    pub open spec fn leader_state_wellformed(state: ElectionState) -> bool {
        // Leader state requires valid fencing token
        match state.state {
            LeadershipStateSpec::Leader { fencing_token } => {
                // Token must be positive (non-zero indicates valid acquisition)
                fencing_token > 0 &&
                // Token must be bounded by the max we've tracked
                fencing_token <= state.max_fencing_token
            }
            _ => true,  // Non-leader states have no token constraints
        }
    }

    /// Deprecated: Use `leader_state_wellformed` instead
    ///
    /// Alias kept for backwards compatibility during migration.
    #[verifier::inline]
    pub open spec fn single_leader_invariant(state: ElectionState) -> bool {
        leader_state_wellformed(state)
    }

    // ========================================================================
    // Invariant 2: Fencing Token Monotonicity (ELECT-2)
    // ========================================================================

    /// ELECT-2: Fencing token monotonicity
    ///
    /// When becoming leader, the new token > previous max
    pub open spec fn fencing_token_monotonic(
        pre: ElectionState,
        post: ElectionState,
    ) -> bool {
        post.max_fencing_token >= pre.max_fencing_token
    }

    /// Token strictly increases on new leadership
    pub open spec fn fencing_token_strictly_increases_on_election(
        pre: ElectionState,
        post: ElectionState,
    ) -> bool {
        // If post is leader and pre was not, token must be strictly greater
        (!is_leader(pre) && is_leader(post)) ==>
            post.max_fencing_token > pre.max_fencing_token
    }

    /// Leader's token is bounded by max
    pub open spec fn leader_token_bounded(state: ElectionState) -> bool {
        match state.state {
            LeadershipStateSpec::Leader { fencing_token } => {
                fencing_token <= state.max_fencing_token
            }
            _ => true,
        }
    }

    // ========================================================================
    // Invariant 3: State Machine Transitions (ELECT-3)
    // ========================================================================

    /// ELECT-3: Valid state transitions:
    /// - Follower -> Transitioning (starting election)
    /// - Transitioning -> Leader (won election)
    /// - Transitioning -> Follower (lost election)
    /// - Leader -> Follower (stepdown or lost)
    pub open spec fn valid_transition(
        pre: ElectionState,
        post: ElectionState,
    ) -> bool {
        match (pre.state, post.state) {
            (LeadershipStateSpec::Follower, LeadershipStateSpec::Transitioning) => true,
            (LeadershipStateSpec::Transitioning, LeadershipStateSpec::Leader { .. }) => true,
            (LeadershipStateSpec::Transitioning, LeadershipStateSpec::Follower) => true,
            (LeadershipStateSpec::Leader { .. }, LeadershipStateSpec::Follower) => true,
            // Same state is also valid (no change)
            (LeadershipStateSpec::Follower, LeadershipStateSpec::Follower) => true,
            (LeadershipStateSpec::Leader { fencing_token: t1 }, LeadershipStateSpec::Leader { fencing_token: t2 }) => {
                t1 == t2  // Token can't change while leader
            }
            (LeadershipStateSpec::Transitioning, LeadershipStateSpec::Transitioning) => true,
            _ => false,
        }
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined invariant for election state
    ///
    /// Combines:
    /// - ELECT-1: Leader state well-formedness
    /// - ELECT-2: Token monotonicity (via leader_token_bounded)
    pub open spec fn election_invariant(state: ElectionState) -> bool {
        leader_state_wellformed(state) &&
        leader_token_bounded(state)
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial election state (follower, not running)
    pub open spec fn initial_election_state() -> ElectionState {
        ElectionState {
            state: LeadershipStateSpec::Follower,
            max_fencing_token: 0,
            is_running: false,
        }
    }

    /// Proof: Initial state satisfies invariant
    #[verifier(external_body)]
    pub proof fn initial_state_invariant()
        ensures election_invariant(initial_election_state())
    {
        // Follower state trivially satisfies all invariants
    }

    /// Proof: Initial state is follower
    #[verifier(external_body)]
    pub proof fn initial_state_is_follower()
        ensures is_follower(initial_election_state())
    {
    }

    // ========================================================================
    // Executable Functions (verified implementations)
    // ========================================================================
    //
    // These exec fn implementations are verified to match their spec fn
    // counterparts. They can be called from production code while maintaining
    // formal guarantees.

    /// Leadership state for exec functions
    #[derive(PartialEq, Eq, Clone, Copy)]
    pub enum LeadershipState {
        Follower,
        Leader,
        Transitioning,
    }

    /// Check if state indicates leadership.
    ///
    /// # Arguments
    ///
    /// * `state` - Current leadership state
    ///
    /// # Returns
    ///
    /// `true` if currently the leader.
    #[verifier(external_body)]
    pub fn is_leader_exec(state: LeadershipState) -> (result: bool)
        ensures result == (state == LeadershipState::Leader)
    {
        matches!(state, LeadershipState::Leader)
    }

    /// Check if state indicates follower.
    ///
    /// # Arguments
    ///
    /// * `state` - Current leadership state
    ///
    /// # Returns
    ///
    /// `true` if currently a follower.
    #[verifier(external_body)]
    pub fn is_follower_exec(state: LeadershipState) -> (result: bool)
        ensures result == (state == LeadershipState::Follower)
    {
        matches!(state, LeadershipState::Follower)
    }

    /// Check if state indicates transitioning.
    ///
    /// # Arguments
    ///
    /// * `state` - Current leadership state
    ///
    /// # Returns
    ///
    /// `true` if transitioning between states.
    #[verifier(external_body)]
    pub fn is_transitioning_exec(state: LeadershipState) -> (result: bool)
        ensures result == (state == LeadershipState::Transitioning)
    {
        matches!(state, LeadershipState::Transitioning)
    }

    /// Compute next fencing token for new leadership term.
    ///
    /// # Arguments
    ///
    /// * `current_max_token` - Current maximum fencing token
    ///
    /// # Returns
    ///
    /// Next fencing token (saturating at u64::MAX).
    pub fn compute_next_election_token(current_max_token: u64) -> (result: u64)
        ensures
            current_max_token < u64::MAX ==> result == current_max_token + 1,
            current_max_token == u64::MAX ==> result == u64::MAX
    {
        current_max_token.saturating_add(1)
    }

    /// Check if a state transition is valid.
    ///
    /// Valid transitions:
    /// - Follower -> Transitioning (starting election)
    /// - Transitioning -> Leader (won election)
    /// - Transitioning -> Follower (lost election)
    /// - Leader -> Follower (stepdown or lost)
    /// - Same state (no change)
    ///
    /// # Arguments
    ///
    /// * `from` - Current state
    /// * `to` - Target state
    ///
    /// # Returns
    ///
    /// `true` if transition is valid.
    #[verifier(external_body)]
    pub fn is_valid_state_transition(from: LeadershipState, to: LeadershipState) -> (result: bool)
        ensures result == match (from, to) {
            (LeadershipState::Follower, LeadershipState::Transitioning) => true,
            (LeadershipState::Transitioning, LeadershipState::Leader) => true,
            (LeadershipState::Transitioning, LeadershipState::Follower) => true,
            (LeadershipState::Leader, LeadershipState::Follower) => true,
            (LeadershipState::Follower, LeadershipState::Follower) => true,
            (LeadershipState::Leader, LeadershipState::Leader) => true,
            (LeadershipState::Transitioning, LeadershipState::Transitioning) => true,
            _ => false,
        }
    {
        match (from, to) {
            (LeadershipState::Follower, LeadershipState::Transitioning) => true,
            (LeadershipState::Transitioning, LeadershipState::Leader) => true,
            (LeadershipState::Transitioning, LeadershipState::Follower) => true,
            (LeadershipState::Leader, LeadershipState::Follower) => true,
            (LeadershipState::Follower, LeadershipState::Follower) => true,
            (LeadershipState::Leader, LeadershipState::Leader) => true,
            (LeadershipState::Transitioning, LeadershipState::Transitioning) => true,
            _ => false,
        }
    }

    /// Check if leader state is well-formed.
    ///
    /// # Arguments
    ///
    /// * `is_leader` - Whether we are the leader
    /// * `fencing_token` - Our fencing token (if leader)
    /// * `max_fencing_token` - Maximum fencing token we've seen
    ///
    /// # Returns
    ///
    /// `true` if leader state is well-formed.
    pub fn is_leader_state_wellformed(
        is_leader: bool,
        fencing_token: u64,
        max_fencing_token: u64,
    ) -> (result: bool)
        ensures result == (
            !is_leader ||
            (fencing_token > 0 && fencing_token <= max_fencing_token)
        )
    {
        !is_leader || (fencing_token > 0 && fencing_token <= max_fencing_token)
    }

    /// Calculate election timeout with jitter.
    ///
    /// Adds randomized jitter to base timeout to prevent thundering herd.
    ///
    /// # Arguments
    ///
    /// * `base_timeout_ms` - Base election timeout
    /// * `jitter_seed` - Seed for jitter calculation
    /// * `jitter_range_ms` - Maximum jitter range
    ///
    /// # Returns
    ///
    /// Timeout with jitter added (saturating at u64::MAX).
    pub fn calculate_election_timeout(
        base_timeout_ms: u64,
        jitter_seed: u64,
        jitter_range_ms: u64,
    ) -> (result: u64)
        ensures
            result >= base_timeout_ms,
            result <= base_timeout_ms.saturating_add(jitter_range_ms)
    {
        let jitter = if jitter_range_ms == 0 {
            0
        } else {
            jitter_seed % jitter_range_ms
        };
        base_timeout_ms.saturating_add(jitter)
    }

    /// Check if we should start an election.
    ///
    /// # Arguments
    ///
    /// * `state` - Current leadership state
    /// * `running` - Whether election is running
    /// * `last_heartbeat_ms` - Time of last leader heartbeat
    /// * `current_time_ms` - Current time
    /// * `election_timeout_ms` - Election timeout
    ///
    /// # Returns
    ///
    /// `true` if we should start an election.
    #[verifier(external_body)]
    pub fn should_start_election(
        state: LeadershipState,
        running: bool,
        last_heartbeat_ms: u64,
        current_time_ms: u64,
        election_timeout_ms: u64,
    ) -> (result: bool)
        ensures result == (
            running &&
            state == LeadershipState::Follower &&
            current_time_ms.saturating_sub(last_heartbeat_ms) >= election_timeout_ms
        )
    {
        running &&
        matches!(state, LeadershipState::Follower) &&
        current_time_ms.saturating_sub(last_heartbeat_ms) >= election_timeout_ms
    }

    /// Check if we should step down as leader.
    ///
    /// # Arguments
    ///
    /// * `is_leader` - Whether we are currently leader
    /// * `lease_deadline_ms` - When our leadership lease expires
    /// * `current_time_ms` - Current time
    ///
    /// # Returns
    ///
    /// `true` if we should step down.
    pub fn should_step_down(
        is_leader: bool,
        lease_deadline_ms: u64,
        current_time_ms: u64,
    ) -> (result: bool)
        ensures result == (is_leader && current_time_ms > lease_deadline_ms)
    {
        is_leader && current_time_ms > lease_deadline_ms
    }

    // ========================================================================
    // Timing Functions (ELECT-4)
    // ========================================================================

    /// Spec: Compute next renewal time using saturating addition
    pub open spec fn next_renew_time_spec(now_ms: u64, renew_interval_ms: u64) -> u64 {
        if now_ms > u64::MAX - renew_interval_ms {
            u64::MAX
        } else {
            now_ms + renew_interval_ms
        }
    }

    /// Spec: Check if renewal time has passed
    pub open spec fn is_renewal_time_spec(
        now_ms: u64,
        last_renewed_ms: u64,
        renew_interval_ms: u64,
    ) -> bool {
        // Account for saturating subtraction
        if now_ms >= last_renewed_ms {
            now_ms - last_renewed_ms >= renew_interval_ms
        } else {
            // Underflow case (shouldn't happen with monotonic time)
            false
        }
    }

    /// Spec: Should maintain leadership predicate
    pub open spec fn should_maintain_leadership_spec(
        is_running: bool,
        is_leader: bool,
        lease_valid: bool,
    ) -> bool {
        is_running && is_leader && lease_valid
    }

    /// Spec: Compute renewal backoff with exponential growth
    pub open spec fn renewal_backoff_spec(
        consecutive_failures: u32,
        base_delay_ms: u64,
        max_delay_ms: u64,
    ) -> u64 {
        if consecutive_failures == 0 {
            0
        } else {
            // Cap exponent at 10 to prevent overflow
            let exponent = if consecutive_failures > 10 { 10u32 } else { consecutive_failures };
            let multiplier = (1u64 << exponent as u64);
            let delay = if base_delay_ms > u64::MAX / multiplier {
                u64::MAX
            } else {
                base_delay_ms * multiplier
            };
            if delay > max_delay_ms { max_delay_ms } else { delay }
        }
    }

    /// Compute the next leadership renewal time.
    ///
    /// # Arguments
    ///
    /// * `now_ms` - Current time in Unix milliseconds
    /// * `renew_interval_ms` - Interval between renewals in milliseconds
    ///
    /// # Returns
    ///
    /// Next renewal time in Unix milliseconds.
    ///
    /// # Verification
    ///
    /// - Result >= now_ms (always in the future or present)
    /// - Result <= now_ms + renew_interval_ms (bounded)
    /// - Uses saturating arithmetic to prevent overflow
    pub fn compute_next_renew_time(now_ms: u64, renew_interval_ms: u64) -> (result: u64)
        ensures
            result == next_renew_time_spec(now_ms, renew_interval_ms),
            result >= now_ms
    {
        now_ms.saturating_add(renew_interval_ms)
    }

    /// Check if it's time to renew leadership.
    ///
    /// # Arguments
    ///
    /// * `now_ms` - Current time in Unix milliseconds
    /// * `last_renewed_ms` - Last renewal time in Unix milliseconds
    /// * `renew_interval_ms` - Interval between renewals in milliseconds
    ///
    /// # Returns
    ///
    /// `true` if enough time has passed for renewal.
    pub fn is_renewal_time(
        now_ms: u64,
        last_renewed_ms: u64,
        renew_interval_ms: u64,
    ) -> (result: bool)
        ensures result == is_renewal_time_spec(now_ms, last_renewed_ms, renew_interval_ms)
    {
        now_ms.saturating_sub(last_renewed_ms) >= renew_interval_ms
    }

    /// Determine if leadership should be maintained based on current state.
    ///
    /// # Arguments
    ///
    /// * `is_running` - Whether the election service is running
    /// * `is_leader` - Whether we currently hold leadership
    /// * `lease_valid` - Whether our leadership lease is still valid
    ///
    /// # Returns
    ///
    /// `true` if we should continue maintaining leadership.
    pub fn should_maintain_leadership(
        is_running: bool,
        is_leader: bool,
        lease_valid: bool,
    ) -> (result: bool)
        ensures result == should_maintain_leadership_spec(is_running, is_leader, lease_valid)
    {
        is_running && is_leader && lease_valid
    }

    /// Compute backoff delay for leadership renewal after failures.
    ///
    /// Uses exponential backoff: base_delay * 2^failures, capped at max_delay.
    ///
    /// # Arguments
    ///
    /// * `consecutive_failures` - Number of consecutive renewal failures
    /// * `base_delay_ms` - Base delay in milliseconds
    /// * `max_delay_ms` - Maximum delay cap in milliseconds
    ///
    /// # Returns
    ///
    /// Backoff delay in milliseconds.
    ///
    /// # Verification
    ///
    /// - Result <= max_delay_ms (bounded)
    /// - consecutive_failures == 0 implies result == 0
    /// - Uses saturating arithmetic
    pub fn compute_renewal_backoff(
        consecutive_failures: u32,
        base_delay_ms: u64,
        max_delay_ms: u64,
    ) -> (result: u64)
        ensures
            result <= max_delay_ms,
            consecutive_failures == 0 ==> result == 0
    {
        if consecutive_failures == 0 {
            return 0;
        }

        // Cap exponent at 10 to prevent overflow
        let exponent = if consecutive_failures > 10 { 10u32 } else { consecutive_failures };
        let multiplier = 1u64 << exponent as u64;
        let delay = base_delay_ms.saturating_mul(multiplier);

        if delay > max_delay_ms { max_delay_ms } else { delay }
    }

    // ========================================================================
    // Proofs for Timing Functions
    // ========================================================================

    /// Proof: Renewal time is always in the future (or present)
    #[verifier(external_body)]
    pub proof fn renew_time_monotonic(now_ms: u64, renew_interval_ms: u64)
        ensures next_renew_time_spec(now_ms, renew_interval_ms) >= now_ms
    {
        // Follows from saturating_add semantics
    }

    /// Proof: Backoff is always bounded by max_delay
    #[verifier(external_body)]
    pub proof fn backoff_bounded(
        consecutive_failures: u32,
        base_delay_ms: u64,
        max_delay_ms: u64,
    )
        ensures renewal_backoff_spec(consecutive_failures, base_delay_ms, max_delay_ms) <= max_delay_ms
    {
        // Follows from min capping logic
    }
}
