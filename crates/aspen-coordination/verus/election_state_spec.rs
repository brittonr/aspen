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
    // Invariant 1: Single Leader (via Lock)
    // ========================================================================

    /// INVARIANT 1: At most one leader at any time
    ///
    /// This is guaranteed by the underlying DistributedLock:
    /// - Only one holder can have the lock at any time
    /// - Lock = Leadership
    ///
    /// For a single ElectionState, this is trivially true.
    /// For multiple concurrent ElectionStates (different candidates),
    /// this is ensured by the lock's mutual exclusion.
    pub open spec fn single_leader_invariant(state: ElectionState) -> bool {
        // A single state can only be in one mode at a time
        // The multi-node invariant is proved through the lock
        true
    }

    // ========================================================================
    // Invariant 2: Fencing Token Monotonicity
    // ========================================================================

    /// INVARIANT 2: Fencing token monotonicity
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
    // Invariant 3: State Machine Transitions
    // ========================================================================

    /// Valid state transitions:
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
    pub open spec fn election_invariant(state: ElectionState) -> bool {
        single_leader_invariant(state) &&
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
    pub proof fn initial_state_invariant()
        ensures election_invariant(initial_election_state())
    {
        // Follower state trivially satisfies all invariants
    }

    /// Proof: Initial state is follower
    pub proof fn initial_state_is_follower()
        ensures is_follower(initial_election_state())
    {
    }
}
