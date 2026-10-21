# Adopt command-pattern resource reservation

## Why

The reference scheduler evaluates completions but places resources through a client-mediated sequence: read capacity, separately select a worker, and later write an allocation based on potentially outdated observations. Concurrent actors can interleave between those steps, so correctness depends on the client's observation freshness rather than on the authoritative state. The TigerBeetle execution-pattern review (2026-09) identified the single-command shape — evaluate the whole operation against current authoritative state, then commit — as the useful borrow, without introducing a second source of truth beside the existing reservation owner.

Two related gaps ride on the same contract. Batch admission lacks explicit item-count, byte-count, and waiting-time limits with per-item identities and results. And lease expiry is bookkeeping only: expiring a lease does not stop an external job, yet nothing records that capacity must not be treated as safely reusable unless the enforcement or termination contract supports it. Both findings are static source review; no contended-reservation or external-stop fault was executed.

## What Changes

- Add a single command-pattern reservation transition — reserve resources for a job with a logical operation and an expected generation, evaluated atomically against current authoritative state. r[molten.reservation_command.atomic_eval]
- Keep capacity, selection, and allocation decisions inside the pure transition over authoritative state; the shell only executes the decided effects. r[molten.reservation_command.pure_core]
- Add bounded batch admission with explicit item-count, byte-count, and waiting-time limits, per-item identities, and per-item results; batching never silently changes which operations are atomic together. r[molten.reservation_command.bounded_batches]
- Require that expired-lease capacity is treated as reusable only when the enforcement or termination contract supports the conclusion; expiry alone records staleness, not release. r[molten.reservation_command.expiry_not_release]

## Impact

- **Files**: `molten-core` reference scheduler transitions and operations, resource-accounting types, batch admission, fabric-time docs and receipt fixtures.
- **Testing**: contended-reservation traces, stale-generation rejection, batch-limit boundaries, per-item outcome isolation, expiry-without-enforcement retention, capacity accounting after every path.
- **Non-goals**: no second reservation ledger, no adoption of the two-phase transfer wire format, no claim that lease expiry stops external jobs, no performance claim.

## Dependencies

- `resume-blocked-scheduler-runnables` owns blocked-runnable capacity accounting; capacity composition tests there must keep passing.
- `bind-scheduler-completions-to-lease-epoch` owns the completion fencing token; this change consumes generation identity from the same fencing source.
- `add-protocol-aware-simulation-oracles` owns cost measurement; this change owns the API shape and limits, not benchmarking.
