## Why

`retention-gc-plan-v1` and `retention-gc-apply-v1` make destructive retention decisions auditable before mutation, but subsystem GC entrypoints still need an explicit execution gate so physical deletion, tombstoning, or invalidation cannot be invoked with only raw destructive evidence.

## What Changes

- Add a retention GC execution gate receipt that binds a destructive candidate to a passing apply receipt before subsystem mutation.
- Require apply refs on non-dry-run ledger GC, chunk-store GC, and eval-cache invalidation paths before physical mutation or tombstoning.
- Keep normal destructive admission and retention evaluation as the live safety gate; apply/execute receipts remain deletion-safety evidence only.

## Impact

- **Files**: `src/retention.rs`, `src/ledger.rs`, `src/chunk_store.rs`, `src/eval_cache.rs`, `src/main.rs`, tests, docs, accepted runtime spec.
- **Testing**: Unit and CLI coverage for pass, missing apply, wrong-scope apply, and drift-after-apply denial without physical mutation.
