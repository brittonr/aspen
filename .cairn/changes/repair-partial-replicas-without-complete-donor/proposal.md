# Repair partial replicas without a complete donor

## Why

The content-replication extension separates replica policy from byte verification and authority, and permits stale replicas to supply verified bytes. `repair-lost-replicas-after-prior-success` owns the stale-success reuse defect. No tracked change owns the harder recovery shape: several reachable replicas each hold corrupted or missing objects, no single replica holds the complete required closure, but every required object has at least one intact, verifiable copy somewhere in the cohort.

Today a repair plan that selects donors replica-by-replica can conclude that no eligible donor exists because each candidate replica is individually incomplete. The TigerBeetle review (2026-09) identified closure-granularity repair as the useful borrow from physical determinism, applied at the canonical-content boundary rather than unrelated storage files. This is a design review finding; no distributed repair scenario was executed.

## What Changes

- Add a bounded repair scenario where the required content closure is assembled from multiple partial donors, with per-object verification before admission into the repaired set. r[molten.closure_repair.per_object]
- Require the planner to treat donor eligibility per object, not per replica; an incomplete replica is eligible as a donor for the objects it holds intact. r[molten.closure_repair.donor_eligibility]
- Record a typed incomplete-repair outcome when any required object has no intact verifiable copy anywhere, without discarding progress on the objects that were repaired. r[molten.closure_repair.incomplete_outcome]
- Preserve the existing boundaries: verified bytes match already-established identities, repair never establishes which world head is authoritative, and stale replicas supplying verified bytes do not count as current placement. r[molten.closure_repair.boundary]

## Impact

- **Files**: `molten-core` content-replication repair planning and verification transitions, simulation fixtures, operator status, docs.
- **Testing**: multi-partial-donor closure repair, per-object eligibility, zero-intact-copy incomplete outcome, identity-mismatch rejection, and non-claims for authority.
- **Non-goals**: no claim that repair restores authority, no demand for byte-identical unrelated storage files, no change to the stale-success rules owned by `repair-lost-replicas-after-prior-success`.

## Dependencies

- `repair-lost-replicas-after-prior-success` owns fresh availability evidence versus historical success; keep scopes separate.
- `add-world-head-rollback-witnessing` owns authority recovery; this change verifies bytes only.
