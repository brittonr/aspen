## Phase 1: Workflow records

- [x] [serial] r[molten.retention.remote_clearance_request_response] Add canonical request and response records for peer-produced remote GC clearance.
- [x] [serial] r[molten.retention.remote_clearance_import_gate] Add a fail-closed import receipt that stores only passing scope-matching peer clearance values locally.

## Phase 2: Operator surface

- [x] [parallel] r[molten.retention.remote_clearance_workflow_cli] Add CLI commands to build requests, produce peer responses, import responses, and summarize workflow artifacts.
- [x] [parallel] r[molten.retention.remote_clearance_workflow_diagnostics] Surface stale, revoked, retained, wrong-peer, wrong-remote, wrong-request, and tampered-response diagnostics without treating workflow receipts as authority.

## Phase 3: Verification

- [x] [serial] r[molten.retention.remote_clearance_workflow_tests] Test pass import, retained/stale peer denial, tampered/wrong request response denial, and destructive admission using imported peer clearance.
- [x] [serial] r[molten.retention.remote_clearance_workflow_tests] Verify and archive the change.
