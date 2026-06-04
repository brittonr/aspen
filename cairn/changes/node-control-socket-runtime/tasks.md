## Phase 1: Durable local control profile

- [x] [serial] r[molten.node_control_socket.spec.persistent_inbox] Persist canonical node control requests in an explicit state-root inbox and outbox profile.
- [x] [serial] r[molten.node_control_socket.spec.queue_receipts] Emit queue/dispatch receipts that bind request refs and local control profile evidence.
- [x] [parallel] r[molten.node_control_socket.spec.no_ambient_control] Reject control submission or dispatch that relies on ambient state roots.

## Phase 2: Locking and dispatch

- [x] [serial] r[molten.node_control_socket.spec.process_lock] Write an active node lock bound to startup evidence and reject duplicate/stale dispatch.
- [x] [serial] r[molten.node_control_socket.spec.dispatch_receipts] Dispatch status and shutdown via submitted Preserves requests and deny unwired install/run/gate operations before side effects.
- [x] [parallel] r[molten.node_control_socket.spec.authority_resource_gate] Require explicit authority, policy, and resource refs for passing control receipts.

## Phase 3: Ledger and operator coverage

- [x] [serial] r[molten.node_control_socket.spec.ledger_imports] Import requests, queue receipts, suboperation receipts, health/shutdown receipts, and control receipts into the node ledger.
- [x] [parallel] r[molten.node_control_socket.spec.tests] Add library and CLI tests for submit, dispatch status, dispatch shutdown, stale lock denial, and fail-closed unwired operations.
