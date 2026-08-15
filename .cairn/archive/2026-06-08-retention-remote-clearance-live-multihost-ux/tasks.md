## Phase 1: Multi-host live commands

- [x] [serial] r[molten.retention.remote_clearance_live_multihost_request] Add a requester-side command that stores a clearance request and sends its ref through node-control live transport.
- [x] [serial] r[molten.retention.remote_clearance_live_multihost_response] Add a peer-side command that stores a clearance response and sends its ref back through node-control live transport.
- [x] [serial] r[molten.retention.remote_clearance_live_multihost_import] Add a requester-side command that imports the peer response and stores the live workflow evidence.

## Phase 2: Safety and diagnostics

- [x] [serial] r[molten.retention.remote_clearance_live_multihost_import_gate] Preserve `retention-remote-gc-clearance-import-v1` as the only step that stores usable peer clearance for destructive admission.
- [x] [parallel] r[molten.retention.remote_clearance_live_multihost_diagnostics] Surface missing or denied send, receive, ingress, wrong-peer, wrong-request, wrong-remote, retained, stale, revoked, and tampered-response diagnostics without treating live transport as trust.

## Phase 3: Verification

- [x] [serial] r[molten.retention.remote_clearance_live_multihost_tests] Test request send, response send, final workflow assembly, denied transport evidence, and destructive admission through imported peer clearance.
- [x] [serial] r[molten.retention.remote_clearance_live_multihost_tests] Verify and archive the change.
