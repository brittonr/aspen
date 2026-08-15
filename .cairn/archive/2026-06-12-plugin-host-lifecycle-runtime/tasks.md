## Phase 1: Plugin records

- [x] [serial] r[molten.plugin_host_lifecycle.spec.install_gate] Define `plugin-manifest-v1` with artifact, ABI, lifecycle callbacks, schemas, effects, hostcalls, policy, resource, and supply-chain refs.
- [x] [serial] r[molten.plugin_host_lifecycle.spec.install_gate] Define install, permission, lifecycle, hostcall, health, upgrade, and removal receipt DTOs.
- [x] [parallel] r[molten.plugin_host_lifecycle.spec.install_gate] Classify plugin manifests and receipts in ledger/catalog/MCP views.
- [x] [parallel] r[molten.plugin_host_lifecycle.spec.hostcalls] Define the first `molten.plugin.host-abi.v1` Preserves result/error conventions.

## Phase 2: Install and activation gates

- [x] [serial] r[molten.plugin_host_lifecycle.spec.install_gate] Install only artifact-backed plugins with admitted schema/ABI/effect manifests.
- [x] [serial] r[molten.plugin_host_lifecycle.spec.install_gate] Gate permissions through authority, policy, resource, effect-handle, and supply-chain evidence.
- [x] [parallel] r[molten.plugin_host_lifecycle.spec.hostcalls] Bind reviewed Wasm/Steel/native-adapter executor preflight receipts to plugin activation.
- [x] [parallel] r[molten.plugin_host_lifecycle.spec.hostcalls] Deny undeclared filesystem/network/env/clock/process/node-control hostcalls before execution.

## Phase 3: Runtime lifecycle

- [x] [serial] r[molten.plugin_host_lifecycle.spec.hostcalls] Implement init/start/health/stop/remove callbacks through executor hostcall/effect receipts.
- [x] [serial] r[molten.plugin_host_lifecycle.spec.cleanup] Integrate plugin lifecycle with node adapter startup and service supervision receipts.
- [x] [parallel] r[molten.plugin_host_lifecycle.spec.cleanup] Convert plugin failures into lifecycle receipts without corrupting node state.
- [x] [parallel] r[molten.plugin_host_lifecycle.spec.cleanup] Retract plugin-owned assertions, service refs, handles, and catalog entries on stop/remove.

## Phase 4: Upgrade and tests

- [x] [serial] r[molten.plugin_host_lifecycle.spec.upgrade] Add plugin upgrade receipts with schema/ABI compatibility checks and rollback/cleanup evidence.
- [x] [serial] r[molten.plugin_host_lifecycle.spec.install_gate] Test install/start/health/stop/remove with a minimal reviewed plugin fixture.
- [x] [parallel] r[molten.plugin_host_lifecycle.spec.hostcalls] Test ambient hostcall denial, stale provenance, missing effect manifest, failed health, and cleanup.
- [x] [parallel] r[molten.plugin_host_lifecycle.spec.upgrade] Add Hegel properties for lifecycle receipt determinism and no-authority-escalation invariants.
