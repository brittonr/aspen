# Tasks: steel-vm-executor

- [x] [serial] r[molten.runtime.steel_vm_executor.review] Require validated Steel source/callable/allowed-hostcall review receipts before VM execution.
- [x] [serial] r[molten.runtime.steel_vm_executor.preserves_bridge] Implement deterministic Preserves-to-Steel input and Steel-to-Preserves output conversion without lossy coercions.
- [x] [serial] r[molten.runtime.steel_vm_executor.hostcalls] Register only reviewed Molten hostcall primitives and route every request through admission/effect/replay rails.
- [x] [serial] r[molten.runtime.steel_vm_executor.sandbox] Disable ambient filesystem, network, process, environment, clock, random, dynamic loading, and unreviewed modules.
- [x] [serial] r[molten.runtime.steel_vm_executor.receipts] Add deterministic resource limits for fuel/reductions, allocation, hostcall count, and input/output bytes.
- [x] [serial] r[molten.runtime.steel_vm_executor.receipts] Emit and validate canonical Steel execution receipts bound to source/callable/review/input/output/hostcall/resource refs.
- [x] [parallel] r[molten.runtime.steel_vm_executor.review] Extend executor conformance suites to compare native, reviewed Steel, and reviewed Wasm actor-output behavior.
- [x] [parallel] r[molten.runtime.steel_vm_executor.sandbox] Add negative tests for missing review receipts, forbidden ambient access, undeclared hostcalls, invalid output, resource exhaustion, and replay tampering.
