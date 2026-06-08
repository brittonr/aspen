## Phase 1: Remote clearance model

- [x] [serial] r[molten.retention.remote_gc_clearance_receipts] Add canonical per-remote GC clearance receipts with peer, object, action, policy, authority, freshness, revocation, and retained-remote bindings.
- [x] [serial] r[molten.retention.remote_gc_all_remotes] Require every destructive remote ref to be accounted for by a current passing clearance before mutation.

## Phase 2: Destructive admission plumbing

- [x] [serial] r[molten.retention.remote_gc_reconciliation_gate] Gate ledger GC, chunk GC, and eval-cache invalidation on reconciled remote clearance in addition to local remote-GC admissions.
- [x] [parallel] r[molten.retention.remote_gc_diagnostics] Surface per-peer and per-remote clearance diagnostics in destructive receipts without treating clearance as authority.
- [x] [parallel] r[molten.retention.remote_gc_cli] Add CLI support for creating and supplying remote clearance refs.

## Phase 3: Verification

- [x] [serial] r[molten.retention.remote_gc_reconciliation_tests] Add tests for partial remote sets, stale or revoked clearance, wrong peer/object/action, retained remote refs, forged refs, and all-clear pass.
- [x] [serial] r[molten.retention.remote_gc_reconciliation_tests] Verify and archive the change.
