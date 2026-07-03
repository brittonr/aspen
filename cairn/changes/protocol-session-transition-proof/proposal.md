## Why

Protocol sessions are another finite state machine: projected endpoint states should only accept legal send, receive, branch, and offer transitions. Invalid operation receipts must not be accepted as session evidence, or choreography projectability stops being a useful safety boundary.

## What Changes

- Add requirements for endpoint operation transition legality.
- Require generated/local proof traces over projected protocol session states.
- Require lifecycle gate replay to reject missing, stale, ambiguous, or out-of-order operation evidence.

## Impact

- **Files**: protocol session operation/gate logic and tests.
- **Testing**: valid send/receive/branch/offer traces, invalid operation denial, missing terminal evidence denial, and receipt binding checks.
