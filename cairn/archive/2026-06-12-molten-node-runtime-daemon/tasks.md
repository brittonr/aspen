## Phase 1: Node configuration and startup evidence

- [x] [serial] r[molten.node_runtime_daemon.config_dto] Define canonical `node-config-v1` with explicit identity, state-root, adapter, policy, capability, resource, and effect profile refs.
- [x] [serial] r[molten.node_runtime_daemon.startup_receipt] Define `node-startup-receipt-v1` binding config, identity, adapter startup, version, policy, capability, and resource evidence.
- [x] [parallel] r[molten.node_runtime_daemon.no_ambient_state] Reject node startup without explicit state-root/profile refs and no-ambient-authority checks.
- [x] [parallel] r[molten.node_runtime_daemon.ledger_classification] Classify node config/startup/control/shutdown artifacts in ledger/catalog views.

## Phase 2: Adapter lifecycle

- [x] [serial] r[molten.node_runtime_daemon.adapter_order] Start ledger, artifact registry, chunk store, typed storage, eval cache, remote dataspace, service supervision, job runtime, coordination, plugin-host, catalog/MCP, and control adapters in deterministic dependency order.
- [x] [serial] r[molten.node_runtime_daemon.adapter_receipts] Emit `node-adapter-receipt-v1` for adapter start, verify, deny, and shutdown decisions.
- [x] [parallel] r[molten.node_runtime_daemon.index_verify] Verify or rebuild local Redb/indexed adapters before admitting control requests.
- [x] [parallel] r[molten.node_runtime_daemon.resource_profile] Bind startup and adapter runtime budgets to resource receipts.

## Phase 3: Local control surface

- [x] [serial] r[molten.node_runtime_daemon.control_request] Define `node-control-request-v1` and `node-control-receipt-v1` for local status/install/run/gate/shutdown commands.
- [x] [serial] r[molten.node_runtime_daemon.preserves_control] Implement a local-only Preserves control socket/file/stdio profile; rendered text is non-normative.
- [x] [parallel] r[molten.node_runtime_daemon.authority_gate] Gate every control request through authority, policy, effect-handle, and resource evidence.
- [x] [parallel] r[molten.node_runtime_daemon.subreceipt_binding] Bind sub-receipts from artifact, job, remote, storage, and gate operations into the control receipt.

## Phase 4: Shutdown and recovery

- [x] [serial] r[molten.node_runtime_daemon.graceful_shutdown] Add receipt-backed shutdown: stop intake, drain admitted turns/jobs, persist indexes, close adapters.
- [x] [serial] r[molten.node_runtime_daemon.restart_recovery] On restart, verify previous startup/shutdown receipts and adapter indexes before declaring healthy.
- [x] [parallel] r[molten.node_runtime_daemon.health_receipts] Emit health receipts binding adapter refs, head refs, open jobs, and replay eligibility.
- [x] [parallel] r[molten.node_runtime_daemon.tests] Add CLI and library tests for init/run/status/control denial/shutdown/restart recovery.
