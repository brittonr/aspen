## Why

`retention-destructive-authority-evidence` made ledger GC, chunk GC, and cache invalidation require explicit retention evidence refs before destructive mutation. Those refs are currently only syntactically validated as `blake3:` refs and can still be forged, stale, scoped to the wrong requester/action/object, or used without a complete reference-index or remote-GC proof.

## What Changes

- Destructive retention evidence refs are admitted by parsing local retention admission receipts and matching them to requester, object ref, object kind, retention class, and destructive action.
- Apply-mode destructive operations fail closed when policy, authority, supporting evidence, reference-index proof, or remote-GC evidence is missing, stale, revoked, mismatched, or unresolved.
- Ledger GC, chunk GC, and eval-cache invalidation receipts expose admission diagnostics and admitted refs while preserving the existing rule that evidence is not an authority grant unless it is typed authority evidence.

## Impact

- **Files**: `src/retention.rs`, `src/ledger.rs`, `src/chunk_store.rs`, `src/eval_cache.rs`, `src/main.rs`, README/docs, runtime-spine spec.
- **Testing**: Add fail-closed and passing coverage for forged refs, wrong requester/action/object/class, missing reference-index proof, retained refs, and remote uncertainty/remote-GC proof.
