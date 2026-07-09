# Tasks: node-control-service-fsm

- [ ] [serial] r[molten.node_runtime.service_fsm_model] Define reviewed node-control service states, events, transition relation, and shell intents in a pure service FSM core.
- [ ] [serial] r[molten.node_runtime.service_fsm_model] Refactor a first node-control serve/run-loop slice so startup, service-lock, heartbeat, loop, listener, supervisor, and shutdown decisions are derived from the FSM core.
- [ ] [parallel] r[molten.node_runtime.service_fsm_receipts] Bind service-lock, heartbeat, supervisor, service-run, live-listener, loop, health, and shutdown receipts to prior state, event, next or preserved state, startup/lock/policy refs, decision, and diagnostics.
- [ ] [parallel] r[molten.node_runtime.service_fsm_lock_recovery] Model duplicate-runner denial, stale-lock recovery, restart bounds, heartbeat timeout, and shutdown drain as explicit transitions with preserved-state denial semantics.
- [ ] [parallel] r[molten.node_runtime.service_fsm_tests] Add positive tests for normal startup/serve/drain/stop and negative tests for duplicate runner, stale lock without recovery, stale startup binding, heartbeat timeout, restart bound, and shutdown drain over limit.
- [ ] [serial] r[molten.node_runtime.service_fsm_tests] Run focused node daemon tests and Cairn validation, then record evidence in implementation notes.