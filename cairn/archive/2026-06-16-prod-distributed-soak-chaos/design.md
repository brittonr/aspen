## Context

Molten has local Iroh-shaped transport, live node-control send/import/listener workflows, Trellis workflow gates, remote dataspace envelopes, job workers, coordination receipts, delivery idempotency, and operator dogfood. The remaining question is how those surfaces behave together across time and faults.

## Design

### Soak topology

Define a minimal but production-shaped topology:

- at least two live nodes with persistent identities and explicit state roots;
- live peer tickets and authority grants;
- node-control workflow bundle export/verify/gate/apply/reconcile/ack;
- one remote dataspace/service exchange;
- one job worker assignment/execution path;
- one coordination operation with fencing/idempotency;
- evidence export from every node.

### Fault matrix

The initial fault matrix should cover:

- network delay/drop/partition/rejoin;
- duplicate, stale, and conflicting operation ids;
- stale or wrong peer tickets and authority grants;
- node restart during queued control work;
- partial artifact/chunk availability;
- resource pressure and queue bounds;
- corrupted or missing receipt artifacts;
- retention pins and tombstones during sync or cleanup.

### Evidence and replay

Every scenario should emit a canonical `prod-soak-run-v1` or equivalent receipt that binds node refs, topology refs, fault profile refs, child receipts, resource summaries, and pass/deny diagnostics. Deterministic or recorded paths should be replayed; inherently live observations should be marked non-replayable and excluded from deterministic pass claims unless recorded delivery logs exist.

### Non-goals

- Do not use soak evidence as authority.
- Do not claim formal consensus or transport correctness beyond the tested topology and fault matrix.
- Do not require internet-wide deployment before a constrained internal pilot.
