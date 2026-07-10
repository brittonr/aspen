# Tasks: vm-shard-evidence-scope-split

## Phase 1: Scope classification core

- [x] [serial] r[molten.testing.vm_shard_scope.synthetic_metadata_boundary] Define shard evidence scope classes and pure validation for metadata-only versus executable VM claims.
- [x] [parallel] r[molten.testing.vm_shard_scope.aggregate_scope_denial] Add aggregate diagnostics for synthetic-ref-only platform claims, log-only success, unavailable evidence promotion, and missing executable VM child receipts.

## Phase 2: Check wiring

- [x] [serial] r[molten.testing.vm_shard_scope.synthetic_metadata_boundary] Update shard and aggregate receipts to bind explicit evidence scope and caveats.
- [x] [parallel] r[molten.testing.vm_shard_scope.aggregate_scope_denial] Add positive and negative fixtures covering metadata-only shards, executable VM shards, aggregate indexing, and platform-claim denial.

## Phase 3: Documentation and validation

- [x] [parallel] r[molten.testing.vm_shard_scope.synthetic_metadata_boundary] Document the difference between synthetic shard metadata and executable NixOS VM evidence.
- [x] [serial] r[molten.testing.vm_shard_scope.aggregate_scope_denial] Run focused NixOS VM receipt tests, aggregate tests, and traceability coverage updates.
