# Tasks: dataspace-watch-informers

## Phase 1: Watch cursors and events

- [x] [serial] r[molten.resource_watch.revision_cursor_streams] Define pure watch event and revision cursor DTOs for added, modified, deleted, bookmark, and compacted events over resource refs and generations.
- [x] [parallel] r[molten.resource_watch.revision_cursor_streams] Add positive watch fixtures for ordered create/update/delete streams and negative fixtures for stale cursors, skipped revisions, reordered events, and log-only watch claims.

## Phase 2: Informer snapshots and selector authority

- [x] [serial] r[molten.resource_watch.informer_snapshot_consistency] Implement pure informer state transitions that validate initial list refs, event application, final cursors, relist requirements, and replay receipts.
- [x] [parallel] r[molten.resource_watch.informer_snapshot_consistency] Add positive informer-cache fixtures and negative fixtures for missed events, duplicate events, wrong starting cursor, and compaction without relist evidence.
- [x] [serial] r[molten.resource_watch.selector_authority_bounds] Gate selector scope, label matching, and cross-scope watch expansion through authority and policy evidence.
- [x] [parallel] r[molten.resource_watch.selector_authority_bounds] Add positive authorized selector fixtures and negative fixtures for broad discovery, unauthorized labels, cross-scope watches, and unsupported selector operators.

## Phase 3: Documentation and validation

- [x] [serial] r[molten.resource_watch.revision_cursor_streams] Documented watch/informer semantics as dataspace-backed and non-Kubernetes-compatible, and ran focused watch/informer tests plus `cairn validate --root .`
