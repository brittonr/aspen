## Why

Aspen has Redb-backed state, content-addressed chunks, snapshots, and actor-turn atomicity, but system extensions lack generic durable-log, ordered-store, snapshot, atomic-batch, and effect-transaction ports. A database, replicated log, scheduler, or object store would otherwise have to reach into storage internals, assume actor semantics, or build persistence outside capability and simulation boundaries.

## What Changes

- Add versioned local durable-state ports for append logs, ordered key/value state, immutable snapshot objects, checkpoints, and bounded atomic batches.
- Define explicit durability levels, flush boundaries, compare/precondition behavior, atomicity domains, retention, truncation, compaction, and recovery semantics.
- Generalize reserve, commit, abort, inspect, and reconcile as effect-transaction operations for selected multi-step shell effects.
- Represent success, failure, cancellation, and uncertain outcomes explicitly, with idempotency and generation fencing where supported.
- Provide production storage adapters and deterministic crash/recovery adapters behind the same contracts.
- Preserve explicit non-claims: local durability is not replication, consensus, distributed transactions, or extension-level correctness.

## Impact

- **Files**: canonical durable-state models, adapter registry, Redb/chunk integration, system-extension effects, simulation disk model, checkpoint/recovery paths, operator readback, fixtures, and a new `durable-state-ports` accepted spec.
- **Testing**: adapter conformance, ordered scans, atomic batches, append/flush/truncate, snapshots, crash points, torn/uncertain outcomes, idempotency, fencing, reserve/commit/abort recovery, quota, and corruption tests.
- **Safety**: every adapter declares its exact atomicity and durability domain; cross-port or distributed guarantees are denied unless a separate consistency protocol supplies and proves them.
