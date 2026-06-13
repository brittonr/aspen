# Tasks: Node Control Live Send Diagnostics

- [x] [serial] r[molten.node_control_live_send_diagnostics.spec.expected_ticket_guards] Add expected receiver node/topic/endpoint guards to live send and deny before transport on mismatch.
- [x] [parallel] r[molten.node_control_live_send_diagnostics.spec.peer_import_preflight] Preflight sender-state-root peer admission refs and emit `live-ticket-import` guidance when missing or stale.
- [x] [parallel] r[molten.node_control_live_send_diagnostics.spec.authority_import_preflight] Preflight sender-state-root authority grant refs and emit `authority-grant-import` guidance when missing or invalid.
- [x] [serial] r[molten.node_control_live_send_diagnostics.spec.receipt_checks] Add live-send receipt check labels for address support, operation binding, sender evidence, and join/publish success.
- [x] [parallel] r[molten.node_control_live_send_diagnostics.spec.non_authority] Keep diagnostics and import hints out of receiver-side admission authority/provenance gates.
- [x] [serial] r[molten.node_control_live_send_diagnostics.spec.tests] Cover missing import and expected-ticket mismatch diagnostics in tests.
