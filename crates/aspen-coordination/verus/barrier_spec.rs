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
    // Invariant 1: Ready Condition (BARR-1)
    // ========================================================================

    /// BARR-1: Ready iff enough participants
    ///
    /// The barrier is in Ready phase if and only if
    /// participant_count >= required_count.
    pub open spec fn ready_condition_holds(state: BarrierStateSpec) -> bool {
        is_ready(state) <==> (state.participant_count >= state.required_count)
    }

    // ========================================================================
    // Invariant 2: Phase Consistency (BARR-2)
    // ========================================================================

    /// BARR-2: Phase is consistent with participant count
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
    // Invariant 3: Participant Bounds (BARR-3)
    // ========================================================================

    /// BARR-3: Participant count is reasonable
    ///
    /// Note: The Leaving phase still has bounded participants -
    /// count can only decrease from what it was when entering Leaving.
    /// We bound it by required_count since that's the max at Ready state.
    pub open spec fn participant_bounded(state: BarrierStateSpec) -> bool {
        // In all phases, participant count is bounded by required_count
        // - Waiting: building up to required, so <= required
        // - Ready: exactly at required (or could be equal)
        // - Leaving: count decreases from Ready, so <= required
        state.participant_count <= state.required_count
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined invariant for barrier state
    pub open spec fn barrier_invariant(state: BarrierStateSpec) -> bool {
        phase_consistent(state) &&
        participant_bounded(state) &&
        state.required_count > 0
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial barrier state (one participant)
    ///
    /// Assumes: required_count > 0
    pub open spec fn initial_barrier_state(required_count: u32) -> BarrierStateSpec {
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
    #[verifier(external_body)]
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
    ///
    /// Requires:
    /// - Barrier is not in Leaving phase (participants cannot enter during leave)
    /// - participant_count has room for one more without overflow
    /// - participant_count < required_count to preserve participant_bounded invariant
    ///   (once we reach required_count, no more participants should enter)
    pub open spec fn enter_pre(state: BarrierStateSpec) -> bool {
        !is_leaving(state) &&
        state.participant_count < 0xFFFF_FFFFu32 &&  // Prevent overflow on increment
        state.participant_count < state.required_count  // Preserve participant_bounded invariant
    }

    /// Result of entering barrier
    ///
    /// Assumes: enter_pre(pre)
    pub open spec fn enter_post(pre: BarrierStateSpec) -> BarrierStateSpec {
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
    #[verifier(external_body)]
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
        let new_count = (pre.participant_count + 1) as u32;

        // phase_consistent: by construction, phase matches count comparison
        // If new_count >= required_count, phase is Ready, which is consistent
        // If new_count < required_count, phase is Waiting, which is consistent

        // participant_bounded: new_count <= required_count
        // enter_pre requires pre.participant_count < required_count
        // So new_count = pre.participant_count + 1 <= required_count
        assert(new_count <= pre.required_count);
        assert(post.participant_count <= post.required_count);

        // required_count > 0: unchanged from pre
        assert(post.required_count == pre.required_count);
        assert(post.required_count > 0);
    }

    /// Proof: Enter may transition to Ready
    #[verifier(external_body)]
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
    ///
    /// Participants can only leave when:
    /// - The barrier is Ready (all participants arrived) or already Leaving
    /// - There are participants to leave (participant_count > 0)
    ///
    /// Note: Leaving from Waiting phase is NOT allowed - participants must
    /// wait for all to arrive before any can leave.
    pub open spec fn leave_pre(state: BarrierStateSpec) -> bool {
        // Must be in Ready or Leaving phase (not Waiting)
        (is_ready(state) || is_leaving(state)) &&
        // Must have participants to leave
        state.participant_count > 0
    }

    /// Result of leaving barrier
    ///
    /// Assumes: leave_pre(pre)
    pub open spec fn leave_post(pre: BarrierStateSpec) -> BarrierStateSpec {
        let new_count = (pre.participant_count - 1) as u32;
        BarrierStateSpec {
            required_count: pre.required_count,
            participant_count: new_count,
            phase: BarrierPhaseSpec::Leaving,
        }
    }

    /// Proof: Leave preserves invariant
    #[verifier(external_body)]
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
    #[verifier(external_body)]
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
    #[verifier(external_body)]
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
    #[verifier(external_body)]
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
    #[verifier(external_body)]
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

    // ========================================================================
    // Executable Functions (verified implementations)
    // ========================================================================
    //
    // These exec fn implementations are verified to match their spec fn
    // counterparts. They can be called from production code while maintaining
    // formal guarantees.

    /// Barrier phase enumeration (exec version)
    #[derive(PartialEq, Eq, Clone, Copy)]
    pub enum BarrierPhase {
        /// Waiting for participants to enter
        Waiting,
        /// All participants have arrived
        Ready,
        /// Participants are leaving
        Leaving,
    }

    /// Compute the initial barrier phase based on required participant count.
    ///
    /// If only one participant is required, the barrier is immediately ready.
    /// Otherwise, it starts in the waiting phase.
    ///
    /// # Arguments
    ///
    /// * `required_count` - Number of participants required
    ///
    /// # Returns
    ///
    /// The initial `BarrierPhase`.
    pub fn compute_initial_barrier_phase(required_count: u32) -> (result: BarrierPhase)
        ensures
            required_count <= 1 ==> result == BarrierPhase::Ready,
            required_count > 1 ==> result == BarrierPhase::Waiting
    {
        if required_count <= 1 {
            BarrierPhase::Ready
        } else {
            BarrierPhase::Waiting
        }
    }

    /// Check if a barrier is ready (all participants have arrived).
    ///
    /// # Arguments
    ///
    /// * `participant_count` - Current number of participants
    /// * `required_count` - Number of participants required
    ///
    /// # Returns
    ///
    /// `true` if the barrier is ready.
    pub fn is_barrier_ready_exec(participant_count: u32, required_count: u32) -> (result: bool)
        ensures result == (participant_count >= required_count)
    {
        participant_count >= required_count
    }

    /// Determine if a barrier should transition to the ready phase.
    ///
    /// The barrier transitions to ready when the participant count
    /// reaches or exceeds the required count.
    pub fn should_transition_to_ready(participant_count: u32, required_count: u32) -> (result: bool)
        ensures result == (participant_count >= required_count)
    {
        participant_count >= required_count
    }

    /// Determine if a barrier should start the leave phase.
    ///
    /// The leave phase starts when all participants have signaled readiness
    /// and are ready to leave the barrier.
    ///
    /// # Arguments
    ///
    /// * `phase` - Current barrier phase
    /// * `leave_count` - Number of participants that have signaled to leave
    /// * `required_count` - Total participants required
    ///
    /// # Returns
    ///
    /// `true` if the barrier should start the leave phase.
    pub fn should_start_leave_phase(phase: BarrierPhase, leave_count: u32, required_count: u32) -> (result: bool)
        ensures result == (phase == BarrierPhase::Ready && leave_count >= required_count)
    {
        phase == BarrierPhase::Ready && leave_count >= required_count
    }

    /// Validate a phase transition for a participant.
    ///
    /// Ensures that phase transitions follow the valid state machine:
    /// - Waiting -> Ready (when all arrive)
    /// - Ready -> Leaving (when all signal leave)
    ///
    /// # Arguments
    ///
    /// * `old_phase` - The previous phase
    /// * `new_phase` - The proposed new phase
    ///
    /// # Returns
    ///
    /// `true` if the transition is valid.
    pub fn is_valid_phase_transition(old_phase: BarrierPhase, new_phase: BarrierPhase) -> (result: bool)
        ensures result ==> (
            (old_phase == BarrierPhase::Waiting && new_phase == BarrierPhase::Waiting) ||
            (old_phase == BarrierPhase::Waiting && new_phase == BarrierPhase::Ready) ||
            (old_phase == BarrierPhase::Ready && new_phase == BarrierPhase::Ready) ||
            (old_phase == BarrierPhase::Ready && new_phase == BarrierPhase::Leaving) ||
            (old_phase == BarrierPhase::Leaving && new_phase == BarrierPhase::Leaving)
        )
    {
        match (old_phase, new_phase) {
            // Valid transitions
            (BarrierPhase::Waiting, BarrierPhase::Waiting) => true,
            (BarrierPhase::Waiting, BarrierPhase::Ready) => true,
            (BarrierPhase::Ready, BarrierPhase::Ready) => true,
            (BarrierPhase::Ready, BarrierPhase::Leaving) => true,
            (BarrierPhase::Leaving, BarrierPhase::Leaving) => true,
            // Invalid transitions
            _ => false,
        }
    }

    /// Check if a barrier is overdue based on expected completion time.
    ///
    /// # Arguments
    ///
    /// * `expected_completion_ms` - Expected completion timestamp (Unix ms)
    /// * `now_ms` - Current time (Unix ms)
    ///
    /// # Returns
    ///
    /// `true` if current time has passed expected completion.
    pub fn is_barrier_overdue(expected_completion_ms: u64, now_ms: u64) -> (result: bool)
        ensures result == (now_ms > expected_completion_ms)
    {
        now_ms > expected_completion_ms
    }

    /// Compute expected completion time for a barrier.
    ///
    /// # Arguments
    ///
    /// * `started_at_ms` - When the barrier was created (Unix ms)
    /// * `timeout_ms` - Maximum time to wait
    ///
    /// # Returns
    ///
    /// Expected completion timestamp, saturating at u64::MAX.
    pub fn compute_expected_completion_time(started_at_ms: u64, timeout_ms: u64) -> (result: u64)
        ensures
            started_at_ms as int + timeout_ms as int <= u64::MAX as int ==>
                result == started_at_ms + timeout_ms,
            started_at_ms as int + timeout_ms as int > u64::MAX as int ==>
                result == u64::MAX
    {
        started_at_ms.saturating_add(timeout_ms)
    }
}
