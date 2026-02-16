//! Saga Pattern Verification Specifications
//!
//! Formal specifications for distributed saga transactions,
//! LIFO compensation ordering, and state machine transitions.
//!
//! # Key Invariants
//!
//! 1. **SAGA-1: Step Bounds**: steps.len() <= MAX_SAGA_STEPS
//! 2. **SAGA-2: LIFO Compensation**: Last-to-first rollback order
//! 3. **SAGA-3: Valid State Transitions**: No invalid transitions
//! 4. **SAGA-4: Step Index Bounds**: Indices always valid
//! 5. **SAGA-5: Compensation Retries**: Bounded retry attempts
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-jobs/verus/saga_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Maximum number of saga steps
    pub const MAX_SAGA_STEPS: u64 = 100;

    /// Maximum compensation retries
    pub const MAX_COMPENSATION_RETRIES: u64 = 5;

    /// Base delay for compensation retry (milliseconds)
    pub const COMPENSATION_RETRY_BASE_MS: u64 = 100;

    // ========================================================================
    // State Model
    // ========================================================================

    /// Step execution result
    pub enum StepResultSpec {
        Success,
        Failed,
        NotExecuted,
    }

    /// Compensation result
    pub enum CompensationResultSpec {
        Success,
        Failed,
        Skipped,
        PendingRetry { attempts: u64 },
    }

    /// Abstract saga step
    pub struct SagaStepSpec {
        /// Whether step was executed
        pub was_executed: bool,
        /// Whether compensation is required
        pub requires_compensation: bool,
        /// Step result
        pub result: StepResultSpec,
        /// Compensation result
        pub compensation_result: Option<CompensationResultSpec>,
    }

    /// Saga state enumeration
    pub enum SagaStateSpec {
        /// Not yet started
        NotStarted,
        /// Executing forward
        Executing { current_step: u64 },
        /// All steps completed
        Completed,
        /// Rolling back
        Compensating {
            failed_step: u64,
            current_compensation: u64,
        },
        /// Rollback completed
        CompensationCompleted,
        /// Rollback failed (poison state)
        CompensationFailed { failed_compensation_step: u64 },
    }

    /// Complete saga state
    pub struct SagaSpec {
        /// Number of steps
        pub step_count: u64,
        /// Current state
        pub state: SagaStateSpec,
        /// Steps executed count
        pub executed_count: u64,
    }

    // ========================================================================
    // Invariant SAGA-1: Step Bounds
    // ========================================================================

    /// SAGA-1: Step count bounded
    pub open spec fn step_count_bounded(step_count: u64) -> bool {
        step_count <= MAX_SAGA_STEPS
    }

    /// Builder enforces step bounds
    pub open spec fn builder_enforces_bounds(
        current_step_count: u64,
        add_step_succeeds: bool,
    ) -> bool {
        (current_step_count >= MAX_SAGA_STEPS) ==> !add_step_succeeds
    }

    /// Proof: Built sagas have bounded steps
    pub proof fn build_produces_bounded(step_count: u64)
        requires step_count <= MAX_SAGA_STEPS
        ensures step_count_bounded(step_count)
    {
        // By construction
    }

    // ========================================================================
    // Invariant SAGA-2: LIFO Compensation Order
    // ========================================================================

    /// SAGA-2: Compensation follows LIFO order
    ///
    /// If step N fails, compensation starts from step N-1 and proceeds to 0.
    pub open spec fn lifo_compensation_order(
        failed_step: u64,
        first_compensation: u64,
    ) -> bool {
        if failed_step > 0 {
            first_compensation == failed_step - 1
        } else {
            // First step failed, no compensation needed
            true
        }
    }

    /// Compensation decrements correctly
    pub open spec fn compensation_decrements(
        current_compensation: u64,
        next_compensation: u64,
    ) -> bool {
        if current_compensation > 0 {
            next_compensation == current_compensation - 1
        } else {
            // Last compensation, transitions to CompensationCompleted
            true
        }
    }

    /// Proof: LIFO order is maintained
    pub proof fn lifo_order_maintained(
        failed_step: u64,
        current: u64,
        next: u64,
    )
        requires
            lifo_compensation_order(failed_step, current),
            compensation_decrements(current, next),
        ensures
            // Next is strictly less (progressing toward 0)
            next < current || current == 0
    {
        // Compensation decrements toward 0
    }

    // ========================================================================
    // Invariant SAGA-3: Valid State Transitions
    // ========================================================================

    /// Valid transitions from NotStarted
    pub open spec fn valid_from_not_started(next: SagaStateSpec) -> bool {
        matches!(next, SagaStateSpec::Executing { current_step: 0 })
    }

    /// Valid transitions from Executing
    pub open spec fn valid_from_executing(
        current_step: u64,
        step_count: u64,
        next: SagaStateSpec,
    ) -> bool {
        match next {
            SagaStateSpec::Executing { current_step: next_step } => {
                // Move to next step
                next_step == current_step + 1 && next_step < step_count
            }
            SagaStateSpec::Completed => {
                // All steps done
                current_step + 1 >= step_count
            }
            SagaStateSpec::Compensating { failed_step, current_compensation } => {
                // Step failed, start compensation
                failed_step == current_step &&
                (current_step > 0 ==> current_compensation == current_step - 1)
            }
            _ => false,
        }
    }

    /// Valid transitions from Compensating
    pub open spec fn valid_from_compensating(
        current_compensation: u64,
        next: SagaStateSpec,
    ) -> bool {
        match next {
            SagaStateSpec::Compensating { current_compensation: next_comp, .. } => {
                // Move to next compensation (decrement)
                current_compensation > 0 && next_comp == current_compensation - 1
            }
            SagaStateSpec::CompensationCompleted => {
                // All compensations done
                current_compensation == 0
            }
            SagaStateSpec::CompensationFailed { .. } => {
                // Compensation step failed
                true
            }
            _ => false,
        }
    }

    /// CompensationFailed is terminal
    pub open spec fn compensation_failed_terminal(current: SagaStateSpec, next: SagaStateSpec) -> bool {
        matches!(current, SagaStateSpec::CompensationFailed { .. }) ==>
        matches!(next, SagaStateSpec::CompensationFailed { .. })
    }

    /// CompensationCompleted is terminal
    pub open spec fn compensation_completed_terminal(current: SagaStateSpec, next: SagaStateSpec) -> bool {
        matches!(current, SagaStateSpec::CompensationCompleted) ==>
        matches!(next, SagaStateSpec::CompensationCompleted)
    }

    /// Completed is terminal
    pub open spec fn completed_terminal(current: SagaStateSpec, next: SagaStateSpec) -> bool {
        matches!(current, SagaStateSpec::Completed) ==>
        matches!(next, SagaStateSpec::Completed)
    }

    // ========================================================================
    // Invariant SAGA-4: Step Index Bounds
    // ========================================================================

    /// SAGA-4: Current step index is always valid
    pub open spec fn step_index_valid(
        current_step: u64,
        step_count: u64,
    ) -> bool {
        current_step < step_count
    }

    /// Compensation index is always valid
    pub open spec fn compensation_index_valid(
        current_compensation: u64,
        failed_step: u64,
    ) -> bool {
        current_compensation < failed_step
    }

    /// State indices are valid
    pub open spec fn state_indices_valid(state: SagaStateSpec, step_count: u64) -> bool {
        match state {
            SagaStateSpec::NotStarted => true,
            SagaStateSpec::Executing { current_step } => {
                step_index_valid(current_step, step_count)
            }
            SagaStateSpec::Completed => true,
            SagaStateSpec::Compensating { failed_step, current_compensation } => {
                failed_step <= step_count &&
                (failed_step > 0 ==> current_compensation < failed_step)
            }
            SagaStateSpec::CompensationCompleted => true,
            SagaStateSpec::CompensationFailed { failed_compensation_step } => {
                failed_compensation_step < step_count
            }
        }
    }

    // ========================================================================
    // Invariant SAGA-5: Compensation Retries
    // ========================================================================

    /// SAGA-5: Retry attempts bounded
    pub open spec fn retry_bounded(attempts: u64) -> bool {
        attempts <= MAX_COMPENSATION_RETRIES
    }

    /// Exceeded retries trigger failure
    pub open spec fn exceeded_retries_fail(
        attempts: u64,
        result_is_failed: bool,
    ) -> bool {
        (attempts >= MAX_COMPENSATION_RETRIES) ==> result_is_failed
    }

    /// Exponential backoff calculation
    pub open spec fn retry_backoff_ms(attempts: u64) -> u64 {
        // base * 2^attempts, capped
        let multiplier = if attempts < 10 { 1u64 << attempts } else { 1024u64 };
        COMPENSATION_RETRY_BASE_MS * multiplier
    }

    // ========================================================================
    // Operation Specifications
    // ========================================================================

    /// start_saga precondition
    pub open spec fn start_saga_pre(step_count: u64) -> bool {
        step_count > 0 && step_count_bounded(step_count)
    }

    /// start_saga postcondition
    pub open spec fn start_saga_post(
        saga: SagaSpec,
        step_count: u64,
    ) -> bool {
        saga.step_count == step_count &&
        matches!(saga.state, SagaStateSpec::Executing { current_step: 0 }) &&
        saga.executed_count == 0
    }

    /// complete_step precondition
    pub open spec fn complete_step_pre(
        saga: SagaSpec,
        step_index: u64,
    ) -> bool {
        match saga.state {
            SagaStateSpec::Executing { current_step } => {
                current_step == step_index &&
                step_index < saga.step_count
            }
            _ => false,
        }
    }

    /// complete_step postcondition
    pub open spec fn complete_step_post(
        pre: SagaSpec,
        post: SagaSpec,
        step_index: u64,
    ) -> bool {
        post.executed_count == pre.executed_count + 1 &&
        match post.state {
            SagaStateSpec::Executing { current_step } => {
                current_step == step_index + 1
            }
            SagaStateSpec::Completed => {
                step_index + 1 >= pre.step_count
            }
            _ => false,
        }
    }

    /// fail_step precondition
    pub open spec fn fail_step_pre(
        saga: SagaSpec,
        step_index: u64,
    ) -> bool {
        match saga.state {
            SagaStateSpec::Executing { current_step } => {
                current_step == step_index
            }
            _ => false,
        }
    }

    /// fail_step postcondition
    pub open spec fn fail_step_post(
        pre: SagaSpec,
        post: SagaSpec,
        step_index: u64,
    ) -> bool {
        if step_index > 0 {
            match post.state {
                SagaStateSpec::Compensating { failed_step, current_compensation } => {
                    failed_step == step_index &&
                    lifo_compensation_order(step_index, current_compensation)
                }
                _ => false,
            }
        } else {
            // First step failed, no compensation needed
            matches!(post.state, SagaStateSpec::CompensationCompleted)
        }
    }

    // ========================================================================
    // Combined Saga Invariant
    // ========================================================================

    /// Complete invariant for saga state
    pub open spec fn saga_invariant(saga: SagaSpec) -> bool {
        // Step count bounded
        step_count_bounded(saga.step_count) &&
        // State indices valid
        state_indices_valid(saga.state, saga.step_count) &&
        // Executed count bounded
        saga.executed_count <= saga.step_count
    }

    /// Proof: start_saga produces valid saga
    pub proof fn start_saga_valid(step_count: u64)
        requires start_saga_pre(step_count)
        ensures {
            let saga = SagaSpec {
                step_count: step_count,
                state: SagaStateSpec::Executing { current_step: 0 },
                executed_count: 0,
            };
            saga_invariant(saga)
        }
    {
        // Pre ensures step_count > 0 and bounded
    }

    /// Proof: Operations maintain saga invariant
    pub proof fn operations_maintain_invariant(
        pre: SagaSpec,
        post: SagaSpec,
        step_index: u64,
    )
        requires
            saga_invariant(pre),
            complete_step_pre(pre, step_index),
            complete_step_post(pre, post, step_index),
        ensures saga_invariant(post)
    {
        // complete_step_post ensures valid next state
    }

    // ========================================================================
    // Action Selection
    // ========================================================================

    /// Next action to take for saga
    pub enum SagaActionSpec {
        Execute { step_index: u64 },
        Compensate { step_index: u64 },
        SkipCompensation { step_index: u64 },
        Done,
    }

    /// get_next_action specification
    pub open spec fn get_next_action_correct(
        saga: SagaSpec,
        action: SagaActionSpec,
    ) -> bool {
        match saga.state {
            SagaStateSpec::NotStarted => {
                matches!(action, SagaActionSpec::Execute { step_index: 0 })
            }
            SagaStateSpec::Executing { current_step } => {
                match action {
                    SagaActionSpec::Execute { step_index } => step_index == current_step,
                    _ => false,
                }
            }
            SagaStateSpec::Compensating { current_compensation, .. } => {
                match action {
                    SagaActionSpec::Compensate { step_index } => step_index == current_compensation,
                    SagaActionSpec::SkipCompensation { step_index } => step_index == current_compensation,
                    _ => false,
                }
            }
            SagaStateSpec::Completed |
            SagaStateSpec::CompensationCompleted |
            SagaStateSpec::CompensationFailed { .. } => {
                matches!(action, SagaActionSpec::Done)
            }
        }
    }
}
