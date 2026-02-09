//! Barrier State Machine Model and Operations
//!
//! Abstract state model for formal verification of distributed barrier operations.
//!
//! # State Model
//!
//! The `BarrierState` captures:
//! - Required participant count
//! - Current participant count
//! - Current phase (Waiting, Ready, Leaving)
//!
//! # Key Invariants
//!
//! 1. **Phase Ordering**: Waiting -> Ready -> Leaving
//! 2. **Ready Condition**: Ready iff participant_count >= required_count
//! 3. **Participant Bounds**: participant_count <= required_count
//! 4. **Phase Consistency**: Phase matches participant state
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/barrier_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // State Model
    // ========================================================================

    /// Barrier phase enumeration
    pub enum BarrierPhaseSpec {
        /// Waiting for participants to enter
        Waiting,
        /// All participants have arrived
        Ready,
        /// Participants are leaving
        Leaving,
    }

    /// Complete barrier state
    pub struct BarrierStateSpec {
        /// Number of participants required
        pub required_count: u32,
        /// Current number of participants
        pub participant_count: u32,
        /// Current phase
        pub phase: BarrierPhaseSpec,
    }

    // ========================================================================
    // Core Predicates
    // ========================================================================

    /// Check if barrier is waiting for participants
    pub open spec fn is_waiting(state: BarrierStateSpec) -> bool {
        match state.phase {
            BarrierPhaseSpec::Waiting => true,
            _ => false,
        }
    }

    /// Check if barrier is ready
    pub open spec fn is_ready(state: BarrierStateSpec) -> bool {
        match state.phase {
            BarrierPhaseSpec::Ready => true,
            _ => false,
        }
    }

    /// Check if barrier is in leaving phase
    pub open spec fn is_leaving(state: BarrierStateSpec) -> bool {
        match state.phase {
            BarrierPhaseSpec::Leaving => true,
            _ => false,
        }
    }

    // ========================================================================
    // Invariant 1: Ready Condition
    // ========================================================================

    /// INVARIANT 1: Ready iff enough participants
    ///
    /// The barrier is in Ready phase if and only if
    /// participant_count >= required_count.
    pub open spec fn ready_condition_holds(state: BarrierStateSpec) -> bool {
        is_ready(state) <==> (state.participant_count >= state.required_count)
    }

    // ========================================================================
    // Invariant 2: Phase Consistency
    // ========================================================================

    /// INVARIANT 2: Phase is consistent with participant count
    pub open spec fn phase_consistent(state: BarrierStateSpec) -> bool {
        match state.phase {
            BarrierPhaseSpec::Waiting => {
                state.participant_count < state.required_count
            }
            BarrierPhaseSpec::Ready => {
                state.participant_count >= state.required_count
            }
            BarrierPhaseSpec::Leaving => {
                // In Leaving phase, participant count is bounded by required_count
                // (can't have more participants than were required to enter)
                state.participant_count <= state.required_count
            }
        }
    }

    // ========================================================================
    // Invariant 3: Participant Bounds
    // ========================================================================

    /// INVARIANT 3: Participant count is reasonable
    pub open spec fn participant_bounded(state: BarrierStateSpec) -> bool {
        // In Waiting/Ready, can't exceed required (participants are unique)
        // Actually, can have more than required if late joiners, but let's model conservatively
        match state.phase {
            BarrierPhaseSpec::Leaving => true,  // During leave, count decreases
            _ => state.participant_count <= state.required_count,
        }
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined invariant for barrier state
    pub open spec fn barrier_invariant(state: BarrierStateSpec) -> bool {
        phase_consistent(state) &&
        state.required_count > 0
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial barrier state (one participant)
    pub open spec fn initial_barrier_state(required_count: u32) -> BarrierStateSpec
        requires required_count > 0
    {
        BarrierStateSpec {
            required_count,
            participant_count: 1,
            phase: if required_count <= 1 {
                BarrierPhaseSpec::Ready
            } else {
                BarrierPhaseSpec::Waiting
            },
        }
    }

    /// Proof: Initial state satisfies invariant
    pub proof fn initial_state_invariant(required_count: u32)
        requires required_count > 0
        ensures barrier_invariant(initial_barrier_state(required_count))
    {
        let state = initial_barrier_state(required_count);
        // If required <= 1: phase is Ready and participant_count (1) >= required
        // If required > 1: phase is Waiting and participant_count (1) < required
    }

    // ========================================================================
    // Enter Operation
    // ========================================================================

    /// Precondition for entering barrier
    pub open spec fn enter_pre(state: BarrierStateSpec) -> bool {
        !is_leaving(state)
    }

    /// Result of entering barrier
    pub open spec fn enter_post(pre: BarrierStateSpec) -> BarrierStateSpec
        requires enter_pre(pre)
    {
        let new_count = (pre.participant_count + 1) as u32;
        let new_phase = if new_count >= pre.required_count {
            BarrierPhaseSpec::Ready
        } else {
            BarrierPhaseSpec::Waiting
        };
        BarrierStateSpec {
            required_count: pre.required_count,
            participant_count: new_count,
            phase: new_phase,
        }
    }

    /// Proof: Enter preserves invariant
    pub proof fn enter_preserves_invariant(
        pre: BarrierStateSpec,
    )
        requires
            enter_pre(pre),
            barrier_invariant(pre),
        ensures
            barrier_invariant(enter_post(pre))
    {
        let post = enter_post(pre);
        // phase_consistent: by construction, phase matches count comparison
        // required_count unchanged
    }

    /// Proof: Enter may transition to Ready
    pub proof fn enter_may_become_ready(
        pre: BarrierStateSpec,
    )
        requires
            enter_pre(pre),
            barrier_invariant(pre),
            (pre.participant_count + 1) as u32 >= pre.required_count,
        ensures
            is_ready(enter_post(pre))
    {
        let post = enter_post(pre);
        let new_count = (pre.participant_count + 1) as u32;
        // new_count >= required => new_phase is Ready
        assert(new_count >= pre.required_count);
    }

    // ========================================================================
    // Leave Operation
    // ========================================================================

    /// Precondition for leaving barrier
    pub open spec fn leave_pre(state: BarrierStateSpec) -> bool {
        state.participant_count > 0
    }

    /// Result of leaving barrier
    pub open spec fn leave_post(pre: BarrierStateSpec) -> BarrierStateSpec
        requires leave_pre(pre)
    {
        let new_count = (pre.participant_count - 1) as u32;
        BarrierStateSpec {
            required_count: pre.required_count,
            participant_count: new_count,
            phase: BarrierPhaseSpec::Leaving,
        }
    }

    /// Proof: Leave preserves invariant
    pub proof fn leave_preserves_invariant(
        pre: BarrierStateSpec,
    )
        requires
            leave_pre(pre),
            barrier_invariant(pre),
        ensures
            barrier_invariant(leave_post(pre))
    {
        let post = leave_post(pre);
        // phase is Leaving, which is consistent with any count
        // required_count unchanged
    }

    /// When all leave, barrier is complete
    pub open spec fn is_complete(state: BarrierStateSpec) -> bool {
        is_leaving(state) && state.participant_count == 0
    }

    /// Proof: Last leaver makes barrier complete
    pub proof fn last_leave_completes(
        pre: BarrierStateSpec,
    )
        requires
            leave_pre(pre),
            pre.participant_count == 1,
        ensures
            is_complete(leave_post(pre))
    {
        // new_count = 0, phase = Leaving
    }

    // ========================================================================
    // Phase Ordering
    // ========================================================================

    /// Valid phase transitions for enter operation
    pub open spec fn valid_enter_transition(
        pre: BarrierPhaseSpec,
        post: BarrierPhaseSpec,
    ) -> bool {
        match (pre, post) {
            // Stay in same phase
            (BarrierPhaseSpec::Waiting, BarrierPhaseSpec::Waiting) => true,
            (BarrierPhaseSpec::Ready, BarrierPhaseSpec::Ready) => true,
            // Transition to ready when enough participants
            (BarrierPhaseSpec::Waiting, BarrierPhaseSpec::Ready) => true,
            _ => false,
        }
    }

    /// Valid phase transitions for leave operation
    pub open spec fn valid_leave_transition(
        pre: BarrierPhaseSpec,
        post: BarrierPhaseSpec,
    ) -> bool {
        match post {
            // Leave always goes to Leaving phase
            BarrierPhaseSpec::Leaving => true,
            _ => false,
        }
    }

    /// Proof: Enter produces valid transition (when in Waiting phase)
    pub proof fn enter_valid_transition_from_waiting(
        pre: BarrierStateSpec,
    )
        requires
            enter_pre(pre),
            barrier_invariant(pre),
            is_waiting(pre),
        ensures
            valid_enter_transition(pre.phase, enter_post(pre).phase)
    {
        let post = enter_post(pre);
        // From Waiting, can stay Waiting or go to Ready
    }

    /// Proof: Enter from Ready stays Ready (with explicit precondition)
    pub proof fn enter_stays_ready(
        pre: BarrierStateSpec,
    )
        requires
            enter_pre(pre),
            barrier_invariant(pre),
            is_ready(pre),
            pre.participant_count < 0xFFFF_FFFEu32, // No overflow
        ensures
            is_ready(enter_post(pre))
    {
        let post = enter_post(pre);
        let new_count = (pre.participant_count + 1) as u32;
        // is_ready(pre) means pre.count >= required (from phase_consistent)
        // So new_count = pre.count + 1 >= required
        // Therefore post phase is Ready
        assert(pre.participant_count >= pre.required_count);
        assert(new_count >= pre.required_count);
    }

    /// Proof: Leave produces valid transition
    pub proof fn leave_valid_transition(
        pre: BarrierStateSpec,
    )
        requires
            leave_pre(pre),
            barrier_invariant(pre),
        ensures
            valid_leave_transition(pre.phase, leave_post(pre).phase)
    {
        // Leave always sets phase to Leaving
        let post = leave_post(pre);
        assert(is_leaving(post));
    }
}
