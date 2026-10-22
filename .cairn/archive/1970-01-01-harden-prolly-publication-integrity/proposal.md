# Proposal: Harden Prolly publication integrity

## Why

A static review on 2026-09-09 (base `87c289bf68c4`) found that the Prolly adapter
classifies every Redb error as a definite, non-unknown outcome
(`src/prolly_map/store.rs`, `redb_error` sets `outcome_unknown = false`), while
`publish_prolly_edit` only routes `outcome_unknown = true` errors and explicit
`Unknown` observations into `reconcile_publication`. Redb's documented semantics warn
that a failed `WriteTransaction::commit()` may already have become durable. The local
adapter therefore cannot reconcile a commit failure that the reconciliation path was
designed to handle.

The same review found that the GC boundary deletes candidate blocks using admission
flags supplied by the caller (`generation_current`, candidate inventories) without the
port making the current-head and pin comparison indivisible with deletion. A stale
admission created before a later root publication can remove newly reachable or
protected content, including blocks staged for an in-progress publication.

These are correctness findings at the adapter contract level. They must land before
any batching, incremental-edit, or asynchronous publication work.

## What Changes

- Introduce phase-aware error classification in the Prolly Redb adapter:
  validation and pre-mutation failures remain definite rejections; commit failures
  whose durability outcome is not established become `outcome_unknown = true` and
  enter reconciliation. Staging-block uncertainty and head-publication uncertainty
  are classified separately. r[molten.prolly.publication.classification]
- Extend publication reconciliation with explicit readback classes: reopened store
  contains the exact successor root means applied; the exact predecessor means not
  applied; conflicting, corrupt, unavailable, or inconclusive readback remains
  unknown or quarantined. r[molten.prolly.publication.reconcile]
- Replace unconditional `delete_blocks` at the GC port with a
  compare-retention-inventory-and-delete operation whose head, pin, and
  retention-generation check is indivisible with deletion in the owning adapter or
  single-writer owner. Blocks staged for an in-progress publication are protected.
  r[molten.prolly.gc.currentness]
- Add positive and negative regression tests, including commit-point fault
  injection after possible durability and publication between GC planning and
  deletion. r[molten.prolly.publication.tests]

## Impact

- `src/prolly_map/store.rs`, `src/prolly_map/service.rs`, `src/prolly_map/ports.rs`,
  and the `ProllyBlockStorePort` trait in `crates/molten-core/src/prolly_map/`.
- Fault-conformance fixtures under the existing semantic fault framework.
- No change to canonical profiles, node encoding, or published root identity.

## Out of Scope

- Replacing Redb with another engine; importing a second replication stack.
- Performance work (read sessions, incremental edits, diff skipping) — those are
  tracked in `prolly-storage-work-reduction` and land after this change.
- Changing world-promotion semantics beyond reusing its existing
  transactional-reconciliation interpretation of "unknown".

## Affected Specs

- `prolly-publication-integrity`: phase-aware commit classification, readback
  reconciliation classes, GC currentness at the port boundary, and fault coverage.
