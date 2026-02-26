//! Verus Formal Specifications for Aspen Jobs
//!
//! This crate contains standalone Verus specifications for verifying the
//! correctness of job scheduling, saga transactions, and retry policies.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! nix run .#verify-verus-jobs          # Verify all specs
//! nix run .#verify-verus-jobs -- quick # Syntax check only
//! nix run .#verify-verus                 # Verify all (Core + Raft + Coordination + Jobs)
//! ```
//!
//! # Module Overview
//!
//! ## Saga Pattern
//! - `saga_spec`: Saga state machine and LIFO compensation ordering
//!
//! ## Priority & Retry
//! - `priority_spec`: Priority ordering and retry policy bounds
//!
//! # Invariants Verified
//!
//! ## Saga State Machine
//!
//! 1. **SAGA-1: Step Bounds**: steps.len() <= MAX_SAGA_STEPS
//!    - Enforced at construction time
//!    - Prevents unbounded memory usage
//!
//! 2. **SAGA-2: LIFO Compensation**: Compensation order is last-to-first
//!    - Failed step triggers compensation from step-1 to 0
//!    - Each compensation step decrements current_compensation
//!
//! 3. **SAGA-3: Valid State Transitions**:
//!    - NotStarted -> Executing (on start)
//!    - Executing -> Completed (on all steps done)
//!    - Executing -> Compensating (on step failure)
//!    - Compensating -> CompensationCompleted (on all compensations done)
//!    - Compensating -> CompensationFailed (on compensation error)
//!    - CompensationFailed is terminal (poison state)
//!
//! 4. **SAGA-4: Step Index Bounds**: current_step < steps.len()
//!    - Executing.current_step is always valid
//!    - Compensating.current_compensation is always valid
//!
//! 5. **SAGA-5: Compensation Retries**: retries <= MAX_COMPENSATION_RETRIES
//!    - PendingRetry.attempts <= 5
//!    - Exceeding triggers CompensationFailed
//!
//! ## Priority Ordering
//!
//! 6. **PRIO-1: Total Order**: Critical > High > Normal > Low
//!    - all_ordered() returns [Critical, High, Normal, Low]
//!    - Used for priority queue dequeuing
//!
//! 7. **PRIO-2: Queue Names**: Each priority has unique queue name
//!    - Low -> "low", Normal -> "normal", High -> "high", Critical -> "critical"
//!
//! ## Retry Policy
//!
//! 8. **RETRY-1: max_attempts >= 1**: All policies have at least 1 attempt
//!    - None has max_attempts = 1 (the initial attempt)
//!    - Fixed/Exponential have explicit max_attempts
//!
//! 9. **RETRY-2: Exponential Bounds**: delay * multiplier^n <= max_delay
//!    - Exponential backoff saturates at max_delay
//!    - Prevents unbounded delay growth
//!
//! 10. **RETRY-3: Custom Consistency**: delays.len() >= max_attempts
//!     - Custom policy has enough delays for all attempts
//!
//! # Trusted Axioms
//!
//! The specifications assume:
//! - System clock advances monotonically
//! - KV store operations are linearizable (via Raft)
//! - JSON serialization is deterministic

use vstd::prelude::*;

verus! {
    // Re-export saga specifications
    pub use saga_spec::*;

    // Re-export priority specifications
    pub use priority_spec::*;
}

mod priority_spec;
mod saga_spec;
