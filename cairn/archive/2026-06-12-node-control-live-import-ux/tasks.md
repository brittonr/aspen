# Tasks: Node Control Live Import UX

- [x] [serial] r[molten.node_control_live_import_ux.spec.ticket_import_receipt] Add canonical live-ticket import receipts and ledger classification.
- [x] [parallel] r[molten.node_control_live_import_ux.spec.ticket_admission_freshness] Validate optional peer-admission schema, ticket/node/topic/peer binding, not-before sequence, and expiry before importing.
- [x] [serial] r[molten.node_control_live_import_ux.spec.authority_import_receipt] Add canonical authority-grant import receipts and ledger classification.
- [x] [parallel] r[molten.node_control_live_import_ux.spec.authority_binding] Validate grant peer/node/operation/scope/epoch/expiry/revocation bounds before importing.
- [x] [parallel] r[molten.node_control_live_import_ux.spec.import_non_authority] Keep import receipts out of receiver bootstrap, authority, policy/resource, idempotency, and provenance admission.
- [x] [serial] r[molten.node_control_live_import_ux.spec.cli] Expose `live-ticket-import` and `authority-grant-import` CLI commands with binding flags and receipt output.
