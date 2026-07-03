## Why

Delivery state governs whether side effects occur once, are replayed as duplicates, or deny as stale/gap/conflict cases. To prove the state machine, we need traces showing first commit, duplicate suppression, retry-before-side-effects, stale denial, and replay-log equivalence.

## What Changes

- Add idempotency state proof requirements for delivery windows and operation ids.
- Require no-side-effect evidence for duplicate, stale, gap, and conflict denials.
- Require replay delivery logs to produce deterministic runtime events without live network reads.

## Impact

- **Files**: delivery idempotency, remote dataspace delivery, and replay tests.
- **Testing**: generated delivery sequences, duplicate and stale negative tests, replay log equivalence, and receipt binding checks.
