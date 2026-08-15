## Phase 1: Live workflow records

- [x] [serial] r[molten.retention.remote_clearance_live_transport] Add node-control live workflow receipts that carry remote clearance request and response artifacts.
- [x] [serial] r[molten.retention.remote_clearance_live_import_gate] Ensure live workflow completion still imports through `retention-remote-gc-clearance-import-v1` before local destructive admission may use peer clearance.

## Phase 2: Operator surface

- [x] [parallel] r[molten.retention.remote_clearance_live_cli] Add deterministic CLI support for request/respond/import over live loopback transport.
- [x] [parallel] r[molten.retention.remote_clearance_live_diagnostics] Surface live transport, peer, request, response, retained, stale, revoked, and tampered-response diagnostics as evidence-only records.

## Phase 3: Verification

- [x] [serial] r[molten.retention.remote_clearance_live_tests] Test passing loopback import, retained or stale peer denial, wrong peer/request denial, tampered response denial, and destructive admission using imported live clearance.
- [x] [serial] r[molten.retention.remote_clearance_live_tests] Verify and archive the change.
