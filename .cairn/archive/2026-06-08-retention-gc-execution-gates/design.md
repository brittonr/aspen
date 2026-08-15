## Context

Dry-run plans are non-authoritative, and apply-from-plan already recomputes and writes retention tombstone evidence before content mutation. The remaining gap is the final subsystem mutation boundary: callers can still ask ledger/chunk/cache cleanup to mutate by supplying destructive evidence directly.

## Decisions

### 1. Execution gate receipts are per candidate

**Choice:** Introduce `retention-gc-execute-v1` as a per-object gate receipt that verifies a provided apply ref matches subsystem, action, object ref, object kind, retention class, pass decision, unchanged apply plan, retention receipt ref, and tombstone ref.

**Rationale:** Ledger, chunk, and cache GC can select multiple objects. Per-candidate receipts avoid treating a batch-level plan as authority for unrelated objects.

### 2. Subsystems still run normal retention admission

**Choice:** A passing execution gate is necessary but not sufficient. Subsystems must still run destructive admission and retention evaluation immediately before mutation and deny if those checks drift.

**Rationale:** Apply receipts are evidence only. Fresh pins, retained dependencies, stale admissions, or remote clearance changes must still block mutation.

### 3. Dry-run remains available without apply refs

**Choice:** Dry-run GC paths may omit apply refs. Non-dry-run mutation paths require matching apply refs and record execution gate refs in subsystem receipts.

**Rationale:** Operators need exploration without side effects, while physical mutation must be plan/apply-bound.

## Risks / Trade-offs

- Multi-object GC callers must supply one apply ref per selected object before mutation.
- Apply receipts can become stale as retention state changes; normal retention evaluation catches that drift before deletion.
