## Design

The first loop profile is intentionally local and file-backed. `molten node run-loop` requires an explicit state root and an active `node-control-lock-v1` bound to the current startup receipt. It scans the control inbox in deterministic file-name order and dispatches at most `--max-requests` entries. The bound prevents unbounded work and gives operators a repeatable way to batch control processing in tests or scripts.

Each loop emits two new receipt artifacts:

- `node-control-heartbeat-receipt-v1` binds the active startup receipt, active lock ref, loop sequence, processed count, and local loop profile.
- `node-control-loop-receipt-v1` binds the startup receipt, heartbeat ref, request bound, processed request refs, dispatched control receipt refs, stop status, diagnostics, and checks for bounded processing, deterministic order, idempotent duplicate dispatch, and shutdown stop behavior.

Dispatch remains the only operation boundary. The loop calls the same dispatcher used by `control-dispatch`, so operation preflight, ledger-resolved payload checks, operation subreceipts, and final control receipts stay in one implementation. A passing `shutdown` request removes the active lock through the existing stop path; the loop observes that dispatched operation and exits immediately.

Duplicate request handling is request-ref based. If a request with a canonical ref already has an outbox control receipt, dispatch archives the duplicate request, writes a duplicate dispatch queue receipt, and returns the prior control receipt. If the archived evidence for that ref is malformed or conflicts with the duplicate, dispatch fails closed before re-running side effects.

## Non-goals

- No network socket or IPC listener is added.
- No autonomous background process management is added.
- No new authority model is added; requests still carry explicit authority, policy, and resource refs and side-effecting operations still fail closed before effects.
