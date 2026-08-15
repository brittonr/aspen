## Phase 1: Implementation

- [x] [serial] Add `retention-gc-execute-v1` schema, storage, parsing, summaries, and ledger classification. r[molten.retention.gc_execution_gates]
- [x] [serial] Require matching apply refs on non-dry-run ledger GC, chunk-store GC, and eval-cache invalidation before physical mutation. r[molten.retention.gc_execution_gates]
- [x] [serial] Cover pass, missing apply, wrong-scope apply, and drift-after-apply denial in unit/CLI tests. r[molten.retention.gc_execution_gates]
- [x] [serial] Update docs/specs and archive the change after validation. r[molten.retention.gc_execution_gates]
