# Implementation review: committed-change subscriber conformance

## Disposition

The remaining subscriber fault-conformance implementation is complete. This is
an implementation review, not independent security approval or production
readiness authority. Executed results and retained failed attempts are in
`evidence/2026-10-26-subscriber-verification.md`.

## Reviewed invariants

- The reproduced loss occurs only at the wrong within-revision comparison:
  `StreamCursor::after` already advances the ordinal. Inclusive subscription
  restores the next-event contract without changing event/cursor identities.
- The exercise invokes actual stream commit, subscribe and dedup functions. It
  checkpoints a baseline, publishes one multi-event revision, consumes an
  unacknowledged page and restores only the saved checkpoint. It does not
  manufacture successful deliveries or claim an OS-process restart.
- Case validation bounds event count and page size before allocation. Evidence
  revalidation checks vector lengths before indexing, exact service/epoch/phase
  bindings, one modeled restart, unchanged checkpoint, canonical event identities,
  complete replay and exact deduplication.
- Missing evidence yields failed conformance. Receipt revalidation recomputes the
  subscriber diagnostics and both subscriber/aggregate dispositions. An honestly
  failed receipt remains serializable; falsely passing evidence is rejected.
- Profile and canonical receipt cohorts advance together to v2, including source
  schemas, identity domains, Nickel contracts/export and Nix inventories. No v1
  compatibility alias or default success is retained.
- The focused policy compilation includes the actual committed-stream mechanism.
  Policy findings were repaired, not suppressed. Integer lengths retain the same
  eight-byte encoding; duplicate accumulation stays input-bounded and ordered.
- The projection-only permanent test was removed instead of extending copy
  assertions. Permanent regressions target loss, replay, tampered evidence and
  receipt identity. Executed smoke and Nix checks cover profile/export drift.
- Dataspace Observe, durable-work delivery, election/lease decisions, content
  authorization, root dependencies and runtime enrollment remain unchanged.
- These requirement IDs were already accepted by the earlier partial sync. The
  delta now declares MODIFIED requirements, permitting the reviewed cursor
  scenarios to replace that block while preserving the other requirement bodies.
  The first preview refused the old ADDED operation without mutating accepted specs.

## Limits

The evidence covers the named deterministic in-process subscriber scenario and
bounded shell/receipt integration. It proves neither persistent subscriber
checkpointing nor actual process-crash recovery, physical power loss, arbitrary
storage correctness, whole-system correctness or release eligibility. Existing
world-witness and physical-failure non-claims remain required in receipts.
