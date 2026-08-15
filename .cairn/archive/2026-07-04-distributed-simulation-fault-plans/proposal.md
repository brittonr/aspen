## Why

Molten has strong local, CLI, dogfood, and NixOS VM evidence, but distributed regressions are still expensive to localize when they first appear in VM or live soak checks. We need a cheap deterministic layer that can exercise message delay, drop, reorder, duplication, partition, rejoin, crash, restart, and resource pressure before the VM topology runs.

## What Changes

- Add a canonical seeded distributed fault-plan model for simulation runs.
- Add a pure deterministic multi-peer simulation core over virtual clocks, explicit queues, topology state, and injected faults.
- Emit canonical distributed test run receipts that bind topology, seed, fault plan, source/test identity, child refs, decisions, diagnostics, and allowed variance.
- Add property/model tests for distributed safety invariants such as idempotency, no transport-derived authority, deny-before-side-effects, and restart replay stability.
- Document how simulation evidence complements but does not replace VM, live soak, authority, policy, provenance, resource, source-gate, or retention gates.

## Impact

This creates the fast middle layer between pure unit tests and heavy VM checks. It should reduce feedback time for protocol and distributed-runtime changes while preserving the existing evidence-only boundaries.
