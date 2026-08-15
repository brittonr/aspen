## Phase 1: Bounded control loop

- [x] [serial] r[molten.node_control_loop.spec.bounded_loop] Add a bounded node control loop that drains inbox requests in deterministic order.
- [x] [serial] r[molten.node_control_loop.spec.heartbeat_receipts] Emit startup-lock-bound heartbeat and loop receipts for each loop run.

## Phase 2: Idempotency and shutdown

- [x] [serial] r[molten.node_control_loop.spec.idempotent_duplicates] Return prior control receipts for duplicate request refs without re-running side effects.
- [x] [serial] r[molten.node_control_loop.spec.shutdown_stops_loop] Stop the loop after a passing shutdown dispatch removes the active lock.

## Phase 3: CLI and coverage

- [x] [serial] r[molten.node_control_loop.spec.cli] Add `molten node run-loop --state-root ... --max-requests ...`.
- [x] [parallel] r[molten.node_control_loop.spec.tests] Add tests for multi-request loop dispatch, duplicate idempotency, shutdown stop, stale lock denial, and CLI loop receipts.
