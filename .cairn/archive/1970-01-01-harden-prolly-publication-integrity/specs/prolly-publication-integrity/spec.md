# Prolly Publication Integrity Specification

## ADDED Requirements

### Requirement: Phase-aware commit error classification

r[molten.prolly.publication.classification] The Prolly adapter MUST classify port
failures by phase: failures known to precede mutation MUST report a definite outcome;
commit failures on block staging or root publication whose durability outcome is not
established MUST report `outcome_unknown = true`; read, codec, and validation errors
MUST NOT report `outcome_unknown = true`. Staging uncertainty and publication
uncertainty MUST be classified separately.

#### Scenario: Commit failure enters reconciliation

- GIVEN a `compare_and_advance` write whose `commit()` fails after possible durability
- WHEN the adapter classifies the failure
- THEN the resulting port error MUST carry `outcome_unknown = true` and
  `publish_prolly_edit` MUST route the edit into reconciliation.

#### Scenario: Validation failure stays definite

- GIVEN a snapshot that fails validation before any write begins
- WHEN the adapter classifies the failure
- THEN the error MUST NOT carry `outcome_unknown = true` and MUST NOT enter
  reconciliation.

#### Scenario: Staging ambiguity stays separate

- GIVEN a staging `commit()` failure whose durability is not established
- WHEN the adapter classifies the failure
- THEN the classification MUST NOT by itself mark the head publication unknown.

### Requirement: Reconciliation resolves readback into explicit classes

r[molten.prolly.publication.reconcile] Publication reconciliation MUST resolve
uncertainty by reading back the durable store: the exact successor root MUST resolve
to applied; the exact predecessor root MUST resolve to not applied; a conflicting,
corrupt, unavailable, or inconclusive readback MUST remain unknown or enter
quarantine consistent with the existing world-promotion model.

#### Scenario: Successor readback resolves applied

- GIVEN reconciliation after an unknown commit outcome
- WHEN the reopened store contains the exact expected successor root and generation
- THEN the publication MUST resolve to applied with a recorded observation.

#### Scenario: Inconclusive readback stays unresolved

- GIVEN reconciliation after an unknown commit outcome
- WHEN the readback fails, is corrupt, or shows a conflicting head
- THEN the outcome MUST remain unknown or quarantined and MUST NOT be reported as
  applied or not applied.

### Requirement: GC deletion is indivisible with retention currentness

r[molten.prolly.gc.currentness] The deletion boundary MUST compare current durable
heads, pins, and the retention generation against the admission inventory and perform
that comparison indivisibly with deletion in the owning adapter or single-writer
owner. A block reachable from an admitted root, protected by a current pin, or
required by a protected in-progress publication MUST NOT be deleted.

#### Scenario: Stale admission cannot delete newly reachable content

- GIVEN a GC admission built before a later root publication makes a candidate block
  reachable
- WHEN the compare-and-delete operation executes against the current store
- THEN the operation MUST refuse deletion of the newly reachable block and MUST
  return a typed mismatch outcome without mutating storage.

#### Scenario: In-progress publication blocks are protected

- GIVEN blocks staged for a publication whose head has not advanced
- WHEN any GC deletion executes
- THEN the staged blocks MUST be excluded by the pin inventory and MUST remain
  present after the deletion completes.

### Requirement: Fault coverage spans commit, GC, and readback

r[molten.prolly.publication.tests] The change MUST add positive and negative
regression tests covering: commit failure after possible durability, publication
between GC planning and deletion, corrupted or missing readback, and stale admission
against an advanced head. Any third-party fault-injection dependency MUST be a
dev-dependency with recorded source provenance.

#### Scenario: Fault fixtures fail against the previous implementation

- GIVEN the pre-change adapter that classifies all Redb errors as definite and
  deletes on admission flags alone
- WHEN the new fault fixtures run against that implementation
- THEN each fixture MUST fail, demonstrating the fixture detects the defect.

#### Scenario: Coverage is anchored to the fault framework

- GIVEN the existing semantic fault-conformance phases with restart readback
- WHEN the new tests are authored
- THEN they MUST extend the existing framework phases rather than introduce a
  disconnected harness.
