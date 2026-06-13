# Design: Node Control Supervised Runner

## Overview
`molten node serve` is a local supervisor shell around the existing node-control primitives. A serve tick performs three deterministic steps:

1. Acquire and refresh a service heartbeat bound to the active startup and service lock.
2. Scan the local-Iroh ingress topic in sorted path order and deliver every published envelope through `deliver_node_control_ingress`.
3. Drain the durable control inbox through `run_control_loop` with the configured per-tick request bound.

The runner never dispatches a request directly. Remote ingress continues to stop at the inbox boundary, and install/run provenance gates remain inside normal dispatch.

## Records
- `node-control-service-lock-v1` binds the active startup receipt, node identity, configured topic, max ticks, per-tick request bound, and a deterministic service run id.
- `node-control-service-heartbeat-receipt-v1` records each tick, the service lock ref, startup ref, cumulative ingress deliveries, cumulative dispatched requests, and diagnostics.
- `node-control-service-run-receipt-v1` records the final decision, service lock ref, startup ref, topic, tick count, heartbeat refs, ingress receipt refs, loop receipt refs, processed request refs, stop status, diagnostics, and checks.

## Locking and shutdown
The service lock is separate from the node active control lock. `serve` refuses to start if another service lock is present. A passing shutdown dispatch removes the active node control lock; the runner records `stopped=true`, removes the service lock, and emits the final service run receipt. Bounded runs that end because `--max-ticks` is reached also remove the service lock after writing the final receipt.

## Determinism
All scans are sorted by path/ref. Each tick is bounded by `--max-requests-per-tick`, and the runner is bounded by `--max-ticks` unless operators choose a high bound for long-lived use. Receipts capture every ingress delivery and loop receipt needed to replay what the supervisor drove.
