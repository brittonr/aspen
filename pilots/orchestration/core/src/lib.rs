// SPDX-License-Identifier: AGPL-3.0-or-later
#![no_std]
#![forbid(unsafe_code)]
//! Pure allocation-execution laws. No storage, transport, process, clock,
//! allocator, randomness, policy evaluator, or backend dependency.
//!
//! These types are in-memory inputs, NOT canonical wire identity or authority.
//! An adapter must validate canonical Preserves identities and current fabric
//! authority before constructing inputs. Placement and reservation remain owned
//! by Molten's membership/placement components; this crate does not replace them.

use core::num::NonZeroU64;

macro_rules! reference_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub [u8; 32]);
    };
}
reference_type!(RunId);
reference_type!(AssignmentRef);
reference_type!(WorkloadRef);
reference_type!(NodeRef);
reference_type!(ResultRef);

/// Exact admitted assignment, including attempt epoch. Possession is NOT a grant.
/// `run` identifies an execution request, not the immutable workload definition.
/// Replacement must receive a fresh assignment ref and an advanced fabric fence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fence {
    pub run: RunId,
    pub assignment: AssignmentRef,
    pub workload: WorkloadRef,
    pub node: NodeRef,
    pub generation: NonZeroU64,
    pub epoch: NonZeroU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalResult {
    Succeeded(ResultRef),
    Failed(ResultRef),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Assigned,
    /// Persisted BEFORE dispatch. Recovery must never blindly redispatch this.
    Dispatching,
    Running,
    /// A start/effect may have happened; explicit observation is required.
    Unknown,
    /// This attempt definitively did not start. This is not a retry grant.
    RejectedBeforeStart,
    Terminal(TerminalResult),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Allocation {
    fence: Fence,
    revision: u64,
    phase: Phase,
}

impl Allocation {
    /// Create the execution record for an ALREADY admitted and reserved assignment.
    /// Admission, atomic insertion, run uniqueness and resource accounting belong
    /// to the composing application/store; this constructor performs no effects.
    pub const fn assigned(fence: Fence) -> Self {
        Self {
            fence,
            revision: 0,
            phase: Phase::Assigned,
        }
    }

    /// Structural reconstitution only: the caller separately verifies canonical
    /// state identity, durability, and current authority.
    pub fn restore(fence: Fence, revision: u64, phase: Phase) -> Result<Self, TransitionError> {
        let valid = match phase {
            Phase::Assigned => revision == 0,
            Phase::Dispatching => revision == 1,
            Phase::Running | Phase::Unknown | Phase::RejectedBeforeStart => revision == 2,
            Phase::Terminal(_) => revision == 2 || revision == 3,
        };
        if !valid {
            return Err(TransitionError::InvalidRestoredRevision);
        }
        Ok(Self { fence, revision, phase })
    }

    pub const fn fence(&self) -> Fence {
        self.fence
    }
    pub const fn revision(&self) -> u64 {
        self.revision
    }
    pub const fn phase(&self) -> Phase {
        self.phase
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    BeginDispatch,
    StartObserved,
    StartRejected,
    StartUncertain,
    Complete(TerminalResult),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionError {
    InvalidRestoredRevision,
    FenceMismatch,
    IllegalTransition { phase: Phase, event: Event },
    RevisionExhausted,
}

/// A proposal, not a commit or authorization. Fields cannot be forged by a shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    expected: Allocation,
    next: Allocation,
}

impl Transition {
    pub const fn expected(&self) -> Allocation {
        self.expected
    }
    pub const fn next(&self) -> Allocation {
        self.next
    }
}

/// Deterministic transition; rejection cannot mutate the input.
/// There is no automatic retry, reassignment, timeout, or resource-release event.
/// Current fenced terminal observations may reconcile an uncertain start, but
/// the shell must independently admit that observation and its evidence.
pub fn transition(current: &Allocation, fence: Fence, event: Event) -> Result<Transition, TransitionError> {
    if fence != current.fence {
        return Err(TransitionError::FenceMismatch);
    }
    let phase = match (current.phase, event) {
        (Phase::Assigned, Event::BeginDispatch) => Phase::Dispatching,
        (Phase::Dispatching, Event::StartObserved) => Phase::Running,
        (Phase::Dispatching, Event::StartRejected) => Phase::RejectedBeforeStart,
        (Phase::Dispatching, Event::StartUncertain) => Phase::Unknown,
        (Phase::Dispatching | Phase::Running | Phase::Unknown, Event::Complete(result)) => Phase::Terminal(result),
        (phase, event) => return Err(TransitionError::IllegalTransition { phase, event }),
    };
    let revision = current.revision.checked_add(1).ok_or(TransitionError::RevisionExhausted)?;
    Ok(Transition {
        expected: *current,
        next: Allocation { fence, revision, phase },
    })
}

#[cfg(test)]
mod overflow_tests {
    use super::*;

    #[test]
    fn revision_overflow_fails_without_mutation() {
        let fence = Fence {
            run: RunId([1; 32]),
            assignment: AssignmentRef([2; 32]),
            workload: WorkloadRef([3; 32]),
            node: NodeRef([4; 32]),
            generation: NonZeroU64::new(1).unwrap(),
            epoch: NonZeroU64::new(1).unwrap(),
        };
        let state = Allocation {
            fence,
            revision: u64::MAX,
            phase: Phase::Assigned,
        };
        assert_eq!(transition(&state, fence, Event::BeginDispatch), Err(TransitionError::RevisionExhausted));
        assert_eq!(state.revision(), u64::MAX);
        assert_eq!(state.phase(), Phase::Assigned);
    }
}
