## Why

Destructive retention admission now validates local policy, authority, supporting evidence, reference-index, and remote-GC admission refs. A remaining safety gap is remote uncertainty: a single local `remote-gc` admission can name remote refs, but Molten does not yet require per-remote clearance evidence that binds the peer, requester, object, action, policy, authority, freshness, revocation state, and retained-remote status.

## What Changes

- Add canonical per-remote GC clearance receipts for destructive retention flows.
- Require destructive retention admission to reconcile every configured/known remote ref with a current passing clearance before local deletion, tombstoning, redaction, or compaction.
- Surface per-peer/per-remote diagnostics in ledger GC, chunk GC, and eval-cache invalidation receipts.
- Add operator CLI support for producing and supplying remote clearance evidence.

## Impact

- **Files**: `src/retention.rs`, `src/main.rs`, `src/preserves_rail.rs`, `src/ledger.rs`, `src/chunk_store.rs`, `src/eval_cache.rs`, README/docs, Cairn runtime-spine specs.
- **Testing**: Unit/CLI coverage for partial remote sets, stale/revoked clearance, wrong peer/object/action, retained remote refs, and all-clear pass.
