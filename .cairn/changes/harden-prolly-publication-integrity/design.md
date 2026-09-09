# Design: Harden Prolly publication integrity

## Context

The review at base `87c289bf68c4` established two adapter-contract gaps:

1. `redb_error` in `src/prolly_map/store.rs` maps every Redb error through
   `ProllyPortError::new(..., false)`. `publish_prolly_edit` in
   `src/prolly_map/service.rs` reconciles only on `outcome_unknown == true` or an
   explicit `Unknown` observation. Commit errors therefore bypass reconciliation even
   though Redb documents that a failed commit may already be durable.
2. `execute_prolly_gc` authorizes deletion from caller-supplied admission flags and
   then calls `port.delete_blocks(&plan.candidate_unreachable)`. The port performs no
   indivisible recheck of durable heads or pins, so a stale admission can delete
   content that became reachable or protected after the admission was built.

## Goals

- Ambiguous publication outcomes stay ambiguous until reconciliation resolves them.
- Stale deletion admissions cannot remove newly reachable, pinned, or
  in-flight-publication content.
- Preserve the pure core: classification policy lives in `molten-core` as pure
  functions over typed observations; the Redb adapter owns only I/O facts.

## Non-Goals

- No change to canonical profiles, node encoding, or root identity.
- No performance optimization; `prolly-storage-work-reduction` follows this change.
- No new interpretation of "unknown": reuse the transactional-reconciliation model
  already used by world promotion.

## Approach

### Phase-aware classification

Introduce a pure classifier in `molten-core`:

```
classify_port_failure(phase, error) -> DefiniteRejection | NotApplied | Unknown
```

- `phase` distinguishes validation/admission, block staging, and root publication.
- Failures known to precede any mutation map to `NotApplied`.
- Commit failures on staging and publication map to `Unknown`; the port error carries
  `outcome_unknown = true` only for these phases.
- Read errors, codec errors, and validation errors keep `outcome_unknown = false`.

The adapter calls the classifier; it does not hand-assign booleans.

### Reconciliation readback classes

`reconcile_publication` gains explicit outcomes over a reopened store:

- exact successor head present → `Applied`
- exact predecessor head present → `NotApplied`
- conflicting head, corrupt head bytes, unavailable store, or inconclusive readback
  → remain `Unknown` or quarantine, per the existing world-promotion model.

Staging uncertainty is resolved separately from head uncertainty: staged blocks are
idempotent under identity-verified re-staging, so staging ambiguity never implies
publication ambiguity.

### GC compare-and-delete

Replace the deletion port signature with:

```
compare_retention_inventory_and_delete(
    expected_heads, expected_pins, expected_retention_generation, candidates
) -> GcDeletionOutcome
```

The Redb adapter performs the inventory comparison and the deletion inside one write
transaction. Mismatch or inconclusive state yields a typed outcome without deletion.
The pure core keeps planning and admission validation unchanged. Blocks staged for an
in-progress publication are protected by the pin inventory, which the publication
path must register before staging.

### Testing

- Positive: commit succeeds; readback classes resolve as specified; GC deletes only
  when the inventory matches.
- Negative: injected commit failure after possible durability (dev-only fault
  injection may reuse `komora-io` fault-injection as a dev-dependency, recorded with
  provenance); publication between GC planning and deletion; corrupted readback;
  stale admission against a newly advanced head.
- Reuse the existing semantic fault-conformance phases and restart readback rather
  than a new harness.

## Risks

- Changing the port trait is a breaking adapter change; both local and test adapters
  update in the same change.
- Over-classifying errors as unknown would surface quarantines on ordinary I/O
  failures; the classifier keeps the unknown class limited to commit-phase failures
  whose durability is not established.
