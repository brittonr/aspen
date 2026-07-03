## Why

Vat promises, near/far refs, actormap transactions, revocation cleanup, and distributed ref lifetimes are state machines with authority consequences. They should prove that pending/resolved/broken/cancelled states, handoffs, and revocations cannot mint authority or leave stale live refs reachable.

## What Changes

- Add requirements for promise, reference lifetime, and actormap transaction proof traces.
- Require generated or fixture traces for pending, resolution, breakage, cancellation, handoff, stale-use denial, and revocation cleanup.
- Require negative evidence for synchronous far calls, stale distributed refs, unresolved pipeline use, rollback leaks, and authority amplification.

## Impact

- **Files**: runtime vat fixtures, runtime predicates, lifecycle cleanup, authority snapshot predicates, and tests.
- **Testing**: valid promise/ref lifetimes, transaction commit/rollback, revocation cleanup, stale-use denial, and no-authority-amplification checks.
