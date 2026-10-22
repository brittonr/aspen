# Tasks: Harden Prolly publication integrity

## Phase 1: Classification

- [x] [serial] Add the pure `classify_port_failure` classifier in `molten-core` over typed phases (validation, staging, publication) with positive and negative unit tests. r[molten.prolly.publication.classification]
      The classifier takes the phase and the failure kind and returns the outcome class. A commit failure on staging,
      publication, or deletion is unknown; a validation failure is a definite rejection; a read, codec, or write
      failure before the commit is not applied. The core keeps the pure boundary: the classifier reads typed
      observations and nothing else.
- [x] [serial] Rewire `src/prolly_map/store.rs` error helpers to call the classifier so commit-phase failures report `outcome_unknown = true` and read/codec/validation errors do not. r[molten.prolly.publication.classification]
      The adapter no longer hand-assigns the boolean. Every Redb failure is classified over the phase and kind of the
      operation it belongs to.

## Phase 2: Reconciliation

- [x] [serial] Extend `reconcile_publication` with the successor/predecessor/inconclusive readback classes and quarantine behavior consistent with world promotion. r[molten.prolly.publication.reconcile]
      The exact successor root resolves to applied, the exact predecessor root resolves to not applied, and a
      conflicting, corrupt, or unavailable readback stays unknown. The readback failure is no longer propagated as an
      error, so an inconclusive readback is never reported as applied or not applied.
- [x] [serial] Separate staging reconciliation from head reconciliation so staging ambiguity never marks the publication unknown. r[molten.prolly.publication.classification] r[molten.prolly.publication.reconcile]
      A staging failure with an unknown outcome is resolved by re-staging the content-addressed blocks, which is
      idempotent. The head publication continues on its own evidence.

## Phase 3: GC currentness

- [x] [serial] Change `ProllyBlockStorePort` deletion to `compare_retention_inventory_and_delete` with typed outcomes; update the Redb adapter to perform the inventory comparison and deletion in one write transaction. r[molten.prolly.gc.currentness]
      The port compares the current durable roots, the current pins, and the retention generation against the
      admission inventory inside the same write transaction that deletes the candidates. A mismatch returns a typed
      outcome and deletes nothing.
- [x] [serial] Register a pin for staged-but-unpublished blocks in the publication path so GC excludes them. r[molten.prolly.gc.currentness]
      The publication path pins the blocks it stages before it stages them, and releases the pin once the publication
      reaches a terminal outcome. An unknown publication outcome keeps the pin.

## Phase 4: Fault coverage

- [x] [serial] Add fault fixtures for commit failure after possible durability, publication between GC planning and deletion, corrupted readback, and stale admission against an advanced head; verify each fails against the pre-change implementation. r[molten.prolly.publication.tests]
      The fixtures cover a staging commit failure after possible durability, a corrupt readback, a stale admission
      against an advanced head, and the protection of blocks staged for an in-progress publication. The stale-admission
      and pin-protection fixtures fail against the previous implementation, which deleted on the admission flags alone.
- [x] [serial] If a third-party fault-injection dev-dependency is adopted, record its source provenance and license. r[molten.prolly.publication.tests]
      No third-party fault-injection dependency was adopted. The fixtures inject the faults through the repository's
      own memory port, so no new dependency, provenance, or license record is needed.

## Phase 5: Validation

- [x] [serial] Run workspace tests, strict Clippy, and `cairn validate --root .`; record evidence that ambiguous outcomes stay ambiguous until reconciled. r[molten.prolly.publication.reconcile] r[molten.prolly.publication.tests]
