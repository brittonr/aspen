## Why

Molten's core evidence and local-node workflows are now substantial, but production use also needs operational muscle: packaging, state lifecycle, observability, backup/restore, upgrade/rollback, incident response, and operator runbooks. Without those drills, a passing dogfood receipt proves code-path health but not that an operator can safely run, recover, and upgrade a real node.

## What Changes

- Define production node deployment profiles and configuration boundaries.
- Add backup/restore and state-integrity drill evidence for ledgers, Redb stores, chunks, identity, receipts, and retention state.
- Add structured observability/SLO checks for node, adapters, queues, receipts, resource pressure, and live transport.
- Add upgrade, rollback, and restart-recovery drills with canonical receipts.
- Add operator runbooks for startup, shutdown, source-gate refresh, evidence export, backup, restore, upgrade, rollback, and emergency stop.

## Impact

This work makes the difference between a deterministic demo and an operable service. It remains evidence-oriented: operational receipts prove that the operator workflow ran and was reviewed; they do not grant authority or bypass subsystem gates.
