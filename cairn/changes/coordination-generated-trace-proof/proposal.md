## Why

Coordination invariants are stateful: mutual exclusion, fencing monotonicity, FIFO order, semaphore bounds, duplicate replay, and denial no-mutation only become convincing across generated request sequences. Existing examples cover important paths; generated traces turn those examples into a bounded proof harness.

## What Changes

- Add generated coordination trace proof requirements.
- Require denial no-mutation checks after every denied generated operation.
- Require duplicate operation replay checks inside generated traces.

## Impact

- **Files**: coordination tests and, if needed, pure trace-step helpers.
- **Testing**: Hegel generated traces across lock, queue, semaphore, rate-limit, election, and barrier surfaces as implemented.
