## Why

Node control startup, service locking, heartbeats, supervisor decisions, stale-lock recovery, live listener drains, run-loop ticks, shutdown, and duplicate-runner denial are currently spread across daemon receipts and shell behavior. The receipts name important checks, but the service lifecycle itself should be modeled as a deterministic state machine.

An explicit node-control service FSM makes it easier to prove single-active-service behavior, stale-lock recovery, bounded restart policy, shutdown drain semantics, and deny-before-side-effects behavior.

## What Changes

- Define a node-control service lifecycle FSM for startup, active lock ownership, serving, draining, stopped, stale-lock recovery, restart admission, and duplicate-runner denial.
- Bind service lock, heartbeat, supervisor, service-run, live-listener, loop, and shutdown receipts to explicit service state transitions.
- Keep filesystem locks, control sockets, live Iroh, and receipt file writes in shell code; keep transition planning pure.
- Add positive and negative tests for normal serve/shutdown, duplicate runner denial, stale lock with and without recovery policy, heartbeat timeout, restart bound, and shutdown drain.

## Impact

Node-control service behavior becomes reviewable without relying on filesystem timing or live Iroh. Operators get clearer receipts for why a service advanced, drained, recovered, or denied before mutation.