# Tasks: generated-distributed-fault-interleavings

## Phase 1: Generator model

- [x] [parallel] r[molten.testing.distributed_simulation.generated_fault_interleavings] Add bounded Hegel generators for distributed topology, scheduler profile, command sequence, fault plan, and evidence ref presence.
- [x] [parallel] r[molten.testing.distributed_simulation.generated_repro_seed] Define a canonical generated-case repro artifact that binds seed, topology, scheduler, fault plan, commands, invariant name, diagnostics, and receipt refs.

## Phase 2: Property coverage

- [x] [serial] r[molten.testing.distributed_simulation.generated_fault_interleavings] Add positive generated properties for deterministic replay, idempotent duplicates, restart stability, and benign fault convergence.
- [x] [serial] r[molten.testing.distributed_simulation.generated_fault_interleavings] Add negative generated properties for missing authority, unauthorized transport, stale evidence, corrupted receipts, resource pressure, ambient drift, and partitioned quorum.

## Phase 3: Repro and promotion workflow

- [x] [parallel] r[molten.testing.distributed_simulation.generated_repro_seed] Add failing-seed readback and documentation that explains how to promote a generated failure into a named fixture.
- [x] [serial] r[molten.testing.distributed_simulation.generated_repro_seed] Run focused generated-property tests and `cairn validate --root .`, or record the blocker and next best check.
