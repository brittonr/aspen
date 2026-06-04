# Tasks: Node Control Live Serve Listener

## Phase 1: Canonical listener evidence

- [x] [serial] r[molten.node_control_live_listener.spec.listener_receipts] Add listener receipt schema and ledger classification.
- [x] [serial] r[molten.node_control_live_listener.spec.session_evidence] Record bounded neighbor/session observations as non-authority evidence.

## Phase 2: Serve integration

- [x] [serial] r[molten.node_control_live_listener.spec.receive_before_drain] Add live listener processing before supervised control drain.
- [x] [serial] r[molten.node_control_live_listener.spec.bounded_listener] Bound listener event polling by explicit event and timeout limits.
- [x] [parallel] r[molten.node_control_live_listener.spec.listener_receipts] Add `molten node serve --live-iroh` CLI mode.

## Phase 3: Coverage and validation

- [x] [parallel] r[molten.node_control_live_listener.spec.loopback_tests] Add local two-endpoint listener loopback coverage.
- [x] [serial] r[molten.node_control_live_listener.spec.loopback_tests] Run Molten validation gates and Cairn strict validation with the checked-out Cairn policy.
