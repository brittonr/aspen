# Tasks: peer-handoff-bundles

## Phase 1: Generic bundle model

- [x] [serial] r[molten.peer_handoff.bundle_model] Define canonical `peer-handoff-bundle-v1` records for tickets, peer/session/admission evidence, scopes, capability/resource/policy refs, optional authority grants, freshness, revocation, and supporting receipts.
- [x] [serial] r[molten.peer_handoff.verify_gate] Implement pure verify/gate validation for member refs, expected bindings, freshness, malformed members, duplicate members, and wrong-scope evidence.
- [x] [parallel] r[molten.peer_handoff.authority_boundary] Preserve explicit denial when a handoff bundle is presented as operation authority, provenance, source-gate, retention, execution, or resource trust.

## Phase 2: Import/apply and compatibility

- [x] [serial] r[molten.peer_handoff.import_apply] Add import/apply flows that import only verified members and dry-run subsystem preflight by default.
- [x] [parallel] r[molten.peer_handoff.node_control_compat] Adapt node-control live workflow bundle commands to emit or accept the generic handoff form while preserving existing receipt semantics.
- [x] [parallel] r[molten.peer_handoff.consumer_scope_binding] Add subsystem scope checks for remote dataspace, job worker, retention clearance, and remote artifact sync consumers.

## Phase 3: Diagnostics and tests

- [x] [serial] r[molten.peer_handoff.diagnostics] Add diagnostics that identify missing bundle members, stale tickets, wrong peer/node/topic/scope bindings, and missing authority imports.
- [x] [serial] r[molten.peer_handoff.positive_negative_tests] Add positive bundle verify/import/apply tests and negative tests for malformed members, wrong scope, missing admission, stale ticket, duplicate member, transport-only evidence, and authority-bound operation denial.

## Phase 4: Validation

- [x] [serial] r[molten.peer_handoff.validation] Run focused handoff, node-control bundle, remote dataspace/job/retention/sync consumer tests, formatting, and Cairn validation before archiving.
