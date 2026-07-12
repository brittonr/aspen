## Phase 1: Durable-state contracts

- [x] [serial] Add canonical descriptors, commands, outcomes, ids, durability levels, atomicity domains, schemas, quotas, retention, and non-claims for durable logs, ordered stores, snapshots, and checkpoints. r[molten.fabric_durability.port_contracts]
- [x] [serial] Implement pure validation for namespaces, generations, keys, ranges, sequence positions, preconditions, batches, durability transitions, quotas, and profile compatibility. r[molten.fabric_durability.ordered_store] r[molten.fabric_durability.durable_log]
- [x] [parallel] Add positive port fixtures and negative malformed, out-of-domain, stale-generation, over-quota, unsupported-durability, and cross-adapter batch fixtures. r[molten.fabric_durability.port_contracts]

## Phase 2: Storage operations and adapters

- [x] [serial] Implement append, read, scan, flush, prefix or tail inspection, bounded truncate, and retention operations for the durable-log port. r[molten.fabric_durability.durable_log]
- [x] [parallel] Implement point reads, ordered range scans, compare/precondition writes, and bounded atomic batches for the ordered-store port. r[molten.fabric_durability.ordered_store] r[molten.fabric_durability.atomic_batch]
- [x] [parallel] Implement immutable snapshot, checkpoint, restore, inventory, and generation-fencing operations. r[molten.fabric_durability.snapshot_recovery]
- [x] [serial] Place live Redb and content-addressed storage shells behind the canonical ports without exposing backend handles. r[molten.fabric_durability.live_sim_parity]

## Phase 3: Effect transactions and recovery

- [x] [serial] Add canonical reserve, commit, abort, inspect, expiry, and reconcile transitions for declared effect-transaction profiles. r[molten.fabric_durability.effect_transaction]
- [x] [parallel] Represent buffered, durable, failed, cancelled, and uncertain outcomes and enforce idempotency and generation fencing where declared. r[molten.fabric_durability.uncertain_outcomes]
- [x] [parallel] Add startup inventory and fail-closed recovery for gaps, corruption, incompatible schema, stale generations, and unresolved effect transactions. r[molten.fabric_durability.snapshot_recovery]

## Phase 4: Deterministic simulation and evidence

- [x] [serial] Add the deterministic disk adapter with modeled buffering, flush, capacity, latency, crash points, partial external effects, and injected corruption. r[molten.fabric_durability.live_sim_parity]
- [x] [parallel] Add bounded commit, checkpoint, recovery, corruption, and aggregate resource evidence plus operator readback. r[molten.fabric_durability.evidence]
- [x] [parallel] Enforce local-only durability and atomicity non-claims in descriptors, receipts, and readback. r[molten.fabric_durability.non_claims]

## Phase 5: Validation

- [x] [serial] Run shared adapter conformance, ordering, atomic-batch, crash matrix, idempotency, fencing, effect-transaction, recovery, quota, corruption, and cleanup tests. r[molten.fabric_durability.final_validation]
- [x] [serial] Run Cairn validation and proposal, design, and tasks gates before sync and archive. r[molten.fabric_durability.final_validation]
