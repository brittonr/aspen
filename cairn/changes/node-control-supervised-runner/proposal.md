# Proposal: Node Control Supervised Runner

## Summary
Add a bounded supervised node runner that repeatedly delivers local-Iroh ingress envelopes, drains the durable control inbox, emits service receipts, and exits cleanly on shutdown.

## Motivation
Molten now has durable node control, provenance-gated operation dispatch, a bounded local control loop, and deterministic local-Iroh ingress. Operators need one command that keeps those pieces moving without introducing a direct-dispatch network bypass.

## Scope
- Canonical service lock, heartbeat, and run receipt artifacts.
- `molten node serve --state-root ...` with bounded tick controls for deterministic tests and long-lived operation.
- Deterministic scan of published local-Iroh ingress envelopes before each inbox drain.
- Reuse of existing `deliver_node_control_ingress` and `run_control_loop` paths.
- Duplicate active-runner denial before side effects.
- Tests for duplicate runner denial, ingress-to-dispatch, shutdown, and heartbeat continuity.

## Out of Scope
- Real live network listener or peer transport setup.
- Supervisor restart policy across process crashes.
- New node-control operations.
