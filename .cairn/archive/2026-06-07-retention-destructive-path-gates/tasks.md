## Phase 1: Destructive retention gates

- [x] [serial] r[molten.retention.ledger_gc_gate] Gate evidence-ledger GC removal with passing retention receipts.
- [x] [serial] r[molten.retention.chunk_gc_gate] Gate chunk-store manifest/chunk removal and tombstone receipts with passing retention receipts.
- [x] [serial] r[molten.retention.eval_cache_tombstone_gate] Gate evaluation-cache invalidation tombstones with passing retention receipts.
- [x] [serial] r[molten.retention.secret_cleanup_gate] Require secret cleanup receipts to bind actual passing retention receipts for the cleaned secret.

## Phase 2: Receipts, CLI, and tests

- [x] [parallel] r[molten.retention.subsystem_receipt_refs] Expose retention receipt refs in subsystem receipts and CLI diagnostics.
- [x] [parallel] r[molten.retention.destructive_gate_tests] Test pass and fail-closed denial paths for ledger GC, chunk GC, cache invalidation, and secret cleanup.
