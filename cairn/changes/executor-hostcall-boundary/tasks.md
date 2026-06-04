# Tasks: executor-hostcall-boundary

- [x] [serial] r[molten.runtime.executor_hostcall_boundary.envelopes] Define canonical actor input, hostcall request/decision, actor output, and executor preflight schemas.
- [x] [serial] r[molten.runtime.executor_hostcall_boundary.shell_admission] Route hostcalls through existing policy, capability, budget, effect-log, and replay validation rails.
- [x] [serial] r[molten.runtime.executor_hostcall_boundary.native_preflight] Add native executor preflight evidence with actor-kind binding, sandbox refs, allowed hostcalls, validation, and gate receipt checks.
- [x] [serial] r[molten.runtime.executor_hostcall_boundary.steel_preflight] Add Steel executor preflight receipts for reviewed modules/callables and allowed hostcalls.
- [x] [serial] r[molten.runtime.executor_hostcall_boundary.wasm_preflight] Add Wasm/component executor preflight receipts for module refs, inspection results, WIT refs, and sandbox config.
- [x] [parallel] r[molten.runtime.executor_hostcall_boundary.conformance] Add conformance suites comparing native, Steel, and Wasm actors over identical Preserves inputs.
- [x] [parallel] r[molten.runtime.executor_hostcall_boundary.tests] Add negative tests for ambient IO attempts, undeclared hostcalls, stale preflight receipts, unsupported executor kinds, and replay divergence.
- [x] [serial] r[molten.runtime.executor_hostcall_boundary.wasmtime_execution] Add a minimal no-WASI Wasmtime executor path for reviewed Wasm hostcall actors with fuel/memory limits and canonical execution receipts.
- [x] [parallel] r[molten.runtime.executor_hostcall_boundary.native_tests] Add native-boundary tests for recorded hostcall envelopes, gate receipt checks, tampered hostcall validation, and replay divergence classification.
