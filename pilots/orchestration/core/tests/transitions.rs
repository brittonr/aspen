// SPDX-License-Identifier: AGPL-3.0-or-later
use core::num::NonZeroU64;

use molten_orchestrator_core::*;

fn fence() -> Fence {
    Fence {
        run: RunId([1; 32]),
        assignment: AssignmentRef([2; 32]),
        workload: WorkloadRef([3; 32]),
        node: NodeRef([4; 32]),
        generation: NonZeroU64::new(1).unwrap(),
        epoch: NonZeroU64::new(1).unwrap(),
    }
}
fn next(state: Allocation, event: Event) -> Allocation {
    transition(&state, state.fence(), event).unwrap().next()
}
fn dispatched() -> Allocation {
    next(Allocation::assigned(fence()), Event::BeginDispatch)
}
fn result() -> TerminalResult {
    TerminalResult::Succeeded(ResultRef([7; 32]))
}

#[test]
fn plan_is_pure_and_repeatable() {
    let state = Allocation::assigned(fence());
    let a = transition(&state, fence(), Event::BeginDispatch).unwrap();
    assert_eq!(a, transition(&state, fence(), Event::BeginDispatch).unwrap());
    assert_eq!(state.phase(), Phase::Assigned);
    assert_eq!(state.revision(), 0);
    assert_eq!(a.expected(), state);
    assert_eq!(a.next().revision(), 1);
}

#[test]
fn successful_lifecycle_advances_revisions() {
    let running = next(dispatched(), Event::StartObserved);
    assert_eq!(running.phase(), Phase::Running);
    let done = next(running, Event::Complete(result()));
    assert_eq!(done.phase(), Phase::Terminal(result()));
    assert_eq!(done.revision(), 3);
}

#[test]
fn every_fence_dimension_is_checked_before_mutation() {
    let state = dispatched();
    let mut variants = [fence(); 6];
    variants[0].run = RunId([9; 32]);
    variants[1].assignment = AssignmentRef([9; 32]);
    variants[2].workload = WorkloadRef([9; 32]);
    variants[3].node = NodeRef([9; 32]);
    variants[4].generation = NonZeroU64::new(2).unwrap();
    variants[5].epoch = NonZeroU64::new(2).unwrap();
    for stale in variants {
        assert_eq!(transition(&state, stale, Event::Complete(result())), Err(TransitionError::FenceMismatch));
        assert_eq!(state, dispatched());
    }
}

#[test]
fn same_owner_from_earlier_attempt_is_rejected() {
    let old = fence();
    let current = Fence {
        epoch: NonZeroU64::new(3).unwrap(),
        ..old
    };
    let state = next(Allocation::assigned(current), Event::BeginDispatch);
    assert_eq!(transition(&state, old, Event::Complete(result())), Err(TransitionError::FenceMismatch));
    assert!(transition(&state, current, Event::Complete(result())).is_ok());
}

#[test]
fn uncertain_start_is_not_a_retry_grant() {
    let unknown = next(dispatched(), Event::StartUncertain);
    assert_eq!(unknown.phase(), Phase::Unknown);
    assert!(transition(&unknown, fence(), Event::BeginDispatch).is_err());
    assert!(transition(&unknown, fence(), Event::StartRejected).is_err());
    assert!(transition(&unknown, fence(), Event::Complete(result())).is_ok());
}

#[test]
fn persisted_dispatch_cannot_be_dispatched_again() {
    assert!(transition(&dispatched(), fence(), Event::BeginDispatch).is_err());
}

#[test]
fn completion_before_dispatch_is_denied() {
    assert!(transition(&Allocation::assigned(fence()), fence(), Event::Complete(result())).is_err());
}

#[test]
fn terminal_and_definite_rejection_are_absorbing() {
    let states = [
        next(dispatched(), Event::Complete(result())),
        next(dispatched(), Event::StartRejected),
    ];
    for state in states {
        for event in [
            Event::BeginDispatch,
            Event::StartObserved,
            Event::StartUncertain,
            Event::StartRejected,
            Event::Complete(result()),
        ] {
            assert!(transition(&state, fence(), event).is_err());
        }
    }
}

#[test]
fn bounded_sequences_never_dispatch_twice_or_change_fence() {
    let events = [
        Event::BeginDispatch,
        Event::StartObserved,
        Event::StartRejected,
        Event::StartUncertain,
        Event::Complete(result()),
    ];
    // Exhaust every length-6 sequence, including repeated/invalid observations.
    for mut encoded in 0..5_u32.pow(6) {
        let mut state = Allocation::assigned(fence());
        let mut dispatches = 0;
        let mut completions = 0;
        for _ in 0..6 {
            let event = events[(encoded % 5) as usize];
            encoded /= 5;
            let before = state;
            if let Ok(proposal) = transition(&state, fence(), event) {
                state = proposal.next();
                dispatches += u8::from(event == Event::BeginDispatch);
                completions += u8::from(matches!(event, Event::Complete(_)));
                assert_eq!(state.revision(), before.revision() + 1);
            } else {
                assert_eq!(state, before);
            }
            assert_eq!(state.fence(), fence());
            assert!(dispatches <= 1);
            assert!(completions <= 1);
        }
    }
}
