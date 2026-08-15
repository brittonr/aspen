# Proposal: Node Control Supervisor Policy

## Summary
Add canonical supervisor policy artifacts and supervisor receipts so `molten node serve` can make restart, stale-lock, duplicate-runner, heartbeat, and shutdown-drain decisions with durable provenance.

## Motivation
The supervised runner already has service locks and bounded ticks, but its recovery and restart behavior is implicit. Operators need fail-closed policy evidence for stale service locks, bounded restart attempts, duplicate runner denial, and graceful shutdown drain limits before live/remote node control becomes routine.

## Scope
- Canonical `node-control-supervisor-policy-v1` artifacts covering max restarts, restart windows, heartbeat timeout, shutdown drain bounds, and stale-lock recovery.
- Canonical `node-control-supervisor-receipt-v1` receipts for restart admission/denial, stale lock recovery, duplicate runner denial, and shutdown drain outcomes.
- CLI helper `molten node supervisor-policy-fixture` and `molten node serve --supervisor-policy`.
- Service-run receipts that bind optional supervisor policy and supervisor receipt refs.
- Tests for stale service lock fail-closed behavior, policy-admitted recovery, duplicate runner denial, bounded restarts, and shutdown drain receipts.

## Out of Scope
- A long-running unbounded process supervisor.
- Treating supervisor policy as operation authority or payload provenance.
- Replacing service/runtime supervision beyond the node-control service runner.
