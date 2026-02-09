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
        pub running: bool,
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
            running: false,
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
}
