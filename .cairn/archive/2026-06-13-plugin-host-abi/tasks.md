## Phase 1: ABI artifact model

- [x] [serial] r[molten.host_abi.artifact_model] Define host ABI plugin manifests with ABI version, value encoding boundary, hostcall/effect refs, lifecycle callbacks, constraints, policy refs, resource refs, and supply-chain evidence refs.
- [x] [serial] r[molten.host_abi.preserves_results] Define canonical Preserves `plugin-host-abi-result-v1` values with status, optional payload ref, and explicit error text; richer error-class/redaction/retry metadata remains receipt-backed or future extension.
- [x] [parallel] r[molten.host_abi.no_aspen_rpc_shape] Document Aspen as prior art only; Molten does not adopt Aspen's JSON/postcard RPC ABI shape.
- [x] [parallel] r[molten.host_abi.version_receipts] Record ABI id/version through manifest-bound install, permission, lifecycle, hostcall, health, removal, and upgrade receipts.

## Phase 2: Lifecycle and hostcalls

- [x] [serial] r[molten.host_abi.lifecycle_exports] Define initial lifecycle callbacks for init, start, health, stop, remove, and upgrade; richer artifact_info/turn/request/timer/event callbacks remain future extensions.
- [x] [serial] r[molten.host_abi.effect_hostcalls] Expose declared hostcalls only through admitted executor/effect receipt wrappers, with storage-read coverage and ambient-network denial.
- [x] [parallel] r[molten.host_abi.namespace_isolation] Enforce authority, policy, resource, supply-chain, and effect-boundary isolation on permission, lifecycle, and hostcall receipts.
- [x] [parallel] r[molten.host_abi.supervision_integration] Route failed health, callback failure, stop/remove, and cleanup evidence through lifecycle/supervision receipts.

## Phase 3: Compatibility and tests

- [x] [serial] r[molten.host_abi.compatibility] Define ABI compatibility checks for plugin upgrades using plugin id, ABI, retained schema refs, rollback refs, and cleanup refs.
- [x] [parallel] r[molten.host_abi.wasm_binding_plan] Decide the first binding form as primitive canonical Preserves/receipt interface; WIT/component wrappers remain future adapters.
- [x] [serial] r[molten.host_abi.hostcall_tests] Add tests that undeclared or unauthorized hostcalls, raw host paths, stale provenance, failed health, and incomplete cleanup are denied before side effects.
- [x] [parallel] r[molten.host_abi.property_tests] Add Hegel property tests for manifest/ref determinism, lifecycle/effect mapping, result encoding stability, and no-ambient-access invariants.
