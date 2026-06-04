## Phase 1: ABI artifact model

- [ ] [serial] r[molten.host_abi.artifact_model] Define host ABI artifacts with version, value encoding, hostcall/effect mapping, lifecycle exports, result/error encoding, constraints, policy refs, and evidence refs.
- [ ] [serial] r[molten.host_abi.preserves_results] Define canonical Preserves result/error variants with stable error classes, redaction metadata, receipt refs, and retry/idempotency guidance.
- [ ] [parallel] r[molten.host_abi.no_aspen_rpc_shape] Document Aspen as prior art only; Molten does not adopt Aspen's JSON/postcard RPC ABI shape.
- [ ] [parallel] r[molten.host_abi.version_receipts] Record ABI id/version in execution and hostcall receipts.

## Phase 2: Lifecycle and hostcalls

- [ ] [serial] r[molten.host_abi.lifecycle_exports] Define initial lifecycle callbacks for artifact_info, init, handle_turn/request, health, on_timer/on_event, and shutdown.
- [ ] [serial] r[molten.host_abi.effect_hostcalls] Expose send/assert/retract/observe/blob/storage/trace/clock/random only through admitted effect-request wrappers.
- [ ] [parallel] r[molten.host_abi.namespace_isolation] Enforce namespace and resource isolation on every hostcall.
- [ ] [parallel] r[molten.host_abi.supervision_integration] Route callback failures through lifecycle/supervision receipts.

## Phase 3: Compatibility and tests

- [ ] [serial] r[molten.host_abi.compatibility] Define ABI compatibility checks for artifact installation and upgrade sessions.
- [ ] [parallel] r[molten.host_abi.wasm_binding_plan] Decide first Wasm binding form: WIT/component or primitive Preserves byte interface.
- [ ] [serial] r[molten.host_abi.hostcall_tests] Add tests that undeclared or unauthorized hostcalls are denied before side effects.
- [ ] [parallel] r[molten.host_abi.property_tests] Add Hegel property tests for hostcall/effect mapping, result encoding stability, and no-ambient-access invariants.
