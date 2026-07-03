## Why

Retention GC is a destructive state machine over plan, apply, execute, audit, receipts, tombstones, and remote clearance. Existing gates encode the lifecycle, but release review needs proof that drift, missing admission, missing apply refs, and stale remote-clearance evidence deny before deletion, invalidation, tombstoning, or redaction side effects.

## What Changes

- Add requirements for retention GC lifecycle proof traces.
- Require plan recomputation and apply/execute/audit binding checks as proof obligations.
- Require negative evidence for drift, missing admission, missing tombstone, missing apply ref, stale remote clearance, and no-mutation denial.

## Impact

- **Files**: retention GC core, destructive subsystem call sites, audit/readback tests, and proof fixtures.
- **Testing**: plan/apply/execute/audit pass trace, drift denial, missing apply denial, stale clearance denial, and state/content unchanged on denial.
