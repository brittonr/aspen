# Tasks

## Phase 1: Classification

- [ ] [serial] Add the pure `classify_port_failure` classifier in `molten-core` over typed phases (validation, staging, publication) with positive and negative unit tests. r[molten.prolly.publication.classification]
- [ ] [serial] Rewire `src/prolly_map/store.rs` error helpers to call the classifier so commit-phase failures report `outcome_unknown = true` and read/codec/validation errors do not. r[molten.prolly.publication.classification]

## Phase 2: Reconciliation

- [ ] [serial] Extend `reconcile_publication` with the successor/predecessor/inconclusive readback classes and quarantine behavior consistent with world promotion. r[molten.prolly.publication.reconcile]
- [ ] [serial] Separate staging reconciliation from head reconciliation so staging ambiguity never marks the publication unknown. r[molten.prolly.publication.classification] r[molten.prolly.publication.reconcile]

## Phase 3: GC currentness

- [ ] [serial] Change `ProllyBlockStorePort` deletion to `compare_retention_inventory_and_delete` with typed outcomes; update the Redb adapter to perform the inventory comparison and deletion in one write transaction. r[molten.prolly.gc.currentness]
- [ ] [serial] Register a pin for staged-but-unpublished blocks in the publication path so GC excludes them. r[molten.prolly.gc.currentness]

## Phase 4: Fault coverage

- [ ] [serial] Add fault fixtures for commit failure after possible durability, publication between GC planning and deletion, corrupted readback, and stale admission against an advanced head; verify each fails against the pre-change implementation. r[molten.prolly.publication.tests]
- [ ] [serial] If a third-party fault-injection dev-dependency is adopted, record its source provenance and license. r[molten.prolly.publication.tests]

## Phase 5: Validation

- [ ] [serial] Run workspace tests, strict Clippy, and `cairn validate --root .`; record evidence that ambiguous outcomes stay ambiguous until reconciled. r[molten.prolly.publication.reconcile] r[molten.prolly.publication.tests]
